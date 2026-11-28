use crate::tr;
use crate::app::{
    Action, BackupAt, BackupEnd, BackupJob, Check, ClashChoice, Confirm, Damage, Data, Dock, DownloadEnd, EraseStop, Field, FileKind,
    AppsScope, FileOp, FileTask, Fold, Folder, Frame, FsAction, FsChoice, FsRow, Have, HeroArt, Hit, Install, JobEnd, Keys, Pick, Place,
    ChargeMode, Region, Rows, Screen, SearchScope, SkywardCheck, SkywardEnd, SkywardJob, SkywardStage, Slider, SortKey, Source,
    State, Switch, THERMAL_LIMIT_LEAST, THERMAL_LIMIT_MOST, AutoRoot, KernelSuEnd, KernelSuJob, KernelSuStage, is_newer,
    SystemsWork, Target, UsbDrive, UsbLink, Wipe, Entry,
    Fetch, ImageAction, ImageCopy, RecordSetting, Serving, VbmetaCopy,
    ADDABLE, BACKUP_PARTS, DATA_ERASED, DATA_PART, DATA_PARTITION, DOWNLOADS, FASTBOOT_CHOICES, MEDIA_PART, SCREEN_OFF, TARGETS,
    TIME_ZONES, UTC, USB_DRIVE_UNREADABLE, USB_DRIVE_WANTED, WIPES, installs, listed, starting,
};
use crate::canvas::Canvas;
use crate::color::Rgba;
use crate::editor;
use crate::files;
use crate::fmt::{self, Verb};
use crate::font::Fonts;
use crate::geom::Rect;
use crate::lang::{same, Lang, Text};
use crate::search;
use crate::term::{self, Term};
use crate::texts;
use crate::theme::Theme;
use crate::icons::{self, Icon};
use crate::widgets::{self, Caption, Detail, Fact, Outline, Row, Spec, SystemCard, SystemMark, Tone, Trail};
use skyward_info::{DeviceInfo, RUNNING, VERSION};
use skyward_install::{Outcome, Place as FlashPlace, Reflash, Refusal, Stage};
use skyward_net::download::Progress;
use skyward_net::supplicant::Security;
use skyward_net::wlan::{Bands, Network, Power, Stage as Joining};
use skyward_systems::{Held, Who};
use skyward_vab::{Slot, Standing};
use std::path::Path;

// The colours the stock Settings fills its icons' tiles with, the same in
// light and dark mode.
const BLUE: Rgba = Rgba::rgb(0x34, 0x82, 0xff);
const GREEN: Rgba = Rgba::rgb(0x1d, 0xcd, 0x3a);
const PURPLE: Rgba = Rgba::rgb(0x77, 0x67, 0xf9);
const SLATE: Rgba = Rgba::rgb(0x8c, 0x9d, 0xb0);
const ORANGE: Rgba = Rgba::rgb(0xff, 0x69, 0x00);
const RED: Rgba = Rgba::rgb(0xfa, 0x38, 0x2e);

/// What every page draws from: style, typefaces, the device readings, and
/// what the pages hold, from the choices made on them to where the data
/// partition stands.
pub struct Ctx<'a> {
    pub t: &'a Theme,
    pub f: &'a Fonts,
    pub info: &'a DeviceInfo,
    pub state: &'a State,
    // The shell the terminal page shows, once one has started.
    pub term: Option<&'a Term>,
    // The part of the page to draw; all of it if none is given. The page
    // is laid out whole either way.
    pub band: Option<Band>,
}

/// Rows `rows` of a page, and of them the columns `cols`, each from the
/// first to one past the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Band {
    pub rows: (u32, u32),
    pub cols: (u32, u32),
}

pub fn render(screen: Screen, cx: &Ctx) -> Frame {
    match screen {
        Screen::Decrypt => decrypt(cx),
        Screen::Home => home(cx),
        Screen::Settings => settings(cx),
        Screen::HomeOrder => home_order(cx),
        Screen::HomeSection => home_section(cx),
        Screen::HomeSectionView => home_section_view(cx),
        Screen::HomeIcon => home_icon(cx),
        Screen::EntryName => entry_name(cx),
        Screen::HomeTitle => home_title(cx),
        Screen::Layout => layout(cx),
        Screen::LayoutBrowse => layout_browse(cx),
        Screen::Reboot => reboot(cx),
        Screen::Wipe => wipe(cx),
        Screen::Mount => mount(cx),
        Screen::Partitions => partitions(cx),
        Screen::Install => install(cx),
        Screen::Choose(place) => choose(cx, place),
        Screen::Installing => installing(cx),
        Screen::About => about(cx),
        Screen::Specs => specs(cx),
        Screen::Root => root(cx),
        Screen::KernelSu => kernel_su(cx),
        Screen::KernelSuKmi => kernel_su_kmi(cx),
        Screen::Magisk => magisk(cx),
        Screen::Charge => charge(cx),
        Screen::FileTransfer => file_transfer(cx),
        Screen::ScreenOff => screen_off(cx),
        Screen::Search => search(cx),
        Screen::Backup => backup(cx),
        Screen::Restore => restore(cx),
        Screen::RestoreBackup => restore_backup(cx),
        Screen::BackupJob => backup_job(cx),
        Screen::Tasks => crate::tools::tasks_page(cx),
        Screen::PowerOff => power_off(cx),
        Screen::ImageBackup => image_backup(cx),
        Screen::Record => record(cx),
        Screen::Modules => modules(cx),
        Screen::DataApps => data_apps(cx),
        Screen::SystemApps => system_apps(cx),
        Screen::DataAppInfo => data_app_info(cx),
        Screen::ModuleInfo => module_info(cx),
        Screen::Files => files(cx),
        Screen::FileInfo => file_info(cx),
        Screen::Permissions => permissions(cx),
        Screen::Editor => editor(cx),
        Screen::Terminal => terminal(cx),
        Screen::Relabel => relabel(cx),
        Screen::Filesystems => filesystems(cx),
        Screen::Filesystem => filesystem(cx),
        Screen::PartWork => part_work(cx),
        Screen::Language => language(cx),
        Screen::TimeZone => time_zone(cx),
        Screen::TimeZones(region) => time_zones(cx, region),
        Screen::Logs => logs(cx),
        Screen::Update => update(cx),
        Screen::Serve => serve(cx),
        Screen::Fetch => fetch(cx),
        Screen::Avb => avb(cx),
        Screen::SkywardUpdate => skyward_update(cx),
        Screen::License => license(cx),
        Screen::FontLicense => font_license(cx),
        Screen::Slot => slot(cx),
        Screen::Systems => systems(cx),
        Screen::Fastboot => fastboot(cx),
        Screen::Erase => erase(cx),
        Screen::CantStart => cant_start(cx),
        Screen::Unhandled => unhandled(cx),
        Screen::Rename => rename(cx),
        Screen::Clash => clash(cx),
        Screen::FileJob => file_job(cx),
        Screen::Wlan => wlan(cx),
        Screen::WlanPassword => wlan_password(cx),
        Screen::WlanNetwork => wlan_network(cx),
        Screen::FilesOtg => files_otg(cx),
        Screen::WlanAdd => wlan_add(cx),
        Screen::WlanStatic => wlan_static(cx),
    }
}

pub fn record(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    // 这一页比一屏长一点：底下那句说明以前被屏幕边缘切掉半行，现在按内容裁、
    // 能滚到底（2026-11-21）。
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(["这一段", "這一段", "This take"]));
    let state = match (s.record.on, s.record.paused) {
        (true, true) => l.pick(["已暂停", "已暫停", "Paused"]),
        (true, false) => l.pick(["正在录制", "正在錄影", "Recording"]),
        _ => l.pick(["没有在录", "沒有在錄", "Not recording"]),
    };
    let length = if s.record.on { s.record.elapsed.clone() } else { "—".to_string() };
    let file = if s.record.file.is_empty() { l.pick(["还没有录过", "還沒有錄過", "None yet"]).to_string() } else { s.record.file.clone() };
    let spec = if s.record.spec.is_empty() { "—".to_string() } else { s.record.spec.clone() };
    let rows = [
        (l.pick(["状态", "狀態", "State"]), state.to_string()),
        (l.pick(["已录", "已錄", "Length"]), length),
        (l.pick(["文件", "檔案", "File"]), file),
        (l.pick(["规格", "規格", "Format"]), spec),
    ];
    let y = widgets::info_card(&mut c, t, f, y, &rows);

    // 录成什么样：三条各点一下换一档。正在录的时候不让改 —— 画面大小一变，
    // 编码线程与已经写出去的文件头都得重来。
    let y = widgets::section(&mut c, t, f, y, l.pick(["录成什么样", "錄成什麼樣", "How it records"]));
    let panel = (t.m.w, t.m.h);
    let scale_names = [l.pick(["原画", "原畫", "Full"]), l.pick(["高清", "高清", "Sharp"]), l.pick(["标准", "標準", "Std"]), l.pick(["流畅", "流暢", "Smooth"])];
    let quality_names = [l.pick(["高", "高", "High"]), l.pick(["中", "中", "Mid"]), l.pick(["低", "低", "Low"])];
    let sizes: Vec<String> = (0..crate::record::SCALES.len())
        .map(|k| {
            let z = crate::record::Settings::of(panel, k, s.record.quality, s.record.fps);
            format!("{}×{}", z.width, z.height)
        })
        .collect();
    let chosen = crate::record::Settings::of(panel, s.record.scale, s.record.quality, s.record.fps);
    let per_minute = chosen.per_frame() as f64 * chosen.fps as f64 * 60.0 / 1e6;
    let values = [
        format!("{} {}", scale_names[s.record.scale.min(3)], sizes[s.record.scale.min(3)]),
        quality_names[s.record.quality.min(2)].to_string(),
        format!("{:.0} fps", crate::record::RATES[s.record.fps.min(3)]),
    ];
    let sized = format!("{}×{} · 每分钟约 {per_minute:.0} MB", chosen.width, chosen.height);
    let locked = l.pick(["正在录制，停下这一段才能改", "正在錄影，停下這一段才能改", "Recording: stop this take to change it"]);
    let rows = [
        Row::new(l.pick(["分辨率", "解析度", "Size"]))
            .icon(BLUE, Icon::Record)
            .subtitle(if s.record.on { locked } else { sized.as_str() })
            .trail(Trail::Value(values[0].as_str())),
        Row::new(l.pick(["画质", "畫質", "Quality"]))
            .icon(ORANGE, Icon::Record)
            .subtitle(if s.record.on { locked } else { l.pick(["画质越高越清楚，文件也越大", "畫質越高越清楚，檔案也越大", "Higher quality is sharper and bigger"]) })
            .trail(Trail::Value(values[1].as_str())),
        Row::new(l.pick(["帧率", "幀率", "Frame rate"]))
            .icon(GREEN, Icon::Record)
            .subtitle(if s.record.on { locked } else { l.pick(["60 帧最顺，15 帧最省；太高会补帧", "60 幀最順，15 幀最省；太高會補幀", "60 is smoothest, 15 the lightest; too high and frames repeat"]) })
            .trail(Trail::Value(values[2].as_str())),
        Row::new(l.pick(["容器", "容器", "Container"]))
            .icon(SLATE, Icon::Record)
            .subtitle(if s.record.on {
                locked
            } else {
                l.pick(["AVI 到处都能放；MP4 相册更认，但断电那份打不开", "AVI 到處都能放；MP4 相簿更認，但斷電那份打不開", "AVI plays anywhere; MP4 suits galleries, but a cut file has no index"])
            })
            .trail(Trail::Value(if s.record.format == 0 { "AVI" } else { "MP4" })),
    ];
    let list = widgets::list(&mut c, t, f, y, &rows);
    let mut out = hits(&list.rows, &[
        Action::RecordSetting(RecordSetting::Scale),
        Action::RecordSetting(RecordSetting::Quality),
        Action::RecordSetting(RecordSetting::Fps),
        Action::RecordSetting(RecordSetting::Format),
    ]);

    let doing = if s.record.on { l.pick(["停止录制", "停止錄影", "Stop recording"]) } else { l.pick(["开始录制", "開始錄影", "Start recording"]) };
    let rows = [Row::new(doing).icon(RED, Icon::Record).trail(Trail::None)];
    let list = widgets::list(&mut c, t, f, list.bottom + t.m.dp(10.0), &rows);
    out.extend(hits(&list.rows, &[Action::Record]));
    let bottom = widgets::footnote(&mut c, t, f, list.bottom, l.pick([
        "录好的文件在内部存储的 DCIM/Screenrecords，边录边写：断电最多丢最后一秒。录的时候那枚胶囊可以拖到任意位置：点左边的红点是暂停或继续，点右边的方块是结束这一段。",
        "錄好的檔案在內部儲存的 DCIM/Screenrecords，邊錄邊寫：斷電最多丟最後一秒。錄的時候那枚膠囊可以拖到任意位置：點左邊的紅點是暫停或繼續，點右邊的方塊是結束這一段。",
        "Takes go to DCIM/Screenrecords on internal storage and are written as they are recorded: a power cut loses the last second at most. Drag the pill anywhere while it runs; tap its dot to pause or carry on, tap its square to end the take.",
    ]));
    Frame::new(t, fit(c, t, bottom), out, l.pick(["录屏", "錄影", "Screen recording"]), top)
}

/// Label and colour of each swipe-to-confirm control: the filesystem
/// page's as `s` says what it is to do.
pub fn confirm_label(which: Confirm, s: &State) -> (&'static str, bool) {
    use texts::confirm;
    let (label, danger) = match which {
        Confirm::Avb => (["滑动以写入", "滑動以寫入", "Slide to write"], true),
        Confirm::Filesystem => match s.fs_action {
            FsAction::Repair => (confirm::REPAIR, true),
            FsAction::Remake(_) => (confirm::CHANGE, true),
        },
        Confirm::Reboot => (["滑动以重启", "滑動以重新啟動", "Slide to reboot"], false),
        Confirm::Record => (["滑动以录屏", "滑動以錄影", "Slide to record"], false),
        Confirm::Wipe => (["滑动以清除", "滑動以清除", "Slide to wipe"], true),
        Confirm::Flash => (["滑动以刷入", "滑動以刷入", "Slide to flash"], true),
        Confirm::Backup => (["滑动以备份", "滑動以備份", "Slide to back up"], false),
        Confirm::Restore => (["滑动以恢复", "滑動以還原", "Slide to restore"], true),
        Confirm::Slot => (["滑动以切换", "滑動以切換", "Slide to switch"], true),
        Confirm::FactoryReset => (["滑动以恢复出厂设置", "滑動以恢復原廠設定", "Slide to factory reset"], true),
        Confirm::DeleteEntry | Confirm::DeleteBackup => (["滑动以删除", "滑動以刪除", "Slide to delete"], true),
        Confirm::DeleteApp => (["滑动以卸载应用", "滑動以卸載應用程式", "Slide to uninstall"], true),
        Confirm::DeleteModule => (["滑动以卸载模块", "滑動以卸載模組", "Slide to uninstall"], true),
        Confirm::KeepLock => (["滑动以开启", "滑動以開啟", "Slide to turn on"], true),
        Confirm::Install => (INSTALL, false),
        Confirm::InstallWipe => (["滑动以安装并清除", "滑動以安裝並清除", "Slide to install and wipe"], true),
        Confirm::RunScript => (INSTALL, true),
        Confirm::Merge => (["滑动以合并", "滑動以合併", "Slide to merge"], true),
        Confirm::Relabel => (["滑动以修复", "滑動以修復", "Slide to fix"], true),
        Confirm::SwitchSystem => (["滑动以切换并重启", "滑動以切換並重新啟動", "Slide to switch and reboot"], false),
        Confirm::RemoveSystem => (["滑动以删除", "滑動以刪除", "Slide to remove"], true),
        Confirm::InstallSystem => (INSTALL, false),
        Confirm::PowerOff => (["滑动以关机", "滑動以關機", "Slide to power off"], true),
        Confirm::ImageRestore => (["滑动以写回分区", "滑動以寫回分割區", "Slide to write back"], true),
        Confirm::ImageDelete => (["滑动以删除镜像", "滑動以刪除映像檔", "Slide to delete the image"], true),
        Confirm::VbmetaRestore => (["滑动以写回 vbmeta", "滑動以寫回 vbmeta", "Slide to write the vbmeta back"], true),
        Confirm::KernelSu => (["滑动以写入 KernelSU", "滑動以寫入 KernelSU", "Slide to write KernelSU"], false),
        Confirm::Magisk => (["滑动以写入 Magisk", "滑動以寫入 Magisk", "Slide to write Magisk"], false),
    };
    (s.lang.pick(label), danger)
}

/// The home page's large title.
pub const HOME_TITLE: &str = "Skyward";

/// Where the phone can restart to, and what is said under the name of one
/// that wants it.
const REBOOT_TARGETS: [(Text, Option<Text>); 6] = [
    (texts::reboot::target::SYSTEM, None),
    (texts::reboot::target::RECOVERY, None),
    (same("Fastboot"), Some(texts::reboot::target::FASTBOOT_NOTE)),
    (same("Bootloader"), None),
    (same("EDL"), Some(texts::reboot::target::EDL_NOTE)),
    (texts::reboot::target::POWER_OFF, None),
];

/// What pages are called both on the rows that open them and in their own
/// titles.
// 上游 v0.5.0 把这些文案搬进了 `locales/*.toml`，本工程的语言表裁到三种，
// 页面上仍用这三语的常量（2026-11-22）。
const INSTALL: Text = ["滑动以安装", "滑動以安裝", "Slide to install"];
const UNREADABLE: Text = ["无法读取", "無法讀取", "Unreadable"];
pub const SEARCH_HINT: Text = ["搜索功能与分区", "搜尋功能與分割區", "Search features and partitions"];
const RENAME: Text = ["重命名", "重新命名", "Rename"];
const RELABEL: Text = ["修复 SELinux 上下文", "修復 SELinux 上下文", "Fix SELinux contexts"];
const NOTES: Text = ["注意事项", "注意事項", "Notes"];
const CANCEL: Text = ["取消", "取消", "Cancel"];

const WLAN: Text = ["无线网络", "無線網路", "WLAN"];
/// Whether the WLAN opens as the recovery starts.
const WLAN_AUTO: Text = ["WiFi 自动连接", "WiFi 自動連線", "WLAN on at start"];
/// Whether the password of the network joined last is kept with it.
const WLAN_KEEP: Text = ["记住 WiFi 密码", "記住 WiFi 密碼", "Keep the password"];
const DECRYPT: Text = ["解密", "解密", "Decrypt"];
const CHARGE: Text = ["充电设置", "充電設定", "Charge settings"];
const ADVANCED: Text = ["高级工具", "進階工具", "Advanced"];
const SETTINGS: Text = ["选项设置", "選項設定", "Options"];
const REBOOT: Text = ["重启系统", "重新啟動", "Reboot"];
const ABOUT: Text = ["关于本机", "關於手機", "About phone"];
const SPECS: Text = ["全部参数与信息", "全部規格與資訊", "All specs"];
const LOGS: Text = ["导出日志", "匯出記錄", "Export logs"];
const SCREEN_TIMEOUT: Text = ["息屏时间", "螢幕自動關閉", "Screen timeout"];
const UPDATE: Text = ["系统更新", "系統更新", "System update"];
const SKYWARD_UPDATE: Text = ["版本更新", "版本更新", "Version update"];
const FILES: Text = ["文件管理", "檔案管理", "File manager"];
const TERMINAL: Text = ["模拟终端", "模擬終端", "Terminal emulator"];
const LANGUAGE: Text = ["语言", "語言", "Language"];
const TIME_ZONE: Text = ["时区", "時區", "Time zone"];
const LICENSE: Text = ["开源许可", "開放原始碼授權", "Open source licenses"];
const FONTS: Text = ["界面字体", "介面字型", "UI fonts"];
const DUAL_BOOT: Text = ["第二系统", "第二系統", "Dual boot"];

/// A canvas for a page `h` tall in the colour `bg`, holding the rows `cx`
/// asks to be drawn.
pub(crate) fn sheet(cx: &Ctx, h: u32, bg: Rgba) -> Canvas {
    let w = cx.t.m.w;
    let Band { rows, cols } = cx.band.unwrap_or(Band { rows: (0, h), cols: (0, w) });
    Canvas::band(w, h, rows.0..rows.1, cols.0..cols.1, bg)
}

pub(crate) fn page(cx: &Ctx) -> Canvas {
    sheet(cx, cx.t.m.h, cx.t.pal.bg)
}

/// A canvas tall enough for a scrolling page; [`fit`] trims it to the content.
pub(crate) fn long_page(cx: &Ctx) -> Canvas {
    long_page_n(cx, 3)
}

/// 一个可以比三屏更高的滚动页：`screens` 屏那么高，给分区表、镜像列表这类
/// 长度不由页面决定的页面用。绘制是分带的，画布高多少就能滚到多少。
pub(crate) fn long_page_n(cx: &Ctx, screens: u32) -> Canvas {
    sheet(cx, cx.t.m.h * screens, cx.t.pal.bg)
}

/// How tall a long page whose content ends at `bottom` is: to the bottom
/// margin after it, and never shorter than one screen.
fn fitted(t: &Theme, bottom: f32) -> u32 {
    ((bottom + t.m.margin * 2.0).ceil() as u32).max(t.m.h)
}

/// Trim a long page to its content plus the bottom margin, and never shorter
/// than one screen.
pub(crate) fn fit(c: Canvas, t: &Theme, bottom: f32) -> Canvas {
    c.crop_height(fitted(t, bottom))
}

pub(crate) fn hits(rects: &[Rect], actions: &[Action]) -> Vec<Hit> {
    rects.iter().zip(actions).map(|(&rect, &action)| Hit { rect, action }).collect()
}

/// Recovery home, laid out like the Settings root: title, search, a device
/// card, then grouped entries. The status bar carries the readings: the
/// slot, the data partition's lock and the battery's temperature.
pub fn home(cx: &Ctx) -> Frame {
    let (t, f, i, s, l) = (cx.t, cx.f, cx.info, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    // 搜索栏回到首页顶部：占住自己的一块位置，页面往上滚时折起来。
    // 点它进搜索页，能按名字找功能，也能找分区（2026-11-25）。
    let bar = Rect::new(t.m.margin, t.m.content_top, t.m.w as f32 - t.m.margin * 2.0, t.m.search_h);
    let mut y = bar.y + bar.h + t.m.after_search;
    let cards_top = y;
    let fold = Fold { bar, hint: l.pick(SEARCH_HINT), span: cards_top - bar.y };
    out.push(Hit { rect: bar, action: Action::Open(Screen::Search) });

    // Under the phone's name the card holds only what asks for something to
    // be done: a battery too low to install on, an update waiting to merge,
    // and a restart into the slot the system does not run from now.
    let low = match i.battery.as_ref().and_then(skyward_install::battery_refusal) {
        Some(Refusal::Battery { level, charging }) => Some(fmt::battery_low(l, level, charging)),
        _ => None,
    };
    let next = match (i.slot, &cx.state.boot_slots) {
        (Some(now), Some(Ok(slots))) if slots.active != now => Some(fmt::next_boot(l, slots.active, now)),
        _ => None,
    };
    let mut rows = Vec::new();
    let mut opens = Vec::new();
    if let Some((title, note)) = &low {
        rows.push(Row::new(title).icon(RED, Icon::Battery).subtitle(note).trail(Trail::None));
        opens.push(None);
    }
    if let Some(note) = fmt::snapshot_note(l, &i.snapshot) {
        rows.push(Row::new(fmt::snapshot(l, &i.snapshot)).icon(ORANGE, Icon::Layers).subtitle(note).trail(Trail::Arrow));
        opens.push(Some(Action::Open(Screen::Update)));
    }
    if let Some((title, note)) = &next {
        rows.push(Row::new(title).icon(ORANGE, Icon::Power).subtitle(note).trail(Trail::Arrow));
        opens.push(Some(Action::Open(Screen::Slot)));
    }
    if cx.state.system_notice.is_some() {
        let why = l.pick(texts::home::TAP_TO_SEE_WHY);
        rows.push(Row::new(l.pick(texts::systems::SECOND_FAILED)).icon(RED, Icon::Systems).subtitle(why).trail(Trail::Arrow));
        opens.push(Some(Action::Open(Screen::Systems)));
    }
    // The account's row is the phone's own: the owner's picture and the name
    // the phone was given, with the model and the system under it, and the page
    // of details a tap away. It waits for the partition it is kept on to be
    // open; before that neither the picture nor the name is there.
    // The model and the codename only: the version is on the page the row opens,
    // and inside the row it runs past the end.
    let detail = format!("{} \u{b7} {}", i.model, i.codename);
    // Under the model, the four readings the popsicle build puts there: the
    // slot, the data partition, the room left and the charge, with what the
    // charger is giving drawn after the percentage (2026-11-25).
    let slot = i.slot.map(Slot::letter).unwrap_or("\u{2014}");
    let partition = fmt::data(l, &s.data);
    let free = i.storage.free.map(fmt::gb).unwrap_or_else(|| "\u{2014}".to_owned());
    // While a charger is in, the figure and what it delivers share the top
    // line; the caption under them says what that is. Unplugged, the top line
    // is the level alone (2026-11-25).
    let charge = match &i.battery {
        Some(b) => format!("{}%", b.level),
        None => "\u{2014}".to_owned(),
    };
    // While a charger is in, what it delivers rides on the same line as the
    // level, after a full stop of its own (2026-11-25).
    let charge_watts = match (&i.battery, s.charge.now) {
        (Some(b), Some(tenths)) if b.charging() => Some(fmt::watts(tenths).replace('W', "w")),
        _ => None,
    };
    // The line under it says what that figure is rather than repeating it.
    let charge_label: Option<&str> = match charge_watts {
        Some(_) => Some(l.pick(["功率", "功率", "Power"])),
        None => None,
    };
    let charge_what = match &i.battery {
        Some(b) if b.charging() => l.pick(["充电中", "充電中", "Charging"]),
        _ => l.pick(["电量", "電量", "Battery"]),
    };

    let stats: [(&str, Option<&str>, &str, Option<&str>); 4] = [
        (slot, None, l.pick(["当前槽位", "當前槽位", "Current slot"]), None),
        (partition, None, l.pick(["数据分区", "資料分區", "Data"]), None),
        (&free, None, l.pick(["可用空间", "可用空間", "Free"]), None),
        (&charge, charge_watts.as_deref(), charge_what, charge_label),
    ];
    match &s.device_name {
        Some(name) => {
            let account = widgets::account_card_with_stats(&mut c, t, f, y, name, Some(&detail), s.avatar.as_ref(), &stats);
            out.push(Hit { rect: account.rows[0], action: Action::Open(Screen::About) });
            y = account.bottom + t.m.card_gap;
        }
        // Nothing of the phone itself read yet: the model alone stands in, and
        // a tap still goes to the details.
        None => {
            let card = widgets::account_card_with_stats(&mut c, t, f, y, &i.model, Some(&i.codename), None, &stats);
            out.push(Hit { rect: card.rows[0], action: Action::Open(Screen::About) });
            y = card.bottom + t.m.card_gap;
        }
    }
    // What asks for something to be done stands on its own under it: a battery
    // too low to install on, an update waiting to merge, a restart into the
    // slot the system does not run from now.
    if !rows.is_empty() {
        let list = widgets::list(&mut c, t, f, y, &rows);
        out.extend(list.rows.iter().zip(&opens).filter_map(|(&rect, open)| open.map(|action| Hit { rect, action })));
        y = list.bottom + t.m.card_gap;
    }
    // 首页：置顶的散条 → 各分类 → 「其他」。每栏是一张卡，第一行是栏名，
    // 点一下就折叠/展开（折叠状态记在设置里）。没有分类时置顶与「其他」
    // 合成一张卡（2026-11-27）。
    let systems_text = systems_value(cx);
    let wlan_text = cx.state.wlan.as_ref().map(|w| fmt::wlan_value(l, w));
    let skyward_text = skyward_value(cx);
    let offered = |entries: &[Entry]| -> Vec<Entry> { entries.iter().copied().filter(|entry| home_offers(cx, *entry)).collect() };
    // 第一次用（设置里没有任何分栏）：显示层给一栏内置的「刷机与备份」。
    let fresh = cx.state.home_sections.is_empty()
        && cx.state.home_top.is_empty()
        && crate::app::rest_is_default(&cx.state.home_rest);
    // 只有第一次用（显示层补一栏内置分类）才需要自己造一份；其余情况直接借用
    // `State` 里的，首页是常驻页，滑一下不该克隆整张布局（2026-11-27）。
    let (sections, top_all, rest_all): (std::borrow::Cow<[crate::app::Section]>, std::borrow::Cow<[Entry]>, std::borrow::Cow<[Entry]>) = if fresh {
        let mut rest = cx.state.home_rest.clone();
        rest.retain(|entry| !matches!(entry, Entry::Install | Entry::Backup | Entry::Restore));
        (std::borrow::Cow::Owned(crate::app::default_sections(l)), std::borrow::Cow::Borrowed(&[]), std::borrow::Cow::Owned(rest))
    } else {
        (
            std::borrow::Cow::Borrowed(&cx.state.home_sections[..]),
            std::borrow::Cow::Borrowed(&cx.state.home_top[..]),
            std::borrow::Cow::Borrowed(&cx.state.home_rest[..]),
        )
    };
    if sections.is_empty() {
        let loose = offered(&top_all.iter().chain(rest_all.iter()).copied().collect::<Vec<_>>());
        if !loose.is_empty() {
            let rows: Vec<Row> = loose.iter().map(|entry| home_row(*entry, l, &cx.state.entry_names, wlan_text, systems_text.as_deref(), skyward_text, cx.state.file_transfer)).collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            out.extend(hits(&list.rows, &loose.iter().map(|entry| entry.action()).collect::<Vec<_>>()));
            y = list.bottom;
        }
    } else {
        let top = offered(top_all.as_ref());
        if !top.is_empty() {
            let rows: Vec<Row> = top.iter().map(|entry| home_row(*entry, l, &cx.state.entry_names, wlan_text, systems_text.as_deref(), skyward_text, cx.state.file_transfer)).collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            out.extend(hits(&list.rows, &top.iter().map(|entry| entry.action()).collect::<Vec<_>>()));
            y = list.bottom + t.m.card_gap;
        }
        // 「其他」也是一栏：位置由 home_rest_at 说，删掉的分类的条目都落到这里，
        // 所以它能上下挪，却不能删（2026-11-27）。
        let rest_at = cx.state.home_rest_at.unwrap_or(sections.len()).min(sections.len());
        for (i, section) in sections.iter().enumerate() {
            if i == rest_at {
                let after = rest_card(&mut c, t, f, l, y, &mut out, cx.state, wlan_text, systems_text.as_deref(), skyward_text, offered(rest_all.as_ref()));
                if after > y {
                    y = after + t.m.card_gap;
                }
            }
            let entries = offered(&section.entries);
            let name: String = if section.name.chars().count() > 14 {
                format!("{}…", section.name.chars().take(14).collect::<String>())
            } else {
                section.name.clone()
            };
            let title = if entries.is_empty() {
                format!("{} · {}", name, l.pick(["空", "空", "empty"]))
            } else {
                format!("{} · {}", name, entries.len())
            };
            y = section_card(&mut c, t, f, l, y, &mut out, &cx.state.entry_names, wlan_text, systems_text.as_deref(), skyward_text, cx.state.file_transfer, title, section.icon, entries, section.collapsed, Action::HomeFoldSection(i)) + t.m.card_gap;
        }
        if rest_at >= sections.len() {
            let after = rest_card(&mut c, t, f, l, y, &mut out, cx.state, wlan_text, systems_text.as_deref(), skyward_text, offered(rest_all.as_ref()));
            if after > y {
                y = after;
            }
        }
    }
    // However little it holds, the page scrolls far enough to fold its bar
    // away.
    let height = fitted(t, y).max(t.m.h + fold.span.ceil() as u32);
    // 顶上那个大字：主人起过名字就用它（2026-11-27）。
    let title = if cx.state.home_title.is_empty() {
        std::borrow::Cow::Borrowed(HOME_TITLE)
    } else {
        std::borrow::Cow::Owned(cx.state.home_title.clone())
    };
    Frame { back: false, fold: Some(fold), ..Frame::new(t, c.crop_height(height), out, title, cards_top) }
}

/// 首页上的一栏：第一行是栏名（点它折叠），后面是条目（展开时）。
/// 栏目头统一是「名称 · 数量」+ 右侧一枚折角（展开朝上、折叠朝下，
/// 灰色、与别处的箭头同尺寸），整行可点。2026-11-27。
#[allow(clippy::too_many_arguments)]
fn section_card<'a>(
    c: &mut Canvas,
    t: &Theme,
    f: &Fonts,
    l: Lang,
    y: f32,
    out: &mut Vec<Hit>,
    names: &'a std::collections::BTreeMap<String, String>,
    wlan: Option<&'a str>,
    systems: Option<&'a str>,
    skyward: Option<&'a str>,
    file_transfer: bool,
    title: String,
    icon: Icon,
    entries: Vec<Entry>,
    collapsed: bool,
    fold: Action,
) -> f32 {
    let mut rows: Vec<Row> = vec![Row::new(&title).icon(BLUE, icon)];
    if !collapsed {
        rows.extend(entries.iter().map(|entry| home_row(*entry, l, names, wlan, systems, skyward, file_transfer)));
    }
    let list = widgets::list(c, t, f, y, &rows);
    out.push(Hit { rect: list.rows[0], action: fold });
    let m = &t.m;
    let d = m.dp(30.0);
    let bx = list.rows[0].x + list.rows[0].w - m.inner - d / 2.0;
    let by = list.rows[0].y + list.rows[0].h / 2.0;
    widgets::fold_button(c, t, bx, by, d, collapsed);
    if !collapsed {
        out.extend(hits(&list.rows[1..], &entries.iter().map(|entry| entry.action()).collect::<Vec<_>>()));
    }
    list.bottom
}

/// 「其他」这一栏：删掉的分类的条目都落到这里，所以它在排序页能上下挪，
/// 却不能删（2026-11-27）。空着就什么都不画。
#[allow(clippy::too_many_arguments)]
fn rest_card<'a>(
    c: &mut Canvas,
    t: &Theme,
    f: &Fonts,
    l: Lang,
    y: f32,
    out: &mut Vec<Hit>,
    state: &State,
    wlan: Option<&'a str>,
    systems: Option<&'a str>,
    skyward: Option<&'a str>,
    entries: Vec<Entry>,
) -> f32 {
    if entries.is_empty() {
        return y;
    }
    let name = if state.home_rest_name.is_empty() {
        l.pick(["其他", "其他", "Others"]).to_string()
    } else {
        state.home_rest_name.clone()
    };
    let title = format!("{} · {}", name, entries.len());
    section_card(c, t, f, l, y, out, &state.entry_names, wlan, systems, skyward, state.file_transfer, title, Icon::Folder, entries, state.home_rest_collapsed, Action::HomeFoldRest)
}

/// 这一条在现在的手机上摆不摆（没有 WLAN、没有双系统、没有更新时不摆）。
fn home_offers(cx: &Ctx, entry: Entry) -> bool {
    match entry {
        Entry::Wlan => cx.state.wlan.is_some(),
        Entry::Systems => cx.state.offers(Screen::Systems),
        Entry::SkywardUpdate => cx.state.offers(Screen::SkywardUpdate),
        _ => true,
    }
}

/// 首页每一条的图标颜色。
fn home_color(entry: Entry) -> Rgba {
    match entry {
        Entry::Wlan | Entry::Files | Entry::Backup | Entry::SkywardUpdate => BLUE,
        Entry::Modules | Entry::DataApps | Entry::Restore | Entry::Systems => PURPLE,
        Entry::Install => GREEN,
        Entry::Settings | Entry::HomeOrder => SLATE,
        Entry::Reboot => ORANGE,
        Entry::Wipe => RED,
        // 从高级页搬来的那些，颜色与高级页一致。
        Entry::Decrypt | Entry::Serve | Entry::Update => BLUE,
        Entry::Terminal | Entry::Partitions => SLATE,
        Entry::Fetch | Entry::Charge => GREEN,
        Entry::Avb => RED,
        Entry::Root | Entry::Usb | Entry::Relabel => PURPLE,
        Entry::Logs => ORANGE,
        Entry::FileTransfer => BLUE,
    }
}

/// 一行首页条目：名字、图标；WLAN／第二系统／版本更新带值。
fn home_row<'a>(
    entry: Entry,
    l: Lang,
    names: &'a std::collections::BTreeMap<String, String>,
    wlan: Option<&'a str>,
    systems: Option<&'a str>,
    skyward: Option<&'a str>,
    file_transfer: bool,
) -> Row<'a> {
    let mut row = Row::new(entry.shown(l, names)).icon(home_color(entry), entry.icon());
    // 文件传输点进去再开关（2026-11-27）：这一行只露出它现在开着还是关着，
    // 和别的条目一样是「点进去」。
    if entry == Entry::FileTransfer {
        let text = if file_transfer {
            l.pick(["开", "開", "On"])
        } else {
            l.pick(["关", "關", "Off"])
        };
        return row.trail(Trail::Value(text));
    }
    let value = match entry {
        Entry::Wlan => wlan,
        Entry::Systems => systems,
        Entry::SkywardUpdate => skyward,
        _ => None,
    };
    if let Some(value) = value {
        row = row.trail(Trail::Value(value));
    }
    row
}

/// 首页分栏：一栏一栏列出来，点一条拎起来，再点一条放到它前面，或点栏标题
/// 放到那栏末尾；栏标题没拎东西时点开是改名/删除；最下面是新建栏与恢复默认。
/// 2026-11-27。
pub fn home_order(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let mut y = m.content_top;
    let hint = l.pick([
        "点一条拎起来，再点一条放到它前面；点栏标题放到那栏末尾。",
        "點一條拎起來，再點一條放到它前面；點欄標題放到那欄末尾。",
        "Tap an entry to pick it up, then tap where it goes; tap a heading to put it at that section's end.",
    ]);
    let pick = s.home_pick;
    // 第一次用的时候，内置的「刷机与备份」也要在排序页里看得见、动得了。
    let fresh = s.home_sections.is_empty()
        && s.home_top.is_empty()
        && crate::app::rest_is_default(&s.home_rest);
    let (sections, top_list, rest_list) = if fresh {
        let mut rest = s.home_rest.clone();
        rest.retain(|entry| !matches!(entry, Entry::Install | Entry::Backup | Entry::Restore));
        (crate::app::default_sections(l), Vec::new(), rest)
    } else {
        (s.home_sections.clone(), s.home_top.clone(), s.home_rest.clone())
    };

    // 单独放在首页的条目（最上面那一张卡）。点小标题把拎着的那条放上来。
    {
        let before = y;
        y = widgets::section(&mut c, t, f, y, l.pick(["单独放在首页", "單獨放在首頁", "Loose on Home"]));
        out.push(Hit {
            rect: Rect::new(m.margin, before, m.w as f32 - m.margin * 2.0, m.section_block),
            action: Action::HomeToTop,
        });
        if top_list.is_empty() {
            y = widgets::footnote(&mut c, t, f, y, l.pick(["拎一条起来，点这里放到最上面", "拎一條起來，點這裡放到最上面", "Pick an entry, then tap here to put it on top"]));
        } else {
            let rows: Vec<Row> = s
                .home_top
                .iter()
                .map(|entry| {
                    let mut row = Row::new(entry.shown(l, &s.entry_names)).icon(home_color(*entry), entry.icon());
                    if pick == Some(*entry) {
                        row = row.subtitle(hint);
                    }
                    row
                })
                .collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            let actions: Vec<Action> = s
                .home_top
                .iter()
                .map(|entry| match pick {
                    Some(picked) if picked == *entry => Action::HomePick(*entry),
                    Some(_) => Action::HomePlace(*entry),
                    None => Action::HomePick(*entry),
                })
                .collect();
            out.extend(hits(&list.rows, &actions));
            pencil_buttons(&mut c, t, &mut out, &list.rows, &s.home_top);
            y = list.bottom;
        }
    }

    // 每一栏：小标题（点它就是「放进这一栏」或进它的二级页）+ 一张条目卡。
    // 分类画成标题而不是一张卡：卡挨着卡会粘在一起，标题天然分开（2026-11-27）。
    let rest_at = s.home_rest_at.unwrap_or(sections.len()).min(sections.len());
    for (i, section) in sections.iter().enumerate() {
        if i == rest_at {
            y = order_rest_block(&mut c, t, f, l, y, &mut out, s, pick, hint) + m.card_gap;
        }
        let before = y;
        // 标题带上条目数；右边留一个「×」，不用进二级页也能删。
        let count = section.entries.len();
        let label = if count == 0 {
            format!("{} · {}", section.name, l.pick(["空", "空", "empty"]))
        } else {
            format!("{} · {count}", section.name)
        };
        y = widgets::section(&mut c, t, f, y, &label);
        out.push(Hit {
            rect: Rect::new(m.margin, before, m.w as f32 - m.margin * 2.0 - m.dp(56.0), m.section_block),
            action: match pick {
                Some(_) => Action::HomeToSection(Some(i)),
                None => Action::HomeOpenSection(i),
            },
        });
        // 一栏四个动作排成一排，同一尺寸的圆钮：改名、上移、下移、删除。
        let d = m.dp(30.0);
        let step = m.dp(38.0);
        let cy = before + m.section_block / 2.0;
        let right = m.w as f32 - m.margin - d / 2.0;
        let acts = [
            (Action::HomeEditSection(i), Icon::Pencil),
            (Action::HomeShiftSection { at: i, up: true }, Icon::Sort),
            (Action::HomeShiftSection { at: i, up: false }, Icon::Sort),
            (Action::HomeDeleteSection(i), Icon::Wipe),
        ];
        for (k, (action, icon)) in acts.into_iter().enumerate() {
            let cx = right - (3 - k) as f32 * step;
            let danger = matches!(action, Action::HomeDeleteSection(_));
            widgets::icon_button(&mut c, t, cx, cy, d, if danger { t.pal.danger } else { t.pal.subtitle }, icon);
            out.push(Hit { rect: Rect::new(cx - d / 2.0 - m.dp(6.0), cy - d / 2.0 - m.dp(6.0), d + m.dp(12.0), d + m.dp(12.0)), action });
        }
        if section.entries.is_empty() {
            y = widgets::footnote(&mut c, t, f, y, l.pick(["这一栏还是空的", "這一欄還是空的", "Empty for now"]));
            continue;
        }
        let rows: Vec<Row> = section
            .entries
            .iter()
            .map(|entry| {
                let mut row = Row::new(entry.shown(l, &s.entry_names)).icon(home_color(*entry), entry.icon());
                if pick == Some(*entry) {
                    row = row.subtitle(hint);
                }
                row
            })
            .collect();
        let list = widgets::list(&mut c, t, f, y, &rows);
        let actions: Vec<Action> = section
            .entries
            .iter()
            .map(|entry| match pick {
                Some(picked) if picked == *entry => Action::HomePick(*entry),
                Some(_) => Action::HomePlace(*entry),
                None => Action::HomePick(*entry),
            })
            .collect();
        out.extend(hits(&list.rows, &actions));
        pencil_buttons(&mut c, t, &mut out, &list.rows, &section.entries);
        y = list.bottom;
    }

    // 「其他」摆在 rest_at 说的位置；它不能删，只能上下挪（2026-11-27）。
    if rest_at >= sections.len() {
        y = order_rest_block(&mut c, t, f, l, y, &mut out, s, pick, hint);
    }

    // 新建一栏、恢复默认。
    let more = widgets::list(
        &mut c,
        t,
        f,
        y + m.card_gap,
        &[
            Row::new(l.pick(["首页标题", "首頁標題", "Home title"])).icon(SLATE, Icon::Pencil).trail(Trail::Arrow),
            Row::new(l.pick(["布局备份", "佈局備份", "Layout backup"])).icon(PURPLE, Icon::Backup).trail(Trail::Arrow),
            Row::new(l.pick(["新建分类", "新增分類", "New section"])).icon(BLUE, Icon::Layers).trail(Trail::Arrow),
            Row::new(l.pick(["恢复默认顺序", "恢復預設順序", "Restore the default order"])).tint(t.pal.danger),
        ],
    );
    out.extend(hits(&more.rows, &[
        Action::EditHomeTitle,
        Action::Open(Screen::Layout),
        Action::HomeNewSection,
        Action::HomeReset,
    ]));
    let note = widgets::footnote(
        &mut c,
        t,
        f,
        more.bottom,
        l.pick([
            "点栏标题可以改名或删除；删掉的栏里的条目回到「其他」。不支持的入口在首页不显示，但顺序里仍占着位置。",
            "點欄標題可以改名或刪除；刪掉的欄裡的條目回到「其他」。不支援的入口在首頁不顯示，但順序裡仍佔著位置。",
            "Tap a heading to rename or remove it; its entries go back to Others. Entries the phone has none of are left out, their places kept.",
        ]),
    );
    Frame::new(t, fit(c, t, note), out, l.pick(["首页分类", "首頁分類", "Home sections"]), m.content_top)
}

/// 排序页每一条右边一枚铅笔：点它进改名页；整行仍然是拎起来 / 放下
/// （2026-11-27）。
fn pencil_buttons(c: &mut Canvas, t: &Theme, out: &mut Vec<Hit>, rows: &[Rect], entries: &[Entry]) {
    let m = &t.m;
    let d = m.dp(30.0);
    for (k, row) in rows.iter().enumerate() {
        let cx = row.x + row.w - m.inner - d / 2.0;
        let cy = row.y + row.h / 2.0;
        widgets::icon_button(c, t, cx, cy, d, t.pal.subtitle, Icon::Pencil);
        out.push(Hit {
            rect: Rect::new(cx - d / 2.0 - m.dp(6.0), cy - d / 2.0 - m.dp(6.0), d + m.dp(12.0), d + m.dp(12.0)),
            action: Action::EntryRename(entries[k]),
        });
    }
}

/// 排序页上的「其他」：标题（点它把拎着的条目放进来）、改名与上下挪，以及
/// 它里面的条目。它不能删——删掉的分类的条目都落到这里（2026-11-27）。
#[allow(clippy::too_many_arguments)]
fn order_rest_block(
    c: &mut Canvas,
    t: &Theme,
    f: &Fonts,
    l: Lang,
    mut y: f32,
    out: &mut Vec<Hit>,
    s: &State,
    pick: Option<Entry>,
    hint: &str,
) -> f32 {
    let m = &t.m;
    let before = y;
    let rest_name = if s.home_rest_name.is_empty() {
        l.pick(["其他", "其他", "Others"]).to_string()
    } else {
        s.home_rest_name.clone()
    };
    y = widgets::section(c, t, f, y, &format!("{rest_name} · {}", s.home_rest.len()));
    let d = m.dp(30.0);
    let step = m.dp(38.0);
    out.push(Hit {
        rect: Rect::new(m.margin, before, m.w as f32 - m.margin * 2.0 - step * 3.0, m.section_block),
        action: Action::HomeToSection(None),
    });
    let cy = before + m.section_block / 2.0;
    let right = m.w as f32 - m.margin - d / 2.0;
    // 三个钮：改名、上移、下移。没有删除——它是兜底的那一栏。
    let acts = [
        (Action::HomeRenameRest, Icon::Pencil),
        (Action::HomeShiftRest { up: true }, Icon::Sort),
        (Action::HomeShiftRest { up: false }, Icon::Sort),
    ];
    for (k, (action, icon)) in acts.into_iter().enumerate() {
        let bx = right - (2 - k) as f32 * step;
        widgets::icon_button(c, t, bx, cy, d, t.pal.subtitle, icon);
        out.push(Hit {
            rect: Rect::new(bx - d / 2.0 - m.dp(6.0), cy - d / 2.0 - m.dp(6.0), d + m.dp(12.0), d + m.dp(12.0)),
            action,
        });
    }
    if s.home_rest.is_empty() {
        return widgets::footnote(c, t, f, y, l.pick(["没有剩下的条目", "沒有剩下的條目", "Nothing left over"]));
    }
    let rows: Vec<Row> = s
        .home_rest
        .iter()
        .map(|entry| {
            let mut row = Row::new(entry.shown(l, &s.entry_names)).icon(home_color(*entry), entry.icon());
            if pick == Some(*entry) {
                row = row.subtitle(hint);
            }
            row
        })
        .collect();
    let list = widgets::list(c, t, f, y, &rows);
    let actions: Vec<Action> = s
        .home_rest
        .iter()
        .map(|entry| match pick {
            Some(picked) if picked == *entry => Action::HomePick(*entry),
            Some(_) => Action::HomePlace(*entry),
            None => Action::HomePick(*entry),
        })
        .collect();
    out.extend(hits(&list.rows, &actions));
    pencil_buttons(c, t, out, &list.rows, &s.home_rest);
    list.bottom
}

/// 某一栏自己的页面：这一栏的条目在这里排序；「其他」里的条目点一下收进来；
/// 最下面能改名、删除（删了条目回「其他」）。2026-11-27。
pub fn home_section_view(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let mut y = m.content_top;
    let Some(i) = s.section_view.filter(|i| *i < s.home_sections.len()) else {
        return Frame::new(t, fit(c, t, y), out, l.pick(["首页分类", "首頁分類", "Home sections"]), m.content_top);
    };
    let section = &s.home_sections[i];
    // 标题栏是固定字（Frame 的标题只收 &'static str），栏名画成页面里的小标题。
    y = widgets::section_first(&mut c, t, f, y, &section.name);
    let hint = l.pick([
        "点一条拎起来，再点一条放到它前面。",
        "點一條拎起來，再點一條放到它前面。",
        "Tap an entry to pick it up, then tap where it goes.",
    ]);
    let pick = s.home_pick;
    if section.entries.is_empty() {
        y = widgets::footnote(&mut c, t, f, y, l.pick(["这一栏还是空的：下面「其他」里的条目点一下就进来。", "這一欄還是空的：下面「其他」裡的條目點一下就進來。", "This section is empty: tap entries under Others to bring them in."]));
    } else {
        let rows: Vec<Row> = section
            .entries
            .iter()
            .map(|entry| {
                let mut row = Row::new(entry.shown(l, &s.entry_names)).icon(home_color(*entry), entry.icon());
                if pick == Some(*entry) {
                    row = row.subtitle(hint);
                }
                row
            })
            .collect();
        let list = widgets::list(&mut c, t, f, y, &rows);
        let actions: Vec<Action> = section
            .entries
            .iter()
            .map(|entry| match pick {
                Some(picked) if picked == *entry => Action::HomePick(*entry),
                Some(_) => Action::HomePlace(*entry),
                None => Action::HomePick(*entry),
            })
            .collect();
        out.extend(hits(&list.rows, &actions));
        y = list.bottom;
    }
    if !s.home_rest.is_empty() {
        y = widgets::section(&mut c, t, f, y + m.card_gap, l.pick(["其他里的条目", "其他裡的條目", "Entries in Others"]));
        let rows: Vec<Row> = s.home_rest.iter().map(|entry| Row::new(entry.shown(l, &s.entry_names)).icon(home_color(*entry), entry.icon()).trail(Trail::Arrow)).collect();
        let list = widgets::list(&mut c, t, f, y, &rows);
        out.extend(hits(&list.rows, &s.home_rest.iter().map(|entry| Action::HomeAdopt(*entry)).collect::<Vec<_>>()));
        y = list.bottom;
    }
    let more = widgets::list(
        &mut c,
        t,
        f,
        y + m.card_gap,
        &[
            Row::new(l.pick(["改名", "改名", "Rename"])).trail(Trail::Arrow),
            Row::new(l.pick(["上移这一栏", "上移這一欄", "Move up"])).trail(Trail::Arrow),
            Row::new(l.pick(["下移这一栏", "下移這一欄", "Move down"])).trail(Trail::Arrow),
            Row::new(l.pick(["删除这一栏", "刪除這一欄", "Remove this section"])).tint(t.pal.danger),
        ],
    );
    out.extend(hits(&more.rows, &[
        Action::HomeEditSection(i),
        Action::HomeShiftSection { at: i, up: true },
        Action::HomeShiftSection { at: i, up: false },
        Action::HomeDeleteSection(i),
    ]));
    let note = widgets::footnote(&mut c, t, f, more.bottom, l.pick([
        "删掉的栏里的条目回到「其他」。",
        "刪掉的欄裡的條目回到「其他」。",
        "Entries of a section removed go back to Others.",
    ]));
    Frame::new(t, fit(c, t, note), out, l.pick(["首页分类", "首頁分類", "Home sections"]), m.content_top)
}

/// 挑栏目图标：全部图标排成格子，点一个就用它。
pub fn home_icon(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let columns = 5usize;
    let gap = m.dp(10.0);
    let size = (m.w as f32 - m.margin * 2.0 - gap * (columns as f32 - 1.0)) / columns as f32;
    let mut y = top;
    for (at, icon) in crate::icons::ALL.iter().enumerate() {
        let column = at % columns;
        let row = at / columns;
        let x = m.margin + column as f32 * (size + gap);
        let at_y = y + row as f32 * (size + gap);
        if *icon == s.section_icon {
            // 选中的那枚加一圈浅底，一眼看得出来。
            c.fill_round_rect(x - 4.0, at_y - 4.0, size + 8.0, size + 8.0, m.card_radius, t.pal.search);
        }
        icons::tile(&mut c, t, x + size / 2.0 - m.dp(24.0), at_y + size / 2.0 - m.dp(24.0), if *icon == s.section_icon { t.pal.accent } else { BLUE }, *icon);
        out.push(Hit { rect: Rect::new(x, at_y, size, size), action: Action::HomePickIcon(at) });
    }
    let rows = crate::icons::ALL.len().div_ceil(columns);
    y += rows as f32 * (size + gap);
    let note = widgets::footnote(&mut c, t, f, y, l.pick(["点一个图标就用它", "點一個圖示就用它", "Tap an icon to use it"]));
    Frame::new(t, fit(c, t, note), out, l.pick(["选个图标", "選個圖示", "Pick an icon"]), top)
}

/// 给一栏起名字（新建或改名），改名时还能把它删掉。
pub fn home_section(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let title = match s.section_edit {
        Some(_) => l.pick(["给这一栏改名", "給這一欄改名", "Rename the section"]),
        None => l.pick(["新建分类", "新增分類", "New section"]),
    };
    let y = widgets::section_first(&mut c, t, f, top, l.pick(["名字", "名字", "Name"]));
    let (below, caret) = widgets::text_field(&mut c, t, f, y, &s.section_name);
    out.push(Hit {
        rect: Rect::new(m.margin, y, m.w as f32 - m.margin * 2.0, m.dp(56.0)),
        action: Action::TypeInto(Field::Name),
    });
    let mut bottom = below;
    // 图标：点开是全部图标，选一个回来。
    let icon_row = widgets::list(
        &mut c,
        t,
        f,
        below + m.card_gap,
        &[Row::new(l.pick(["图标", "圖示", "Icon"])).icon(BLUE, s.section_icon).trail(Trail::Arrow)],
    );
    out.extend(hits(&icon_row.rows, &[Action::Open(Screen::HomeIcon)]));
    bottom = icon_row.bottom;
    if let Some(i) = s.section_edit {
        let gone = widgets::list(
            &mut c,
            t,
            f,
            bottom + m.card_gap,
            &[Row::new(l.pick(["删除这一栏", "刪除這一欄", "Remove this section"])).tint(t.pal.danger)],
        );
        out.extend(hits(&gone.rows, &[Action::HomeDeleteSection(i)]));
        bottom = gone.bottom;
    }
    keyboard_hits(&mut c, t, f, s, l.pick(["完成", "完成", "Done"]), &mut out);
    Frame { caret: Some(caret), ..Frame::new(t, c, out, title, top) }
}

/// 给首页的某一条改名：名字只换首页上显示的字，留空就是回它自己的名字
/// （2026-11-27）。
pub fn entry_name(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let title = l.pick(["给这一条改名", "給這一條改名", "Rename the entry"]);
    // 改的是哪一条：把它的名字写成小标题，免得改错。
    let which = match s.entry_edit {
        Some(entry) => entry.shown(l, &s.entry_names),
        None => l.pick(["首页条目", "首頁條目", "A home entry"]),
    };
    let y = widgets::section_first(&mut c, t, f, top, which);
    let (below, caret) = widgets::text_field(&mut c, t, f, y, &s.entry_name);
    out.push(Hit {
        rect: Rect::new(m.margin, y, m.w as f32 - m.margin * 2.0, m.dp(56.0)),
        action: Action::TypeInto(Field::Name),
    });
    let mut bottom = below;
    if s.entry_edit.is_some_and(|entry| s.entry_names.contains_key(entry.id())) {
        let reset = widgets::list(
            &mut c,
            t,
            f,
            bottom + m.card_gap,
            &[Row::new(l.pick(["改回默认名字", "改回預設名字", "Back to its own name"])).trail(Trail::Arrow)],
        );
        out.extend(hits(&reset.rows, &[Action::EntryNameReset]));
        bottom = reset.bottom;
    }
    widgets::footnote(
        &mut c,
        t,
        f,
        bottom + m.dp(8.0),
        l.pick([
            "名字只换首页上显示的字，功能不变；留着空的按原来的名字显示。",
            "名字只換首頁上顯示的字，功能不變；留著空的按原來的名字顯示。",
            "Only the words on the home page change, not what the entry does; left empty it wears its own name again.",
        ]),
    );
    keyboard_hits(&mut c, t, f, s, l.pick(["完成", "完成", "Done"]), &mut out);
    Frame { caret: Some(caret), ..Frame::new(t, c, out, title, top) }
}

/// 改首页顶上那个大字：留空就回到 Skyward（2026-11-27）。
pub fn home_title(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let title = l.pick(["首页标题", "首頁標題", "Home title"]);
    let y = widgets::section_first(&mut c, t, f, top, l.pick(["名字", "名字", "Name"]));
    let (below, caret) = widgets::text_field(&mut c, t, f, y, &s.home_title_edit);
    out.push(Hit {
        rect: Rect::new(m.margin, y, m.w as f32 - m.margin * 2.0, m.dp(56.0)),
        action: Action::TypeInto(Field::Name),
    });
    widgets::footnote(
        &mut c,
        t,
        f,
        below + m.dp(8.0),
        l.pick([
            "这是首页最上面那个大字；留着空的就还是 Skyward。字太长会被自动缩小到一行里。",
            "這是首頁最上面那個大字；留著空的就還是 Skyward。字太長會被自動縮小到一行裡。",
            "This is the large word at the top of the home page; left empty it stays Skyward. A long one is set smaller to fit one line.",
        ]),
    );
    keyboard_hits(&mut c, t, f, s, l.pick(["完成", "完成", "Done"]), &mut out);
    Frame { caret: Some(caret), ..Frame::new(t, c, out, title, top) }
}

/// 布局备份：首页的分组、顺序、名字、标题，以及控制中心的磁贴，写成
/// 一个文件，也能从文件读回来（2026-11-27）。
pub fn layout(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let mut y = m.content_top;

    // 这一份现在是什么样：几栏、几条、几枚磁贴。
    let held: usize = s.home_top.len() + s.home_rest.len() + s.home_sections.iter().map(|section| section.entries.len()).sum::<usize>();
    let about = format!(
        "{} {} · {} {} · {} {}",
        l.pick(["分类", "分類", "Sections"]),
        s.home_sections.len(),
        l.pick(["条目", "條目", "Entries"]),
        held,
        l.pick(["磁贴", "磁貼", "Tiles"]),
        s.tiles.len(),
    );
    y = widgets::section_first(&mut c, t, f, y, l.pick(["这一份", "這一份", "This one"]));
    let card = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(&about).icon(BLUE, Icon::Layers),
            Row::new(l.pick(["导出到存储", "匯出到儲存", "Write it to storage"])).icon(GREEN, Icon::Backup).trail(Trail::Arrow),
            Row::new(l.pick(["导出到别处…", "匯出到別處…", "Write it elsewhere…"])).icon(GREEN, Icon::Folder).trail(Trail::Arrow),
        ],
    );
    // 第一行只说这一份有多少，点了不动。
    out.extend(hits(&card.rows[1..], &[Action::ExportLayout, Action::LayoutBrowseExport]));
    y = card.bottom;

    // 存储里的那些：点一条就按它改。
    y = widgets::section(&mut c, t, f, y + m.card_gap, l.pick(["存储里的布局", "儲存裡的佈局", "Layouts in storage"]));
    match &s.layouts {
        None => y = widgets::footnote(&mut c, t, f, y, l.pick(["正在读……", "正在讀……", "Reading…"])),
        Some(Err(why)) => y = widgets::footnote(&mut c, t, f, y, why),
        Some(Ok(files)) if files.is_empty() => {
            y = widgets::footnote(
                &mut c,
                t,
                f,
                y,
                l.pick([
                    "还没有导出过布局。导出一次，文件就落在存储的 Skyward 文件夹里。",
                    "還沒有匯出過佈局。匯出一次，檔案就落在儲存的 Skyward 資料夾裡。",
                    "Nothing written out yet: export once and the file lands in the Skyward folder in storage.",
                ]),
            )
        }
        Some(Ok(files)) => {
            let rows: Vec<Row> = files
                .iter()
                .map(|file| Row::new(&file.name).icon(BLUE, Icon::Backup).trail(Trail::Arrow))
                .collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            out.extend(hits(&list.rows, &(0..files.len()).map(Action::ImportLayout).collect::<Vec<_>>()));
            y = list.bottom;
        }
    }
    // 不在默认位置的文件：自己挑。
    let elsewhere = widgets::list(
        &mut c,
        t,
        f,
        y + m.card_gap,
        &[Row::new(l.pick(["从别处导入…", "從別處匯入…", "Take one from elsewhere…"])).icon(PURPLE, Icon::Folder).trail(Trail::Arrow)],
    );
    out.extend(hits(&elsewhere.rows, &[Action::LayoutBrowseImport]));
    y = elsewhere.bottom;

    let note = widgets::footnote(&mut c, t, f, y + m.dp(10.0), l.pick([
        "导入会按文件里那一份改掉首页的分组、顺序、名字、标题，以及控制中心的磁贴；文件本身不动。文件名里的时间是导出它的那一刻。",
        "匯入會按檔案裡那一份改掉首頁的分組、順序、名字、標題，以及控制中心的磁貼；檔案本身不動。檔名裡的時間是匯出它的那一刻。",
        "Importing changes the home page's sections, order, names and title, and the control centre's tiles, to what the file holds; the file itself is left alone. The time in its name is when it was written out.",
    ]));
    Frame::new(t, fit(c, t, note), out, l.pick(["布局备份", "佈局備份", "Layout backup"]), m.content_top)
}

/// 挑布局的位置：导出时挑一个目录，导入时挑一个 `.json` 文件（2026-11-27）。
pub fn layout_browse(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let mut y = m.content_top;
    let exporting = s.layout_exporting;

    // 现在在哪一层。
    y = widgets::section_first(&mut c, t, f, y, s.layout_dir.path.as_str());
    let up = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[Row::new(l.pick(["上一层", "上一層", "Up one level"])).icon(SLATE, Icon::Layers).trail(Trail::Arrow)],
    );
    out.extend(hits(&up.rows, &[Action::LayoutUp]));
    y = up.bottom;
    if exporting {
        let here = widgets::list(
            &mut c,
            t,
            f,
            y + m.card_gap,
            &[Row::new(l.pick(["导出到这一层", "匯出到這一層", "Write the layout here"]))
                .icon(GREEN, Icon::Backup)
                .trail(Trail::Arrow)],
        );
        out.extend(hits(&here.rows, &[Action::LayoutExportHere]));
        y = here.bottom;
    }

    // 目录在前，`.json` 在后。
    match &s.layout_dir.entries {
        Err(why) => y = widgets::footnote(&mut c, t, f, y + m.card_gap, why),
        Ok(entries) => {
            let folders: Vec<&crate::app::FileRow> = entries
                .iter()
                .filter(|entry| matches!(entry.kind, crate::app::FileKind::Folder { .. }))
                .collect();
            let files: Vec<&crate::app::FileRow> = entries
                .iter()
                .filter(|entry| matches!(entry.kind, crate::app::FileKind::File) && entry.name.ends_with(".json"))
                .collect();
            y = widgets::section(&mut c, t, f, y + m.card_gap, l.pick(["目录", "目錄", "Folders"]));
            if folders.is_empty() {
                y = widgets::footnote(&mut c, t, f, y, l.pick(["这一层没有子目录", "這一層沒有子目錄", "No folders here"]));
            } else {
                let rows: Vec<Row> = folders.iter().map(|entry| Row::new(&entry.name).icon(BLUE, Icon::Folder).trail(Trail::Arrow)).collect();
                let list = widgets::list(&mut c, t, f, y, &rows);
                out.extend(hits(
                    &list.rows,
                    &(0..folders.len()).map(Action::LayoutEnter).collect::<Vec<_>>(),
                ));
                y = list.bottom;
            }
            if !exporting {
                y = widgets::section(&mut c, t, f, y + m.card_gap, l.pick(["布局文件", "佈局檔案", "Layout files"]));
                if files.is_empty() {
                    y = widgets::footnote(&mut c, t, f, y, l.pick(["这一层没有 .json 文件", "這一層沒有 .json 檔案", "No .json files here"]));
                } else {
                    let rows: Vec<Row> = files.iter().map(|entry| Row::new(&entry.name).icon(PURPLE, Icon::Backup).trail(Trail::Arrow)).collect();
                    let list = widgets::list(&mut c, t, f, y, &rows);
                    out.extend(hits(
                        &list.rows,
                        &(0..files.len()).map(Action::LayoutPick).collect::<Vec<_>>(),
                    ));
                    y = list.bottom;
                }
            }
        }
    }

    let note = l.pick(match exporting {
        true => [
            "挑一个目录，点「导出到这一层」就把布局写在那里；内部存储与 U 盘都能挑。",
            "挑一個目錄，點「匯出到這一層」就把佈局寫在那裡；內部儲存與 U 盤都能挑。",
            "Pick a folder, then write the layout there: internal storage and a USB drive both work.",
        ],
        false => [
            "点一个 .json 文件就按它改布局；目录照常进去看。",
            "點一個 .json 檔案就按它改佈局；目錄照常進去看。",
            "Tap a .json file to lay it over the pages as it stands; folders open as usual.",
        ],
    });
    let note = widgets::footnote(&mut c, t, f, y + m.dp(10.0), note);
    Frame::new(t, fit(c, t, note), out, l.pick(["挑一个位置", "挑一個位置", "Choose a place"]), m.content_top)
}

/// What the home page says beside its entry to the systems: the one that
/// starts, where a second system is installed, once they are read.
fn systems_value(cx: &Ctx) -> Option<String> {
    let l = cx.state.lang;
    match &cx.state.systems {
        Some(Ok(systems)) => Some(match (&systems.other, starting(systems)) {
            (None, _) => l.pick(texts::home::NO_SECOND_SYSTEM).into(),
            (Some(_), Who::Installed) => systems.installed.name.clone(),
            (Some(other), Who::Other) => other.name.clone(),
        }),
        _ => None,
    }
}

/// What the advanced page says beside its system update entry: a download
/// under way, a newer system found, or where an update the system installed
/// stands.
fn update_value(cx: &Ctx) -> &'static str {
    let (s, l) = (cx.state, cx.state.lang);
    if s.downloading() {
        return l.pick(texts::update::DOWNLOADING);
    }
    if s.newest().is_some_and(|r| cx.info.os_version.as_deref() != Some(r.version.as_str())) {
        return l.pick(texts::update::NEW_VERSION);
    }
    fmt::update_value(l, &s.update)
}

/// Root 自己的页：装了哪个、模块、以及安装后修补新槽的开关。
pub fn root(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let mut y = t.m.content_top;

    let (kernel_su, magisk) = (kernel_su_saying(&s), magisk_saying(&s));
    let rows = [
        Row::new(l.pick(texts::kernel_su::TITLE))
            .icon(BLUE, Icon::KernelSu)
            .trail(Trail::Value(&kernel_su)),
        Row::new(l.pick(texts::magisk::TITLE))
            .icon(PURPLE, Icon::Magisk)
            .trail(Trail::Value(&magisk)),
        Row::new(l.pick(texts::modules::TITLE)).icon(GREEN, Icon::Layers),
    ];
    let opens = [
        Action::Open(Screen::KernelSu),
        Action::Open(Screen::Magisk),
        Action::Open(Screen::Modules),
    ];
    let first = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&first.rows, &opens));
    y = first.bottom + t.m.card_gap;

    // 在线安装：这两张卡本身就是写入动作（要滑动确认），与上面两行「点进去看
    // 现状、选 KMI」是两回事，不会进同一个页面。
    y = widgets::section(&mut c, t, f, y, l.pick(texts::root::INSTALL_ONLINE));
    let (ks_label, mg_label) = (
        tr!(l, "滑动以在线写入 KernelSU", "滑動以線上寫入 KernelSU", "Slide to write KernelSU online"),
        tr!(l, "滑动以在线写入 Magisk", "滑動以線上寫入 Magisk", "Slide to write Magisk online"),
    );
    let (ks_progress, ks_text) = write_state(s.kernel_su_job.as_ref(), s.root_bytes, l, &ks_label);
    let (mg_progress, mg_text) = write_state(s.magisk_job.as_ref(), s.root_bytes, l, &mg_label);
    for (label, progress, which) in [(ks_text, ks_progress, Confirm::KernelSu), (mg_text, mg_progress, Confirm::Magisk)] {
        let track = widgets::confirm(&mut c, t, f, y, &label, false, progress);
        out.push(Hit { rect: track, action: Action::Confirm(which) });
        y = track.y + track.h + t.m.card_gap;
    }

    y = widgets::section(&mut c, t, f, y, l.pick(texts::root::TITLE));
    let auto_root = auto_root_saying(s, l);
    let rows = [
        Row::new(l.pick(texts::root::AUTO_PATCH)).trail(Trail::Value(&auto_root)),
    ];
    let picks = [Action::SetAutoRoot(s.auto_root.next())];
    let second = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&second.rows, &picks));
    y = second.bottom;
    let note = widgets::footnote(&mut c, t, f, y, l.pick(texts::root::AUTO_PATCH_NOTE));
    Frame { back: false, ..Frame::new(t, c.crop_height(fitted(t, note)), out, l.pick(texts::root::TITLE), t.m.content_top) }
}

/// KernelSU 的页：现状、要写的 KMI，以及写进当前槽的 boot 镜像。
pub fn kernel_su(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let mut y = t.m.content_top;

    let status = s.kernel_su.as_ref().and_then(|r| r.as_ref().ok());
    let why = match s.kernel_su.as_ref() {
        Some(Err(why)) => Some(why.clone()),
        _ => None,
    };
    let mut rows = Vec::new();
    let version = status
        .and_then(|st| st.version.clone())
        .unwrap_or_else(|| l.pick(texts::kernel_su::NONE).into());
    let (slot_label, lock_label, latest_label) = (
        tr!(l, "当前槽位", "目前槽位", "Current slot"),
        tr!(l, "开机锁", "開機鎖", "Bootloader"),
        tr!(l, "在线最新", "線上最新", "Latest online"),
    );
    let latest = online_saying(l, s.latest_kernelsu.as_deref(), status.and_then(|st| st.version.as_deref()), !status.is_some_and(|st| st.installed));
    match status {
        Some(st) => {
            rows.push(Row::new(l.pick(st.installed.then(|| texts::kernel_su::INSTALLED).unwrap_or_else(|| texts::kernel_su::NOT_INSTALLED))).trail(Trail::Value(&version)));
            rows.push(Row::new(&latest_label).trail(Trail::Value(&latest)));
            if let Some(target) = &st.backup_target {
                rows.push(Row::new(l.pick(texts::kernel_su::INSTALLED_TO)).trail(Trail::Value(target)));
            }
            if let Some(slot) = &st.slot {
                rows.push(Row::new(&slot_label).trail(Trail::Value(slot)));
            }
            rows.push(Row::new(&lock_label).trail(Trail::Value(l.pick([st.unlocked.then(|| "已解锁").unwrap_or("已锁定"), st.unlocked.then(|| "已解鎖").unwrap_or("已鎖定"), st.unlocked.then(|| "Unlocked").unwrap_or("Locked")]))));
        }
        None => rows.push(Row::new(l.pick(texts::kernel_su::NOT_READ)).trail(Trail::None)),
    }
    let list = widgets::list(&mut c, t, f, y, &rows);
    y = list.bottom + t.m.card_gap;

    // Writing it: the stage the job is at says how far it has come, and the
    // swipe under it writes the KMI the list chose.
    let said = match s.kernel_su_job.as_ref().map(|j| j.stage) {
        Some(KernelSuStage::Preparing) => l.pick(texts::kernel_su::WORKING),
        Some(KernelSuStage::BackingUp) => l.pick(texts::kernel_su::WORKING),
        Some(KernelSuStage::Patching) => l.pick(texts::kernel_su::WORKING),
        Some(KernelSuStage::Writing) => l.pick(texts::kernel_su::WORKING),
        Some(KernelSuStage::Checking) | None => l.pick(texts::kernel_su::INSTALL),
    };
    let stage_note = widgets::footnote(&mut c, t, f, y, said);
    y = stage_note + t.m.card_gap;
    let idle = tr!(l, "滑动以写入 KernelSU", "滑動以寫入 KernelSU", "Slide to write KernelSU");
    let (progress, write_label) = write_state(s.kernel_su_job.as_ref(), s.root_bytes, l, &idle);
    let track = widgets::confirm(&mut c, t, f, y, &write_label, false, progress);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::KernelSu) });
    y = track.y + track.h + t.m.card_gap;

    // The KMIs, each one the row to write it; the one the phone's kernel
    // names is marked, and the one written is too.
    if let Some(st) = status {
        y = widgets::section(&mut c, t, f, y, l.pick(texts::kernel_su::KMI));
        let kmis: Vec<Row> = st
            .kmis
            .iter()
            .enumerate()
            .map(|(k, row)| Row::new(&row.id).subtitle(&row.target).trail(Trail::Check(s.kernel_su_kmi == Some(k) || st.default == Some(k))))
            .collect();
        let picks: Vec<Action> = (0..kmis.len()).map(Action::KernelSuKmi).collect();
        let chosen = widgets::list(&mut c, t, f, y, &kmis);
        out.extend(hits(&chosen.rows, &picks));
        y = chosen.bottom + t.m.card_gap;
    }
    let note = widgets::footnote(&mut c, t, f, y, l.pick(texts::kernel_su::NOTE));
    y = note + t.m.card_gap;
    let online = widgets::footnote(&mut c, t, f, y, l.pick(texts::kernel_su::ONLINE_NOTE));
    y = online + t.m.card_gap;
    if let Some(why) = &why {
        let at = widgets::footnote(&mut c, t, f, y, why);
        y = at + t.m.card_gap;
    }

    let saved_at = widgets::section(&mut c, t, f, y, &texts::kernel_su::restore_target(l, st_target(status.and_then(|st| st.backup_target.clone()))));
    // 没有备份记录时不摆那一行可点的「已恢复」——点了必然报 no KernelSU backup
    // record（2026-11-26，取自 Ff428 的真机修正）。
    let bottom = if status.is_some_and(|st| st.has_backup) {
        let row = Row::new(l.pick(texts::kernel_su::RESTORED)).icon(SLATE, Icon::Restore);
        let list = widgets::list(&mut c, t, f, saved_at, &[row]);
        out.extend(hits(&list.rows, &[Action::RestoreKernelSu]));
        list.bottom
    } else {
        widgets::footnote(&mut c, t, f, saved_at, l.pick([
            "本机没有原厂 boot 备份，先写入一次 Root 才有得恢复。",
            "本機沒有原廠 boot 備份，先寫入一次 Root 才有得恢復。",
            "No stock boot backup on this phone; write a root once before restoring.",
        ]))
    };
    Frame { back: false, ..Frame::new(t, c.crop_height(fitted(t, bottom)), out, l.pick(texts::kernel_su::TITLE), t.m.content_top) }
}


/// 写入卡该停在哪、写什么：滑到底开始后停在底并说「正在写入」；写完停在底说
/// 「已写入」；失败回到原位，让手指知道可以再滑一次（2026-11-26）。
fn write_state(job: Option<&KernelSuJob>, bytes: Option<(u64, Option<u64>, u64)>, l: Lang, idle: &str) -> (f32, String) {
    match job {
        Some(job) if job.outcome.is_none() => {
            // 下载中把百分比与速度写在卡上，看得见进展（2026-11-26）。
            let worked = match bytes {
                Some((done, Some(total), rate)) if total > 0 => {
                    format!("{}% · {}", done.saturating_mul(100) / total, per_second(rate))
                }
                Some((done, None, rate)) => format!("{} · {}", fmt::bytes(done), per_second(rate)),
                _ => tr!(l, "准备中…", "準備中…", "Preparing…"),
            };
            (1.0, format!("{} {}", tr!(l, "正在在线写入", "正在線上寫入", "Writing online"), worked))
        }
        Some(job) if matches!(job.outcome, Some(KernelSuEnd::Failed(_))) => {
            (0.0, tr!(l, "写入未完成，可以再滑一次", "寫入未完成，可以再滑一次", "Not written; slide again"))
        }
        Some(_) => (1.0, tr!(l, "已写入", "已寫入", "Written")),
        None => (0.0, idle.to_string()),
    }
}

/// 在线那一版怎么写在页面上：比装着的这一版新就写「可升级」，一样或旧写
/// 「已是最新」，装都没装就只写版本号（2026-11-26）。
fn online_saying(l: Lang, online: Option<&str>, installed: Option<&str>, not_installed: bool) -> String {
    let Some(online) = online else {
        return tr!(l, "未获取（需要联网）", "未取得（需要連網）", "Not fetched (needs a network)");
    };
    if not_installed {
        return online.to_string();
    }
    match is_newer(installed, Some(online)) {
        Some(true) => format!("{online} · {}", tr!(l, "可升级", "可升級", "Update available")),
        Some(false) => format!("{online} · {}", tr!(l, "已是最新", "已是最新", "Up to date")),
        None => online.to_string(),
    }
}

/// 每秒多少字节，写成人看得懂的样子。
fn per_second(rate: u64) -> String {
    if rate == 0 {
        "…".to_string()
    } else {
        format!("{}/s", fmt::bytes(rate))
    }
}

/// The KMI the KernelSU page writes: the list, each one checked or not.
pub fn kernel_su_kmi(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let y = t.m.content_top;
    let st = s.kernel_su.as_ref().and_then(|r| r.as_ref().ok());
    let rows: Vec<Row> = st
        .map(|st| {
            st.kmis
                .iter()
                .enumerate()
                .map(|(k, row)| Row::new(&row.id).subtitle(&row.target).trail(Trail::Check(s.kernel_su_kmi == Some(k) || st.default == Some(k))))
                .collect()
        })
        .unwrap_or_default();
    let picks: Vec<Action> = (0..rows.len()).map(Action::KernelSuKmi).collect();
    let list = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&list.rows, &picks));
    Frame { back: false, ..Frame::new(t, c.crop_height(fitted(t, list.bottom)), out, l.pick(texts::kernel_su::KMI), t.m.content_top) }
}

/// Magisk 的页：现状与写进当前槽的 boot 镜像。
pub fn magisk(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let mut y = t.m.content_top;
    let st = s.magisk.as_ref().and_then(|r| r.as_ref().ok());
    let version = st
        .and_then(|st| st.version.clone())
        .unwrap_or_else(|| l.pick(texts::magisk::NONE).into());
    let (latest_label, latest) = (
        tr!(l, "在线最新", "線上最新", "Latest online"),
        online_saying(l, s.latest_magisk.as_deref(), st.and_then(|st| st.version.as_deref()), !st.is_some_and(|st| st.installed)),
    );
    let mut rows = Vec::new();
    match st {
        Some(st) => {
            rows.push(Row::new(l.pick(st.installed.then(|| texts::kernel_su::INSTALLED).unwrap_or_else(|| texts::kernel_su::NOT_INSTALLED))).trail(Trail::Value(&version)));
            rows.push(Row::new(&latest_label).trail(Trail::Value(&latest)));
            if let Some(target) = &st.backup_target {
                rows.push(Row::new(l.pick(texts::kernel_su::INSTALLED_TO)).trail(Trail::Value(target)));
            }
            rows.push(Row::new(&latest_label).trail(Trail::Value(&latest)));
            rows.push(Row::new(l.pick(texts::magisk::TARGET)).trail(Trail::Value(&st.target)));
            if let Some(slot) = &st.slot {
                rows.push(Row::new(l.pick(texts::kernel_su::TARGET)).trail(Trail::Value(slot)));
            }
        }
        None => rows.push(Row::new(l.pick(texts::kernel_su::NOT_READ)).trail(Trail::None)),
    }
    let list = widgets::list(&mut c, t, f, y, &rows);
    y = list.bottom + t.m.card_gap;
    let note = widgets::footnote(&mut c, t, f, y, l.pick(texts::magisk::NOTE));
    y = note + t.m.card_gap;
    let online = widgets::footnote(&mut c, t, f, y, l.pick(texts::kernel_su::ONLINE_NOTE));
    y = online + t.m.card_gap;
    let idle = tr!(l, "滑动以写入 Magisk", "滑動以寫入 Magisk", "Slide to write Magisk");
    let (progress, write_label) = write_state(s.magisk_job.as_ref(), s.root_bytes, l, &idle);
    let track = widgets::confirm(&mut c, t, f, y, &write_label, false, progress);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::Magisk) });
    y = track.y + track.h + t.m.card_gap;
    let saved_at = widgets::section(&mut c, t, f, y, &texts::magisk::restore_target(l, st_target(st.map(|s| s.target.clone()))));
    let bottom = if st.is_some_and(|st| st.has_backup) {
        let row = Row::new(l.pick(texts::kernel_su::RESTORED)).icon(SLATE, Icon::Restore);
        let list = widgets::list(&mut c, t, f, saved_at, &[row]);
        out.extend(hits(&list.rows, &[Action::RestoreMagisk]));
        list.bottom
    } else {
        widgets::footnote(&mut c, t, f, saved_at, l.pick([
            "本机没有原厂 boot 备份，先写入一次 Root 才有得恢复。",
            "本機沒有原廠 boot 備份，先寫入一次 Root 才有得恢復。",
            "No stock boot backup on this phone; write a root once before restoring.",
        ]))
    };
    Frame { back: false, ..Frame::new(t, c.crop_height(fitted(t, bottom)), out, l.pick(texts::magisk::TITLE), t.m.content_top) }
}

/// What the KernelSU row of the root page says beside its name.
fn kernel_su_saying(s: &State) -> String {
    match s.kernel_su.as_ref() {
        Some(Ok(st)) => st.version.as_deref().map(short_version).unwrap_or_else(|| s.lang.pick(texts::kernel_su::NOT_INSTALLED).into()),
        Some(Err(why)) => clip_saying(why),
        None => s.lang.pick(texts::kernel_su::NOT_READ).to_owned(),
    }
}

/// `3.3.1-56-gcfc60f8d (uapi: 6) (32736)` → `3.3.1`：右侧那一栏很窄，
/// 整串写进去会把「KernelSU」挤成「K...」（2026-11-26，取自 Ff428 的真机修正）。
fn short_version(version: &str) -> String {
    version.split(['-', ' ', '(', '+']).next().unwrap_or(version).to_string()
}

/// 一行说法最多这么长，理由太长就截断，别把右侧那一栏撑满。
fn clip_saying(why: &str) -> String {
    let line = why.lines().next().unwrap_or(why).trim();
    let mut out: String = line.chars().take(28).collect();
    if out.chars().count() < line.chars().count() {
        out.push('…');
    }
    out
}

/// What the Magisk row of the root page says beside its name.
fn magisk_saying(s: &State) -> String {
    match s.magisk.as_ref() {
        Some(Ok(st)) => st.version.as_deref().map(short_version).unwrap_or_else(|| s.lang.pick(texts::kernel_su::NOT_INSTALLED).into()),
        Some(Err(why)) => clip_saying(why),
        None => s.lang.pick(texts::kernel_su::NOT_READ).to_owned(),
    }
}

/// The partition a backup belongs to, as the restore row's title names it.
fn st_target(target: Option<String>) -> String {
    match target {
        Some(t) => t,
        None => "\u{2014}".to_owned(),
    }
}

/// What the auto-patch choice on the root page says beside its name.
fn auto_root_saying(s: &State, l: Lang) -> String {
    match s.auto_root {
        AutoRoot::None => l.pick(texts::root::NONE).to_owned(),
        AutoRoot::KernelSu => l.pick(texts::kernel_su::TITLE).to_owned(),
        AutoRoot::Magisk => l.pick(texts::magisk::TITLE).to_owned(),
    }
}

/// Preferences, opening with the stock「显示与亮度」theme previews.
pub fn settings(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let themes = widgets::theme_picker(&mut c, t, f, l, top + t.m.dp(8.9));
    out.extend(hits(&themes.rows, &[Action::Theme { dark: false }, Action::Theme { dark: true }]));
    let mut y = widgets::section(&mut c, t, f, themes.bottom, l.pick(texts::settings::BRIGHTNESS));
    let off = fmt::screen_off(l, SCREEN_OFF[s.screen_off]);
    let below = Row::new(l.pick(texts::page::SCREEN_TIMEOUT)).trail(Trail::Value(&off));
    let slider = widgets::slider_card(&mut c, t, f, y, Caption::Sun, l.pick(texts::settings::SCREEN_BRIGHTNESS), s.brightness, Some(below));
    out.push(Hit { rect: slider.track, action: Action::Slider(Slider::Brightness) });
    if let Some(r) = slider.below {
        out.push(Hit { rect: r, action: Action::Open(Screen::ScreenOff) });
    }
    y = widgets::section(&mut c, t, f, slider.bottom, l.pick(texts::settings::VIBRATION));
    let below = Row::new(l.pick(texts::settings::TOUCH_VIBRATION)).trail(Trail::Switch(s.vibrate));
    let vibration = widgets::slider_card(&mut c, t, f, y, Caption::Vibrate, l.pick(texts::settings::VIBRATION_INTENSITY), s.vibration, Some(below));
    out.push(Hit { rect: vibration.track, action: Action::Slider(Slider::Vibration) });
    if let Some(r) = vibration.below {
        out.push(Hit { rect: r, action: Action::Toggle(Switch::Vibrate) });
    }
    y = widgets::section(&mut c, t, f, vibration.bottom, l.pick(texts::settings::GENERAL));
    let general = widgets::list(&mut c, t, f, y, &[
        Row::new(l.pick(texts::page::LANGUAGE)).icon(BLUE, Icon::Globe).trail(Trail::Value(l.name())),
        Row::new(l.pick(texts::page::TIME_ZONE)).icon(ORANGE, Icon::Clock).trail(Trail::Value(s.zone_name())),
    ]);
    out.extend(hits(&general.rows, &[Action::Open(Screen::Language), Action::Open(Screen::TimeZone)]));
    // USB 那块（文件传输）搬到首页当条目了，设置里不再重复（2026-11-27）。
    let bottom = general.bottom;
    // 终端不在这里：高级页已经有一个，同一个东西开两个入口反而添乱
    //（2026-11-17）。关于与更新也一样，更新在首页底部单独一栏。
    Frame::new(t, fit(c, t, bottom), out, l.pick(SETTINGS), top)
}

/// What the settings page says beside its Skyward update entry: an update
/// under way, or a newer release found.
fn skyward_value(cx: &Ctx) -> Option<&'static str> {
    let (s, l) = (cx.state, cx.state.lang);
    if s.updating_skyward() {
        // 具体百分比在更新页里；首页这一行只说它还在进行，退出来也看得见。
        return Some(l.pick(["正在更新", "正在更新", "Updating"]));
    }
    match (s.skyward_newest(), &s.skyward_job) {
        (_, Some(SkywardJob { outcome: Some(SkywardEnd::Updated(_)), .. })) => Some(l.pick(texts::skyward_update::UPDATED)),
        (Some(n), _) if n.newer && n.build.is_some() => Some(l.pick(texts::update::NEW_VERSION)),
        _ => None,
    }
}

/// How long the screen stays on with nothing touched, opened from the
/// settings.
pub fn screen_off(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let top = t.m.content_top;
    let labels = SCREEN_OFF.map(|ms| fmt::screen_off(l, ms));
    let rows: Vec<Row> = labels.iter().enumerate().map(|(k, label)| Row::new(label).trail(Trail::Check(s.screen_off == k))).collect();
    let times = widgets::list(&mut c, t, f, top, &rows);
    let picks: Vec<Action> = (0..SCREEN_OFF.len()).map(Action::ScreenOff).collect();
    let y = widgets::footnote(&mut c, t, f, times.bottom, l.pick(texts::screen_off::NOTE));
    if SCREEN_OFF[s.screen_off].is_none() {
        widgets::footnote(&mut c, t, f, y, l.pick(texts::screen_off::STAYS_ON));
    }
    Frame::new(t, c, hits(&times.rows, &picks), l.pick(texts::page::SCREEN_TIMEOUT), top)
}

/// Mount manager: the slot's partitions, each with a switch to mount it
/// read-only or unmount it; one that programs are using says which, and
/// stays.
pub fn mount(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;

    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::mount::PARTITIONS));
    let bottom = match &s.mounts {
        Err(why) => {
            let y = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::mount::UNREADABLE)).trail(Trail::None)]).bottom;
            widgets::footnote(&mut c, t, f, y, why)
        }
        Ok(mounts) if mounts.is_empty() => widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::mount::NONE)).trail(Trail::None)]).bottom,
        Ok(mounts) => {
            let subs: Vec<String> = mounts
                .iter()
                .map(|m| match m.holders.is_empty() {
                    true => m.point.clone(),
                    false => texts::mount::in_use_by(l, &m.point, m.holders.join(l.pick(texts::list::SEPARATOR))),
                })
                .collect();
            let rows: Vec<Row> = mounts
                .iter()
                .zip(&subs)
                .map(|(m, sub)| {
                    let trail = if m.holders.is_empty() { Trail::Switch(m.mounted) } else { Trail::Text(l.pick(texts::mount::IN_USE)) };
                    Row::new(&m.base).subtitle(sub).trail(trail)
                })
                .collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            out.extend(list.rows.iter().zip(mounts).enumerate().map(|(i, (&rect, m))| {
                let action = match m.holders.is_empty() {
                    true => Action::MountToggle(i),
                    false => Action::Unavailable(l.pick(texts::mount::CANT_UNMOUNT_IN_USE)),
                };
                Hit { rect, action }
            }));
            let y = match &s.mount_failure {
                Some(failure) => widgets::footnote(&mut c, t, f, list.bottom, &fmt::mount_failure(l, failure)),
                None => list.bottom,
            };
            let about = match cx.info.slot.map(|s| s.letter()) {
                Some(slot) => texts::mount::shows_slot(l, slot),
                None => l.pick(texts::mount::SHOWS).into(),
            };
            widgets::footnote(&mut c, t, f, y, &about)
        }
    };

    Frame::new(t, fit(c, t, bottom), out, l.pick(texts::page::MOUNT), top)
}

/// Who made the recovery, as the About page names them under its version.
const AUTHOR: &str = "© 2026 Kirk Lin";

/// About this device, modelled on the stock「我的设备」: the wordmark, the
/// version and its author over a moving colour field, and the summary,
/// hardware and link cards scrolling over it. Only the cards are drawn
/// here; the canvas is clear wherever the field shows.
pub fn about(cx: &Ctx) -> Frame {
    let (t, f) = (cx.t, cx.f);
    let m = &t.m;
    // The page is drawn whole, under the field: laid out first to learn
    // how tall it comes to, then drawn that tall.
    let (_, _, bottom) = about_cards(cx, &mut Canvas::band(m.w, m.h * 3, 0..0, 0..0, Rgba { r: 0, g: 0, b: 0, a: 0 }));
    let mut c = Canvas::new(m.w, fitted(t, bottom));
    let (out, summary_top, bottom) = about_cards(cx, &mut c);
    let canvas = fit(c, t, bottom);
    let art = HeroArt {
        wordmark: widgets::wordmark(t, f, "Skyward", m.dp(233.7), m.dp(37.8)),
        version: widgets::version_line(t, f, RUNNING),
        author: widgets::author_line(t, f, AUTHOR),
        runs: canvas.runs(),
    };
    Frame { header: false, hero: Some(art), ..Frame::new(t, canvas, out, cx.state.lang.pick(texts::page::ABOUT), summary_top) }
}

/// The phone's processor by its marketing name in `l`, from the phone's
/// settings, or else by its cores.
fn soc(l: Lang, i: &DeviceInfo) -> Option<&str> {
    i.soc.as_ref().map(|n| l.among(n.hans.as_str(), n.hant.as_str(), n.en.as_str()))
}

/// The About page's cards on `c`: what touching them does, where the
/// first starts, and where the last ends.
fn about_cards(cx: &Ctx, c: &mut Canvas) -> (Vec<Hit>, f32, f32) {
    let (t, f, i, l) = (cx.t, cx.f, cx.info, cx.state.lang);
    let m = &t.m;
    let tone = Tone::hero(t);
    let mut out = Vec::new();

    let storage = fmt::storage_used(&i.storage).unwrap_or_else(|| fmt::capacity(i.storage.total));
    let mut facts = vec![
        Fact { title: l.pick(texts::specs::MODEL), value: i.model.clone(), link: false },
        Fact { title: l.pick(texts::specs::STORAGE), value: storage, link: true },
    ];
    if let Some(v) = &i.os_version {
        facts.push(Fact { title: l.pick(texts::specs::OS_VERSION), value: v.clone(), link: false });
    }
    if let Some(r) = &i.android {
        facts.push(Fact { title: l.pick(texts::specs::ANDROID_VERSION), value: r.clone(), link: false });
    }
    let summary = widgets::facts(c, t, f, m.dp(490.7), &facts, &tone);
    out.push(Hit { rect: summary.rows[1], action: Action::Open(Screen::Specs) });

    let processor = soc(l, i).map_or_else(|| fmt::cpu(l, &i.cpu), str::to_string);
    let mut specs = vec![(l.pick(texts::specs::PROCESSOR), processor), (l.pick(texts::specs::RAM), fmt::ram(i.ram_kib))];
    if let Some(design) = i.battery.as_ref().and_then(|b| b.design) {
        specs.push((l.pick(texts::specs::BATTERY_CAPACITY), fmt::mah(design)));
    }
    if let Some(d) = i.screen {
        specs.push((l.pick(texts::specs::SCREEN_SIZE), fmt::inches(d)));
    }
    if let Some(p) = i.panel {
        specs.push((l.pick(texts::specs::RESOLUTION), fmt::panel(p)));
    }
    let y = widgets::spec_card(c, t, f, summary.bottom + m.card_gap, &fmt::model_lines(&i.model), &specs, &tone);
    let links = widgets::list_toned(c, t, f, y + m.card_gap, &[Row::new(l.pick(texts::page::SPECS)), Row::new(l.pick(texts::page::LOGS))], &tone);
    out.extend(hits(&links.rows, &[Action::Open(Screen::Specs), Action::Open(Screen::Logs)]));
    (out, summary.rows[0].y, links.bottom)
}

/// Every reading the recovery has, modelled on the stock「全部参数与信息」.
pub fn specs(cx: &Ctx) -> Frame {
    let (t, f, i, l) = (cx.t, cx.f, cx.info, cx.state.lang);
    let gap = t.m.card_gap;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    // This page leaves more room under the title than the settings pages.
    let top = t.m.dp(188.0);

    let mut grid = vec![Spec { icon: Outline::Phone, label: l.pick(texts::specs::MODEL), value: fmt::model_lines(&i.model) }];
    if let Some(design) = i.battery.as_ref().and_then(|b| b.design) {
        grid.push(Spec { icon: Outline::Battery, label: l.pick(texts::specs::BATTERY_CAPACITY), value: fmt::mah(design) });
    }
    let cpu = match soc(l, i) {
        Some(soc) => format!("{soc}\n{}", fmt::cpu(l, &i.cpu)),
        None => fmt::cpu(l, &i.cpu),
    };
    grid.push(Spec { icon: Outline::Chip, label: l.pick(texts::specs::PROCESSOR), value: cpu });
    grid.push(Spec { icon: Outline::Memory, label: l.pick(texts::specs::RAM), value: fmt::ram(i.ram_kib) });
    if let Some(p) = i.panel {
        grid.push(Spec { icon: Outline::Resolution, label: l.pick(texts::specs::RESOLUTION), value: fmt::panel(p) });
    }
    if let Some(d) = i.screen {
        grid.push(Spec { icon: Outline::Screen, label: l.pick(texts::specs::SCREEN_SIZE), value: fmt::inches(d) });
    }
    grid.push(Spec { icon: Outline::Storage, label: l.pick(texts::specs::STORAGE_CAPACITY), value: fmt::capacity(i.storage.total) });
    grid.push(Spec { icon: Outline::Tag, label: l.pick(texts::specs::CODENAME), value: i.codename.clone() });
    let mut y = widgets::spec_grid(&mut c, t, f, top, &grid);

    if let Some(v) = &i.os_version {
        y = widgets::info_card(&mut c, t, f, y + gap, &[(l.pick(texts::specs::OS_VERSION), v.clone())]);
    }
    if let Some(r) = &i.android {
        y = widgets::info_card(&mut c, t, f, y + gap, &[(l.pick(texts::specs::ANDROID_VERSION), fmt::android(l, r, i.security_patch.as_ref()))]);
    }
    y = widgets::info_card(&mut c, t, f, y + gap, &[
        (l.pick(["Recovery 版本", "Recovery 版本", "Recovery version"]), format!("Skyward {RUNNING}")),
        (l.pick(["内核版本", "核心版本", "Kernel version"]), i.kernel.clone()),
    ]);

    let mut boot = Vec::new();
    if let Some(s) = i.slot {
        boot.push((l.pick(texts::slot::CURRENT), s.letter().to_string()));
        boot.push((l.pick(texts::page::UPDATE), fmt::snapshot(l, &i.snapshot).to_string()));
    }
    if let Some(unlocked) = i.boot.unlocked {
        let state = if unlocked { texts::specs::UNLOCKED } else { texts::specs::LOCKED };
        boot.push(("Bootloader", l.pick(state).to_string()));
    }
    if let Some(v) = &i.boot.verified {
        boot.push((l.pick(texts::specs::VERIFIED_BOOT), fmt::verified(l, v)));
    }
    boot.push((l.pick(texts::specs::DATA_PARTITION), fmt::data(l, &cx.state.data).to_string()));
    y = widgets::info_card(&mut c, t, f, y + gap, &boot);

    if let Some(b) = &i.battery {
        let mut rows = vec![(l.pick(texts::specs::LEVEL), fmt::level(l, b))];
        if let Some(v) = b.temp {
            rows.push((l.pick(texts::specs::TEMPERATURE), fmt::celsius(v)));
        }
        if let Some(v) = fmt::wear(l, b) {
            rows.push((l.pick(texts::specs::HEALTH), v));
        }
        if let Some(v) = b.cycles {
            rows.push((l.pick(texts::specs::CHARGE_CYCLES), texts::specs::cycles(l, v)));
        }
        if let Some(v) = b.voltage {
            rows.push((l.pick(texts::specs::VOLTAGE), fmt::volts(v)));
        }
        y = widgets::info_card(&mut c, t, f, y + gap, &rows);
    }

    let total = fmt::capacity(i.storage.total);
    let storage = match i.storage.free.map(fmt::gb) {
        Some(free) => texts::specs::free_of(l, free, total),
        None => texts::specs::total_unmounted(l, total),
    };
    y = widgets::info_card(&mut c, t, f, y + gap, &[(l.pick(texts::specs::INTERNAL_STORAGE), storage)]);
    let links = widgets::list(&mut c, t, f, y + gap, &[
        Row::new(l.pick(texts::page::LICENSE)).trail(Trail::Value("GPL-3.0")),
        Row::new(l.pick(texts::page::FONTS)).trail(Trail::Value("MiSans")),
    ]);
    out.extend(hits(&links.rows, &[Action::Open(Screen::License), Action::Open(Screen::FontLicense)]));
    Frame::new(t, fit(c, t, links.bottom), out, l.pick(texts::page::SPECS), top)
}

/// The data partition and its users' own files. The page opens the
/// partition as it shows, which is how it learns each user's screen lock;
/// a user whose files wait on one is asked with the control that lock is
/// answered with, from the start.
pub fn decrypt(cx: &Ctx) -> Frame {
    let (c, l) = (page(cx), cx.state.lang);
    match &cx.state.data {
        Data::Locked { user, keys, checking } => code(cx, c, *user, *keys, *checking),
        // Without a lock kept, the page the start opens on is the one to type
        // it into: nothing turns while the phone's own parts come up, and the
        // keyboard takes nothing until the lock is known.
        Data::Closed | Data::Opening if !cx.state.remember => code(cx, c, 0, Keys::Digits, true),
        Data::Closed | Data::Opening => data_status(
            cx,
            c,
            Icon::Lock,
            l.pick(["解密 data 分区", "解密 data 分割區", "Decrypt the data partition"]),
            Some(if cx.state.prepared {
                l.pick(["正在读取锁屏方式", "正在讀取螢幕鎖定方式", "Reading the screen lock"])
            } else {
                l.pick(["正在准备分区与服务", "正在準備分割區與服務", "Bringing the phone's own parts up"])
            }),
            None,
            true,
        ),
        Data::Open => {
            let t = cx.t;
            let m = &t.m;
            let mut frame = data_status(
                cx,
                c,
                Icon::Folder,
                l.pick(texts::decrypt::DECRYPTED),
                Some(l.pick(texts::decrypt::SEE_FILES)),
                None,
                false,
            );
            // The lock is shown here too, and dropped from here: once it is
            // kept the decrypt page opens straight into the menu, so this is
            // where it is come back to in order to stop keeping it.
            let y = m.dp(150.0) + m.dp(98.0) + m.dp(52.0) + m.dp(24.0);
            let (kept, not_kept) = (
                l.pick(["已记住密码", "已記住密碼", "Screen lock kept"]),
                l.pick(["未记住密码", "未記住密碼", "Screen lock not kept"]),
            );
            let row = widgets::list(
                &mut frame.canvas,
                t,
                cx.f,
                y,
                &[Row::new(match cx.state.remember {
                    true => kept,
                    false => not_kept,
                })
                .icon(if cx.state.remember { PURPLE } else { SLATE }, Icon::Lock)
                .trail(Trail::Switch(cx.state.remember))],
            );
            frame.hits.extend(hits(&row.rows, &[Action::Toggle(Switch::Remember)]));
            frame
        }
        Data::Failed(why) => data_status(cx, c, Icon::Lock, l.pick(texts::decrypt::FAILED), None, Some(why), false),
        Data::Erased => data_status(
            cx,
            c,
            Icon::Wipe,
            l.pick(DATA_ERASED),
            Some(l.pick(["下次开机时将重新创建", "下次開機時將重新建立", "It's made anew when the phone next starts"])),
            None,
            false,
        ),
    }
}

/// How far apart, in dp, the lines of the Decrypt page's title are where
/// it takes more than one.
const TITLE_PITCH: f32 = 32.0;

/// Where the partition stands, under its sign: a line on it, or why it
/// did not open and a way to try again; then the way off the page.
fn data_status(cx: &Ctx, mut c: Canvas, icon: Icon, title: &str, note: Option<&str>, why: Option<&str>, busy: bool) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let x = m.w as f32 / 2.0;
    let sign_cy = m.dp(150.0);
    c.fill_circle(x, sign_cy, m.dp(30.0), t.pal.card);
    icons::glyph(&mut c, icon, x, sign_cy, m.dp(40.0), t.pal.accent);
    // A ring about the sign that turns while the partition is being opened --
    // or while the page waits for the phone's own parts, which the opening
    // needs -- so that a page that is waiting has something moving on it.
    if busy {
        let r = m.dp(48.0);
        let start = -std::f32::consts::FRAC_PI_2 + cx.state.data_phase * std::f32::consts::TAU;
        c.stroke_circle(x, sign_cy, r, m.dp(1.5), t.pal.card);
        for k in 0..4 {
            let a = 0.5 + 0.5 * (k as f32 / 4.0);
            let color = crate::color::mix(t.pal.bg, t.pal.accent, a * 0.55);
            c.stroke_arc(x, sign_cy, r, start - 0.14 * (4 - k) as f32, 0.15, m.dp(2.5), color);
        }
        c.stroke_arc(x, sign_cy, r, start, 0.9, m.dp(2.5), t.pal.accent);
    }
    c.text_center(&f.heading, title, x, sign_cy + m.dp(88.0), m.dp(24.0), t.pal.text);
    if let Some(note) = note {
        c.text_center(&f.body, note, x, sign_cy + m.dp(142.0), m.sub_px, t.pal.subtitle);
    }
    // No bar: the opening says nothing of how far along it is, and one drawn to
    // a guess of its length only ever disagreed with the work (2026-11-16).
    // The turning ring over the sign says the page is waiting, and that is all
    // that is known.
    let mut out = Vec::new();
    if let Some(why) = why {
        widgets::paragraph(&mut c, t, f, sign_cy + m.dp(112.0), why);
        let again = widgets::button(&mut c, t, f, m.h as f32 * 0.62, l.pick(["重试", "重試", "Try again"]));
        out.push(Hit { rect: again, action: Action::OpenData });
    }
    let leave_cy = m.h as f32 - m.dp(64.0);
    let leave = l.pick(if matches!(cx.state.data, Data::Open | Data::Erased) { texts::common::DONE } else { texts::decrypt::NOT_NOW });
    c.text_center(&f.heading, leave, x, leave_cy, m.row_title_px, t.pal.accent);
    out.push(Hit { rect: Rect::new(x - m.dp(80.0), leave_cy - m.dp(24.0), m.dp(160.0), m.dp(48.0)), action: Action::Skip });
    Frame { header: false, ..Frame::new(t, c, out, l.pick(texts::page::DECRYPT), 0.0) }
}

/// A user's own files wait on their screen lock: the control it is
/// answered with, which takes nothing while what was entered is checked.
fn code(cx: &Ctx, mut c: Canvas, user: u32, keys: Keys, checking: bool) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let x = m.w as f32 / 2.0;
    let mut out = Vec::new();
    let title = match (keys == Keys::Pattern, user) {
        (true, 0) => l.pick(texts::decrypt::DRAW_PATTERN).to_string(),
        (true, user) => texts::decrypt::draw_pattern_of(l, user),
        (false, 0) => l.pick(texts::decrypt::ENTER_LOCK).to_string(),
        (false, user) => texts::decrypt::enter_lock_of(l, user),
    };
    let note = l.pick(if checking { texts::decrypt::CHECKING } else { texts::decrypt::TO_SEE_FILES });
    let room = m.w as f32 - m.title_x * 2.0;
    let more = widgets::centred(&mut c, &f.heading, &title, x, m.dp(130.0), m.dp(24.0), m.dp(TITLE_PITCH), t.pal.text, room);
    widgets::centred(&mut c, &f.body, note, x, m.dp(166.0) + more, m.sub_px, m.sub_pitch, t.pal.subtitle, room);
    let mut pattern = None;
    let bottom = match keys {
        Keys::Letters => {
            let top = kb_top(&t);
            out.extend(crate::ime::bar(&mut c, &t, f, &s.ime, text_floor(&t)));
            let caps = widgets::keyboard(&mut c, t, f, l, top, s.layer, s.key_held(), l.pick(DECRYPT), s.ime.enabled);
            if !checking {
                out.extend(caps.into_iter().map(|(rect, cap)| Hit { rect, action: Action::Type(cap) }));
            }
            top
        }
        Keys::Pattern => {
            let span = m.dp(300.0);
            let cy = m.dp(300.0) + span / 2.0;
            widgets::pattern_lock(&mut c, t, x, cy, span, &s.pattern);
            if !checking {
                out.push(Hit { rect: Rect::new(x - span, cy - span, span * 2.0, span * 2.0), action: Action::Pattern });
                pattern = Some((x, cy, span));
            }
            m.h as f32 - m.dp(26.0)
        }
        Keys::Digits => {
            let top = m.h as f32 - widgets::keypad_height(t);
            let caps = widgets::keypad(&mut c, t, f, l, top, s.key_held());
            if !checking {
                out.extend(caps.into_iter().map(|(rect, cap)| Hit { rect, action: Action::Type(cap) }));
            }
            top
        }
    };
    // The dots are up under the words, and what is kept is said below them; a
    // pattern is its own showing of what has been drawn.
    if keys != Keys::Pattern {
        widgets::code_dots(&mut c, t, x, m.dp(212.0), s.code.chars().count());
    }
    let skip_cy = bottom - m.dp(34.0);
    c.text_center(&f.heading, l.pick(texts::decrypt::NOT_NOW), x, skip_cy, m.row_title_px, t.pal.accent);
    out.push(Hit { rect: Rect::new(x - m.dp(80.0), skip_cy - m.dp(24.0), m.dp(160.0), m.dp(48.0)), action: Action::Skip });
    // Once something has been typed, it can be kept for the starts that
    // follow: the risk of keeping it is put before it when it is.
    if !s.code.is_empty() && !checking {
        // Under the dots, clear of the pattern where that is what answers.
        let row_y = match keys {
            Keys::Pattern => m.dp(636.0),
            _ => m.dp(258.0),
        };
        // The switch is the state itself: taken across, the lock is kept; left
        // as it is, it is not, and the words say which of the two it is.
        // A pill about what is written, not a row across the page: the dots
        // above are small, and a card from edge to edge would drown them.
        let kept = l.pick(["已记住密码", "已記住密碼", "Screen lock kept"]);
        let not_kept = l.pick(["未记住密码", "未記住密碼", "Screen lock not kept"]);
        let word = match s.remember {
            true => kept,
            false => not_kept,
        };
        let px = m.sub_px;
        let pad = m.dp(18.0);
        let gap = m.dp(12.0);
        let pill_h = m.dp(48.0);
        let text = c.text_width(&f.body, word, px);
        let total = pad + text + gap + m.switch_w + pad;
        let left = x - total / 2.0;
        c.fill_round_rect(left, row_y, total, pill_h, pill_h / 2.0, t.pal.card);
        c.text_center(&f.body, word, left + pad + text / 2.0, row_y + pill_h / 2.0, px, t.pal.text);
        widgets::switch(&mut c, t, left + total - pad, row_y + pill_h / 2.0, s.remember);
        out.push(Hit { rect: Rect::new(left, row_y, total, pill_h), action: Action::Toggle(Switch::Remember) });
    }
    Frame { header: false, pattern, ..Frame::new(t, c, out, l.pick(DECRYPT), 0.0) }
}

/// Reboot target picker with a swipe-to-confirm control.
pub fn reboot(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let mut y = widgets::section_first(&mut c, t, f, top, l.pick(texts::reboot::TO));
    let rows: Vec<Row> = REBOOT_TARGETS
        .iter()
        .enumerate()
        .map(|(k, &(title, sub))| {
            let r = Row::new(l.pick(title)).trail(Trail::Check(s.reboot == k));
            match sub {
                Some(text) => r.subtitle(l.pick(text)),
                None => r,
            }
        })
        .collect();
    let targets = widgets::list(&mut c, t, f, y, &rows);
    let picks: Vec<Action> = (0..TARGETS.len()).map(Action::RebootTarget).collect();
    out.extend(hits(&targets.rows, &picks));
    y = targets.bottom;
    if let Some(slot) = cx.info.slot {
        let next = match &s.boot_slots {
            Some(Ok(slots)) if slots.active == slot => texts::reboot::current(l, slot.letter()),
            Some(Ok(slots)) => slots.active.letter().to_string(),
            Some(Err(_)) => l.pick(texts::common::UNREADABLE).to_string(),
            None => String::new(),
        };
        y = widgets::section(&mut c, t, f, y, l.pick(texts::slot::TITLE));
        let slots = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::reboot::NEXT_BOOT)).trail(Trail::Value(&next))]);
        out.extend(hits(&slots.rows, &[Action::Open(Screen::Slot)]));
        y = slots.bottom;
    }
    let (label, danger) = confirm_label(Confirm::Reboot, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::Reboot) });
    Frame::new(t, c, out, l.pick(texts::page::REBOOT), top)
}

/// What the charger is doing: the power it delivers, the protocol it
/// negotiated, and the battery it fills.
///
/// This page reads and does not set. On this phone the nodes that would hold
/// a charge limit are policy votes rather than figures, and a number written
/// to them is refused, so the recovery cannot change how fast the phone
/// charges — the kernel and the charger settle that between them before the
/// page is ever drawn. What the page can do is say what they settled on,
/// which is what a recovery is asked for when a charge looks slow.
pub fn charge(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let ch = &s.charge;
    let b = cx.info.battery.as_ref();

    // What the charger is giving, and the most it has given this charge: the
    // two numbers that say how fast this charge is running.
    let now = match ch.now {
        Some(tenths) => fmt::watts(tenths),
        None if b.is_some_and(|b| b.plugged) => l.pick(["读取中", "讀取中", "Reading"]).to_owned(),
        None => l.pick(["未接充电器", "未接充電器", "No charger"]).to_owned(),
    };
    let most = match ch.most {
        Some(tenths) => fmt::watts(tenths),
        None => "\u{2014}".to_owned(),
    };
    let y = widgets::section_first(&mut c, t, f, top, l.pick(["现在", "現在", "Now"]));
    let rows = [Row::new(&now).icon(BLUE, Icon::Battery).trail(Trail::Value(&most))];
    let list = widgets::list(&mut c, t, f, y, &rows);
    let mut y = list.bottom;

    // How fast the phone is let to charge, chosen before the charger and the
    // battery are told of: it is the one thing on this page that changes what
    // the phone does rather than what it says. The programs that settle it are
    // started as the recovery starts, so a step chosen here reaches the phone
    // at the next start, and the page says so rather than letting the steps
    // look as if they had taken at once.
    {
        let steps: [(ChargeMode, Text, Text); 3] = [
            (
                ChargeMode::Plain,
                ["默认", "預設", "Default"],
                [
                    "不启动充电服务，内核把电流限在它给未认证电池的上限，5W+。",
                    "不啟動充電服務，核心把電流限在它給未認證電池的上限，5W+。",
                    "Neither service runs, so the kernel holds the current at the ceiling it sets an unchecked battery: 5W+.",
                ],
            ),
            (
                ChargeMode::Fast,
                ["快充", "快充", "Fast"],
                [
                    "启动小米充电服务，内核被告知电池已通过认证，10W+。",
                    "啟動小米充電服務，核心被告知電池已通過認證，10W+。",
                    "Xiaomi's charging service runs, so the kernel is told the battery passed its check: 10W+.",
                ],
            ),
            (
                ChargeMode::Fastest,
                ["超快充", "超快充", "Fastest"],
                [
                    "再启动小米的认证进程，充电器通过校验后可升压到 PPS 档，40W+。",
                    "再啟動小米的認證行程，充電器通過校驗後可升壓到 PPS 檔，40W+。",
                    "Xiaomi's secret keeper runs as well, so the charger may be verified and the protocol go on to the adjustable kind: 40W+.",
                ],
            ),
        ];
        y = widgets::section(&mut c, t, f, y, l.pick(["充电档位", "充電檔位", "Charge step"]));        let rows: Vec<Row> = steps
            .iter()
            .map(|(mode, name, detail)| Row::new(l.pick(*name)).subtitle(l.pick(*detail)).trail(Trail::Check(s.charge_mode == *mode)))
            .collect();
        let list = widgets::list(&mut c, t, f, y, &rows);
        y = list.bottom;
        // Every row is a hit, the one already chosen too: a row that alone
        // took no touch would be the one row that never darkens under a
        // finger, and the three would read as three different backgrounds
        // (2026-11-25). Tapping the chosen one changes nothing, which is what
        // `act` does with a step that is already the one kept.
        for (i, (mode, _, _)) in steps.iter().enumerate() {
            out.extend(hits(&list.rows[i..=i], &[Action::Toggle(Switch::ChargeStep(*mode))]));
        }
        y = widgets::footnote(
            &mut c,
            t,
            f,
            y + t.m.dp(8.0),
            l.pick([
                "档位在下次开机时生效：它决定 recovery 启动哪些小米服务，而这些服务只在启动时决定一次。",
                "檔位在下次開機時生效：它決定 recovery 啟動哪些小米服務，而這些服務只在啟動時決定一次。",
                "A step takes effect the next time the recovery starts: it decides which of Xiaomi's services the recovery starts, and those are decided once, as it starts.",
            ]),
        );
    }

    // The thermal guard, above the charger it watches: how warm the battery
    // may get before the charge is stepped down, and starting the recovery
    // again to make the step take. It is neither offered nor obeyed on the
    // plain step, where nothing a charge draws can warm the battery enough to
    // matter, so the slider is drawn dimmed and takes no touch.
    {
        let guarded = s.charge_mode.guarded();
        y = widgets::section(&mut c, t, f, y + t.m.dp(8.0), l.pick(["温控", "溫控", "Thermal guard"]));
        let span = f32::from(THERMAL_LIMIT_MOST - THERMAL_LIMIT_LEAST);
        let level = (f32::from(s.thermal_limit - THERMAL_LIMIT_LEAST) / span).clamp(0.0, 1.0);
        // The limit is written into the label, since the card has no value of
        // its own: the switch below it is the guard itself.
        let label = format!(
            "{} · {} °C",
            l.pick(["电池温度上限", "電池溫度上限", "Battery temperature limit"]),
            s.thermal_limit
        );
        let below = Row::new(l.pick(["超出后降低充电档位", "超出後降低充電檔位", "Step the charge down when past"]))
            .trail(Trail::Switch(s.thermal_guard && guarded));
        let card = widgets::slider_card(&mut c, t, f, y + t.m.dp(8.0), Caption::Thermometer, &label, level, Some(below));
        // The limit is the same number whatever the step, so it is drawn as it
        // stands even where it cannot be changed: a slider drawn at its left
        // end would say the limit had been moved there.
        //
        // A limit is only worth setting while the guard that reads it is on,
        // so the slider takes no touch where the switch is off, or where the
        // step starts nothing the guard has any say over. The switch itself is
        // still the guard's to turn, wherever a step is guarded at all.
        // The card keeps the same background whatever the step: a card laid
        // over with a press colour read as one card of a different colour from
        // its neighbours, which is what it looked like on the phone
        // (2026-11-25). What it cannot take is the touch, and that is said by
        // the slider being left out of the hits rather than by a wash.
        let lit = guarded && s.thermal_guard;
        if lit {
            out.push(Hit { rect: card.track, action: Action::Slider(Slider::ThermalLimit) });
        }
        if guarded && let Some(r) = card.below {
            out.push(Hit { rect: r, action: Action::Toggle(Switch::ThermalGuard) });
        }
        y = card.bottom;
        y = widgets::footnote(
            &mut c,
            t,
            f,
            y + t.m.dp(8.0),
            if guarded {
                l.pick([
                    "开关和温度都是即时生效的：开启后温度才可调，调完就按新温度盯着，电池一到这个温度就把充电档位降一档，然后重启回 recovery 按新档位充电。默认档不启动任何服务，充电很慢，不会让电池过热，所以默认档下这一项不可调，也不生效。",
                    "開關和溫度都是即時生效的：開啟後溫度才可調，調完就按新溫度盯著，電池一到這個溫度就把充電檔位降一檔，然後重啟回 recovery 按新檔位充電。預設檔不啟動任何服務，充電很慢，不會讓電池過熱，所以預設檔下這一項不可調，也不生效。",
                    "The switch and the temperature both take at once: the limit may be set only while the guard is on, and it is watched as it is set, so a battery that reaches it has the charge stepped down there and then, starting the recovery again to charge at the new step. The default step starts no service at all, so its charge is slow enough not to overheat the battery: the guard is neither offered nor obeyed there.",
                ])
            } else {
                l.pick([
                    "默认档不启动任何服务，充电很慢，不会让电池过热，所以这一项不可调，也不生效。",
                    "預設檔不啟動任何服務，充電很慢，不會讓電池過熱，所以這一項不可調，也不生效。",
                    "The default step starts no service at all, so its charge is slow enough not to overheat the battery: this is neither offered nor obeyed there.",
                ])
            },
        );
    }

    // What the charger negotiated, and what it says it can give. These are
    // the kernel's own words, left as it writes them: a protocol name is a
    // name, and translating it would only make it harder to look up.
    let mut charger: Vec<(&str, String)> = Vec::new();
    if let Some(protocol) = &ch.protocol {
        charger.push((l.pick(["协议", "協定", "Protocol"]), protocol.clone()));
    }
    if let Some(rated) = ch.rated {
        charger.push((l.pick(["充电器标称", "充電器標稱", "Charger rated"]), fmt::watts(rated * 10)));
    }
    if let Some(uv) = ch.volts {
        charger.push((l.pick(["输入电压", "輸入電壓", "Input voltage"]), fmt::volts(uv)));
    }
    if let Some(ua) = ch.amps {
        charger.push((l.pick(["输入电流", "輸入電流", "Input current"]), fmt::amps(ua)));
    }
    if !charger.is_empty() {
        y = widgets::section(&mut c, t, f, y, l.pick(["充电器", "充電器", "Charger"]));
        y = widgets::info_card(&mut c, t, f, y, &charger);
    }

    // The battery being filled: what it is at, how warm it is, and what the
    // charge is doing to it. The warmth matters more here than anywhere else
    // on the pages, since a fast charge is what makes it.
    if let Some(b) = b {
        let mut cells = vec![(l.pick(["电量", "電量", "Level"]), fmt::level(l, b))];
        if let Some(v) = b.temp {
            cells.push((l.pick(["温度", "溫度", "Temperature"]), fmt::celsius(v)));
        }
        if let Some(w) = b.watts() {
            cells.push((l.pick(["电池功率", "電池功率", "Battery power"]), fmt::watts((w * 10.0).round() as u32)));
        }
        if let Some(v) = b.voltage {
            cells.push((l.pick(["电池电压", "電池電壓", "Battery voltage"]), fmt::volts(v)));
        }
        if let Some(v) = fmt::wear(l, b) {
            cells.push((l.pick(["健康", "健康狀態", "Health"]), v));
        }
        if let Some(v) = b.cycles {
            cells.push((l.pick(["充电循环", "充電循環", "Charge cycles"]), tr!(l, "{v} 次", "{v} 次", "{v}")));
        }
        y = widgets::section(&mut c, t, f, y, l.pick(["电池", "電池", "Battery"]));
        y = widgets::info_card(&mut c, t, f, y, &cells);
    }

    // What the page cannot do, said plainly, since a page about charging that
    // stayed silent would look as if it could make the phone charge faster.
    // The step above settles which of the phone's own programs run; the
    // protocol itself is still the charger's and the kernel's to settle.
    let note = l.pick([
        "档位只决定 recovery 启动哪些小米服务；能到多少功率仍由充电器与内核协商，并不保证达到某个数值。",
        "檔位只決定 recovery 啟動哪些小米服務；能到多少功率仍由充電器與核心協商，並不保證達到某個數值。",
        "A step settles which of Xiaomi's services the recovery runs: how much power comes of it is still the charger's and the kernel's to settle, and no particular figure is promised.",
    ]);
    y = widgets::footnote(&mut c, t, f, y + t.m.dp(10.0), note);

    Frame::new(t, fit(c, t, y), out, l.pick(CHARGE), top)
}

/// 文件传输：内部存储交给电脑的那个开关，以及它此刻有没有真的给出去。
/// 首页那一条点进来，开关放在这一页，首页只露「开 / 关」（2026-11-27）。
pub fn file_transfer(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let on = s.file_transfer;
    let open = s.data.storage_open();

    let y = widgets::section_first(&mut c, t, f, top, l.pick(["开关", "開關", "Switch"]));
    let rows = [Row::new(l.pick(["文件传输", "檔案傳輸", "File transfer"]))
        .icon(BLUE, Icon::Usb)
        .subtitle(l.pick([
            "打开后内部存储交给电脑；数据线插上，电脑上就能看到手机。",
            "開啟後內部儲存交給電腦；資料線插上，電腦上就能看到手機。",
            "Turn it on and internal storage goes to the computer: plug the cable in and the phone shows up there.",
        ]))
        .trail(Trail::Switch(on))];
    let list = widgets::list(&mut c, t, f, y, &rows);
    out.push(Hit { rect: list.rows[0], action: Action::Toggle(Switch::FileTransfer) });
    let mut y = list.bottom;

    y = widgets::section(&mut c, t, f, y + t.m.card_gap, l.pick(["现在", "現在", "Now"]));
    let now = match (s.transfer_running, open) {
        (false, _) => l.pick([
            "关着，电脑上看不到手机",
            "關著，電腦上看不到手機",
            "Off: the computer finds no phone",
        ]),
        (true, true) => l.pick([
            "开着，内部存储已经交给电脑",
            "開著，內部儲存已經交給電腦",
            "On: internal storage is offered",
        ]),
        (true, false) => l.pick([
            "开着，但内部存储还没解锁，暂时不交给电脑",
            "開著，但內部儲存還沒解鎖，暫時不交給電腦",
            "On, but internal storage is not unlocked yet, so nothing is offered",
        ]),
    };
    let rows = [Row::new(now).icon(if s.transfer_running { BLUE } else { SLATE }, Icon::Layers)];
    let list = widgets::list(&mut c, t, f, y, &rows);
    y = list.bottom;

    let note = l.pick([
        "开关只决定 recovery 里那套 MTP 服务跑不跑；电脑上看不看得见，还要数据线插在电脑上、电脑认这个设备。数据分区没解锁时不会把盘交出去，先解密再来。",
        "開關只決定 recovery 裡那套 MTP 服務跑不跑；電腦上看不看得見，還要資料線插在電腦上、電腦認這個裝置。資料分區沒解鎖時不會把盤交出去，先解密再來。",
        "The switch settles whether the recovery's own MTP service runs: whether the computer sees the phone still needs the cable in it and the device recognised there. Nothing is offered while the data partition is locked, so unlock it first.",
    ]);
    y = widgets::footnote(&mut c, t, f, y + t.m.dp(10.0), note);

    Frame::new(t, fit(c, t, y), out, l.pick(["文件传输", "檔案傳輸", "File transfer"]), top)
}

/// Installing, opened from the home page: the system on the phone now, and
/// where a package can come from.
pub fn install(cx: &Ctx) -> Frame {
    let (t, f, i, l) = (cx.t, cx.f, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut facts = Vec::new();
    if let Some(v) = &i.os_version {
        facts.push((l.pick(texts::install::CURRENT_SYSTEM), v.clone()));
    }
    if let Some(p) = &i.security_patch {
        facts.push((l.pick(texts::install::SECURITY_PATCH), fmt::patch(l, p)));
    }
    if let Some(s) = i.slot {
        facts.push((l.pick(texts::slot::CURRENT), texts::install::slot_detail(l, s.letter(), s.other().letter())));
    }
    let ways = l.pick(texts::install::WAYS);
    let y = if facts.is_empty() {
        widgets::section_first(&mut c, t, f, top, ways)
    } else {
        let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::common::THIS_PHONE));
        let y = widgets::info_card(&mut c, t, f, y, &facts);
        widgets::section(&mut c, t, f, y, ways)
    };
    let mut rows = Vec::new();
    if let Some(note) = fmt::snapshot_note(l, &i.snapshot) {
        rows.push(Row::new(fmt::snapshot(l, &i.snapshot)).icon(ORANGE, Icon::Layers).subtitle(note).trail(Trail::None));
    }
    // Internal storage opens with the data partition, which the decrypt
    // page opens or tells why it does not.
    let data = &cx.state.data;
    let (storage, from_storage) = if data.storage_open() {
        (texts::install::CHOOSE_IN_STORAGE, Action::Open(Screen::Choose(Place::Storage)))
    } else if *data == Data::Erased {
        (texts::data::ERASED, Action::Open(Screen::Decrypt))
    } else {
        (texts::decrypt::NEEDED, Action::Open(Screen::Decrypt))
    };
    // The drive's page waits for one to be plugged in, and says why one
    // that is cannot be read.
    let usb = match &cx.state.usb_drive {
        Some(UsbDrive::Shown) => texts::install::CHOOSE_ON_USB_DRIVE,
        Some(UsbDrive::Unreadable(_)) => texts::usb_drive::UNREADABLE,
        None => texts::usb_drive::WANTED,
    };
    rows.push(Row::new(install_title(l, Source::File(Place::Storage))).icon(BLUE, Icon::Phone).subtitle(l.pick(storage)));
    rows.push(Row::new(install_title(l, Source::File(Place::UsbDrive))).icon(PURPLE, Icon::UsbDrive).subtitle(l.pick(usb)));
    let sideload = l.pick(texts::install::SIDELOAD);
    rows.push(Row::new(install_title(l, Source::Computer)).icon(GREEN, Icon::Usb).subtitle(sideload));
    let methods = widgets::list(&mut c, t, f, y, &rows);
    let from_usb = Action::Open(Screen::Choose(Place::UsbDrive));
    let mut out = hits(&methods.rows[methods.rows.len() - 3..], &[from_storage, from_usb, Action::Sideload]);
    // A phone that can hold a second system installs one from here too.
    let what = l.pick(match cx.state.dual_boot {
        true => texts::install::WHAT_WITH_SECOND_SYSTEM,
        false => texts::install::WHAT,
    });
    let y = widgets::footnote(&mut c, t, f, methods.bottom, what);
    let y = widgets::section(&mut c, t, f, y, l.pick(texts::common::OPTIONS));
    let check = Row::new(l.pick(texts::install::VERIFY_SIGNATURES)).trail(Trail::Switch(cx.state.check_signature));
    let signature = widgets::list(&mut c, t, f, y, &[check]);
    out.extend(hits(&signature.rows, &[Action::Toggle(Switch::Signature)]));
    let mut y = widgets::footnote(&mut c, t, f, signature.bottom, l.pick(texts::install::VERIFY_SIGNATURES_NOTE));
    if cx.state.recovery_partitions {
        let row = Row::new(l.pick(texts::install::FLASH_SKYWARD_AGAIN)).trail(Trail::Switch(cx.state.keep_recovery));
        let options = widgets::list(&mut c, t, f, y + t.m.card_gap, &[row]);
        out.extend(hits(&options.rows, &[Action::Toggle(Switch::KeepRecovery)]));
        y = widgets::footnote(&mut c, t, f, options.bottom, l.pick(texts::install::FLASH_SKYWARD_AGAIN_NOTE));
    }
    Frame::new(t, fit(c, t, y), out, l.pick(texts::page::INSTALL), top)
}

/// How installing from `source` is named: on the install page, and as the
/// title of the page that installs.
fn install_title(l: Lang, source: Source) -> &'static str {
    use texts::install::title;
    l.pick(match source {
        Source::Computer => title::FROM_COMPUTER,
        Source::File(Place::Storage) => title::FROM_STORAGE,
        Source::File(Place::UsbDrive) => title::FROM_USB_DRIVE,
        Source::Opened { .. } => title::FROM_FILES,
        Source::Asked { security: false } => title::SYSTEM_UPDATE,
        Source::Asked { security: true } => title::SECURITY_UPDATE,
        Source::Rescue => title::RESCUE,
    })
}

/// What a place is called at the top of its folders.
fn place_name(l: Lang, place: Place) -> &'static str {
    l.pick(match place {
        Place::Storage => texts::place::STORAGE,
        Place::UsbDrive => texts::place::USB_DRIVE,
    })
}

/// A one-line card on how things stand.
fn standing<'a>(title: &'a str, color: Rgba, icon: Icon, detail: &'a str) -> Row<'a> {
    Row::new(title).icon(color, icon).subtitle(detail).trail(Trail::None)
}

fn share(done: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (done as f64 / total as f64) as f32
    }
}

fn percent(share: f32) -> u32 {
    (share * 100.0).floor() as u32
}

/// Where an update stands, and the share of its step done when known.
fn update_stage(l: Lang, stage: Option<&Stage>) -> (String, Option<f32>) {
    match stage {
        Some(Stage::Writing { done, total }) => {
            let s = share(*done, *total);
            (texts::installing::update_writing(l, percent(s)), Some(s))
        }
        Some(Stage::Verifying { index, count, partition }) => {
            (texts::installing::update_verifying(l, partition, index + 1, count), Some(*index as f32 / *count as f32))
        }
        Some(Stage::Finishing) => (l.pick(texts::installing::UPDATE_FINISHING).into(), None),
        Some(Stage::Reflashing { done, total }) => (l.pick(texts::installing::REFLASHING).into(), Some(*done as f32 / *total as f32)),
        Some(
            Stage::Resizing | Stage::Flashing { .. } | Stage::Script { .. } | Stage::Signature { .. } | Stage::Allocating { .. } | Stage::Root { .. } | Stage::Storing,
        )
        | None => {
            (l.pick(texts::installing::GETTING_READY).into(), None)
        }
    }
}

/// Where installing a system beside the installed one stands, and the share
/// of its step done when known.
fn system_stage(l: Lang, stage: Option<&Stage>) -> (String, Option<f32>) {
    match stage {
        Some(Stage::Allocating { done, total }) => {
            let s = share(*done, *total);
            (texts::installing::system_allocating(l, percent(s)), Some(s))
        }
        Some(Stage::Root { done, total }) => {
            let s = share(*done, *total);
            let said = match done * 2 <= *total {
                true => texts::installing::system_writing(l, percent(s)),
                false => texts::installing::system_verifying(l, percent(s)),
            };
            (said, Some(s))
        }
        Some(Stage::Storing) => (l.pick(texts::installing::SYSTEM_STORING).into(), None),
        _ => (l.pick(texts::installing::GETTING_READY).into(), None),
    }
}

/// What the card says while an install script runs, and how far it is: the
/// last line the script showed and how far it says it is, then Skyward
/// going back into the recovery partitions it wrote over.
fn script_stage<'a>(l: Lang, stage: Option<&Stage>, said: &'a [String]) -> (&'a str, Option<f32>) {
    let running = l.pick(texts::installing::RUNNING_SCRIPT);
    let last = said.iter().rev().map(|line| line.trim()).find(|line| !line.is_empty()).unwrap_or(running);
    match stage {
        Some(Stage::Reflashing { done, total }) => (l.pick(texts::installing::REFLASHING), Some(*done as f32 / *total as f32)),
        Some(Stage::Script { thousandths }) => (last, Some(f32::from(*thousandths) / 1000.0)),
        _ => (last, None),
    }
}

/// How Skyward went back into the recovery partitions the install wrote
/// over, where it did.
fn reflashed(c: &mut Canvas, cx: &Ctx, y: f32) -> f32 {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    match &cx.state.reflash {
        None => y,
        Some(Reflash::Written(written)) => {
            let said = match written.as_slice() {
                [slot] => texts::installing::reflashed_one(l, slot.letter()),
                [a, b] => texts::installing::reflashed_two(l, a.letter(), b.letter()),
                other => unreachable!("a phone has two slots, and Skyward went back into {other:?}"),
            };
            widgets::footnote(c, t, f, y, &said)
        }
        Some(Reflash::Failed(why)) => {
            let y = widgets::footnote(c, t, f, y, l.pick(texts::installing::REFLASH_FAILED));
            widgets::footnote(c, t, f, y, why)
        }
    }
}

/// What the install script showed, where there is any: its heading, and
/// under it the lines as the page's long list, the last thing on the page.
fn script_output(c: &mut Canvas, cx: &Ctx, y: f32, rows: &mut Option<Rows>) -> f32 {
    let lines = &cx.state.script_output;
    if lines.is_empty() {
        return y;
    }
    let y = widgets::section(c, cx.t, cx.f, y, cx.state.lang.pick(texts::installing::SCRIPT_OUTPUT));
    *rows = Some(Rows { edges: widgets::output_edges(c, cx.t, cx.f, y, lines), action: None });
    y
}

/// The slots an image can be flashed to, each by its letter where the
/// slot the phone runs from is known.
fn pick_title(l: Lang, pick: Pick, current: Option<Slot>) -> String {
    match (pick, current) {
        (Pick::Current, Some(slot)) => texts::flash::current_slot(l, slot.letter()),
        (Pick::Current, None) => l.pick(texts::slot::CURRENT).into(),
        (Pick::Other, Some(slot)) => texts::flash::other_slot(l, slot.other().letter()),
        (Pick::Other, None) => l.pick(texts::flash::OTHER_SLOT_UNNAMED).into(),
        (Pick::Both, _) => l.pick(texts::flash::BOTH_SLOTS).into(),
    }
}

/// Installing a package step by step: receiving what the computer sends
/// with `adb sideload`, or reading the file chosen, then installing it.
pub fn installing(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let (mut out, mut rows) = (Vec::new(), None);
    let top = t.m.content_top;
    let computer = s.source == Source::Computer;
    let bottom = match &s.install {
        Install::Idle | Install::Starting | Install::Ending if computer => {
            let row = standing(l.pick(texts::installing::SWITCHING_USB), GREEN, Icon::Usb, l.pick(texts::installing::TO_SIDELOAD));
            widgets::list(&mut c, t, f, top, &[row]).bottom
        }
        Install::Idle | Install::Starting | Install::Ending if s.source == Source::Rescue => {
            let row = standing(l.pick(texts::installing::SWITCHING_USB), GREEN, Icon::Usb, l.pick(texts::installing::TO_RESCUE));
            widgets::list(&mut c, t, f, top, &[row]).bottom
        }
        Install::Idle | Install::Starting | Install::Ending => {
            widgets::list(&mut c, t, f, top, &[Row::new(l.pick(texts::installing::READING_FILE)).icon(GREEN, Icon::Install).trail(Trail::None)]).bottom
        }
        Install::Opening => {
            let opening = Row::new(l.pick(texts::installing::OPENING_DATA)).icon(GREEN, Icon::Lock).trail(Trail::None);
            let y = widgets::list(&mut c, t, f, top, &[opening]).bottom;
            asked_package(&mut c, cx, y)
        }
        Install::Locked(_) => {
            let row = standing(l.pick(texts::installing::CANT_READ_YET), ORANGE, Icon::Lock, l.pick(texts::decrypt::NEEDED));
            let y = widgets::list(&mut c, t, f, top, &[row]).bottom;
            let open = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::page::DECRYPT))]);
            out.extend(hits(&open.rows, &[Action::Open(Screen::Decrypt)]));
            let y = widgets::footnote(&mut c, t, f, open.bottom, l.pick(texts::installing::INSTALLS_ONCE_DECRYPTED));
            asked_package(&mut c, cx, y)
        }
        Install::Waiting if s.source == Source::Rescue => {
            let row = standing(l.pick(texts::installing::WAITING), GREEN, Icon::Usb, l.pick(texts::installing::CONNECT));
            let y = widgets::list(&mut c, t, f, top, &[row]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(texts::installing::RUN_ON_COMPUTER));
            let y = widgets::info_card(&mut c, t, f, y, &[
                (l.pick(texts::installing::RESCUE_INSTALL), l.pick(texts::installing::RESCUE_INSTALL_NOTE).into()),
                ("adb rescue getprop", l.pick(texts::installing::RESCUE_GETPROP_NOTE).into()),
                ("adb rescue wipe userdata", l.pick(texts::installing::RESCUE_WIPE_NOTE).into()),
            ]);
            widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::RESCUE_NOTE))
        }
        Install::Waiting => {
            let row = standing(l.pick(texts::installing::WAITING), GREEN, Icon::Usb, l.pick(texts::installing::CONNECT));
            let y = widgets::list(&mut c, t, f, top, &[row]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(texts::installing::RUN_ON_COMPUTER));
            let y = widgets::info_card(&mut c, t, f, y, &[(l.pick(texts::installing::SIDELOAD), l.pick(texts::installing::SIDELOAD_FILE_NOTE).into())]);
            widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::SIDELOAD_NOTE))
        }
        Install::Checking { stage: Some(Stage::Signature { done, total }), .. } => {
            let read = share(*done, *total);
            let detail = texts::installing::read(l, percent(read));
            let y = widgets::progress_card(&mut c, t, f, top, l.pick(texts::installing::VERIFYING_SIGNATURE), &detail, Some(read));
            let note = l.pick(match s.source {
                Source::Computer => texts::installing::CANCEL_ENDS_SIDELOAD,
                Source::Rescue => texts::installing::CANCEL_LEAVES_RESCUE,
                Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => texts::installing::CANCEL,
                Source::File(Place::UsbDrive) | Source::Opened { drive: true } => texts::installing::CANCEL_KEEP_USB_DRIVE,
            });
            widgets::footnote(&mut c, t, f, y, note)
        }
        Install::Checking { size, .. } => {
            let detail = texts::installing::file_size(l, fmt::bytes(*size));
            let y = widgets::list(&mut c, t, f, top, &[standing(l.pick(texts::installing::READING_FILE), GREEN, Icon::Install, &detail)]).bottom;
            match s.source {
                Source::Computer | Source::Rescue => widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::KEEP_CABLE)),
                Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => y,
                Source::File(Place::UsbDrive) | Source::Opened { drive: true } => widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::KEEP_USB_DRIVE)),
            }
        }
        Install::Choosing(choice) => {
            let (name, holds) = fmt::image_kind(l, choice.kind);
            let detail = texts::installing::size_and_holds(l, fmt::bytes(choice.size), &holds);
            let y = widgets::list(&mut c, t, f, top, &[standing(name, GREEN, Icon::Install, &detail)]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(texts::flash::PARTITION));
            let rows: Vec<Row> = choice
                .partitions
                .iter()
                .enumerate()
                .map(|(k, p)| Row::new(&p.name).subtitle(fmt::partition_role(l, p)).trail(Trail::Check(k == choice.partition)))
                .collect();
            let parts = widgets::list(&mut c, t, f, y, &rows);
            let choose: Vec<Action> = (0..choice.partitions.len()).map(Action::FlashPartition).collect();
            out.extend(hits(&parts.rows, &choose));
            // A partition the phone has in each slot goes to the slots
            // picked, and a logical one to the slot the phone runs from,
            // which is shown with nothing to pick; super is in no slot.
            let bottom = match choice.partitions[choice.partition].place {
                FlashPlace::Slotted => {
                    let y = widgets::section(&mut c, t, f, parts.bottom, l.pick(texts::slot::TITLE));
                    let titles = Pick::ALL.map(|p| pick_title(l, p, i.slot));
                    let rows: Vec<Row> =
                        Pick::ALL.iter().zip(&titles).map(|(p, title)| Row::new(title).trail(Trail::Check(*p == choice.pick))).collect();
                    let picks = widgets::list(&mut c, t, f, y, &rows);
                    out.extend(hits(&picks.rows, &Pick::ALL.map(Action::FlashSlots)));
                    picks.bottom
                }
                FlashPlace::Logical => {
                    let y = widgets::section(&mut c, t, f, parts.bottom, l.pick(texts::slot::TITLE));
                    let title = pick_title(l, Pick::Current, i.slot);
                    widgets::list(&mut c, t, f, y, &[Row::new(&title).trail(Trail::Check(true))]).bottom
                }
                FlashPlace::Single => parts.bottom,
            };
            let (label, danger) = confirm_label(Confirm::Flash, cx.state);
            let track = widgets::confirm(&mut c, t, f, bottom + t.m.dp(28.0), label, danger, s.confirm);
            out.push(Hit { rect: track, action: Action::Confirm(Confirm::Flash) });
            let note = l.pick(match s.source {
                Source::Computer => texts::flash::OVERWRITES_ENDS_SIDELOAD,
                Source::Rescue => texts::flash::OVERWRITES_LEAVES_RESCUE,
                Source::File(_) | Source::Opened { .. } | Source::Asked { .. } => texts::flash::OVERWRITES,
            });
            widgets::footnote(&mut c, t, f, track.y + track.h, note)
        }
        Install::Confirming { version, wipe } => {
            use texts::installing::confirm;
            let (title, color, icon, detail, which) = if *wipe {
                let detail = match (version, s.cache_partition) {
                    (Some(v), true) => confirm::version_resets_with_cache(l, v),
                    (Some(v), false) => confirm::version_resets(l, v),
                    (None, true) => l.pick(confirm::UPDATE_RESETS_WITH_CACHE).into(),
                    (None, false) => l.pick(confirm::UPDATE_RESETS).into(),
                };
                (l.pick(confirm::WIPE_NEEDED).to_string(), ORANGE, Icon::Wipe, detail, Confirm::InstallWipe)
            } else {
                let title = match version {
                    Some(v) => confirm::version(l, v),
                    None => l.pick(confirm::UPDATE).into(),
                };
                let detail = match i.slot.map(|s| s.other().letter()) {
                    Some(slot) => confirm::to_slot(l, slot),
                    None => l.pick(confirm::STARTS_AFTER_REBOOT).into(),
                };
                (title, GREEN, Icon::Install, detail, Confirm::Install)
            };
            let y = widgets::list(&mut c, t, f, top, &[standing(&title, color, icon, &detail)]).bottom;
            let (label, danger) = confirm_label(which, cx.state);
            let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
            out.push(Hit { rect: track, action: Action::Confirm(which) });
            widgets::footnote(&mut c, t, f, track.y + track.h, l.pick(texts::installing::CANCEL_WRITES_NOTHING))
        }
        Install::Updating { version, stage, overwriting } => {
            let title = match version {
                Some(v) => texts::installing::named(l, v),
                None => l.pick(texts::installing::UPDATE).into(),
            };
            let (detail, done) = update_stage(l, stage.as_ref());
            let y = widgets::progress_card(&mut c, t, f, top, &title, &detail, done);
            let until = until_installed(l, s.source);
            let note = match i.slot.map(|s| s.other().letter()) {
                Some(slot) => texts::installing::until_and_slot(l, until, slot),
                None => until.to_string(),
            };
            let y = widgets::footnote(&mut c, t, f, y, &note);
            if *overwriting {
                widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::OVERWRITING))
            } else {
                y
            }
        }
        Install::Flashing { partition, stage } => {
            let (detail, done) = match stage {
                Some(Stage::Flashing { done, total }) => {
                    let s = share(*done, *total);
                    let detail = match done * 2 < *total {
                        true => texts::flash::writing(l, percent(s)),
                        false => texts::flash::verifying(l, percent(s)),
                    };
                    (detail, Some(s))
                }
                _ => (l.pick(texts::installing::GETTING_READY).into(), None),
            };
            let y = widgets::progress_card(&mut c, t, f, top, &texts::flash::flashing(l, partition), &detail, done);
            let keep = l.pick(match s.source {
                Source::Computer | Source::Rescue => texts::flash::KEEP_CABLE,
                Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => texts::flash::KEEP_POWER,
                Source::File(Place::UsbDrive) | Source::Opened { drive: true } => texts::flash::KEEP_USB_DRIVE,
            });
            widgets::footnote(&mut c, t, f, y, keep)
        }
        Install::SystemChoosing(choice) => {
            let detail = texts::installing::as_second_system(l, &choice.version);
            let y = widgets::list(&mut c, t, f, top, &[standing(&choice.name, PURPLE, Icon::Systems, &detail)]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(texts::installing::ROOM));
            let sizes: Vec<String> = choice.sizes.iter().map(|&b| fmt::bytes(b)).collect();
            let least = l.pick(texts::installing::LEAST_ROOM);
            let rows: Vec<Row> = sizes
                .iter()
                .enumerate()
                .map(|(k, size)| {
                    let row = Row::new(size).trail(Trail::Check(k == choice.size));
                    if k == 0 { row.subtitle(least) } else { row }
                })
                .collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            let picks: Vec<Action> = (0..choice.sizes.len()).map(Action::SystemSize).collect();
            out.extend(hits(&list.rows, &picks));
            let y = widgets::footnote(&mut c, t, f, list.bottom, l.pick(texts::installing::ROOM_NOTE));
            let (label, danger) = confirm_label(Confirm::InstallSystem, cx.state);
            let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
            out.push(Hit { rect: track, action: Action::Confirm(Confirm::InstallSystem) });
            widgets::footnote(&mut c, t, f, track.y + track.h, l.pick(texts::installing::CANCEL_WRITES_NOTHING))
        }
        Install::Systeming { name, stage } => {
            let (detail, done) = system_stage(l, stage.as_ref());
            let y = widgets::progress_card(&mut c, t, f, top, &texts::installing::named(l, name), &detail, done);
            widgets::footnote(&mut c, t, f, y, until_installed(l, s.source))
        }
        Install::ConfirmingScript => {
            let name = s.package_name.as_deref().unwrap_or(l.pick(texts::installing::FLASHABLE_ZIP));
            let detail = l.pick(texts::installing::SCRIPT_WARNING);
            let mut y = widgets::list(&mut c, t, f, top, &[standing(name, ORANGE, Icon::Install, detail)]).bottom;
            if let Some(note) = fmt::snapshot_script_note(l, &i.snapshot) {
                y = widgets::footnote(&mut c, t, f, y, note);
            }
            let (label, danger) = confirm_label(Confirm::RunScript, cx.state);
            let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
            out.push(Hit { rect: track, action: Action::Confirm(Confirm::RunScript) });
            widgets::footnote(&mut c, t, f, track.y + track.h, l.pick(texts::installing::CANCEL_RUNS_NOTHING))
        }
        Install::Scripting { stage, .. } => {
            let (detail, done) = script_stage(l, stage.as_ref(), &s.script_output);
            let y = widgets::progress_card(&mut c, t, f, top, l.pick(texts::installing::SCRIPT), detail, done);
            let y = widgets::footnote(&mut c, t, f, y, until_installed(l, s.source));
            script_output(&mut c, cx, y, &mut rows)
        }
        Install::Wiping => {
            let y = widgets::progress_card(&mut c, t, f, top, l.pick(texts::wipe::WIPING), factory_reset(s), None);
            widgets::footnote(&mut c, t, f, y, l.pick(texts::wipe::KEEP_POWER))
        }
        Install::Wiped(Ok(())) => {
            let made_anew = l.pick(texts::installing::DATA_MADE_ANEW);
            let y = widgets::list(&mut c, t, f, top, &[standing(l.pick(texts::wipe::WIPED), GREEN, Icon::Wipe, made_anew)]).bottom;
            widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::MORE_COMMANDS))
        }
        Install::Wiped(Err(stop)) => {
            let (what, detail) = fmt::erase_stop(l, stop);
            let mut y = widgets::list(&mut c, t, f, top, &[standing(l.pick(texts::wipe::INCOMPLETE), RED, Icon::Wipe, &what)]).bottom;
            if let Some(d) = detail {
                y = widgets::footnote(&mut c, t, f, y, &d);
            }
            widgets::footnote(&mut c, t, f, y, l.pick(texts::installing::MORE_COMMANDS))
        }
        Install::Finished { outcome, damage } => finished(&mut c, cx, top, outcome, damage.as_ref(), &mut out, &mut rows),
    };
    // What the system asked for says why, where it said.
    let bottom = match (s.source, &s.asked.reason) {
        (Source::Asked { .. }, Some(reason)) => widgets::info_card(&mut c, t, f, bottom + t.m.dp(12.0), &[(l.pick(texts::installing::REASON), reason.clone())]),
        _ => bottom,
    };
    Frame { rows, ..Frame::new(t, fit(c, t, bottom), out, install_title(l, s.source), top) }
}

/// The package the system asked for, by the path it gave, at `y`.
fn asked_package(c: &mut Canvas, cx: &Ctx, y: f32) -> f32 {
    let name = cx.state.package_name.clone().unwrap_or_default();
    widgets::info_card(c, cx.t, cx.f, y + cx.t.m.dp(12.0), &[(cx.state.lang.pick(texts::installing::PACKAGE), name)])
}

/// What not to do while a package from `source` is installed.
fn until_installed(l: Lang, source: Source) -> &'static str {
    l.pick(match source {
        Source::Computer | Source::Rescue => texts::installing::UNTIL_KEEP_CABLE,
        Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => texts::installing::UNTIL_KEEP_POWER,
        Source::File(Place::UsbDrive) | Source::Opened { drive: true } => texts::installing::UNTIL_KEEP_USB_DRIVE,
    })
}

/// How an install ended, and where to go from there: another package from
/// the computer, or back to choose another file.
fn finished(
    c: &mut Canvas,
    cx: &Ctx,
    top: f32,
    outcome: &Outcome,
    damage: Option<&Damage>,
    out: &mut Vec<Hit>,
    rows: &mut Option<Rows>,
) -> f32 {
    let (t, f, source, l) = (cx.t, cx.f, cx.state.source, cx.state.lang);
    let gap = t.m.card_gap;
    use texts::installed;
    let choose_another = installed::CHOOSE_ANOTHER;
    let (again, another) = match source {
        Source::Computer => (Row::new(l.pick(installed::SEND_AGAIN)).icon(GREEN, Icon::Usb), Action::Sideload),
        Source::File(_) | Source::Opened { .. } => (Row::new(l.pick(choose_another)).icon(GREEN, Icon::Folder), Action::Back),
        Source::Asked { .. } => (Row::new(l.pick(choose_another)).icon(GREEN, Icon::Folder), Action::Open(Screen::Install)),
        Source::Rescue => (Row::new(l.pick(installed::LEAVE_RESCUE)).icon(GREEN, Icon::Usb), Action::Back),
    };
    let restart = Row::new(l.pick(installed::REBOOT_TO_SYSTEM)).icon(ORANGE, Icon::Power);
    let done = l.pick(installed::INSTALLED);
    let nothing_written = l.pick(installed::NOTHING_WRITTEN);
    match outcome {
        Outcome::Installed { slot, .. } => {
            let detail = installed::to_slot(l, slot.letter());
            let y = widgets::list(c, t, f, top, &[standing(done, GREEN, Icon::Install, &detail)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[restart]);
            out.extend(hits(&next.rows, &[Action::RebootTo(Target::System)]));
            let y = reflashed(c, cx, next.bottom);
            afterwards_of(c, cx, y, true)
        }
        Outcome::Flashed { partition, slots } => {
            let detail = match slots.as_slice() {
                [] => installed::flashed_whole(l, partition),
                [slot] => installed::flashed_one(l, partition, slot.letter()),
                [a, b] => installed::flashed_two(l, partition, a.letter(), b.letter()),
                other => unreachable!("a phone has two slots, and the image went to {other:?}"),
            };
            let y = widgets::list(c, t, f, top, &[standing(l.pick(installed::FLASHED), GREEN, Icon::Install, &detail)]).bottom;
            let mut rows = vec![restart];
            let mut acts = vec![Action::RebootTo(Target::System)];
            if *partition == "recovery" {
                rows.push(Row::new(l.pick(installed::REBOOT_TO_RECOVERY)).icon(ORANGE, Icon::Power));
                acts.push(Action::RebootTo(Target::Recovery));
            }
            rows.push(again);
            acts.push(another);
            let next = widgets::list(c, t, f, y + gap, &rows);
            out.extend(hits(&next.rows, &acts));
            afterwards_of(c, cx, next.bottom, true)
        }
        Outcome::Ran { .. } => {
            let ran = l.pick(installed::SCRIPT_RAN);
            let y = widgets::list(c, t, f, top, &[standing(done, GREEN, Icon::Install, ran)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[restart, again]);
            out.extend(hits(&next.rows, &[Action::RebootTo(Target::System), another]));
            let y = reflashed(c, cx, next.bottom);
            let y = afterwards_of(c, cx, y, true);
            script_output(c, cx, y, rows)
        }
        Outcome::SystemInstalled { name } => {
            let detail = installed::second_system(l, name);
            let y = widgets::list(c, t, f, top, &[standing(done, GREEN, Icon::Systems, &detail)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[Row::new(l.pick(texts::page::DUAL_BOOT)).icon(PURPLE, Icon::Systems)]);
            out.extend(hits(&next.rows, &[Action::Open(Screen::Systems)]));
            afterwards_of(c, cx, next.bottom, true)
        }
        Outcome::Cancelled => {
            let y = widgets::list(c, t, f, top, &[standing(l.pick(installed::CANCELLED), SLATE, Icon::Install, nothing_written)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[again]);
            out.extend(hits(&next.rows, &[another]));
            next.bottom
        }
        Outcome::Refused(r) => {
            let reason = fmt::refusal(l, r, &cx.info.codename, source, cx.state.zone());
            let y = widgets::list(c, t, f, top, &[standing(l.pick(installed::CANT_INSTALL), ORANGE, Icon::Install, &reason)]).bottom;
            // What the system asked for and did not go in leaves the phone
            // where it was, from which it may go back to the system.
            let (rows, acts) = match source {
                Source::Asked { .. } => (vec![again, restart], vec![another, Action::RebootTo(Target::System)]),
                _ => (vec![again], vec![another]),
            };
            let next = widgets::list(c, t, f, y + gap, &rows);
            out.extend(hits(&next.rows, &acts));
            let y = widgets::footnote(c, t, f, next.bottom, nothing_written);
            afterwards_of(c, cx, y, false)
        }
        Outcome::Failed(failure) => {
            let (what, detail) = fmt::failure(l, failure, source);
            let y = widgets::list(c, t, f, top, &[standing(l.pick(installed::FAILED), RED, Icon::Install, &what)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[again, restart]);
            out.extend(hits(&next.rows, &[another, Action::RebootTo(Target::System)]));
            // The package is sent again, or installed or flashed again.
            let sent = matches!(source, Source::Computer | Source::Rescue);
            let warning = match (damage, sent) {
                (Some(Damage::System), true) => l.pick(installed::damage::SYSTEM_SEND).into(),
                (Some(Damage::System), false) => l.pick(installed::damage::SYSTEM_INSTALL).into(),
                (Some(Damage::Partition(p)), true) => installed::damage::partition_send(l, p),
                (Some(Damage::Partition(p)), false) => installed::damage::partition_flash(l, p),
                (Some(Damage::Script), true) => l.pick(installed::damage::SCRIPT_SEND).into(),
                (Some(Damage::Script), false) => l.pick(installed::damage::SCRIPT_INSTALL).into(),
                (None, _) => l.pick(installed::damage::NONE).to_string(),
            };
            let mut y = widgets::footnote(c, t, f, next.bottom, &warning);
            if let Some(d) = detail {
                y = widgets::footnote(c, t, f, y, &d);
            }
            let y = reflashed(c, cx, y);
            let y = afterwards_of(c, cx, y, false);
            script_output(c, cx, y, rows)
        }
    }
}

/// What comes after an install, where it is not the user's to choose:
/// rescue waits for the computer's next command, and a package the system
/// asked for, once in, has the phone leave on its own.
fn afterwards_of(c: &mut Canvas, cx: &Ctx, y: f32, installed: bool) -> f32 {
    let l = cx.state.lang;
    let leaving = match cx.state.asked.shutdown_after {
        true => texts::installed::leaving::POWER_OFF,
        false => texts::installed::leaving::REBOOT,
    };
    match cx.state.source {
        Source::Rescue => widgets::footnote(c, cx.t, cx.f, y, l.pick(texts::installing::MORE_COMMANDS)),
        Source::Asked { .. } if installed => widgets::footnote(c, cx.t, cx.f, y, l.pick(leaving)),
        _ => y,
    }
}

/// What the wipe page lists of [`WIPES`] beyond everything, under the
/// partition each is in.
const WIPE_GROUPS: [(Text, &[Wipe]); 2] = [
    (texts::wipe::part::DATA, &[Wipe::AppData, Wipe::Media, Wipe::AppCaches, Wipe::SystemCaches, Wipe::Art, Wipe::Logs, Wipe::Updates]),
    (texts::wipe::part::CACHE, &[Wipe::Cache]),
];

/// What a factory reset erases, as the wipe page tells it of its first
/// choice, and the page that carries one out for the system tells it.
fn factory_reset(s: &State) -> &'static str {
    s.lang.pick(match s.cache_partition {
        true => texts::wipe::factory_reset::WITH_CACHE,
        false => texts::wipe::factory_reset::WITHOUT_CACHE,
    })
}

/// A choice of the wipe page as it lists it: its title, and exactly what it
/// erases.
fn wipe_row(w: Wipe, s: &State) -> (&'static str, String) {
    use texts::wipe::{part, what};
    let l = s.lang;
    let (title, what): (Text, String) = match w {
        Wipe::All => (texts::wipe::ALL, factory_reset(s).into()),
        Wipe::AppData => (part::APP_DATA, l.pick(what::APP_DATA).into()),
        Wipe::Media => (part::MEDIA, l.pick(what::MEDIA).into()),
        Wipe::AppCaches => (part::APP_CACHES, l.pick(what::APP_CACHES).into()),
        Wipe::SystemCaches => (part::SYSTEM_CACHES, l.pick(what::SYSTEM_CACHES).into()),
        Wipe::Art => (part::ART, l.pick(what::ART).into()),
        Wipe::Logs => match s.phone_logs {
            0 => (part::LOGS, l.pick(what::LOGS).into()),
            n => (part::LOGS, what::logs_and_own(l, n as u64)),
        },
        Wipe::Updates => (part::UPDATES, l.pick(what::UPDATES).into()),
        Wipe::Cache => (texts::wipe::EVERYTHING, l.pick(what::CACHE).into()),
    };
    (l.pick(title), what)
}

/// What to erase: everything, as a factory reset does, or parts of it on
/// their own, the cache partition among them where the phone has one; then
/// a swipe erases what is chosen.
pub fn wipe(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let chosen = |w: Wipe| s.wipe[WIPES.iter().position(|&x| x == w).expect("every choice is one of WIPES")];
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::wipe::RESET));
    let (name, sub) = wipe_row(Wipe::All, s);
    let all = widgets::list(&mut c, t, f, y, &[Row::new(name).subtitle(&sub).trail(Trail::Check(chosen(Wipe::All)))]);
    out.extend(hits(&all.rows, &[Action::WipeItem(Wipe::All)]));
    let mut y = widgets::footnote(&mut c, t, f, all.bottom, l.pick(texts::wipe::STAYS));
    for (title, items) in WIPE_GROUPS {
        let shown: Vec<Wipe> = items.iter().copied().filter(|&w| w != Wipe::Cache || s.cache_partition).collect();
        if shown.is_empty() {
            continue;
        }
        let below = widgets::section(&mut c, t, f, y, l.pick(title));
        let lines: Vec<(&str, String)> = shown.iter().map(|&w| wipe_row(w, s)).collect();
        let rows: Vec<Row> =
            shown.iter().zip(&lines).map(|(&w, (name, sub))| Row::new(name).subtitle(sub).trail(Trail::Check(chosen(w)))).collect();
        let list = widgets::list(&mut c, t, f, below, &rows);
        out.extend(hits(&list.rows, &shown.iter().map(|&w| Action::WipeItem(w)).collect::<Vec<_>>()));
        y = list.bottom;
    }
    let y = widgets::footnote(&mut c, t, f, y, l.pick(texts::wipe::NOTE));
    let (label, danger) = confirm_label(Confirm::Wipe, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::Wipe) });
    Frame::new(t, fit(c, t, track.y + track.h), out, l.pick(texts::page::WIPE), top)
}

/// 键盘的顶边：键盘照旧贴着屏幕底。
fn kb_top(t: &Theme) -> f32 {
    t.m.h as f32 - widgets::keyboard_height(t)
}

/// 页面内容能用到的最低一行：键盘的顶边再让出拼音条那一条。
fn text_floor(t: &Theme) -> f32 {
    kb_top(t) - crate::ime::height(t)
}

/// A page laid out above the on-screen keyboard: where the keyboard's top
/// is, and its keys as touch targets.
fn keyboard_hits(c: &mut Canvas, t: &Theme, f: &Fonts, s: &State, enter: &str, out: &mut Vec<Hit>) -> f32 {
    let top = kb_top(t);
    out.extend(crate::ime::bar(c, t, f, &s.ime, text_floor(t)));
    let keys = widgets::keyboard(c, t, f, s.lang, top, s.layer, s.key_held(), enter, s.ime.enabled);
    out.extend(keys.into_iter().map(|(rect, cap)| Hit { rect, action: Action::Type(cap) }));
    top
}

/// Finding a feature or partition: the field where the title would be, the
/// button that leaves the search beside it, what it finds, and the
/// keyboard.
pub fn search(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();
    let (at, cancel) = widgets::search_layout(t);
    // 进来找什么，框里就提示什么：找模块、找应用，还是找功能。
    let hint = match s.search_scope {
        SearchScope::Files => l.pick(["搜索文件", "搜尋檔案", "Search files"]),
        SearchScope::Apps => l.pick(["搜索应用", "搜尋應用程式", "Search apps"]),
        SearchScope::Modules => l.pick(["搜索模块", "搜尋模組", "Search modules"]),
        SearchScope::Features => l.pick(SEARCH_HINT),
    };
    let field = widgets::search_field(&mut c, t, f, at.x, at.w, at.y + at.h / 2.0, &s.search, hint);
    widgets::cancel_button(&mut c, t, cancel);
    out.push(Hit { rect: cancel, action: Action::CloseSearch });
    let top = field.field.y + field.field.h + m.dp(12.0);
    let kb_edge = text_floor(&t);
    // 找什么由进来的地方说：整机的功能，或者某一页里的应用、模块。
    let query = s.search.trim().to_lowercase();
    // 空表放在外面：行借的是它们里面的字符串，得比 rows 活得久。
    let no_apps: Vec<crate::app::dataapp::DataApp> = Vec::new();
    let mut nothing_found = String::new();
    let no_modules: Vec<crate::app::ModuleRow> = Vec::new();
    let mut rows: Vec<Row> = Vec::new();
    let mut acts: Vec<Action> = Vec::new();
    match s.search_scope {
        SearchScope::Apps => {
            let apps = match &s.apps {
                Some(Ok(list)) => list,
                _ => &no_apps,
            };
            for (k, a) in apps.iter().enumerate() {
                if !query.is_empty() && !(a.label.to_lowercase().contains(&query) || a.package.to_lowercase().contains(&query)) {
                    continue;
                }
                let mut row = Row::new(&a.label).icon(PURPLE, Icon::Layers);
                row.sub = Some(&a.package);
                rows.push(row);
                acts.push(Action::DataAppOpen(k));
            }
        }
        SearchScope::Modules => {
            let mods = match &s.modules {
                Some(Ok(list)) => list,
                _ => &no_modules,
            };
            for (k, m) in mods.iter().enumerate() {
                if !query.is_empty() && !(m.name.to_lowercase().contains(&query) || m.id.to_lowercase().contains(&query)) {
                    continue;
                }
                rows.push(Row::new(&m.name).icon(BLUE, Icon::Layers).trail(Trail::Switch(m.enabled)));
                acts.push(Action::ModuleOpen(k));
            }
        }
        SearchScope::Files => {
            // 当前文件夹里的名字过一遍：行点开就是那一项，用它在这一页里的
            // 第几行，页面照旧打开它（2026-11-23）。
            if let Ok(list) = &s.folder(Screen::Files).entries {
                for (k, e) in list.iter().enumerate() {
                    if !query.is_empty() && !e.name.to_lowercase().contains(&query) {
                        continue;
                    }
                    let mut row = Row::new(&e.name);
                    if matches!(e.kind, FileKind::Folder { .. }) {
                        row = row.icon(PURPLE, Icon::Folder);
                    }
                    rows.push(row);
                    acts.push(Action::OpenEntry(k));
                }
            }
        }
        SearchScope::Features => {
            if query.is_empty() {
                let how = l.pick([
                    "输入功能或分区名称，支持拼音全拼和首字母，例如输入 bf 可找到「备份」。",
                    "輸入功能或分割區名稱，支援英文和漢語拼音的全拼與首字母，例如輸入 bf 可找到「備份」。",
                    "Type the name of a feature or partition, such as backup to find Backup.",
                ]);
                widgets::footnote(&mut c, t, f, top, how);
            } else {
                let found = search::find_on(&s.search, l, &s.entry_names, |screen| s.offers(screen));
                if found.is_empty() {
                    let typed = s.search.trim();
                    nothing_found = tr!(l, "未找到\"{typed}\"", "找不到「{typed}」", "Nothing found for “{typed}”");
                    rows.push(Row::new(&nothing_found).trail(Trail::None));
                } else {
                    for e in &found {
                        // 主人给首页那一条改过名字的，就按新名字列出来（2026-11-27）。
                        let named = search::home_name(e.screen, &s.entry_names).unwrap_or_else(|| l.pick(e.title));
                        rows.push(Row::new(named).subtitle(l.pick(e.place)));
                        acts.push(Action::Open(e.screen));
                    }
                }
            }
        }
    }
    // 没输入时先说一句这页有多少东西可找。
    if query.is_empty() && s.search_scope != SearchScope::Features {
        let how = match (s.search_scope, &s.apps, &s.modules) {
            (SearchScope::Apps, Some(Ok(list)), _) => {
                let n = list.len();
                tr!(l, "共 {n} 个应用，输入名称或包名筛选", "共 {n} 個應用程式，輸入名稱或包名篩選", "{n} apps; type a name or package")
            }
            (SearchScope::Modules, _, Some(Ok(list))) => {
                let n = list.len();
                tr!(l, "共 {n} 个模块，输入名称或目录名筛选", "共 {n} 個模組，輸入名稱或目錄名篩選", "{n} modules; type a name or folder")
            }
            (SearchScope::Files, _, _) => l.pick(["在这个文件夹里找", "在這個資料夾裡找", "Find a name in this folder"]).to_string(),
            _ => l.pick(["输入名称筛选", "輸入名稱篩選", "Type a name to narrow it"]).to_string(),
        };
        widgets::footnote(&mut c, t, f, top, &how);
    }
    if !rows.is_empty() {
        let list = widgets::list(&mut c, t, f, top, &rows);
        // 键盘盖住的地方不接点击。
        for (&rect, action) in list.rows.iter().zip(&acts) {
            if rect.y + rect.h <= kb_edge {
                out.push(Hit { rect, action: action.clone() });
            }
        }
    }
    let find = l.pick(texts::search::SEARCH);
    keyboard_hits(&mut c, t, f, s, find, &mut out);
    if let Some(rect) = field.clear {
        out.push(Hit { rect, action: Action::ClearSearch });
    }
    Frame { back: false, header: false, caret: Some(field.caret), ..Frame::new(t, c, out, find, 0.0) }
}

/// Backing up: what to take, where it goes, and how.
pub fn backup(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::backup::CONTENTS));
    let system = fmt::snapshot_backup_note(l, &i.snapshot).unwrap_or(l.pick(texts::backup::SYSTEM_HOLDS));
    let (data, media) = match s.data == Data::Open {
        true => (texts::backup::DATA_HOLDS, texts::backup::MEDIA_HOLDS),
        false => (texts::backup::DATA_HOLDS_LOCKED, texts::backup::MEDIA_HOLDS_LOCKED),
    };
    let boot = l.pick(texts::backup::BOOT_HOLDS);
    let modem = l.pick(texts::backup::MODEM_HOLDS);
    let subs = [boot, system, l.pick(data), l.pick(media), modem];
    let rows: Vec<Row> = BACKUP_PARTS
        .iter()
        .zip(subs)
        .enumerate()
        .map(|(k, (name, sub))| Row::new(l.pick(*name)).subtitle(sub).trail(Trail::Check(s.backup[k])))
        .collect();
    let parts = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&parts.rows, &[0, 1, 2, 3, 4].map(Action::BackupPart)));
    let y = widgets::section(&mut c, t, f, parts.bottom, l.pick(texts::backup::SAVE_TO));
    let storage = match (s.data.storage_open(), i.storage.free.map(fmt::gb)) {
        (true, Some(free)) => texts::backup::free(l, free),
        (true, None) => l.pick(texts::backup::FREE_UNKNOWN).to_string(),
        (false, _) if s.data == Data::Erased => l.pick(texts::data::ERASED).to_string(),
        (false, _) => l.pick(texts::decrypt::NEEDED).to_string(),
    };
    let usb = l.pick(match &s.usb_drive {
        Some(UsbDrive::Shown) => texts::backup::CONNECTED,
        Some(UsbDrive::Unreadable(_)) => texts::usb_drive::UNREADABLE,
        None => texts::usb_drive::WANTED,
    });
    let places = widgets::list(&mut c, t, f, y, &[
        Row::new(place_name(l, Place::Storage)).subtitle(&storage).trail(Trail::Check(s.backup_to == Place::Storage)),
        Row::new(place_name(l, Place::UsbDrive)).subtitle(usb).trail(Trail::Check(s.backup_to == Place::UsbDrive)),
    ]);
    out.extend(hits(&places.rows, &[Action::BackupTo(Place::Storage), Action::BackupTo(Place::UsbDrive)]));
    let y = widgets::section(&mut c, t, f, places.bottom, l.pick(texts::common::OPTIONS));
    let options = widgets::list(&mut c, t, f, y, &[
        Row::new(l.pick(texts::backup::COMPRESS)).subtitle(l.pick(texts::backup::COMPRESS_NOTE)).trail(Trail::Switch(s.compress)),
        Row::new(l.pick(texts::backup::VERIFY)).subtitle(l.pick(texts::backup::VERIFY_NOTE)).trail(Trail::Switch(s.verify)),
    ]);
    out.extend(hits(&options.rows, &[Action::Toggle(Switch::Compress), Action::Toggle(Switch::Verify)]));
    let y = widgets::footnote(&mut c, t, f, options.bottom, l.pick(texts::backup::NOTE));
    let (label, danger) = confirm_label(Confirm::Backup, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::Backup) });
    Frame::new(t, fit(c, t, track.y + track.h), out, l.pick(texts::page::BACKUP), top)
}

/// The backups found, newest first, to restore one; those that cannot be
/// read are listed after them, to be deleted.
pub fn restore(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::restore::BACKUPS));
    let bottom = if s.backups.is_empty() {
        widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::restore::NONE)).trail(Trail::None)]).bottom
    } else {
        let lines: Vec<(String, String)> = s
            .backups
            .iter()
            .map(|b| match &b.held {
                Ok(h) => (fmt::datetime(l, h.made, s.zone()), format!("{} · {} · {}", fmt::parts(l, &h.parts), fmt::bytes(h.size), place_name(l, b.place))),
                Err(_) => (b.name.clone(), format!("{} · {}", l.pick(texts::common::UNREADABLE), place_name(l, b.place))),
            })
            .collect();
        let rows: Vec<Row> = lines.iter().map(|(title, sub)| Row::new(title).subtitle(sub)).collect();
        let list = widgets::list(&mut c, t, f, y, &rows);
        out.extend(list.rows.iter().enumerate().map(|(k, &rect)| Hit { rect, action: Action::OpenBackup(k) }));
        list.bottom
    };
    let bottom = widgets::footnote(&mut c, t, f, bottom, l.pick(texts::restore::NOTE));
    Frame::new(t, fit(c, t, bottom), out, l.pick(texts::page::RESTORE), top)
}

/// One backup: when and where it was made, and which of its parts to
/// restore. One made on another phone is not restored here, and one that
/// cannot be read can only be deleted.
pub fn restore_backup(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let (entry, chosen) = s.restore.as_ref().expect("the restore page shows a backup");
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let location = l.pick(texts::common::LOCATION);
    let y = match &entry.held {
        Err(why) => {
            let facts = [(l.pick(texts::restore::FOLDER), entry.name.clone()), (location, place_name(l, entry.place).to_string())];
            let y = widgets::info_card(&mut c, t, f, top, &facts);
            widgets::footnote(&mut c, t, f, y, &texts::restore::unreadable(l, why))
        }
        Ok(held) => {
            let mut facts = vec![
                (l.pick(texts::restore::MADE), fmt::datetime(l, held.made, s.zone())),
                (l.pick(texts::common::SIZE), fmt::bytes(held.size)),
                (location, place_name(l, entry.place).to_string()),
            ];
            if let Some(system) = &held.system {
                facts.push((l.pick(texts::restore::SYSTEM_VERSION), system.clone()));
            }
            if let Some(slot) = held.slot {
                facts.push((l.pick(texts::slot::TITLE), slot.letter().to_string()));
            }
            let y = widgets::info_card(&mut c, t, f, top, &facts);
            if held.device != i.codename {
                widgets::footnote(&mut c, t, f, y, &texts::backup::refused::other_phone(l, &held.device))
            } else {
                let y = widgets::section(&mut c, t, f, y, l.pick(texts::restore::CONTENTS));
                let kept: Vec<usize> = (0..BACKUP_PARTS.len()).filter(|&k| held.parts[k]).collect();
                let rows: Vec<Row> = kept.iter().map(|&k| Row::new(l.pick(BACKUP_PARTS[k])).trail(Trail::Check(chosen[k]))).collect();
                let list = widgets::list(&mut c, t, f, y, &rows);
                out.extend(list.rows.iter().zip(&kept).map(|(&rect, &k)| Hit { rect, action: Action::RestorePart(k) }));
                let mut y = widgets::footnote(&mut c, t, f, list.bottom, l.pick(texts::restore::OVERWRITES));
                if chosen[DATA_PART] {
                    y = widgets::footnote(&mut c, t, f, y, l.pick(texts::restore::DATA_NOTE));
                }
                if chosen[MEDIA_PART] {
                    y = widgets::footnote(&mut c, t, f, y, l.pick(texts::restore::MEDIA_NOTE));
                }
                // The boot partitions' vbmeta checks the system's: one of
                // another version without the other may not start.
                if let (Some(was), Some(now)) = (&held.system, &i.os_version)
                    && was != now
                    && chosen[0] != chosen[1]
                {
                    y = widgets::footnote(&mut c, t, f, y, &texts::restore::other_version(l, was, now));
                }
                let (label, danger) = confirm_label(Confirm::Restore, cx.state);
                let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
                out.push(Hit { rect: track, action: Action::Confirm(Confirm::Restore) });
                track.y + track.h
            }
        }
    };
    let delete = widgets::list(&mut c, t, f, y + t.m.dp(28.0), &[Row::new(l.pick(texts::restore::DELETE)).tint(t.pal.danger).trail(Trail::None)]);
    out.extend(hits(&delete.rows, &[Action::AskDelete]));
    let mut bottom = delete.bottom;
    if s.asking_delete {
        let y = widgets::footnote(&mut c, t, f, delete.bottom, l.pick(texts::restore::ASK_DELETE));
        let (label, danger) = confirm_label(Confirm::DeleteBackup, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
        out.push(Hit { rect: track, action: Action::Confirm(Confirm::DeleteBackup) });
        bottom = track.y + track.h;
    }
    Frame::new(t, fit(c, t, bottom), out, l.pick(texts::restore::TITLE), top)
}

/// The share of its pass the part under way has done.
fn part_share(a: &BackupAt) -> f32 {
    if a.of == 0 {
        1.0
    } else {
        share(a.done, a.of).min(1.0)
    }
}

/// How far backing up or restoring has come over all, 0 to 1, and the
/// bytes of the parts the pass under way has gone through, the part under
/// way by its share. Each part counts by its bytes in each pass: a restore
/// checks all its parts first, and a backup checks them last.
fn job_progress(job: &BackupJob, a: &BackupAt) -> (f32, u64) {
    let total: u64 = job.plan.iter().map(|(_, n)| n).sum();
    let before: u64 = job.plan[..a.item].iter().map(|(_, n)| n).sum();
    let passed = before as f64 + f64::from(part_share(a)) * job.plan[a.item].1 as f64;
    let second = job.checks && a.checking != job.restore;
    let passes = if job.checks { 2.0 } else { 1.0 };
    let over = (if second { total as f64 } else { 0.0 } + passed) / (passes * total.max(1) as f64);
    (over as f32, passed as u64)
}

/// What a backup or a restore does to a part as it goes through them:
/// takes it, puts it back, or checks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pass {
    BackUp,
    Restore,
    Verify,
}

impl Pass {
    // The pass that writes: a restore's, or a backup's.
    fn writing(restore: bool) -> Pass {
        if restore { Pass::Restore } else { Pass::BackUp }
    }

    // The pass a job is on at `a`.
    fn at(job: &BackupJob, a: &BackupAt) -> Pass {
        if a.checking { Pass::Verify } else { Pass::writing(job.restore) }
    }

    // Under way: 正在备份, Backing up.
    fn doing(self) -> Text {
        match self {
            Pass::BackUp => texts::backup::job::BACKING_UP,
            Pass::Restore => texts::backup::job::RESTORING,
            Pass::Verify => texts::backup::job::VERIFYING,
        }
    }

    // Done to a part: 已备份, Backed up.
    fn done(self) -> Text {
        match self {
            Pass::BackUp => texts::backup::job::BACKED_UP,
            Pass::Restore => texts::backup::job::RESTORED,
            Pass::Verify => texts::backup::job::VERIFIED,
        }
    }

    // Under way on the part `name`: 正在备份 boot.
    fn doing_part(self, l: Lang, name: &str) -> String {
        match self {
            Pass::BackUp => texts::backup::job::backing_up_part(l, name),
            Pass::Restore => texts::backup::job::restoring_part(l, name),
            Pass::Verify => texts::backup::job::verifying_part(l, name),
        }
    }

    // Under way on a part, `p` hundredths of it done: 正在备份 37%.
    fn doing_share(self, l: Lang, p: u32) -> String {
        match self {
            Pass::BackUp => texts::backup::job::backing_up_share(l, p),
            Pass::Restore => texts::backup::job::restoring_share(l, p),
            Pass::Verify => texts::backup::job::verifying_share(l, p),
        }
    }

    // How much of the whole is done: 已备份 1.2GB，共 5.0GB.
    fn done_of(self, l: Lang, passed: &str, total: &str) -> String {
        match self {
            Pass::BackUp => texts::backup::job::backed_up_of(l, passed, total),
            Pass::Restore => texts::backup::job::restored_of(l, passed, total),
            Pass::Verify => texts::backup::job::verified_of(l, passed, total),
        }
    }
}

/// Where part `k` of a backup or restore stands, as its row says it.
fn part_state(l: Lang, job: &BackupJob, k: usize) -> String {
    use std::cmp::Ordering;
    use texts::backup::job as said;
    let wrote = Pass::writing(job.restore).done();
    let ended = job.outcome.is_some();
    let a = match (&job.outcome, job.at) {
        (Some(BackupEnd::Saved(_)), _) => return l.pick(if job.checks { said::VERIFIED } else { wrote }).to_string(),
        (Some(BackupEnd::Restored { .. }), _) => return l.pick(wrote).to_string(),
        (_, None) => return l.pick(if ended { said::NOT_STARTED } else { texts::common::WAITING }).to_string(),
        (_, Some(a)) => a,
    };
    let state = match k.cmp(&a.item) {
        Ordering::Less if a.checking => said::VERIFIED,
        Ordering::Less => wrote,
        Ordering::Equal if ended => texts::common::UNFINISHED,
        Ordering::Equal => return Pass::at(job, &a).doing_share(l, percent(part_share(&a))),
        // Ahead of the pass under way: a backup checks its parts once all
        // are written, and a restore writes them once all are checked.
        Ordering::Greater => match (a.checking, job.restore, ended) {
            (true, false, false) => wrote,
            (true, false, true) => said::NOT_VERIFIED,
            (false, false, false) | (true, true, false) => texts::common::WAITING,
            (false, false, true) => said::NOT_BACKED_UP,
            (false, true, false) => said::VERIFIED,
            (_, true, true) => said::NOT_RESTORED,
        },
    };
    l.pick(state).to_string()
}

/// Backing up or restoring: how far it has come over all, each part and
/// where it stands, and a way to stop while stopping breaks nothing. Once
/// it has ended, how, and where to go.
pub fn backup_job(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let job = s.backup_job.as_ref().expect("the backup page shows work");
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    use texts::backup::job as said;
    let icon = if job.restore { Icon::Restore } else { Icon::Backup };
    let writing = job.at.is_some_and(|a| !a.checking);
    let mut y = match &job.outcome {
        None => {
            let (title, detail, done) = match job.at {
                Some(a) => {
                    let (over, passed) = job_progress(job, &a);
                    let total = fmt::bytes(job.plan.iter().map(|(_, n)| n).sum());
                    let pass = Pass::at(job, &a);
                    (pass.doing_part(l, &job.plan[a.item].0), pass.done_of(l, &fmt::bytes(passed), &total), over)
                }
                None if job.restore => (l.pick(said::READING).to_string(), l.pick(said::CHECKING).to_string(), 0.0),
                None => (l.pick(texts::installing::GETTING_READY).to_string(), l.pick(said::SIZING).to_string(), 0.0),
            };
            widgets::progress_card(&mut c, t, f, top, &title, &detail, Some(done))
        }
        Some(end) => {
            let gone = l.pick(if job.restore { texts::installed::NOTHING_WRITTEN } else { said::NOTHING_SAVED });
            let failed = l.pick(if job.restore { said::RESTORE_FAILED } else { said::BACKUP_FAILED });
            let (title, color, detail) = match end {
                BackupEnd::Saved(name) => {
                    let saved = match job.place {
                        Place::Storage => said::saved_to_storage(l, name),
                        Place::UsbDrive => said::saved_to_usb_drive(l, name),
                    };
                    (l.pick(said::BACKUP_COMPLETE), GREEN, saved)
                }
                BackupEnd::Restored { .. } => (l.pick(said::RESTORE_COMPLETE), GREEN, l.pick(said::USES_RESTORED).to_string()),
                BackupEnd::Stopped => (l.pick(texts::common::STOPPED), SLATE, gone.to_string()),
                BackupEnd::Refused(r) => {
                    let title = l.pick(if job.restore { said::CANT_RESTORE } else { said::CANT_BACK_UP });
                    (title, ORANGE, fmt::backup_refusal(l, r, job.restore))
                }
                BackupEnd::Failed(_) if job.restore && writing => (failed, RED, l.pick(said::PARTLY_WRITTEN).to_string()),
                BackupEnd::Failed(_) => (failed, RED, gone.to_string()),
            };
            widgets::list(&mut c, t, f, top, &[standing(title, color, icon, &detail)]).bottom
        }
    };
    match &job.outcome {
        None => {
            let note = match (job.restore, job.at, job.place) {
                (false, _, Place::Storage) => said::BACKUP_KEEP_POWER,
                (false, _, Place::UsbDrive) => said::BACKUP_KEEP_USB_DRIVE,
                (true, Some(a), Place::Storage) if !a.checking => said::RESTORE_KEEP_POWER,
                (true, Some(a), Place::UsbDrive) if !a.checking => said::RESTORE_KEEP_USB_DRIVE,
                (true, _, _) => said::NOTHING_UNTIL_VERIFIED,
            };
            y = widgets::footnote(&mut c, t, f, y, l.pick(note));
            // A restore that has begun writing goes on to its end.
            if !(job.restore && writing) {
                let label = l.pick(if job.stopping { texts::common::STOPPING } else { texts::common::STOP });
                let stop = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(label).trail(Trail::None)]);
                if !job.stopping {
                    out.extend(hits(&stop.rows, &[Action::StopBackup]));
                }
                y = stop.bottom;
            }
        }
        Some(end) => {
            match end {
                BackupEnd::Restored { unmade } if !unmade.is_empty() => {
                    let note = said::users_gone(l, unmade.join(l.pick(texts::list::SEPARATOR)));
                    y = widgets::footnote(&mut c, t, f, y, &note);
                }
                BackupEnd::Failed(why) => y = widgets::footnote(&mut c, t, f, y, why),
                _ => {}
            }
            let (rows, acts): (Vec<Row>, Vec<Action>) = match end {
                BackupEnd::Restored { .. } => (
                    vec![Row::new(l.pick(texts::installed::REBOOT_TO_SYSTEM)).icon(ORANGE, Icon::Power), Row::new(l.pick(texts::common::DONE)).trail(Trail::None)],
                    vec![Action::RebootTo(Target::System), Action::Back],
                ),
                _ => (vec![Row::new(l.pick(texts::common::DONE)).trail(Trail::None)], vec![Action::Back]),
            };
            let next = widgets::list(&mut c, t, f, y + t.m.card_gap, &rows);
            out.extend(hits(&next.rows, &acts));
            y = next.bottom;
        }
    }
    // The parts come after what can be done, which a long list would push
    // off the screen.
    if !job.plan.is_empty() {
        let heading = l.pick(if job.restore { texts::restore::CONTENTS } else { texts::backup::CONTENTS });
        let y0 = widgets::section(&mut c, t, f, y, heading);
        let states: Vec<String> = (0..job.plan.len()).map(|k| part_state(l, job, k)).collect();
        let rows: Vec<Row> = job.plan.iter().zip(&states).map(|((name, _), state)| Row::new(name).trail(Trail::Text(state))).collect();
        y = widgets::list(&mut c, t, f, y0, &rows).bottom;
    }
    let title = l.pick(match (&job.outcome, job.restore) {
        (None, restore) => Pass::writing(restore).doing(),
        (Some(BackupEnd::Saved(_)), _) => said::BACKUP_COMPLETE,
        (Some(BackupEnd::Restored { .. }), _) => said::RESTORE_COMPLETE,
        (Some(_), false) => said::BACKUP_INCOMPLETE,
        (Some(_), true) => said::RESTORE_INCOMPLETE,
    });
    Frame { back: job.outcome.is_some(), ..Frame::new(t, fit(c, t, y), out, title, top) }
}

/// Tools for the experienced.
/// The folder the file manager shows, served to a browser on the same
/// network: the folder, the switch that serves it, and where it is.
pub fn serve(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let mut y = widgets::section(&mut c, t, f, m.content_top, l.pick(["传输", "傳輸", "Transfer"]));
    let folder = &s.files.path;
    let row = Row::new(l.pick(["文件夹", "資料夾", "Folder"]))
        .icon(BLUE, Icon::Folder)
        .subtitle(folder)
        .trail(Trail::Value(l.pick(["更改", "更改", "Change"])));
    let pick = widgets::list(&mut c, t, f, y, &[row]);
    out.extend(hits(&pick.rows, &[Action::ServePick]));
    y = widgets::section(&mut c, t, f, pick.bottom, l.pick(["服务", "服務", "Server"]));
    let on = matches!(s.serving, Serving::Running { .. });
    let mut line = Row::new(l.pick(["启动服务", "啟動服務", "Serve"]))
        .icon(if on { GREEN } else { SLATE }, Icon::Globe)
        .trail(Trail::Switch(on));
    if let Serving::Failed(why) = &s.serving {
        line = line.subtitle(why);
    }
    let run = widgets::list(&mut c, t, f, y, &[line]);
    out.extend(hits(&run.rows, &[Action::ServeToggle]));
    let below = run.bottom;
    if let Serving::Running { at, served } = &s.serving {
        let said = tr!(l, "已应答 {served} 次请求", "已回應 {served} 次請求", "{served} requests answered");
        // 服务开着但 WLAN 没连上时端口其实没有地址可给别人：照实说，别摆一个空地址。
        let joined = s.wlan.as_ref().and_then(|w| w.joined.as_ref()).is_some();
        let address = match joined {
            true => at.clone(),
            false => tr!(l, "WLAN 未连接，连上后再看地址", "WLAN 未連線，連上後再看位址", "Not joined to Wi-Fi; the address shows once it is"),
        };
        widgets::info_card(&mut c, t, f, below + m.card_gap, &[
            (l.pick(["地址", "位址", "Address"]), address),
            (l.pick(["请求", "請求", "Requests"]), said),
        ]);
    } else {
        let hint = l.pick([
            "在同一网络下用浏览器打开上面的地址，即可上传或下载这个文件夹里的文件。WLAN 需先连接。",
            "在同一網路下用瀏覽器開啟上面的位址，即可上傳或下載這個資料夾裡的檔案。Wi-Fi 需先連線。",
            "Open the address above in a browser on the same network to move files in and out of this folder. Wi-Fi has to be joined first.",
        ]);
        widgets::footnote(&mut c, t, f, below, hint);
    }
    Frame::new(t, c, out, l.pick(SERVE), m.content_top)
}

const SERVE: Text = ["无线传输", "無線傳輸", "Wireless transfer"];
const FETCH: Text = ["网络下载", "網路下載", "Download"];
const AVB: Text = ["AVB 校验", "AVB 驗證", "AVB verification"];
const TASKS: Text = ["任务", "工作", "Tasks"];

/// Verified boot's two checks in the slot the phone runs from, each turned by
/// its own row, and written once the swipe below is dragged to its end.
pub fn avb(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut out = Vec::new();
    let (now_verity, now_verification) = match &s.avb {
        Some(Ok(now)) => *now,
        _ => (false, false),
    };
    // Before the swipe, the rows show what is about to be written.
    let (verity, verification) = s.avb_pending.unwrap_or((now_verity, now_verification));
    let rows = widgets::list(&mut c, t, f, top, &[
        Row::new(l.pick(["逐块校验（dm-verity）", "逐塊校驗（dm-verity）", "Block checks (dm-verity)"]))
            .icon(RED, Icon::Shield)
            .trail(Trail::Switch(verity)),
        Row::new(l.pick(["启动校验（verification）", "啟動驗證（verification）", "Boot checks (verification)"]))
            .icon(RED, Icon::Shield)
            .trail(Trail::Switch(verification)),
    ]);
    out.extend(hits(&rows.rows, &[Action::Avb(false), Action::Avb(true)]));
    let note = l.pick([
        "关闭后，改过的系统才能启动：启动校验关掉，bootloader 不再检查分区；逐块校验关掉，读到的块不再与签名比对。两者都要解锁 bootloader。",
        "關閉後，改過的系統才能啟動：關閉啟動驗證後 bootloader 不再檢查分割區；關閉逐塊校驗後讀到的區塊不再與簽章比對。兩者都需要解鎖 bootloader。",
        "Off, a changed system can start: with boot checks off the bootloader no longer checks the partitions, and with block checks off a block read is no longer compared with its signature. Both need an unlocked bootloader.",
    ]);
    let y = widgets::footnote(&mut c, t, f, rows.bottom, note);
    let y = match &s.avb_notice {
        Some(text) => widgets::footnote(&mut c, t, f, y, text),
        None => y,
    };
    let y = match &s.avb {
        Some(Err(why)) => widgets::footnote(&mut c, t, f, y, why),
        _ => y,
    };
    // 留着的那几份 vbmeta：留一份、选一份、问要不要写回。写回用的是页底那
    // 一条滑道——问的时候它换成写回，不问的时候它还是写上面那两个开关。
    let y = widgets::section(&mut c, t, f, y, l.pick(["vbmeta 备份", "vbmeta 備份", "vbmeta kept"]));
    let y = match &s.vbmeta_copies {
        None => widgets::footnote(&mut c, t, f, y, l.pick(["正在读取", "正在讀取", "Reading"])),
        Some(Err(why)) => widgets::footnote(&mut c, t, f, y, why),
        Some(Ok(list)) if list.is_empty() => {
            widgets::footnote(&mut c, t, f, y, l.pick(["还没有留过副本", "還沒有留過備份", "No copy kept yet"]))
        }
        Some(Ok(list)) => {
            let details: Vec<String> = list.iter().map(|copy| vbmeta_copy_note(l, copy)).collect();
            let rows: Vec<Row> = list
                .iter()
                .zip(&details)
                .enumerate()
                .map(|(k, (copy, detail))| Row::new(&copy.name).subtitle(detail).trail(Trail::Check(s.vbmeta_pick == Some(k))))
                .collect();
            let kept = widgets::list(&mut c, t, f, y, &rows);
            out.extend(hits(&kept.rows, &(0..list.len()).map(Action::VbmetaPick).collect::<Vec<_>>()));
            kept.bottom
        }
    };
    let keep = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[Row::new(l.pick(["留一份当前的 vbmeta", "留一份目前的 vbmeta", "Keep this slot's vbmeta"]))
            .icon(RED, Icon::Shield)
            .subtitle(place_name(l, s.backup_to))],
    );
    out.extend(hits(&keep.rows, &[Action::VbmetaKeep]));
    let mut y = keep.bottom;
    let picked = s.vbmeta_pick.and_then(|k| match &s.vbmeta_copies {
        Some(Ok(list)) => list.get(k),
        _ => None,
    });
    if let Some(copy) = picked {
        let name = copy.name.clone();
        let label = tr!(l, "写回「{name}」", "寫回「{name}」", "Write \u{201c}{name}\u{201d} back");
        let ask = widgets::list(&mut c, t, f, y, &[Row::new(&label).icon(RED, Icon::Restore)]);
        out.extend(hits(&ask.rows, &[Action::VbmetaAsk(true)]));
        y = ask.bottom;
        if s.vbmeta_asking {
            let cancel = widgets::list(
                &mut c,
                t,
                f,
                y,
                &[Row::new(l.pick(["先不写回", "先不寫回", "Leave it as it is"])).trail(Trail::None)],
            );
            out.extend(hits(&cancel.rows, &[Action::VbmetaAsk(false)]));
            y = cancel.bottom;
        }
    }
    y = widgets::footnote(
        &mut c,
        t,
        f,
        y + t.m.card_gap,
        l.pick([
            "写回一份比手机旧的回滚索引，bootloader 就不再让手机开机：这种写回会被拒绝，页上会说清楚两个索引各是多少。",
            "寫回一份比手機舊的回滾索引，bootloader 就不再讓手機開機：這種寫回會被拒絕，頁上會說清楚兩個索引各是多少。",
            "A copy whose rollback index is below the phone's makes the bootloader refuse to start it, so such a write is refused and both indexes are said.",
        ]),
    );
    let which = match s.vbmeta_asking {
        true => Confirm::VbmetaRestore,
        false => Confirm::Avb,
    };
    let (label, danger) = confirm_label(which, s);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(which) });
    Frame::new(t, fit(c, t, track.y + track.h), out, l.pick(AVB), top)
}

/// A length of bytes in the units the pages speak.
fn mb(bytes: u64) -> String {
    match bytes {
        b if b >= 1 << 20 => format!("{:.1} MB", b as f64 / (1u64 << 20) as f64),
        b if b >= 1 << 10 => format!("{:.1} KB", b as f64 / (1u64 << 10) as f64),
        b => format!("{b} B"),
    }
}

/// A file from an address typed on the page's keyboard: the field and the
/// button, the way an address too long to type is taken from a file or from
/// another device through the page served here, and how the download stands.
pub fn fetch(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();
    let wide = m.w as f32 - m.margin * 2.0;
    let floor = text_floor(&t) - m.dp(10.0);
    let top = m.content_top - m.dp(66.0);
    let hint = l.pick(["粘贴或输入完整网址", "貼上或輸入完整網址", "Type or paste a full address"]);
    let field = widgets::search_field(&mut c, t, f, m.margin, wide, top + m.dp(24.0), &s.fetch_url, hint);
    let running = matches!(s.fetch, Fetch::Running { .. });
    let label = match running {
        true => l.pick(["停止下载", "停止下載", "Stop"]),
        false => l.pick(["开始下载", "開始下載", "Download"]),
    };
    let (bh, bw) = (m.dp(48.0), m.dp(220.0).min(wide));
    let mut y = field.field.y + field.field.h + m.dp(26.0);
    let (bx, by) = ((m.w as f32 - bw) / 2.0, y);
    c.fill_round_rect(bx, by, bw, bh, bh / 2.0, t.pal.accent);
    c.text_center(&f.heading, label, bx + bw / 2.0, by + bh / 2.0, m.row_title_px, Rgba::rgb(255, 255, 255));
    out.push(Hit {
        rect: Rect::new(bx, by, bw, bh),
        action: match running { true => Action::FetchStop, false => Action::FetchStart },
    });
    y = by + bh + m.dp(16.0);
    let from_file = l.pick([
        "从 /sdcard/Download/link.txt 读链接",
        "從 /sdcard/Download/link.txt 讀連結",
        "Take the link from /sdcard/Download/link.txt",
    ]);
    c.text_center(&f.body, from_file, m.w as f32 / 2.0, y + m.dp(18.0), m.sub_px, t.pal.accent);
    out.push(Hit { rect: Rect::new(m.margin, y, wide, m.dp(36.0)), action: Action::FetchFromFile });
    y += m.dp(36.0) + m.dp(10.0);
    // A link too long for this keyboard can be typed on another device, which
    // opens a page this recovery serves; the code is where that page is.
    if let Some(at) = s.fetch_portal.clone() {
        let caption = m.dp(40.0);
        let room = (floor - y - caption - m.dp(14.0)).max(m.dp(96.0));
        let side = m.dp(168.0).min(wide).min(room);
        let x0 = (m.w as f32 - side) / 2.0;
        let quiet = 2.0f32;
        if let Ok(code) = qrcodegen::QrCode::encode_text(&at, qrcodegen::QrCodeEcc::Medium) {
            let n = code.size();
            let cell = side / (n as f32 + quiet * 2.0);
            c.fill_round_rect(x0 - m.dp(12.0), y - m.dp(6.0), side + m.dp(24.0), side + m.dp(24.0), m.dp(14.0), Rgba::rgb(255, 255, 255));
            for row in 0..n {
                for col in 0..n {
                    if code.get_module(col, row) {
                        let px = x0 + (col as f32 + quiet) * cell;
                        let py = y + m.dp(6.0) + (row as f32 + quiet) * cell;
                        c.fill_rect(px, py, cell + 0.5, cell + 0.5, Rgba::rgb(12, 12, 16));
                    }
                }
            }
        }
        y += side + m.dp(20.0);
        let how = l.pick([
            "用另一台手机扫这个码，在网页里输入网址，发回来就填进上面的框",
            "用另一台手機掃這個碼，在網頁裡輸入網址，傳回來就填進上面的框",
            "Scan this on another device, type the address there, and it fills the field above",
        ]);
        widgets::footnote(&mut c, t, f, y, how);
        y += caption;
    }
    match &s.fetch {
        Fetch::Idle => {
            let how = l.pick([
                "文件保存到 /sdcard/Download，名字取网址里最后一段。",
                "檔案儲存到 /sdcard/Download，名字取網址裡最後一段。",
                "Files are saved in /sdcard/Download, named after the last part of the address.",
            ]);
            widgets::footnote(&mut c, t, f, y, how);
        }
        Fetch::Running { done, total } => {
            let share = total.map(|t| (*done as f32 / (t.max(1) as f32)).clamp(0.0, 1.0));
            let detail = match (total, share) {
                (Some(total), Some(share)) => {
                    let (done_s, total_s, pct) = (mb(*done), mb(*total), percent(share));
                    tr!(l, "{done_s} / {total_s}（{pct}%）", "{done_s} / {total_s}（{pct}%）", "{done_s} of {total_s} ({pct}%)")
                }
                _ => mb(*done),
            };
            widgets::progress_card(&mut c, t, f, y, l.pick(["正在下载", "正在下載", "Downloading"]), &detail, share);
        }
        Fetch::Done { path, bytes } => {
            let (where_to, size_s) = (path.clone(), mb(*bytes));
            let detail = tr!(l, "已保存 {where_to}（{size_s}）", "已儲存 {where_to}（{size_s}）", "Saved {where_to} ({size_s})");
            widgets::info_card(&mut c, t, f, y, &[(l.pick(["完成", "完成", "Done"]), detail)]);
        }
        Fetch::Failed(why) => {
            widgets::info_card(&mut c, t, f, y, &[(l.pick(["失败", "失敗", "Failed"]), why.clone())]);
        }
        Fetch::Stopped => {
            widgets::info_card(&mut c, t, f, y, &[(l.pick(["已停止", "已停止", "Stopped"]), String::new())]);
        }
    }
    let title = l.pick(FETCH);
    let enter = match running {
        true => l.pick(["停止下载", "停止下載", "Stop"]),
        false => l.pick(["开始下载", "開始下載", "Download"]),
    };
    keyboard_hits(&mut c, t, f, s, enter, &mut out);
    if let Some(rect) = field.clear {
        out.push(Hit { rect, action: Action::ClearSearch });
    }
    Frame { header: false, caret: Some(field.caret), ..Frame::new(t, c, out, title, 0.0) }
}

/// The modules the phone's own system loads — KernelSU's and Magisk's keep them
/// the same way — each with the switch that turns it on or off.
pub fn modules(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let top = t.m.content_top;
    let mut c = page(cx);
    let mut out = Vec::new();
    // 与应用管理同一套：一屏一页，行高按「名称 + 版本」那一行量。
    let probe = Canvas::band(t.m.w, 1, 0..1, 0..t.m.w, t.pal.bg);
    let sample_h = {
        let mut sample = Row::new("Ag").icon(BLUE, Icon::Layers);
        sample.sub = Some("v1.0 · someone");
        widgets::one_height(&probe, t, &f, &sample)
    };
    let room = t.m.h as f32 - top - t.m.margin * 2.0 - sample_h / 2.0;
    let per = ((room / sample_h).floor() as usize).max(1);
    let mut rows: Vec<Row> = Vec::new();
    let mut acts: Vec<Action> = Vec::new();
    let mut pages = 1usize;
    match &s.modules {
        None => rows.push(Row::new(l.pick(["正在读取…", "正在讀取…", "Reading…"]))),
        Some(Err(why)) => {
            let mut row = Row::new(l.pick(["读取失败", "讀取失敗", "Not read"]));
            row.sub = Some(why.as_str());
            rows.push(row);
        }
        Some(Ok(list)) if list.is_empty() => rows.push(Row::new(l.pick([
            "没有安装模块",
            "沒有安裝模組",
            "No modules installed",
        ]))),
        Some(Ok(list)) => {
            pages = list.len().div_ceil(per).max(1);
            let page = s.module_page.min(pages - 1);
            let from = page * per;
            let to = (from + per).min(list.len());
            for (k, m) in list.iter().enumerate().take(to).skip(from) {
                let mut row = Row::new(&m.name).icon(if m.enabled { BLUE } else { SLATE }, Icon::Layers).trail(Trail::Switch(m.enabled));
                let off = l.pick(["已关闭", "已關閉", "Off"]);
                let removing = l.pick(["下次启动时移除", "下次啟動時移除", "removed next start"]);
                row.sub = Some(match (m.enabled, m.removing, m.note.is_empty()) {
                    (false, true, _) => removing,
                    (false, false, _) => off,
                    (true, true, _) => removing,
                    (true, false, true) => m.about.as_str(),
                    (true, false, false) => m.note.as_str(),
                });
                rows.push(row);
                acts.push(Action::ModuleOpen(k));
            }
        }
    }
    // 返回箭头那一行：右上角「安装」「搜索模块」；下一行只放翻页与页数。
    let ph = t.m.dp(widgets::PILL_DP);
    // 搜索在返回箭头那一行，翻页与标题「应用管理」同一行。
    let py = t.m.back_cy - ph / 2.0;
    // 与标题同一水平线。实测：标题的墨迹从 title_top 起、占到 0.92×字号，
    // 中心就在 0.46×字号处（f.title.ink 给的是相对基线的值，用它会偏上）。
    let pager_y = t.m.title_top + t.m.title_px * 0.46 - ph / 2.0;
    let list_top = top;
    let page = match &s.modules {
        Some(Ok(list)) if !list.is_empty() => s.module_page.min(list.len().div_ceil(per).max(1) - 1),
        _ => 0,
    };
    let gaps = t.m.dp(widgets::PILL_GAP_DP);
    let mut x_right = t.m.w as f32 - t.m.margin;
    let search = l.pick(["搜索模块", "搜尋模組", "Search modules"]);
    let w = widgets::pill_width(&c, t, &f, search);
    x_right -= w;
    let wd = widgets::pill(&mut c, t, &f, x_right, py, search, t.pal.search, t.pal.text);
    out.push(pill_hit(t, x_right, py, wd, ph, Action::SearchModules));
    x_right -= gaps;
    let install = l.pick(["安装", "安裝", "Install"]);
    let w = widgets::pill_width(&c, t, &f, install);
    x_right -= w;
    let wd = widgets::pill(&mut c, t, &f, x_right, py, install, t.pal.search, t.pal.text);
    out.push(pill_hit(t, x_right, py, wd, ph, Action::Open(Screen::Files)));
    if pages <= 1 {
        let only = l.pick(["只有一页", "只有一頁", "One page"]);
        let w = widgets::pill_width(&c, t, &f, only);
        let x = t.m.w as f32 - t.m.margin - w;
        widgets::pill(&mut c, t, &f, x, pager_y, only, t.pal.search, t.pal.text);
    }
    if pages > 1 {
        // 与应用页一样：三格位置固定，不可用的只画淡。
        let gaps = t.m.dp(widgets::PILL_GAP_DP);
        let counter = format!("{}/{}", page + 1, pages);
        let back = l.pick(["上一页", "上一頁", "Newer"]);
        let fwd = l.pick(["下一页", "下一頁", "Older"]);
        let w_prev = widgets::pill_width(&c, t, &f, back);
        let w_next = widgets::pill_width(&c, t, &f, fwd);
        let w_counter = c.text_width(&f.value, &counter, t.m.sub_px) + t.m.dp(8.0);
        let x_next = t.m.w as f32 - t.m.margin - w_next;
        let x_counter = x_next - gaps - w_counter;
        let x_prev = x_counter - gaps - w_prev;
        let faint = t.pal.subtitle;
        if page + 1 < pages {
            out.push(Hit {
                rect: Rect::new(t.m.w as f32 * 0.66, pager_y - t.m.dp(12.0), t.m.w as f32 * 0.34 - t.m.margin, ph + t.m.dp(24.0)),
                action: Action::ModulePage(1),
            });
        }
        if page > 0 {
            let wd = widgets::pill(&mut c, t, &f, x_prev, pager_y, back, t.pal.search, t.pal.text);
            let left = x_prev - t.m.dp(16.0);
            let right = (x_prev + wd + t.m.dp(8.0)).min(t.m.w as f32 * 0.62);
            out.push(Hit {
                rect: Rect::new(left, pager_y - t.m.dp(10.0), (right - left).max(wd), ph + t.m.dp(20.0)),
                action: Action::ModulePage(-1),
            });
        } else {
            widgets::pill(&mut c, t, &f, x_prev, pager_y, back, t.pal.search, faint);
        }
        c.text_center(&f.value, &counter, x_counter + w_counter / 2.0, pager_y + ph / 2.0, t.m.sub_px, faint);
        if page + 1 < pages {
            widgets::pill(&mut c, t, &f, x_next, pager_y, fwd, t.pal.search, t.pal.text);
        } else {
            widgets::pill(&mut c, t, &f, x_next, pager_y, fwd, t.pal.search, faint);
        }
    }
    let listed = widgets::list(&mut c, t, f, list_top, &rows);
    out.extend(hits(&listed.rows, &acts));
    Frame::new(t, c, out, l.pick(MODULES), top)
}

/// One module: what it says of itself, and the two things that can be done
/// with it — switched off or on, and taken away at the next start.
pub fn module_info(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let top = t.m.content_top;
    let at = s.module_at.unwrap_or(0);
    let picked = match &s.modules {
        Some(Ok(list)) => list.get(at),
        _ => None,
    };
    let Some(m) = picked else {
        return Frame::new(t, c, Vec::new(), l.pick(MODULES), top);
    };
    let state_note = match (m.enabled, m.removing) {
        (true, true) => l.pick(["已开启 · 下次启动时移除", "已開啟 · 下次啟動時移除", "On · removed next start"]),
        (true, false) => l.pick(["已开启", "已開啟", "On"]),
        (false, _) => l.pick(["已关闭", "已關閉", "Off"]),
    };
    let mut facts: Vec<(&str, String)> = vec![(l.pick([["版本", "版本", "Version"]][0]), m.note.clone())];
    if !m.about.is_empty() {
        facts.push((l.pick([["说明", "說明", "About"]][0]), m.about.clone()));
    }
    facts.push((l.pick([["状态", "狀態", "State"]][0]), state_note.to_string()));
    facts.push((l.pick([["位置", "位置", "Where"]][0]), format!("/data/adb/modules/{}", m.id)));
    let y = widgets::info_card(&mut c, t, f, top, &facts);
    let switch_label = if m.enabled {
        l.pick(["关闭模块", "關閉模組", "Turn it off"])
    } else {
        l.pick(["开启模块", "開啟模組", "Turn it on"])
    };
    let remove_label = if m.removing {
        l.pick(["取消卸载", "取消卸載", "Keep it"])
    } else {
        l.pick(["卸载模块", "卸載模組", "Uninstall"])
    };
    let acts = widgets::list(&mut c, t, f, y + t.m.card_gap, &[
        Row::new(switch_label).icon(BLUE, Icon::Layers),
        Row::new(remove_label).icon(RED, Icon::Wipe).tint(t.pal.danger),
    ]);
    let out = hits(&acts.rows, &[Action::ModuleToggle(at), Action::ModuleRemove(at)]);
    Frame::new(t, c, out, l.pick(MODULES), top)
}


const MODULES: Text = ["模块管理", "模組管理", "Modules"];

/// 小药丸的点击区：竖直放宽到 44dp，手指略有抖动也算点在它上面。
fn pill_hit(t: &Theme, x: f32, y: f32, w: f32, h: f32, action: Action) -> Hit {
    let pad = (t.m.dp(44.0) - h).max(0.0) / 2.0;
    Hit { rect: Rect::new(x - t.m.dp(4.0), y - pad, w + t.m.dp(8.0), h + pad * 2.0), action }
}

const DATA_APPS: Text = ["应用管理", "應用程式管理", "Apps"];

/// The apps the phone has in its data partition, newest first.
pub fn data_apps(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let top = t.m.content_top;
    let mut c = page(cx);
    let mut out = Vec::new();
    // 一屏能放几行：按“名称 + 包名”那一行的实际高度算，再多留半行余量，
    // 免得最后一行被裁掉半截。
    let probe = Canvas::band(t.m.w, 1, 0..1, 0..t.m.w, t.pal.bg);
    let sample_h = {
        let mut sample = Row::new("Ag").icon(PURPLE, Icon::Layers);
        sample.sub = Some("com.example.app");
        widgets::one_height(&probe, t, &f, &sample)
    };
    let room = t.m.h as f32 - top - t.m.margin * 2.0 - sample_h / 2.0;
    let per = ((room / sample_h).floor() as usize).max(1);
    let mut rows: Vec<Row> = Vec::new();
    let mut acts: Vec<Action> = Vec::new();
    let mut pages = 1usize;
    // 开关：用户应用 / 系统应用。看一眼数，选中的那颗高亮。
    let user_n = match &s.apps {
        Some(Ok(list)) => list.iter().filter(|a| !a.system).count(),
        _ => 0,
    };
    let system_n = match &s.apps {
        Some(Ok(list)) => list.iter().filter(|a| a.system).count(),
        _ => 0,
    };
    match &s.apps {
        None => rows.push(Row::new(l.pick(["正在读取…", "正在讀取…", "Reading…"]))),
        Some(Err(why)) => {
            let mut row = Row::new(l.pick(["读取失败", "讀取失敗", "Not read"]));
            row.sub = Some(why.as_str());
            rows.push(row);
        }
        Some(Ok(list)) if list.is_empty() => rows.push(Row::new(l.pick([
            "数据分区里没有应用",
            "資料分區裡沒有應用程式",
            "No apps in the data partition",
        ]))),
        Some(Ok(list)) => {
            pages = list.len().div_ceil(per).max(1);
            let page = s.app_page.min(pages - 1);
            let from = page * per;
            let to = (from + per).min(list.len());
            for (k, a) in list.iter().enumerate().take(to).skip(from) {
                let mut row = Row::new(&a.label).icon(PURPLE, Icon::Layers);
                row.sub = Some(&a.package);
                rows.push(row);
                acts.push(Action::DataAppOpen(k));
            }
        }
    }
    // 返回箭头那一行只放「搜索应用」，靠右；翻页键与页数另起一行放在它下面。
    let ph = t.m.dp(widgets::PILL_DP);
    // 搜索在返回箭头那一行；翻页与标题同一水平线（墨迹中心在 0.46×字号处）。
    let py = t.m.back_cy - ph / 2.0;
    let pager_y = t.m.title_top + t.m.title_px * 0.46 - ph / 2.0;
    let list_top = top;
    let page = s.app_page.min(pages.saturating_sub(1));
    let mut x = t.m.w as f32 - t.m.margin;
    let search_label = l.pick(["搜索应用", "搜尋應用程式", "Search apps"]);
    let w = widgets::pill_width(&c, t, &f, search_label);
    x -= w;
    let wd = widgets::pill(&mut c, t, &f, x, py, search_label, t.pal.search, t.pal.text);
    out.push(pill_hit(t, x, py, wd, ph, Action::SearchApps));
    // 「系统应用」：只看的清单，单独一页。
    let sys_label = l.pick(["系统应用", "系統應用程式", "System apps"]);
    let w = widgets::pill_width(&c, t, &f, sys_label);
    x -= t.m.dp(widgets::PILL_GAP_DP) + w;
    let wd = widgets::pill(&mut c, t, &f, x, py, sys_label, t.pal.search, t.pal.subtitle);
    out.push(pill_hit(t, x, py, wd, ph, Action::Open(Screen::SystemApps)));
    // 只有一页时也说一句，别留一块空。
    if pages <= 1 {
        let only = l.pick(["只有一页", "只有一頁", "One page"]);
        let w = widgets::pill_width(&c, t, &f, only);
        let x = t.m.w as f32 - t.m.margin - w;
        widgets::pill(&mut c, t, &f, x, pager_y, only, t.pal.search, t.pal.text);
    }
    if pages > 1 {
        // 三格位置固定：上一页、页码、下一页。不可用的那格只画淡，不参与点击，
        // 也不移动——否则翻页后按钮挪位，下一次点会点到另一个键上（2026-11-18）。
        let gaps = t.m.dp(widgets::PILL_GAP_DP);
        let counter = format!("{}/{}", page + 1, pages);
        let w_prev = widgets::pill_width(&c, t, &f, l.pick(["上一页", "上一頁", "Newer"]));
        let w_next = widgets::pill_width(&c, t, &f, l.pick(["下一页", "下一頁", "Older"]));
        let w_counter = c.text_width(&f.value, &counter, t.m.sub_px) + t.m.dp(8.0);
        let x_next = t.m.w as f32 - t.m.margin - w_next;
        let x_counter = x_next - gaps - w_counter;
        let x_prev = x_counter - gaps - w_prev;
        let faint = t.pal.subtitle;
        let back = l.pick(["上一页", "上一頁", "Newer"]);
        let fwd = l.pick(["下一页", "下一頁", "Older"]);
        // 屏幕右侧三分之一都是「下一页」：手指偏一点也照样翻。
        if page + 1 < pages {
            out.push(Hit {
                rect: Rect::new(t.m.w as f32 * 0.66, pager_y - t.m.dp(12.0), t.m.w as f32 * 0.34 - t.m.margin, ph + t.m.dp(24.0)),
                action: Action::AppPage(1),
            });
        }
        if page > 0 {
            let wd = widgets::pill(&mut c, t, &f, x_prev, pager_y, back, t.pal.search, t.pal.text);
            // 上一页只到屏幕 62% 处，右边那一大片留给「下一页」。
            let left = x_prev - t.m.dp(16.0);
            let right = (x_prev + wd + t.m.dp(8.0)).min(t.m.w as f32 * 0.62);
            out.push(Hit {
                rect: Rect::new(left, pager_y - t.m.dp(10.0), (right - left).max(wd), ph + t.m.dp(20.0)),
                action: Action::AppPage(-1),
            });
        } else {
            widgets::pill(&mut c, t, &f, x_prev, pager_y, back, t.pal.search, faint);
        }
        c.text_center(&f.value, &counter, x_counter + w_counter / 2.0, pager_y + ph / 2.0, t.m.sub_px, faint);
        if page + 1 < pages {
            widgets::pill(&mut c, t, &f, x_next, pager_y, fwd, t.pal.search, t.pal.text);
        } else {
            widgets::pill(&mut c, t, &f, x_next, pager_y, fwd, t.pal.search, faint);
        }
    }
    let listed = widgets::list(&mut c, t, f, list_top, &rows);
    out.extend(hits(&listed.rows, &acts));
    // 记下这次画出来的总页数：点空时能核对翻页键是否已在画面上。
    // （pages 只在函数内，借用 &Ctx 只读，所以这里写到别处由调用方转记。）
    Frame::new(t, c, out, l.pick(DATA_APPS), top)
}

/// 系统分区里的预装应用：与用户应用同一套排布，但只能看——没有开关也没有
/// 卸载，条目本身不可点（2026-11-18）。
pub fn system_apps(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let top = t.m.content_top;
    let mut c = page(cx);
    let probe = Canvas::band(t.m.w, 1, 0..1, 0..t.m.w, t.pal.bg);
    let sample_h = {
        let mut sample = Row::new("Ag").icon(SLATE, Icon::Layers);
        sample.sub = Some("/system/app");
        widgets::one_height(&probe, t, &f, &sample)
    };
    let room = t.m.h as f32 - top - t.m.margin * 2.0 - sample_h / 2.0;
    let per = ((room / sample_h).floor() as usize).max(1);
    let mut rows: Vec<Row> = Vec::new();
    let mut acts: Vec<Action> = Vec::new();
    let mut pages = 1usize;
    match &s.sys_apps {
        None => rows.push(Row::new(l.pick(["正在读取…", "正在讀取…", "Reading…"]))),
        Some(list) if list.is_empty() => rows.push(Row::new(l.pick([
            "没有读到系统应用",
            "沒有讀到系統應用程式",
            "No system apps read",
        ]))),
        Some(list) => {
            pages = list.len().div_ceil(per).max(1);
            let page = s.sys_page.min(pages - 1);
            let from = page * per;
            let to = (from + per).min(list.len());
            for (k, a) in list.iter().enumerate().take(to).skip(from) {
                let mut row = Row::new(&a.label).icon(SLATE, Icon::Layers);
                row.sub = Some(&a.package);
                // 状态：排队优先，其次已停用；没有就是一颗普通的行。
                let queued = s.app_state_queued.get(&a.package).copied();
                let disabled = s.app_disabled.as_ref().and_then(|m| m.get(&a.package)).copied().unwrap_or(false);
                let state = match queued {
                    Some(true) => Some(l.pick(["待停用", "待停用", "Will disable"])),
                    Some(false) => Some(l.pick(["待启用", "待啟用", "Will enable"])),
                    None if disabled => Some(l.pick(["已停用", "已停用", "Disabled"])),
                    None => None,
                };
                if let Some(text) = state {
                    row = row.trail(Trail::Text(text));
                }
                rows.push(row);
                acts.push(Action::SysAppToggle(k));
            }
        }
    }
    let page = s.sys_page.min(pages.saturating_sub(1));
    let ph = t.m.dp(widgets::PILL_DP);
    let pager_y = t.m.title_top + t.m.title_px * 0.46 - ph / 2.0;
    let mut out = Vec::new();
    if pages <= 1 {
        let only = l.pick(["只有一页", "只有一頁", "One page"]);
        let w = widgets::pill_width(&c, t, &f, only);
        let x = t.m.w as f32 - t.m.margin - w;
        widgets::pill(&mut c, t, &f, x, pager_y, only, t.pal.search, t.pal.text);
    } else {
        let gaps = t.m.dp(widgets::PILL_GAP_DP);
        let counter = format!("{}/{}", page + 1, pages);
        let back = l.pick(["上一页", "上一頁", "Newer"]);
        let fwd = l.pick(["下一页", "下一頁", "Older"]);
        let w_prev = widgets::pill_width(&c, t, &f, back);
        let w_next = widgets::pill_width(&c, t, &f, fwd);
        let w_counter = c.text_width(&f.value, &counter, t.m.sub_px) + t.m.dp(8.0);
        let x_next = t.m.w as f32 - t.m.margin - w_next;
        let x_counter = x_next - gaps - w_counter;
        let x_prev = x_counter - gaps - w_prev;
        let faint = t.pal.subtitle;
        if page + 1 < pages {
            out.push(Hit {
                rect: Rect::new(t.m.w as f32 * 0.66, pager_y - t.m.dp(12.0), t.m.w as f32 * 0.34 - t.m.margin, ph + t.m.dp(24.0)),
                action: Action::SysPage(1),
            });
        }
        if page > 0 {
            let wd = widgets::pill(&mut c, t, &f, x_prev, pager_y, back, t.pal.search, t.pal.text);
            let left = x_prev - t.m.dp(16.0);
            let right = (x_prev + wd + t.m.dp(8.0)).min(t.m.w as f32 * 0.62);
            out.push(Hit {
                rect: Rect::new(left, pager_y - t.m.dp(10.0), (right - left).max(wd), ph + t.m.dp(20.0)),
                action: Action::SysPage(-1),
            });
        } else {
            widgets::pill(&mut c, t, &f, x_prev, pager_y, back, t.pal.search, faint);
        }
        c.text_center(&f.value, &counter, x_counter + w_counter / 2.0, pager_y + ph / 2.0, t.m.sub_px, faint);
        if page + 1 < pages {
            widgets::pill(&mut c, t, &f, x_next, pager_y, fwd, t.pal.search, t.pal.text);
        } else {
            widgets::pill(&mut c, t, &f, x_next, pager_y, fwd, t.pal.search, faint);
        }
    }
    let listed = widgets::list(&mut c, t, f, top, &rows);
    out.extend(hits(&listed.rows, &acts));
    Frame::new(t, c, out, l.pick(SYSTEM_APPS), top)
}

const SYSTEM_APPS: Text = ["系统应用", "系統應用程式", "System apps"];

/// One app: what it is, when it came, how much room it takes, and the way to
/// take it off.
pub fn data_app_info(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let top = t.m.content_top;
    let at = s.app_at.unwrap_or(0);
    let picked = match &s.apps {
        Some(Ok(list)) => list.get(at),
        _ => None,
    };
    let Some(a) = picked else {
        return Frame::new(t, c, Vec::new(), l.pick(DATA_APPS), top);
    };
    let facts: Vec<(&str, String)> = vec![
        ("包名", a.package.clone()),
        ("名称", a.label.clone()),
        ("位置", a.dir.clone()),
        ("占用", crate::fmt::file_size(a.size)),
        ("安装", crate::fmt::datetime(l, a.installed, s.zone())),
        ("更新", crate::fmt::datetime(l, a.updated, s.zone())),
    ];
    let y = widgets::info_card(&mut c, t, f, top, &facts);
    // 系统分区的应用不从这里删：删了系统起不来。
    let uninstall = if a.system {
        Row::new(l.pick(["系统应用，不能这样卸载", "系統應用程式，不能這樣卸載", "A system app: not removed here"])).icon(SLATE, Icon::Shield)
    } else {
        Row::new(l.pick(["卸载应用", "卸載應用程式", "Uninstall"])).icon(RED, Icon::Wipe).tint(t.pal.danger)
    };
    let acts = widgets::list(&mut c, t, f, y + t.m.card_gap, &[uninstall]);
    let out = if a.system {
        hits(&acts.rows, &[Action::Unavailable(l.pick(["系统应用不能这样卸载", "系統應用程式不能這樣卸載", "A system app is not removed here"]))])
    } else {
        hits(&acts.rows, &[Action::DataAppRemove(at)])
    };
    Frame::new(t, c, out, l.pick(DATA_APPS), top)
}

/// Giving the data partition's files the SELinux labels the installed
/// system's rules give them: what it does, and a swipe to start it; then
/// how far it has come, a way to stop it, and how it ended.
pub fn relabel(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let title = l.pick(texts::page::RELABEL);
    let Some(job) = &s.relabel else {
        let mut y = widgets::paragraph(&mut c, t, f, top, l.pick(texts::relabel::WHAT));
        let note = match s.data {
            Data::Open | Data::Locked { .. } => None,
            Data::Erased => Some(texts::relabel::DATA_ERASED),
            Data::Closed | Data::Opening | Data::Failed(_) => Some(texts::relabel::OPEN_FIRST),
        };
        if let Some(note) = note {
            y = widgets::footnote(&mut c, t, f, y, l.pick(note));
        }
        let (label, danger) = confirm_label(Confirm::Relabel, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
        out.push(Hit { rect: track, action: Action::Confirm(Confirm::Relabel) });
        return Frame::new(t, c, out, title, top);
    };
    let repair = Verb::of(&crate::app::PartKind::Repair);
    let heading = match &job.outcome {
        None => repair.going(l),
        Some(JobEnd::Done) => repair.complete(l),
        Some(JobEnd::Stopped) => l.pick(texts::common::STOPPED).into(),
        Some(JobEnd::Failed(_)) => repair.failed(l),
    };
    let detail = match (job.total, job.checked) {
        (0, _) if job.outcome.is_none() => l.pick(texts::installing::GETTING_READY).to_string(),
        (total, checked) => texts::relabel::checked(l, checked, total),
    };
    let y = widgets::progress_card(&mut c, t, f, top, &heading, &detail, Some(share(job.checked, job.total)));
    match &job.outcome {
        None => {
            let label = l.pick(if job.stopping { texts::common::STOPPING } else { texts::common::STOP });
            let stop = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(label).trail(Trail::None)]);
            if !job.stopping {
                out.extend(hits(&stop.rows, &[Action::StopRelabel]));
            }
        }
        Some(end) => {
            let changed = job.changed;
            let note = match end {
                JobEnd::Done if changed == 0 => l.pick(texts::relabel::NOTHING_NEEDED).to_string(),
                JobEnd::Done => texts::relabel::changed(l, changed),
                JobEnd::Stopped => texts::relabel::stopped(l, changed),
                JobEnd::Failed(why) => texts::relabel::failed(l, why),
            };
            let y = widgets::footnote(&mut c, t, f, y, &note);
            let finish = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::common::DONE)).trail(Trail::None)]);
            out.extend(hits(&finish.rows, &[Action::Back]));
        }
    }
    Frame { back: job.outcome.is_some(), ..Frame::new(t, c, out, title, top) }
}

/// The partitions whose filesystems can be repaired or made afresh as
/// another, each with what it holds and its size.
pub fn filesystems(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::mount::PARTITIONS));
    let bottom = match &s.filesystems {
        None => widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::filesystems::READING)).trail(Trail::None)]).bottom,
        Some(Err(why)) => {
            let y = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::mount::UNREADABLE)).trail(Trail::None)]).bottom;
            widgets::footnote(&mut c, t, f, y, why)
        }
        Some(Ok(rows)) if rows.is_empty() => widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::filesystems::NONE)).trail(Trail::None)]).bottom,
        Some(Ok(rows)) => {
            let subs: Vec<String> = rows.iter().map(|r| format!("{} · {}", fs_held(l, r, &s.data), fmt::bytes(r.size))).collect();
            let shown: Vec<Row> = rows.iter().zip(&subs).map(|(r, sub)| Row::new(&r.name).subtitle(sub)).collect();
            let list = widgets::list(&mut c, t, f, y, &shown);
            out.extend(hits(&list.rows, &(0..rows.len()).map(Action::OpenFilesystem).collect::<Vec<_>>()));
            widgets::footnote(&mut c, t, f, list.bottom, l.pick(texts::filesystems::NOTE))
        }
    };
    Frame::new(t, fit(c, t, bottom), out, l.pick(texts::page::FILESYSTEMS), top)
}

/// 分区管理：先把常做的几件事摆在上面，再把分区表连同大小列出来
/// （2026-11-19，照 qingzhengdoufu 的分区管理页合并；导出的是分区原样镜像）。
pub fn partitions(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let rows = [
        Row::new(l.pick(["\u{5237}\u{5165}\u{5206}\u{533a}\u{955c}\u{50cf}", "\u{5237}\u{5165}\u{5206}\u{5272}\u{5340}\u{6620}\u{50cf}\u{6a94}", "Flash a partition image"])).icon(BLUE, Icon::Install),
        Row::new(l.pick(["\u{6587}\u{4ef6}\u{7cfb}\u{7edf}\u{7ba1}\u{7406}", "\u{6a94}\u{6848}\u{7cfb}\u{7d71}\u{7ba1}\u{7406}", "Filesystems"])).icon(PURPLE, Icon::Mount),
        Row::new(l.pick(["\u{6302}\u{8f7d}\u{7ba1}\u{7406}", "\u{6302}\u{8f09}\u{7ba1}\u{7406}", "Mounts"])).icon(BLUE, Icon::Layers),
        Row::new(l.pick(IMAGE_BACKUP)).icon(GREEN, Icon::Backup),
    ];
    let list = widgets::list(&mut c, t, f, top, &rows);
    out.extend(hits(
        &list.rows,
        &[
            Action::Open(Screen::Choose(Place::Storage)),
            Action::Open(Screen::Filesystems),
            Action::Open(Screen::Mount),
            Action::Open(Screen::ImageBackup),
        ],
    ));
    let mut y = list.bottom + t.m.card_gap;
    y = widgets::section(&mut c, t, f, y, l.pick(["\u{5206}\u{533a}", "\u{5206}\u{5272}\u{5340}", "Partitions"]));
    match &s.partitions {
        Some(Ok(list_rows)) => {
            let names: Vec<String> = list_rows.iter().map(|(name, _)| name.clone()).collect();
            let sizes: Vec<String> = list_rows.iter().map(|(_, size)| fmt::bytes(*size)).collect();
            let shown: Vec<Row> = names
                .iter()
                .zip(&sizes)
                .enumerate()
                .map(|(k, (name, size))| {
                    let row = Row::new(name).icon(PURPLE, Icon::Layers);
                    if s.dump_pick == Some(k) { row.trail(Trail::Check(true)) } else { row.trail(Trail::Text(size)) }
                })
                .collect();
            let pick = widgets::list(&mut c, t, f, y, &shown);
            out.extend(hits(&pick.rows, &(0..names.len()).map(Action::PickDump).collect::<Vec<_>>()));
            y = pick.bottom + t.m.card_gap;
            match s.dump_pick.and_then(|k| names.get(k).cloned()) {
                Some(name) => {
                    let note = tr!(l, "导出「{name}」到内部存储", "匯出「{name}」到內部儲存", "Export \u{201c}{name}\u{201d} to internal storage");
                    let one = widgets::list(&mut c, t, f, y, &[Row::new(&note).icon(ORANGE, Icon::Backup)]);
                    out.extend(hits(&one.rows, &[Action::DumpNow]));
                    y = one.bottom + t.m.card_gap;
                }
                None => {
                    y = widgets::footnote(&mut c, t, f, y, l.pick(["点一个分区选中它，再导出成镜像。", "點一個分割區選中它，再匯出成映像檔。", "Tap a partition to choose it, then export it as an image."]));
                }
            }
        }
        Some(Err(why)) => y = widgets::footnote(&mut c, t, f, y, why),
        None => y = widgets::footnote(&mut c, t, f, y, l.pick(["正在读取", "正在讀取", "Reading"])),
    }
    Frame::new(t, fit(c, t, y), out, l.pick(["分区管理", "分割區管理", "Partitions"]), top)
}

/// What a partition holds, as its row and page name it: the data
/// partition's is read only once it is open, and until then the row says
/// where it stands, as the home page does.
fn fs_held(l: Lang, row: &FsRow, data: &Data) -> String {
    match (&row.filesystem, row.name == DATA_PARTITION) {
        (Some(filesystem), _) => filesystem.clone(),
        (None, true) => fmt::data(l, data).into(),
        (None, false) => l.pick(texts::filesystems::UNRECOGNIZED).into(),
    }
}

/// One partition: what it holds, what can be done to it, and a swipe to do
/// what is chosen.
pub fn filesystem(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let Some(Ok(rows)) = &s.filesystems else { unreachable!("a partition's page opens from the list read") };
    let row = &rows[s.fs_chosen];
    let data = row.name == DATA_PARTITION;
    let facts = [
        (l.pick(texts::filesystems::MOUNT_POINT), row.mount_point.clone()),
        (l.pick(texts::filesystems::FILESYSTEM), fs_held(l, row, &s.data)),
        (l.pick(texts::common::SIZE), fmt::bytes(row.size)),
    ];
    let y = widgets::info_card(&mut c, t, f, top, &facts);
    let y = widgets::section(&mut c, t, f, y, l.pick(texts::common::ACTIONS));
    let chosen = |action: &FsAction| s.fs_action == *action;
    let remakes: Vec<String> = row.others.iter().map(|fs| texts::filesystems::change_to(l, fs)).collect();
    let made: Vec<String> = row.others.iter().map(|_| remade(row, s)).collect();
    let repair = Row::new(l.pick(texts::filesystems::REPAIR))
        .subtitle(l.pick(texts::filesystems::REPAIR_NOTE))
        .trail(Trail::Check(chosen(&FsAction::Repair)));
    let mut shown = vec![repair];
    let mut actions = vec![Action::ChooseFs(FsChoice::Repair)];
    for (k, fs) in row.others.iter().enumerate() {
        shown.push(Row::new(&remakes[k]).subtitle(&made[k]).trail(Trail::Check(chosen(&FsAction::Remake(fs.clone())))));
        actions.push(Action::ChooseFs(FsChoice::Remake(k)));
    }
    let list = widgets::list(&mut c, t, f, y, &shown);
    out.extend(hits(&list.rows, &actions));
    let mut y = list.bottom;
    if row.others.is_empty() {
        y = widgets::footnote(&mut c, t, f, y, l.pick(texts::filesystems::CANT_CHANGE));
    }
    if data {
        y = widgets::footnote(&mut c, t, f, y, l.pick(texts::filesystems::DATA_CLOSES));
    }
    if !row.others.is_empty() {
        y = widgets::footnote(&mut c, t, f, y, l.pick(texts::filesystems::CANT_UNDO));
    }
    let (label, danger) = confirm_label(Confirm::Filesystem, s);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::Filesystem) });
    Frame::new(t, fit(c, t, track.y + track.h), out, partition_title(l, &row.name), top)
}

/// The title of the page of a partition the filesystem page lists.
fn partition_title(l: Lang, name: &str) -> &'static str {
    l.pick(match name {
        DATA_PARTITION => texts::wipe::part::DATA,
        "metadata" => texts::wipe::part::METADATA,
        "cache" => texts::wipe::part::CACHE,
        _ => texts::flash::PARTITION,
    })
}

/// What making `row` afresh as another filesystem erases: the data
/// partition goes as a factory reset goes, with /metadata and the cache
/// partition beside it.
fn remade(row: &FsRow, s: &State) -> String {
    match (row.name == DATA_PARTITION, s.cache_partition) {
        (true, true) => s.lang.pick(texts::filesystems::FORMATS_DATA_AND_CACHE).into(),
        (true, false) => s.lang.pick(texts::filesystems::FORMATS_DATA).into(),
        (false, _) => texts::filesystems::formats(s.lang, &row.name),
    }
}

/// Work on whole partitions: each partition and where it stands, what the
/// program said of the last, and once it has ended, how. What the system
/// asked for leaves on its own once it went through every partition, and
/// offers where to go otherwise; what the user chose goes back to its page.
pub fn part_work(cx: &Ctx) -> Frame {
    use crate::app::{ItemEnd, ItemState, PartKind};
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let work = s.part_work.as_ref().expect("the page of partition work shows work");
    let mut c = long_page(cx);
    let (mut out, mut rows) = (Vec::new(), None);
    let top = t.m.content_top;
    let verb = Verb::of(&work.kind);
    // Work on one partition fails; work through several may get partway.
    let (title, several) = match &work.kind {
        PartKind::Disable => (texts::parts::title::WIPE, true),
        PartKind::Unshare => (texts::parts::title::UNSHARE, true),
        PartKind::Repair => (texts::parts::title::REPAIR, false),
        PartKind::Remake(_) => (texts::parts::title::FORMAT, false),
    };
    let failed = |state: &ItemState| matches!(state, ItemState::Ended(ItemEnd::Failed(_)));
    let short = matches!(work.outcome, Some(Err(_))) || work.items.iter().any(|(_, state)| failed(state));
    let heading = match (&work.outcome, short, several) {
        (None, _, _) => verb.going(l),
        (Some(_), false, _) => verb.complete(l),
        (Some(_), true, true) => verb.incomplete(l),
        (Some(_), true, false) => verb.failed(l),
    };
    let mut y = widgets::section_first(&mut c, t, f, top, &heading);
    if !work.items.is_empty() {
        let states: Vec<String> = work
            .items
            .iter()
            .map(|(_, state)| match state {
                ItemState::Waiting if work.outcome.is_some() => verb.not_done(l),
                ItemState::Waiting => l.pick(texts::common::WAITING).into(),
                ItemState::Working => verb.going(l),
                ItemState::Ended(end) => fmt::item_end(l, end, verb),
            })
            .collect();
        let held: Vec<Option<&str>> = work
            .items
            .iter()
            .map(|(_, state)| match state {
                ItemState::Ended(ItemEnd::NotShared(fs)) if !fs.is_empty() => Some(fs.as_str()),
                _ => None,
            })
            .collect();
        let shown: Vec<Row> = work
            .items
            .iter()
            .zip(&states)
            .zip(&held)
            .map(|(((name, _), state), held)| {
                let row = Row::new(name).trail(Trail::Text(state));
                match held {
                    Some(fs) => row.subtitle(fs),
                    None => row,
                }
            })
            .collect();
        y = widgets::list(&mut c, t, f, y, &shown).bottom;
    }
    let leaving = match s.asked.shutdown_after {
        true => texts::parts::leaving::POWER_OFF,
        false => texts::parts::leaving::REBOOT,
    };
    let mut notes: Vec<String> = Vec::new();
    match &work.outcome {
        None if work.asked => notes.push(l.pick(leaving).into()),
        None => notes.push(verb.hold_on(l)),
        Some(Err(stop)) => {
            let (what, detail) = fmt::part_stop(l, stop, &cx.info.codename);
            notes.push(what);
            notes.extend(detail);
        }
        Some(Ok(())) => {}
    }
    for (name, state) in &work.items {
        if let ItemState::Ended(ItemEnd::Failed(Some(why))) = state {
            notes.push(texts::common::with_reason(l, verb.failed_on(l, name), why));
        }
    }
    for note in &notes {
        y = widgets::footnote(&mut c, t, f, y, note);
    }
    if let (true, Some(reason)) = (work.asked, &s.asked.reason) {
        y = widgets::info_card(&mut c, t, f, y + t.m.dp(12.0), &[(l.pick(texts::installing::REASON), reason.clone())]);
    }
    match (&work.outcome, work.asked) {
        (None, _) => {}
        (Some(_), true) if !short => {}
        // The system's work ran without the slot's partitions, which going
        // on in the recovery brings up first, as leaving fastboot does.
        (Some(_), true) => {
            let below = widgets::section(&mut c, t, f, y, l.pick(texts::common::ACTIONS));
            let rows: Vec<Row> = AFTERWARDS.iter().map(|&(title, _)| Row::new(l.pick(title)).trail(Trail::None)).collect();
            let actions = AFTERWARDS.map(|(_, action)| if action == Action::Dismiss { Action::EnterRecovery } else { action });
            let choices = widgets::list(&mut c, t, f, below, &rows);
            out.extend(hits(&choices.rows, &actions));
            y = choices.bottom;
        }
        (Some(_), false) => {
            let finish = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::common::DONE)).trail(Trail::None)]);
            out.extend(hits(&finish.rows, &[Action::Back]));
            y = finish.bottom;
        }
    }
    if !work.output.is_empty() {
        let below = widgets::section(&mut c, t, f, y, l.pick(texts::parts::OUTPUT));
        rows = Some(Rows { edges: widgets::output_edges(&c, t, f, below, &work.output), action: None });
        y = below;
    }
    let back = work.outcome.is_some() && !work.asked;
    Frame { rows, back, ..Frame::new(t, fit(c, t, y), out, l.pick(title), top) }
}

/// The system on the phone now; the newest system the phone maker's
/// server knows, and downloading its full package to install it; and
/// where an update the system installed stands, merging it into the slot it
/// went to: how far a merge under way has come, and a swipe to merge one
/// that waits there for the system to start it.
pub fn update(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut out = Vec::new();
    if let Standing::Merging(done) = s.update {
        let share = done as f32;
        let detail = texts::common::share_done(l, percent(share));
        let y = widgets::progress_card(&mut c, t, f, top, l.pick(texts::snapshot::MERGING), &detail, Some(share));
        let y = widgets::footnote(&mut c, t, f, y, l.pick(texts::update::MERGING_NOTE));
        return Frame::new(t, fit(c, t, y), out, l.pick(texts::page::UPDATE), top);
    }
    let mut y = top;
    if let Some(v) = &i.os_version {
        y = widgets::section_first(&mut c, t, f, y, l.pick(texts::common::THIS_PHONE));
        y = widgets::info_card(&mut c, t, f, y, &[(l.pick(texts::update::CURRENT_VERSION), v.clone())]);
    }
    if s.update_server {
        let online_update = l.pick(texts::update::ONLINE);
        y = if y == top { widgets::section_first(&mut c, t, f, y, online_update) } else { widgets::section(&mut c, t, f, y, online_update) };
        y = online(&mut c, cx, y, &mut out);
    }
    let next = match &s.boot_slots {
        Some(Ok(slots)) => Some(slots.active),
        _ => None,
    };
    let (title, detail) = fmt::update(l, &s.update, next);
    let color = match s.update {
        Standing::None => SLATE,
        Standing::Merged => GREEN,
        Standing::Failed(_) | Standing::Unknown => RED,
        _ => ORANGE,
    };
    let mut row = Row::new(title).icon(color, Icon::Layers).trail(Trail::None);
    if let Some(detail) = &detail {
        row = row.subtitle(detail);
    }
    let merging = l.pick(texts::update::MERGING);
    y = if y == top { widgets::section_first(&mut c, t, f, y, merging) } else { widgets::section(&mut c, t, f, y, merging) };
    y = widgets::list(&mut c, t, f, y, &[row]).bottom;
    if s.update == Standing::Waiting {
        let (label, danger) = confirm_label(Confirm::Merge, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
        out.push(Hit { rect: track, action: Action::Confirm(Confirm::Merge) });
        y = widgets::footnote(&mut c, t, f, track.y + track.h, l.pick(texts::update::CANT_GO_BACK));
    }
    Frame::new(t, fit(c, t, y), out, l.pick(texts::page::UPDATE), top)
}

/// Asking the update server for the newest system, then its full package:
/// downloading it, how far that has come, and installing it once it is
/// whole; then what to note before installing it.
fn online(c: &mut Canvas, cx: &Ctx, y: f32, out: &mut Vec<Hit>) -> f32 {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let release = match &s.check {
        Check::Asking => return widgets::list(c, t, f, y, &[Row::new(l.pick(texts::update::CHECKING)).trail(Trail::None)]).bottom,
        Check::Answered(Some(r)) => r,
        other => {
            let mut row = Row::new(l.pick(texts::update::CHECK));
            if !s.online() {
                row = row.subtitle(l.pick(texts::update::WLAN_FIRST));
            }
            let check = widgets::list(c, t, f, y, &[row]);
            out.extend(hits(&check.rows, &[Action::CheckUpdate]));
            return match other {
                Check::Failed(why) => widgets::footnote(c, t, f, check.bottom, &fmt::unanswered(l, why)),
                Check::Answered(None) => widgets::footnote(c, t, f, check.bottom, l.pick(texts::update::NONE_KNOWN)),
                _ => check.bottom,
            };
        }
    };
    let newer = i.os_version.as_deref() != Some(release.version.as_str());
    let title = if newer { texts::update::new_version_named(l, &release.version) } else { l.pick(texts::update::UP_TO_DATE).to_string() };
    let size = match release.length {
        Some(length) => fmt::file_size(length),
        None => fmt::package_size(&release.size),
    };
    let package = texts::update::full_package(l, size);
    let mut y = widgets::list(c, t, f, y, &[Row::new(&title).icon(GREEN, Icon::Layers).subtitle(&package).trail(Trail::None)]).bottom;
    match (&s.download, s.have) {
        (Some(job), _) if job.outcome.is_none() => {
            let (heading, detail, share) = match job.progress {
                None => (l.pick(texts::installing::GETTING_READY), String::new(), None),
                Some(p) => downloaded(l, p, job.rate),
            };
            y = widgets::progress_card(c, t, f, y + t.m.card_gap, heading, &detail, share);
            let label = l.pick(if job.stopping { texts::update::PAUSING } else { texts::update::PAUSE });
            let stop = widgets::list(c, t, f, y + t.m.card_gap, &[Row::new(label).trail(Trail::None)]);
            if !job.stopping {
                out.extend(hits(&stop.rows, &[Action::StopDownload]));
            }
            y = stop.bottom;
        }
        (_, Have::Whole(_)) => {
            let row = Row::new(l.pick(texts::page::INSTALL)).subtitle(l.pick(texts::update::DOWNLOADED_TO_STORAGE));
            let install = widgets::list(c, t, f, y + t.m.card_gap, &[row]);
            out.extend(hits(&install.rows, &[Action::InstallDownloaded]));
            y = install.bottom;
        }
        (job, have) => {
            match job.as_ref().and_then(|j| j.outcome.as_ref()) {
                Some(DownloadEnd::Failed(why)) => y = widgets::footnote(c, t, f, y, &fmt::download_failure(l, why)),
                Some(DownloadEnd::Stopped) => y = widgets::footnote(c, t, f, y, l.pick(texts::update::PAUSED)),
                _ => {}
            }
            let title = l.pick(if matches!(have, Have::Part(_)) { texts::update::RESUME } else { texts::update::DOWNLOAD });
            let had = match have {
                Have::Part(done) => Some(texts::update::downloaded(l, fmt::file_size(done))),
                _ => None,
            };
            let sub = if !s.data.storage_open() {
                Some(l.pick(texts::decrypt::NEEDED))
            } else if !s.online() {
                Some(l.pick(texts::update::WLAN_FIRST))
            } else {
                had.as_deref()
            };
            let row = Row::new(title);
            let download = widgets::list(c, t, f, y + t.m.card_gap, &[sub.map_or(row, |sub| row.subtitle(sub))]);
            out.extend(hits(&download.rows, &[Action::Download]));
            y = download.bottom;
        }
    }
    let saved = texts::update::saved_in(l, DOWNLOADS);
    let points = [l.pick(texts::update::KEEPS_DATA), saved.as_str(), l.pick(texts::update::RESUMABLE), l.pick(texts::update::OPTIMIZES)];
    widgets::notes(c, t, f, y + t.m.card_gap, l.pick(texts::common::NOTES), &points)
}

/// What a progress card says of a download, and the share of it done:
/// what an earlier download left being checked, or the rest coming at
/// `rate` bytes a second.
fn downloaded(l: Lang, progress: Progress, rate: u64) -> (&'static str, String, Option<f32>) {
    match progress {
        Progress::Checking { done, total } => (l.pick(texts::update::CHECKING_PART), format!("{} / {}", fmt::file_size(done), fmt::file_size(total)), Some(share(done, total))),
        Progress::Downloading { done, total } => (
            l.pick(texts::update::DOWNLOADING),
            format!("{} / {} · {}/s", fmt::file_size(done), fmt::file_size(total), fmt::file_size(rate)),
            Some(share(done, total)),
        ),
    }
}

/// Skyward's own update: the build running now; Skyward's newest release
/// on GitHub, and updating to it; then what to note before updating.
pub fn skyward_update(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut out = Vec::new();
    let y = widgets::section_first(&mut c, t, f, top, l.pick(["本机", "本機", "This phone"]));
    let y = widgets::info_card(&mut c, t, f, y, &[(l.pick(["当前版本", "目前版本", "Current version"]), RUNNING.to_string())]);
    let y = widgets::section(&mut c, t, f, y, l.pick(["在线更新", "線上更新", "Online update"]));
    let y = skyward_online(&mut c, cx, y, &mut out);
    // What the release says it changed, cut to the few lines that fit here.
    if let Some(newest) = cx.state.skyward_newest()
        && !newest.notes.is_empty()
    {
        let points: Vec<&str> = newest.notes.iter().map(String::as_str).collect();
        let y2 = widgets::notes(&mut c, t, f, y + t.m.card_gap, l.pick(["本次更新", "本次更新", "What changed"]), &points);
        return Frame::new(t, fit(c, t, y2), out, l.pick(SKYWARD_UPDATE), top);
    }
    let points = [
        l.pick([
            "新版本从 GitHub 下载，校验 SHA-256 后写入 recovery 分区，不会改动系统和用户数据。",
            "新版本從 GitHub 下載，驗證 SHA-256 後寫入 recovery 分割區，不會改動系統和使用者資料。",
            "The new version is downloaded from GitHub and goes into the recovery partitions once its SHA-256 checks out. The system and user data are left as they are.",
        ]),
        l.pick([
            "新版本将写入下次启动所用槽位的 recovery 分区；另一个槽位装有 Skyward 时，也会一并更新。",
            "新版本將寫入下次啟動所用槽位的 recovery 分割區；另一個槽位裝有 Skyward 時，也會一併更新。",
            "The new version goes into the recovery partition of the slot that starts next, and into the other slot's where it holds Skyward.",
        ]),
        l.pick(["更新完成后，重启到恢复模式即可使用新版本。", "更新完成後，重新啟動到復原模式即可使用新版本。", "Once updated, reboot to recovery to use the new version."]),
    ];
    let y = widgets::notes(&mut c, t, f, y + t.m.card_gap, l.pick(NOTES), &points);
    Frame::new(t, fit(c, t, y), out, l.pick(SKYWARD_UPDATE), top)
}

/// Asking GitHub for Skyward's newest release, then updating to it: its
/// build for the phone downloaded and written into the recovery partitions,
/// how far that has come, and how it ended.
fn skyward_online(c: &mut Canvas, cx: &Ctx, y: f32, out: &mut Vec<Hit>) -> f32 {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let gap = t.m.card_gap;
    let newest = match &s.skyward_check {
        SkywardCheck::Asking => return widgets::list(c, t, f, y, &[Row::new(l.pick(texts::update::CHECKING)).trail(Trail::None)]).bottom,
        SkywardCheck::Answered(n) => n,
        other => {
            let mut row = Row::new(l.pick(texts::update::CHECK));
            if !s.online() {
                row = row.subtitle(l.pick(texts::update::WLAN_FIRST));
            }
            let check = widgets::list(c, t, f, y, &[row]);
            out.extend(hits(&check.rows, &[Action::CheckSkyward]));
            return match other {
                SkywardCheck::Failed(why) => widgets::footnote(c, t, f, check.bottom, &fmt::unanswered(l, why)),
                _ => check.bottom,
            };
        }
    };
    let version = &newest.version;
    let (title, detail) = match (newest.newer, &newest.build) {
        (true, Some(build)) => (texts::update::new_version_named(l, version), texts::skyward_update::package(l, fmt::file_size(build.size))),
        (true, None) => (texts::update::new_version_named(l, version), l.pick(texts::skyward_update::NO_PACKAGE).to_string()),
        (false, _) => (l.pick(texts::update::UP_TO_DATE).to_string(), texts::skyward_update::newest_is(l, version)),
    };
    let y = widgets::list(c, t, f, y, &[Row::new(&title).icon(GREEN, Icon::Install).subtitle(&detail).trail(Trail::None)]).bottom;
    if !newest.newer || newest.build.is_none() {
        return y;
    }
    match &s.skyward_job {
        Some(job) if job.outcome.is_none() => {
            let (heading, detail, share) = match job.stage {
                None => (l.pick(texts::installing::GETTING_READY), String::new(), None),
                Some(SkywardStage::Download(p)) => downloaded(l, p, job.rate),
                Some(SkywardStage::Write { done, total }) => {
                    let part = share(done, total);
                    (l.pick(texts::skyward_update::WRITING), texts::common::share_done(l, percent(part)), Some(part))
                }
            };
            let y = widgets::progress_card(c, t, f, y + gap, heading, &detail, share);
            if matches!(job.stage, Some(SkywardStage::Write { .. })) {
                return widgets::footnote(c, t, f, y, l.pick(texts::skyward_update::KEEP_POWER));
            }
            let stop = widgets::list(c, t, f, y + gap, &[Row::new(l.pick(if job.stopping { texts::common::STOPPING } else { texts::common::STOP })).trail(Trail::None)]);
            if !job.stopping {
                out.extend(hits(&stop.rows, &[Action::StopSkywardUpdate]));
            }
            stop.bottom
        }
        Some(SkywardJob { outcome: Some(SkywardEnd::Updated(slots)), .. }) => {
            let detail = match slots.as_slice() {
                [slot] => texts::skyward_update::in_one(l, version, slot.letter()),
                [a, b] => texts::skyward_update::in_two(l, version, a.letter(), b.letter()),
                other => unreachable!("a phone has two slots, and Skyward went into {other:?}"),
            };
            let y = widgets::list(c, t, f, y + gap, &[standing(l.pick(texts::skyward_update::SUCCEEDED), GREEN, Icon::Install, &detail)]).bottom;
            let restart = widgets::list(c, t, f, y + gap, &[Row::new(l.pick(texts::installed::REBOOT_TO_RECOVERY)).icon(ORANGE, Icon::Power)]);
            out.extend(hits(&restart.rows, &[Action::RebootTo(Target::Recovery)]));
            restart.bottom
        }
        job => {
            let mut y = y;
            match job.as_ref().and_then(|j| j.outcome.as_ref()) {
                Some(SkywardEnd::Stopped) => y = widgets::footnote(c, t, f, y, l.pick(texts::skyward_update::STOPPED)),
                Some(SkywardEnd::Download(why)) => y = widgets::footnote(c, t, f, y, &fmt::skyward_download_failure(l, why)),
                Some(SkywardEnd::NotWritten(why)) => {
                    y = widgets::footnote(c, t, f, y, l.pick(texts::skyward_update::NOT_WRITTEN));
                    y = widgets::footnote(c, t, f, y, why);
                }
                Some(SkywardEnd::WriteFailed(slots, why)) => {
                    let warning = match slots.as_slice() {
                        [slot] => texts::skyward_update::partly_one(l, slot.letter()),
                        [a, b] => texts::skyward_update::partly_two(l, a.letter(), b.letter()),
                        other => unreachable!("a phone has two slots, and Skyward was written to {other:?}"),
                    };
                    y = widgets::footnote(c, t, f, y, &warning);
                    y = widgets::footnote(c, t, f, y, why);
                }
                Some(SkywardEnd::Updated(_)) => unreachable!("an update that went in shows how it did"),
                None => {}
            }
            let mut row = Row::new(l.pick(texts::skyward_update::DOWNLOAD_AND_UPDATE));
            if !s.online() {
                row = row.subtitle(l.pick(texts::update::WLAN_FIRST));
            }
            let update = widgets::list(c, t, f, y + gap, &[row]);
            out.extend(hits(&update.rows, &[Action::UpdateSkyward]));
            update.bottom
        }
    }
}

/// The WLAN switch; then the network chosen and how joining it goes, and
/// the networks in range, strongest first.
pub fn wlan(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let w = s.wlan.as_ref().expect("the WLAN page shows on a phone whose WLAN is brought up");
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let on = !matches!(w.power, Power::Off | Power::Failed(_));
    // The WLAN's switch, and the two settings that go with it: opened at
    // start, and the password kept beside the network it was joined with.
    let switch = widgets::list(&mut c, t, f, top, &[
        Row::new(l.pick(WLAN)).trail(Trail::Switch(on)),
        Row::new(l.pick(WLAN_AUTO)).trail(Trail::Switch(s.wlan_auto)),
        Row::new(l.pick(WLAN_KEEP)).trail(Trail::Switch(s.wlan_keep)),
    ]);
    out.extend(hits(&switch.rows, &[Action::Toggle(Switch::Wlan), Action::Toggle(Switch::WlanAuto), Action::Toggle(Switch::WlanKeep)]));
    let mut y = switch.bottom;
    // 手动 IP 的入口，和现在用的是哪一套。2026-11-27。
    let manual = widgets::list(
        &mut c,
        t,
        f,
        y + t.m.card_gap,
        &[Row::new(l.pick(texts::wlan::MANUAL_IP)).trail(Trail::Value(l.pick(if s.static_on {
            texts::wlan::MANUAL_IP_ON
        } else {
            texts::wlan::MANUAL_IP_OFF
        })))],
    );
    out.extend(hits(&manual.rows, &[Action::OpenWlanStatic]));
    y = manual.bottom;
    match &w.power {
        Power::Off => y = widgets::footnote(&mut c, t, f, y, l.pick(texts::wlan::TURN_ON_NOTE)),
        Power::Failed(why) => y = widgets::footnote(&mut c, t, f, y, &texts::wlan::cant_turn_on(l, why)),
        Power::Starting => y = widgets::footnote(&mut c, t, f, y, l.pick(texts::wlan::TURNING_ON_WLAN)),
        Power::On => {
            let locked = |s: Security| !matches!(s, Security::Open | Security::Owe);
            // The network chosen stands out in the accent, as the stock
            // page shows the one it is on.
            if let Some(j) = &w.joined {
                let found = w.networks.iter().find(|n| n.ssid == j.ssid);
                let (signal, bands) = match (&j.stage, found) {
                    (Joining::Connected(link), found) => (Some(link.signal), found.map_or(Bands::of(link.frequency), |n| n.bands)),
                    (_, Some(n)) => (Some(n.signal), n.bands),
                    (_, None) => (None, Bands::default()),
                };
                let tag = fmt::band_tag(bands);
                let mut row = Row::new(&j.name).fan(signal.map_or(0, fmt::signal_level)).subtitle(fmt::joining(l, &j.stage));
                row = row.trail(Trail::Network { locked: locked(j.security) });
                if let Some(tag) = &tag {
                    row = row.tag(tag);
                }
                let joined = widgets::list_toned(&mut c, t, f, y + t.m.card_gap, &[row], &Tone::lit(t));
                out.extend(hits(&joined.rows, &[Action::OpenJoined]));
                out.push(Hit { rect: widgets::round_button(t, joined.rows[0]), action: Action::OpenJoined });
                y = joined.bottom;
                if let Joining::Connected(link) = &j.stage
                    && let Some(note) = link.reach.as_ref().and_then(|r| fmt::reach(l, r))
                {
                    y = widgets::footnote(&mut c, t, f, y, &note);
                }
            }
            let others: Vec<(usize, &Network)> =
                w.networks.iter().enumerate().filter(|(_, n)| w.joined.as_ref().is_none_or(|j| j.ssid != n.ssid)).collect();
            // 记住的那些网：在哪里就连哪里，点一下用记住的密码连它；已连的那张副标题写「已连接」。
            if !s.wlan_saved.is_empty() {
                let on = w.joined.as_ref().map(|j| j.ssid.clone());
                let head = l.pick(["已保存的网络", "已保存的網路", "Saved networks"]);
                y = widgets::section(&mut c, t, f, y + t.m.card_gap, head);
                let rows: Vec<Row> = s
                    .wlan_saved
                    .iter()
                    .map(|n| {
                        let mut row = Row::new(&n.name).trail(Trail::Network { locked: locked(n.security) });
                        if on.as_deref() == Some(n.ssid.as_slice()) {
                            row.sub = Some(l.pick(["已连接", "已連線", "Connected"]));
                        }
                        row
                    })
                    .collect();
                let kept = widgets::list(&mut c, t, f, y, &rows);
                out.extend(hits(&kept.rows, &(0..s.wlan_saved.len()).map(Action::JoinKept).collect::<Vec<_>>()));
                y = kept.bottom;
            }
            let (below, rescan) = widgets::section_action(&mut c, t, f, y, l.pick(texts::wlan::NEARBY), l.pick(texts::wlan::REFRESH));
            out.push(Hit { rect: rescan, action: Action::Rescan });
            y = below;
            if others.is_empty() {
                y = widgets::footnote(&mut c, t, f, y - t.m.dp(10.0), l.pick(texts::wlan::LOOKING)) + t.m.card_gap;
            }
            let tags: Vec<Option<String>> = others.iter().map(|(_, n)| fmt::band_tag(n.bands)).collect();
            let mut rows: Vec<Row> = others
                .iter()
                .zip(&tags)
                .map(|((_, n), tag)| {
                    let row = Row::new(&n.name).fan(fmt::signal_level(n.signal)).trail(Trail::Network { locked: locked(n.security) });
                    match tag {
                        Some(tag) => row.tag(tag),
                        None => row,
                    }
                })
                .collect();
            // The list ends with adding a network by hand, as the stock one
            // does.
            rows.push(Row::new(l.pick(texts::wlan::ADD_NETWORK)).tint(t.pal.accent).trail(Trail::None));
            let list = widgets::list(&mut c, t, f, y, &rows);
            let mut picks: Vec<Action> = others.iter().map(|(k, _)| Action::JoinNetwork(*k)).collect();
            picks.push(Action::AddNetwork);
            out.extend(hits(&list.rows, &picks));
            // The round buttons come after the rows they sit on, and a touch
            // finds the last control under it.
            for (rect, (k, _)) in list.rows.iter().zip(&others) {
                out.push(Hit { rect: widgets::round_button(t, *rect), action: Action::NetworkDetails(*k) });
            }
            y = list.bottom;
        }
    }
    Frame::new(t, fit(c, t, y), out, l.pick(texts::page::WLAN), top)
}

/// The password of the network chosen: its name, what is typed, a switch
/// to show it, and the keyboard.
pub fn wlan_password(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let network = s.joining.as_ref().expect("the password page is for a network");
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, &network.name);
    let shown = if s.show_password { s.password.clone() } else { "•".repeat(s.password.chars().count()) };
    let (y, caret) = widgets::text_field(&mut c, t, f, y, &shown);
    let show = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::wlan::SHOW_PASSWORD)).trail(Trail::Switch(s.show_password))]);
    out.extend(hits(&show.rows, &[Action::Toggle(Switch::ShowPassword)]));
    keyboard_hits(&mut c, t, f, s, l.pick(texts::wlan::CONNECT), &mut out);
    Frame { caret: Some(caret), ..Frame::new(t, c, out, l.pick(texts::wlan::ENTER_PASSWORD), top) }
}

/// A network added by hand: its name, how it is protected, its password
/// where it takes one with the switch that shows it, and the keyboard,
/// which types into the field touched. The protections open over the page
/// from their row, which alone then answers a touch.
pub fn wlan_add(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let a = s.adding.as_ref().expect("the page adds a network");
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let field = |y: f32| Rect::new(m.margin, y, m.w as f32 - m.margin * 2.0, m.dp(56.0));
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::wlan::NETWORK_NAME));
    let (below, mut caret) = widgets::text_field(&mut c, t, f, y, &a.name);
    out.push(Hit { rect: field(y), action: Action::TypeInto(Field::Name) });
    // Held down while its list is open, as the stock row stays.
    let tone = if a.choosing { Tone::page(&t.pressed()) } else { Tone::page(t) };
    let protection = [Row::new(l.pick(texts::wlan::SECURITY_TITLE)).trail(Trail::Choice(fmt::security(l, a.security)))];
    let security = widgets::list_toned(&mut c, t, f, below + m.card_gap, &protection, &tone);
    out.extend(hits(&security.rows, &[Action::ChooseSecurity]));
    if a.security.password() {
        let y = widgets::section(&mut c, t, f, security.bottom, l.pick(texts::wlan::PASSWORD));
        let shown = if s.show_password { s.password.clone() } else { "•".repeat(s.password.chars().count()) };
        let (below, typed) = widgets::text_field(&mut c, t, f, y, &shown);
        out.push(Hit { rect: field(y), action: Action::TypeInto(Field::Password) });
        if a.field == Field::Password {
            caret = typed;
        }
        let show = widgets::list(&mut c, t, f, below + m.card_gap, &[Row::new(l.pick(texts::wlan::SHOW_PASSWORD)).trail(Trail::Switch(s.show_password))]);
        out.extend(hits(&show.rows, &[Action::Toggle(Switch::ShowPassword)]));
    }
    keyboard_hits(&mut c, t, f, s, l.pick(texts::wlan::CONNECT), &mut out);
    if !a.choosing {
        return Frame { caret: Some(caret), ..Frame::new(t, c, out, l.pick(texts::wlan::ADD_NETWORK), top) };
    }
    // A touch anywhere off the list closes it.
    out.clear();
    out.push(Hit { rect: Rect::new(0.0, 0.0, m.w as f32, m.h as f32), action: Action::CloseChoices });
    let names = ADDABLE.map(|p| fmt::security(l, p));
    let chosen = ADDABLE.iter().position(|&p| p == a.security);
    let held = ADDABLE.iter().position(|&p| s.held == Some(Action::PickSecurity(p)));
    let list = widgets::choices(&mut c, t, f, security.rows[0], &names, chosen, held);
    out.extend(hits(&list, &ADDABLE.map(Action::PickSecurity)));
    Frame::new(t, c, out, l.pick(texts::wlan::ADD_NETWORK), top)
}

/// 手动 IP 页：地址、掩码、网关、DNS 四格，键盘，「改回 DHCP」那一行。
/// 2026-11-27。
pub fn wlan_static(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let gap = m.dp(10.0);
    let cw = (m.w as f32 - m.margin * 2.0 - gap) / 2.0;
    let label_h = m.dp(20.0);
    let mut caret = None;
    // 一行两格：地址|掩码，网关|DNS。四行竖排会顶到键盘上，装不下
    // （2026-11-27，真机 DNS 那格被键盘盖住）。
    let cells: [(Field, &'static str, &str); 4] = [
        (Field::Address, l.pick(texts::wlan::ADDRESS), &s.static_ip.address),
        (Field::Mask, l.pick(texts::wlan::MASK), &s.static_ip.mask),
        (Field::Gateway, l.pick(texts::wlan::GATEWAY), &s.static_ip.gateway),
        (Field::Dns, l.pick(texts::wlan::DNS), &s.static_ip.dns),
    ];
    let mut rows_bottom = top;
    for pair in cells.chunks(2) {
        let y = rows_bottom + label_h;
        for (k, (kind, label, value)) in pair.iter().enumerate() {
            let x = m.margin + k as f32 * (cw + gap);
            c.text_top(&f.label, label, x + m.inner, rows_bottom, m.label_px, t.pal.section, None);
            let (_, typed) = widgets::text_field_in(&mut c, t, f, x, y, cw, value);
            if s.static_field == *kind {
                caret = Some(typed);
            }
            out.push(Hit { rect: Rect::new(x, y, cw, m.dp(56.0)), action: Action::TypeInto(*kind) });
        }
        rows_bottom = y + m.dp(56.0) + gap;
    }
    // 两种模式：用这组地址，或自动获取。勾在现在用的那个上，点另一边就换
    // 过去；点已经勾着的那边什么都不做（2026-11-27，开关语义不清）。
    let rows = [
        Row::new(l.pick(texts::wlan::USE_MANUAL)).trail(Trail::Check(s.static_on)),
        Row::new(l.pick(texts::wlan::MANUAL_IP_OFF)).trail(Trail::Check(!s.static_on)),
    ];
    let list = widgets::list(&mut c, t, f, rows_bottom + m.dp(2.0), &rows);
    out.extend(hits(&list.rows, &[Action::ApplyStatic, Action::UseDhcp]));
    widgets::footnote(&mut c, t, f, list.bottom, l.pick(texts::wlan::MANUAL_IP_NOTE));
    keyboard_hits(&mut c, t, f, s, l.pick(texts::wlan::APPLY), &mut out);
    Frame { caret, ..Frame::new(t, c, out, l.pick(texts::wlan::MANUAL_IP), top) }
}

/// A network's details, in the stock page's grid: for the one chosen, how
/// joining it goes and what it gave, and forgetting it; for any other, how
/// strong and how protected it is.
pub fn wlan_network(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let network = s.details.as_ref().expect("the details page is of a network");
    let joined = s.wlan.as_ref().and_then(|w| w.joined.as_ref()).filter(|j| j.ssid == network.ssid);
    let y = widgets::section_first(&mut c, t, f, top, &network.name);
    let detail = |icon, title, value: String| Detail { icon, title, value };
    let security = detail(Outline::Shield, l.pick(texts::wlan::SECURITY_TITLE), fmt::security(l, network.security).to_string());
    let signal = l.pick(texts::wlan::SIGNAL_STRENGTH);
    let title = l.pick(texts::wlan::DETAILS);
    let Some(j) = joined else {
        let items = [
            detail(Outline::Wifi, l.pick(texts::wlan::BANDS), fmt::bands(l, network.bands)),
            detail(Outline::Signal, signal, fmt::signal(l, network.signal).to_string()),
            security,
        ];
        let y = widgets::detail_grid(&mut c, t, f, y, &items);
        return Frame::new(t, fit(c, t, y), out, title, top);
    };
    let mut items = vec![detail(Outline::Info, l.pick(texts::wlan::STATUS), fmt::joining(l, &j.stage).to_string())];
    match &j.stage {
        Joining::Connected(link) => {
            if let Some(n) = link.generation {
                items.push(detail(Outline::Wifi, l.pick(texts::wlan::STANDARD), fmt::generation(l, n)));
            }
            if let Some(speed) = link.speed {
                items.push(detail(Outline::Speed, l.pick(texts::wlan::LINK_SPEED), format!("{speed} Mbps")));
            }
            items.push(detail(Outline::Signal, signal, fmt::signal(l, link.signal).to_string()));
            items.push(security);
            items.push(detail(
                Outline::Info,
                l.pick(texts::wlan::ADDRESS_SOURCE),
                fmt::address_source(l, link.by_hand).to_string(),
            ));
            items.push(detail(Outline::Address, l.pick(texts::wlan::IP_ADDRESS), link.address.to_string()));
            items.push(detail(Outline::Mask, l.pick(texts::wlan::SUBNET_MASK), link.mask.to_string()));
            if let Some(router) = link.router {
                items.push(detail(Outline::Router, l.pick(texts::wlan::ROUTER), router.to_string()));
            }
        }
        _ => {
            items.push(detail(Outline::Signal, signal, fmt::signal(l, network.signal).to_string()));
            items.push(security);
        }
    }
    let y = widgets::detail_grid(&mut c, t, f, y, &items);
    let forget = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::wlan::FORGET)).tint(t.pal.danger).trail(Trail::None)]);
    out.extend(hits(&forget.rows, &[Action::LeaveNetwork]));
    Frame::new(t, fit(c, t, forget.bottom), out, title, top)
}

/// The file manager, every entry of the folder shown, folders first.
/// The files on the OTG drive, all of them: folders and files alike.
///
/// An entry picked in the file manager is put down here from the bar held at
/// the bottom, so a file on the phone is copied onto the drive.
pub fn files_otg(cx: &Ctx) -> Frame {
    let l = cx.state.lang;
    let top = l.pick(["OTG", "OTG", "OTG"]);
    let dock = cx.state.clip.as_ref().map(|clip| {
        let go = l.pick(match clip.op {
            FileOp::Copy => ["复制到此处", "複製到這裡", "Copy here"],
            FileOp::Move => ["移动到此处", "移動到這裡", "Move here"],
        });
        let (canvas, rects) = widgets::dock_bar(cx.t, cx.f, files::page(cx.t), &[l.pick(CANCEL), go]);
        Dock { canvas, hits: hits(&rects, &[Action::Unpick, Action::Paste]) }
    });
    let frame = listing(
        cx,
        &cx.state.otg,
        top,
        None,
        l.pick(["U 盘为空", "USB 隨身碟是空的", "The drive is empty"]),
        Action::OpenOtgEntry,
        top,
        None,
        None,
        None,
        Some((l.pick(["内部存储", "內部儲存空間", "Internal storage"]), Action::Open(Screen::Files))),
        None,
    );
    Frame { dock, ..frame }
}

pub fn files(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    // 排序的面板是这一页上的另一屏：返回键先关它，不离开文件管理（2026-11-24）。
    if s.sort_panel {
        return sort_page(cx);
    }
    // An entry picked to copy or move goes to the folder shown, from the
    // bar held at the bottom.
    let picking = s.picking.then(|| {
        let use_here = l.pick(["用这个文件夹", "用這個資料夾", "Serve this folder"]);
        let (canvas, rects) = widgets::dock_bar(t, f, files::page(t), &[use_here]);
        Dock { canvas, hits: hits(&rects, &[Action::ServeUseHere]) }
    });
    let dock = picking.or_else(|| s.clip.as_ref().map(|clip| {
        let go = l.pick(match clip.op {
            FileOp::Copy => texts::files::COPY_HERE,
            FileOp::Move => texts::files::MOVE_HERE,
        });
        let (canvas, rects) = widgets::dock_bar(t, f, files::page(t), &[l.pick(texts::common::CANCEL), go]);
        Dock { canvas, hits: hits(&rects, &[Action::Unpick, Action::Paste]) }
    }));
    // Choosing several entries: the bar at the foot carries what can be done
    // with them, and the page's own button starts and stops the choosing.
    let choosing = s.choosing.then(|| {
        let all = l.pick(["全选", "全選", "All"]);
        let none = l.pick(["全不选", "全不選", "None"]);
        let copy = l.pick(["复制", "複製", "Copy"]);
        let cut = l.pick(["移动", "移動", "Move"]);
        let delete = l.pick(["删除", "刪除", "Delete"]);
        let (canvas, rects) = widgets::dock_bar_danger(t, f, files::page(t), &[all, copy, cut, delete], Some(3));
        let _ = none;
        Dock { canvas, hits: hits(&rects, &[Action::ChooseAll, Action::PasteChosen(FileOp::Copy), Action::PasteChosen(FileOp::Move), Action::AskDeleteChosen]) }
    });
    // 排序收进面板：面板是这一页上的整页（底部的 dock 在真机上命中不上，
    // 改成整页之后每一行都由页面自己的命中框回答，2026-11-24）。
    let dock = choosing.or(dock);
    // 右上角那个终端圆钮去掉了：那两枚胶囊（U 盘、搜索）就在路径条右边。
    let button = None;
    let root = l.pick(["内部存储", "內部儲存空間", "Internal storage"]);
    let other = l.pick(["U 盘", "USB 隨身碟", "USB drive"]);
    // 排序整组收进面板：这一枚只开面板。
    let sort_open = l.pick(["排序", "排序", "Sort"]);
    let entry_action: fn(usize) -> Action = if s.choosing { Action::ChooseEntryAt } else { Action::OpenEntry };
    // The way into choosing several sits at the end of the order row, after
    // folders first; once choosing, it says how to stop.
    let many = l.pick(match s.choosing {
        true => ["完成", "完成", "Done"],
        false => ["多选", "多選", "Select"],
    });
    // 已经在 /sdcard 里就不再画这一枚；否则路径条右端给一个「sdcard」胶囊（2026-11-24）。
    let card = l.pick(["sdcard", "sdcard", "sdcard"]);
    let on_card = s.files.path.trim_end_matches('/') == "/sdcard";
    let card_place: Option<(&str, Action)> = if on_card { None } else { Some((card, Action::FilesSdCard)) };
    let frame = listing(
        cx,
        &s.files,
        root,
        None,
        l.pick(["暂无文件", "沒有檔案", "No files"]),
        entry_action,
        l.pick(FILES),
        // 这一行只有路径；sdcard 接在「内部存储」后面（2026-11-24）。
        None,
        None,
        None,
        None,
        card_place,
    );
    // 顶部那一排：跟着返回按钮的圆钮，全是图标，画成四个一样的圆。
    let chips: [(Icon, Action, bool); 4] = [
        (Icon::Sort, Action::SortPanel(true), false),
        (Icon::Choose, Action::ChooseMany(!s.choosing), s.choosing),
        (Icon::UsbDrive, Action::FilesOtg, false),
        (Icon::Search, Action::SearchFiles, false),
    ];
    let (d, gap) = (t.m.dp(40.0), t.m.dp(8.0));
    let mut art = Canvas::new((d * 4.0 + gap * 3.0).ceil() as u32, d.ceil() as u32);
    let mut bar_hits = Vec::new();
    for (k, (icon, action, on)) in chips.iter().enumerate() {
        let x = k as f32 * (d + gap);
        // 底色用按钮色，亮色主题下也看得出是一个个圆；card 与页面同色，画了看不见。
        let (fill, ink) = if *on { (t.pal.button_press, t.pal.danger) } else { (t.pal.search, t.pal.text) };
        art.fill_circle(x + d / 2.0, d / 2.0, d / 2.0, fill);
        crate::icons::glyph(&mut art, *icon, x + d / 2.0, d / 2.0, d * 0.56, ink);
        bar_hits.push(Hit { rect: Rect::new(x, 0.0, d, d), action: *action });
    }
    // 位置放在「文件管理」这几个字的右边，和标题同一条线上。
    let title = l.pick(FILES);
    let zero = Canvas::new(0, 0);
    let px = zero.fitting(&f.title, title, t.m.title_px, t.m.w as f32 - t.m.title_x - t.m.margin);
    let tw = zero.text_width(&f.title, title, px);
    let ax = t.m.title_x + tw + t.m.dp(16.0);
    let ay = t.m.title_top - t.m.dp(4.0);
    Frame { dock, button, bar_art: None, bar_hits, title_art: Some((art, ax, ay)), ..frame }
}

/// 排序的面板：按什么排、升序还是降序都在这一张里点；选完就收起来。
/// 整页而不是底部 dock：dock 在真机上命中不上，整页之后每行由页面自己的命中框回答。
fn sort_page(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let keys: [(&str, SortKey); 4] = [
        (l.pick(["名称", "名稱", "Name"]), SortKey::Name),
        (l.pick(["大小", "大小", "Size"]), SortKey::Size),
        (l.pick(["修改时间", "修改時間", "Modified"]), SortKey::Modified),
        (l.pick(["类型", "類型", "Type"]), SortKey::Type),
    ];
    let sides: [(&str, bool); 2] = [
        (l.pick(["升序 ↑", "升序 ↑", "Up ↑"]), false),
        (l.pick(["降序 ↓", "降序 ↓", "Down ↓"]), true),
    ];
    let mut rows: Vec<Row> = Vec::new();
    for (label, key) in keys {
        rows.push(Row::new(label).trail(Trail::Check(s.sort == key)));
    }
    for (label, desc) in sides {
        rows.push(Row::new(label).trail(Trail::Check(s.sort_desc == desc)));
    }
    let done = l.pick(texts::common::DONE);
    let top = m.dp(56.0);
    let h = top + m.dp(56.0) * rows.len() as f32 + m.dp(96.0);
    let mut c = Canvas::new(m.w, h.ceil() as u32);
    c.clear(files::page(t));
    let mut out = Vec::new();
    let block = widgets::list(&mut c, t, f, top, &rows);
    for (k, rect) in block.rows.iter().enumerate() {
        let action = match keys.get(k) {
            Some((_, key)) => Action::SortTo(*key),
            None => Action::SortDesc(sides[k - keys.len()].1),
        };
        out.push(Hit { rect: *rect, action });
    }
    let y = block.bottom + m.dp(20.0);
    let px = m.dp(17.0);
    let h_row = m.dp(44.0);
    let w = c.text_width(&f.value, done, px) + m.dp(56.0);
    let x = (m.w as f32 - w) / 2.0;
    widgets::pill_sized(&mut c, f, x, y, h_row, px, m.dp(20.0), done, t.pal.search, t.pal.text);
    out.push(Hit { rect: Rect::new(x, y, w, h_row), action: Action::SortPanel(false) });
    let bottom = y + h_row;
    Frame::new(t, fit(c, t, bottom), out, l.pick(["排序", "排序", "Sort"]), top)
}

/// A place's folders and the files in them that can be installed, to
/// choose one. A USB drive's page waits for one to be plugged in, and the
/// drive's files show as soon as it is.
pub fn choose(cx: &Ctx, place: Place) -> Frame {
    let (s, l) = (cx.state, cx.state.lang);
    let unreadable;
    let absent = match (place, &s.usb_drive) {
        (Place::UsbDrive, None) => Some((l.pick(texts::choose::NO_USB_DRIVE), l.pick(texts::choose::CONNECT_USB_DRIVE))),
        (Place::UsbDrive, Some(UsbDrive::Unreadable(why))) => {
            unreadable = texts::choose::usb_drive_format(l, why);
            Some((l.pick(texts::usb_drive::UNREADABLE), unreadable.as_str()))
        }
        _ => None,
    };
    let empty = l.pick(["暂无可安装的文件", "沒有可安裝的檔案", "Nothing here to install"]);
    // The page that chooses a file has no order of its own: it shows the same
    // three controls, which turn the order of the file manager it came from.
    listing(cx, s.folder(Screen::Choose(place)), place_name(l, place), absent, empty, Action::ChooseEntry, l.pick(["选择文件", "選擇檔案", "Choose a file"]), None, None, None, None, None)
}

/// A folder laid out as the stock file manager lays one out: the path as
/// capsules from `top`, held under the bar once scrolled there, then the
/// entries in one list on the plain page, each row drawn as it scrolls into
/// view and touched for `action`. In place of the list, `absent` says why
/// nothing of the folder can be shown, and `empty` that it holds nothing.
fn listing(
    cx: &Ctx,
    folder: &Folder,
    top: &str,
    absent: Option<(&str, &str)>,
    empty: &str,
    action: fn(usize) -> Action,
    title: &'static str,
    // 排序那一枚：标签，与点开面板的动作。
    order: Option<(&str, Action)>,
    // 多选那一枚：标签、动作，以及它现在亮着没有。
    extra: Option<(&str, Action, bool)>,
    // 搜索这一页的名字，没有搜索的页面给 `None`。
    find: Option<(&str, Action)>,
    // The other place the files can be shown from, beside the trail.
    places_other: Option<(&str, Action)>,
    // 接在路径末尾的可点胶囊：另一个存储位置（2026-11-24）。
    tail: Option<(&str, Action)>,
) -> Frame {
    let (t, f) = (cx.t, cx.f);
    let bg = files::page(t);
    let mut c = sheet(cx, t.m.h, bg);
    // 有「额外一排」时（排序 / 搜索 / 多选 / 另一个位置），列表要往下让出那一排，
    // 否则第一行会被那排盖住、点不中（2026-11-24：sdcard 胶囊盖住了第一行）。
    let extra_dp = if order.is_some() || extra.is_some() || find.is_some() || places_other.is_some() { files::SORT_ROW_DP } else { 0.0 };
    let rows_top = files::LIST_TOP_DP + extra_dp;
    let rows = match (absent, &folder.entries) {
        (Some((note, why)), _) => {
            files::note(&mut c, t, f, note, Some(why));
            None
        }
        (None, Err(why)) => {
            files::note(&mut c, t, f, cx.state.lang.pick(texts::files::UNREADABLE), Some(why));
            None
        }
        (None, Ok(entries)) if entries.is_empty() => {
            files::note(&mut c, t, f, empty, None);
            None
        }
        (None, Ok(entries)) => Some(files::rows(t, f, entries, action, rows_top)),
    };
    let pinned = Some(files::strip_with(t, f, top, &folder.path, order, extra, find, places_other, tail));
    Frame { bg, rows, pinned, ..Frame::new(t, c, Vec::new(), title, t.m.dp(rows_top)) }
}

/// Row `i` of the page's long list over the whole of `c`, sized for it,
/// drawn as the row comes into view.
pub fn row(screen: Screen, cx: &Ctx, i: usize, c: &mut Canvas) {
    match screen {
        // 应用管理的一行：名字在上，包名在下，图标统一一个。
        Screen::DataApps => {
            if let Some(Ok(list)) = &cx.state.apps {
                if let Some(a) = list.get(i) {
                    // 名字只占一行：超了就截断加省略号，行高才稳。
                    let m = &cx.t.m;
                    let text_x = m.inner + m.icon + m.icon_gap;
                    let room = m.w as f32 - m.margin * 2.0 - text_x - m.inner;
                    // 二分找能放下的前缀：一个字符一个字符地试，长包名会一次
                    // 量上百遍，翻页就卡那一下（2026-11-18）。
                    let bounds: Vec<usize> = a.label.char_indices().map(|(i, _)| i).collect();
                    let fits = |end: usize| c.text_width(&cx.f.heading, &a.label[..end], m.row_title_px) <= room;
                    let mut lo = 0usize;
                    let mut hi = bounds.len() + 1;
                    while lo + 1 < hi {
                        let mid = (lo + hi) / 2;
                        let end = bounds.get(mid).copied().unwrap_or(a.label.len());
                        if fits(end) {
                            lo = mid;
                        } else {
                            hi = mid;
                        }
                    }
                    let cut = bounds.get(lo).copied().unwrap_or(a.label.len());
                    let label = a.label[..cut].to_string();
                    let mut row = Row::new(&label).icon(PURPLE, Icon::Layers);
                    row.sub = Some(&a.package);
                    widgets::draw_one(c, cx.t, cx.f, &row);
                }
            }
        }
        Screen::Files | Screen::FilesOtg | Screen::Choose(_) => {
            let folder = cx.state.folder(screen);
            let entries = folder.entries.as_ref().expect("a folder listed draws its rows");
            let chosen = cx.state.choosing && cx.state.chosen.contains(&entries[i].name);
            files::row_marked(c, cx.t, cx.f, cx.state.lang, &entries[i], folder.listed, cx.state.zone(), chosen);
        }
        Screen::Installing => widgets::output_row(c, cx.t, cx.f, &cx.state.script_output, i),
        Screen::PartWork => {
            let work = cx.state.part_work.as_ref().expect("the page of partition work shows work");
            widgets::output_row(c, cx.t, cx.f, &work.output, i)
        }
        other => unreachable!("{other:?} draws no list row by row"),
    }
}

/// One entry of a folder, a file or a folder: what it is, and what can be
/// done with it. Deleting asks first, with a swipe.
pub fn file_info(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let e = s.file.as_ref().expect("the file page shows an entry");
    let folder = matches!(e.kind, FileKind::Folder { .. });
    let mut c = long_page(cx);
    let m = &t.m;
    let (w, h) = (m.w as f32, m.h as f32);
    let side = m.margin;
    let top = m.content_top;
    let (card, name_ink, note_ink, pill_bg, pill_ink) = if t.dark {
        (
            Rgba::rgb(28, 28, 30),
            Rgba::rgb(242, 242, 242),
            Rgba::rgb(128, 128, 128),
            Rgba::rgb(38, 38, 38),
            Rgba::rgb(214, 216, 222),
        )
    } else {
        (
            Rgba::rgb(246, 247, 250),
            Rgba::rgb(0, 0, 0),
            Rgba::rgb(102, 102, 102),
            Rgba::rgb(238, 240, 245),
            Rgba::rgb(40, 44, 52),
        )
    };

    // 这一个文件能做些什么。能装能改的排在最前面，删除永远在最后一格、红字。
    let mut pills: Vec<(String, Action, bool)> = Vec::new();
    if installs(e) {
        let image = Path::new(&e.name).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("img"));
        pills.push((l.pick(if image { ["刷入", "刷入", "Flash"] } else { ["安装", "安裝", "Install"] }).to_string(), Action::InstallEntry, false));
    }
    if e.kind == FileKind::File {
        pills.push((l.pick(["编辑", "編輯", "Edit"]).to_string(), Action::EditEntry, false));
    }
    // 四排：复制/移动、复制到 U 盘/移动到 U 盘、重命名/文件权限、删除独占一行
    //（2026-11-17）。移动就是剪切，同一边挑目录；到另一边是一步直达。
    pills.push((l.pick(["复制", "複製", "Copy"]).to_string(), Action::PickEntry(FileOp::Copy), false));
    pills.push((l.pick(["移动", "移動", "Move"]).to_string(), Action::PickEntry(FileOp::Move), false));
    match s.file_on_drive {
        true => {
            pills.push((l.pick(["复制到手机", "複製到手機", "Copy to phone"]).to_string(), Action::ToPhone(FileOp::Copy), false));
            pills.push((l.pick(["移动到手机", "移動到手機", "Move to phone"]).to_string(), Action::ToPhone(FileOp::Move), false));
        }
        false => {
            pills.push((l.pick(["复制到 U 盘", "複製到 USB", "Copy to drive"]).to_string(), Action::ToDrive(FileOp::Copy), false));
            pills.push((l.pick(["移动到 U 盘", "移動到 USB", "Move to drive"]).to_string(), Action::ToDrive(FileOp::Move), false));
        }
    }
    // 一个 zip 还能当模块装：KSU / Magisk 的模块就是这个形状。
    if e.kind == FileKind::File && e.name.to_lowercase().ends_with(".zip") {
        pills.push((l.pick(["作为模块安装", "當作模組安裝", "Install as module"]).to_string(), Action::InstallAsModule, false));
    }
    pills.push((l.pick(RENAME).to_string(), Action::RenameEntry, false));
    if !matches!(e.kind, FileKind::Link { .. }) {
        pills.push((l.pick(["文件权限", "檔案權限", "Permissions"]).to_string(), Action::OpenPermissions, false));
    }
    pills.push((l.pick(["删除", "刪除", "Delete"]).to_string(), Action::AskDelete, true));

    // 等宽网格：三列（选项多的）或两列，格子一样宽、左右都对齐。删除单独
    // 一行、占满整行——它和别的不是一类事，挤在角落里反而像没排完。
    let (danger, others) = match pills.split_last() {
        Some((last, rest)) if last.2 => (Some(last.clone()), rest.to_vec()),
        _ => (None, pills.clone()),
    };
    // 列数按总格数挑，保证最多两行——三行排下来像没排完（2026-11-17）。
    // 删除也算一格，所以是「总格数的一半向上取整」。
    // 第一排四个，剩下的排第二排并**把整排均分**：第二排三个就三个各占
    // 三分之一，不留空档（2026-11-17）。
    // 每排两个、均匀铺满；删除单独占最后一整行（2026-11-17）。
    let all: Vec<(String, Action, bool)> = others.clone();
    let mut rows: Vec<(usize, usize)> = all
        .len()
        .checked_add(1)
        .map(|_| (0..all.len()).step_by(2).map(|k| (k, (k + 2).min(all.len()))).collect::<Vec<_>>())
        .unwrap_or_default();
    if danger.is_some() {
        rows.push((all.len(), all.len()));
    }
    let row_count = rows.len();
    let gap = m.dp(widgets::PILL_GAP_DP);
    let ph = m.dp(widgets::PILL_DP);
    let room = w - side * 2.0;
    let px = m.dp(widgets::PILL_TEXT_DP);
    let grid_px = m.dp(13.0);
    let grid_pad = m.dp(7.0);
    let inner = m.dp(18.0);
    let card_h = inner * 2.0 + m.dp(42.0);
    let title_px = m.dp(13.0);
    let inner_h = card_h;
    let sheet_h = m.dp(30.0) + row_count as f32 * ph + (row_count.saturating_sub(1)) as f32 * gap + m.dp(18.0);
    // 整块（信息卡 + 操作格）在页面里居中：全压在底部会让上半屏空得像没加载完。
    let block_h = inner_h + m.dp(12.0) + sheet_h;
    let block_top = ((h - block_h) / 2.0).max(top);
    let sheet_top = block_top + inner_h + m.dp(12.0);
    let card_top = block_top;

    // 上面这张卡只说一件事：正在操作哪一个。名字、大小或项数、改于何时。
    // 卡片按内容高，不抻到底：空着一大片会把视线拽走。
    // 整块贴着底部：卡片紧挨在操作格上面，像从下面升起来的一段
    //（空着的那一截留在页面标题之下，不把视线拉成两段）。
    c.fill_round_rect(side, card_top, room, card_h, m.card_radius, card);
    let cx = side + inner;
    let avatar = m.dp(42.0);
    let ay = card_top + inner;
    match e.kind {
        FileKind::Folder { .. } => {
            icons::glyph(&mut c, Icon::Folder, cx + avatar / 2.0, ay + avatar / 2.0 - m.dp(4.0), avatar * 0.8, Rgba::rgb(246, 196, 68));
        }
        _ => {
            let half = avatar * 0.78;
            c.fill_round_rect(cx + (avatar - half) / 2.0, ay + (avatar - half) / 2.0, half, half, m.dp(6.0), Rgba::rgb(120, 168, 255));
            let ext = Path::new(&e.name).extension().map(|x| x.to_string_lossy().to_uppercase()).unwrap_or_default();
            let ext: String = ext.chars().take(4).collect();
            if !ext.is_empty() {
                let pxl = m.dp(10.0);
                let tw = c.text_width(&f.value, &ext, pxl);
                c.text(&f.value, &ext, cx + avatar / 2.0 - tw / 2.0, ay + avatar / 2.0 + pxl * 0.36, pxl, Rgba::rgb(255, 255, 255), None);
            }
        }
    }
    let text_x = cx + avatar + m.dp(16.0);
    let text_room = side + room - inner - text_x;
    let title = cut_to(&c, &f.value, &e.name, m.dp(17.0), text_room);
    c.text(&f.value, &title, text_x, ay + m.dp(18.0), m.dp(17.0), name_ink, None);
    let holds = match &e.kind {
        FileKind::Folder { items: Some(n) } => match (l, *n) {
            (Lang::En, 1) => "1 item".to_string(),
            (_, n) => tr!(l, "{n} 项", "{n} 項", "{n} items"),
        },
        FileKind::Folder { items: None } => l.pick(UNREADABLE).to_string(),
        FileKind::Link { .. } => l.pick(["链接", "連結", "A link"]).to_string(),
        _ => fmt::file_size(e.size),
    };
    let when = fmt::datetime(l, e.modified, s.zone());
    c.text(&f.value, &format!("{holds} · {when}"), text_x, ay + m.dp(38.0), m.dp(13.0), note_ink, None);

    // 下面：一句标题，再一层等宽的药丸。
    c.text(&f.value, l.pick(["文件的操作", "檔案的操作", "What can be done"]), side, sheet_top + m.dp(20.0), title_px, note_ink, None);
    let mut rects: Vec<Rect> = Vec::new();
    let mut acts: Vec<Action> = Vec::new();
    for (r, (from, to)) in rows.iter().enumerate() {
        let n = to - from;
        let y = sheet_top + m.dp(30.0) + r as f32 * (ph + gap);
        // 删除那一排：一格占满整行。
        if n == 0 {
            if let Some((label, action, _)) = &danger {
                c.fill_round_rect(side, y, room, ph, ph / 2.0, pill_bg);
                c.text_center(&f.value, label, side + room / 2.0, y + ph / 2.0, px, t.pal.danger);
                rects.push(Rect::new(side, y, room, ph));
                acts.push(action.clone());
            }
            continue;
        }
        let cell = (room - gap * (n as f32 - 1.0)) / n as f32;
        for (i, k) in (*from..*to).enumerate() {
            let (label, action, bad) = &all[k];
            let x = side + i as f32 * (cell + gap);
            c.fill_round_rect(x, y, cell, ph, ph / 2.0, pill_bg);
            let shown = cut_to(&c, &f.value, label, grid_px, cell - grid_pad * 2.0);
            c.text_center(&f.value, &shown, x + cell / 2.0, y + ph / 2.0, grid_px, if *bad { t.pal.danger } else { pill_ink });
            rects.push(Rect::new(x, y, cell, ph));
            acts.push(action.clone());
        }
    }
    let page_title = l.pick(if folder { ["文件夹信息", "資料夾資訊", "Folder info"] } else { ["文件信息", "檔案資訊", "File info"] });
    let bottom = sheet_top + sheet_h + m.dp(8.0);
    Frame::new(t, fit(c, t, bottom), hits(&rects, &acts), page_title, top)
}

/// `text` cut to `w` pixels, its middle taken out.
fn cut_to(c: &Canvas, face: &crate::font::Face, text: &str, px: f32, w: f32) -> String {
    let width = |t: &str| c.text_width(face, t, px);
    if width(text) <= w {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let room = w - width("…");
    let mut right = chars.len();
    while right > 0 && width(&chars[right - 1..].iter().collect::<String>()) <= room / 2.0 {
        right -= 1;
    }
    let left_room = room - width(&chars[right..].iter().collect::<String>());
    let mut left = 0;
    while left < right && width(&chars[..=left].iter().collect::<String>()) <= left_room {
        left += 1;
    }
    format!("{}…{}", chars[..left].iter().collect::<String>(), chars[right..].iter().collect::<String>())
}

/// The permission bits of the entry the file page shows, a switch each that
/// takes effect at once: reading, writing and running for its owner, its
/// group and everyone else, then the set-ID and sticky bits.
pub fn permissions(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let e = s.file.as_ref().expect("the permissions page shows an entry");
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mode = e.mode & 0o7777;
    let bits = fmt::mode(e.mode);
    use texts::files::mode as said;
    let facts = [(l.pick(texts::files::NAME), e.name.clone()), (l.pick(texts::files::PERMISSIONS), said::bits(l, bits, format!("{mode:04o}")))];
    let mut y = widgets::info_card(&mut c, t, f, top, &facts);
    let mut out = Vec::new();
    let classes = [(said::OWNER, 6), (said::GROUP, 3), (said::OTHERS, 0)];
    let (read, write, run) = (l.pick(said::READ), l.pick(said::WRITE), l.pick(said::EXECUTE));
    let groups = classes.map(|(title, shift)| (l.pick(title), [(read, 4 << shift), (write, 2 << shift), (run, 1 << shift)]));
    let special = (l.pick(said::SPECIAL), [("SUID", 0o4000), ("SGID", 0o2000), (l.pick(said::STICKY), 0o1000)]);
    for (title, bits) in groups.into_iter().chain([special]) {
        y = widgets::section(&mut c, t, f, y, title);
        let rows = bits.map(|(name, bit)| Row::new(name).trail(Trail::Switch(mode & bit != 0)));
        let list = widgets::list(&mut c, t, f, y, &rows);
        out.extend(hits(&list.rows, &bits.map(|(_, bit)| Action::ModeBit(bit))));
        y = list.bottom;
    }
    Frame::new(t, fit(c, t, y), out, l.pick(texts::files::CHANGE_PERMISSIONS), top)
}

/// A new name for the entry the file page shows, typed on the keyboard.
pub fn rename(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let (y, caret) = widgets::text_field(&mut c, t, f, top, &s.new_name);
    widgets::footnote(&mut c, t, f, y, l.pick(texts::files::RENAME_NOTE));
    keyboard_hits(&mut c, t, f, s, l.pick(texts::common::DONE), &mut out);
    Frame { caret: Some(caret), ..Frame::new(t, c, out, l.pick(texts::files::RENAME), top) }
}

/// The folder an entry goes to already holds its name: what to do.
pub fn clash(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let (clip, into, _) = s.clash.as_ref().expect("the clash page shows a clash");
    let name = Path::new(&clip.path).file_name().map_or_else(|| clip.path.clone(), |n| n.to_string_lossy().into_owned());
    use texts::files::clash as said;
    let skip = l.pick(match clip.op {
        FileOp::Copy => said::SKIP_COPY,
        FileOp::Move => said::SKIP_MOVE,
    });
    let mut c = page(cx);
    let top = t.m.content_top;
    let y = widgets::paragraph(&mut c, t, f, top, &said::there(l, into, name));
    let choices = widgets::list(&mut c, t, f, y + t.m.dp(12.0), &[
        Row::new(l.pick(said::REPLACE)).subtitle(l.pick(said::REPLACE_NOTE)),
        Row::new(l.pick(said::SKIP)).subtitle(skip),
        Row::new(l.pick(said::KEEP_BOTH)).subtitle(l.pick(said::KEEP_BOTH_NOTE)),
    ]);
    let out = hits(&choices.rows, &[
        Action::Resolve(ClashChoice::Replace),
        Action::Resolve(ClashChoice::Skip),
        Action::Resolve(ClashChoice::KeepBoth),
    ]);
    Frame::new(t, c, out, l.pick(said::TITLE), top)
}

/// Copying, moving or deleting: how far it has come and a way to stop it,
/// then, if it did not finish, why.
pub fn file_job(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let job = s.job.as_ref().expect("the job page shows work on files");
    let path = match &job.task {
        FileTask::Copy { path, .. } | FileTask::Move { path, .. } | FileTask::Delete { path } => path,
    };
    use texts::files::job as said;
    let name = Path::new(path).file_name().map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
    let share = |(done, total): (u64, u64)| if total == 0 { 0.0 } else { done as f32 / total as f32 };
    // The bar stays while the work measures, so that the button under it
    // does not move. Deleting, and copying entries that hold no bytes, is
    // counted in entries.
    let (detail, done) = match (job.progress, &job.task) {
        (None, _) => (l.pick(texts::installing::GETTING_READY).to_string(), 0.0),
        (Some((bytes, _)), FileTask::Copy { .. }) if bytes.1 > 0 => (said::copied_of(l, fmt::file_size(bytes.0), fmt::file_size(bytes.1)), share(bytes)),
        (Some((bytes, _)), FileTask::Move { .. }) if bytes.1 > 0 => (said::moved_of(l, fmt::file_size(bytes.0), fmt::file_size(bytes.1)), share(bytes)),
        (Some((_, entries)), FileTask::Copy { .. }) => (said::copied_items(l, entries.0, entries.1), share(entries)),
        (Some((_, entries)), FileTask::Move { .. }) => (said::moved_items(l, entries.0, entries.1), share(entries)),
        (Some((_, entries)), FileTask::Delete { .. }) => (said::deleted_items(l, entries.0, entries.1), share(entries)),
    };
    let mut c = page(cx);
    let top = t.m.content_top;
    let y = widgets::progress_card(&mut c, t, f, top, &name, &detail, Some(done));
    let mut out = Vec::new();
    match &job.outcome {
        None => {
            let label = l.pick(if job.stopping { texts::common::STOPPING } else { texts::common::STOP });
            let stop = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(label).trail(Trail::None)]);
            if !job.stopping {
                out.extend(hits(&stop.rows, &[Action::StopJob]));
            }
        }
        Some(end) => {
            let why = match (end, &job.task) {
                (JobEnd::Stopped, FileTask::Copy { .. }) => l.pick(said::COPY_STOPPED).to_string(),
                (JobEnd::Stopped, FileTask::Move { .. }) => l.pick(said::MOVE_STOPPED).to_string(),
                (JobEnd::Stopped, FileTask::Delete { .. }) => l.pick(said::DELETE_STOPPED).to_string(),
                (JobEnd::Failed(why), FileTask::Copy { .. }) => said::copy_failed(l, why),
                (JobEnd::Failed(why), FileTask::Move { .. }) => said::move_failed(l, why),
                (JobEnd::Failed(why), FileTask::Delete { .. }) => said::delete_failed(l, why),
                (JobEnd::Done, _) => unreachable!("work done leaves its page"),
            };
            let y = widgets::footnote(&mut c, t, f, y, &why);
            let finish = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(texts::common::DONE)).trail(Trail::None)]);
            out.extend(hits(&finish.rows, &[Action::Back]));
        }
    }
    let title = l.pick(match (&job.task, job.outcome.is_none()) {
        (FileTask::Copy { .. }, true) => said::COPYING,
        (FileTask::Copy { .. }, false) => said::COPY_INCOMPLETE,
        (FileTask::Move { .. }, true) => said::MOVING,
        (FileTask::Move { .. }, false) => said::MOVE_INCOMPLETE,
        (FileTask::Delete { .. }, true) => said::DELETING,
        (FileTask::Delete { .. }, false) => said::DELETE_INCOMPLETE,
    });
    Frame { back: job.outcome.is_some(), ..Frame::new(t, c, out, title, top) }
}

/// A shell on a terminal: the screen it writes, the keys a phone keyboard
/// lacks, and the keyboard, every key of which goes to the shell.
pub fn terminal(cx: &Ctx) -> Frame {
    let (t, f, s) = (cx.t, cx.f, cx.state);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    keyboard_hits(&mut c, t, f, s, s.lang.pick(texts::keyboard::ENTER), &mut out);
    let l = term::layout(t, f, top);
    term::draw(&mut c, t, f, &l, cx.term);
    let held = cx.term.is_some_and(|term| term.ctrl);
    let keys = term::keys(&mut c, t, f, &l, &term::TERM_KEYS, held);
    out.extend(keys.into_iter().map(|(rect, key)| Hit { rect, action: Action::TermKey(key) }));
    Frame { scrollback: Some((l.card, l.pitch)), ..Frame::new(t, c, out, s.lang.pick(texts::page::TERMINAL), top) }
}

/// A text file being edited: where it is, its lines wrapped on a card, the
/// keys a phone keyboard lacks for moving the cursor, and the keyboard.
/// Once it is changed, the round button saves it.
pub fn editor(cx: &Ctx) -> Frame {
    let (t, f, s) = (cx.t, cx.f, cx.state);
    let ed = s.editor.as_ref().expect("the editor page shows a file");
    let mut c = page(cx);
    let mut out = Vec::new();
    keyboard_hits(&mut c, t, f, s, s.lang.pick(texts::keyboard::NEW_LINE), &mut out);
    editor::caption(&mut c, t, f, &ed.path);
    let l = editor::layout(t, f);
    let caret = editor::draw(&mut c, t, f, &l, ed);
    let keys = term::keys(&mut c, t, f, &l, &editor::EDITOR_KEYS, false);
    out.extend(keys.into_iter().map(|(rect, key)| Hit { rect, action: Action::TermKey(key) }));
    let button = ed.changed.then_some((widgets::Glyph::Done, Action::SaveText));
    Frame { scrollback: Some((l.card, l.pitch)), caret, button, ..Frame::new(t, c, out, s.lang.pick(texts::files::EDIT), t.m.content_top) }
}

/// The languages the pages speak, each named in itself, to speak another.
pub fn language(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let rows: Vec<Row> = Lang::ALL.iter().map(|&k| Row::new(k.name()).trail(Trail::Check(k == l))).collect();
    let list = widgets::list(&mut c, t, f, top, &rows);
    let out = hits(&list.rows, &Lang::ALL.map(Action::Language));
    Frame::new(t, fit(c, t, list.bottom), out, l.pick(texts::page::LANGUAGE), top)
}

/// The time zone the status bar's clock and the times on the pages keep.
/// On a phone whose installed system's zone is known, a switch follows it
/// and tells which it is; a phone with no system, or one that has set no
/// zone, shows none. Under it a zone is chosen from a part of the world,
/// whose row names the zone kept while that is one of its own.
pub fn time_zone(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = match &s.system_zone {
        Some(zone) => {
            let name = listed(&zone.name).map_or(zone.name.as_str(), |at| l.pick(TIME_ZONES[at].name));
            let system = format!("{} · {name}", fmt::utc_offset(s.now, &zone.rules));
            let follow = Row::new(l.pick(texts::time_zone::FOLLOW_SYSTEM)).subtitle(&system).trail(Trail::Switch(s.follow_system_zone));
            let follow = widgets::list(&mut c, t, f, top, &[follow]);
            out.extend(hits(&follow.rows, &[Action::Toggle(Switch::SystemTimeZone)]));
            widgets::section(&mut c, t, f, follow.bottom, l.pick(texts::time_zone::CHOOSE))
        }
        None => top,
    };
    let kept = s.zone_listed();
    let rows = Region::ALL.map(|region| {
        let row = Row::new(l.pick(region.name()));
        match kept.filter(|&at| TIME_ZONES[at].region == Some(region)) {
            Some(at) => row.trail(Trail::Value(l.pick(TIME_ZONES[at].name))),
            None => row,
        }
    });
    let regions = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&regions.rows, &Region::ALL.map(|region| Action::Open(Screen::TimeZones(region)))));
    let utc = listed(UTC).expect("UTC is a zone listed");
    let offset = fmt::utc_offset(s.now, fmt::rules(utc));
    let universal = Row::new(l.pick(TIME_ZONES[utc].name)).subtitle(&offset).trail(Trail::Check(kept == Some(utc)));
    let universal = widgets::list(&mut c, t, f, regions.bottom + t.m.card_gap, &[universal]);
    out.extend(hits(&universal.rows, &[Action::TimeZone(utc)]));
    let bottom = widgets::footnote(&mut c, t, f, universal.bottom, l.pick(texts::time_zone::NOTE));
    Frame::new(t, fit(c, t, bottom), out, l.pick(texts::page::TIME_ZONE), top)
}

/// The time zones of one part of the world, from the one furthest behind
/// UTC now to the one furthest ahead, each with how far that is and the
/// cities it is known by.
pub fn time_zones(cx: &Ctx, region: Region) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut zones: Vec<usize> = (0..TIME_ZONES.len()).filter(|&at| TIME_ZONES[at].region == Some(region)).collect();
    zones.sort_by_key(|&at| fmt::utc_offset_seconds(s.now, fmt::rules(at)));
    let lines: Vec<String> = zones.iter().map(|&at| format!("{} · {}", fmt::utc_offset(s.now, fmt::rules(at)), l.pick(TIME_ZONES[at].cities))).collect();
    let kept = s.zone_listed();
    let rows: Vec<Row> = zones.iter().zip(&lines).map(|(&at, line)| Row::new(l.pick(TIME_ZONES[at].name)).subtitle(line).trail(Trail::Check(kept == Some(at)))).collect();
    let list = widgets::list(&mut c, t, f, top, &rows);
    let picks: Vec<Action> = zones.iter().map(|&at| Action::TimeZone(at)).collect();
    Frame::new(t, fit(c, t, list.bottom), hits(&list.rows, &picks), l.pick(region.name()), top)
}

/// Saving the logs, as a bug report asks for them.
pub fn logs(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::logs::CONTENTS));
    let parts = widgets::list(&mut c, t, f, y, &[
        Row::new(l.pick(texts::logs::RECOVERY_LOG)).subtitle(l.pick(texts::logs::RECOVERY_LOG_NOTE)).trail(Trail::Check(s.logs[0])),
        Row::new(l.pick(texts::logs::KERNEL_LOG)).subtitle(l.pick(texts::logs::KERNEL_LOG_NOTE)).trail(Trail::Check(s.logs[1])),
    ]);
    out.extend(hits(&parts.rows, &[Action::LogPart(0), Action::LogPart(1)]));
    let y = widgets::section(&mut c, t, f, parts.bottom, l.pick(texts::logs::SAVE_TO));
    let storage = l.pick(match cx.state.data.storage_open() {
        true => texts::logs::FOLDER,
        false => texts::decrypt::NEEDED,
    });
    let places = widgets::list(&mut c, t, f, y, &[
        Row::new(l.pick(texts::logs::TEMPORARY)).subtitle(l.pick(texts::logs::TEMPORARY_NOTE)).trail(Trail::Check(s.logs_to == 0)),
        Row::new(place_name(l, Place::Storage)).subtitle(storage).trail(Trail::Check(s.logs_to == 1)),
    ]);
    out.extend(hits(&places.rows, &[Action::LogsTo(0), Action::LogsTo(1)]));
    let y = widgets::footnote(&mut c, t, f, places.bottom, l.pick(texts::logs::NOTE));
    let export = widgets::button(&mut c, t, f, y + t.m.dp(24.0), l.pick(texts::logs::EXPORT));
    out.push(Hit { rect: export, action: Action::ExportLogs });
    Frame::new(t, c, out, l.pick(texts::page::LOGS), top)
}

/// The words the licence gives for a program's notice, which every
/// language but English reads its own rendering ahead of.
const LICENSE_ORIGINAL: &str = "Skyward\nCopyright (C) 2026 Kirk Lin\n\n\
This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.\n\n\
This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.\n\n\
You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.";

/// Skyward's own licence.
pub fn license(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let y = widgets::info_card(&mut c, t, f, top, &[(l.pick(texts::license::TERMS_TITLE), l.pick(texts::license::TERMS).to_string())]);
    let bottom = match l.pick(texts::license::ORIGINAL_ALONE) {
        true => widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), LICENSE_ORIGINAL),
        false => {
            let y = widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), l.pick(texts::license::NOTICE));
            let y = widgets::section(&mut c, t, f, y, l.pick(texts::license::ORIGINAL));
            widgets::paragraph(&mut c, t, f, y - t.m.dp(16.0), LICENSE_ORIGINAL)
        }
    };
    Frame::new(t, fit(c, t, bottom), Vec::new(), l.pick(texts::page::LICENSE), top)
}

/// The monospaced typeface's licence, which names the copyright holders on
/// its first lines; the image carries the same file beside the font.
const MONO_LICENSE: &str = include_str!("../../../resources/fonts/KirkMono-OFL.txt");

/// The typefaces, as they name themselves, and the terms each is used under.
pub fn font_license(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let (typeface, version, copyright) = (l.pick(texts::fonts::TYPEFACE), l.pick(texts::fonts::VERSION), l.pick(texts::fonts::COPYRIGHT));
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::fonts::INTERFACE_TEXT));
    let y = widgets::info_card(&mut c, t, f, y, &[
        (typeface, f.info.family.clone()),
        (version, f.info.version.clone()),
        (copyright, f.info.copyright.clone().expect("MiSans names its copyright holder")),
    ]);
    let y = widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), l.pick(texts::fonts::MISANS));
    let y = widgets::section(&mut c, t, f, y, l.pick(texts::fonts::TERMINAL_TEXT));
    let holders: Vec<&str> = MONO_LICENSE.lines().take_while(|l| l.starts_with("Copyright")).collect();
    let y = widgets::info_card(&mut c, t, f, y, &[
        (typeface, f.mono_info.family.clone()),
        (version, f.mono_info.version.clone()),
        (copyright, holders.join("\n")),
        (l.pick(texts::license::TERMS_TITLE), l.pick(texts::fonts::OFL).to_string()),
    ]);
    let bottom = widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), l.pick(texts::fonts::MONO));
    // 名字里的 emoji：随包附带的两份 Noto 字体，与上面两种一样要在页面上说明。
    let y = widgets::section(&mut c, t, f, bottom, l.pick(texts::fonts::EMOJI_TEXT));
    let bottom = widgets::paragraph(&mut c, t, f, y, l.pick(texts::fonts::EMOJI));
    Frame::new(t, fit(c, t, bottom), Vec::new(), l.pick(texts::page::FONTS), top)
}

/// Which slot the phone starts from next, as the boot control holds it,
/// and what it holds of each; picking the other and swiping switches.
/// While an update is in flight the slot stays as the update set it.
pub fn slot(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let title = l.pick(texts::slot::BOOT_SLOT);
    let Some(current) = i.slot else {
        widgets::list(&mut c, t, f, top, &[Row::new(l.pick(texts::slot::NO_SLOTS)).trail(Trail::None)]);
        return Frame::new(t, c, out, title, top);
    };
    let slots = match &s.boot_slots {
        Some(Ok(slots)) => *slots,
        Some(Err(why)) => {
            let list = widgets::list(&mut c, t, f, top, &[Row::new(l.pick(texts::slot::UNREADABLE)).trail(Trail::None)]);
            widgets::footnote(&mut c, t, f, list.bottom, why);
            return Frame::new(t, c, out, title, top);
        }
        None => return Frame::new(t, c, out, title, top),
    };
    let chosen = s.next_slot.unwrap_or(slots.active);
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::reboot::NEXT_BOOT));
    let titles = [Slot::A, Slot::B].map(|slot| texts::slot::named(l, slot.letter()));
    let subs = [Slot::A, Slot::B].map(|slot| {
        let k = slot.number() as usize;
        l.pick(match (slot == current, slots.bootable[k], slots.successful[k]) {
            (true, false, _) => texts::slot::RUNNING_UNBOOTABLE,
            (true, true, false) => texts::slot::RUNNING_UNSTARTED,
            (true, true, true) => texts::slot::RUNNING,
            (false, false, _) => texts::slot::OTHER_UNBOOTABLE,
            (false, true, false) => texts::slot::OTHER_UNSTARTED,
            (false, true, true) => texts::slot::OTHER,
        })
    });
    let rows: Vec<Row> =
        [Slot::A, Slot::B].iter().enumerate().map(|(k, &slot)| Row::new(&titles[k]).subtitle(subs[k]).trail(Trail::Check(slot == chosen))).collect();
    let list = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&list.rows, &[Slot::A, Slot::B].map(Action::PickSlot)));
    if let Some(note) = fmt::snapshot_slot_note(l, &i.snapshot) {
        widgets::footnote(&mut c, t, f, list.bottom, note);
        return Frame::new(t, c, out, title, top);
    }
    let y = widgets::footnote(&mut c, t, f, list.bottom, l.pick(texts::slot::SWITCH_NOTE));
    let (label, danger) = confirm_label(Confirm::Slot, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::Slot) });
    Frame::new(t, c, out, title, top)
}

/// The two systems as big cards one over the other, laid out as the boot
/// slot page lays out its slots: the main system, in the phone's own
/// partitions, and the second, whose files lie in the data partition, or
/// the place to install one, the one that starts next checked. Picking the
/// other and swiping writes its boot images into the slot that starts
/// next, and restarts into it. Under them, the second system, and removing
/// it.
pub fn systems(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let title = l.pick(texts::page::DUAL_BOOT);
    // While the boot partitions are written, or the second system removed,
    // the page only says so: of a switch the system asked for, perhaps
    // before the systems are read.
    if let Some(work) = s.systems_work {
        let (heading, detail) = match work {
            SystemsWork::Switching(to) => {
                let name = match (&s.systems, to) {
                    (Some(Ok(systems)), Who::Installed) => Some(systems.installed.name.as_str()),
                    (Some(Ok(systems)), Who::Other) => systems.other.as_ref().map(|o| o.name.as_str()),
                    _ => None,
                };
                let heading = match name {
                    Some(name) => texts::systems::switching_to(l, name),
                    None => l.pick(texts::systems::SWITCHING).to_string(),
                };
                (heading, l.pick(texts::systems::WRITING_BOOT_IMAGES))
            }
            SystemsWork::Removing => (l.pick(texts::systems::REMOVING).to_string(), l.pick(texts::systems::GIVING_ROOM_BACK)),
        };
        let y = widgets::progress_card(&mut c, t, f, top, &heading, detail, None);
        let y = widgets::footnote(&mut c, t, f, y, l.pick(texts::systems::KEEP_POWER));
        return Frame::new(t, fit(c, t, y), out, title, top);
    }
    let systems = match &s.systems {
        Some(Ok(systems)) => systems,
        Some(Err(why)) => {
            let list = widgets::list(&mut c, t, f, top, &[Row::new(l.pick(texts::systems::UNREADABLE)).trail(Trail::None)]);
            let y = widgets::footnote(&mut c, t, f, list.bottom, why);
            return Frame::new(t, fit(c, t, y), out, title, top);
        }
        None => return Frame::new(t, fit(c, t, top), out, title, top),
    };
    let starts = starting(systems);
    let chosen = s.system_pick.unwrap_or(starts);
    let second_role = l.pick(texts::systems::SECOND);
    // Boot partitions that hold a second system's image nothing is kept of
    // start neither system the page knows.
    let main = SystemCard {
        role: l.pick(texts::systems::MAIN),
        name: &systems.installed.name,
        detail: systems.installed.version.as_deref().unwrap_or_default(),
        mark: SystemMark::Check(chosen == Who::Installed && systems.held != Held::Unrecorded),
    };
    let second = match &systems.other {
        Some(other) => SystemCard { role: second_role, name: &other.name, detail: &other.version, mark: SystemMark::Check(chosen == Who::Other) },
        None => SystemCard {
            role: second_role,
            name: l.pick(texts::systems::INSTALL),
            detail: l.pick(texts::systems::CHOOSE_PACKAGE),
            mark: SystemMark::Opens,
        },
    };
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::reboot::NEXT_BOOT));
    let shown = widgets::system_cards(&mut c, t, f, y, &[main, second]);
    let second_opens = match systems.other {
        Some(_) => Action::PickSystem(Who::Other),
        None => Action::Open(Screen::Install),
    };
    out.extend(hits(&shown.rows, &[Action::PickSystem(Who::Installed), second_opens]));
    let mut y = shown.bottom + t.m.dp(12.0);
    if let Some(why) = &s.system_notice {
        y = widgets::info_card(&mut c, t, f, y, &[(l.pick(texts::systems::SECOND_FAILED), why.clone())]);
    }
    if chosen != starts {
        match (chosen, fmt::snapshot_system_note(l, &i.snapshot)) {
            (Who::Other, Some(note)) => y = widgets::footnote(&mut c, t, f, y, note),
            _ => {
                let (label, danger) = confirm_label(Confirm::SwitchSystem, s);
                let track = widgets::confirm(&mut c, t, f, y + t.m.dp(12.0), label, danger, s.confirm);
                out.push(Hit { rect: track, action: Action::Confirm(Confirm::SwitchSystem) });
                y = track.y + track.h;
            }
        }
    }
    y = widgets::footnote(&mut c, t, f, y, l.pick(texts::systems::WHAT));
    match systems.held {
        Held::Recorded => {}
        Held::Rewritten => y = widgets::footnote(&mut c, t, f, y, l.pick(texts::systems::REWRITTEN)),
        Held::Unrecorded => y = widgets::footnote(&mut c, t, f, y, l.pick(texts::systems::UNRECORDED)),
    }
    if systems.other.is_some() && systems.changed {
        y = widgets::footnote(&mut c, t, f, y, l.pick(texts::systems::CHANGED));
    }
    if let Some(other) = &systems.other {
        y = widgets::section(&mut c, t, f, y, second_role);
        let size = fmt::bytes(other.size);
        let facts = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(texts::systems::SPACE_USED)).trail(Trail::Text(&size))]);
        let mounted = matches!(s.data, Data::Open | Data::Locked { .. });
        let remove = Row::new(l.pick(texts::systems::REMOVE)).tint(t.pal.danger);
        let (remove, act) = match mounted {
            true => (remove.trail(Trail::None), Action::AskDelete),
            false => (remove.subtitle(l.pick(texts::decrypt::NEEDED)), Action::Open(Screen::Decrypt)),
        };
        let removing = widgets::list(&mut c, t, f, facts.bottom + t.m.card_gap, &[remove]);
        out.extend(hits(&removing.rows, &[act]));
        y = removing.bottom;
        if s.asking_delete && mounted {
            let ask = l.pick(match starts {
                Who::Other => texts::systems::ASK_REMOVE_RUNNING,
                Who::Installed => texts::systems::ASK_REMOVE,
            });
            let y2 = widgets::footnote(&mut c, t, f, y, ask);
            let (label, danger) = confirm_label(Confirm::RemoveSystem, s);
            let track = widgets::confirm(&mut c, t, f, y2 + t.m.dp(24.0), label, danger, s.confirm);
            out.push(Hit { rect: track, action: Action::Confirm(Confirm::RemoveSystem) });
            y = track.y + track.h;
        }
    }
    Frame::new(t, fit(c, t, y), out, title, top)
}

/// The phone's own fastbootd serving the computer: whether it runs, how far
/// the computer has the port, and where to go from here, chosen with the
/// keys and lit where they are.
pub fn fastboot(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = page(cx);
    let top = t.m.content_top;
    let running = l.pick(if s.fastboot.running { texts::fastboot::RUNNING } else { texts::fastboot::NOT_RUNNING });
    let link = l.pick(match s.fastboot.link {
        UsbLink::Connected => texts::fastboot::CONNECTED,
        UsbLink::Connecting => texts::fastboot::CONNECTING,
        UsbLink::Unplugged => texts::fastboot::NO_COMPUTER,
    });
    let slot = i.slot.map(|s| s.letter());
    let mut facts = vec![
        Row::new("fastbootd").trail(Trail::Text(running)),
        Row::new("USB").trail(Trail::Text(link)),
        Row::new(l.pick(texts::specs::CODENAME)).trail(Trail::Text(&i.codename)),
    ];
    if let Some(letter) = slot {
        facts.push(Row::new(l.pick(texts::slot::CURRENT)).trail(Trail::Text(letter)));
    }
    let y = widgets::section_first(&mut c, t, f, top, l.pick(texts::fastboot::STATUS));
    let state = widgets::list(&mut c, t, f, y, &facts);
    let y = widgets::footnote(&mut c, t, f, state.bottom, l.pick(texts::fastboot::NOTE));
    let y = widgets::section(&mut c, t, f, y, l.pick(texts::fastboot::LEAVE));
    let rows: Vec<Row> = FASTBOOT_CHOICES.iter().map(|&(title, _)| Row::new(l.pick(title)).trail(Trail::None)).collect();
    let choices = widgets::list(&mut c, t, f, y, &rows);
    let actions: Vec<Action> = FASTBOOT_CHOICES.iter().map(|&(_, action)| action).collect();
    let out = hits(&choices.rows, &actions);
    widgets::footnote(&mut c, t, f, choices.bottom, l.pick(texts::fastboot::KEYS));
    Frame { back: false, ..Frame::new(t, c, out, "Fastboot", top) }
}

/// Where to go once what the system asked for has stopped or cannot be
/// done, in the order the pages list them.
const AFTERWARDS: [(Text, Action); 3] = [
    (texts::installed::REBOOT_TO_SYSTEM, Action::RebootTo(Target::System)),
    (texts::afterwards::ENTER_RECOVERY, Action::Dismiss),
    (texts::reboot::target::POWER_OFF, Action::RebootTo(Target::PowerOff)),
];

/// The list of [`AFTERWARDS`] at `y`, and where it ends. Work the user
/// chose in the recovery goes back to its home page, rather than into it.
fn afterwards(c: &mut Canvas, t: &Theme, f: &Fonts, l: Lang, y: f32, asked: bool, out: &mut Vec<Hit>) -> f32 {
    let rows: Vec<Row> = AFTERWARDS
        .iter()
        .map(|&(title, action)| Row::new(l.pick(if action == Action::Dismiss && !asked { texts::afterwards::BACK_HOME } else { title })).trail(Trail::None))
        .collect();
    let choices = widgets::list(c, t, f, y, &rows);
    out.extend(hits(&choices.rows, &AFTERWARDS.map(|(_, action)| action)));
    choices.bottom
}

/// Erasing: each part and how far it has come, and once erasing has
/// stopped, why. What the system asked for leaves on its own once it is
/// done, and offers where to go only if it stops; what the user chose
/// offers where to go once it ends either way.
pub fn erase(cx: &Ctx) -> Frame {
    use std::cmp::Ordering;
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let e = &s.erase;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let failed = matches!(e.outcome, Some(Err(_)));
    // Only a part that went wrong was under way; whatever else stopped
    // erasing stopped it before any part began.
    let begun = !failed || matches!(e.outcome, Some(Err(EraseStop::Failed(..))));
    let heading = l.pick(match &e.outcome {
        None => texts::wipe::WIPING,
        Some(Ok(())) => texts::wipe::WIPED,
        Some(Err(_)) => texts::wipe::INCOMPLETE,
    });
    let mut y = widgets::section_first(&mut c, t, f, top, heading);
    if !e.parts.is_empty() {
        let states: Vec<String> = (0..e.parts.len())
            .map(|k| match (k.cmp(&e.done), failed) {
                (Ordering::Less, _) => l.pick(texts::wipe::WIPED).into(),
                (Ordering::Equal, false) => match e.progress {
                    Some((done, total)) if total > 0 => texts::wipe::wiping_share(l, percent(share(done, total))),
                    _ => l.pick(texts::wipe::WIPING).into(),
                },
                (Ordering::Equal, true) if begun => l.pick(texts::common::UNFINISHED).into(),
                (Ordering::Greater, false) => l.pick(texts::common::WAITING).into(),
                (Ordering::Equal | Ordering::Greater, true) => l.pick(texts::wipe::NOT_WIPED).into(),
            })
            .collect();
        let rows: Vec<Row> = e.parts.iter().zip(&states).map(|(&part, state)| Row::new(fmt::part(l, part)).trail(Trail::Text(state))).collect();
        y = widgets::list(&mut c, t, f, y, &rows).bottom;
    }
    let notes: Vec<String> = match (&e.outcome, e.asked, e.reset) {
        (Some(Err(stop)), _, _) => {
            let (what, detail) = fmt::erase_stop(l, stop);
            std::iter::once(what).chain(detail).collect()
        }
        (_, true, true) => {
            let leaving = match s.asked.shutdown_after {
                true => texts::erase::ALL_THEN_POWER_OFF,
                false => texts::erase::ALL_THEN_REBOOT,
            };
            vec![l.pick(leaving).into()]
        }
        (_, true, false) => vec![l.pick(if s.asked.shutdown_after { texts::parts::leaving::POWER_OFF } else { texts::parts::leaving::REBOOT }).into()],
        (None, false, _) => vec![l.pick(texts::wipe::KEEP_POWER).into()],
        (Some(Ok(())), false, true) => vec![l.pick(texts::erase::MADE_ANEW).into()],
        (Some(Ok(())), false, false) => Vec::new(),
    };
    for note in &notes {
        y = widgets::footnote(&mut c, t, f, y, note);
    }
    if let (true, Some(reason)) = (e.asked, &s.asked.reason) {
        y = widgets::info_card(&mut c, t, f, y + t.m.dp(12.0), &[(l.pick(texts::installing::REASON), reason.clone())]);
    }
    if failed || (!e.asked && e.outcome.is_some()) {
        let below = widgets::section(&mut c, t, f, y, l.pick(texts::common::ACTIONS));
        y = afterwards(&mut c, t, f, l, below, e.asked, &mut out);
    }
    let title = l.pick(match (e.reset, e.asked) {
        (true, _) => texts::wipe::RESET,
        (false, true) => texts::erase::WIPE_CACHE,
        (false, false) => texts::page::WIPE,
    });
    Frame { back: false, ..Frame::new(t, fit(c, t, y), out, title, top) }
}

/// The system could not start Android and asks whether to erase all user
/// data: start it again, look around the recovery first, or erase.
pub fn cant_start(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let mut y = widgets::paragraph(&mut c, t, f, top, l.pick(texts::cant_start::WHAT));
    if let Some(reason) = &s.asked.reason {
        y = widgets::info_card(&mut c, t, f, y + t.m.dp(12.0), &[(l.pick(texts::installing::REASON), reason.clone())]);
    }
    y = widgets::section(&mut c, t, f, y, l.pick(texts::common::ACTIONS));
    let rows = [
        Row::new(l.pick(texts::common::TRY_AGAIN)).subtitle(l.pick(texts::cant_start::STARTS_AGAIN)).trail(Trail::None),
        Row::new(l.pick(texts::afterwards::ENTER_RECOVERY)).subtitle(l.pick(texts::cant_start::WIPES_NOTHING)).trail(Trail::None),
    ];
    let choices = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&choices.rows, &[Action::RebootTo(Target::System), Action::Dismiss]));
    let y = widgets::footnote(&mut c, t, f, choices.bottom, l.pick(texts::cant_start::CANT_UNDO));
    let (label, danger) = confirm_label(Confirm::FactoryReset, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::FactoryReset) });
    Frame { back: false, ..Frame::new(t, c, out, l.pick(texts::cant_start::TITLE), top) }
}

/// Something the system asked of the recovery that it does not do: what
/// was asked, and where to go instead.

/// 关机页：手机会整个关掉，按电源键才能再开；手上正在安装、备份或写入的活
/// 会被切断，可能留下没写完的分区——这些话写在滑动之前，而不是关完才知道。
///
/// 从「青菜豆腐」分支的 `pages.rs` 搬来（`power_off`），行文与配色照我们的
/// 页面改过；滑动本身还是 [`Confirm::PowerOff`]，落到
/// [`Effect::Reboot`](crate::app::Effect::Reboot) 的 `PowerOff`。
pub fn power_off(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let x = t.m.w as f32 / 2.0;
    let sign_cy = t.m.dp(120.0);
    c.fill_circle(x, sign_cy, t.m.dp(30.0), t.pal.card);
    icons::glyph(&mut c, Icon::Power, x, sign_cy, t.m.dp(40.0), t.pal.accent);
    c.text_center(&f.heading, l.pick(POWER_OFF), x, sign_cy + t.m.dp(64.0), t.m.dp(24.0), t.pal.text);
    c.text_center(
        &f.body,
        l.pick([
            "手机将完全关闭，按电源键可再次开机。",
            "手機將完全關閉，按電源鍵可再次開機。",
            "The phone turns off completely. Press the power key to turn it on again.",
        ]),
        x,
        sign_cy + t.m.dp(96.0),
        t.m.sub_px,
        t.pal.subtitle,
    );
    let y = sign_cy + t.m.dp(150.0);
    let running = l.pick([
        "正在进行的安装、备份或写入会被中断，并可能留下未完成的分区。",
        "正在進行的安裝、備份或寫入會被中斷，並可能留下未完成的分割區。",
        "An install, backup or write under way is cut off, and may leave a partition half-written.",
    ]);
    let y = widgets::footnote(&mut c, t, f, y, running);
    let (label, danger) = confirm_label(Confirm::PowerOff, s);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(Confirm::PowerOff) });
    Frame::new(t, fit(c, t, track.y + track.h), out, l.pick(POWER_OFF), top)
}

const POWER_OFF: Text = ["关机", "關機", "Power off"];

/// 分区镜像的备份：把分区原样导成镜像、把镜像写回它的分区、把不要的镜像删掉。
///
/// 从「青菜豆腐」分支的 `pages.rs` 搬来（`image_backup`）：那边是「列分区 →
/// 选一个 → 滑动导出」，我们把它接在自己的 `read_partitions` 与
/// `Effect::DumpImage` 上（导出还是原来那条路，只是旁边多留一份 sha256），
/// 再把写回和删除补上。写回与删除都只动选中的那一份，滑道只有一条，含义由
/// 「写回分区／删除镜像」两行里选中的那行决定。
pub fn image_backup(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    // 分区多、镜像多的时候这一页比三屏还长：画布按内容撑开，别让底下被裁掉
    // 之后连滑都滑不到（2026-11-20）。
    let mut c = long_page_n(cx, 8);
    let mut out = Vec::new();
    let top = t.m.content_top;
    // 导出：选一个分区，滑一下。
    let y = widgets::section_first(&mut c, t, f, top, l.pick(["导出成镜像", "匯出成映像檔", "Export as an image"]));
    let y = match &s.partitions {
        None => widgets::footnote(&mut c, t, f, y, l.pick(["正在读取分区", "正在讀取分割區", "Reading partitions"])),
        Some(Err(error)) => widgets::footnote(&mut c, t, f, y, error),
        Some(Ok(parts)) => {
            let sizes: Vec<String> = parts.iter().map(|(name, size)| image_size_note(l, name, *size)).collect();
            let rows: Vec<Row> = parts
                .iter()
                .zip(&sizes)
                .enumerate()
                .map(|(k, ((name, _), size))| Row::new(name).subtitle(size).trail(Trail::Check(s.dump_pick == Some(k))))
                .collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            out.extend(hits(&list.rows, &(0..parts.len()).map(Action::PickDump).collect::<Vec<_>>()));
            let picked = s.dump_pick.and_then(|k| parts.get(k)).map(|(name, _)| name.clone());
            match picked {
                Some(name) => {
                    let note = tr!(l, "导出「{name}」", "匯出「{name}」", "Export \u{201c}{name}\u{201d}");
                    let one = widgets::list(&mut c, t, f, list.bottom, &[Row::new(&note).icon(GREEN, Icon::Backup)]);
                    out.extend(hits(&one.rows, &[Action::DumpNow]));
                    one.bottom
                }
                None => widgets::footnote(&mut c, t, f, list.bottom, l.pick(["点一个分区选中它。", "點一個分割區選中它。", "Tap a partition to choose it."])),
            }
        }
    };
    // 放哪儿：跟备份用的是同一组地方。
    let y = widgets::section(&mut c, t, f, y, l.pick(["存放位置", "存放位置", "Save to"]));
    let places = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(place_name(l, Place::Storage)).trail(Trail::Check(s.backup_to == Place::Storage)),
            Row::new(place_name(l, Place::UsbDrive)).trail(Trail::Check(s.backup_to == Place::UsbDrive)),
        ],
    );
    out.extend(hits(&places.rows, &[Action::BackupTo(Place::Storage), Action::BackupTo(Place::UsbDrive)]));
    // 已有的镜像：选一份，再决定写回还是删除。
    let y = widgets::section(&mut c, t, f, places.bottom, l.pick(["已有的镜像", "已有的映像檔", "Images kept"]));
    let y = match &s.image_copies {
        None => widgets::footnote(&mut c, t, f, y, l.pick(["正在读取", "正在讀取", "Reading"])),
        Some(Err(error)) => widgets::footnote(&mut c, t, f, y, error),
        Some(Ok(list)) if list.is_empty() => widgets::footnote(&mut c, t, f, y, l.pick(["这里还没有镜像", "這裡還沒有映像檔", "No images here yet"])),
        Some(Ok(list)) => {
            let details: Vec<String> = list.iter().map(|copy| image_copy_note(l, copy)).collect();
            let rows: Vec<Row> = list
                .iter()
                .zip(&details)
                .enumerate()
                .map(|(k, (copy, detail))| Row::new(&copy.partition).subtitle(detail).trail(Trail::Check(s.image_pick == Some(k))))
                .collect();
            let found = widgets::list(&mut c, t, f, y, &rows);
            out.extend(hits(&found.rows, &(0..list.len()).map(Action::ImagePick).collect::<Vec<_>>()));
            found.bottom
        }
    };
    // 一滑动做什么。
    let y = widgets::section(&mut c, t, f, y, l.pick(["对选中的镜像", "對選中的映像檔", "With the image chosen"]));
    let choices = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(l.pick(["写回它的分区", "寫回它的分割區", "Write it back"]))
                .subtitle(l.pick(["整份核对过再写，核对不过就不写", "整份核對過再寫，核對不過就不寫", "Checked whole before anything is written"]))
                .trail(Trail::Check(s.image_action == ImageAction::Restore)),
            Row::new(l.pick(["删除这份镜像", "刪除這份映像檔", "Delete it"])).trail(Trail::Check(s.image_action == ImageAction::Delete)),
        ],
    );
    out.extend(hits(
        &choices.rows,
        &[Action::ImageAction(ImageAction::Restore), Action::ImageAction(ImageAction::Delete)],
    ));
    // 再读一遍：从电脑上拷进来的镜像，不用离开这一页就能看见。
    let again = widgets::list(
        &mut c,
        t,
        f,
        choices.bottom,
        &[Row::new(l.pick(["重新读取镜像", "重新讀取映像檔", "Read the images again"])).icon(BLUE, Icon::Restore)],
    );
    out.extend(hits(&again.rows, &[Action::ImageRefresh]));
    let y = widgets::footnote(
        &mut c,
        t,
        f,
        again.bottom + t.m.card_gap,
        l.pick([
            "导出会写进 Skyward/backup，并在旁边留一份 sha256；写回之前整份核对一遍。",
            "匯出會寫進 Skyward/backup，並在旁邊留一份 sha256；寫回之前整份核對一遍。",
            "Exports go into Skyward/backup with a sha256 beside them, and are checked whole before they are written back.",
        ]),
    );
    let which = match s.image_action {
        ImageAction::Restore => Confirm::ImageRestore,
        ImageAction::Delete => Confirm::ImageDelete,
    };
    let (label, danger) = confirm_label(which, s);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit { rect: track, action: Action::Confirm(which) });
    Frame::new(t, fit(c, t, track.y + track.h), out, l.pick(IMAGE_BACKUP), top)
}

/// 分区在导出那一行下面的一句：多重，super 另外说明。
fn image_size_note(l: Lang, name: &str, size: u64) -> String {
    if name == "super" {
        format!(
            "{} \u{b7} {}",
            fmt::bytes(size),
            l.pick(["共享分区，包含双槽逻辑分区", "共用分割區，包含雙槽邏輯分割區", "Shared, holds both slots' logical partitions"])
        )
    } else {
        fmt::bytes(size)
    }
}

/// 一份镜像下面的一句：多大、什么时候写的、有没有校验。
fn image_copy_note(l: Lang, copy: &ImageCopy) -> String {
    let checked = match copy.sha256 {
        Some(_) => l.pick(["有校验", "有核對", "checked"]),
        None => l.pick(["无校验", "無核對", "no checksum"]),
    };
    format!("{} \u{b7} {} \u{b7} {checked}", fmt::bytes(copy.size), date_of(copy.made))
}

/// 秒数写成 `YYYY-MM-DD HH:MM`（按 UTC）；读不懂就留一个短横。
fn date_of(seconds: i64) -> String {
    if seconds <= 0 {
        return "\u{2014}".to_string();
    }
    let days = seconds / 86_400;
    let rest = seconds % 86_400;
    let (mut year, mut day) = (1970i64, days);
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let length = if leap { 366 } else { 365 };
        if day < length {
            break;
        }
        day -= length;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let months = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0;
    while month < 12 && day >= months[month] {
        day -= months[month];
        month += 1;
    }
    format!("{year:04}-{:02}-{:02} {:02}:{:02}", month + 1, day + 1, rest / 3600, rest % 3600 / 60)
}

const IMAGE_BACKUP: Text = ["镜像备份", "映像檔備份", "Partition images"];

/// 一份 vbmeta 副本下面的一句：哪一路槽位、多大、回滚索引是多少。
fn vbmeta_copy_note(l: Lang, copy: &VbmetaCopy) -> String {
    let slot = match copy.suffix.is_empty() {
        true => l.pick(["槽位不明", "槽位不明", "slot unknown"]),
        false => &copy.suffix,
    };
    let index = match &copy.rollback {
        Ok(index) => tr!(l, "回滚索引 {index}", "回滾索引 {index}", "rollback {index}"),
        Err(_) => l.pick(["回滚索引读不出", "回滾索引讀不出", "rollback unreadable"]).to_string(),
    };
    format!("{slot} \u{b7} {} \u{b7} {index}", fmt::bytes(copy.size))
}

pub fn unhandled(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let mut items = vec![(l.pick(texts::unhandled::REQUEST), s.asked.unhandled.clone().unwrap_or_default())];
    if let Some(reason) = &s.asked.reason {
        items.push((l.pick(texts::installing::REASON), reason.clone()));
    }
    let y = widgets::info_card(&mut c, t, f, top, &items);
    let y = widgets::footnote(&mut c, t, f, y, l.pick(texts::unhandled::NOTE));
    let y = widgets::section(&mut c, t, f, y, l.pick(texts::common::ACTIONS));
    afterwards(&mut c, t, f, l, y, true, &mut out);
    Frame { back: false, ..Frame::new(t, c, out, l.pick(texts::unhandled::TITLE), top) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::BackupRefusal;

    fn job(restore: bool, checks: bool) -> BackupJob {
        let plan = vec![("boot".into(), 200), ("system".into(), 800)];
        BackupJob { restore, place: Place::Storage, checks, plan, at: None, outcome: None, stopping: false }
    }

    fn at(item: usize, done: u64, of: u64, checking: bool) -> BackupAt {
        BackupAt { item, done, of, checking }
    }

    #[test]
    fn a_script_shows_its_last_line_until_skyward_goes_back() {
        let said = ["- Flashing boot".to_string(), "  ".to_string()];
        let l = Lang::Hans;
        assert_eq!(script_stage(l, None, &[]), ("正在运行安装脚本", None));
        assert_eq!(script_stage(Lang::En, None, &[]), ("Running the install script", None));
        assert_eq!(script_stage(l, Some(&Stage::Script { thousandths: 250 }), &said), ("- Flashing boot", Some(0.25)), "past a blank line");
        assert_eq!(script_stage(l, Some(&Stage::Reflashing { done: 1, total: 4 }), &said), ("正在重新刷入 Skyward", Some(0.25)));
        assert_eq!(update_stage(Lang::En, Some(&Stage::Reflashing { done: 3, total: 4 })), ("Flashing Skyward again".to_string(), Some(0.75)));
        assert_eq!(update_stage(Lang::Hant, Some(&Stage::Writing { done: 1, total: 4 })).0, "第 1 步，共 3 步：寫入新系統 25%");
    }

    #[test]
    fn a_backup_checks_its_parts_last_and_a_restore_first() {
        // Halfway through writing system, 600 of the 1000 bytes: the first
        // of two passes, then the second.
        assert_eq!(job_progress(&job(false, true), &at(1, 400, 800, false)), (0.3, 600));
        assert_eq!(job_progress(&job(false, true), &at(1, 400, 800, true)), (0.8, 600));
        assert_eq!(job_progress(&job(false, false), &at(1, 400, 800, false)), (0.6, 600), "without a check, writing is all");
        assert_eq!(job_progress(&job(true, true), &at(0, 100, 200, true)), (0.05, 100));
        assert_eq!(job_progress(&job(true, true), &at(0, 100, 200, false)), (0.55, 100));
    }

    #[test]
    fn each_part_says_where_it_stands() {
        let states = |j: &BackupJob| (0..2).map(|k| part_state(Lang::Hans, j, k)).collect::<Vec<_>>();
        let mut b = job(false, true);
        assert_eq!(states(&b), ["等待中", "等待中"]);
        b.at = Some(at(1, 400, 800, false));
        assert_eq!(states(&b), ["已备份", "正在备份 50%"]);
        b.at = Some(at(0, 50, 200, true));
        assert_eq!(states(&b), ["正在校验 25%", "已备份"]);
        b.outcome = Some(BackupEnd::Failed("the backup does not hold what was stored".into()));
        assert_eq!(states(&b), ["未完成", "未校验"]);
        b.outcome = Some(BackupEnd::Saved("20261021-141639".into()));
        assert_eq!(states(&b), ["已校验", "已校验"]);
        let mut r = job(true, true);
        r.at = Some(at(1, 0, 800, true));
        assert_eq!(states(&r), ["已校验", "正在校验 0%"]);
        r.at = Some(at(0, 200, 400, false));
        assert_eq!(states(&r), ["正在恢复 50%", "已校验"]);
        r.outcome = Some(BackupEnd::Failed("write /dev/block/by-name/boot_b".into()));
        assert_eq!(states(&r), ["未完成", "未恢复"]);
        r.at = None;
        r.outcome = Some(BackupEnd::Refused(BackupRefusal::OlderAndroid { was: "17".into(), now: "16".into() }));
        assert_eq!(states(&r), ["未开始", "未开始"]);
    }

    #[test]
    fn each_part_says_where_it_stands_in_the_language() {
        let mut b = job(false, true);
        b.at = Some(at(1, 400, 800, false));
        assert_eq!((0..2).map(|k| part_state(Lang::En, &b, k)).collect::<Vec<_>>(), ["Backed up", "Backing up 50%"]);
        let mut r = job(true, true);
        r.at = Some(at(0, 200, 400, false));
        assert_eq!((0..2).map(|k| part_state(Lang::Hant, &r, k)).collect::<Vec<_>>(), ["正在還原 50%", "已驗證"]);
    }
}
