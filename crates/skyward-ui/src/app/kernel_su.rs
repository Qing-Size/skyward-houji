//! KernelSU's page: what the phone runs, which KMI its module was built for,
//! and writing the chosen one into boot or init_boot.

use super::*;
use crate::texts;

/// One module the recovery can write, as the page lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KmiRow {
    /// `android<release>-<kernel>`, as KernelSU names it.
    pub id: String,
    pub android: u8,
    pub kernel: String,
    /// The partition it goes into by default.
    pub target: String,
}

/// 从 `v3.3.2`、`3.3.1-56-gcfc60f8d`、`27.0` 这样的写法里取出三段数字。
pub fn version_parts(text: &str) -> Option<(u32, u32, u32)> {
    let trimmed = text.trim().trim_start_matches(['v', 'V']);
    let mut parts = [0u32; 3];
    let mut at = 0;
    for piece in trimmed.split(['.', '-', '+', ' ']) {
        if at >= 3 {
            break;
        }
        let digits: String = piece.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            break;
        }
        parts[at] = digits.parse().ok()?;
        at += 1;
    }
    (at > 0).then_some((parts[0], parts[1], parts[2]))
}

/// 在线的那一版比装着的这一版新吗：新 → Some(true)，一样或旧 → Some(false)，
/// 有一边读不出来 → None。只升不降，页面据此拦下写入（2026-11-26）。
pub fn is_newer(installed: Option<&str>, online: Option<&str>) -> Option<bool> {
    let (installed, online) = (version_parts(installed?)?, version_parts(online?)?);
    Some(online > installed)
}

/// What the phone and KernelSU's payloads say before anything is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelSuStatus {
    /// KernelSU's userspace was found in the data partition.
    pub installed: bool,
    /// The stock image this recovery wrote KernelSU over is still there.
    pub has_backup: bool,
    /// The partition the record's backup came from.
    pub backup_target: Option<String>,
    pub version: Option<String>,
    /// The bootloader lets its boot images be written.
    pub unlocked: bool,
    /// The slot whose boot image is written, where the phone has slots.
    pub slot: Option<String>,
    /// The row the kernel's own version picks, where it picks one.
    pub default: Option<usize>,
    pub kmis: Vec<KmiRow>,
}

/// How far writing KernelSU has come.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelSuStage {
    Preparing,
    Patching,
    BackingUp,
    Writing,
    Checking,
}

/// How writing KernelSU ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KernelSuEnd {
    Installed { target: String, backup: String, manager: String },
    Restored { target: String },
    Failed(String),
}

/// Writing KernelSU, or how it last ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelSuJob {
    pub stage: KernelSuStage,
    pub outcome: Option<KernelSuEnd>,
    /// The latest release's version, once it has been downloaded.
    pub fetched: Option<String>,
    /// Why the latest release could not be fetched, when the built-in
    /// payloads were used instead.
    pub fallback: Option<String>,
}

impl KernelSuStage {
    pub fn done(self) -> bool {
        matches!(self, KernelSuStage::Checking)
    }
}

impl App {
    /// What the phone and KernelSU's payloads say, read as the page opens.
    pub fn set_kernel_su(&mut self, status: Result<KernelSuStatus, String>) {
        if let Ok(s) = &status
            && self.state.kernel_su_kmi.is_none()
        {
            self.state.kernel_su_kmi = s.default;
        }
        self.state.kernel_su = Some(status);
        self.refresh();
    }

    /// Choose the KMI to write, by its place in the list.
    pub fn set_kernel_su_kmi(&mut self, at: usize) {
        if self.state.kernel_su.as_ref().is_some_and(|s| s.as_ref().is_ok_and(|s| at < s.kmis.len())) {
            self.state.kernel_su_kmi = Some(at);
            self.refresh();
        }
    }

    /// Writing KernelSU has begun.
    pub fn kernel_su_started(&mut self) {
        self.state.kernel_su_job = Some(KernelSuJob { stage: KernelSuStage::Preparing, outcome: None, fetched: None, fallback: None });
        self.refresh();
    }

    /// How far writing KernelSU has come.
    pub fn kernel_su_progress(&mut self, stage: KernelSuStage) {
        let job = self.state.kernel_su_job.get_or_insert(KernelSuJob { stage, outcome: None, fetched: None, fallback: None });
        if job.outcome.is_none() {
            job.stage = stage;
            self.refresh();
        }
    }

    /// The latest release has been downloaded; the page and the toast say
    /// which version is being written.
    pub fn kernel_su_fetched(&mut self, version: String, now: u64) {
        if let Some(job) = &mut self.state.kernel_su_job {
            job.fetched = Some(version.clone());
        }
        let said = format!("{} {version}", self.state.lang.pick(texts::kernel_su::DOWNLOADED));
        self.report(&said, now);
        self.refresh();
    }

    /// The latest release could not be fetched; the built-in payloads are
    /// used, and both the page and the toast say why.
    pub fn kernel_su_fallback(&mut self, why: String, now: u64) {
        if let Some(job) = &mut self.state.kernel_su_job {
            job.fallback = Some(why.clone());
        }
        let said = format!("{} {why}", self.state.lang.pick(texts::kernel_su::FALLBACK));
        self.report(&said, now);
        self.refresh();
    }

    /// Writing KernelSU ended, and the page says how.
    pub fn kernel_su_ended(&mut self, end: KernelSuEnd, now: u64) {
        let job = self.state.kernel_su_job.get_or_insert(KernelSuJob { stage: KernelSuStage::Checking, outcome: None, fetched: None, fallback: None });
        job.outcome = Some(end);
        self.wake(now);
        self.refresh();
    }

    /// A root image was not written, and the toast says why.
    pub fn root_failed(&mut self, why: &str, now: u64) {
        let said = format!("{} {why}", self.state.lang.pick(texts::kernel_su::FAILED));
        self.report(&said, now);
    }

    /// 下载进度与速度，页面照它写「42% · 1.2 MiB/s」。
    pub fn set_root_bytes(&mut self, done: u64, total: Option<u64>, rate: u64) {
        self.state.root_bytes = Some((done, total, rate));
        self.refresh();
    }

    /// 在线上的两个最新版本号，页面先告诉用户会装到哪一版。
    pub fn set_latest_root(&mut self, kernelsu: Option<String>, magisk: Option<String>) {
        self.state.latest_kernelsu = kernelsu;
        self.state.latest_magisk = magisk;
        self.refresh();
    }

    /// 问了几次都没问到：页面写「未获取」，不再是「读取中」。
    pub fn mark_latest_unavailable(&mut self) {
        self.state.latest_root_failed = true;
        self.refresh();
    }

    /// 下载结束（成功、失败或取消），不再显示进度。
    pub fn clear_root_bytes(&mut self) {
        self.state.root_bytes = None;
        self.refresh();
    }

    /// The KMI the page would write.
    pub fn kernel_su_kmi(&self) -> Option<&KmiRow> {
        let status = self.state.kernel_su.as_ref()?.as_ref().ok()?;
        let at = self.state.kernel_su_kmi.or(status.default)?;
        status.kmis.get(at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use crate::app::{Confirm, Data, Effect};

    fn status() -> KernelSuStatus {
        KernelSuStatus {
            installed: false,
            has_backup: false,
            backup_target: None,
            version: None,
            unlocked: true,
            slot: Some("B".into()),
            default: Some(1),
            kmis: vec![
                KmiRow { id: "android13-5.10".into(), android: 13, kernel: "5.10".into(), target: "boot".into() },
                KmiRow { id: "android17-6.18".into(), android: 17, kernel: "6.18".into(), target: "init_boot".into() },
            ],
        }
    }

    #[test]
    fn an_online_version_is_read_and_compared_with_the_installed_one() {
        assert_eq!(version_parts("v3.3.2"), Some((3, 3, 2)));
        assert_eq!(version_parts("3.3.1-56-gcfc60f8d"), Some((3, 3, 1)));
        assert_eq!(version_parts("27.0"), Some((27, 0, 0)));
        assert_eq!(version_parts("nightly"), None);
        assert_eq!(is_newer(Some("3.3.1-56-gcfc60f8d"), Some("v3.3.2")), Some(true), "升级");
        assert_eq!(is_newer(Some("3.3.2"), Some("v3.3.2")), Some(false), "一样不算升级");
        assert_eq!(is_newer(Some("3.3.3"), Some("v3.3.2")), Some(false), "不降级");
        assert_eq!(is_newer(None, Some("v3.3.2")), None, "没装过就不比");
    }

    #[test]
    fn the_page_opens_on_the_kernel_s_default_and_takes_another_kmi() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::KernelSu, W, H, PPD);
        app.set_kernel_su(Ok(status()));
        assert_eq!(app.kernel_su_kmi().unwrap().id, "android17-6.18");
        // The KMIs are on the page itself: tapping one writes that KMI.
        assert!(tap(&mut app, &f, &i, Action::KernelSuKmi(0)).is_empty());
        assert_eq!(app.screen(), Screen::KernelSu);
        assert_eq!(app.kernel_su_kmi().unwrap().id, "android13-5.10");
    }

    #[test]
    fn writing_the_chosen_kmi_asks_for_it_and_locks_the_page_until_it_ends() {
        let mut app = App::new(Screen::KernelSu, W, H, PPD);
        app.set_kernel_su(Ok(status()));
        app.state.data = Data::Open;
        let fx = app.confirmed(Confirm::KernelSu, 0);
        assert_eq!(fx, vec![Effect::Haptic, Effect::InstallKernelSu { kmi: "android17-6.18".into(), module: None }]);
        assert!(app.state.kernel_su_job.is_some());
        assert_eq!(app.confirmed(Confirm::KernelSu, 0), vec![Effect::Haptic], "the write is not started twice");
    }

    #[test]
    fn a_chosen_local_module_goes_with_the_kmi() {
        let mut app = App::new(Screen::KernelSu, W, H, PPD);
        app.set_kernel_su(Ok(status()));
        app.state.data = Data::Open;
        app.state.kernel_su_ko = Some("/sdcard/Download/kernelsu.ko".into());
        assert_eq!(
            app.confirmed(Confirm::KernelSu, 0),
            vec![Effect::Haptic, Effect::InstallKernelSu {
                kmi: "android17-6.18".into(),
                module: Some("/sdcard/Download/kernelsu.ko".into()),
            }]
        );
    }

    #[test]
    fn a_locked_bootloader_is_refused_before_anything_is_written() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::KernelSu, W, H, PPD);
        let mut status = status();
        status.unlocked = false;
        app.set_kernel_su(Ok(status));
        app.state.data = Data::Open;
        assert!(tap(&mut app, &f, &i, Action::Confirm(Confirm::KernelSu)).is_empty());
        assert!(app.state.kernel_su_job.is_none());
    }

    #[test]
    fn writing_reports_its_stages_and_end_without_losing_them() {
        let mut app = App::new(Screen::KernelSu, W, H, PPD);
        app.kernel_su_started();
        app.kernel_su_progress(KernelSuStage::Patching);
        assert_eq!(app.state.kernel_su_job.as_ref().unwrap().stage, KernelSuStage::Patching);
        app.kernel_su_ended(
            KernelSuEnd::Installed {
                target: "init_boot".into(),
                backup: "/data/media/0/Skyward/root/a.img".into(),
                manager: "/sdcard/Download/Skyward/KernelSU.apk".into(),
            },
            0,
        );
        assert!(matches!(app.state.kernel_su_job.as_ref().unwrap().outcome, Some(KernelSuEnd::Installed { .. })));
    }
}
