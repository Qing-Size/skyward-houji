//! AVB 页的另一半：把当前槽位的 vbmeta 留一份下来，再按份写回。
//!
//! 从「青菜豆腐」分支的 `skyward/src/vbmeta.rs` 搬来：那边是「留一份、按份
//! 写回、回滚索引低了就拒绝」这一套，写回用的是它自己的 `avbctl::write_back`。
//! 我们这边的 `avbctl` 已经有 `dump`／`restore`，`restore` 自己就按回滚索引
//! 拒绝，所以这一层只做三件事：把 `dump` 下来的那份写到存储上、把存着的那几
//! 份读出来、把选中的那份交给 `restore`。

use crate::tr;
use super::*;

/// 副本放的地方的名字，从那个地方的顶层算起：跟 `avbctl::KEPT` 是同一处。
pub const VBmeta_FOLDER: &str = "Skyward/vbmeta";

impl App {
    /// 留着的副本读回来了：列在页上。
    pub fn set_vbmeta_copies(&mut self, found: Result<Vec<VbmetaCopy>, String>) {
        self.state.vbmeta_copies = Some(found);
        if self.state.vbmeta_pick.is_some_and(|k| self.vbmeta_chosen(k).is_none()) {
            self.state.vbmeta_pick = None;
            self.state.vbmeta_asking = false;
        }
        self.refresh();
    }

    /// 留好了，说一声，并把清单重读一遍。
    pub fn vbmeta_kept(&mut self, done: Result<String, String>, now: u64) {
        match done {
            Ok(name) => {
                let l = self.state.lang;
                let said = tr!(l, "已备份当前 vbmeta：{name}", "已備份目前 vbmeta：{name}", "This slot's vbmeta is kept: {name}");
                self.say(&said, now);
            }
            Err(why) => self.say(&why, now),
        }
    }

    /// 写回完了（或没有），说一声。
    pub fn vbmeta_restored(&mut self, done: Result<u64, String>, now: u64) {
        self.state.confirm = 0.0;
        self.state.vbmeta_asking = false;
        match done {
            Ok(index) => {
                let l = self.state.lang;
                let said = tr!(
                    l,
                    "已写回并校验（回滚索引 {index}），重启后生效。",
                    "已寫回並驗證（回滾索引 {index}），重新啟動後生效。",
                    "Written back and verified (rollback index {index}); effective after the next start."
                );
                self.say(&said, now);
            }
            Err(why) => self.say(&why, now),
        }
    }

    /// 页上选中的那份副本，按它的位置。
    pub(super) fn vbmeta_chosen(&self, k: usize) -> Option<&VbmetaCopy> {
        match &self.state.vbmeta_copies {
            Some(Ok(list)) => list.get(k),
            _ => None,
        }
    }

    /// 现在选中的那份。
    pub(super) fn vbmeta_picked(&self) -> Option<&VbmetaCopy> {
        self.state.vbmeta_pick.and_then(|k| self.vbmeta_chosen(k))
    }

    /// 副本存在哪几处：手上读得到的那几处。
    pub fn vbmeta_places(&self) -> (bool, bool) {
        (self.state.data.storage_open(), self.state.usb_drive == Some(UsbDrive::Shown))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;

    fn copy(name: &str, suffix: &str, rollback: Result<u64, String>, made: i64) -> VbmetaCopy {
        VbmetaCopy { place: Place::Storage, name: name.into(), suffix: suffix.into(), size: 8192, rollback, made }
    }

    #[test]
    fn the_copies_kept_are_read_and_one_that_goes_missing_leaves_nothing_chosen() {
        // 读出副本这一步在运行时那一边（页面一显示就读），这里管的是读回来
        // 之后页面上怎么站：清单在、选中的那份没了就不再选中。
        let mut app = App::new(Screen::Avb, W, H, PPD);
        assert!(app.state.vbmeta_copies.is_none(), "nothing read yet");
        app.set_vbmeta_copies(Ok(vec![
            copy("vbmeta_a-100.img", "_a", Ok(4), 100),
            copy("vbmeta_a-200.img", "_a", Ok(5), 200),
        ]));
        app.act(Action::VbmetaPick(1), 0);
        assert_eq!(app.state.vbmeta_pick, Some(1));
        app.set_vbmeta_copies(Ok(vec![copy("vbmeta_a-100.img", "_a", Ok(4), 100)]));
        assert_eq!(app.state.vbmeta_pick, None, "the copy chosen is gone, so nothing is chosen");
        assert!(!app.state.vbmeta_asking, "and the page's swipe goes back to writing the checks");
        assert_eq!(app.state.vbmeta_copies.as_ref().unwrap().as_ref().unwrap().len(), 1);
    }

    #[test]
    fn keeping_a_copy_goes_where_the_page_says_it_is_kept() {
        let mut app = App::new(Screen::Avb, W, H, PPD);
        app.state.data = Data::Open;
        assert_eq!(app.act(Action::VbmetaKeep, 0), vec![Effect::KeepVbmeta(Place::Storage)]);
        app.state.usb_drive = Some(UsbDrive::Shown);
        app.act(Action::BackupTo(Place::UsbDrive), 0);
        assert_eq!(app.act(Action::VbmetaKeep, 0), vec![Effect::KeepVbmeta(Place::UsbDrive)]);
        // 位置读不到就不留：说一句，别写到看不见的地方。
        app.state.data = Data::Closed;
        app.state.usb_drive = None;
        app.act(Action::BackupTo(Place::Storage), 0);
        assert!(app.act(Action::VbmetaKeep, 0).is_empty());
    }

    #[test]
    fn a_copy_chosen_turns_the_page_s_swipe_into_writing_it_back() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Avb, W, H, PPD);
        app.state.data = Data::Open;
        app.state.avb = Some(Ok((true, true)));
        app.set_vbmeta_copies(Ok(vec![
            copy("vbmeta_a-100.img", "_a", Ok(3), 100),
            copy("vbmeta_a-200.img", "_a", Err("does not start with AVB0".into()), 200),
        ]));
        app.act(Action::VbmetaPick(0), 0);
        app.act(Action::VbmetaAsk(true), 0);
        assert_eq!(
            swipe(&mut app, &f, &i, Confirm::VbmetaRestore),
            vec![Effect::RestoreVbmeta { place: Place::Storage, name: "vbmeta_a-100.img".into() }],
            "the copy chosen is the one written back"
        );
        assert_eq!(app.state.confirm, 0.0);
    }

    #[test]
    fn a_copy_that_does_not_read_is_still_listed() {
        let mut app = App::new(Screen::Avb, W, H, PPD);
        app.set_vbmeta_copies(Ok(vec![copy("vbmeta_a-200.img", "_a", Err("cut short".into()), 200)]));
        assert_eq!(app.state.vbmeta_copies.as_ref().unwrap().as_ref().unwrap().len(), 1);
        // 选它也行：写回会在核对那一步说出它读不出来，页面不拦着。
        app.act(Action::VbmetaPick(0), 0);
        assert_eq!(app.state.vbmeta_pick, Some(0));
    }
}
