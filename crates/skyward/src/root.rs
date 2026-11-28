//! Writing KernelSU into the current slot: the KMI the page picked, the
//! phone's own `ksud`, and the boot image it came with.

use anyhow::{bail, ensure, Context, Result};
use sha2::{Digest, Sha256};
use skyward_install::{ImageKind, Ramdisk};
use serde_json::Value;
use skyward_ui::app::{KernelSuEnd, KernelSuStage, KernelSuStatus, KmiRow, MagiskStatus};
use skyward_net::http::{self, Servers};
use skyward_vab::Slot;
use std::fs::{self, File};
use std::io::{Cursor, Read, Seek, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const ROOT_DIR: &str = "/data/media/0/Skyward/root";
const KERNELSU_MANIFEST: &str = "kernelsu.conf";
const MAGISK_MANIFEST: &str = "magisk.conf";
const KERNELSU_WORK: &str = "/tmp/skyward-kernelsu";
const MAGISK_WORK: &str = "/tmp/skyward-magisk";

pub enum Task {
    /// 从 GitHub 取最新的 KernelSU，写进当前槽。
    InstallKernelSu { kmi: String, slot: Option<Slot>, servers: Servers, module: Option<String> },
    RestoreKernelSu,
    /// 从 GitHub 取最新的 Magisk APK，写进当前槽。
    InstallMagisk { slot: Option<Slot>, servers: Servers },
    RestoreMagisk,
}

pub enum Event {
    Stage(KernelSuStage),
    Fetched(String),
    Fallback(String),
    Ended(KernelSuEnd),
}

/// 在线上的两个最新版本号，页面上先告诉用户会装到哪一版。
pub enum VersionEvent {
    Done(Option<String>, Option<String>),
}

pub struct Versions {
    worker: skyward_sys::worker::Worker<VersionEvent>,
}

impl Versions {
    pub fn start(servers: Servers) -> Result<Versions> {
        let worker = skyward_sys::worker::Worker::start("root-versions", move |tell| {
            let agent = http::agent(servers);
            let ask = |url: &str| -> Option<String> {
                let mut response = agent
                    .get(url)
                    .config()
                    .timeout_global(Some(Duration::from_secs(20)))
                    .build()
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .header("User-Agent", format!("Skyward/{}", skyward_info::VERSION))
                    .call()
                    .ok()?;
                let status = response.status();
                if !status.is_success() {
                    return None;
                }
                let body = response.body_mut().read_to_string().ok()?;
                let release: Value = serde_json::from_str(&body).ok()?;
                release.get("tag_name").and_then(Value::as_str).map(str::to_string)
            };
            let kernelsu = ask(skyward_root::KERNELSU_LATEST);
            let magisk = ask(skyward_root::MAGISK_LATEST);
            let _ = skyward_sys::kmsg(&format!("root: online versions are {kernelsu:?} and {magisk:?}"));
            tell.tell(VersionEvent::Done(kernelsu, magisk));
        })?;
        Ok(Versions { worker })
    }

    pub fn fd(&self) -> std::os::fd::RawFd {
        self.worker.fd()
    }

    pub fn events(&self) -> Vec<VersionEvent> {
        self.worker.events()
    }
}

pub struct Worker {
    worker: skyward_sys::worker::Worker<Event>,
    /// 手指把卡滑回去时置位：下载与写入看到它就收手。
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// 正在下的这一份：已下多少字节、总共多少（不知道时为 None）。
    bytes: std::sync::Arc<std::sync::Mutex<(u64, Option<u64>)>>,
}

impl Worker {
    pub fn start(task: Task) -> Result<Worker> {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let bytes = std::sync::Arc::new(std::sync::Mutex::new((0u64, None)));
        let (stop_in, bytes_in) = (stop.clone(), bytes.clone());
        let worker = skyward_sys::worker::Worker::start("root", move |tell| {
            // 整件事都要有个结果：网络卡住时不能让页面一直停在「正在写入」。
            // 写 Root 本身是下载 + 打补丁 + 写分区，给七分钟；超时就报错。
            const DEADLINE: Duration = Duration::from_secs(420);
            let inner = tell.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            let spawned = std::thread::Builder::new().name("root-io".into()).spawn(move || {
                let stage = |stage| {
                    inner.tell(Event::Stage(stage));
                };
                let fetched = inner.clone();
                let net = Net { stop: stop_in, bytes: bytes_in };
                let end = match task {
                    Task::InstallKernelSu { kmi, slot, servers, module } => install_latest(
                        &kmi,
                        slot,
                        servers,
                        module.as_deref(),
                        &stage,
                        &|version| {
                            fetched.tell(Event::Fetched(version));
                        },
                        &net,
                    ),
                    Task::RestoreKernelSu => restore(&stage),
                    Task::InstallMagisk { slot, servers } => install_magisk(slot, servers, &stage, &net),
                    Task::RestoreMagisk => restore_magisk(&stage),
                };
                let _ = tx.send(end);
            });
            let end = match spawned {
                Ok(_) => match rx.recv_timeout(DEADLINE) {
                    Ok(end) => end,
                    Err(_) => {
                        let _ = skyward_sys::kmsg("root: the write did not come back in time");
                        KernelSuEnd::Failed(format!("下载或写入没有在 {} 秒内完成，已放弃", DEADLINE.as_secs()))
                    }
                },
                Err(e) => KernelSuEnd::Failed(format!("{e}")),
            };
            tell.tell(Event::Ended(end));
        })?;
        Ok(Worker { worker, stop, bytes })
    }

    /// 让它收手：下载中会立刻结束，已写出去的不会回滚。
    pub fn stop(&self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// 正在下的这一份：已下字节与总字节。
    pub fn bytes(&self) -> (u64, Option<u64>) {
        *self.bytes.lock().expect("the byte count is not held by a thread that panicked")
    }

    pub fn fd(&self) -> std::os::fd::RawFd {
        self.worker.fd()
    }

    pub fn events(&self) -> Vec<Event> {
        self.worker.events()
    }
}

/// What the phone and KernelSU's payloads say, read as the page opens.
pub fn status(info: &skyward_info::DeviceInfo) -> Result<KernelSuStatus, String> {
    let suffix = info.slot.map(Slot::suffix).unwrap_or("");
    let targets = ShownTargets::read(suffix);
    let kmis: Vec<KmiRow> = skyward_root::KMIS
        .iter()
        .map(|k| KmiRow {
            id: k.id.into(),
            android: k.android,
            kernel: k.kernel.into(),
            target: targets.of(k.target.name()).into(),
        })
        .collect();
    let default = skyward_root::default(info.android.as_deref(), &info.kernel)
        .and_then(|k| kmis.iter().position(|row| row.id == k.id));
    let record = fs::read_to_string(Path::new(ROOT_DIR).join(KERNELSU_MANIFEST)).ok();
    let installed = match &record {
        Some(text) => field(text, "restored").as_deref() != Some("1"),
        None => Path::new("/data/adb/ksu").exists(),
    };
    let has_backup = record.as_deref().is_some_and(backup_present);
    let backup_target = record.as_deref().and_then(recorded_target);
    let version = installed.then(read_version).flatten();
    Ok(KernelSuStatus {
        installed,
        has_backup,
        backup_target,
        version,
        unlocked: info.boot.unlocked.unwrap_or(false),
        slot: info.slot.map(|s| s.letter().to_string()),
        default,
        kmis,
    })
}

fn read_version() -> Option<String> {
    let direct = fs::read_to_string("/data/adb/ksu/version").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let semver = direct
        .or_else(|| program_version("/data/adb/ksud", "ksud "))
        .or_else(|| program_version("/data/adb/ksu/bin/ksud", "ksud "))
        .or_else(|| fs::read_to_string(Path::new(ROOT_DIR).join(KERNELSU_MANIFEST)).ok().and_then(|text| field(&text, "version")));
    detailed(semver, package_version_code("me.weishu.kernelsu"))
}

/// The manager's version code, as Android keeps it in `packages.list`, is
/// the five or six digit number the KernelSU and Magisk releases are known
/// by beside their dotted version.
fn package_version_code(package: &str) -> Option<String> {
    let text = fs::read_to_string("/data/system/packages.list").ok()?;
    let line = text.lines().find(|line| line.split_whitespace().next() == Some(package))?;
    let fields: Vec<&str> = line.split_whitespace().collect();
    fields.get(7).filter(|code| (5..=6).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_digit())).map(|code| (*code).to_string())
}

fn detailed(semver: Option<String>, code: Option<String>) -> Option<String> {
    match (semver, code) {
        (Some(semver), Some(code)) => Some(format!("{semver} ({code})")),
        (Some(semver), None) => Some(semver),
        (None, code) => code,
    }
}

fn program_version(path: &str, prefix: &str) -> Option<String> {
    let out = Command::new(path).arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next()?.trim();
    Some(line.strip_prefix(prefix).unwrap_or(line).to_string())
}

/// What the phone and Magisk's APK say, read as the page opens.
pub fn magisk_status(info: &skyward_info::DeviceInfo) -> Result<MagiskStatus, String> {
    let suffix = info.slot.map(Slot::suffix).unwrap_or("");
    let target = magisk_target_name(suffix).to_string();
    let record = fs::read_to_string(Path::new(ROOT_DIR).join(MAGISK_MANIFEST)).ok();
    let installed = match &record {
        Some(text) => field(text, "restored").as_deref() != Some("1"),
        None => Path::new("/data/adb/magisk").exists(),
    };
    let has_backup = record.as_deref().is_some_and(backup_present);
    let backup_target = record.as_deref().and_then(recorded_target);
    let version = if installed {
        read_magisk_version().or_else(|| record.as_ref().and_then(|text| field(text, "version")))
    } else {
        None
    };
    Ok(MagiskStatus {
        installed,
        has_backup,
        backup_target,
        version,
        unlocked: info.boot.unlocked.unwrap_or(false),
        slot: info.slot.map(|s| s.letter().to_string()),
        target: target.to_string(),
    })
}

/// Which root manager is on the phone, as an install that writes boot has
/// to know it before patching the new slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstalledRoot {
    KernelSu,
    Magisk,
}

pub fn installed_root() -> Option<InstalledRoot> {
    let kernel_su = fs::read_to_string(Path::new(ROOT_DIR).join(KERNELSU_MANIFEST)).ok();
    let kernel_su = match &kernel_su {
        Some(text) => field(text, "restored").as_deref() != Some("1"),
        None => Path::new("/data/adb/ksu").exists(),
    };
    let magisk = fs::read_to_string(Path::new(ROOT_DIR).join(MAGISK_MANIFEST)).ok();
    let magisk = match &magisk {
        Some(text) => field(text, "restored").as_deref() != Some("1"),
        None => Path::new("/data/adb/magisk").exists(),
    };
    match (kernel_su, magisk) {
        (true, _) => Some(InstalledRoot::KernelSu),
        (false, true) => Some(InstalledRoot::Magisk),
        (false, false) => None,
    }
}

fn read_magisk_version() -> Option<String> {
    let semver = fs::read_to_string("/data/adb/magisk/util_functions.sh")
        .ok()
        .and_then(|text| field(&text, "MAGISK_VER"))
        .map(|version| version.trim_matches(|c| c == '\'' || c == '"').to_string())
        .or_else(|| program_version("/data/adb/magisk/magisk", ""))
        .or_else(|| program_version("/data/adb/magisk/magisk64", ""));
    detailed(semver, package_version_code("com.topjohnwu.magisk"))
}

/// The KernelSU KMI to patch with when the page has not chosen one: the
/// last one written, or the kernel's own default.
pub fn auto_kmi(info: &skyward_info::DeviceInfo) -> Option<String> {
    fs::read_to_string(Path::new(ROOT_DIR).join(KERNELSU_MANIFEST))
        .ok()
        .and_then(|text| field(&text, "kmi"))
        .filter(|id| skyward_root::find(id).is_some())
        .or_else(|| skyward_root::default(info.android.as_deref(), &info.kernel).map(|k| k.id.to_string()))
}

/// The module the last install wrote, when it was one the user chose:
/// auto-patching the same kernel needs the same module, and one that has
/// gone missing is a reason not to patch, not to fall back to another.
pub fn auto_module() -> Result<Option<String>, String> {
    let Ok(text) = fs::read_to_string(Path::new(ROOT_DIR).join(KERNELSU_MANIFEST)) else { return Ok(None) };
    match field(&text, "module").as_deref() {
        None | Some("builtin") => Ok(None),
        Some(path) if Path::new(path).is_file() => Ok(Some(path.to_string())),
        Some(path) => Err(format!("the KernelSU module {path} is gone")),
    }
}

fn field(text: &str, name: &str) -> Option<String> {
    let key = format!("{name}=");
    text.lines().find_map(|line| line.trim().strip_prefix(&key).map(|v| v.trim().to_string()))
}

/// The start of a server's answer, for a sentence that says what went
/// wrong without filling the page with it.
fn clip(text: &str) -> String {
    let start: String = text.chars().take(200).collect();
    match start.len() == text.len() {
        true => start,
        false => format!("{start}…"),
    }
}

/// Whether the record's backup is still where it was written, and was
/// taken from a stock image.
fn backup_present(text: &str) -> bool {
    field(text, "stock").as_deref() != Some("0") && field(text, "backup").is_some_and(|path| Path::new(&path).is_file())
}

/// The backup a record for this target and slot still holds, whole.
fn usable_backup(record: Option<&str>, target: &str, suffix: &str) -> Option<(PathBuf, String)> {
    let text = record?;
    if field(text, "target").as_deref() != Some(target) || field(text, "slot").as_deref() != Some(suffix) {
        return None;
    }
    if field(text, "stock").as_deref() == Some("0") {
        return None;
    }
    let backup = PathBuf::from(field(text, "backup")?);
    if !backup.is_file() {
        return None;
    }
    let sum = sha256(&backup).ok()?;
    if let Some(want) = field(text, "sha256")
        && want != sum
    {
        return None;
    }
    Some((backup, sum))
}

/// The partition a record's backup came from, as the restore row names it.
fn recorded_target(text: &str) -> Option<String> {
    let target = field(text, "target")?;
    Some(format!("{target}{}", field(text, "slot").unwrap_or_default()))
}

/// 一次下载要用的两样东西：手指滑回去时置的停止标志，以及给页面看的字节数。
struct Net {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    bytes: std::sync::Arc<std::sync::Mutex<(u64, Option<u64>)>>,
}

impl Net {
    fn stopped(&self) -> bool {
        self.stop.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn set(&self, done: u64, total: Option<u64>) {
        if let Ok(mut held) = self.bytes.lock() {
            *held = (done, total);
        }
    }
}

struct Payload {
    module: Vec<u8>,
    ksud: Vec<u8>,
    manager: Vec<u8>,
    version: String,
}



fn download_latest(kmi_id: &str, servers: Servers, stage: &dyn Fn(KernelSuStage), net: &Net) -> Result<Payload> {
    stage(KernelSuStage::Preparing);
    let agent = http::agent(servers.clone());
    // 整次请求给一个上限：网络卡住时宁可报错，也不要让页面停在「正在写入」。
    let _ = skyward_sys::kmsg("root: asking GitHub for the KernelSU release");
    let mut response = agent
        .get(skyward_root::KERNELSU_LATEST)
        .config()
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", format!("Skyward/{}", skyward_info::VERSION))
        .call()
        .with_context(|| format!("ask {}", skyward_root::KERNELSU_LATEST))?;
    let status = response.status();
    let body = response.body_mut().read_to_string().with_context(|| format!("read the answer of {}", skyward_root::KERNELSU_LATEST))?;
    ensure!(status.is_success(), "GitHub answered {status}: {}", clip(&body));
    let _ = skyward_sys::kmsg("root: GitHub answered the KernelSU release");
    let release: Value = serde_json::from_str(&body).context("read GitHub's KernelSU release")?;
    let tag = release.get("tag_name").and_then(Value::as_str).context("the KernelSU release names no tag")?.to_string();
    let assets = release.get("assets").and_then(Value::as_array).context("the KernelSU release names no assets")?;
    let named = |name: &str| assets.iter().find(|a| a.get("name").and_then(Value::as_str) == Some(name));
    let ksud = named("ksud-aarch64-linux-android").context("the KernelSU release names no aarch64 ksud")?;
    let ko_name = format!("lkm-aarch64-{kmi_id}_kernelsu.ko");
    let ko = named(&ko_name).with_context(|| format!("the KernelSU release has no {ko_name}"))?;
    let manager = assets
        .iter()
        .find(|a| {
            a.get("name").and_then(Value::as_str).is_some_and(|name| {
                name.starts_with("KernelSU_") && name.ends_with("-release.apk") && !name.contains("spoofed")
            })
        })
        .context("the KernelSU release names no manager APK")?;
    let servers = servers.clone();
    let ksud = fetch(&agent, ksud, stage, net, &servers).context("download KernelSU's ksud")?;
    let module = fetch(&agent, ko, stage, net, &servers).context("download KernelSU's module")?;
    let manager = fetch(&agent, manager, stage, net, &servers).context("download KernelSU's manager")?;
    Ok(Payload { module, ksud, manager, version: tag })
}

fn fetch(agent: &ureq::Agent, asset: &Value, stage: &dyn Fn(KernelSuStage), net: &Net, servers: &Servers) -> Result<Vec<u8>> {
    let url = asset.get("browser_download_url").and_then(Value::as_str).context("the asset names no address")?;
    let name = asset.get("name").and_then(Value::as_str).unwrap_or(url);
    stage(KernelSuStage::Preparing);
    // 与「在线更新本体」走同一套：github 的链接再套上镜像前缀，先并行测速，
    // 快的那一个排最前（2026-11-26）。
    let mut addresses = vec![url.to_string()];
    for mirror in skyward_net::github::mirrors() {
        addresses.push(format!("{mirror}{url}"));
    }
    let addresses = crate::updates::by_speed(servers, addresses);
    let mut tried = Vec::new();
    for address in addresses {
        if net.stopped() {
            bail!("已取消");
        }
        let _ = skyward_sys::kmsg(&format!("root: downloading {name} · {}", host_of(&address)));
        match download(agent, &address, name, net) {
            Ok(bytes) => {
                if let Some(digest) = asset.get("digest").and_then(Value::as_str)
                    && let Some(want) = digest.strip_prefix("sha256:")
                {
                    let got = hex::encode(Sha256::digest(&bytes));
                    if got != want {
                        tried.push(format!("{}: 校验不符", host_of(&address)));
                        continue;
                    }
                }
                let _ = skyward_sys::kmsg(&format!("root: {name} downloaded, {} bytes", bytes.len()));
                return Ok(bytes);
            }
            Err(e) => {
                let _ = skyward_sys::kmsg(&format!("root: {name} from {} failed: {e:#}", host_of(&address)));
                tried.push(format!("{}: {e:#}", host_of(&address)));
            }
        }
    }
    bail!("{name} 没能下载：{}", tried.join("; "))
}

/// 地址里的主机名，日志里说清楚走的是直连还是哪一个镜像。
fn host_of(address: &str) -> &str {
    address.split("//").nth(1).and_then(|rest| rest.split('/').next()).unwrap_or(address)
}

/// 一份文件：整体有上限，边下边报字节数，手指滑回去就收手。
fn download(agent: &ureq::Agent, address: &str, name: &str, net: &Net) -> Result<Vec<u8>> {
    let mut response = agent
        .get(address)
        .config()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .header("Accept", "application/octet-stream")
        .header("User-Agent", format!("Skyward/{}", skyward_info::VERSION))
        .call()
        .with_context(|| format!("download {name}"))?;
    let status = response.status();
    ensure!(status.is_success(), "downloading {name} answered {status}");
    let total = response.body().content_length();
    net.set(0, total);
    let mut bytes = Vec::new();
    let mut reader = response.body_mut().with_config().limit(u64::MAX).reader();
    let mut buffer = [0u8; 64 * 1024];
    let mut last = Instant::now();
    loop {
        if net.stopped() {
            bail!("已取消");
        }
        let read = reader.read(&mut buffer).with_context(|| format!("read {name}"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        net.set(bytes.len() as u64, total);
        if last.elapsed() >= Duration::from_secs(2) {
            last = Instant::now();
            match total {
                Some(total) => {
                    let _ = skyward_sys::kmsg(&format!("root: {name} {}/{} KiB", bytes.len() / 1024, total / 1024));
                }
                None => {
                    let _ = skyward_sys::kmsg(&format!("root: {name} {} KiB", bytes.len() / 1024));
                }
            }
        }
    }
    Ok(bytes)
}

fn install_latest(
    kmi_id: &str,
    slot: Option<Slot>,
    servers: Servers,
    module: Option<&str>,
    stage: &dyn Fn(KernelSuStage),
    fetched: &dyn Fn(String),
    net: &Net,
) -> KernelSuEnd {
    let run = || -> Result<KernelSuEnd> {
        let kmi = skyward_root::find(kmi_id).with_context(|| format!("KernelSU has no KMI {kmi_id}"))?;
        let payload = download_latest(kmi_id, servers, stage, net)?;
        if let Some(installed) = read_version()
            && matches!(skyward_ui::app::is_newer(Some(&installed), Some(&payload.version)), Some(false))
        {
            bail!("装着的是 {installed}，在线是 {}：只升不降，没写", payload.version);
        }
        fetched(payload.version.clone());
        install_with_payload(kmi, payload, slot, module, stage)
    };
    run().unwrap_or_else(|e| KernelSuEnd::Failed(format!("{e:#}")))
}

fn install_with_payload(
    kmi: &skyward_root::Kmi,
    mut payload: Payload,
    slot: Option<Slot>,
    module: Option<&str>,
    stage: &dyn Fn(KernelSuStage),
) -> Result<KernelSuEnd> {
    ensure!(installed_root() != Some(InstalledRoot::Magisk), "Magisk is installed; restore its boot image before installing KernelSU");
    ensure!(Path::new(skyward_crypto::data::SDCARD).is_dir(), "internal storage is not open");
    if let Some(path) = module {
        let bytes = fs::read(path).with_context(|| format!("read the KernelSU module {path}"))?;
        ensure!(
            bytes.len() > 20
                && bytes.starts_with(b"\x7fELF")
                && bytes[4] == 2
                && bytes[5] == 1
                && u16::from_le_bytes([bytes[18], bytes[19]]) == 183,
            "{path} is not an aarch64 kernel module"
        );
        payload.module = bytes;
    }
    stage(KernelSuStage::Preparing);
    let suffix = suffix_of(slot)?;
    let (target, partition) = target_partition(kmi, suffix)?;
    let partition_size = skyward_block::size_bytes(&partition)?;
    fs::create_dir_all(ROOT_DIR).context("make the KernelSU backup folder")?;
    fs::remove_dir_all(KERNELSU_WORK).ok();
    fs::create_dir_all(KERNELSU_WORK).context("make the temporary KernelSU folder")?;
    stage(KernelSuStage::BackingUp);
    let (backup, sum, stock) = stock_backup(&Path::new(ROOT_DIR).join(KERNELSU_MANIFEST), &partition, target, suffix, "stock", kernelsu_patched)?;
    let ksud = Path::new(KERNELSU_WORK).join("ksud");
    write_asset(&ksud, &payload.ksud)?;
    let ko = Path::new(KERNELSU_WORK).join(format!("{}.ko", kmi.id));
    write_asset(&ko, &payload.module)?;
    let patched = Path::new(KERNELSU_WORK).join("patched.img");
    stage(KernelSuStage::Patching);
    let out = Command::new(&ksud)
        .arg("boot-patch")
        .arg("-b")
        .arg(&partition)
        .arg("-m")
        .arg(&ko)
        .arg("-o")
        .arg(KERNELSU_WORK)
        .arg("--out-name")
        .arg("patched.img")
        .arg("--kmi")
        .arg(kmi.id)
        .output()
        .context("run KernelSU's ksud")?;
    ensure!(
        out.status.success(),
        "ksud failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    stage(KernelSuStage::Checking);
    let len = fs::metadata(&patched).with_context(|| format!("read {}", patched.display()))?.len();
    ensure!(len <= partition_size, "the patched image is {len} bytes; {partition:?} holds {partition_size}");
    let mut file = File::open(&patched).with_context(|| format!("open {}", patched.display()))?;
    let image = skyward_install::image::open(&mut file, len).context("read the patched boot image")?;
    ensure!(matches!(image.kind, ImageKind::Boot { .. }), "the patch did not make a boot image");
    let manager_dir = Path::new(skyward_crypto::data::SDCARD).join("Download/Skyward");
    fs::create_dir_all(&manager_dir).context("make the folder for the KernelSU manager")?;
    let manager = manager_dir.join(skyward_root::MANAGER_NAME);
    write_file(&manager, &payload.manager, 0o644)?;
    stage(KernelSuStage::Writing);
    let start = Instant::now();
    let mut last = Instant::now();
    skyward_install::image::flash(&mut file, len, std::slice::from_ref(&partition), &mut |_, _| {
        if last.elapsed() >= Duration::from_millis(200) {
            last = Instant::now();
            stage(KernelSuStage::Writing);
        }
    })
    .with_context(|| format!("write {}", partition.display()))?;
    let manifest = format!(
        "version={}\nmodule={}\nkmi={}\ntarget={}\nslot={}\nbackup={}\nmanager={}\nsha256={}\nstock={}\nwritten={}\n",
        payload.version,
        module.unwrap_or("builtin"),
        kmi.id,
        target,
        suffix,
        backup.display(),
        manager.display(),
        sum,
        u8::from(stock),
        start.elapsed().as_secs()
    );
    fs::write(Path::new(ROOT_DIR).join(KERNELSU_MANIFEST), manifest).context("write the KernelSU record")?;
    fs::remove_dir_all(KERNELSU_WORK).ok();
    Ok(KernelSuEnd::Installed {
        target: format!("{target}{suffix}"),
        backup: backup.display().to_string(),
        manager: manager.display().to_string(),
    })
}

fn restore(stage: &dyn Fn(KernelSuStage)) -> KernelSuEnd {
    restore_inner(stage).unwrap_or_else(|e| KernelSuEnd::Failed(format!("{e:#}")))
}

fn restore_inner(stage: &dyn Fn(KernelSuStage)) -> Result<KernelSuEnd> {
    let text = fs::read_to_string(Path::new(ROOT_DIR).join(KERNELSU_MANIFEST)).context("no KernelSU backup record")?;
    ensure!(field(&text, "stock").as_deref() != Some("0"), "the KernelSU record's backup is not a stock image");
    let target = field(&text, "target").context("the KernelSU record names no target")?;
    let slot = field(&text, "slot").unwrap_or_default();
    let backup = PathBuf::from(field(&text, "backup").context("the KernelSU record names no backup")?);
    ensure!(backup.is_file(), "the KernelSU backup {} is gone", backup.display());
    if let Some(want) = field(&text, "sha256") {
        let got = sha256(&backup)?;
        ensure!(got == want, "the KernelSU backup does not read back as it was written");
    }
    stage(KernelSuStage::Writing);
    let partition = Path::new(skyward_block::BY_NAME).join(format!("{target}{slot}"));
    let len = fs::metadata(&backup)?.len();
    let mut file = File::open(&backup)?;
    skyward_install::image::flash(&mut file, len, std::slice::from_ref(&partition), &mut |_, _| {})
        .with_context(|| format!("write {}", partition.display()))?;
    let mut text = text;
    if field(&text, "restored").is_none() {
        text.push_str("restored=1\n");
    }
    fs::write(Path::new(ROOT_DIR).join(KERNELSU_MANIFEST), text).context("record the restored boot image")?;
    Ok(KernelSuEnd::Restored { target: format!("{target}{slot}") })
}

fn install_magisk(slot: Option<Slot>, servers: Servers, stage: &dyn Fn(KernelSuStage), net: &Net) -> KernelSuEnd {
    install_magisk_inner(slot, servers, stage, net).unwrap_or_else(|e| KernelSuEnd::Failed(format!("{e:#}")))
}

/// 从 GitHub 取最新的 Magisk 发行里的 APK：补丁器、程序与脚本都在里面。
fn download_magisk(servers: Servers, stage: &dyn Fn(KernelSuStage), net: &Net) -> Result<(Vec<u8>, String)> {
    stage(KernelSuStage::Preparing);
    let agent = http::agent(servers.clone());
    let _ = skyward_sys::kmsg("root: asking GitHub for the Magisk release");
    let mut response = agent
        .get(skyward_root::MAGISK_LATEST)
        .config()
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", format!("Skyward/{}", skyward_info::VERSION))
        .call()
        .with_context(|| format!("ask {}", skyward_root::MAGISK_LATEST))?;
    let status = response.status();
    let body = response.body_mut().read_to_string().with_context(|| format!("read the answer of {}", skyward_root::MAGISK_LATEST))?;
    ensure!(status.is_success(), "GitHub answered {status}: {}", clip(&body));
    let release: Value = serde_json::from_str(&body).context("read GitHub's Magisk release")?;
    let tag = release.get("tag_name").and_then(Value::as_str).context("the Magisk release names no tag")?.to_string();
    let assets = release.get("assets").and_then(Value::as_array).context("the Magisk release names no assets")?;
    let apk = assets
        .iter()
        .find(|a| {
            a.get("name").and_then(Value::as_str).is_some_and(|name| name.starts_with("Magisk-") && name.ends_with(".apk"))
        })
        .context("the Magisk release names no APK")?;
    let bytes = fetch(&agent, apk, stage, net, &servers).context("download Magisk's APK")?;
    ensure!(bytes.starts_with(b"PK"), "what was downloaded is not an APK");
    Ok((bytes, tag))
}

fn install_magisk_inner(slot: Option<Slot>, servers: Servers, stage: &dyn Fn(KernelSuStage), net: &Net) -> Result<KernelSuEnd> {
    ensure!(installed_root() != Some(InstalledRoot::KernelSu), "KernelSU is installed; restore its boot image before installing Magisk");
    ensure!(Path::new(skyward_crypto::data::SDCARD).is_dir(), "internal storage is not open");
    stage(KernelSuStage::Preparing);
    let suffix = suffix_of(slot)?;
    let (target, partition) = magisk_target(suffix)?;
    let partition_size = skyward_block::size_bytes(&partition)?;
    fs::create_dir_all(ROOT_DIR).context("make the Magisk backup folder")?;
    fs::remove_dir_all(MAGISK_WORK).ok();
    fs::create_dir_all(MAGISK_WORK).context("make the temporary Magisk folder")?;
    stage(KernelSuStage::BackingUp);
    let (backup, sum, stock) = stock_backup(&Path::new(ROOT_DIR).join(MAGISK_MANIFEST), &partition, target, suffix, "magisk-stock", magisk_patched)?;
    let (magisk_apk, magisk_version) = download_magisk(servers, stage, net)?;
    if let Some(installed) = read_magisk_version()
        && matches!(skyward_ui::app::is_newer(Some(&installed), Some(&magisk_version)), Some(false))
    {
        bail!("装着的是 {installed}，在线是 {magisk_version}：只升不降，没写");
    }
    let apk = Cursor::new(&magisk_apk);
    let mut zip = zip::ZipArchive::new(apk).context("read Magisk's APK")?;
    for (entry, name, mode) in [
        ("lib/arm64-v8a/libmagiskboot.so", "magiskboot", 0o755),
        ("lib/arm64-v8a/libmagisk.so", "magisk", 0o755),
        ("lib/arm64-v8a/libmagiskinit.so", "magiskinit", 0o755),
        ("lib/arm64-v8a/libinit-ld.so", "init-ld", 0o755),
        ("assets/boot_patch.sh", "boot_patch.sh", 0o755),
        ("assets/util_functions.sh", "util_functions.sh", 0o755),
    ] {
        extract(&mut zip, entry, &Path::new(MAGISK_WORK).join(name), mode)
            .with_context(|| format!("take {entry} out of Magisk's APK"))?;
    }
    let magisk = Path::new(MAGISK_WORK).join("magisk");
    let preinit = Command::new(&magisk)
        .arg("--preinit-device")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let patched = Path::new(MAGISK_WORK).join("new-boot.img");
    stage(KernelSuStage::Patching);
    let out = Command::new("/system/bin/sh")
        .arg("boot_patch.sh")
        .arg(&partition)
        .current_dir(MAGISK_WORK)
        .env("BOOTMODE", "true")
        .env("OUTFD", "1")
        .env("PATH", format!("{}:/system/bin:/system/xbin", MAGISK_WORK))
        .env("PREINITDEVICE", &preinit)
        .output()
        .context("run Magisk's boot_patch.sh")?;
    ensure!(
        out.status.success(),
        "Magisk's boot patch failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    stage(KernelSuStage::Checking);
    let len = fs::metadata(&patched).with_context(|| format!("read {}", patched.display()))?.len();
    ensure!(len <= partition_size, "the patched image is {len} bytes; {partition:?} holds {partition_size}");
    let mut file = File::open(&patched).with_context(|| format!("open {}", patched.display()))?;
    let image = skyward_install::image::open(&mut file, len).context("read the patched boot image")?;
    ensure!(matches!(image.kind, ImageKind::Boot { .. }), "the patch did not make a boot image");
    let manager_dir = Path::new(skyward_crypto::data::SDCARD).join("Download/Skyward");
    fs::create_dir_all(&manager_dir).context("make the folder for the Magisk manager")?;
    let manager = manager_dir.join(skyward_root::MAGISK_MANAGER_NAME);
    write_file(&manager, &magisk_apk, 0o644)?;
    stage(KernelSuStage::Writing);
    let start = Instant::now();
    let mut last = Instant::now();
    skyward_install::image::flash(&mut file, len, std::slice::from_ref(&partition), &mut |_, _| {
        if last.elapsed() >= Duration::from_millis(200) {
            last = Instant::now();
            stage(KernelSuStage::Writing);
        }
    })
    .with_context(|| format!("write {}", partition.display()))?;
    let manifest = format!(
        "version={}\ntarget={}\nslot={}\nbackup={}\nmanager={}\nsha256={}\nstock={}\nwritten={}\n",
        magisk_version,
        target,
        suffix,
        backup.display(),
        manager.display(),
        sum,
        u8::from(stock),
        start.elapsed().as_secs()
    );
    fs::write(Path::new(ROOT_DIR).join(MAGISK_MANIFEST), manifest).context("write the Magisk record")?;
    fs::remove_dir_all(MAGISK_WORK).ok();
    Ok(KernelSuEnd::Installed {
        target: format!("{target}{suffix}"),
        backup: backup.display().to_string(),
        manager: manager.display().to_string(),
    })
}

fn restore_magisk(stage: &dyn Fn(KernelSuStage)) -> KernelSuEnd {
    restore_magisk_inner(stage).unwrap_or_else(|e| KernelSuEnd::Failed(format!("{e:#}")))
}

fn restore_magisk_inner(stage: &dyn Fn(KernelSuStage)) -> Result<KernelSuEnd> {
    let text = fs::read_to_string(Path::new(ROOT_DIR).join(MAGISK_MANIFEST)).context("no Magisk backup record")?;
    ensure!(field(&text, "stock").as_deref() != Some("0"), "the Magisk record's backup is not a stock image");
    let target = field(&text, "target").context("the Magisk record names no target")?;
    let slot = field(&text, "slot").unwrap_or_default();
    let backup = PathBuf::from(field(&text, "backup").context("the Magisk record names no backup")?);
    ensure!(backup.is_file(), "the Magisk backup {} is gone", backup.display());
    if let Some(want) = field(&text, "sha256") {
        let got = sha256(&backup)?;
        ensure!(got == want, "the Magisk backup does not read back as it was written");
    }
    stage(KernelSuStage::Writing);
    let partition = Path::new(skyward_block::BY_NAME).join(format!("{target}{slot}"));
    let len = fs::metadata(&backup)?.len();
    let mut file = File::open(&backup)?;
    skyward_install::image::flash(&mut file, len, std::slice::from_ref(&partition), &mut |_, _| {})
        .with_context(|| format!("write {}", partition.display()))?;
    let mut text = text;
    if field(&text, "restored").is_none() {
        text.push_str("restored=1\n");
    }
    fs::write(Path::new(ROOT_DIR).join(MAGISK_MANIFEST), text).context("record the restored boot image")?;
    Ok(KernelSuEnd::Restored { target: format!("{target}{slot}") })
}

fn magisk_target(suffix: &str) -> Result<(&'static str, PathBuf)> {
    magisk_partition(suffix).with_context(|| format!("neither init_boot nor boot is here for slot {suffix:?}"))
}

fn magisk_target_name(suffix: &str) -> &'static str {
    magisk_partition(suffix).map_or("boot", |(name, _)| name)
}

/// Magisk patches the ramdisk, so it goes where this build put one:
/// init_boot on the phones that moved it there, boot on the ones that did
/// not.
fn magisk_partition(suffix: &str) -> Option<(&'static str, PathBuf)> {
    let init_boot = Path::new(skyward_block::BY_NAME).join(format!("init_boot{suffix}"));
    let boot = Path::new(skyward_block::BY_NAME).join(format!("boot{suffix}"));
    if init_boot.exists() && (has_ramdisk(&init_boot).unwrap_or(false) || !boot.exists()) {
        return Some(("init_boot", init_boot));
    }
    if boot.exists() {
        return Some(("boot", boot));
    }
    init_boot.exists().then_some(("init_boot", init_boot))
}

fn suffix_of(slot: Option<Slot>) -> Result<&'static str> {
    match slot {
        Some(slot) => Ok(slot.suffix()),
        None => Ok(skyward_vab::current_slot()?.map(Slot::suffix).unwrap_or("")),
    }
}

fn target_partition(kmi: &skyward_root::Kmi, suffix: &str) -> Result<(&'static str, PathBuf)> {
    let preferred = target_for(kmi, suffix);
    let first = Path::new(skyward_block::BY_NAME).join(format!("{preferred}{suffix}"));
    let other = match preferred {
        "boot" => "init_boot",
        _ => "boot",
    };
    let second = Path::new(skyward_block::BY_NAME).join(format!("{other}{suffix}"));
    if first.exists() {
        // The kernel version picks the partition to offer, but a build that
        // put its ramdisk in the other one is the phone's own layout to say.
        if second.exists() && !has_ramdisk(&first)? && has_ramdisk(&second)? {
            return Ok((other, second));
        }
        return Ok((preferred, first));
    }
    ensure!(second.exists(), "neither {preferred} nor {other} is here for slot {suffix:?}");
    Ok((other, second))
}

/// The partition a KMI's module goes into on this phone: the one its kernel
/// version asks for, or the other where this build has only that.
fn target_for(kmi: &skyward_root::Kmi, suffix: &str) -> &'static str {
    let preferred = kmi.target.name();
    if Path::new(skyward_block::BY_NAME).join(format!("{preferred}{suffix}")).exists() {
        return preferred;
    }
    match preferred {
        "boot" => "init_boot",
        _ => "boot",
    }
}

/// What the KMI page and install write: the partition each KMI's module
/// goes into, read once for the slot, as [`target_partition`] chooses it.
struct ShownTargets {
    boot: &'static str,
    init_boot: &'static str,
}

impl ShownTargets {
    fn read(suffix: &str) -> ShownTargets {
        let boot = Path::new(skyward_block::BY_NAME).join(format!("boot{suffix}"));
        let init_boot = Path::new(skyward_block::BY_NAME).join(format!("init_boot{suffix}"));
        let ramdisk = |path: &Path| path.exists() && has_ramdisk(path).unwrap_or(false);
        let pick = |preferred: &'static str| -> &'static str {
            let (first, first_path, second, second_path) = match preferred {
                "boot" => ("boot", &boot, "init_boot", &init_boot),
                _ => ("init_boot", &init_boot, "boot", &boot),
            };
            if first_path.exists() {
                if second_path.exists() && !ramdisk(first_path) && ramdisk(second_path) {
                    return second;
                }
                return first;
            }
            if second_path.exists() { second } else { first }
        };
        ShownTargets { boot: pick("boot"), init_boot: pick("init_boot") }
    }

    fn of(&self, preferred: &str) -> &'static str {
        match preferred {
            "boot" => self.boot,
            _ => self.init_boot,
        }
    }
}

/// Whether a boot image carries the ramdisk KernelSU has to patch. A 5.10
/// phone built with init_boot keeps only the kernel in boot.
fn has_ramdisk(path: &Path) -> Result<bool> {
    let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut head = vec![0u8; 4096];
    let n = file.read(&mut head).with_context(|| format!("read {}", path.display()))?;
    head.truncate(n);
    if !head.starts_with(b"ANDROID!") {
        return Ok(false);
    }
    Ok(matches!(skyward_install::image::detect(&mut file, &head)?, ImageKind::Boot { ramdisk, .. } if ramdisk != Ramdisk::None))
}

/// The stock image a restore writes back: the record's own backup where the
/// partition already carries the manager, as that backup is the stock image
/// the patched one was made from; otherwise a copy of the partition, which
/// is stock. A copy taken while the partition was patched is marked not
/// stock, so the page does not offer to restore it.
fn stock_backup(
    manifest: &Path,
    partition: &Path,
    target: &str,
    suffix: &str,
    stem: &str,
    patched: fn(&Path) -> Result<bool>,
) -> Result<(PathBuf, String, bool)> {
    let record = fs::read_to_string(manifest).ok();
    let patched = patched(partition).unwrap_or(false);
    if patched
        && let Some((backup, sum)) = usable_backup(record.as_deref(), target, suffix)
    {
        return Ok((backup, sum, true));
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let backup = Path::new(ROOT_DIR).join(format!("{stem}-{}-{target}-{stamp}.img", suffix.trim_start_matches('_')));
    let sum = copy_partition(partition, &backup)?;
    Ok((backup, sum, !patched))
}

/// Whether the partition already carries KernelSU's module in its ramdisk.
fn kernelsu_patched(partition: &Path) -> Result<bool> {
    let mut file = File::open(partition).with_context(|| format!("open {}", partition.display()))?;
    skyward_install::image::ramdisk_holds(&mut file, |name| name == "kernelsu.ko")
}

/// Whether the partition already carries Magisk's patched ramdisk.
fn magisk_patched(partition: &Path) -> Result<bool> {
    let mut file = File::open(partition).with_context(|| format!("open {}", partition.display()))?;
    skyward_install::image::ramdisk_holds(&mut file, |name| name == ".backup/.magisk" || name == "overlay.d/sbin/magisk.xz")
}

fn copy_partition(partition: &Path, backup: &Path) -> Result<String> {    let size = skyward_block::size_bytes(partition)?;
    let mut input = File::open(partition).with_context(|| format!("open {}", partition.display()))?;
    let mut output = File::create(backup).with_context(|| format!("make {}", backup.display()))?;
    let mut buf = vec![0u8; 1 << 20];
    let mut sum = Sha256::new();
    let mut left = size;
    while left > 0 {
        let n = left.min(buf.len() as u64) as usize;
        input.read_exact(&mut buf[..n]).with_context(|| format!("read {}", partition.display()))?;
        output.write_all(&buf[..n]).with_context(|| format!("write {}", backup.display()))?;
        sum.update(&buf[..n]);
        left -= n as u64;
    }
    output.sync_all().context("flush the backup")?;
    Ok(hex::encode(sum.finalize()))
}

fn sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut sum = Sha256::new();
    let mut buf = [0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        sum.update(&buf[..n]);
    }
    Ok(hex::encode(sum.finalize()))
}

fn write_asset(path: &Path, bytes: &[u8]) -> Result<()> {
    write_file(path, bytes, 0o755)
}

fn extract<R: Read + Seek>(zip: &mut zip::ZipArchive<R>, entry: &str, path: &Path, mode: u32) -> Result<()> {
    let mut file = zip.by_name(entry)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    write_file(path, &bytes, mode)
}

fn write_file(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    let mut file = File::create(path).with_context(|| format!("make {}", path.display()))?;
    file.write_all(bytes).with_context(|| format!("write {}", path.display()))?;
    file.sync_all()?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).with_context(|| format!("set the permissions of {}", path.display()))?;
    Ok(())
}
