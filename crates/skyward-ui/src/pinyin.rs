//! 中文输入：AOSP 的拼音解码器（`third_party/aosp-pinyin`，Apache-2.0）。
//!
//! 词典不预置在仓库里：`build.rs` 先在主机上编出 AOSP 自带的
//! `pinyinime_dictbuilder`，跑一遍把 `rawdict_utf16_65105_freq.txt` 压成
//! `dict_pinyin.dat`，再随二进制嵌进来；解码器只认文件，所以启动时把它写进
//! 一个私有临时目录，用户词典也落在那里，进程退出即删。
//!
//! 编不出来（主机上没有 POSIX 头，或者 `pinyin-input` 关着）的时候，候选恒为
//! 空，页面照样能打字母：没有候选时把那串字母原样落下。

#[cfg(feature = "pinyin-input")]
use std::{
    ffi::CString,
    sync::{Mutex, OnceLock},
};

/// 解码器的上限：一次最多问几个字、多长的串（与 C 适配层一致）。
#[cfg(feature = "pinyin-input")]
const MAX_SPELLING: usize = 32;
#[cfg(feature = "pinyin-input")]
const MAX_CHARS: usize = 16;
#[cfg(feature = "pinyin-input")]
const MAX_CANDIDATES: usize = 24;

/// 一条候选：写进输入框的字，和它从拼音串里去掉的字节数。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    pub consumed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Composition {
    pub enabled: bool,
    pub spelling: String,
    pub page: usize,
    pub candidates: Vec<Candidate>,
}

impl Default for Composition {
    /// 默认关着：打开输入框先打英文，按一下「中/EN」才走拼音。这样密码、网址
    /// 这些页不会被拼音吃掉按键（这条键盘处处可用，不能默认把字母收进拼写）。
    fn default() -> Self {
        Composition { enabled: false, spelling: String::new(), page: 0, candidates: Vec::new() }
    }
}

impl Composition {
    pub fn update(&mut self) {
        self.candidates = candidates(&self.spelling);
        self.page = 0;
    }
    pub fn clear(&mut self) {
        self.spelling.clear();
        self.candidates.clear();
        self.page = 0;
    }
    pub fn choose(&mut self, index: usize) -> Option<String> {
        let candidate = self.candidates.get(index)?.clone();
        self.spelling.drain(..candidate.consumed.min(self.spelling.len()));
        self.spelling = self.spelling.trim_start_matches('\'').into();
        self.update();
        Some(candidate.text)
    }
}

#[cfg(feature = "pinyin-input")]
unsafe extern "C" {
    fn im_open_decoder(system: *const std::ffi::c_char, user: *const std::ffi::c_char) -> bool;
    fn im_set_max_lens(spelling: usize, characters: usize);
    fn skyward_pinyin_query(input: *const std::ffi::c_char, index: usize, output: *mut u16, capacity: usize, consumed: *mut usize)
    -> usize;
}

#[cfg(feature = "pinyin-input")]
struct Engine {
    _files: tempfile::TempDir,
}

/// 引擎为什么没打开 —— 页面那边只会看到「候选是空的」，原因记在这里，
/// 由 runtime 抄进 kmsg，`adb shell dmesg | grep pinyin` 就能看见。
#[cfg(feature = "pinyin-input")]
static TROUBLE: Mutex<Option<String>> = Mutex::new(None);

#[cfg(feature = "pinyin-input")]
fn trouble(why: impl Into<String>) -> Option<Engine> {
    let why = why.into();
    if let Ok(mut slot) = TROUBLE.lock() {
        *slot = Some(why);
    }
    None
}

/// 临时目录：先 `/tmp`（recovery 的 rootfs 里一直有，上游也用这个），退而用
/// `TMPDIR` / 系统默认。不能一步 `tempdir()`：安卓上 `TMPDIR` 常常是
/// `/data/local/tmp`，recovery 里 `/data` 还没挂，那样建目录必失败。
#[cfg(feature = "pinyin-input")]
fn temporary() -> Result<tempfile::TempDir, String> {
    let mut tried = Vec::new();
    for dir in ["/tmp", ""] {
        let mut builder = tempfile::Builder::new();
        builder.prefix("skyward-pinyin-");
        let made = if dir.is_empty() { builder.tempdir() } else { builder.tempdir_in(dir) };
        match made {
            Ok(files) => return Ok(files),
            Err(e) => tried.push(format!("{}: {e}", if dir.is_empty() { "TMPDIR" } else { dir })),
        }
    }
    Err(format!("no usable temporary directory（{}）", tried.join(";")))
}

#[cfg(feature = "pinyin-input")]
impl Engine {
    fn open() -> Option<Self> {
        let files = match temporary() {
            Ok(files) => files,
            Err(why) => return trouble(why),
        };
        let system = files.path().join("system.dat");
        let user = files.path().join("user.dat");
        let dictionary = include_bytes!(concat!(env!("OUT_DIR"), "/res/raw/dict_pinyin.dat"));
        if let Err(e) = std::fs::write(&system, dictionary) {
            return trouble(format!("writing the dictionary to {}: {e}", system.display()));
        }
        let (system, user) = (CString::new(system.to_str()?).ok()?, CString::new(user.to_str()?).ok()?);
        if !unsafe { im_open_decoder(system.as_ptr(), user.as_ptr()) } {
            return trouble("im_open_decoder refused the dictionary");
        }
        unsafe {
            im_set_max_lens(MAX_SPELLING, MAX_CHARS);
        }
        Some(Self { _files: files })
    }
}

#[cfg(feature = "pinyin-input")]
fn engine() -> &'static Mutex<Option<Engine>> {
    static ENGINE: OnceLock<Mutex<Option<Engine>>> = OnceLock::new();
    ENGINE.get_or_init(|| Mutex::new(Engine::open()))
}

/// 现在就去开一次引擎，把它没开成的原因带回去（没有就是 None）。
/// 开机后由 runtime 叫一次：这样 dmesg 里能直接看出是字典、临时目录还是
/// 解码器本身的问题，不必等用户先按一遍键盘。
#[cfg(feature = "pinyin-input")]
pub fn prepare() -> Option<String> {
    let _opened = engine().lock().ok()?;
    TROUBLE.lock().ok().and_then(|slot| slot.clone())
}

/// 没有引擎的构建：永远没有原因可报。
#[cfg(not(feature = "pinyin-input"))]
pub fn prepare() -> Option<String> {
    None
}

#[cfg(feature = "pinyin-input")]
static LAST: Mutex<Option<(String, Vec<Candidate>)>> = Mutex::new(None);

#[cfg(not(feature = "pinyin-input"))]
pub fn candidates(_: &str) -> Vec<Candidate> {
    Vec::new()
}

#[cfg(feature = "pinyin-input")]
pub fn candidates(spelling: &str) -> Vec<Candidate> {
    if spelling.is_empty() || spelling.len() > MAX_SPELLING || !spelling.bytes().all(|c| c.is_ascii_lowercase() || c == b'\'') {
        return Vec::new();
    }
    // 一帧里要问好几遍（页码一遍、排版一遍），而解码器一次搜索不便宜。
    if let Ok(last) = LAST.lock()
        && let Some((asked, picks)) = last.as_ref()
        && asked == spelling
    {
        return picks.clone();
    }
    let result = decode(spelling);
    if let Ok(mut last) = LAST.lock() {
        *last = Some((spelling.to_string(), result.clone()));
    }
    result
}

#[cfg(feature = "pinyin-input")]
fn decode(spelling: &str) -> Vec<Candidate> {
    let Ok(engine) = engine().lock() else {
        return Vec::new();
    };
    if engine.is_none() {
        return Vec::new();
    }
    let Ok(input) = CString::new(spelling) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for index in 0..MAX_CANDIDATES {
        let mut text = [0u16; MAX_CHARS + 1];
        let mut consumed = 0;
        let count = unsafe { skyward_pinyin_query(input.as_ptr(), index, text.as_mut_ptr(), text.len(), &mut consumed) };
        if index >= count {
            break;
        }
        let end = text.iter().position(|c| *c == 0).unwrap_or(text.len());
        if end > 0
            && consumed > 0
            && let Ok(text) = String::from_utf16(&text[..end])
        {
            result.push(Candidate { text, consumed });
        }
    }
    result
}

#[cfg(all(test, feature = "pinyin-input"))]
mod tests {
    use super::*;
    #[test]
    fn real_dictionary_decodes_chinese_and_keeps_a_bounded_candidate_list() {
        let result = candidates("nihao");
        assert!(result.iter().any(|c| c.text == "你好"), "{result:?}");
        assert!(result.len() <= MAX_CANDIDATES);
        for invalid in ["", "../../data", "你", "ni hao", "0123"] {
            assert!(candidates(invalid).is_empty());
        }
        assert!(candidates(&"a".repeat(33)).is_empty());
    }
    #[test]
    fn choosing_a_phrase_consumes_its_spelling_without_committing_raw_letters() {
        let mut composition = Composition { spelling: "nihao".into(), ..Composition::default() };
        composition.update();
        let index = composition.candidates.iter().position(|c| c.text == "你好").unwrap();
        assert_eq!(composition.choose(index).as_deref(), Some("你好"));
        assert!(composition.spelling.is_empty());
    }
}
