//! The settings the pages keep from one start of the recovery to the next:
//! the Settings page's, the install page's options, what a backup takes and
//! where it goes, and which logs are exported where. They are kept as JSON,
//! each value as what it means, a language as its locale, a time in seconds,
//! a time zone by its name in the tz database, a part or a place by its
//! name, so a later version that lists them otherwise reads them the same.

use crate::app::{AutoRoot, Place, State, SystemZone, SCREEN_OFF, TIME_ZONES};
use crate::Lang;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// How long settings stand unchanged before they are written, so a slider
/// dragged across the page is written once, after the hand stops.
pub const WAIT_MS: u64 = 2000;

/// The parts a backup takes, by name, in the order of the backup page.
const PARTS: [&str; 5] = ["boot", "system", "data", "media", "modem"];
/// Where a backup goes, by name.
const STORAGE: &str = "storage";
const USB_DRIVE: &str = "usb_drive";
/// The logs exported, and where they go, by name, in the order of the page
/// that exports them.
const LOGS: [&str; 2] = ["recovery", "kernel"];
const LOG_PLACES: [&str; 2] = ["temporary", "storage"];

/// The settings as they are kept. A value missing is the one a recovery
/// starts with, and one this version does not know is left out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The language chosen on the language page, as a locale; none while
    /// the pages speak the one the system names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// The language the pages last spoke, as a locale, for a start that
    /// names none, as fastboot's does.
    pub spoken: Option<String>,
    pub dark: bool,
    /// The brightness slider's place, from 0 at its left end to 1.
    pub brightness: f32,
    /// How long the screen stays on with nothing touched, in seconds; 0
    /// keeps it on.
    pub screen_off: u64,
    /// The clock keeps the time zone the installed system is set to.
    pub follow_system_time_zone: bool,
    /// The time zone chosen on its pages, by its name in the tz database.
    pub time_zone: String,
    /// The time zone the installed system was last seen set to, by its
    /// name in the tz database, for a start before the data partition
    /// opens.
    pub system_time_zone: Option<String>,
    pub vibrate: bool,
    /// The vibration slider's place, from 0 at its left end to 1.
    pub vibration: f32,
    /// A computer connected over USB is offered internal storage.
    pub file_transfer: bool,
    /// How fast the phone is let to charge, by the name of its step.
    pub charge_mode: String,
    /// Step the charge down when the battery gets warmer than the limit.
    pub thermal_guard: bool,
    /// The battery temperature limit, in whole degrees Celsius.
    pub thermal_limit: u8,
    /// 安装后自动用哪个 Root 修补新槽：`none` / `kernelsu` / `magisk`。
    #[serde(default)]
    pub auto_root: String,
    pub install: Install,
    pub backup: Backup,
    pub logs: Logs,
    /// 手动指定的 IPv4；有它就不走 DHCP。2026-11-27。
    #[serde(default)]
    pub wlan_static: Option<StaticV4>,
    /// 自己建的首页分类；空着就是不分栏，全部落在「其他」。2026-11-27。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub home_sections: Vec<HomeSection>,
    /// 「其他」在首页折起来了，以及它的名字。
    #[serde(default, skip_serializing_if = "is_false")]
    pub home_rest_collapsed: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub home_rest_name: String,
    /// 「其他」摆在分类之间的哪个位置；没有就是末尾。2026-11-27。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_rest_at: Option<usize>,
    /// 首页顶上那个大字；空着就是 Skyward。2026-11-27。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub home_title: String,
    /// 首页条目的自定义名字，按 `Entry::id()` 写；没有的用页面自带的名字。
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub entry_names: std::collections::BTreeMap<String, String>,
    /// 单独放在首页最上面的条目。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub home_top: Vec<String>,
    /// 「其他」那一栏的顺序，按 `Entry::id()` 写。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub home_rest: Vec<String>,
    /// 旧版的平铺顺序：只读，读到就搬进 `home_rest`。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub home_order: Vec<String>,
    /// 旧版高级页的顺序与隐藏名单：只读，读到就丢掉（页面已去掉，2026-11-27）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub advanced_order: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub advanced_hidden: Vec<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// 设置文件里的一栏：名字与条目 id。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HomeSection {
    pub name: String,
    pub entries: Vec<String>,
    /// 栏目图标，按 `Icon::id()` 写；空的按 Layels 处理。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// 在首页折起来了。
    #[serde(default)]
    pub collapsed: bool,
}

/// 手动指定的 IPv4：地址、掩码、网关，和 DNS（逗号或空格分隔，可空，空则用网关）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StaticV4 {
    pub address: String,
    pub mask: String,
    pub gateway: String,
    pub dns: String,
}

/// The install page's options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Install {
    /// A zip's install script runs only if the zip was signed with one of
    /// the phone's update keys.
    pub check_signature: bool,
    /// Skyward goes back into the recovery partitions an install writes
    /// over.
    pub keep_recovery: bool,
}

/// What a backup takes and where it goes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Backup {
    /// The parts it takes: boot, system, data and modem.
    pub parts: Vec<String>,
    /// Where it goes: storage, or usb_drive.
    pub to: String,
    pub compress: bool,
    pub verify: bool,
}

/// Which logs are exported, and where.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Logs {
    /// The logs: recovery and kernel.
    pub kinds: Vec<String>,
    /// Where they go: temporary, or storage.
    pub to: String,
}

impl Default for Settings {
    /// The settings a recovery starts with.
    fn default() -> Self {
        Settings::of(&State::default())
    }
}

impl Default for Install {
    fn default() -> Self {
        Settings::default().install
    }
}

impl Default for Backup {
    fn default() -> Self {
        Settings::default().backup
    }
}

impl Default for Logs {
    fn default() -> Self {
        Settings::default().logs
    }
}

impl Settings {
    /// The settings `state` holds.
    pub fn of(state: &State) -> Settings {
        Settings {
            language: state.language_chosen.then(|| state.lang.tag().to_string()),
            spoken: Some(state.lang.tag().to_string()),
            dark: state.dark,
            brightness: state.brightness,
            screen_off: SCREEN_OFF[state.screen_off].map_or(0, |ms| ms / 1000),
            follow_system_time_zone: state.follow_system_zone,
            time_zone: TIME_ZONES[state.time_zone].tz.to_string(),
            system_time_zone: state.system_zone.as_ref().map(|zone| zone.name.clone()),
            vibrate: state.vibrate,
            vibration: state.vibration,
            file_transfer: state.file_transfer,
            charge_mode: state.charge_mode.name().to_string(),
            thermal_guard: state.thermal_guard,
            auto_root: state.auto_root.name().to_string(),
            thermal_limit: state.thermal_limit,
            install: Install { check_signature: state.check_signature, keep_recovery: state.keep_recovery },
            backup: Backup {
                parts: names(&state.backup, &PARTS),
                to: match state.backup_to {
                    Place::Storage => STORAGE,
                    Place::UsbDrive => USB_DRIVE,
                }
                .to_string(),
                compress: state.compress,
                verify: state.verify,
            },
            logs: Logs { kinds: names(&state.logs, &LOGS), to: LOG_PLACES[state.logs_to].to_string() },
            wlan_static: state.static_on.then(|| state.static_ip.clone()),
            home_sections: state
                .home_sections
                .iter()
                .map(|section| HomeSection {
                    name: section.name.clone(),
                    entries: section.entries.iter().map(|entry| entry.id().to_string()).collect(),
                    icon: section.icon.id().to_string(),
                    collapsed: section.collapsed,
                })
                .collect(),
            home_title: state.home_title.clone(),
            entry_names: state.entry_names.clone(),
            home_rest_at: state.home_rest_at.filter(|at| *at < state.home_sections.len()),
            home_rest_collapsed: state.home_rest_collapsed,
            home_rest_name: state.home_rest_name.clone(),
            home_top: state.home_top.iter().map(|entry| entry.id().to_string()).collect(),
            home_rest: if state.home_sections.is_empty() && state.home_top.is_empty() && crate::app::rest_is_default(&state.home_rest) {
                Vec::new()
            } else {
                state.home_rest.iter().map(|entry| entry.id().to_string()).collect()
            },
            home_order: Vec::new(),
            advanced_order: Vec::new(),
            advanced_hidden: Vec::new(),
        }
    }

    /// Read settings as [`Settings::text`] writes them.
    pub fn read(text: &str) -> Result<Settings> {
        serde_json::from_str(text).context("read the settings")
    }

    /// The settings as their file holds them.
    pub fn text(&self) -> String {
        serde_json::to_string_pretty(self).expect("settings always make JSON") + "\n"
    }

    /// The language chosen on the language page.
    pub fn chosen(&self) -> Option<Lang> {
        self.language.as_deref().and_then(|tag| Lang::for_locale(tag).ok())
    }

    /// The language the pages last spoke.
    pub fn spoken(&self) -> Option<Lang> {
        self.spoken.as_deref().and_then(|tag| Lang::for_locale(tag).ok())
    }

    /// Put the settings into `state`, each that is one the pages can show:
    /// a slider's place between its ends, a time, a time zone, a part or a
    /// place they offer. Any other stays as it is.
    pub fn apply(&self, state: &mut State) {
        if let Some(lang) = self.chosen() {
            state.lang = lang;
            state.language_chosen = true;
        }
        state.dark = self.dark;
        if (0.0..=1.0).contains(&self.brightness) {
            state.brightness = self.brightness;
        }
        if let Some(at) = SCREEN_OFF.iter().position(|off| off.map_or(0, |ms| ms / 1000) == self.screen_off) {
            state.screen_off = at;
        }
        state.follow_system_zone = self.follow_system_time_zone;
        if let Some(mode) = crate::app::ChargeMode::named(&self.charge_mode) {
            state.charge_mode = mode;
        }
        state.thermal_guard = self.thermal_guard;
        if let Some(root) = AutoRoot::named(&self.auto_root) {
            state.auto_root = root;
        }
        state.thermal_limit = self.thermal_limit.clamp(
            crate::app::THERMAL_LIMIT_LEAST,
            crate::app::THERMAL_LIMIT_MOST,
        );
        if let Some(at) = TIME_ZONES.iter().position(|zone| zone.tz == self.time_zone) {
            state.time_zone = at;
        }
        if let Some(zone) = self.system_time_zone.as_deref().and_then(|name| SystemZone::named(name).ok()) {
            state.system_zone = Some(zone);
        }
        state.vibrate = self.vibrate;
        if (0.0..=1.0).contains(&self.vibration) {
            state.vibration = self.vibration;
        }
        state.file_transfer = self.file_transfer;
        state.check_signature = self.install.check_signature;
        state.keep_recovery = self.install.keep_recovery;
        state.backup = picked(&self.backup.parts, &PARTS);
        match self.backup.to.as_str() {
            STORAGE => state.backup_to = Place::Storage,
            USB_DRIVE => state.backup_to = Place::UsbDrive,
            _ => {}
        }
        state.compress = self.backup.compress;
        state.verify = self.backup.verify;
        state.logs = picked(&self.logs.kinds, &LOGS);
        if let Some(at) = LOG_PLACES.iter().position(|place| *place == self.logs.to) {
            state.logs_to = at;
        }
        // 手动 IP：文件里有它就按它来，没有就回 DHCP。2026-11-27。
        state.static_on = self.wlan_static.is_some();
        if let Some(st) = &self.wlan_static {
            state.static_ip = st.clone();
        }
        // 首页分栏：文件里有就用它，没有就全落在「其他」；旧版的平铺顺序
        // 搬进「其他」（2026-11-27）。
        let mut sections: Vec<crate::app::Section> = self
            .home_sections
            .iter()
            .map(|section| crate::app::Section {
                name: section.name.clone(),
                entries: section.entries.iter().filter_map(|id| crate::app::Entry::from_id(id)).collect(),
                icon: crate::icons::Icon::from_id(&section.icon).unwrap_or(crate::icons::Icon::Layers),
                collapsed: section.collapsed,
            })
            .collect();
        let flat = if self.home_rest.is_empty() { &self.home_order } else { &self.home_rest };
        let mut top: Vec<crate::app::Entry> = self.home_top.iter().filter_map(|id| crate::app::Entry::from_id(id)).collect();
        let mut rest: Vec<crate::app::Entry> = flat.iter().filter_map(|id| crate::app::Entry::from_id(id)).collect();
        if sections.is_empty() && top.is_empty() && rest.is_empty() {
            rest = crate::app::DEFAULT_ORDER.to_vec();
        }
        // 0.0.155/0.156 把 file_transfer 自动追加到了「其他」末尾，它埋得太深
        // 找不着；认这个签名（紧跟在 home_order 后面）就把它挪到 WLAN 后面。
        if let Some(at) = rest.iter().position(|e| *e == crate::app::Entry::FileTransfer)
            && at > 0
            && rest[at - 1] == crate::app::Entry::HomeOrder
            && let Some(wlan) = rest.iter().position(|e| *e == crate::app::Entry::Wlan)
        {
            let entry = rest.remove(at);
            let wlan = rest.iter().position(|e| *e == crate::app::Entry::Wlan).unwrap_or(wlan);
            rest.insert(wlan + 1, entry);
        }
        crate::app::normalize(&mut sections, &mut top, &mut rest);
        state.home_sections = sections;
        state.home_top = top;
        state.home_rest = rest;
        state.home_rest_collapsed = self.home_rest_collapsed;
        state.home_rest_name = self.home_rest_name.clone();
        state.home_title = self.home_title.clone();
        state.entry_names = self.entry_names.clone();
        state.home_rest_at = self.home_rest_at.filter(|at| *at <= state.home_sections.len());
    }
}

/// The names of those of `all` that are `ticked`.
fn names(ticked: &[bool], all: &[&str]) -> Vec<String> {
    all.iter().zip(ticked).filter(|&(_, &on)| on).map(|(name, _)| name.to_string()).collect()
}

/// Which of `all` the names `kept` tick.
fn picked<const N: usize>(kept: &[String], all: &[&str; N]) -> [bool; N] {
    all.map(|name| kept.iter().any(|k| k == name))
}

/// What the settings file held as the recovery started.
#[derive(Debug, Clone, PartialEq)]
pub enum Kept {
    Settings(Box<Settings>),
    /// No settings: no file, or one that does not read as settings.
    Nothing,
    /// Where the file is could not be read; it is left alone this time.
    Unreadable,
}

impl Kept {
    /// The settings the file held, if it held any.
    pub fn settings(&self) -> Option<&Settings> {
        match self {
            Kept::Settings(settings) => Some(settings),
            Kept::Nothing | Kept::Unreadable => None,
        }
    }
}

/// Keeps the settings file in step with the settings the pages show:
/// what changed goes into it once it has stood unchanged for [`WAIT_MS`].
#[derive(Debug, Clone, PartialEq)]
pub struct Keeper {
    /// What the file holds; none while that is not known, when the file is
    /// left alone.
    holds: Option<Option<Settings>>,
    /// The settings the pages showed when last looked at, and since when;
    /// with no time once writing them failed, until they change.
    standing: Option<(Settings, Option<u64>)>,
}

impl Keeper {
    pub fn new(kept: &Kept) -> Keeper {
        let holds = match kept {
            Kept::Settings(settings) => Some(Some(settings.as_ref().clone())),
            Kept::Nothing => Some(None),
            Kept::Unreadable => None,
        };
        Keeper { holds, standing: None }
    }

    /// The settings to write at `now`, given those the pages `show`: none
    /// while they are what the file holds, or have stood unchanged for less
    /// than [`WAIT_MS`], or what the file holds is not known.
    pub fn due(&mut self, show: Settings, now: u64) -> Option<Settings> {
        let holds = self.holds.as_ref()?;
        if holds.as_ref() == Some(&show) {
            self.standing = None;
            return None;
        }
        match &self.standing {
            Some((stood, since)) if *stood == show => since.filter(|&since| now >= since + WAIT_MS).map(|_| show),
            _ => {
                self.standing = Some((show, Some(now)));
                None
            }
        }
    }

    /// `settings` are in the file now.
    pub fn written(&mut self, settings: Settings) {
        self.holds = Some(Some(settings));
        self.standing = None;
    }

    /// Writing the settings failed: they are written again once they change.
    pub fn failed(&mut self) {
        if let Some((_, since)) = &mut self.standing {
            *since = None;
        }
    }

    /// The place the file is kept may have been made afresh, as erasing all
    /// data makes it: the settings the pages show are written there again.
    pub fn remade(&mut self) {
        self.holds = Some(None);
        self.standing = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::listed;

    /// Where a zone is in [`TIME_ZONES`], by its name in the tz database.
    fn zone(tz: &str) -> usize {
        listed(tz).expect("a zone listed")
    }

    #[test]
    fn the_settings_go_round_through_their_file() {
        let state = State {
            lang: Lang::En,
            language_chosen: true,
            dark: true,
            brightness: 0.7,
            screen_off: 6,
            follow_system_zone: false,
            time_zone: zone("America/New_York"),
            system_zone: Some(SystemZone::named("Asia/Kabul").unwrap()),
            vibrate: false,
            vibration: 0.4,
            file_transfer: false,
            check_signature: true,
            keep_recovery: false,
            backup: [false, true, true, true, false],
            backup_to: Place::UsbDrive,
            compress: true,
            verify: false,
            logs: [false, true],
            logs_to: 1,
            ..State::default()
        };
        let kept = Settings::read(&Settings::of(&state).text()).unwrap();
        let mut back = State::default();
        kept.apply(&mut back);
        assert_eq!(Settings::of(&back), Settings::of(&state));
        assert_eq!((back.lang, back.screen_off, back.time_zone), (Lang::En, 6, zone("America/New_York")));
        assert!(!back.follow_system_zone);
        assert_eq!(back.system_zone, state.system_zone, "a zone of the system's that the pages do not list");
    }

    #[test]
    fn each_value_is_kept_as_what_it_means() {
        let system = Some(SystemZone::named("Europe/Rome").unwrap());
        let state = State { screen_off: 3, time_zone: zone("Asia/Hong_Kong"), system_zone: system, backup: [true, false, false, false, true], ..State::default() };
        let text = Settings::of(&state).text();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["screen_off"], 120, "two minutes, in seconds");
        assert_eq!(json["time_zone"], "Asia/Hong_Kong");
        assert_eq!(json["follow_system_time_zone"], true, "the system's zone is followed until one is chosen");
        assert_eq!(json["system_time_zone"], "Europe/Rome", "by the name the system gives it");
        assert_eq!(json["backup"]["parts"], serde_json::json!(["boot", "modem"]));
        assert_eq!(json["logs"]["to"], "temporary");
        assert_eq!(json["spoken"], "zh-Hans");
        assert!(json.get("language").is_none(), "no language is chosen until one is");
        let never = Settings::of(&State { screen_off: SCREEN_OFF.len() - 1, ..State::default() });
        assert_eq!(never.screen_off, 0, "a screen that stays on");
    }

    #[test]
    fn what_a_file_leaves_out_is_what_a_recovery_starts_with() {
        let kept = Settings::read(r#"{ "dark": true, "backup": { "compress": true }, "colour": "blue" }"#).unwrap();
        let mut state = State::default();
        kept.apply(&mut state);
        assert!(state.dark && state.compress);
        // 首页「其他」默认就有内容（含高级页搬过来的那些），这里只比文件里
        // 真正写下的那几项（2026-11-27）。
        let back = Settings::of(&state);
        assert_eq!((back.dark, back.backup.compress), (true, true));
        assert!(back.home_sections.is_empty() && back.home_top.is_empty(), "分栏没动就不写");
        assert!(back.home_rest.is_empty(), "「其他」还是默认就不写");
        assert!(!state.language_chosen);
        assert!(Settings::read("dark = true").is_err(), "not JSON");
    }

    #[test]
    fn values_the_pages_cannot_show_leave_the_state_as_it_is() {
        let kept = Settings {
            language: Some("not a locale".into()),
            brightness: 17.5,
            vibration: f32::NAN,
            screen_off: 7,
            time_zone: "Mars/Olympus".into(),
            system_time_zone: Some("Mars/Olympus".into()),
            backup: Backup { parts: vec!["boot".into(), "photos".into()], to: "cloud".into(), ..Backup::default() },
            logs: Logs { kinds: vec!["kernel".into()], to: "printer".into() },
            ..Settings::default()
        };
        let mut state = State::default();
        kept.apply(&mut state);
        let start = State::default();
        assert_eq!((state.brightness, state.vibration), (start.brightness, start.vibration));
        assert_eq!((state.screen_off, state.time_zone, state.backup_to, state.logs_to), (start.screen_off, start.time_zone, start.backup_to, start.logs_to));
        assert!(!state.language_chosen);
        assert_eq!(state.system_zone, None, "a zone the recovery carries no rules for");
        assert_eq!(state.backup, [true, false, false, false, false], "a part this version has not is left out");
        assert_eq!(state.logs, [false, true]);
    }

    #[test]
    fn changed_settings_are_written_once_they_have_stood_for_a_moment() {
        let mut keeper = Keeper::new(&Kept::Settings(Box::default()));
        assert_eq!(keeper.due(Settings::default(), 0), None, "what the file holds is not written again");
        let dark = Settings { dark: true, ..Settings::default() };
        assert_eq!(keeper.due(dark.clone(), 1_000), None);
        let dim = Settings { brightness: 0.5, ..dark.clone() };
        assert_eq!(keeper.due(dim.clone(), 2_500), None, "a change waits again");
        assert_eq!(keeper.due(dim.clone(), 4_000), None);
        assert_eq!(keeper.due(dim.clone(), 4_500), Some(dim.clone()), "having stood for the wait since the last change");
        keeper.written(dim.clone());
        assert_eq!(keeper.due(dim.clone(), 9_000), None, "and once");
        assert_eq!(keeper.due(Settings::default(), 9_000), None, "going back starts another wait");
        assert_eq!(keeper.due(Settings::default(), 11_000), Some(Settings::default()));
    }

    #[test]
    fn settings_that_failed_to_be_written_wait_for_the_next_change() {
        let mut keeper = Keeper::new(&Kept::Nothing);
        let dark = Settings { dark: true, ..Settings::default() };
        keeper.due(dark.clone(), 0);
        assert_eq!(keeper.due(dark.clone(), 2_000), Some(dark.clone()), "an empty file takes what the pages show");
        keeper.failed();
        assert_eq!(keeper.due(dark.clone(), 60_000), None, "not tried at every look");
        let light = Settings::default();
        keeper.due(light.clone(), 61_000);
        assert_eq!(keeper.due(light.clone(), 63_000), Some(light));
    }

    #[test]
    fn a_file_that_could_not_be_read_is_left_alone_until_its_place_is_made_afresh() {
        let mut keeper = Keeper::new(&Kept::Unreadable);
        let dark = Settings { dark: true, ..Settings::default() };
        keeper.due(dark.clone(), 0);
        assert_eq!(keeper.due(dark.clone(), 10_000), None);
        keeper.remade();
        keeper.due(dark.clone(), 10_000);
        assert_eq!(keeper.due(dark.clone(), 12_000), Some(dark.clone()));
        keeper.written(dark.clone());
        keeper.remade();
        keeper.due(dark.clone(), 20_000);
        assert_eq!(keeper.due(dark.clone(), 22_000), Some(dark), "erasing takes the file with it, so it is written again");
    }
}
