//! Keeping this recovery in the recovery partitions an install writes
//! over: this recovery as its partition holds it, what each slot's
//! partition held before the install, and which slots get it back after.

use crate::image;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use skyward_sys::kmsg;
use skyward_vab::Slot;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

const PARTITION: &str = "recovery";

/// What a slot's recovery partition holds: a digest of all of it, and
/// whether that is Skyward, any build of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    pub digest: [u8; 32],
    pub skyward: bool,
}

/// This recovery, held from the current slot's recovery partition before
/// an install, and what each slot's partition held then, slot A first.
pub struct Kept {
    dir: PathBuf,
    image: Vec<u8>,
    own: [u8; 32],
    before: [Held; 2],
}

impl Kept {
    /// Hold this recovery from the recovery partitions among the links in
    /// `dir`, before an install; none when neither slot's partition holds
    /// Skyward, as when it was started without being flashed.
    pub fn hold(dir: &Path, current: Slot) -> Result<Option<Kept>> {
        // A phone whose recovery lives elsewhere, or one whose links are
        // not the ones this names, has no partition to keep Skyward in:
        let paths = [image::device(dir, PARTITION, Slot::A), image::device(dir, PARTITION, Slot::B)];
        if !paths.iter().any(|path| path.exists()) {
            return Ok(None);
        }
        // The slot the recovery runs from is the one whose image is kept;
        // the other only says what it held. A partition this recovery
        // cannot read is left unknown rather than made a reason to stop the
        // install, and the slot that starts next gets this build back
        // whatever it held (2026-11-25, from Kinaxie/skyward).
        let mut held = [Held { digest: [0; 32], skyward: false }; 2];
        let mut images: [Option<Vec<u8>>; 2] = [None, None];
        for slot in [Slot::A, Slot::B] {
            let path = &paths[slot.number() as usize];
            match read(path) {
                Ok((what, bytes)) => {
                    held[slot.number() as usize] = what;
                    images[slot.number() as usize] = Some(bytes);
                }
                Err(e) => {
                    let _ = kmsg(&format!("keep: {} is not read: {e:#}", path.display()));
                }
            }
        }
        // A build of Skyward in either slot is the one this recovery runs:
        // the phone may have been started from the other slot's partition,
        // or from one flashed while the other started.
        let source = [current, current.other()]
            .into_iter()
            .find(|&slot| held[slot.number() as usize].skyward && images[slot.number() as usize].is_some());
        let Some(source) = source else { return Ok(None) };
        let own = held[source.number() as usize];
        let image = images[source.number() as usize].take().expect("a held image");
        if source != current {
            let _ = kmsg(&format!("keep: this recovery is taken from slot {}'s partition", source.letter()));
        }
        Ok(Some(Kept { dir: dir.to_path_buf(), image, own: own.digest, before: held }))
    }

    /// The slots whose recovery partition gets this recovery back, by what
    /// the partitions hold once the install is done; `next` is the slot
    /// that starts next. A partition that cannot be read now is treated as
    /// changed, so this recovery goes back into it.
    pub fn wanted(&self, next: Slot) -> Result<Vec<Slot>> {
        let after = |slot: Slot| {
            read(&image::device(&self.dir, PARTITION, slot))
                .map(|(held, _)| held)
                .unwrap_or(Held { digest: [0; 32], skyward: false })
        };
        Ok(restore(self.before, [after(Slot::A), after(Slot::B)], self.own, next))
    }

    /// Write this recovery into the recovery partitions of `slots` and read
    /// each back; `progress` hears the bytes so far, of all. Each slot is
    /// written on its own: one that will not take it does not keep the
    /// others from holding this build.
    pub fn write(&self, slots: &[Slot], progress: &mut dyn FnMut(u64, u64)) -> Result<Vec<Slot>> {
        let len = self.image.len() as u64;
        let total = len * slots.len() as u64 * 2;
        let mut done = 0;
        let mut written = Vec::new();
        let mut failed = Vec::new();
        for &slot in slots {
            let path = image::device(&self.dir, PARTITION, slot);
            let result = image::flash(&mut Cursor::new(&self.image), len, std::slice::from_ref(&path), &mut |at, _| {
                progress(done + at, total);
            });
            match result {
                Ok(()) => written.push(slot),
                Err(e) => {
                    let _ = kmsg(&format!("keep: {} does not take this recovery: {e:#}", path.display()));
                    failed.push(format!("{}: {e:#}", path.display()));
                }
            }
            done += len * 2;
        }
        if written.is_empty() {
            bail!("{}", failed.join("; "));
        }
        if !failed.is_empty() {
            let _ = kmsg(&format!("keep: this recovery went back into {written:?}, not {failed:?}"));
        }
        Ok(written)
    }
}

/// What the partition at `path` holds, read past any pages cached of it,
/// and all of it.
fn read(path: &Path) -> Result<(Held, Vec<u8>)> {
    let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    image::drop_cache(&file).with_context(|| format!("drop the cached pages of {}", path.display()))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).with_context(|| format!("read {}", path.display()))?;
    let skyward = image::is_skyward(&mut Cursor::new(&bytes)).with_context(|| format!("look into {}", path.display()))?;
    Ok((Held { digest: Sha256::digest(&bytes).into(), skyward }, bytes))
}

/// The slots whose recovery partition gets this recovery back once an
/// install is done, by what each held `before` and `after` it, slot A
/// first: the slot that starts next, `next`, unless it holds this build,
/// whose digest is `own`; and any other the install wrote over with
/// something that is not Skyward.
pub fn restore(before: [Held; 2], after: [Held; 2], own: [u8; 32], next: Slot) -> Vec<Slot> {
    [Slot::A, Slot::B]
        .into_iter()
        .filter(|&slot| {
            let k = slot.number() as usize;
            match slot == next {
                true => after[k].digest != own,
                false => after[k].digest != before[k].digest && !after[k].skyward,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::samples::recovery;

    const SKYWARD: Held = Held { digest: [1; 32], skyward: true };
    const OWN: [u8; 32] = SKYWARD.digest;
    /// Another build of Skyward.
    const OTHER_BUILD: Held = Held { digest: [4; 32], skyward: true };
    const STOCK: Held = Held { digest: [2; 32], skyward: false };
    const OLD: Held = Held { digest: [3; 32], skyward: false };

    #[test]
    fn the_slot_that_starts_next_gets_this_build() {
        assert_eq!(restore([SKYWARD, OLD], [SKYWARD, STOCK], OWN, Slot::B), [Slot::B], "an update or a package that went to slot B");
        assert_eq!(restore([SKYWARD, OLD], [SKYWARD, OLD], OWN, Slot::B), [Slot::B], "an older recovery there, left alone by the install");
        assert_eq!(restore([SKYWARD, OTHER_BUILD], [SKYWARD, OTHER_BUILD], OWN, Slot::B), [Slot::B], "another build of Skyward there");
        assert!(restore([SKYWARD, SKYWARD], [SKYWARD, SKYWARD], OWN, Slot::A).is_empty(), "this recovery is there already");
    }

    #[test]
    fn elsewhere_it_goes_back_only_where_the_install_wrote_something_else() {
        assert!(restore([SKYWARD, OLD], [SKYWARD, OLD], OWN, Slot::A).is_empty(), "a package that left recovery alone, such as a root installer");
        assert_eq!(restore([SKYWARD, OLD], [SKYWARD, STOCK], OWN, Slot::A), [Slot::B]);
        assert_eq!(restore([SKYWARD, OLD], [STOCK, STOCK], OWN, Slot::A), [Slot::A, Slot::B]);
        assert!(restore([SKYWARD, OLD], [SKYWARD, OTHER_BUILD], OWN, Slot::A).is_empty(), "a package that put another build of Skyward in");
    }

    /// Recovery partitions of `size` bytes holding `a` and `b`.
    fn partitions(a: &[u8], b: &[u8], size: usize) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (name, image) in [("recovery_a", a), ("recovery_b", b)] {
            let mut bytes = image.to_vec();
            bytes.resize(size, 0);
            std::fs::write(dir.path().join(name), bytes).unwrap();
        }
        dir
    }

    #[test]
    fn this_recovery_goes_back_where_an_install_wrote_over_it() {
        let (skyward, stock) = (recovery(true), recovery(false));
        let size = skyward.len().max(stock.len()) + 4096;
        let dir = partitions(&skyward, &stock, size);
        let kept = Kept::hold(dir.path(), Slot::A).unwrap().expect("slot A's partition holds Skyward");
        assert!(kept.wanted(Slot::A).unwrap().is_empty(), "nothing written over");
        // A package puts the stock recovery in both slots.
        let mut written = stock.clone();
        written.resize(size, 0);
        std::fs::write(dir.path().join("recovery_a"), &written).unwrap();
        assert_eq!(kept.wanted(Slot::A).unwrap(), [Slot::A], "slot B held the stock recovery already");
        let mut told = Vec::new();
        assert_eq!(kept.write(&[Slot::A], &mut |done, total| told.push((done, total))).unwrap(), [Slot::A]);
        assert_eq!(told.last(), Some(&(2 * size as u64, 2 * size as u64)), "written and read back");
        let mut whole = skyward.clone();
        whole.resize(size, 0);
        assert_eq!(std::fs::read(dir.path().join("recovery_a")).unwrap(), whole);
        assert!(kept.wanted(Slot::A).unwrap().is_empty());
        assert_eq!(kept.wanted(Slot::B).unwrap(), [Slot::B], "an install that has slot B start next");
    }

    #[test]
    fn a_build_in_either_slot_is_the_one_kept() {
        let dir = partitions(&recovery(true), &recovery(false), 1 << 20);
        assert!(Kept::hold(dir.path(), Slot::B).unwrap().is_some(), "the other slot's partition holds Skyward");
    }

    #[test]
    fn nothing_is_kept_when_this_recovery_was_started_without_being_flashed() {
        let dir = partitions(&recovery(false), &recovery(false), 1 << 20);
        assert!(Kept::hold(dir.path(), Slot::B).unwrap().is_none());
        let erased = partitions(&[], &[], 1 << 20);
        assert!(Kept::hold(erased.path(), Slot::B).unwrap().is_none(), "neither slot holds Skyward");
    }
}
