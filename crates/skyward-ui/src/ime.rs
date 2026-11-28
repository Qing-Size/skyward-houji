//! 拼写条：键盘上方那一条。左边是中/EN 开关，中间是候选字，右边翻页。
//!
//! 只在打开了输入框的页面上出现（见 `app::ime`）；页面自己把键盘的顶边
//! 让出 `height()` 这么高，再在这里画这一条。

use crate::app::{Action, Hit};
use crate::geom::Rect;
use crate::pinyin::Composition;
use crate::{Canvas, Fonts, Theme};

/// 一条上摆几个候选字。
pub const PER_PAGE: usize = 3;

pub fn height(t: &Theme) -> f32 {
    if cfg!(feature = "pinyin-input") { t.m.dp(44.0) } else { 0.0 }
}

pub fn bar(canvas: &mut Canvas, t: &Theme, f: &Fonts, ime: &Composition, top: f32) -> Vec<Hit> {
    let m = &t.m;
    let mut out = Vec::new();
    if !cfg!(feature = "pinyin-input") {
        return out;
    }
    canvas.fill_rect(0.0, top, m.w as f32, m.dp(44.0), t.pal.bg);
    let toggle = Rect::new(m.dp(8.0), top, m.dp(52.0), m.dp(44.0));
    canvas.text_center(
        &f.heading,
        if ime.enabled { "中" } else { "EN" },
        toggle.centre().0,
        toggle.centre().1,
        m.dp(16.0),
        if ime.enabled { t.pal.accent } else { t.pal.subtitle },
    );
    out.push(Hit { rect: toggle, action: Action::ImeToggle });
    let right = m.w as f32 - m.dp(44.0);
    let cell = (right - m.dp(64.0)) / PER_PAGE as f32;
    for (offset, candidate) in ime.candidates.iter().skip(ime.page * PER_PAGE).take(PER_PAGE).enumerate() {
        let rect = Rect::new(m.dp(64.0) + offset as f32 * cell, top, cell, m.dp(44.0));
        canvas.text_vc(
            &f.body,
            &candidate.text,
            rect.x + m.dp(6.0),
            rect.y + m.dp(30.0),
            m.dp(17.0),
            t.pal.text,
            Some(cell - m.dp(12.0)),
        );
        out.push(Hit { rect, action: Action::ImePick(ime.page * PER_PAGE + offset) });
    }
    if !ime.spelling.is_empty() && !ime.candidates.is_empty() {
        canvas.text_top(
            &f.value,
            &ime.spelling,
            m.dp(70.0),
            top + m.dp(2.0),
            m.dp(10.0),
            t.pal.subtitle,
            Some(right - m.dp(76.0)),
        );
    }
    if ime.candidates.is_empty() && !ime.spelling.is_empty() {
        canvas.text_vc(
            &f.body,
            &ime.spelling,
            m.dp(70.0),
            top + m.dp(22.0),
            m.dp(16.0),
            t.pal.text,
            Some(right - m.dp(76.0)),
        );
    }
    if ime.candidates.len() > PER_PAGE {
        let rect = Rect::new(right, top, m.dp(44.0), m.dp(44.0));
        let (x, y) = rect.centre();
        canvas.line(x - m.dp(3.0), y - m.dp(6.0), x + m.dp(3.0), y, m.dp(2.0), t.pal.text);
        canvas.line(x + m.dp(3.0), y, x - m.dp(3.0), y + m.dp(6.0), m.dp(2.0), t.pal.text);
        out.push(Hit { rect, action: Action::ImePage(true) });
    }
    out
}
