//! Module containing state for saving and loading files. 

use std::collections::BTreeMap;
use super::{CueNumber, Cue, ChannelNames, State};

// Saving to and loading from a local file
mod local_file;
// Saving to and loading from a google sheet
mod google_sheet;

impl State {
    /// Commence loading a local file (put up the file picker)
    pub fn file_load_local(&mut self) {
        if self.file_tick_and_is_idle() {
            self.file_state.io_state = FileIoState::LoadingLocalFile(local_file::begin_load());
        } else {
            log::error!("Cannot begin loading a local file because IO is busy");
        }
    }
    /// Polls file IO, updates state (eg loads file) if needed, and returns whether IO is idle so we
    /// can start doing things. This should preferably get called once per frame or whatever
    pub fn file_tick_and_is_idle(&mut self) -> bool {
        match &mut self.file_state.io_state {
            FileIoState::Idle => true,
            FileIoState::SavingLocalFile(save_state) => {
                let is_done = save_state.poll_saved();
                if is_done {
                    self.file_state.io_state = FileIoState::Idle;
                }
                is_done
            }
            FileIoState::SavingGoogleSheet(save_state) => {
                let is_done = save_state.poll_saved();
                if is_done {
                    self.file_state.io_state = FileIoState::Idle;
                }
                is_done
            }
            FileIoState::LoadingLocalFile(load_state) => {
                match load_state.poll_loaded() {
                    Some(load_result) => match load_result {
                        Ok(file_data) => {
                            self.file_load_data(file_data);
                            self.file_state.io_state = FileIoState::Idle;
                            // We succeeded and are done
                            true
                        }
                        Err(()) => {
                            log::error!("Failed to load file");
                            self.file_state.io_state = FileIoState::Idle;
                            // We failed so we are done
                            true
                        }
                    }
                    // Not done yet
                    None => false,
                }
            }
        }
    }
}

/// Represents what file we have loaded, if any, and the state of loading a new file if we are doing
/// that.
#[derive(Default)]
pub struct FileState {
    loaded_file: Option<FileSource>,
    io_state: FileIoState,
}
impl FileState {
    pub fn is_idle(&self) -> bool {
        matches!(self.io_state, FileIoState::Idle)
    }
    pub fn loaded_file(&self) -> &Option<FileSource> {
        &self.loaded_file
    }
    /// Save current program state to the currently loaded file
    pub fn save(&mut self, file_data: FileData) {
        if !self.is_idle() {
            log::warn!("Tried to save but io is busy");
            return;
        }
        match &mut self.loaded_file {
            Some(file) => match file {
                FileSource::LocalFile(file) => {
                    let save_file_state = local_file::save(&file, file_data);
                    self.io_state = FileIoState::SavingLocalFile(save_file_state);
                }
                FileSource::GoogleSheet(file) => {
                    let save_file_state = google_sheet::save(&file, file_data);
                    self.io_state = FileIoState::SavingGoogleSheet(save_file_state);
                }
            }
            None => log::warn!("Tried to save file but no file is loaded to save to"),
        }
    }
}

/// The different places a file can come from.
pub enum FileSource {
    LocalFile(local_file::FileSource),
    GoogleSheet(google_sheet::FileSource),
}

#[derive(Default)]
enum FileIoState {
    #[default]
    Idle,
    SavingLocalFile(local_file::SaveFileState),
    SavingGoogleSheet(google_sheet::SaveFileState),
    LoadingLocalFile(local_file::LoadFileState)
}

/// Data saved in the show file
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FileData {
    cues: BTreeMap<CueNumber, Cue>,
    channel_names: ChannelNames,
}
impl FileData {
    /// Serializes data to RON as a string
    fn serialize(&self) -> Result<String, ()> {
        match ron::to_string(self) {
            Ok(data) => Ok(data),
            Err(err) => {
                log::error!("Failed to serialize file data: {err}");
                Err(())
            }
        }
    }
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
    pub fn file_load_data(&mut self, data: FileData) {
        self.cues = data.cues;
        self.ch_names = data.channel_names;
    }
}

/// Contains "solution" for dealing with futures
mod crimes {
    // On WASM, it is illegal to block the main thread and it is illegal to block on a future. This
    // could possibly be solved less awkwardly though so please FIX it at some point
    //
    // Browser: this application has ONE THREAD. You WILL NOT block on futures
    // YES CHEF
    #[cfg(target_arch = "wasm32")]
    pub use wasm_bindgen_futures::spawn_local as execute_asynchronously;
    #[cfg(not(target_arch = "wasm32"))]
    pub use execute_in_thread as execute_asynchronously;

    /// On native platforms, spawn a thread which blocks on a future (ie executes it)
    pub fn execute_in_thread(future: impl Future + std::marker::Send + 'static) {
        use pollster::FutureExt as _;
        std::thread::spawn(|| {
            future.block_on();
        });
    }
}
