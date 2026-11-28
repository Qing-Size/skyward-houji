//! 录屏页那几档：换档、容器切换、秒数。
use super::*;
use crate::app::testing::*;

#[test]
fn the_container_row_switches_between_avi_and_mp4() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Record, W, H, PPD);
    assert_eq!(app.state.record.format, 0, "AVI to begin with");
    // 点「容器」那一行。
    let frame = app.frame(&f, &i);
    let row = frame
        .hits
        .iter()
        .find(|h| h.action == Action::RecordSetting(RecordSetting::Format))
        .expect("the container row is on the page")
        .rect;
    app.touch(Touch::Down(row.x + row.w / 2.0, row.y + row.h / 2.0), &f, 0);
    app.touch(Touch::Up, &f, 0);
    assert_eq!(app.state.record.format, 1, "MP4 after one tap");
    app.act(Action::RecordSetting(RecordSetting::Format), 0);
    assert_eq!(app.state.record.format, 0, "and back to AVI");
}

#[test]
fn the_elapsed_clock_counts_and_holds_while_paused() {
    let mut app = App::new(Screen::Record, W, H, PPD);
    app.state.record.on = true;
    app.state.record.since = 1_000;
    app.tick(1_000);
    assert_eq!(app.state.record.elapsed, "0:00");
    app.tick(4_500);
    assert_eq!(app.state.record.elapsed, "0:03", "3.5 s reads as 0:03");
    // 暂停在第 3 秒，过了 10 秒还是 0:03。
    app.state.record.paused = true;
    app.state.record.paused_at = 4_500;
    app.tick(14_500);
    assert_eq!(app.state.record.elapsed, "0:03", "the pause is not counted");
    // 继续以后从 3 秒接着走。
    app.state.record.held = 10_000;
    app.state.record.paused = false;
    app.tick(16_500);
    assert_eq!(app.state.record.elapsed, "0:05", "carries on from where it stopped");
}
