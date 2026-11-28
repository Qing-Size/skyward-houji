//! 中文输入：键盘上方的拼写条，把字母拼成拼音、再选成汉字。
//!
//! 只有打开了输入框的页面才有这条：搜索、重命名、WLAN 密码、添加网络、
//! 下载地址，以及锁屏密码为字母的那一页。英文字母照旧走原来的键盘。

use super::*;

impl App {
    /// 这一页现在有输入框在等着打字。
    pub(super) fn text_context(&self) -> bool {
        match self.screen() {
            Screen::Search | Screen::Rename | Screen::WlanPassword | Screen::Fetch => true,
            Screen::WlanAdd => self.state.adding.is_some(),
            // 分类名也要能打中文（2026-11-27）。
            Screen::HomeSection => true,
            // 首页条目的名字一样能打中文（2026-11-27）。
            Screen::EntryName => true,
            // 首页那个大字也是（2026-11-27）。
            Screen::HomeTitle => true,
            // 手动 IP 那页也能切中/EN：数字键一样打，拼音不吃数字（2026-11-27）。
            Screen::WlanStatic => true,
            Screen::Editor => self.state.editor.is_some(),
            Screen::Terminal => true,
            Screen::Decrypt => matches!(self.state.data, Data::Locked { keys: Keys::Letters, checking: false, .. }),
            _ => false,
        }
    }

    /// 拼音条该画出来、也该接键。
    pub(super) fn ime_context(&self) -> bool {
        cfg!(feature = "pinyin-input") && self.text_context()
    }

    /// 把第 `index` 个候选字写进输入框；没有候选就把拼写本身写进去。
    pub(super) fn ime_commit(&mut self, index: usize) -> Vec<Effect> {
        if !self.ime_context() || self.state.ime.spelling.is_empty() {
            return Vec::new();
        }
        let text = self.state.ime.choose(index).unwrap_or_else(|| {
            let text = self.state.ime.spelling.clone();
            self.state.ime.clear();
            text
        });
        if self.screen() == Screen::Terminal {
            self.edits += 1;
            self.refresh();
            return match self.term.as_ref().filter(|term| term.running) {
                Some(_) => vec![Effect::TermInput(text.into_bytes())],
                None => Vec::new(),
            };
        }
        if self.screen() == Screen::Editor {
            if let Some(editor) = self.state.editor.as_mut() {
                for ch in text.chars() {
                    editor.insert(ch);
                }
            }
            self.edits += 1;
            self.refresh();
            return Vec::new();
        }
        self.typing().push_str(&text);
        self.edits += 1;
        self.refresh();
        Vec::new()
    }

    /// 拼音条上按下的是哪一格。
    pub(super) fn ime_action(&mut self, action: Action) {
        if !self.ime_context() {
            return;
        }
        match action {
            Action::ImeToggle => {
                self.ime_commit(0);
                self.state.ime.enabled = !self.state.ime.enabled;
                if !self.state.ime.enabled {
                    self.state.ime.clear();
                }
                self.refresh();
            }
            Action::ImePick(index) if index < self.state.ime.candidates.len() => {
                self.ime_commit(index);
            }
            Action::ImePage(next) => {
                let ime = &mut self.state.ime;
                let pages = ime.candidates.len().div_ceil(crate::ime::PER_PAGE).max(1);
                ime.page = if next { (ime.page + 1) % pages } else { (ime.page + pages - 1) % pages };
                self.refresh();
            }
            _ => {}
        }
    }

    /// 一个键先给拼音条：字母进拼写，退格退一个字母，空格或回车选第一个。
    /// 返回 true 表示这个键已经被拼音条吃掉，不再往输入框写字。
    pub(super) fn ime_key(&mut self, cap: Cap) -> bool {
        if !self.ime_context() || !self.state.ime.enabled {
            return false;
        }
        match cap {
            Cap::Char(ch) if ch.is_ascii_alphabetic() || ch == '\'' => {
                if self.state.ime.spelling.len() < 32 {
                    self.state.ime.spelling.push(ch.to_ascii_lowercase());
                    self.state.ime.update();
                }
                if self.state.layer == Layer::Shifted {
                    self.state.layer = Layer::Letters;
                }
                self.refresh();
                true
            }
            Cap::Backspace if !self.state.ime.spelling.is_empty() => {
                self.state.ime.spelling.pop();
                self.state.ime.update();
                self.refresh();
                true
            }
            Cap::Space | Cap::Enter if !self.state.ime.spelling.is_empty() => {
                self.ime_commit(0);
                true
            }
            Cap::Char(_) if !self.state.ime.spelling.is_empty() => {
                // 不是字母的键先把它拼的东西落下，这个键再照常走。
                self.ime_commit(0);
                false
            }
            _ => false,
        }
    }
}
