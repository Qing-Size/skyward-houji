//! The modules Magisk and KernelSU install: a folder each under
//! `/data/adb/modules`, with a `module.prop` in it, a `disable` file when it
//! is off and a `remove` file when it is to go at the next start.

use anyhow::{Context, Result};
use skyward_ui::app::ModuleRow;
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
#[cfg(target_os = "android")]
use std::process::Command;

#[cfg(target_os = "android")]
const MODULES: &str = "/data/adb/modules";
#[cfg(target_os = "android")]
const UPDATE: &str = "/data/adb/modules_update";

#[cfg(target_os = "android")]
pub fn read() -> Result<Vec<ModuleRow>, String> {
    read_in(Path::new(MODULES), Path::new(UPDATE)).map_err(|e| format!("{e:#}"))
}

/// Install the zip at `path` through whichever root manager is on the
/// phone. The manager runs the module's own installer and stages it where
/// it will be merged at the next start.
#[cfg(target_os = "android")]
pub fn install(path: &str) -> Result<String> {
    let magisk = Path::new("/data/adb/magisk/magisk");
    let ksuds = ["/data/adb/ksu/bin/ksud", "/data/adb/ksud", "/system/bin/ksud"];
    let ksud = ksuds.iter().map(Path::new).find(|path| path.is_file());
    // The manager whose boot image is patched is the one whose modules the
    // installed system loads, even when the other one's files are still
    // in `/data/adb`.
    match crate::root::installed_root() {
        Some(crate::root::InstalledRoot::KernelSu) => {
            let ksud = ksud.context("KernelSU's ksud is not on the phone")?;
            run(ksud, &["module", "install", path])?;
            return Ok("KernelSU".into());
        }
        Some(crate::root::InstalledRoot::Magisk) => {
            anyhow::ensure!(magisk.is_file(), "Magisk's binary is not on the phone");
            run(magisk, &["--install-module", path])?;
            return Ok("Magisk".into());
        }
        None => {}
    }
    if magisk.is_file() {
        run(magisk, &["--install-module", path])?;
        return Ok("Magisk".into());
    }
    if let Some(ksud) = ksud {
        run(ksud, &["module", "install", path])?;
        return Ok("KernelSU".into());
    }
    anyhow::bail!("neither Magisk nor KernelSU is installed")
}

#[cfg(target_os = "android")]
pub enum Event {
    Ended(Result<String, String>),
}

/// Installing a module runs the module's own scripts, so it is done off
/// the thread that draws the pages.
#[cfg(target_os = "android")]
pub struct Worker {
    worker: skyward_sys::worker::Worker<Event>,
}

#[cfg(target_os = "android")]
impl Worker {
    pub fn start(path: String) -> Result<Worker> {
        let worker = skyward_sys::worker::Worker::start("modules", move |tell| {
            let end = install(&path).map_err(|e| format!("{e:#}").chars().take(300).collect::<String>());
            tell.tell(Event::Ended(end));
        })?;
        Ok(Worker { worker })
    }

    pub fn fd(&self) -> std::os::fd::RawFd {
        self.worker.fd()
    }

    pub fn events(&self) -> Vec<Event> {
        self.worker.events()
    }
}

#[cfg(target_os = "android")]
fn run(program: &Path, args: &[&str]) -> Result<()> {
    let out = Command::new(program).args(args).output().with_context(|| format!("run {}", program.display()))?;
    anyhow::ensure!(
        out.status.success(),
        "{} failed: {}{}",
        program.display(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(())
}

fn read_in(modules: &Path, update: &Path) -> Result<Vec<ModuleRow>> {
    let mut found: BTreeMap<String, ModuleRow> = BTreeMap::new();
    for (dir, pending) in [(modules, false), (update, true)] {
        let list = match fs::read_dir(dir) {
            Ok(list) => list,
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(e).with_context(|| format!("read {}", dir.display())),
        };
        for entry in list.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().into_owned();
            let prop = fs::read_to_string(path.join("module.prop")).unwrap_or_default();
            if prop.trim().is_empty() {
                continue;
            }
            let field = |name: &str| {
                let key = format!("{name}=");
                prop.lines().find_map(|line| line.trim().strip_prefix(&key).map(|v| v.trim().to_string())).unwrap_or_default()
            };
            let (name, version, author) = (field("name"), field("version"), field("author"));
            let note = match (version.is_empty(), author.is_empty()) {
                (true, true) => String::new(),
                (false, true) => version,
                (true, false) => author,
                (false, false) => format!("{version} · {author}"),
            };
            let row = ModuleRow {
                id: id.clone(),
                name: if name.is_empty() { id.clone() } else { name },
                note,
                about: field("description"),
                enabled: !path.join("disable").exists(),
                removing: path.join("remove").exists(),
                pending: pending || found.get(&id).is_some_and(|r| r.pending),
            };
            match found.get_mut(&id) {
                Some(old) if pending => old.pending = true,
                Some(_) => {}
                None => {
                    found.insert(id, row);
                }
            }
        }
    }
    let mut rows: Vec<ModuleRow> = found.into_values().collect();
    rows.sort_by_key(|m| m.name.to_lowercase());
    Ok(rows)
}

#[cfg(target_os = "android")]
pub fn switch(id: &str, on: bool) -> Result<()> {
    switch_in(Path::new(MODULES), Path::new(UPDATE), id, on)
}

fn switch_in(modules: &Path, update: &Path, id: &str, on: bool) -> Result<()> {
    for dir in dirs_in(modules, update, id) {
        let flag = dir.join("disable");
        match on {
            true => match fs::remove_file(&flag) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::NotFound => {}
                Err(e) => return Err(e).with_context(|| format!("switch {} on", dir.display())),
            },
            false => fs::write(&flag, b"").with_context(|| format!("switch {} off", dir.display()))?,
        }
    }
    Ok(())
}

#[cfg(target_os = "android")]
pub fn remove(id: &str, on: bool) -> Result<()> {
    remove_in(Path::new(MODULES), Path::new(UPDATE), id, on)
}

fn remove_in(modules: &Path, update: &Path, id: &str, on: bool) -> Result<()> {
    for dir in dirs_in(modules, update, id) {
        let flag = dir.join("remove");
        match on {
            true => fs::write(&flag, b"").with_context(|| format!("mark {} for removal", dir.display()))?,
            false => match fs::remove_file(&flag) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::NotFound => {}
                Err(e) => return Err(e).with_context(|| format!("keep {}", dir.display())),
            },
        }
    }
    Ok(())
}

fn dirs_in(modules: &Path, update: &Path, id: &str) -> Vec<PathBuf> {
    [modules, update].into_iter().map(|base| base.join(id)).filter(|path| path.is_dir()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("skyward-modules-{tag}-{}", std::process::id()));
        fs::remove_dir_all(&path).ok();
        fs::create_dir_all(path.join("modules/one")).unwrap();
        fs::write(path.join("modules/one/module.prop"), "name=One\nversion=v1\nauthor=me\ndescription=does one\n").unwrap();
        path
    }

    #[test]
    fn a_module_is_read_from_its_prop_and_turned_off_by_the_disable_file() {
        let root = scratch("read");
        let modules = root.join("modules");
        let update = root.join("modules_update");
        let rows = read_in(&modules, &update).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].id.as_str(), rows[0].enabled), ("one", true));
        switch_in(&modules, &update, "one", false).unwrap();
        assert!(modules.join("one/disable").exists());
        remove_in(&modules, &update, "one", true).unwrap();
        assert!(modules.join("one/remove").exists());
        fs::remove_dir_all(&root).ok();
    }
}
