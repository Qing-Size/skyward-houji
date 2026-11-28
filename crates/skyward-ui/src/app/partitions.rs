//! 分区管理：把 `by-name` 里的分区列出来（名字与大小），并让其中几个能被
//! 原样导出成镜像文件。
//!
//! 分区镜像的备份（导出、写回、删除）也归这一页的一组方法管：两边说的是同
//! 一批文件——导出写进 `Skyward/backup`，镜像页从那儿把它们列出来。

use crate::tr;
use super::*;

impl App {
    /// 分区表读回来了。
    pub fn set_partitions(&mut self, rows: Result<Vec<(String, u64)>, String>) {
        self.state.partitions = Some(rows);
        self.state.dump_pick = None;
        self.refresh();
    }

    /// 导出写完了，说一声。
    pub fn dump_done(&mut self, done: Result<String, String>, now: u64) {
        match done {
            Ok(path) => {
                let l = self.state.lang;
                let said = tr!(l, "已导出到 {path}", "已匯出到 {path}", "Exported to {path}");
                self.say(&said, now);
            }
            Err(why) => self.say(&why, now),
        }
    }

    /// 镜像找回来了：列在页上，选中的那份如果还在就留着。
    pub fn set_images(&mut self, found: Result<Vec<ImageCopy>, String>) {
        self.state.image_copies = Some(found);
        if self.state.image_pick.is_some_and(|k| self.picked_image(k).is_none()) {
            self.state.image_pick = None;
        }
        self.refresh();
    }

    /// 写回完了，说一声，并把清单重新读一遍。
    pub fn image_restored(&mut self, done: Result<String, String>, now: u64) {
        self.state.confirm = 0.0;
        match done {
            Ok(partition) => {
                let l = self.state.lang;
                let said = tr!(l, "已写回 {partition}", "已寫回 {partition}", "Written back to {partition}");
                self.say(&said, now);
            }
            Err(why) => self.say(&why, now),
        }
    }

    /// 删掉了，说一声，选中的那份就不留了。
    pub fn image_deleted(&mut self, done: Result<(), String>, now: u64) {
        self.state.confirm = 0.0;
        self.state.image_pick = None;
        match done {
            Ok(()) => {
                let l = self.state.lang;
                self.say(l.pick(["镜像已删除", "映像檔已刪除", "The image is deleted"]), now);
            }
            Err(why) => self.say(&why, now),
        }
    }

    /// 页上选中的那份镜像，按它的位置。
    pub(super) fn picked_image(&self, k: usize) -> Option<&ImageCopy> {
        match &self.state.image_copies {
            Some(Ok(list)) => list.get(k),
            _ => None,
        }
    }

    /// 现在选中的那份。
    pub(super) fn image_chosen(&self) -> Option<&ImageCopy> {
        self.state.image_pick.and_then(|k| self.picked_image(k))
    }

    /// 放镜像的那几处：读者手上开着的（内部存储、U 盘）。
    pub fn image_places(&self) -> (bool, bool) {
        (self.state.data.storage_open(), self.state.usb_drive == Some(UsbDrive::Shown))
    }

    /// 选中的那个分区叫什么。
    pub(super) fn dumped_name(&self) -> Option<String> {
        let k = self.state.dump_pick?;
        match &self.state.partitions {
            Some(Ok(list)) => list.get(k).map(|(name, _)| name.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;

    fn copy(file: &str, partition: &str, sha256: Option<&str>) -> ImageCopy {
        ImageCopy { partition: partition.into(), file: file.into(), size: 4096, sha256: sha256.map(Into::into), made: 1_762_486_400 }
    }

    #[test]
    fn opening_the_image_page_reads_the_partitions_and_the_images_kept() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        let fx = app.act(Action::Open(Screen::ImageBackup), 0);
        assert_eq!(app.screen(), Screen::ImageBackup);
        assert!(fx.contains(&Effect::ReadPartitions), "{fx:?}");
        assert!(
            fx.iter().any(|f| matches!(f, Effect::FindImages { .. })),
            "the images kept are read as the page opens: {fx:?}"
        );
        assert_eq!((app.state.image_pick, app.state.image_action), (None, ImageAction::Restore));
    }

    #[test]
    fn the_swipe_writes_back_the_image_chosen_and_checks_it_first() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::ImageBackup, W, H, PPD);
        app.state.data = Data::Open;
        let sum = "1".repeat(64);
        app.set_images(Ok(vec![copy("boot-7.img", "boot", Some(&sum)), copy("system_a-9.img", "system_a", None)]));
        app.act(Action::ImagePick(1), 0);
        assert_eq!(
            swipe(&mut app, &f, &i, Confirm::ImageRestore),
            vec![Effect::RestoreImage { place: Place::Storage, file: "system_a-9.img".into(), partition: "system_a".into(), sha256: None }],
            "the partition and the checksum go with it: the checksum where there is one"
        );
        app.state.confirm = 0.0;
        app.act(Action::ImagePick(0), 0);
        assert_eq!(
            swipe(&mut app, &f, &i, Confirm::ImageRestore),
            vec![Effect::RestoreImage { place: Place::Storage, file: "boot-7.img".into(), partition: "boot".into(), sha256: Some(sum) }]
        );
    }

    #[test]
    fn the_same_swipe_deletes_when_deleting_is_what_the_page_asks_for() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::ImageBackup, W, H, PPD);
        app.state.data = Data::Open;
        app.set_images(Ok(vec![copy("boot-7.img", "boot", None)]));
        app.act(Action::ImagePick(0), 0);
        app.act(Action::ImageAction(ImageAction::Delete), 0);
        assert_eq!(
            swipe(&mut app, &f, &i, Confirm::ImageDelete),
            vec![Effect::DeleteImageBackup { place: Place::Storage, file: "boot-7.img".into() }],
            "the page's one slider now deletes, and only the image chosen"
        );
    }

    #[test]
    fn a_swipe_with_no_image_chosen_asks_for_one_instead_of_writing_anything() {
        let mut app = App::new(Screen::ImageBackup, W, H, PPD);
        app.state.data = Data::Open;
        app.set_images(Ok(vec![copy("boot-7.img", "boot", None)]));
        assert!(app.act(Action::Confirm(Confirm::ImageRestore), 0).is_empty(), "nothing is written");
        assert!(app.act(Action::Confirm(Confirm::ImageDelete), 0).is_empty(), "nothing is deleted");
    }

    #[test]
    fn exporting_writes_into_the_backup_folder_of_the_place_chosen() {
        let mut app = App::new(Screen::ImageBackup, W, H, PPD);
        app.state.data = Data::Open;
        app.set_partitions(Ok(vec![("boot".into(), 100), ("super".into(), 200)]));
        app.act(Action::PickDump(0), 0);
        match app.act(Action::DumpNow, 7_000).as_slice() {
            [Effect::DumpImage { name, path }] => {
                assert_eq!(name, "boot");
                assert_eq!(path.to_string_lossy(), "/storage/emulated/0/Skyward/backup/boot-7.img");
            }
            other => panic!("one export, into the folder the images live in: {other:?}"),
        }
        // 换成 U 盘：同一份分区，写到 U 盘的那个文件夹里。
        app.act(Action::BackupTo(Place::UsbDrive), 0);
        app.state.usb_drive = Some(UsbDrive::Shown);
        match app.act(Action::DumpNow, 7_000).as_slice() {
            [Effect::DumpImage { path, .. }] => assert_eq!(path.to_string_lossy(), "/mnt/usb/Skyward/backup/boot-7.img"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_place_that_cannot_be_read_exports_nothing() {
        let mut app = App::new(Screen::ImageBackup, W, H, PPD);
        app.set_partitions(Ok(vec![("boot".into(), 100)]));
        app.act(Action::PickDump(0), 0);
        // 内部存储没开：说一句，而不是写到一个读不到的地方。
        assert!(app.act(Action::DumpNow, 7_000).is_empty());
    }
}
