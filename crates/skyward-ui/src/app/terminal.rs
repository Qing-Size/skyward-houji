//! The terminal page: its shell's screen, and the keys that go to it.

use super::*;

/// How the terminal page's shell ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TermEnd {
    /// It exited with this code.
    Exited(i32),
    /// This signal stopped it.
    Signalled(i32),
    /// Neither, as the system says it.
    Other(String),
}

impl App {
    /// How many rows and columns the terminal's screen holds.
    pub fn term_size(&self, f: &Fonts) -> (u16, u16) {
        let t = self.theme();
        let l = term::layout(&t, f, t.m.content_top);
        (l.rows, l.cols)
    }

    /// A shell has started for the terminal page, on a blank screen of
    /// `rows` by `cols`.
    pub fn term_started(&mut self, rows: u16, cols: u16) {
        self.term = Some(Term::new(rows, cols));
        self.refresh();
    }

    /// What the shell wrote.
    pub fn term_output(&mut self, bytes: &[u8]) {
        let term = self.term.as_mut().expect("only a shell that started writes");
        term.feed(bytes);
        if self.screen() == Screen::Terminal {
            self.refresh();
        }
    }

    /// The shell has ended as `end` says; its screen says so, and that
    /// Enter starts another.
    pub fn term_ended(&mut self, end: TermEnd) {
        let (l, term) = (self.state.lang, self.term.as_mut().expect("only a shell that started ends"));
        let said = texts::terminal::restart(l, fmt::term_end(l, &end));
        term.feed(format!("\r\n[{said}]\r\n").as_bytes());
        term.running = false;
        if self.screen() == Screen::Terminal {
            self.refresh();
        }
    }

    /// A key of the keyboard on the terminal page. The layer keys change
    /// the keyboard; every other key goes to the shell, and once the shell
    /// has ended, Enter starts another.
    pub(super) fn term_typed(&mut self, cap: Cap) -> Vec<Effect> {
        match cap {
            Cap::Shift => self.state.layer = if self.state.layer == Layer::Shifted { Layer::Letters } else { Layer::Shifted },
            Cap::Show(layer) => self.state.layer = layer,
            _ => {}
        }
        let shifted = self.state.layer == Layer::Shifted;
        if shifted && matches!(cap, Cap::Char(_)) {
            self.state.layer = Layer::Letters;
        }
        self.refresh();
        let Some(term) = self.term.as_mut() else { return Vec::new() };
        if !term.running {
            return if cap == Cap::Enter { vec![Effect::StartShell(self.shell_folder.clone())] } else { Vec::new() };
        }
        let bytes = term.typed(cap, shifted);
        if bytes.is_empty() {
            Vec::new()
        } else {
            vec![Effect::TermInput(bytes)]
        }
    }

    /// Scroll the terminal's screen or the editor's text as a finger goes
    /// from `last` to `y`: a row for each row's height.
    pub(super) fn scroll_history(&mut self, last: f32, y: f32, carry: f32, pitch: f32) -> Drag {
        let carry = carry + (y - last);
        let rows = (carry / pitch).trunc();
        if rows != 0.0 {
            self.scroll_rows(rows as isize);
        }
        Drag::History { last: y, carry: carry - rows * pitch, pitch }
    }

    /// Scroll as a finger going down `rows` rows' heights does, or up for
    /// fewer than none: the terminal's screen back to what the shell wrote
    /// before, the editor's text to the lines before.
    pub(super) fn scroll_rows(&mut self, rows: isize) {
        if self.screen() == Screen::Editor {
            self.state.editor.as_mut().expect("the editor page shows a file").scroll(-rows);
            self.refresh_text();
        } else if let Some(term) = self.term.as_mut() {
            term.scroll(rows);
            self.refresh();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;

    #[test]
    fn the_terminal_sends_every_key_to_its_shell() {
        let (f, i) = (fonts(), info());
        // 终端默认搬到了首页「其他」（2026-11-27）。
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.home_sections.iter_mut().for_each(|section| section.collapsed = false);
        app.state.home_rest_collapsed = false;
        app.refresh();
        assert_eq!(tap(&mut app, &f, &i, Action::Open(Screen::Terminal)), vec![Effect::StartShell("/".into())], "the page asks for a shell the first time");
        let (rows, cols) = app.term_size(&f);
        assert!(rows >= 20 && cols >= 40, "a screen of {rows} rows by {cols}");
        app.term_started(rows, cols);
        assert_eq!(tap(&mut app, &f, &i, Action::Type(Cap::Char('l'))), vec![Effect::TermInput(b"l".to_vec())]);
        assert_eq!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)), vec![Effect::TermInput(b"\r".to_vec())]);
        assert!(tap(&mut app, &f, &i, Action::TermKey(TermKey::Ctrl)).is_empty());
        assert_eq!(tap(&mut app, &f, &i, Action::Type(Cap::Char('c'))), vec![Effect::TermInput(vec![0x03])]);
        assert_eq!(tap(&mut app, &f, &i, Action::TermKey(TermKey::Up)), vec![Effect::TermInput(b"\x1b[A".to_vec())]);
        app.term_output(b"ishtar:/ # ");
        tap_back(&mut app, &f, &i, 0);
        assert!(tap(&mut app, &f, &i, Action::Open(Screen::Terminal)).is_empty(), "the same shell goes on");
        app.term_ended(TermEnd::Exited(0));
        assert!(tap(&mut app, &f, &i, Action::Type(Cap::Char('l'))).is_empty(), "nothing runs to take the keys");
        assert_eq!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)), vec![Effect::StartShell("/".into())]);
    }

    #[test]
    fn the_file_manager_opens_the_terminal_in_the_folder_shown() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Files, W, H, PPD);
        app.set_files("/sdcard/Download".into(), Ok(vec![entry("a.txt", FileKind::File)]), 0);
        // 右上角那个圆钮换成了两枚胶囊（U 盘、搜索），终端的入口不再在页上。
        assert_eq!(app.frame(&f, &i).button, None, "the bar keeps no round button");
        assert_eq!(app.act(Action::TerminalHere, 0), vec![Effect::StartShell("/sdcard/Download".into())], "a shell starts there");
        assert_eq!(app.screen(), Screen::Terminal);
        let (rows, cols) = app.term_size(&f);
        app.term_started(rows, cols);
        app.term_ended(TermEnd::Exited(0));
        assert_eq!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)), vec![Effect::StartShell("/sdcard/Download".into())], "Enter starts it again there");
        app.term_started(rows, cols);
        tap_back(&mut app, &f, &i, 0);
        app.set_files("/data/adb".into(), Ok(Vec::new()), 0);
        assert_eq!(app.act(Action::TerminalHere, 0), vec![Effect::ShellTo("/data/adb".into())], "the shell running goes there");
        assert_eq!(app.screen(), Screen::Terminal);
    }

    #[test]
    fn a_drag_on_the_terminal_scrolls_back_through_what_the_shell_wrote() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Terminal, W, H, PPD);
        let (rows, cols) = app.term_size(&f);
        app.term_started(rows, cols);
        let many: String = (0..100).map(|n| format!("line {n}\r\n")).collect();
        app.term_output(many.as_bytes());
        let (card, pitch) = app.frame(&f, &i).scrollback.expect("the terminal's screen scrolls back");
        let (x, y) = (card.x + card.w / 2.0, card.y + card.h / 3.0);
        app.touch(Touch::Down(x, y), &f, 0);
        app.touch(Touch::Move(x, y + pitch * 5.5), &f, 10);
        app.touch(Touch::Up, &f, 20);
        assert_eq!(app.term.as_ref().expect("the shell started").screen().scrollback(), 5, "down shows five older rows");
        assert_eq!(app.top().1, 0.0, "the page itself stays put");
    }
}
