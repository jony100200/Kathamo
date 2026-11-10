//! Native file dialogs that never block the UI thread.
//!
//! Each call opens the OS dialog on its own thread and returns a [`DialogHandle`].
//! Keep the handle and call [`DialogHandle::poll`] once per frame; it returns
//! `None` while the dialog is open and the [`DialogOutcome`] once it closes.
//!
//! ```no_run
//! use kathamo::dialogs::{self, DialogOutcome, FileFilter};
//! let mut pending = Some(dialogs::pick_file(&[FileFilter::new("Images", &["png", "jpg"])]));
//! // each frame:
//! if let Some(outcome) = pending.as_ref().and_then(|h| h.poll()) {
//!     pending = None;
//!     if let DialogOutcome::Picked(path) = outcome {
//!         println!("{}", path.display());
//!     }
//! }
//! ```
//!
//! Windows and macOS use the system dialog. Other platforms report
//! [`DialogOutcome::Unavailable`] at once, so apps keep a typed path field as fallback.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Mutex;

/// A named group of file extensions, e.g. `FileFilter::new("Images", &["png", "jpg"])`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFilter {
    pub name: String,
    /// Extensions without the dot.
    pub extensions: Vec<String>,
}

impl FileFilter {
    pub fn new(name: &str, extensions: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            extensions: extensions
                .iter()
                .map(|e| e.trim_start_matches('.').to_string())
                .collect(),
        }
    }
}

/// How a dialog closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialogOutcome {
    /// The user chose this path.
    Picked(PathBuf),
    /// The user closed the dialog without choosing.
    Cancelled,
    /// No native dialog on this platform; use a typed path instead.
    Unavailable,
}

/// A dialog running on its own thread. Poll it each frame. `Send + Sync`, so it
/// can live inside an editor or panel state.
#[derive(Debug)]
pub struct DialogHandle {
    rx: Mutex<Receiver<DialogOutcome>>,
}

impl DialogHandle {
    /// Run `pick` on a new thread; its result arrives through [`DialogHandle::poll`].
    pub fn spawn<F>(pick: F) -> Self
    where
        F: FnOnce() -> DialogOutcome + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("kathamo-file-dialog".into())
            .spawn({
                let tx = tx.clone();
                move || {
                    let _ = tx.send(pick());
                }
            });
        if spawned.is_err() {
            let _ = tx.send(DialogOutcome::Unavailable);
        }
        Self { rx: Mutex::new(rx) }
    }

    /// A handle that is already finished with `outcome`.
    pub fn ready(outcome: DialogOutcome) -> Self {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(outcome);
        Self { rx: Mutex::new(rx) }
    }

    /// `None` while the dialog is open; the outcome once it has closed.
    /// The outcome is delivered once; drop the handle after receiving it.
    pub fn poll(&self) -> Option<DialogOutcome> {
        let Ok(rx) = self.rx.lock() else {
            return Some(DialogOutcome::Cancelled);
        };
        match rx.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(TryRecvError::Empty) => None,
            // The dialog thread ended without a result (it panicked).
            Err(TryRecvError::Disconnected) => Some(DialogOutcome::Cancelled),
        }
    }
}

/// Ask the user for an existing file. An empty `filters` slice allows any file.
pub fn pick_file(filters: &[FileFilter]) -> DialogHandle {
    native::open(native::Kind::File, filters.to_vec(), None)
}

/// Ask the user for a folder.
pub fn pick_folder() -> DialogHandle {
    native::open(native::Kind::Folder, Vec::new(), None)
}

/// Ask the user where to save a file, suggesting `file_name`.
pub fn save_file(filters: &[FileFilter], file_name: &str) -> DialogHandle {
    native::open(
        native::Kind::Save,
        filters.to_vec(),
        Some(file_name.to_string()),
    )
}

#[cfg(any(windows, target_os = "macos"))]
mod native {
    use super::{DialogHandle, DialogOutcome, FileFilter};

    pub enum Kind {
        File,
        Folder,
        Save,
    }

    pub fn open(kind: Kind, filters: Vec<FileFilter>, file_name: Option<String>) -> DialogHandle {
        DialogHandle::spawn(move || {
            let mut dialog = rfd::FileDialog::new();
            for f in &filters {
                dialog = dialog.add_filter(&f.name, &f.extensions);
            }
            if let Some(name) = &file_name {
                dialog = dialog.set_file_name(name);
            }
            let picked = match kind {
                Kind::File => dialog.pick_file(),
                Kind::Folder => dialog.pick_folder(),
                Kind::Save => dialog.save_file(),
            };
            picked.map_or(DialogOutcome::Cancelled, DialogOutcome::Picked)
        })
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod native {
    use super::{DialogHandle, DialogOutcome, FileFilter};

    pub enum Kind {
        File,
        Folder,
        Save,
    }

    pub fn open(_kind: Kind, _filters: Vec<FileFilter>, _name: Option<String>) -> DialogHandle {
        DialogHandle::ready(DialogOutcome::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn handle_is_pending_until_the_dialog_thread_answers() {
        let (go_tx, go_rx) = mpsc::channel::<()>();
        let handle = DialogHandle::spawn(move || {
            go_rx.recv().unwrap();
            DialogOutcome::Picked(PathBuf::from("photo.png"))
        });
        assert_eq!(
            handle.poll(),
            None,
            "poll never blocks while the dialog is open"
        );
        go_tx.send(()).unwrap();
        let mut outcome = None;
        for _ in 0..200 {
            outcome = handle.poll();
            if outcome.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            outcome,
            Some(DialogOutcome::Picked(PathBuf::from("photo.png")))
        );
    }

    #[test]
    fn panicking_dialog_reads_as_cancelled_and_ready_is_immediate() {
        let handle = DialogHandle::spawn(|| panic!("dialog failed"));
        let mut outcome = None;
        for _ in 0..200 {
            outcome = handle.poll();
            if outcome.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(outcome, Some(DialogOutcome::Cancelled));
        let ready = DialogHandle::ready(DialogOutcome::Unavailable);
        assert_eq!(ready.poll(), Some(DialogOutcome::Unavailable));
    }

    #[test]
    fn handles_can_live_in_shared_app_state() {
        fn send_sync<T: Send + Sync>() {}
        send_sync::<DialogHandle>();
    }

    #[test]
    fn filters_drop_leading_dots() {
        let f = FileFilter::new("Images", &[".png", "jpg"]);
        assert_eq!(f.extensions, vec!["png".to_string(), "jpg".to_string()]);
    }
}
