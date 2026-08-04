//! Saving to and loading from a local file

use std::sync::mpsc::{self, Receiver};
use super::FileData;

/// A local file which we are loading to/from
#[cfg(not(target_arch = "wasm32"))]
pub struct FileSource {
    path: std::path::PathBuf,
}

/// Stores the progress of loading a local file
pub struct LoadFileState {
    /// Receiver which will receive the file
    rx: Receiver<FileData>
}
impl LoadFileState {
    /// Poll whether we are done loading the file. If we are still waiting, `None`. If we are done,
    /// `Some` with either the file or an oops
    pub fn poll_loaded(&mut self) -> Option<Result<FileData, ()>> {
        match self.rx.try_recv() {
            Ok(file_data) => Some(Ok(file_data)),
            Err(err) => match err {
                // Nothing sent to us yet
                mpsc::TryRecvError::Empty => None,
                // Loader thread failed/died
                mpsc::TryRecvError::Disconnected => Some(Err(())),
            }
        }
    }
}

/// Begin loading a local file
pub fn begin_load() -> LoadFileState {
    let (tx, rx) = mpsc::channel();
    super::crimes::execute_asynchronously(async move {
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
                    let _ = tx.send(file_data);
                }
                Err(err) => log::error!("Failed to deserialize file data: {err}"),
            };
        } else {
            log::info!("File picker dialog did not return a file");
        }
    });
    LoadFileState { rx }
}

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
            }
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
            Ok(data) => {
                match std::fs::write(path, data) {
                    Ok(_) => (),
                    Err(err) => log::error!("Failed to write file: {err}"),
                }
            }
            Err(_) => {
                log::error!("Failed to serialize data.");
            }
        }
        // Regardless of what happened, let the ui know we are done
        tx.send(());
    });
    SaveFileState { rx }
}


// ---------------------------------
// Here be garbage
// ---------------------------------

#[cfg(target_arch = "wasm32")]
pub struct FileSource;
/// Stores the progress of saving to a local file. On WASM, we simply pretend that the write
/// succeeded.
// FIXME: Stop pretending that the write succeeded and write code that actually represents
// what's going on.
#[cfg(target_arch = "wasm32")]
struct SaveFileState;
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
