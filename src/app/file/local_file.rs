//! Saving to and loading from a local file

use super::FileData;
use std::sync::mpsc::{self, Receiver};

/// A local file which we are loading to/from
#[cfg(not(target_arch = "wasm32"))]
pub struct FileSource {
    path: std::path::PathBuf,
}

/// Stores the progress of loading a local file
pub struct LoadFileState {
    /// Receiver which will receive the file
    rx: Receiver<(FileData, FileSource)>,
}
impl LoadFileState {
    /// Poll whether we are done loading the file. If we are still waiting, `None`. If we are done,
    /// `Some` with either the file or an oops
    pub fn poll_loaded(&mut self) -> Option<Result<(FileData, FileSource), ()>> {
        match self.rx.try_recv() {
            Ok(file) => Some(Ok(file)),
            Err(err) => match err {
                // Nothing sent to us yet
                mpsc::TryRecvError::Empty => None,
                // Loader thread failed/died
                mpsc::TryRecvError::Disconnected => Some(Err(())),
            },
        }
    }
}

/// Begin loading a local file
pub fn begin_load() -> LoadFileState {
    let (tx, rx) = mpsc::channel();
    super::crimes::execute_asynchronously(async move {
        begin_load_helper(tx).await;
    });
    LoadFileState { rx }
}

/// This function pops up the file selector dialog and sends the selected file back through the
/// channel. It needs to be conditionally compiled because on WASM we don't get or return the path
/// to the file.
#[cfg(not(target_arch = "wasm32"))]
async fn begin_load_helper(tx: mpsc::Sender<(FileData, FileSource)>) {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("show file", &["ron"])
        .set_directory("/")
        .pick_file()
        .await;
    if let Some(file) = file {
        let data = file.read().await;
        let file_data = ron::de::from_bytes::<FileData>(&data);
        match file_data {
            Ok(file_data) => {
                let file_source = FileSource {
                    path: file.path().to_path_buf(),
                };
                let _ = tx.send((file_data, file_source));
            }
            Err(err) => log::error!("Failed to deserialize file data: {err}"),
        };
    } else {
        log::info!("File picker dialog did not return a file");
    }
}
#[cfg(target_arch = "wasm32")]
async fn begin_load_helper(tx: mpsc::Sender<(FileData, FileSource)>) {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("show file", &["ron"])
        .set_directory("/")
        .pick_file()
        .await;
    if let Some(file) = file {
        let data = file.read().await;
        let file_data = ron::de::from_bytes::<FileData>(&data);
        match file_data {
            Ok(file_data) => {
                let file_source = FileSource;
                let _ = tx.send((file_data, file_source));
            }
            Err(err) => log::error!("Failed to deserialize file data: {err}"),
        };
    } else {
        log::info!("File picker dialog did not return a file");
    }
}

/// Stores the progress of saving to a local file
pub struct SaveAsFileState {
    /// The file picker dialog thread will message us once it's done
    rx: Receiver<()>,
}
impl SaveAsFileState {
    /// Poll for whether we are done picking and saving the file
    pub fn poll_saved(&mut self) -> bool {
        match self.rx.try_recv() {
            // Message received so we are done
            Ok(()) => true,
            Err(err) => match err {
                // Message not received so we are still waiting
                mpsc::TryRecvError::Empty => false,
                // The thread either failed or died so we are done
                mpsc::TryRecvError::Disconnected => true,
            },
        }
    }
}

/// Begin saving program state as a file (open the file picker dialog to choose where to save it)
pub fn begin_save_as(file_data: FileData) -> SaveAsFileState {
    let (tx, rx) = mpsc::channel();
    let file_data = ron::ser::to_string_pretty(&file_data, ron::ser::PrettyConfig::default());
    match file_data {
        Ok(file_data) => {
            super::crimes::execute_asynchronously(async move {
                let file = rfd::AsyncFileDialog::new()
                    .add_filter("show file", &["ron"])
                    .set_file_name("showfile.ron")
                    .set_directory("/")
                    .save_file()
                    .await;
                if let Some(file) = file {
                    let result = file.write(file_data.as_bytes()).await;
                    match result {
                        Ok(_) => log::info!("Good write"),
                        Err(err) => log::warn!("Bad write: {err}"),
                    }
                } else {
                    log::info!("File picker dialog did not return a file");
                }
                // We do not care about whether the other side is still listening
                let _ = tx.send(());
            });
        }
        Err(err) => log::error!("Could not serialize file data: {err}"),
    }
    SaveAsFileState { rx }
}

// ------------------------------------------------------------------------------------------------
// Here be garbage
// ------------------------------------------------------------------------------------------------

/// Stores the progress of saving to a local file
#[cfg(not(target_arch = "wasm32"))]
pub struct SaveFileState {
    /// The writer thread will message us once it is done writing
    rx: Receiver<()>,
}
#[cfg(not(target_arch = "wasm32"))]
impl SaveFileState {
    /// Poll whether we are done writing the file
    pub fn poll_saved(&mut self) -> bool {
        match self.rx.try_recv() {
            // Done writing
            Ok(()) => true,
            Err(err) => match err {
                mpsc::TryRecvError::Empty => false,
                mpsc::TryRecvError::Disconnected => {
                    // IO thread must have crashed
                    log::error!("Failed to save file. IO thread probably crashed.");
                    // Return done so we can move on
                    true
                }
            },
        }
    }
}
/// Store the given data into the given file
#[cfg(not(target_arch = "wasm32"))]
pub fn save(file: &FileSource, data: super::FileData) -> SaveFileState {
    let path = file.path.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        match data.serialize() {
            Ok(data) => match std::fs::write(path, data) {
                Ok(_) => (),
                Err(err) => log::error!("Failed to write file: {err}"),
            },
            Err(_) => {
                log::error!("Failed to serialize data.");
            }
        }
        // Regardless of what happened, let the ui know we are done. We do not care if the ui
        // actually receives this
        let _ = tx.send(());
    });
    SaveFileState { rx }
}

#[cfg(target_arch = "wasm32")]
pub struct FileSource;
/// Stores the progress of saving to a local file. On WASM, we simply pretend that the write
/// succeeded.
// FIXME: Stop pretending that the write succeeded and write code that actually represents
// what's going on.
#[cfg(target_arch = "wasm32")]
pub struct SaveFileState;
#[cfg(target_arch = "wasm32")]
impl SaveFileState {
    /// Poll whether we are done writing the file
    pub fn poll_saved(&mut self) -> bool {
        true
    }
}
/// On WASM, we can't write to system files. Therefore, we log an error and pretend that the write
/// succeeded so that the UI doesn't have to think about it.
#[cfg(target_arch = "wasm32")]
pub fn save(_file: &FileSource, _data: super::FileData) -> SaveFileState {
    log::error!("Can't save an uploaded file. Download the file to save edits.");
    SaveFileState
}
