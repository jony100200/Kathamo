//! Background task lifecycle and progress receipts.
//! Enforces the rule: No model initialization, filesystem scanning, or heavy work blocks the UI thread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Queued,
    Running { progress: f32, message: String },
    Completed { summary: String },
    Failed { error: String },
    Canceled,
}

#[derive(Debug, Clone)]
pub struct TaskReceipt {
    pub id: TaskId,
    pub name: String,
    pub status: TaskStatus,
}

/// A handle passed to worker threads to report progress and check for cancellation.
#[derive(Clone)]
pub struct TaskReporter {
    id: TaskId,
    name: String,
    state: Arc<Mutex<TaskStatus>>,
    cancel_flag: Arc<AtomicBool>,
}

impl TaskReporter {
    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_canceled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    pub fn report_progress(&self, progress: f32, message: impl Into<String>) {
        if !self.is_canceled() {
            let mut guard = self.state.lock().unwrap();
            *guard = TaskStatus::Running {
                progress: progress.clamp(0.0, 1.0),
                message: message.into(),
            };
        }
    }

    pub fn complete(&self, summary: impl Into<String>) {
        let mut guard = self.state.lock().unwrap();
        *guard = TaskStatus::Completed {
            summary: summary.into(),
        };
    }

    pub fn fail(&self, error: impl Into<String>) {
        let mut guard = self.state.lock().unwrap();
        *guard = TaskStatus::Failed {
            error: error.into(),
        };
    }
}

/// A tracker held by the UI to observe and control background tasks.
pub struct TaskController {
    id: TaskId,
    name: String,
    state: Arc<Mutex<TaskStatus>>,
    cancel_flag: Arc<AtomicBool>,
}

impl TaskController {
    pub fn new(id: TaskId, name: impl Into<String>) -> (Self, TaskReporter) {
        let name_str = name.into();
        let state = Arc::new(Mutex::new(TaskStatus::Queued));
        let cancel_flag = Arc::new(AtomicBool::new(false));

        let reporter = TaskReporter {
            id,
            name: name_str.clone(),
            state: Arc::clone(&state),
            cancel_flag: Arc::clone(&cancel_flag),
        };

        let controller = Self {
            id,
            name: name_str,
            state,
            cancel_flag,
        };

        (controller, reporter)
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
        let mut guard = self.state.lock().unwrap();
        *guard = TaskStatus::Canceled;
    }

    pub fn receipt(&self) -> TaskReceipt {
        let guard = self.state.lock().unwrap();
        TaskReceipt {
            id: self.id,
            name: self.name.clone(),
            status: guard.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_lifecycle_reports_progress_and_completion() {
        let (ctrl, reporter) = TaskController::new(TaskId(1), "Scan Workspace");
        assert_eq!(ctrl.receipt().status, TaskStatus::Queued);

        reporter.report_progress(0.5, "Indexed 500 files");
        assert_eq!(
            ctrl.receipt().status,
            TaskStatus::Running {
                progress: 0.5,
                message: "Indexed 500 files".into()
            }
        );

        reporter.complete("Finished indexing");
        assert_eq!(
            ctrl.receipt().status,
            TaskStatus::Completed {
                summary: "Finished indexing".into()
            }
        );
    }

    #[test]
    fn task_cancellation_flags_worker() {
        let (ctrl, reporter) = TaskController::new(TaskId(2), "Export Video");
        assert!(!reporter.is_canceled());

        ctrl.cancel();
        assert!(reporter.is_canceled());
        assert_eq!(ctrl.receipt().status, TaskStatus::Canceled);
    }
}
