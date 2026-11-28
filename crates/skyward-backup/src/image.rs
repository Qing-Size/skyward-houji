//! 单分区的裸镜像：跟带清单的多段备份分开放着，一个分区一个 `.img`，旁边一份
//! `<名字>.img.sha256`。
//!
//! 文件名的形状是 `<分区>-<时间戳>.img`，跟 `pages::partitions` 导出时写的
//! 名字一样，所以分区管理页导出的东西，这一页直接就能列出来、能写回去。
//!
//! 从「青菜豆腐」分支的 `crates/skyward-backup/src/image.rs` 搬来，格式跟着
//! 我们的走：那边是一个备份一个目录、目录里一个 `.img` 加一份校验，我们是
//! 平铺在 `Skyward/backup` 里的 `.img` 加校验；校验的写法（`<64 位十六进制>
//! <文件名>`）与「先整份核对再写回」的做法照搬。

use anyhow::{ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

/// 镜像放在哪个地方的哪个文件夹里，从那个地方的顶层算起。
pub const FOLDER: &str = "Skyward/backup";

/// 一次读写的多少：跟导出时一样 4 MiB 一段。
const CHUNK: usize = 4 << 20;

/// 一份镜像备份，只看元数据，不读内容。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Copy {
    /// 文件名里的分区名。
    pub partition: String,
    /// 文件名，`.img` 一起。
    pub file: String,
    pub size: u64,
    /// 旁边那份校验说的值；没有那份文件时为空，写回时就不核对。
    pub sha256: Option<String>,
    /// 写下来的时间，按秒；读不到时为 0。
    pub made: i64,
}

impl Copy {
    /// 这一份在 `root` 里的全路径。
    pub fn path(&self, root: &Path) -> PathBuf {
        root.join(&self.file)
    }

    /// 校验文件在哪：镜像旁边，名字后面接 `.sha256`。
    pub fn checksum(&self, root: &Path) -> PathBuf {
        root.join(format!("{}.sha256", self.file))
    }
}

/// `<分区>-<时间戳>.img` 里的分区名。
pub fn partition_of(file: &str) -> Option<&str> {
    let stem = file.strip_suffix(".img")?;
    let (name, stamp) = stem.rsplit_once('-')?;
    let stamp = stamp.trim();
    let numeric = !stamp.is_empty() && stamp.bytes().all(|b| b.is_ascii_digit());
    (numeric && !name.is_empty()).then_some(name)
}

/// 镜像文件旁边那份校验的路径。
pub fn checksum_of(image: &Path) -> PathBuf {
    let mut name = image.as_os_str().to_os_string();
    name.push(".sha256");
    PathBuf::from(name)
}

/// `root` 里放着的镜像，最新的在前；读不出来的（不是镜像、名字不成形状）不列。
pub fn copies(root: &Path) -> Result<Vec<Copy>> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".img") || name.starts_with('.') {
            continue;
        }
        let Some(partition) = partition_of(&name) else { continue };
        let metadata = entry.metadata()?;
        if !metadata.is_file() || metadata.len() == 0 {
            continue;
        }
        let made = metadata.modified().ok().and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64);
        let sha256 = read_checksum(&checksum_of(&entry.path()), &name).ok();
        found.push(Copy { partition: partition.to_string(), file: name, size: metadata.len(), sha256, made });
    }
    found.sort_by(|a, b| b.made.cmp(&a.made).then_with(|| a.file.cmp(&b.file)));
    Ok(found)
}

/// 校验文件里写的东西：`<64 位十六进制> <文件名>`，两样都要对上。
fn read_checksum(path: &Path, file: &str) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(metadata.is_file() && metadata.len() <= 512, "invalid image checksum file");
    let text = fs::read_to_string(path)?;
    let fields: Vec<&str> = text.split_whitespace().collect();
    ensure!(
        fields.len() == 2 && fields[0].len() == 64 && fields[0].bytes().all(|c| c.is_ascii_hexdigit()) && fields[1] == file,
        "invalid image checksum record"
    );
    Ok(fields[0].to_ascii_lowercase())
}

/// 把一个分区（或任何文件）原样写成一份镜像：`size` 是要写的字节数（块设备
/// 自己说不出来），边写边算 sha256，写完把 `<文件名>.sha256` 放在旁边。给出
/// 算出来的值。
pub fn dump(source: &Path, size: u64, image: &Path) -> Result<String> {
    if let Some(dir) = image.parent() {
        fs::create_dir_all(dir).with_context(|| format!("make {}", dir.display()))?;
    }
    let mut from = File::open(source).with_context(|| format!("open {}", source.display()))?;
    let mut to = File::create(image).with_context(|| format!("create {}", image.display()))?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; CHUNK];
    let mut left = size;
    while left > 0 {
        let want = (left).min(CHUNK as u64) as usize;
        let read = from.read(&mut buffer[..want]).with_context(|| format!("read {}", source.display()))?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
        to.write_all(&buffer[..read]).with_context(|| format!("write {}", image.display()))?;
        left -= read as u64;
    }
    to.sync_all()?;
    let sum = hex::encode(hash.finalize());
    let file = image.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let checksum = checksum_of(image);
    fs::write(&checksum, format!("{sum}  {file}\n")).with_context(|| format!("write {}", checksum.display()))?;
    Ok(sum)
}

/// 把一份镜像从头读到尾，给出它的 sha256，并把读了多少报出去。
pub fn digest(image: &Path, stop: &AtomicBool, progress: &mut dyn FnMut(u64, u64)) -> Result<String> {
    let mut file = File::open(image).with_context(|| format!("open {}", image.display()))?;
    let total = file.metadata()?.len();
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; CHUNK];
    let mut done = 0u64;
    while done < total {
        cancelled(stop)?;
        let want = (total - done).min(CHUNK as u64) as usize;
        let read = file.read(&mut buffer[..want])?;
        ensure!(read > 0, "image ends early at {done} of {total} bytes");
        hash.update(&buffer[..read]);
        done += read as u64;
        progress(done, total);
    }
    Ok(hex::encode(hash.finalize()))
}

/// 把一份镜像写回 `into`（一般就是 `/dev/block/by-name/<分区>`）。
///
/// 先整份核对一遍，核对过的就是写回去的那一份，中间没有换文件的空隙；核对
/// 不过就不写。`expected` 为 `None`（没有校验文件的旧份）时只照着长度写。
/// 报的是写入了多少、一共多少。
pub fn write_back(image: &Path, into: &Path, expected: Option<&str>, stop: &AtomicBool, progress: &mut dyn FnMut(u64, u64)) -> Result<()> {
    let mut from = File::open(image).with_context(|| format!("open {}", image.display()))?;
    let total = from.metadata()?.len();
    ensure!(total > 0, "the image is empty");
    if let Some(expected) = expected {
        let mut hash = Sha256::new();
        let mut buffer = vec![0u8; CHUNK];
        let mut read_total = 0u64;
        while read_total < total {
            cancelled(stop)?;
            let want = (total - read_total).min(CHUNK as u64) as usize;
            let read = from.read(&mut buffer[..want])?;
            ensure!(read > 0, "image ends early at {read_total} of {total} bytes");
            hash.update(&buffer[..read]);
            read_total += read as u64;
        }
        let sum = hex::encode(hash.finalize());
        ensure!(sum == expected.to_ascii_lowercase(), "the image does not match its checksum: {sum} is not {expected}");
        from.seek(SeekFrom::Start(0))?;
    }
    let mut to = fs::OpenOptions::new().write(true).open(into).with_context(|| format!("open {} for writing", into.display()))?;
    let mut buffer = vec![0u8; CHUNK];
    let mut done = 0u64;
    while done < total {
        cancelled(stop)?;
        let want = (total - done).min(CHUNK as u64) as usize;
        let read = from.read(&mut buffer[..want])?;
        ensure!(read > 0, "image ends early at {done} of {total} bytes");
        to.write_all(&buffer[..read]).with_context(|| format!("write {}", into.display()))?;
        done += read as u64;
        progress(done, total);
    }
    to.sync_all()?;
    Ok(())
}

/// 把一份镜像连同它的校验一起删掉。
pub fn delete(root: &Path, copy: &Copy) -> Result<()> {
    let image = copy.path(root);
    fs::remove_file(&image).with_context(|| format!("remove {}", image.display()))?;
    // 校验文件不是必需的：没有就当时没有。
    match fs::remove_file(copy.checksum(root)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn cancelled(stop: &AtomicBool) -> Result<()> {
    ensure!(!stop.load(Ordering::Relaxed), "stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().expect("a temporary folder")
    }

    fn image_with(root: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(name);
        fs::write(&path, bytes).expect("write the image");
        path
    }

    fn sidecar(image: &Path, sum: &str) {
        let name = image.file_name().unwrap().to_string_lossy().into_owned();
        fs::write(checksum_of(image), format!("{sum}  {name}\n")).expect("write the checksum");
    }

    #[test]
    fn a_name_says_which_partition_it_is_of() {
        assert_eq!(partition_of("boot-1762486400.img"), Some("boot"));
        assert_eq!(partition_of("vbmeta_system-1762486400.img"), Some("vbmeta_system"));
        assert_eq!(partition_of("Super-1.img"), Some("Super"));
        // 时间戳不是数字、名字空了、不是镜像：都不算。
        assert_eq!(partition_of("boot-yesterday.img"), None);
        assert_eq!(partition_of("-1762486400.img"), None);
        assert_eq!(partition_of("boot-1762486400.sha256"), None);
        assert_eq!(partition_of("boot.img"), None);
    }

    #[test]
    fn the_copies_found_are_the_images_with_a_partition_in_their_name_newest_first() {
        let dir = scratch();
        let root = dir.path();
        image_with(root, "boot-100.img", b"boot");
        image_with(root, "system_a-300.img", b"system");
        image_with(root, "notes.img", b"not an image backup");
        image_with(root, "empty-200.img", b"");
        fs::write(root.join("boot-100.img.sha256"), "0".repeat(64) + "  boot-100.img\n").unwrap();
        let found = copies(root).expect("the folder is read");
        let names: Vec<&str> = found.iter().map(|c| c.file.as_str()).collect();
        assert!(names.contains(&"boot-100.img") && names.contains(&"system_a-300.img"), "{names:?}");
        assert!(!names.contains(&"notes.img"), "a name with no partition in it is not one of ours");
        assert!(!names.contains(&"empty-200.img"), "an empty file is not an image");
        let boot = found.iter().find(|c| c.file == "boot-100.img").unwrap();
        assert_eq!(boot.partition, "boot");
        assert_eq!(boot.size, 4);
        assert_eq!(boot.sha256.as_deref(), Some("0".repeat(64).as_str()), "the checksum beside it is read");
        assert!(found.iter().all(|c| c.sha256.is_some() || c.file != "boot-100.img"), "only the one has a checksum");
    }

    #[test]
    fn a_folder_that_is_not_there_holds_no_copies_rather_than_failing() {
        let dir = scratch();
        assert_eq!(copies(&dir.path().join("nothing here")).unwrap(), Vec::new());
    }

    #[test]
    fn a_checksum_that_names_another_file_or_is_mangled_is_no_checksum() {
        let dir = scratch();
        let root = dir.path();
        let image = image_with(root, "boot-1.img", b"boot");
        // 名字对不上：不当校验用。
        fs::write(checksum_of(&image), "0".repeat(64) + "  system-1.img\n").unwrap();
        assert_eq!(copies(root).unwrap()[0].sha256, None);
        // 位数不对也不当。
        fs::write(checksum_of(&image), "abc  boot-1.img\n").unwrap();
        assert_eq!(copies(root).unwrap()[0].sha256, None);
        // 太长的校验文件不读。
        fs::write(checksum_of(&image), "0".repeat(600)).unwrap();
        assert_eq!(copies(root).unwrap()[0].sha256, None);
    }

    #[test]
    fn writing_a_copy_back_checks_it_first_and_says_how_far_it_came() {
        let dir = scratch();
        let root = dir.path();
        let image = image_with(root, "boot-1.img", b"boot the slot's kernel");
        let sum = digest(&image, &AtomicBool::new(false), &mut |_, _| {}).unwrap();
        sidecar(&image, &sum);
        let target = root.join("device");
        fs::write(&target, vec![0u8; 64]).unwrap();
        let mut at = Vec::new();
        write_back(&image, &target, Some(&sum), &AtomicBool::new(false), &mut |done, total| at.push((done, total))).expect("written");
        assert_eq!(&fs::read(&target).unwrap()[..22], b"boot the slot's kernel", "the image went in whole, over what the partition held");
        assert_eq!(at.last().copied(), Some((fs::metadata(&image).unwrap().len(), fs::metadata(&image).unwrap().len())));
        assert_eq!(copies(root).unwrap()[0].sha256.as_deref(), Some(sum.as_str()));
    }

    #[test]
    fn an_image_that_does_not_match_its_checksum_is_not_written_at_all() {
        let dir = scratch();
        let root = dir.path();
        let image = image_with(root, "boot-1.img", b"one thing");
        sidecar(&image, &"0".repeat(64));
        let target = root.join("device");
        fs::write(&target, b"left alone").unwrap();
        let error = write_back(&image, &target, Some(&"0".repeat(64)), &AtomicBool::new(false), &mut |_, _| {}).unwrap_err();
        assert!(format!("{error:#}").contains("does not match its checksum"), "{error:#}");
        assert_eq!(fs::read(&target).unwrap(), b"left alone", "nothing was written");
    }

    #[test]
    fn a_copy_with_no_checksum_beside_it_is_still_written_back() {
        let dir = scratch();
        let root = dir.path();
        let image = image_with(root, "boot-1.img", b"older copy");
        let target = root.join("device");
        fs::write(&target, vec![0u8; 4]).unwrap();
        // 没有校验：只照着长度写，报的是写进去多少。
        write_back(&image, &target, None, &AtomicBool::new(false), &mut |_, _| {}).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"older copy");
    }

    #[test]
    fn stopping_leaves_the_work_where_it_stood() {
        let dir = scratch();
        let root = dir.path();
        // 比一段还大：一段读完才看得见停止。
        let image = image_with(root, "boot-1.img", &vec![7u8; 5 << 20]);
        let stop = AtomicBool::new(false);
        let mut counted = 0;
        let error = digest(&image, &stop, &mut |_, _| {
            counted += 1;
            stop.store(true, Ordering::Relaxed);
        })
        .unwrap_err();
        assert_eq!(format!("{error:#}"), "stopped");
        assert_eq!(counted, 1, "the first piece was read, then the stop was seen");
    }

    #[test]
    fn deleting_takes_the_image_and_its_checksum_and_leaves_the_rest() {
        let dir = scratch();
        let root = dir.path();
        let boot = image_with(root, "boot-1.img", b"boot");
        image_with(root, "system_a-2.img", b"system");
        sidecar(&boot, &digest(&boot, &AtomicBool::new(false), &mut |_, _| {}).unwrap());
        let copy = copies(root).unwrap().into_iter().find(|c| c.partition == "boot").unwrap();
        delete(root, &copy).unwrap();
        assert!(!boot.exists() && !checksum_of(&boot).exists());
        assert!(root.join("system_a-2.img").exists(), "the other copy is left alone");
        let names: Vec<String> = copies(root).unwrap().into_iter().map(|c| c.file).collect();
        assert_eq!(names, ["system_a-2.img"]);
    }
}
