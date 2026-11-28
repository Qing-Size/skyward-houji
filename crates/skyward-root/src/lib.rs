//! The KernelSU KMIs the recovery may write, by the Android release and
//! kernel each was built for; the payloads themselves (ksud, the module, the
//! manager APK) are fetched from GitHub when a root is written, so nothing
//! of them is carried in the image (2026-11-26).
//!
//! Magisk's APK — its boot patcher, programs, scripts and manager — is
//! fetched the same way.

/// KernelSU 的最新发行，写 Root 时从这里取 ksud、对应 KMI 的模块与管理器。
pub const KERNELSU_LATEST: &str = "https://api.github.com/repos/tiann/KernelSU/releases/latest";

/// Magisk 的最新发行，写 Root 时从这里取 APK（补丁器就在里面）。
pub const MAGISK_LATEST: &str = "https://api.github.com/repos/topjohnwu/Magisk/releases/latest";

/// Where KernelSU's installer is told the boot image's ramdisk lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Boot,
    InitBoot,
}

impl Target {
    pub fn name(self) -> &'static str {
        match self {
            Target::Boot => "boot",
            Target::InitBoot => "init_boot",
        }
    }

    /// The partition KernelSU's LKM goes into, as the kernel's own release
    /// says: 5.10 keeps its ramdisk in boot, the later GKI kernels in
    /// init_boot. The phone's real layout is checked before anything is
    /// written; this is only the default the page offers.
    pub fn for_kernel(kernel: &str) -> Option<Target> {
        match kernel_minor(kernel)? {
            "5.10" => Some(Target::Boot),
            _ => Some(Target::InitBoot),
        }
    }
}

/// One KMI, and the phone it was built for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kmi {
    /// `android<release>-<kernel>`, as KernelSU names it.
    pub id: &'static str,
    /// The Android release, in the number `ro.build.version.release` holds.
    pub android: u8,
    /// The kernel's major and minor version, such as `6.1`.
    pub kernel: &'static str,
    /// The partition the module is put into by default.
    pub target: Target,
}

macro_rules! kmi {
    ($id:literal, $android:literal, $kernel:literal, $target:ident) => {
        Kmi { id: $id, android: $android, kernel: $kernel, target: Target::$target }
    };
}

/// Every LKM KernelSU ships, in the order it lists its KMIs.
pub static KMIS: &[Kmi] = &[
    kmi!("android12-5.10", 12, "5.10", Boot),
    kmi!("android13-5.10", 13, "5.10", Boot),
    kmi!("android13-5.15", 13, "5.15", InitBoot),
    kmi!("android14-5.15", 14, "5.15", InitBoot),
    kmi!("android14-6.1", 14, "6.1", InitBoot),
    kmi!("android15-6.6", 15, "6.6", InitBoot),
    kmi!("android16-6.12", 16, "6.12", InitBoot),
    kmi!("android17-6.18", 17, "6.18", InitBoot),
];

/// The manager's own file name, as KernelSU names the release asset.
pub const MANAGER_NAME: &str = "KernelSU.apk";

/// The manager's file name where it is saved for the user.
pub const MAGISK_MANAGER_NAME: &str = "Magisk.apk";

/// The module named `id`, if the release has it.
pub fn find(id: &str) -> Option<&'static Kmi> {
    KMIS.iter().find(|k| k.id == id)
}

/// The module to offer first for a phone whose Android release is `android`
/// and whose kernel release is `kernel`. A phone with an unusual Android
/// release but a known kernel still gets the kernel's modules; when neither
/// pair is there, the caller lets the user choose.
pub fn default(android: Option<&str>, kernel: &str) -> Option<&'static Kmi> {
    let kernel = kernel_minor(kernel)?;
    let android = android.and_then(android_major);
    if let Some(android) = android
        && let Some(k) = KMIS.iter().find(|k| k.kernel == kernel && k.android == android)
    {
        return Some(k);
    }
    // A release that KernelSU did not build for this exact Android version
    // falls back to the newest release it did build for that kernel, and
    // then to any module of the kernel. The page says which one it picked.
    if let Some(android) = android
        && let Some(k) = KMIS.iter().rev().find(|k| k.kernel == kernel && k.android <= android)
    {
        return Some(k);
    }
    KMIS.iter().find(|k| k.kernel == kernel)
}

/// The kernel's `major.minor`, as one of the KMI names, from a release such
/// as `6.1.75-android14-11-g...`.
pub fn kernel_minor(release: &str) -> Option<&'static str> {
    let mut numbers = release.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty());
    let major: u32 = numbers.next()?.parse().ok()?;
    let minor: u32 = numbers.next()?.parse().ok()?;
    Some(match (major, minor) {
        (5, 10) => "5.10",
        (5, 15) => "5.15",
        (6, 1) => "6.1",
        (6, 6) => "6.6",
        (6, 12) => "6.12",
        (6, 18) => "6.18",
        _ => return None,
    })
}

/// The Android release's number, from `17` or a string such as `Android 17`.
pub fn android_major(release: &str) -> Option<u8> {
    release.split(|c: char| !c.is_ascii_digit()).find(|s| !s.is_empty())?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kmi_is_named_once_and_takes_the_partition_its_kernel_says() {
        assert_eq!(KMIS.len(), 8);
        for (i, k) in KMIS.iter().enumerate() {
            assert_eq!(k.target, Target::for_kernel(k.kernel).unwrap());
            assert!(KMIS[..i].iter().all(|other| other.id != k.id), "{} is named once", k.id);
        }
    }

    #[test]
    fn the_kernel_and_android_release_pick_the_module_to_offer() {
        assert_eq!(default(Some("17"), "6.18.12-android17").unwrap().id, "android17-6.18");
        assert_eq!(default(Some("13"), "5.10.198-android13").unwrap().id, "android13-5.10");
        assert_eq!(default(Some("14"), "5.10.198-android12").unwrap().id, "android13-5.10", "a newer Android release on the same kernel");
        assert_eq!(default(None, "6.1.75").unwrap().id, "android14-6.1");
        assert_eq!(default(Some("12"), "4.19.1").map(|k| k.id), None, "a kernel KernelSU does not ship");
    }

    #[test]
    fn the_target_follows_the_kernel_and_can_be_overridden_by_the_page() {
        assert_eq!(Target::for_kernel("5.10.198"), Some(Target::Boot));
        assert_eq!(Target::for_kernel("5.15.1"), Some(Target::InitBoot));
        assert_eq!(Target::for_kernel("6.1.75"), Some(Target::InitBoot));
        assert_eq!(Target::for_kernel("4.19.1"), None);
        assert_eq!(find("android15-6.6").unwrap().target, Target::InitBoot);
        assert!(find("android18-7.0").is_none());
    }

    #[test]
    fn the_release_strings_are_read_from_the_phone_s_own_words() {
        assert_eq!(kernel_minor("6.12.30-android16-8-g1234"), Some("6.12"));
        assert_eq!(kernel_minor("5.10.198"), Some("5.10"));
        assert_eq!(android_major("Android 17"), Some(17));
        assert_eq!(android_major("17"), Some(17));
    }
}
