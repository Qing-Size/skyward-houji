//! 首页的一栏栏：有哪些、按什么顺序摆、什么时候不摆（2026-11-27）。
//!
//! 分类（想建几个建几个）各有名字与自己的条目；没进任何分类的
//! 条目落在「其他」那一栏里（[`State::home_rest`]）。两边合起来必须正好
//! 覆盖每一条一次，[`normalize`] 负责把它修成这样。

use crate::app::Screen;
use crate::icons::Icon;
use crate::lang::Lang;

/// 首页可以摆的一条。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Wlan,
    Files,
    Modules,
    DataApps,
    Install,
    Backup,
    Restore,
    Systems,
    Settings,
    Reboot,
    Wipe,
    SkywardUpdate,
    /// 打开首页排序页；它自己也是一条，可以摆在首页或任何分类里。
    HomeOrder,
    // 高级页的那几条：从高级页「删掉」就是搬到首页来，可以再排序（2026-11-27）。
    Decrypt,
    Terminal,
    Partitions,
    Serve,
    Fetch,
    Avb,
    Root,
    Update,
    Logs,
    Charge,
    Usb,
    Relabel,
    /// 文件传输（原来在设置页的 USB 那块）。
    FileTransfer,
}

/// 高级页原有的十二条，按原来的先后。
pub const ADVANCED: [Entry; 12] = [
    Entry::Decrypt,
    Entry::Terminal,
    Entry::Partitions,
    Entry::Serve,
    Entry::Fetch,
    Entry::Avb,
    Entry::Root,
    Entry::Update,
    Entry::Logs,
    Entry::Charge,
    Entry::Usb,
    Entry::Relabel,
];

/// 默认顺序：与「去掉分组之前」从上到下一致。
pub const DEFAULT_ORDER: [Entry; 14] = [
    Entry::Wlan,
    // 文件传输紧跟 WLAN：它俩都是「联不联得出去」，摆一起顺手（2026-11-27）。
    Entry::FileTransfer,
    Entry::Files,
    Entry::Modules,
    Entry::DataApps,
    Entry::Install,
    Entry::Backup,
    Entry::Restore,
    Entry::Systems,
    Entry::Settings,
    Entry::Reboot,
    Entry::Wipe,
    Entry::SkywardUpdate,
    Entry::HomeOrder,
];

/// 一栏：名字与其中的条目，按摆的顺序。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub entries: Vec<Entry>,
    /// 栏目头上那个图标，从全部图标里挑（2026-11-27）。
    pub icon: Icon,
    /// 在首页折起来了（2026-11-27）。
    pub collapsed: bool,
}

impl Entry {
    /// 写进设置文件的短名字；改了就再读不回来，别动。
    pub fn id(self) -> &'static str {
        match self {
            Entry::Wlan => "wlan",
            Entry::Files => "files",
            Entry::Modules => "modules",
            Entry::DataApps => "data_apps",
            Entry::Install => "install",
            Entry::Backup => "backup",
            Entry::Restore => "restore",
            Entry::Systems => "systems",
            Entry::Settings => "settings",
            Entry::Reboot => "reboot",
            Entry::Wipe => "wipe",
            Entry::SkywardUpdate => "skyward_update",
            Entry::HomeOrder => "home_order",
            Entry::Decrypt => "decrypt",
            Entry::Terminal => "terminal",
            Entry::Partitions => "partitions",
            Entry::Serve => "serve",
            Entry::Fetch => "fetch",
            Entry::Avb => "avb",
            Entry::Root => "root",
            Entry::Update => "update",
            Entry::Logs => "logs",
            Entry::Charge => "charge",
            Entry::Usb => "usb",
            Entry::Relabel => "relabel",
            Entry::FileTransfer => "file_transfer",
        }
    }

    pub fn from_id(id: &str) -> Option<Entry> {
        Some(match id {
            "wlan" => Entry::Wlan,
            "files" => Entry::Files,
            "modules" => Entry::Modules,
            "data_apps" => Entry::DataApps,
            "install" => Entry::Install,
            "backup" => Entry::Backup,
            "restore" => Entry::Restore,
            "systems" => Entry::Systems,
            "settings" => Entry::Settings,
            "reboot" => Entry::Reboot,
            "wipe" => Entry::Wipe,
            "skyward_update" => Entry::SkywardUpdate,
            "home_order" => Entry::HomeOrder,
            "decrypt" => Entry::Decrypt,
            "terminal" => Entry::Terminal,
            "partitions" => Entry::Partitions,
            "serve" => Entry::Serve,
            "fetch" => Entry::Fetch,
            "avb" => Entry::Avb,
            "root" => Entry::Root,
            "update" => Entry::Update,
            "logs" => Entry::Logs,
            "charge" => Entry::Charge,
            "usb" => Entry::Usb,
            "relabel" => Entry::Relabel,
            "file_transfer" => Entry::FileTransfer,
            _ => return None,
        })
    }

    /// 名字，排序页与首页用的是同一句。
    pub fn name(self, l: Lang) -> &'static str {
        match self {
            Entry::Wlan => l.pick(["无线网络", "無線網路", "WLAN"]),
            Entry::Files => l.pick(["文件管理", "檔案管理", "File manager"]),
            Entry::Modules => l.pick(["模块管理", "模組管理", "Modules"]),
            Entry::DataApps => l.pick(["应用管理", "應用程式管理", "Apps"]),
            Entry::Install => l.pick(["安装刷机", "安裝刷機", "Install"]),
            Entry::Backup => l.pick(["创建备份", "建立備份", "Backup"]),
            Entry::Restore => l.pick(["恢复备份", "還原備份", "Restore"]),
            Entry::Systems => l.pick(["第二系统", "第二系統", "Dual boot"]),
            Entry::Settings => l.pick(["选项设置", "選項設定", "Options"]),
            Entry::Reboot => l.pick(["重启系统", "重新啟動", "Reboot"]),
            Entry::Wipe => l.pick(["清除数据", "清除資料", "Wipe data"]),
            Entry::SkywardUpdate => l.pick(["版本更新", "版本更新", "Version update"]),
            Entry::HomeOrder => l.pick(["首页排序", "首頁排序", "Home order"]),
            Entry::Decrypt => l.pick(["解密数据", "解密資料", "Decrypt data"]),
            Entry::Terminal => l.pick(["模拟终端", "模擬終端", "Terminal emulator"]),
            Entry::Partitions => l.pick(["分区管理", "分割區管理", "Partitions"]),
            Entry::Serve => l.pick(["文件服务", "檔案服務", "Serve files"]),
            Entry::Fetch => l.pick(["链接下载", "連結下載", "Fetch a link"]),
            Entry::Avb => l.pick(["校验启动", "校驗啟動", "Verified boot"]),
            Entry::Root => l.pick(["Root管理", "Root管理", "Root"]),
            Entry::Update => l.pick(["系统更新", "系統更新", "System update"]),
            Entry::Logs => l.pick(["导出日志", "匯出記錄", "Export logs"]),
            Entry::Charge => l.pick(["充电设置", "充電設定", "Charging"]),
            Entry::Usb => l.pick(["U 盘管理", "USB 隨身碟管理", "USB drive"]),
            Entry::Relabel => l.pick(["文件标签", "檔案標籤", "Relabel"]),
            Entry::FileTransfer => l.pick(["文件传输", "檔案傳輸", "File transfer"]),
        }
    }

    /// 这一条在页面上显示的名字：自己起的名字优先，没有就用页面自带的。
    /// 自定义名按 `Entry::id()` 存在设置里（2026-11-27）。
    pub fn shown<'a>(self, l: Lang, names: &'a std::collections::BTreeMap<String, String>) -> &'a str {
        match names.get(self.id()) {
            Some(name) if !name.is_empty() => name.as_str(),
            _ => self.name(l),
        }
    }

    /// 点它要做的事：多数是开一页，U 盘那条是进 OTG。
    pub fn action(self) -> crate::app::Action {
        match self {
            Entry::Usb => crate::app::Action::FilesOtg,
            _ => crate::app::Action::Open(self.screen()),
        }
    }

    /// 打开哪一页。
    pub fn screen(self) -> Screen {
        match self {
            Entry::Wlan => Screen::Wlan,
            Entry::Files => Screen::Files,
            Entry::Modules => Screen::Modules,
            Entry::DataApps => Screen::DataApps,
            Entry::Install => Screen::Install,
            Entry::Backup => Screen::Backup,
            Entry::Restore => Screen::Restore,
            Entry::Systems => Screen::Systems,
            Entry::Settings => Screen::Settings,
            Entry::Reboot => Screen::Reboot,
            Entry::Wipe => Screen::Wipe,
            Entry::SkywardUpdate => Screen::SkywardUpdate,
            Entry::HomeOrder => Screen::HomeOrder,
            Entry::Decrypt => Screen::Decrypt,
            Entry::Terminal => Screen::Terminal,
            Entry::Partitions => Screen::Partitions,
            Entry::Serve => Screen::Serve,
            Entry::Fetch => Screen::Fetch,
            Entry::Avb => Screen::Avb,
            Entry::Root => Screen::Root,
            Entry::Update => Screen::Update,
            Entry::Logs => Screen::Logs,
            Entry::Charge => Screen::Charge,
            Entry::Usb => Screen::Files,
            Entry::Relabel => Screen::Relabel,
            Entry::FileTransfer => Screen::FileTransfer,
        }
    }

    /// 图标交给首页画行时决定颜色，形状在这里。
    pub fn icon(self) -> Icon {
        match self {
            Entry::Wlan => Icon::Wlan,
            Entry::Files => Icon::Folder,
            Entry::Modules | Entry::DataApps => Icon::Layers,
            Entry::Install | Entry::SkywardUpdate => Icon::Install,
            Entry::Backup => Icon::Backup,
            Entry::Restore => Icon::Restore,
            Entry::Systems => Icon::Systems,
            Entry::Settings => Icon::Gear,
            Entry::Reboot => Icon::Power,
            Entry::Wipe => Icon::Wipe,
            Entry::HomeOrder => Icon::More,
            Entry::Decrypt => Icon::Lock,
            Entry::Terminal => Icon::Terminal,
            Entry::Partitions => Icon::Layers,
            Entry::Serve | Entry::Fetch => Icon::Globe,
            Entry::Avb | Entry::Relabel => Icon::Shield,
            Entry::Root => Icon::KernelSu,
            Entry::Update => Icon::Install,
            Entry::Logs => Icon::Log,
            Entry::Charge => Icon::Battery,
            Entry::Usb => Icon::UsbDrive,
            Entry::FileTransfer => Icon::Usb,
        }
    }
}

/// 首页「其他」的默认内容：核心条目，加上高级页那十二条（高级页默认不再列它们）。
pub fn default_rest() -> Vec<Entry> {
    let mut rest = DEFAULT_ORDER.to_vec();
    // 高级页那十二条现在就是普通条目，默认全在「其他」里。
    rest.extend([
        Entry::Decrypt,
        Entry::Terminal,
        Entry::Partitions,
        Entry::Serve,
        Entry::Fetch,
        Entry::Avb,
        Entry::Root,
        Entry::Update,
        Entry::Logs,
        Entry::Charge,
        Entry::Usb,
        Entry::Relabel,
    ]);
    rest
}

/// 内置的那一栏：设置里还没有任何分栏时给一栏「刷机与备份」，把最常干的
/// 三件事收进去；改名、删掉都随用户（2026-11-27）。
pub fn default_sections(l: Lang) -> Vec<Section> {
    vec![Section {
        name: l.pick(["刷机与备份", "刷機與備份", "Flash & backup"]).to_string(),
        entries: vec![Entry::Install, Entry::Backup, Entry::Restore],
        icon: Icon::Install,
        collapsed: false,
    }]
}

/// 把「分类 + 其他」修成不重不漏：分类里先到的留下，后到的重复丢掉；
/// 没被任何地方提到的条目按默认顺序补进「其他」。
pub fn normalize(sections: &mut Vec<Section>, top: &mut Vec<Entry>, rest: &mut Vec<Entry>) {
    let mut seen: Vec<Entry> = Vec::new();
    top.retain(|entry| {
        if seen.contains(entry) {
            false
        } else {
            seen.push(*entry);
            true
        }
    });
    for section in sections.iter_mut() {
        section.entries.retain(|entry| {
            if seen.contains(entry) {
                false
            } else {
                seen.push(*entry);
                true
            }
        });
    }
    rest.retain(|entry| {
        if seen.contains(entry) {
            false
        } else {
            seen.push(*entry);
            true
        }
    });
    // 每一条都得在某个地方：分类里、置顶里，或者「其他」里。
    for entry in default_rest() {
        if !seen.contains(&entry) {
            rest.push(entry);
        }
    }
}

/// 某一条现在在哪一栏：`None` 是「其他」，`Some(0..)` 是第几个分类。
pub fn section_of(sections: &[Section], entry: Entry) -> Option<usize> {
    sections.iter().position(|s| s.entries.contains(&entry))
}

/// 从它现在所在的地方拿掉。
pub fn detach(sections: &mut [Section], top: &mut Vec<Entry>, rest: &mut Vec<Entry>, entry: Entry) {
    for section in sections.iter_mut() {
        section.entries.retain(|e| *e != entry);
    }
    top.retain(|e| *e != entry);
    rest.retain(|e| *e != entry);
}

/// 「其他」那一栏还是默认顺序吗（是的话不写进设置文件）。
pub fn rest_is_default(rest: &[Entry]) -> bool {
    rest == default_rest()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Action, App, Screen};

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn ids_go_both_ways() {
        for entry in DEFAULT_ORDER {
            assert_eq!(Entry::from_id(entry.id()), Some(entry));
        }
        assert_eq!(Entry::from_id("nope"), None);
    }

    #[test]
    fn a_stored_order_reads_back_and_gets_missing_entries_appended() {
        let mut order: Vec<Entry> = ids(&["restore", "install", "nonsense"])
            .iter()
            .filter_map(|id| Entry::from_id(id))
            .collect();
        let mut sections = Vec::new();
        normalize(&mut sections, &mut Vec::new(), &mut order);
        assert_eq!(&order[..2], &[Entry::Restore, Entry::Install], "记住的先后照旧");
        assert_eq!(order.len(), default_rest().len(), "一条不少");
        let mut seen = std::collections::BTreeSet::new();
        for entry in &order {
            assert!(seen.insert(entry.id()), "没有重复的条目");
        }
    }

    #[test]
    fn a_section_takes_entries_out_of_the_rest_bucket_and_duplicates_fall_away() {
        let mut sections = vec![
            Section { name: "日常".into(), entries: vec![Entry::Files, Entry::Wlan, Entry::Files], icon: Icon::Layers, collapsed: false },
            Section { name: "刷机".into(), entries: vec![Entry::Install], icon: Icon::Layers, collapsed: false },
        ];
        let mut rest = vec![Entry::Install, Entry::Backup];
        normalize(&mut sections, &mut Vec::new(), &mut rest);
        assert_eq!(sections[0].entries, vec![Entry::Files, Entry::Wlan], "重复的丢掉");
        assert!(!rest.contains(&Entry::Install), "分类拿走的不会同时留在其他");
        assert_eq!(rest[0], Entry::Backup, "其他里原有的先后不变");
        assert_eq!(sections.len(), 2);
    }


    #[test]
    fn an_entry_moves_into_a_section_and_back_to_others() {
        use crate::app::testing::*;
        use crate::app::{Action, App, Screen};
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.home_sections.push(crate::app::Section { name: "日常".into(), entries: vec![], icon: Icon::Layers, collapsed: false });
        let files = Entry::Files;
        app.act(Action::HomePick(files), 0);
        assert_eq!(app.state.home_pick, Some(files));
        app.act(Action::HomeToSection(Some(0)), 0);
        assert_eq!(app.state.home_sections[0].entries, vec![files], "进了那一栏");
        assert!(!app.state.home_rest.contains(&files), "不在其他里了");
        app.act(Action::HomePick(files), 0);
        app.act(Action::HomeToSection(None), 0);
        assert!(app.state.home_sections[0].entries.is_empty());
        assert!(app.state.home_rest.contains(&files));
        // 删栏：里面的条目回到「其他」。
        app.state.home_sections[0].entries.push(files);
        app.state.home_rest.retain(|e| *e != files);
        app.act(Action::HomeDeleteSection(0), 0);
        assert!(app.state.home_sections.is_empty());
        assert!(app.state.home_rest.contains(&files), "删栏后回到其他");
        app.act(Action::HomeReset, 0);
        // 恢复默认为「刷机与备份」+ 其余默认顺序：那三条不在「其他」里。
        assert_eq!(app.state.home_sections.len(), 1, "恢复默认给出内置那一栏");
        assert!(!app.state.home_rest.contains(&Entry::Install), "三条在分类里，不重复出现在其他");
        assert_eq!(app.state.home_rest.len(), default_rest().len() - 3);
    }

    #[test]
    fn placing_before_an_entry_keeps_it_inside_that_entry_s_section() {
        use crate::app::testing::*;
        use crate::app::{Action, App, Screen};
        let mut app = App::new(Screen::Home, W, H, PPD);
        let (backup, restore, install) = (Entry::Backup, Entry::Restore, Entry::Install);
        app.state.home_sections.push(crate::app::Section { name: "刷机".into(), entries: vec![install, backup], icon: Icon::Layers, collapsed: false });
        for entry in [install, backup] {
            app.state.home_rest.retain(|e| *e != entry);
        }
        app.act(Action::HomePick(restore), 0);
        app.act(Action::HomePlace(backup), 0);
        assert_eq!(app.state.home_sections[0].entries, vec![install, restore, backup], "插在目标条目前面");
        assert!(!app.state.home_rest.contains(&restore));
    }

    #[test]
    fn adopting_from_others_fills_the_section_being_opened() {
        use crate::app::testing::*;
        use crate::app::{Action, App, Screen};
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.home_sections.push(crate::app::Section { name: "常用".into(), entries: vec![], icon: Icon::Layers, collapsed: false });
        app.state.section_view = Some(0);
        let logs = Entry::Modules;
        app.act(Action::HomeAdopt(logs), 0);
        assert_eq!(app.state.home_sections[0].entries, vec![logs]);
        assert!(!app.state.home_rest.contains(&logs));
    }

    #[test]
    fn the_settings_keep_sections_and_leave_the_default_out_of_the_file() {
        use crate::app::testing::*;
        let mut app = App::new(Screen::Home, W, H, PPD);
        assert!(crate::settings::Settings::of(&app.state).home_sections.is_empty(), "没有分类就不写");
        app.state.home_sections.push(crate::app::Section { name: "常用".into(), entries: vec![Entry::Files], icon: Icon::Layers, collapsed: false });
        let kept = crate::settings::Settings::of(&app.state);
        assert_eq!(kept.home_sections.len(), 1);
        assert_eq!(kept.home_sections[0].name, "常用");
        let mut state = app.state.clone();
        kept.apply(&mut state);
        assert_eq!(state.home_sections[0].name, "常用");
        assert_eq!(state.home_sections[0].entries, vec![Entry::Files], "读回来一模一样");
    }

    #[test]
    fn an_entry_can_stand_on_top_of_the_home_outside_any_section() {
        use crate::app::testing::*;
        use crate::app::{Action, App, Screen};
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.home_sections.push(crate::app::Section { name: "刷机".into(), entries: vec![Entry::Install], icon: Icon::Layers, collapsed: false });
        app.state.home_rest.retain(|e| *e != Entry::Install);
        let wlan = Entry::Wlan;
        app.act(Action::HomePick(wlan), 0);
        app.act(Action::HomeToTop, 0);
        assert_eq!(app.state.home_top, vec![wlan], "单独摆在最上面");
        assert!(!app.state.home_rest.contains(&wlan), "不再在「其他」里");
        assert!(!app.state.home_sections[0].entries.contains(&wlan), "也不在任何分类里");
        let kept = crate::settings::Settings::of(&app.state);
        assert_eq!(kept.home_top, vec!["wlan".to_string()], "置顶的条目写进设置");
        app.act(Action::HomeReset, 0);
        assert!(app.state.home_top.is_empty(), "恢复默认后置顶清空");
        assert_eq!(app.state.home_sections.len(), 1, "恢复默认给出内置那一栏");
        assert_eq!(app.state.home_rest.len(), default_rest().len() - 3);
    }

    #[test]
    fn the_built_in_section_shows_first_and_lands_in_the_state_on_the_first_touch() {
        use crate::app::testing::*;
        use crate::app::{Action, App, Screen};
        let mut app = App::new(Screen::Home, W, H, PPD);
        assert!(app.state.home_sections.is_empty(), "设置里还没有分栏");
        let (f, i) = (fonts(), info());
        let _ = app.frame(&f, &i); // 首页照样画出内置那一栏
        assert!(app.state.home_sections.is_empty(), "只是画出来，还没落进 state");
        app.act(Action::HomePick(Entry::Files), 0);
        assert_eq!(app.state.home_sections.len(), 1, "一动它就落进 state");
        assert_eq!(app.state.home_sections[0].name, "刷机与备份");
        assert_eq!(
            app.state.home_sections[0].entries,
            vec![Entry::Install, Entry::Backup, Entry::Restore],
            "里面是安装、备份、恢复"
        );
        assert!(!app.state.home_rest.contains(&Entry::Install), "不重复出现在「其他」");
    }

    #[test]
    fn a_folded_section_and_folded_others_are_kept_folded() {
        use crate::app::testing::*;
        use crate::app::{Action, App, Screen};
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.home_sections.push(crate::app::Section { name: "刷机".into(), entries: vec![Entry::Install], icon: Icon::Layers, collapsed: false });
        app.state.home_rest.retain(|e| *e != Entry::Install);
        app.act(Action::HomeFoldSection(0), 0);
        assert!(app.state.home_sections[0].collapsed, "点一下就折起来");
        app.act(Action::HomeFoldRest, 0);
        assert!(app.state.home_rest_collapsed);
        let kept = crate::settings::Settings::of(&app.state);
        assert!(kept.home_sections[0].collapsed, "折叠状态写进设置");
        assert!(kept.home_rest_collapsed, "「其他」的折叠也写进去");
        let mut state = app.state.clone();
        kept.apply(&mut state);
        assert!(state.home_sections[0].collapsed, "重启后还是折着的");
        assert!(state.home_rest_collapsed);
        app.act(Action::HomeFoldSection(0), 0);
        app.act(Action::HomeFoldRest, 0);
        assert!(!app.state.home_sections[0].collapsed && !app.state.home_rest_collapsed, "再点一下又展开");
    }

    #[test]
    fn a_section_keeps_the_icon_chosen_for_it() {
        use crate::app::testing::*;
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.home_sections.push(crate::app::Section {
            name: "刷机".into(),
            entries: vec![Entry::Install],
            icon: crate::icons::Icon::Terminal,
            collapsed: false,
        });
        let kept = crate::settings::Settings::of(&app.state);
        assert_eq!(kept.home_sections[0].icon, "terminal", "图标按短名字存");
        let mut state = app.state.clone();
        kept.apply(&mut state);
        assert_eq!(state.home_sections[0].icon, crate::icons::Icon::Terminal, "读回来还是那枚");
    }

    #[test]
    fn the_others_section_can_be_renamed_but_not_removed() {
        use crate::app::testing::*;
        use crate::app::{Action, App, Screen};
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.act(Action::HomeRenameRest, 0);
        assert_eq!(app.screen(), Screen::HomeSection, "改「其他」的名字走同一页");
        assert!(app.state.section_rest, "改的是「其他」");
        app.state.section_name = "杂项".into();
        app.act(Action::HomeSectionDone, 0);
        assert_eq!(app.state.home_rest_name, "杂项");
        let kept = crate::settings::Settings::of(&app.state);
        assert_eq!(kept.home_rest_name, "杂项", "名字写进设置");
        let mut state = app.state.clone();
        kept.apply(&mut state);
        assert_eq!(state.home_rest_name, "杂项", "读回来还在");
        app.act(Action::HomeReset, 0);
        assert!(app.state.home_rest_name.is_empty(), "恢复默认把名字也还原");
    }

    #[test]
    fn an_old_settings_file_still_gets_the_file_transfer_entry() {
        use crate::app::testing::*;
        use crate::app::{Action, App, Screen};
        // 用户机器上已有的设置：有分栏、有「其他」，但里面没有 file_transfer。
        let kept = crate::settings::Settings::read(
            r#"{"home_sections":[{"name":"刷机","entries":["install"]}],
                "home_rest":["wlan","files","modules","data_apps","systems","advanced","settings","reboot","wipe","skyward_update","home_order"]}"#,
        )
        .expect("reads");
        let mut app = App::new(Screen::Home, W, H, PPD);
        kept.apply(&mut app.state);
        assert!(app.state.home_rest.contains(&Entry::FileTransfer), "旧的设置里没有也要补上");
        let (f, i) = (fonts(), info());
        app.state.home_sections.iter_mut().for_each(|s| s.collapsed = false);
        app.state.home_rest_collapsed = false;
        app.refresh();
        let fr = app.frame(&f, &i);
        let acts: Vec<String> = fr.hits.iter().map(|h| format!("{:?}", h.action)).collect();
        assert!(
            acts.iter().any(|a| a.contains("FileTransfer")),
            "首页上有文件传输那条；实际命中：[{}]",
            acts.join(", ")
        );
    }

}
