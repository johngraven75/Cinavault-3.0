use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, OnceLock,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataTaskProgress {
    pub active: bool,
    pub task: String,
    pub label: String,
    pub current: usize,
    pub total: usize,
    pub percent: u8,
    pub message: String,
}

pub struct MetadataTaskGuard {
    task: String,
    finished: bool,
}

impl MetadataTaskGuard {
    pub fn start(
        task: impl Into<String>,
        label: impl Into<String>,
        total: usize,
        message: impl Into<String>,
    ) -> Self {
        let task = task.into();
        let progress = MetadataTaskProgress {
            active: true,
            task: task.clone(),
            label: label.into(),
            current: 0,
            total,
            percent: active_percent(0, total),
            message: message.into(),
        };
        cancellation_flag().store(false, Ordering::SeqCst);
        replace_progress(progress);
        Self {
            task,
            finished: false,
        }
    }

    pub fn update(&mut self, current: usize, message: impl Into<String>) {
        let mut progress = progress_state()
            .lock()
            .expect("metadata task progress lock");
        if progress.task != self.task || !progress.active {
            return;
        }
        progress.current = current.min(progress.total);
        progress.percent = active_percent(progress.current, progress.total);
        progress.message = message.into();
    }

    pub fn finish(mut self, message: impl Into<String>) {
        let mut progress = progress_state()
            .lock()
            .expect("metadata task progress lock");
        if progress.task == self.task {
            progress.active = false;
            progress.current = progress.total;
            progress.percent = 100;
            progress.message = message.into();
        }
        self.finished = true;
    }
}

impl Drop for MetadataTaskGuard {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let mut progress = progress_state()
            .lock()
            .expect("metadata task progress lock");
        if progress.task == self.task && progress.active {
            progress.active = false;
            progress.percent = 100;
            progress.message = "Task stopped before completion".to_string();
        }
    }
}

#[tauri::command]
pub fn get_metadata_task_progress() -> MetadataTaskProgress {
    progress_state()
        .lock()
        .expect("metadata task progress lock")
        .clone()
}

#[tauri::command]
pub fn stop_ai_agent() -> MetadataTaskProgress {
    cancellation_flag().store(true, Ordering::SeqCst);
    let mut progress = progress_state()
        .lock()
        .expect("metadata task progress lock");
    if progress.active {
        progress.message = "Stop requested; finishing the current item...".to_string();
    }
    progress.clone()
}

pub fn stop_requested() -> bool {
    cancellation_flag().load(Ordering::SeqCst)
}

fn replace_progress(progress: MetadataTaskProgress) {
    *progress_state()
        .lock()
        .expect("metadata task progress lock") = progress;
}

fn progress_state() -> &'static Mutex<MetadataTaskProgress> {
    static PROGRESS: OnceLock<Mutex<MetadataTaskProgress>> = OnceLock::new();
    PROGRESS.get_or_init(|| Mutex::new(inactive_progress()))
}

fn cancellation_flag() -> &'static AtomicBool {
    static CANCELLED: AtomicBool = AtomicBool::new(false);
    &CANCELLED
}

fn inactive_progress() -> MetadataTaskProgress {
    MetadataTaskProgress {
        active: false,
        task: String::new(),
        label: String::new(),
        current: 0,
        total: 0,
        percent: 0,
        message: String::new(),
    }
}

fn active_percent(current: usize, total: usize) -> u8 {
    if total == 0 {
        return 0;
    }
    let percent = current.saturating_mul(100) / total;
    percent.min(99) as u8
}

#[cfg(test)]
fn reset_metadata_task_progress_for_test() {
    replace_progress(inactive_progress());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_percent_is_capped_while_active_and_reaches_100_when_finished() {
        reset_metadata_task_progress_for_test();
        let mut guard =
            MetadataTaskGuard::start("adult_metadata", "Adult Metadata Gather", 4, "Starting");

        guard.update(2, "Scanning media");
        let snapshot = get_metadata_task_progress();
        assert_eq!(snapshot.percent, 50);
        assert!(snapshot.active);

        guard.update(4, "Writing sidecars");
        let snapshot = get_metadata_task_progress();
        assert_eq!(snapshot.percent, 99);
        assert!(snapshot.active);

        guard.finish("Complete");
        let snapshot = get_metadata_task_progress();
        assert_eq!(snapshot.percent, 100);
        assert!(!snapshot.active);
    }
}
