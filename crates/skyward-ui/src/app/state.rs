//! What the pages show and the user changes, all in one place.

use super::*;

/// The warmest and coolest the charge may be let to warm the battery to, in
/// whole degrees Celsius, and where the slider starts.
pub const THERMAL_LIMIT_LEAST: u8 = 40;
pub const THERMAL_LIMIT_MOST: u8 = 55;
pub const THERMAL_LIMIT_DEFAULT: u8 = 45;

/// How fast the phone is let to charge, as the charge page offers it: three
/// steps, each the programs the recovery starts for it, since it is the
/// phone's own programs that tell the kernel what a charge may draw and
/// whether the charger may be verified.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ChargeMode {
    /// Neither program: the kernel holds the charge at the ceiling it sets
    /// for a battery it has not been told passed its check.
    #[default]
    Plain,
    /// Xiaomi's charging service, which passes on the battery's check: the
    /// ceiling lifts, and the charge draws what a plain power delivery
    /// connection gives.
    Fast,
    /// That service and Xiaomi's secret keeper, which has the charger
    /// verified: the protocol goes on to the adjustable kind, and the charge
    /// draws what the charger can give.
    Fastest,
}

impl ChargeMode {
    /// The steps, in the order the page offers them.
    pub const ALL: [ChargeMode; 3] = [ChargeMode::Plain, ChargeMode::Fast, ChargeMode::Fastest];

    /// The name it is kept by in the settings file, so a later version that
    /// lists the steps otherwise reads it the same.
    pub fn name(self) -> &'static str {
        match self {
            ChargeMode::Plain => "plain",
            ChargeMode::Fast => "fast",
            ChargeMode::Fastest => "fastest",
        }
    }

    /// The step a settings file names, where it names one this version has.
    pub fn named(name: &str) -> Option<ChargeMode> {
        ChargeMode::ALL.into_iter().find(|mode| mode.name() == name)
    }

    /// Whether Xiaomi's charging service is started for it.
    pub fn charging_service(self) -> bool {
        matches!(self, ChargeMode::Fast | ChargeMode::Fastest)
    }

    /// Whether Xiaomi's secret keeper is started for it.
    pub fn secret_keeper(self) -> bool {
        matches!(self, ChargeMode::Fastest)
    }

    /// The step below this one, where there is one: what the thermal guard
    /// chooses when the battery gets too warm.
    pub fn stepped_down(self) -> Option<ChargeMode> {
        match self {
            ChargeMode::Plain => None,
            ChargeMode::Fast => Some(ChargeMode::Plain),
            ChargeMode::Fastest => Some(ChargeMode::Fast),
        }
    }

    /// Whether a charge at this step is fast enough for its warmth to be
    /// worth watching: the plain step starts neither of Xiaomi's programs,
    /// and what it draws cannot warm the battery enough to matter, so the
    /// thermal guard is neither offered nor obeyed there.
    pub fn guarded(self) -> bool {
        !matches!(self, ChargeMode::Plain)
    }
}

/// 记住的一张网：名字、SSID 与保护方式；密码不在这里，后台从名单里取来去连。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedWlan {
    pub ssid: Vec<u8>,
    pub name: String,
    pub security: Security,
}

/// The folder served to the network the phone is on: not served, served and
/// where, or why it could not be served.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Serving {
    #[default]
    Off,
    Running {
        /// The address a browser on the same network opens.
        at: String,
        /// How many requests it has answered, so the page shows it is alive.
        served: u64,
    },
    Failed(String),
}

/// How a download asked for on the download page stands.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Fetch {
    /// Nothing asked for yet.
    #[default]
    Idle,
    /// How many bytes have come, and how many the server named where it did.
    Running { done: u64, total: Option<u64> },
    Done { path: String, bytes: u64 },
    Failed(String),
    Stopped,
}

/// 应用管理看的是用户装的应用，还是系统分区里的。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppsScope {
    User,
    System,
}

/// 搜索框找的是哪一类东西：整台机器的功能，还是某一页里的条目。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchScope {
    Features,
    Apps,
    Modules,
    /// 文件管理里当前文件夹的那些名字。
    Files,
}

/// One module the phone's own system would load: KernelSU's and Magisk's keep
/// them the same way, a folder each with a `module.prop` beside it, and a
/// `disable` file inside a folder that is switched off.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleRow {
    /// The folder's name, which is how the module is known.
    pub id: String,
    pub name: String,
    /// What module.prop says of it: its version and who made it.
    pub note: String,
    /// Its own words about what it does.
    pub about: String,
    pub enabled: bool,
    /// Marked to be removed the next time the phone starts.
    pub removing: bool,
    /// A newer copy waits in `modules_update` for the next start.
    pub pending: bool,
}

/// 镜像备份放的地方，从那个地方的顶层算起。
///
/// 跟 `skyward-backup` 的 `image::FOLDER` 是同一处：那边是运行时的读写，
/// 这边只是把路径拼出来交给它，两边的字面量要一样。
pub const IMAGE_FOLDER: &str = "Skyward/backup";

/// 一份分区的裸镜像备份，页面只拿元数据，不读内容。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageCopy {
    /// 文件名里的分区名。
    pub partition: String,
    /// 文件名，`.img` 一起：写回和删除都按它找。
    pub file: String,
    pub size: u64,
    /// 旁边那份校验说的值；没有那份文件时为空。
    pub sha256: Option<String>,
    /// 写下来的时间，按秒；读不到时为 0。
    pub made: i64,
}

/// 留着的一份 vbmeta 副本：它自己在哪儿、说的是哪一路、回滚索引是多少。
///
/// 从「青菜豆腐」分支搬来（那边的 `VbmetaCopy`），字段按我们的
/// `avbctl` 拿得到的东西排：索引读不出来时留着原因，页上照样列着。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VbmetaCopy {
    pub place: Place,
    /// 文件名：写回时按它找。
    pub name: String,
    /// 名字里说的槽位后缀，`_a` 或 `_b`；说不出时为空。
    pub suffix: String,
    pub size: u64,
    /// 这份镜像自己带的回滚索引，读不出来时是原因。
    pub rollback: Result<u64, String>,
    /// 留的时候，按秒；读不到时为 0。
    pub made: i64,
}

/// 镜像页上那一滑动到底做什么：写回分区，还是把这份镜像删掉。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageAction {
    #[default]
    Restore,
    Delete,
}

/// 正在录的那一段，页面上要显示的东西。
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// 这一段开着没有。
    pub on: bool,
    /// 这一段开始的时间，毫秒。
    pub since: u64,
    /// 写进去的文件，显示用。
    pub file: String,
    /// 规格，如 `600×1304 · 30 帧`。
    pub spec: String,
    /// 已录多久，形如 `0:42`；由 `tick` 每秒换一次。
    pub elapsed: String,
    /// 写进文件的帧数，收尾时由外面报回来。
    pub frames: u64,
    /// 分辨率，[`crate::record::SCALES`] 里的第几个。
    pub scale: usize,
    /// 画质，[`crate::record::QUALITIES`] 里的第几个。
    pub quality: usize,
    /// 帧率，[`crate::record::RATES`] 里的第几个。
    pub fps: usize,
    /// 容器：0 是 AVI，1 是 MP4。
    pub format: usize,
    /// 计时胶囊摆在哪儿：左上角的 dp 坐标，`None` 是默认的右上角。
    pub pill: Option<(f32, f32)>,
    /// 按了暂停，这一段停在那儿。
    pub paused: bool,
    /// 停下的时刻，毫秒。
    pub paused_at: u64,
    /// 这一段一共停了多久，毫秒：已录时长要把它减掉。
    pub held: u64,
}

impl Default for Record {
    /// 一上来是标准档：面板的一半、中等画质、30 帧 —— 编码有富余，画面真在动。
    fn default() -> Self {
        Record {
            on: false,
            since: 0,
            file: String::new(),
            spec: String::new(),
            elapsed: String::new(),
            frames: 0,
            scale: crate::record::DEFAULT_SCALE,
            quality: crate::record::DEFAULT_QUALITY,
            fps: crate::record::DEFAULT_FPS,
            format: 0,
            pill: None,
            paused: false,
            paused_at: 0,
            held: 0,
        }
    }
}

/// 录屏页上那三条能改的：分辨率、画质、帧率。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordSetting {
    Scale,
    Quality,
    Fps,
    Format,
}

/// Choices the pages show and the user changes, and where installing
/// stands.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// The language the pages speak.
    pub lang: Lang,
    /// The language was chosen on the language page, now or at an earlier
    /// start, where otherwise it is the one the system names.
    pub language_chosen: bool,
    pub dark: bool,
    pub brightness: f32,
    /// How long the screen stays on with nothing touched, by its place in
    /// [`SCREEN_OFF`].
    pub screen_off: usize,
    /// The clock and the times on the pages keep the time zone the
    /// installed system is set to, where that is known.
    pub follow_system_zone: bool,
    /// The time zone chosen on its pages, by its place in [`TIME_ZONES`],
    /// which the clock keeps while the system's is not followed or not
    /// known.
    pub time_zone: usize,
    /// The time zone the installed system is set to, as far as it is
    /// known: read from the data partition once it opens, and until then
    /// the one read at an earlier start, or the one the system is built to
    /// start in.
    pub system_zone: Option<SystemZone>,
    /// The time, in Unix seconds and to the minute, at which the time zone
    /// pages tell how far each zone is from UTC.
    pub now: i64,
    pub vibrate: bool,
    /// How strongly the motor taps, from 0 at the slider's left end to 1
    /// at its right.
    pub vibration: f32,
    /// A computer connected over USB is offered internal storage.
    pub file_transfer: bool,
    /// 首页条目的自定义名字：`Entry::id()` → 名字；没在里面的用页面自带的名字。
    pub entry_names: std::collections::BTreeMap<String, String>,
    /// 首页顶上那个大字；空着就是 Skyward。
    pub home_title: String,
    /// 改它的时候输入框里的字（2026-11-27）。
    pub home_title_edit: String,
    /// 存储里的布局文件；还没读过是 `None`。
    pub layouts: Option<Result<Vec<crate::layout::File>, String>>,
    /// 挑布局位置时正在看的目录，以及这是在挑导出位置（真）还是挑要导入
    /// 的文件（假）（2026-11-27）。
    pub layout_dir: Folder,
    pub layout_exporting: bool,
    /// 「其他」摆在分类之间的哪个位置：`None` 是末尾（老设置就是这样）。
    /// 它在首页与排序页都能上下挪，但不能删：删掉的分类的条目都落到这里。
    pub home_rest_at: Option<usize>,
    /// 正在改名的那一条，以及名字页输入框里的字（2026-11-27）。
    pub entry_edit: Option<Entry>,
    pub entry_name: String,
    /// 文件传输的服务此刻是否真的在跑：首页开关只说要什么，这一项说实际。
    /// 由 ui.rs 每轮写回（2026-11-27）。
    pub transfer_running: bool,
    pub reboot: usize,
    /// What the wipe page has chosen, by place in [`WIPES`].
    pub wipe: [bool; WIPES.len()],
    /// The phone has a cache partition, which the wipe page then offers.
    pub cache_partition: bool,
    /// How many folders of its own the phone keeps logs in on the data
    /// partition, which erasing the logs takes in.
    pub phone_logs: usize,
    pub pattern: Vec<usize>,
    /// How far the swipe-to-confirm knob is dragged, 0 to 1.
    pub confirm: f32,
    pub install: Install,
    /// Where the package being installed comes from.
    pub source: Source,
    /// The name of the file being installed, when it is one on the phone.
    pub package_name: Option<String>,
    /// What the install script last run showed, its latest lines.
    pub script_output: Vec<String>,
    /// A zip's install script runs only if the zip was signed with one of
    /// the phone's update keys.
    pub check_signature: bool,
    /// The phone keeps its recovery in a partition of each slot, which the
    /// install page then offers to keep Skyward in.
    pub recovery_partitions: bool,
    /// Skyward goes back into the recovery partitions an install writes
    /// over.
    pub keep_recovery: bool,
    /// How Skyward went back into the recovery partitions the install
    /// wrote over, where it did.
    pub reflash: Option<Reflash>,
    /// The mount page's partitions, as the runtime last read them, or why
    /// it could not.
    pub mounts: Result<Vec<MountRow>, String>,
    pub mount_failure: Option<MountFailure>,
    /// Which of [`BACKUP_PARTS`] a backup takes.
    pub backup: [bool; 5],
    /// Where a backup goes.
    pub backup_to: Place,
    pub compress: bool,
    pub verify: bool,
    /// The backups found, newest first, those that cannot be read last.
    pub backups: Vec<BackupEntry>,
    /// The backup the restore page shows, as it was found, and which of its
    /// parts to restore.
    pub restore: Option<(BackupEntry, [bool; 5])>,
    /// Backing up or restoring under way, or how it last ended.
    pub backup_job: Option<BackupJob>,
    /// Which logs to export: the recovery's, the kernel's.
    pub logs: [bool; 2],
    /// Where the logs go: 0 for the temporary folder, 1 for internal
    /// storage.
    pub logs_to: usize,
    /// What the phone's boot control holds of the slots, as the runtime
    /// last read it for the slot or reboot page, or why it could not.
    pub boot_slots: Option<Result<BootSlots, String>>,
    /// The slot picked on the slot page to start next; with none picked,
    /// the page shows the one the boot control starts next.
    pub next_slot: Option<Slot>,
    /// The phone has what a second system needs: boot and init_boot
    /// partitions in both slots, which a switch writes, and recovery
    /// partitions apart from them, which keep Skyward through it.
    pub dual_boot: bool,
    /// Whether this build offers to update itself, as its device says.
    pub updates: bool,
    /// The modules installed for the phone's own system, read when the page
    /// opens: the folder each sits in, what its module.prop says of it, and
    /// whether it is switched on.
    pub modules: Option<Result<Vec<ModuleRow>, String>>,
    /// The module the detail page shows, by its place in the list.
    pub module_at: Option<usize>,
    /// The apps installed in the data partition, read when the page opens.
    pub apps: Option<Result<Vec<crate::app::dataapp::DataApp>, String>>,
    /// The app the detail page shows, by its place in the list.
    pub app_at: Option<usize>,
    /// Each app's icon as pixels ready to draw: width, height and RGBA, the
    /// same order as `apps`.
    pub app_icons: Vec<(u32, u32, Vec<u8>)>,
    /// 模块管理这一页看到的第几屏。
    pub module_page: usize,
    /// 应用管理这一页看到的第几屏（一屏装不下一百多个应用）。
    pub app_page: usize,
    /// 应用管理当前显示哪一类：用户应用或系统应用。
    pub apps_scope: AppsScope,
    /// 系统分区的预装应用，只读页用；没有读过就是 None。
    pub sys_apps: Option<Vec<crate::app::dataapp::DataApp>>,
    /// 只读页看到第几屏。
    pub sys_page: usize,
    /// 系统应用的停用状态（包名 → 是否已停用），进入系统应用页时读一次。
    pub app_disabled: Option<std::collections::BTreeMap<String, bool>>,
    /// 已经在队列里、等下次系统启动执行的停用/启用（包名 → 是否要停用）。
    pub app_state_queued: std::collections::BTreeMap<String, bool>,
    /// 点过多少次翻页：用来判断点击到底有没有进来。
    pub app_page_taps: u32,
    /// 进过多少次应用管理这一页。
    pub app_arrivals: u32,
    /// 上一次落在翻页键上的点击：坐标与方向，用来核对是不是点错了键。
    pub page_tap: Option<(f32, f32, i32)>,
    /// 点在翻页那一条带上却没命中任何控件时的坐标。
    pub page_miss: Option<(f32, f32)>,
    /// 点在翻页带上却没命中时记下的意图：右半边 +1，左半边 -1。等翻页键画出来再执行。
    pub page_pending: Option<i32>,
    /// 最近一次画出来的总页数（用于核对翻页键是否已经在画面上）。
    pub pages_shown: usize
,
    /// 应用管理里等确认卸载的那个应用。
    pub app_confirm: Option<usize>,
    /// 模块管理里等确认卸载的那个模块。
    pub module_confirm: Option<usize>,
    /// A copy or a move waiting to be told where it goes: the phone or the
    /// drive, asked before the folder it lands in is walked to.
    pub place_ask: Option<crate::app::FileOp>,
    /// The systems, as the runtime last read them, or why it could not;
    /// none until it has.
    pub systems: Option<Result<Overview, String>>,
    /// The system picked on the systems page to start next; with none
    /// picked, the page shows the one that starts.
    pub system_pick: Option<Who>,
    /// Work on the systems under way.
    pub systems_work: Option<SystemsWork>,
    /// Why the second system's start stopped, as its first program told
    /// it, which the systems page shows.
    pub system_notice: Option<String>,
    /// Where the data partition stands, and what is typed at the screen
    /// lock its users' own files wait on.
    pub data: Data,
    /// How far opening the data partition has come, 0 to 1, which the bar on
    /// the decrypt page runs toward, and how far the ring about its sign has
    /// turned.
    pub data_phase: f32,
    /// Whether the phone's own parts -- its partitions, its drivers and its
    /// services -- are up. The decrypt page says it waits for them until they
    /// are, rather than that it is at the partition.
    pub prepared: bool,
    pub code: String,
    /// What is typed in the search field.
    pub search: String,
    /// 搜索页在找什么：功能，还是应用、模块列表里的条目。
    pub search_scope: SearchScope,
    /// The on-screen keyboard's layer.
    pub layer: Layer,
    /// 中文输入：正在拼的拼音与候选字。
    pub ime: crate::pinyin::Composition,
    /// 分区管理读到的分区与大小；还没读是 None。
    pub partitions: Option<Result<Vec<(String, u64)>, String>>,
    /// 录屏这一段的状态。
    pub record: Record,
    /// 控制中心摆着哪些磁贴，按摆的顺序；没存过时是 [`TILES`] 那十二个。
    pub tiles: Vec<Tile>,
    /// 分区管理里选中的、准备导出的那个分区。
    pub dump_pick: Option<usize>,
    /// 镜像页读到的那些镜像备份；还没读是 None。
    pub image_copies: Option<Result<Vec<ImageCopy>, String>>,
    /// 镜像页上选中的那份。
    pub image_pick: Option<usize>,
    /// 镜像页上那一滑动是写回还是删除。
    pub image_action: ImageAction,
    /// AVB 页上读到的、留着的 vbmeta 副本；还没读是 None。
    pub vbmeta_copies: Option<Result<Vec<VbmetaCopy>, String>>,
    /// AVB 页上选中的那份副本。
    pub vbmeta_pick: Option<usize>,
    /// AVB 页上正在问「要不要把选中的那份写回」：这时页底那一滑动是写回。
    pub vbmeta_asking: bool,
    /// The key of the on-screen keyboard, or the choice of a list open over
    /// the page, a finger holds down.
    pub held: Option<Action>,
    /// The folder the file manager shows.
    pub files: Folder,
    /// The folder the OTG page shows.
    pub otg: Folder,
    /// The screen lock is about to be kept, and the risk of keeping it is
    /// shown before it is.
    pub asking_remember: bool,
    /// The entry the file page shows was opened on the OTG page, so its path
    /// is measured from the top of the drive rather than the owner's storage.
    pub file_on_drive: bool,
    /// The date the lock screen shows under the clock, as the phone's own
    /// lock screen shows it.
    pub date: String,
    /// The order the file manager shows entries in, kept between starts.
    /// 排序那一组收进了面板里：面板开着没有。
    pub sort_panel: bool,
    pub sort: SortKey,
    pub sort_desc: bool,
    pub folders_first: bool,
    /// The folder of internal storage a file to install is chosen in, its
    /// path from the top of the storage.
    pub storage: Folder,
    /// The USB drive plugged in, as the runtime last found it, and the
    /// folder of it a file to install is chosen in, its path from the top
    /// of the drive.
    pub usb_drive: Option<UsbDrive>,
    pub drive: Folder,
    /// The entry the file page describes.
    pub file: Option<FileRow>,
    pub fastboot: Fastboot,
    pub erase: Erase,
    pub asked: Asked,
    /// The entry picked to copy or move, while a folder for it is looked
    /// for.
    pub clip: Option<Clip>,
    /// What is typed as an entry's new name.
    pub new_name: String,
    /// The text file the editor page shows, as typed so far.
    pub editor: Option<Editor>,
    /// Deleting the entry the file page shows, or the backup the restore
    /// page shows, was asked for; a swipe confirms it.
    pub asking_delete: bool,
    /// The picture the phone shows for its owner, and the name it gives
    /// itself: both are on the encrypted data partition, so they come once it
    /// is open, and the home page shows a row of them once they do.
    pub avatar: Option<crate::avatar::Avatar>,
    pub device_name: Option<String>,
    /// The file manager is choosing several entries: the names it has chosen,
    /// in the folder it shows.
    pub choosing: bool,
    pub chosen: std::collections::BTreeSet<String>,
    /// Deleting the entries chosen was asked for: their full paths, waiting on
    /// the swipe that confirms it.
    pub asking_many: Option<Vec<String>>,
    /// A copy or move waiting on what to do with the names taken where it
    /// goes, and those names.
    pub clash: Option<(Clip, String, Vec<String>)>,
    /// Work on files under way, or how it last ended.
    pub job: Option<FileJob>,
    /// Giving the data partition's files their SELinux labels under way, or
    /// how it last ended.
    pub relabel: Option<RelabelJob>,
    /// The filesystem page's partitions, as the runtime last read them, or
    /// why it could not; none until it has.
    pub filesystems: Option<Result<Vec<FsRow>, String>>,
    /// Which of them the page for one shows, and what it is to do to it:
    /// a repair, which leaves what it holds as it is, until another is
    /// chosen.
    pub fs_chosen: usize,
    pub fs_action: FsAction,
    /// Work on whole partitions under way, or how it last ended.
    pub part_work: Option<PartWork>,
    /// Where an update the system installed stands, as the runtime last
    /// read it.
    pub update: Standing,
    /// The WLAN, as the runtime last told it; none on a phone whose WLAN the
    /// recovery does not bring up.
    pub wlan: Option<Wlan>,
    /// Whether the screen lock that opened the user's files is kept for the
    /// starts that follow, so the decrypt page needs nothing typed.
    pub remember: bool,
    /// The folder served to the network, where it is served and how many
    /// requests it has answered; and whether it is being chosen, the file
    /// manager shown for it.
    pub serving: Serving,
    pub picking: bool,
    /// Both of verified boot's checks in the slot the phone runs from, as
    /// read, as the page has turned them before the swipe, and what the last
    /// write said.
    pub avb: Option<Result<(bool, bool), String>>,
    pub avb_pending: Option<(bool, bool)>,
    pub avb_notice: Option<String>,
    /// What KernelSU and the phone say, and the KMI chosen to write.
    pub kernel_su: Option<Result<KernelSuStatus, String>>,
    pub kernel_su_kmi: Option<usize>,
    /// A `kernelsu.ko` the user chose rather than the one the KMI names.
    pub kernel_su_ko: Option<String>,
    /// Writing KernelSU, or how it last ended.
    pub kernel_su_job: Option<KernelSuJob>,
    /// 正在下的这一份：已下字节、总字节、每秒字节。
    pub root_bytes: Option<(u64, Option<u64>, u64)>,
    /// 在线上的最新版本号（问过 GitHub 才有）。
    pub latest_kernelsu: Option<String>,
    pub latest_magisk: Option<String>,
    /// 问过几次都没问到（开机时还没联网），页面写「未获取」而不是一直「读取中」。
    pub latest_root_failed: bool,
    /// What Magisk and the phone say, and writing it.
    pub magisk: Option<Result<MagiskStatus, String>>,
    pub magisk_job: Option<KernelSuJob>,
    /// An update installed from the pages patches the new slot's boot with
    /// the chosen root manager.
    pub auto_root: AutoRoot,
    /// The address typed on the download page, how its download stands, and
    /// where the page another device types an address for it is.
    pub fetch_url: String,
    pub fetch: Fetch,
    pub fetch_portal: Option<String>,
    /// Whether the WLAN opens as the recovery starts, and whether the
    /// password of the network joined last is kept with the network itself.
    pub wlan_auto: bool,
    pub wlan_keep: bool,
    /// What the runtime last read from the charger: the power it delivers,
    /// the protocol it negotiated, and the most it has given this charge.
    pub charge: ChargeNow,
    /// The battery's temperature in tenths of a degree, as the charge page
    /// last read it, which the battery card shows.
    pub battery_temp: Option<i32>,
    /// The charge step kept, and the thermal guard's own two settings.
    pub charge_mode: ChargeMode,
    pub thermal_guard: bool,
    pub thermal_limit: u8,
    /// 记住的几张网（名字、SSID、保护方式），最近用过的在前；开机按它挨个试。
    pub wlan_saved: Vec<SavedWlan>,
    /// The network the password page is for, and what is typed there.
    pub joining: Option<Network>,
    /// The network the details page is of, as it was found.
    pub details: Option<Network>,
    /// The network the page that adds one is adding.
    pub adding: Option<Adding>,
    /// The password typed on the password page or the page that adds a
    /// network, and whether it shows.
    pub password: String,
    pub show_password: bool,
    /// 手动 IP 页上填的值（地址、掩码、网关、DNS），以及键盘现在进哪一格。
    pub static_ip: crate::settings::StaticV4,
    pub static_field: Field,
    /// 手动 IP 现在生效，而不是 DHCP。2026-11-27。
    pub static_on: bool,
    /// 自己建的首页分类，想建几个建几个。2026-11-27。
    pub home_sections: Vec<Section>,
    /// 单独放在首页最上面的条目（不进任何分类）。
    pub home_top: Vec<Entry>,
    /// 没进任何分类、也不置顶的条目，落在首页最下面的「其他」里。
    pub home_rest: Vec<Entry>,
    /// 「其他」在首页折起来了，以及它的名字（空着用「其他」）。
    pub home_rest_collapsed: bool,
    pub home_rest_name: String,
    /// 排序页拎着的那一条。
    pub home_pick: Option<Entry>,
    /// 正在改名/新建的那个分类：`None` 是新建。
    pub section_edit: Option<usize>,
    /// 二级页正在看的那一栏。
    pub section_view: Option<usize>,
    /// 改名页上打的字，与挑的图标。
    pub section_name: String,
    pub section_icon: crate::icons::Icon,
    /// 名字页改的是「其他」而不是某个分类（它不能删）。2026-11-27。
    pub section_rest: bool,
    /// The phone maker's update server is known, and the update page asks
    /// it.
    pub update_server: bool,
    pub check: Check,
    pub have: Have,
    /// Downloading the newest system's full package, or how that last
    /// ended.
    pub download: Option<DownloadJob>,
    pub skyward_check: SkywardCheck,
    /// Updating Skyward, or how that last ended.
    pub skyward_job: Option<SkywardJob>,
    /// How fast data went over the network joined in the last second.
    pub traffic: Traffic,
}

impl Default for State {
    fn default() -> Self {
        State {
            lang: Lang::Hans,
            language_chosen: false,
            dark: false,
            brightness: SLIDER_START,
            // Two minutes.
            screen_off: 3,
            follow_system_zone: true,
            time_zone: listed(FIRST_ZONE).expect("the first zone is one listed"),
            system_zone: None,
            now: 0,
            vibrate: true,
            vibration: SLIDER_START,
            file_transfer: true,
            entry_names: std::collections::BTreeMap::new(),
            home_title: String::new(),
            home_title_edit: String::new(),
            layouts: None,
            layout_dir: Folder::new("/sdcard"),
            layout_exporting: true,
            home_rest_at: None,
            entry_edit: None,
            entry_name: String::new(),
            transfer_running: false,
            reboot: 0,
            wipe: [false; WIPES.len()],
            cache_partition: false,
            phone_logs: 0,
            pattern: Vec::new(),
            confirm: 0.0,
            install: Install::Idle,
            source: Source::Computer,
            package_name: None,
            script_output: Vec::new(),
            check_signature: false,
            recovery_partitions: false,
            keep_recovery: true,
            reflash: None,
            mounts: Ok(Vec::new()),
            mount_failure: None,
            backup: [true, true, false, false, true],
            backup_to: Place::Storage,
            compress: false,
            verify: true,
            backups: Vec::new(),
            restore: None,
            backup_job: None,
            logs: [true, true],
            logs_to: 0,
            boot_slots: None,
            next_slot: None,
            dual_boot: false,
            updates: true,
            modules: None,
            module_at: None,
            apps: None,
            app_at: None,
            app_icons: Vec::new(),
            module_page: 0,
            app_page: 0,
            apps_scope: AppsScope::User,
            sys_apps: None,
            sys_page: 0,
            app_disabled: None,
            app_state_queued: std::collections::BTreeMap::new(),
            app_page_taps: 0,
            app_arrivals: 0,
            page_tap: None,
            page_miss: None,
            page_pending: None,
            pages_shown: 0,
            app_confirm: None,
            module_confirm: None,
            place_ask: None,
            systems: None,
            system_pick: None,
            systems_work: None,
            system_notice: None,
            data: Data::Closed,
            data_phase: 0.0,
            prepared: false,
            code: String::new(),
            search: String::new(),
            search_scope: SearchScope::Features,
            layer: Layer::Letters,
            ime: crate::pinyin::Composition::default(),
            partitions: None,
            record: Record::default(),
            tiles: DEFAULT.to_vec(),
            dump_pick: None,
            image_copies: None,
            image_pick: None,
            image_action: ImageAction::Restore,
            vbmeta_copies: None,
            vbmeta_pick: None,
            vbmeta_asking: false,
            held: None,
            files: Folder::new("/"),
            otg: Folder::new(""),
            asking_remember: false,
            file_on_drive: false,
            date: String::new(),
            sort_panel: false,
            sort: SortKey::Name,
            sort_desc: false,
            folders_first: true,
            storage: Folder::new(""),
            usb_drive: None,
            drive: Folder::new(""),
            file: None,
            fastboot: Fastboot { running: false, link: UsbLink::Unplugged },
            erase: Erase::default(),
            asked: Asked::default(),
            clip: None,
            new_name: String::new(),
            editor: None,
            asking_delete: false,
            avatar: None,
            device_name: None,
            choosing: false,
            chosen: std::collections::BTreeSet::new(),
            asking_many: None,
            clash: None,
            job: None,
            relabel: None,
            filesystems: None,
            fs_chosen: 0,
            fs_action: FsAction::Repair,
            part_work: None,
            update: Standing::None,
            wlan: None,
            remember: false,
            serving: Serving::Off,
            picking: false,
            avb: None,
            avb_pending: None,
            avb_notice: None,
            kernel_su: None,
            kernel_su_kmi: None,
            kernel_su_ko: None,
            kernel_su_job: None,
            root_bytes: None,
            latest_kernelsu: None,
            latest_magisk: None,
            latest_root_failed: false,
            magisk: None,
            magisk_job: None,
            auto_root: AutoRoot::None,
            fetch_url: String::new(),
            fetch: Fetch::Idle,
            fetch_portal: None,
            wlan_auto: false,
            wlan_keep: false,
            charge: ChargeNow::default(),
            battery_temp: None,
            charge_mode: ChargeMode::default(),
            thermal_guard: true,
            thermal_limit: THERMAL_LIMIT_DEFAULT,
            wlan_saved: Vec::new(),
            joining: None,
            details: None,
            adding: None,
            password: String::new(),
            show_password: false,
            static_ip: crate::settings::StaticV4 { address: String::new(), mask: "255.255.255.0".into(), gateway: String::new(), dns: String::new() },
            static_field: Field::Address,
            static_on: false,
            home_sections: Vec::new(),
            home_top: Vec::new(),
            home_rest: crate::app::home::default_rest(),
            home_rest_collapsed: false,
            home_rest_name: String::new(),
            home_pick: None,
            section_edit: None,
            section_view: None,
            section_name: String::new(),
            section_icon: crate::icons::Icon::Layers,
            section_rest: false,
            update_server: false,
            check: Check::Idle,
            have: Have::Nothing,
            download: None,
            skyward_check: SkywardCheck::Idle,
            skyward_job: None,
            traffic: Traffic::default(),
        }
    }
}

impl State {
    /// The key of the on-screen keyboard a finger holds down.
    pub fn key_held(&self) -> Option<Cap> {
        match self.held {
            Some(Action::Type(cap)) => Some(cap),
            _ => None,
        }
    }

    /// The folder the list on page `screen` shows: the file manager's, or
    /// a place's to choose a file to install in.
    pub fn folder(&self, screen: Screen) -> &Folder {
        match screen {
            Screen::Files => &self.files,
            Screen::FilesOtg => &self.otg,
            Screen::Choose(Place::Storage) => &self.storage,
            Screen::Choose(Place::UsbDrive) => &self.drive,
            other => unreachable!("{other:?} lists no folder"),
        }
    }

    pub(super) fn folder_mut(&mut self, screen: Screen) -> &mut Folder {
        match screen {
            Screen::Files => &mut self.files,
            Screen::FilesOtg => &mut self.otg,
            Screen::Choose(Place::Storage) => &mut self.storage,
            Screen::Choose(Place::UsbDrive) => &mut self.drive,
            Screen::LayoutBrowse => &mut self.layout_dir,
            other => unreachable!("{other:?} lists no folder"),
        }
    }

    /// Whether the files of `place` can be read: internal storage once it
    /// is open, a USB drive once the runtime shows it.
    /// The folder the page shows, as the phone knows it: the OTG page keeps
    /// the folder it shows from the top of the drive, and the file manager the
    /// whole of the path.
    pub fn folder_here(&self, screen: Screen) -> String {
        match screen {
            Screen::FilesOtg => {
                let at = self.otg.path.trim_matches('/');
                match at.is_empty() {
                    true => super::USB_DRIVE.to_string(),
                    false => format!("{}/{}", super::USB_DRIVE, at),
                }
            }
            // A page that lists no folder has nothing to paste into.
            other => match other {
                Screen::Files => self.files.path.clone(),
                Screen::Choose(Place::Storage) => self.storage.path.clone(),
                Screen::Choose(Place::UsbDrive) => self.drive.path.clone(),
                _ => String::new(),
            },
        }
    }

    pub fn place_open(&self, place: Place) -> bool {
        match place {
            Place::Storage => self.data.storage_open(),
            Place::UsbDrive => self.usb_drive == Some(UsbDrive::Shown),
        }
    }

    /// Whether a network is joined and has given an address.
    pub fn online(&self) -> bool {
        matches!(&self.wlan, Some(Wlan { joined: Some(j), .. }) if matches!(j.stage, Joining::Connected(_)))
    }

    /// The newest system the update server told of.
    pub fn newest(&self) -> Option<&Release> {
        match &self.check {
            Check::Answered(Some(r)) => Some(r),
            _ => None,
        }
    }

    /// Whether the full package is being downloaded.
    pub fn downloading(&self) -> bool {
        self.download.as_ref().is_some_and(|d| d.outcome.is_none())
    }

    /// Skyward's newest release, as GitHub told of it.
    pub fn skyward_newest(&self) -> Option<&Newest> {
        match &self.skyward_check {
            SkywardCheck::Answered(n) => Some(n),
            _ => None,
        }
    }

    /// Whether Skyward is being updated.
    pub fn updating_skyward(&self) -> bool {
        self.skyward_job.as_ref().is_some_and(|j| j.outcome.is_none())
    }

    /// Whether the phone has the page `screen`: the WLAN page where the
    /// recovery brings the phone's WLAN up, and updating Skyward where the
    /// phone also keeps its recovery in a partition of each slot.
    /// The apps read for the page, or why they could not be.
    pub fn set_apps(&mut self, list: Result<Vec<crate::app::dataapp::DataApp>, String>) {
        self.apps = Some(list);
    }

    /// The modules read for the page, or why they could not be.
    pub fn set_modules(&mut self, rows: Result<Vec<ModuleRow>, String>) {
        self.modules = Some(rows);
    }

    pub fn offers(&self, screen: Screen) -> bool {
        match screen {
            Screen::Wlan => self.wlan.is_some(),
            Screen::SkywardUpdate => self.updates && self.wlan.is_some() && self.recovery_partitions,
            Screen::Systems => self.dual_boot,
            _ => true,
        }
    }
}
