//! The WLANs a user chose to have kept: whether the radio is opened as the
//! recovery starts, whether the passwords of the networks joined are kept,
//! and those networks with their passwords.
//!
//! 一个文件里记多张网：公司一张、家里一张，开机（或换地方）时按记住的先后
//! 挨个试，哪张在范围内就连哪张。
//!
//! Kept beside the recovery's own settings, readable by root alone. Turning
//! the passwords off leaves the networks already kept as they are, and a
//! factory reset takes them away.

use anyhow::{Context, Result};
use skyward_net::supplicant::Security;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// The partition the file is on, made writable for as long as a write takes.
const METADATA: &str = "/metadata";

/// Where it is kept: beside the recovery's own settings.
pub const FILE: &str = "/metadata/skyward/wlan";

/// A network kept, with the password it was joined with and the kind of
/// protection it has: joining it again as soon as the radio is up needs
/// both, without waiting for a scan to say what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub ssid: Vec<u8>,
    pub password: String,
    pub security: Security,
}

/// What the file holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    /// Open the WLAN as the recovery starts.
    pub auto: bool,
    /// Keep the passwords of the networks joined.
    pub keep: bool,
    /// 记住的网，**最近用过的在前**：开机先试它，试不通再往后。
    pub networks: Vec<Saved>,
}

/// 最多记这么多张，超了丢最久没用的那张。
pub const MOST: usize = 8;

/// 第一次开机没有这个文件时：默认记住密码。
impl Default for Kept {
    fn default() -> Self {
        Kept { auto: false, keep: true, networks: Vec::new() }
    }
}

/// What is kept, as the file holds it.
pub fn read() -> Kept {
    with_metadata(read_unmounted)
}

/// Read it with /metadata mounted for the moment it takes: the pages are built
/// before the recovery mounts it, and a file read then is a file not found.
fn with_metadata<T>(work: impl FnOnce() -> T) -> T {
    let at = std::path::Path::new("/metadata");
    let mounted = skyward_block::mount::is_mount_point(at).unwrap_or(false);
    if !mounted
        && skyward_block::mount::mount_ro(&std::path::Path::new(skyward_block::BY_NAME).join("metadata"), at).is_err()
    {
        return work();
    }
    let done = work();
    if !mounted {
        let _ = skyward_block::mount::umount(at);
    }
    done
}

fn read_unmounted() -> Kept {
    let text = match std::fs::read_to_string(FILE) {
        Ok(text) => text,
        Err(_) => return Kept::default(),
    };
    let mut kept = Kept::default();
    // 每一节是一张网；第一节另有 auto / keep。老文件没有 `[net]` 头，
    // 那张网就写在第一节里，照样读得出来。
    let mut sections: Vec<Vec<&str>> = vec![Vec::new()];
    for line in text.lines() {
        if line.trim() == "[net]" {
            sections.push(Vec::new());
            continue;
        }
        if let Some(last) = sections.last_mut() {
            last.push(line);
        }
    }
    for (index, lines) in sections.iter().enumerate() {
        let mut ssid = None;
        let mut password = None;
        let mut security = None;
        for line in lines {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "auto" if index == 0 => kept.auto = value == "1",
                "keep" if index == 0 => kept.keep = value == "1",
                "ssid" => ssid = unhex(value),
                // 新格式：密码是十六进制，`=`、空格都不会把文件写坏。
                "pw" => password = unhex(value).and_then(|bytes| String::from_utf8(bytes).ok()),
                // 老格式：密码就是原文。
                "password" => password = Some(value.to_string()),
                "security" => security = Some(security_of(value)),
                _ => {}
            }
        }
        if let (Some(ssid), Some(password)) = (ssid, password)
            && !ssid.is_empty()
            && !password.is_empty()
        {
            // 老文件没记保护方式：PSK 与 SAE 都给，两种网都能连。
            kept.networks.push(Saved { ssid, password, security: security.unwrap_or(Security::PskSae) });
        }
    }
    // 名单不再跟着「记住密码」的开关走：开关关掉只是不再记新的。
    kept.networks.truncate(MOST);
    kept
}

/// Do `work` with the partition writable, and read-only again afterwards.
fn writable<T>(work: impl FnOnce() -> Result<T>) -> Result<T> {
    let at = Path::new(METADATA);
    let mounted = skyward_block::mount::is_mount_point(at)?;
    if mounted {
        skyward_block::mount::remount_writable(at, true).context("make /metadata writable")?;
    }
    let done = work();
    if mounted {
        let _ = skyward_block::mount::remount_writable(at, false);
    }
    done
}

fn store(text: &str) -> Result<()> {
    writable(|| {
        let at = Path::new(FILE);
        if let Some(dir) = at.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("make {}", dir.display()))?;
        }
        std::fs::write(at, text).with_context(|| format!("write {}", at.display()))?;
        std::fs::set_permissions(at, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("keep {} to root alone", at.display()))?;
        Ok(())
    })
}

fn render(kept: &Kept) -> String {
    let mut text = format!("auto={}\nkeep={}\n", u8::from(kept.auto), u8::from(kept.keep));
    // `keep` 只管「以后连上的还记不记」，不再拿它挡着写名单：以前一到 keep=0
    // 就只写两行开关，文件一存，记住的那几张网全没了。要删某一张，只有 `forget`。
    for saved in kept.networks.iter().take(MOST) {
        text.push_str("[net]\n");
        text.push_str(&format!("ssid={}\n", hex(&saved.ssid)));
        text.push_str(&format!("pw={}\n", hex(saved.password.as_bytes())));
        text.push_str(&format!("security={}\n", security_code(saved.security)));
    }
    text
}

/// Set whether the WLAN opens as the recovery starts, leaving the rest as it
/// was.
pub fn set_auto(on: bool) -> Result<()> {
    let mut kept = read();
    kept.auto = on;
    store(&render(&kept))
}

/// Set whether a network joined from now on is kept. Turning it off leaves the
/// ones already kept as they are.
pub fn set_keep(on: bool) -> Result<()> {
    let mut kept = read();
    kept.keep = on;
    store(&render(&kept))
}

/// Keep `ssid`, the password it was joined with and the kind of protection it
/// has. 同一张网再存一次就挪到最前面 —— 开机先试它。
pub fn save(ssid: &[u8], password: &str, security: Security) -> Result<()> {
    let mut kept = read();
    kept.keep = true;
    kept.networks.retain(|saved| saved.ssid != ssid);
    kept.networks.insert(0, Saved { ssid: ssid.to_vec(), password: password.to_string(), security });
    kept.networks.truncate(MOST);
    store(&render(&kept))
}

/// Forget `ssid` alone: another network's password is not this one's business.
pub fn forget(ssid: &[u8]) -> Result<()> {
    let mut kept = read();
    kept.networks.retain(|saved| saved.ssid != ssid);
    store(&render(&kept))
}

/// How the kind of protection is written in the file.
fn security_code(security: Security) -> &'static str {
    match security {
        Security::Open => "open",
        Security::Owe => "owe",
        Security::Wep => "wep",
        Security::Psk => "psk",
        Security::Sae => "sae",
        Security::PskSae => "psksae",
        Security::Enterprise => "enterprise",
    }
}

/// What the file's word for it means.
fn security_of(code: &str) -> Security {
    match code {
        "open" => Security::Open,
        "owe" => Security::Owe,
        "wep" => Security::Wep,
        "sae" => Security::Sae,
        "psksae" => Security::PskSae,
        "enterprise" => Security::Enterprise,
        _ => Security::Psk,
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按文件的写法切开：第一个 `[net]` 之前是 auto / keep。
    fn sections(text: &str) -> Vec<Vec<&str>> {
        let mut out: Vec<Vec<&str>> = vec![Vec::new()];
        for line in text.lines() {
            if line.trim() == "[net]" {
                out.push(Vec::new());
            } else if let Some(last) = out.last_mut() {
                last.push(line);
            }
        }
        out
    }

    #[test]
    fn a_name_goes_round_the_file_as_bytes() {
        assert_eq!(unhex(&hex("Skyward客厅".as_bytes())).as_deref(), Some("Skyward客厅".as_bytes()));
    }

    #[test]
    fn a_password_keeps_what_it_holds() {
        assert_eq!(unhex(&hex(b"pa ss=word")), Some(b"pa ss=word".to_vec()));
    }

    #[test]
    fn half_a_byte_is_no_name() {
        assert_eq!(unhex("abc"), None);
    }

    #[test]
    fn what_is_written_is_a_setting_and_one_network() {
        let kept = Kept {
            auto: true,
            keep: true,
            networks: vec![Saved { ssid: b"Sarva5".to_vec(), password: "p=w d".into(), security: Security::PskSae }],
        };
        let text = render(&kept);
        assert_eq!(text, "auto=1\nkeep=1\n[net]\nssid=536172766135\npw=703d772064\nsecurity=psksae\n");
        assert_eq!(sections(&text).len(), 2);
    }

    #[test]
    fn a_second_network_is_kept_beside_the_first_newest_first() {
        let kept = Kept {
            auto: true,
            keep: true,
            networks: vec![
                Saved { ssid: b"home".to_vec(), password: "h".into(), security: Security::Psk },
                Saved { ssid: b"office".to_vec(), password: "o".into(), security: Security::Sae },
            ],
        };
        let text = render(&kept);
        assert_eq!(sections(&text).len(), 3, "两张网各一节");
        assert!(text.find("ssid=686f6d65").unwrap() < text.find("ssid=6f6666696365").unwrap());
    }

    #[test]
    fn a_network_is_still_written_when_the_setting_is_off() {
        let kept = Kept {
            auto: true,
            keep: false,
            networks: vec![Saved { ssid: b"x".to_vec(), password: "y".into(), security: Security::Psk }],
        };
        let text = render(&kept);
        assert!(text.starts_with("auto=1\nkeep=0\n"), "开关照实写");
        assert!(text.contains("ssid=78"), "网络也照写");
        assert_eq!(sections(&text).len(), 1, "一张网一节");
    }

    #[test]
    fn the_file_written_for_one_network_still_reads_as_one() {
        let text = "auto=1\nkeep=1\nssid=536172766135\npassword=p=w d\nsecurity=psksae\n";
        assert_eq!(sections(text).len(), 1, "老文件没有 [net] 头");
        assert_eq!(unhex("536172766135").as_deref(), Some(b"Sarva5".as_slice()));
    }
}
