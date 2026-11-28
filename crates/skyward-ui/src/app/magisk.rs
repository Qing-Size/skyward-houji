//! Magisk's page: what the phone runs, and writing Magisk's boot patcher
//! into the current slot.

use super::*;
use crate::texts;

/// What the phone and Magisk's APK say before anything is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MagiskStatus {
    pub installed: bool,
    pub has_backup: bool,
    /// The partition the record's backup came from, with its slot.
    pub backup_target: Option<String>,
    pub version: Option<String>,
    pub unlocked: bool,
    pub slot: Option<String>,
    /// The partition Magisk would patch on this phone.
    pub target: String,
}

impl App {
    /// What the phone and Magisk's APK say, read as the page opens.
    pub fn set_magisk(&mut self, status: Result<MagiskStatus, String>) {
        self.state.magisk = Some(status);
        self.refresh();
    }

    /// Writing Magisk has begun.
    pub fn magisk_started(&mut self) {
        self.state.magisk_job = Some(KernelSuJob { stage: KernelSuStage::Preparing, outcome: None, fetched: None, fallback: None });
        self.refresh();
    }

    /// How far writing Magisk has come.
    pub fn magisk_progress(&mut self, stage: KernelSuStage) {
        let job = self.state.magisk_job.get_or_insert(KernelSuJob { stage, outcome: None, fetched: None, fallback: None });
        if job.outcome.is_none() {
            job.stage = stage;
            self.refresh();
        }
    }

    /// Writing Magisk ended, and the page says how.
    pub fn magisk_ended(&mut self, end: KernelSuEnd, now: u64) {
        let job = self.state.magisk_job.get_or_insert(KernelSuJob { stage: KernelSuStage::Checking, outcome: None, fetched: None, fallback: None });
        job.outcome = Some(end);
        self.wake(now);
        self.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use crate::app::{Confirm, Data, Effect};

    fn status() -> MagiskStatus {
        MagiskStatus {
            installed: false,
            has_backup: false,
            backup_target: None,
            version: None,
            unlocked: true,
            slot: Some("A".into()),
            target: "init_boot_a".into(),
        }
    }

    #[test]
    fn the_page_writes_after_a_slide_and_locks_the_control_while_it_runs() {
        let mut app = App::new(Screen::Magisk, W, H, PPD);
        app.set_magisk(Ok(status()));
        app.state.data = Data::Open;
        assert_eq!(app.confirmed(Confirm::Magisk, 0), vec![Effect::Haptic, Effect::InstallMagisk]);
        assert!(app.state.magisk_job.is_some());
        assert_eq!(app.confirmed(Confirm::Magisk, 0), vec![Effect::Haptic]);
    }

    #[test]
    fn a_locked_bootloader_is_refused_before_anything_is_written() {
        let mut app = App::new(Screen::Magisk, W, H, PPD);
        let mut status = status();
        status.unlocked = false;
        app.set_magisk(Ok(status));
        app.state.data = Data::Open;
        assert_eq!(app.confirmed(Confirm::Magisk, 0), vec![Effect::Haptic]);
        assert!(app.state.magisk_job.is_none());
    }
}
