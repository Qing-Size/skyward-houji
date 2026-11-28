//! 任务页：这台机器手上在做的事，每样一处，点一下就到那件事的页上。
//!
//! 从「青菜豆腐」分支搬来（`crates/skyward-ui/src/tools.rs` 的 `tasks`），
//! 行是照着我们的状态重排的：那边只有安装、系统下载和浏览器下载，这个构建
//! 里没有浏览器那一块，所以只剩安装、安装脚本的尾巴
//! （`State::script_output`）和系统下载三样。

use crate::app::{Action, Frame, Install, Screen, State};
use crate::color::Rgba;
use crate::icons::Icon;
use crate::pages::{self, Ctx};
use crate::widgets::{self, Row};

const BLUE: Rgba = Rgba::rgb(0x34, 0x82, 0xff);
const GREEN: Rgba = Rgba::rgb(0x1d, 0xcd, 0x3a);
const SLATE: Rgba = Rgba::rgb(0x8c, 0x9d, 0xb0);

/// 任务页上的一行：叫什么、正走到哪儿、点了去哪儿。
struct Task {
    label: &'static str,
    /// 这一行右边的一句状态；没有就不写。
    sub: Option<String>,
    action: Action,
    color: Rgba,
    icon: Icon,
}

/// 安装走到哪儿了，一句话。没有安装时不给行，所以这里拿不到 `Idle`。
fn installing(l: crate::lang::Lang, s: &State) -> &'static str {
    match &s.install {
        Install::Idle => "",
        Install::Finished { outcome, .. } => match outcome {
            skyward_install::Outcome::Failed(_) => l.pick(["上一次没有装完", "上一次沒有裝完", "The last install did not finish"]),
            _ => l.pick(["上一次已经装完", "上一次已經裝完", "The last install finished"]),
        },
        Install::Wiped(Err(_)) => l.pick(["擦除没有做完", "抹除沒有做完", "The erase did not finish"]),
        Install::Wiped(Ok(())) => l.pick(["擦除已结束", "抹除已結束", "The erase finished"]),
        _ => l.pick(["正在进行", "正在進行", "Under way"]),
    }
}

/// 现在手上有哪几件事，按页上从上到下的顺序。
fn tasks(s: &State, l: crate::lang::Lang) -> Vec<Task> {
    let mut out = Vec::new();
    if !matches!(s.install, Install::Idle) {
        out.push(Task {
            label: l.pick(["安装进度", "安裝進度", "Install progress"]),
            sub: Some(installing(l, s).to_string()),
            action: Action::Open(Screen::Installing),
            color: GREEN,
            icon: Icon::Install,
        });
    }
    // 装了 zip 里的安装脚本才有一行：脚本说的话就留在安装页上。
    if !s.script_output.is_empty() {
        out.push(Task {
            label: l.pick(["安装脚本", "安裝腳本", "Install script"]),
            sub: Some(s.script_output.last().cloned().unwrap_or_default()),
            action: Action::Open(Screen::Installing),
            color: SLATE,
            icon: Icon::Log,
        });
    }
    out.push(Task {
        label: l.pick(["系统下载", "系統下載", "System download"]),
        sub: None,
        action: Action::Open(Screen::Update),
        color: BLUE,
        icon: Icon::Phone,
    });
    out
}

pub fn tasks_page(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = pages::long_page(cx);
    let top = t.m.content_top;
    let items = tasks(s, l);
    let rows: Vec<Row> = items
        .iter()
        .map(|task| {
            let row = Row::new(task.label).icon(task.color, task.icon);
            match &task.sub {
                Some(sub) => row.subtitle(sub),
                None => row,
            }
        })
        .collect();
    let list = widgets::list(&mut c, t, f, top, &rows);
    let out = rows_hits(&list.rows, &items);
    let y = widgets::footnote(
        &mut c,
        t,
        f,
        list.bottom + t.m.card_gap,
        l.pick([
            "任务在各自的页上继续：这里只是给它们一个入口。",
            "工作會在各自的頁上繼續：這裡只是給它們一個入口。",
            "Each task carries on in its own page; this page only leads to them.",
        ]),
    );
    Frame::new(t, pages::fit(c, t, y), out, l.pick(["任务", "工作", "Tasks"]), top)
}

fn rows_hits(rects: &[crate::geom::Rect], items: &[Task]) -> Vec<crate::app::Hit> {
    rects
        .iter()
        .zip(items)
        .map(|(&rect, task)| crate::app::Hit { rect, action: task.action })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::Lang;

    fn labels(s: &State, l: Lang) -> Vec<(&'static str, Action)> {
        tasks(s, l).into_iter().map(|t| (t.label, t.action)).collect()
    }

    #[test]
    fn a_page_with_nothing_under_way_offers_the_row_it_always_has() {
        let s = State::default();
        assert_eq!(
            labels(&s, Lang::En),
            [("System download", Action::Open(Screen::Update))],
            "no install and no script: only the download stands"
        );
        assert_eq!(labels(&s, Lang::Hans)[0].0, "系统下载");
        assert_eq!(labels(&s, Lang::Hant)[0].0, "系統下載");
    }

    #[test]
    fn an_install_adds_its_row_where_the_page_puts_it() {
        let mut s = State::default();
        s.install = Install::Waiting;
        assert_eq!(
            labels(&s, Lang::Hans),
            [("安装进度", Action::Open(Screen::Installing)), ("系统下载", Action::Open(Screen::Update))]
        );
        assert_eq!(tasks(&s, Lang::Hans)[0].sub.as_deref(), Some("正在进行"));
        // 装完的那次留着行，说的是它怎么了，而不是永远「正在进行」。
        s.install = Install::Finished { outcome: skyward_install::Outcome::Cancelled, damage: None };
        assert_eq!(tasks(&s, Lang::En)[0].sub.as_deref(), Some("The last install finished"));
    }

    #[test]
    fn a_scripts_last_line_rides_under_its_row_and_nothing_is_said_before_it_runs() {
        let mut s = State::default();
        assert!(tasks(&s, Lang::En).iter().all(|t| t.label != "Install script"), "no script, no row");
        s.script_output = vec!["- Flashing boot".into(), "  done".into()];
        let items = tasks(&s, Lang::En);
        assert_eq!(items[0].label, "Install script");
        assert_eq!(items[0].sub.as_deref(), Some("  done"), "the latest line is the one shown");
        assert_eq!(items[0].action, Action::Open(Screen::Installing), "the page that keeps the script's words");
    }

    /// 高级页上「任务」这一行开的是任务页，页上每一行对应一件事。
    #[test]
    fn the_tasks_page_lists_one_row_for_each_task() {
        let s = State::default();
        let items = tasks(&s, Lang::En);
        assert_eq!(items.len(), 1, "nothing under way: the system download alone");
        assert_eq!(items[0].action, Action::Open(Screen::Update));
    }
}
