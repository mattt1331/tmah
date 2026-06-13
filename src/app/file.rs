//! Module containing serialization and deserialization of show file

// On WASM, it is illegal to block the main thread and it is illegal to block on a future. This
// could possibly be done less awkwardly though so please FIX it at some point
//
// Browser: this application has ONE THREAD. You WILL NOT block on futures
// YES CHEF
#[cfg(not(target_arch = "wasm32"))]
use spawn_future_thread as future_go;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local as future_go;

use super::{ChannelNames, Cue, CueNumber, State};
use std::collections::BTreeMap;

/// Data saved in the show file
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FileData {
    cues: BTreeMap<CueNumber, Cue>,
    channel_names: ChannelNames, // TODO: connections serialization
}

/// Managing file-related state
impl State {
    /// Is the load screen idle, or is some dialog up? Note: this must be called because it also
    /// updates state
    pub fn file_is_idle(&mut self) -> bool {
        match &self.file_state {
            FileState::Idle => true,
            FileState::LoadingFile(rx) => match rx.try_recv() {
                Ok(file_data) => {
                    self.load_file_data(file_data);
                    self.cancel_file_dialog();
                    true
                }
                Err(recv_err) => match recv_err {
                    std::sync::mpsc::TryRecvError::Empty => false,
                    std::sync::mpsc::TryRecvError::Disconnected => {
                        self.cancel_file_dialog();
                        true
                    }
                },
            },
        }
    }
    /// If a file dialog is up, cancel it.
    pub fn cancel_file_dialog(&mut self) {
        self.file_state = FileState::Idle;
    }
    /// Say that we are waiting on a file load
    fn register_file_load(&mut self, rx: std::sync::mpsc::Receiver<FileData>) {
        self.file_state = FileState::LoadingFile(rx);
    }
}
#[derive(Default)]
pub enum FileState {
    #[default]
    Idle,
    LoadingFile(std::sync::mpsc::Receiver<FileData>),
}
/// Extracting/inserting data to save/saved data
impl State {
    /// Returns the data which will be saved in the show file
    pub fn file_data(&self) -> FileData {
        // Remember to update the load function as well
        // Do that first pls
        FileData {
            cues: self.cues.clone(),
            channel_names: self.ch_names.clone(),
        }
    }
    /// Loads the given data (ie sets state equal to provided values)
    pub fn load_file_data(&mut self, data: FileData) {
        self.cues = data.cues;
        self.ch_names = data.channel_names;
    }
}
/// Saving (serializing) data
impl State {
    pub fn save_as(&self) {
        let file_data = ron::to_string(&self.file_data());
        match file_data {
            Ok(file_data) => {
                future_go(async move {
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
                });
            }
            Err(err) => log::error!("Could not serialize file data: {err}"),
        }
    }
}
/// Loading (deserializing) data
impl State {
    pub fn load(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.register_file_load(rx);
        future_go(async move {
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
    }
}

/// On native platforms, spawn a thread which blocks on a future (ie executes it)
fn spawn_future_thread(future: impl Future + std::marker::Send + 'static) {
    use pollster::FutureExt as _;
    std::thread::spawn(|| {
        future.block_on();
    });
}
