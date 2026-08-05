//! Module containing state for saving and loading files.

use super::{ChannelNames, Cue, CueNumber, State};
use std::collections::BTreeMap;

// Saving to and loading from a local file
mod local_file;

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
    pub fn file_get_data(&self) -> FileData {
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

impl State {
    /// Commence loading a local file (put up the file picker)
    pub fn file_load_local(&mut self) {
        if self.file_tick_and_is_idle() {
            self.file_state.io_state = FileIoState::LoadingLocalFile(local_file::begin_load());
        } else {
            log::error!("Cannot begin loading a local file because IO is busy");
        }
    }
    /// Commence saving as a local file (put up the file picker so we can choose where to save it)
    pub fn file_save_as_local(&mut self) {
        if self.file_tick_and_is_idle() {
            self.file_state.io_state =
                FileIoState::SavingAsLocalFile(local_file::begin_save_as(self.file_get_data()))
        } else {
            log::error!("Cannot begin saving to a local file because IO is busy");
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
                    },
                    // Not done yet
                    None => false,
                }
            }
            FileIoState::SavingAsLocalFile(save_state) => {
                let is_done = save_state.poll_saved();
                if is_done {
                    self.file_state.io_state = FileIoState::Idle;
                }
                is_done
            }
        }
    }
    /// Saves (ctrl-s) the program state to the last loaded file. If last loaded file was a google
    /// sheet (which we cannot write to), copy the data to the clipboard so the user can paste it
    /// into the sheet manually. This takes a `Ui` so that it can copy to the clipboard; it does not
    /// render anything.
    pub fn file_save(&mut self, ui: &mut eframe::egui::Ui) {
        let data = self.file_get_data();
        // Note that the closure takes the ui reference
        let clipboard_device = |data| {
            ui.copy_text(data);
        };
        self.file_state.save(data, Some(clipboard_device));
    }
    /// Like `file_save`, but always exports to clipboard
    pub fn file_to_clipboard(&mut self, ui: &mut eframe::egui::Ui) {
        let data = self.file_get_data();
        if let Ok(data) = data.serialize() {
            ui.copy_text(data);
        } else {
            log::error!("Could not serialize data")
        }
    }
    /// Try to load a file from the input field that you can paste into
    pub fn file_try_load_from_clipboard_buffer(&mut self) {
        match ron::de::from_str(&self.file_state.ui_from_clipboard_buffer) {
            Ok(file_data) => {
                self.file_state.loaded_file = Some(FileSource::Clipboard);
                self.file_state.ui_from_clipboard_buffer.clear();
                self.file_state.ui_last_clipboard_error = None;
                self.file_load_data(file_data);
            }
            Err(err) => self.file_state.ui_last_clipboard_error = Some(err),
        }
    }
}

/// Represents what file we have loaded, if any, and the state of loading a new file if we are doing
/// that.
#[derive(Default)]
pub struct FileState {
    loaded_file: Option<FileSource>,
    io_state: FileIoState,
    ui_from_clipboard_buffer: String,
    ui_last_clipboard_error: Option<ron::error::SpannedError>,
}
impl FileState {
    pub fn is_idle(&self) -> bool {
        matches!(self.io_state, FileIoState::Idle)
    }
    pub fn loaded_file(&self) -> &Option<FileSource> {
        &self.loaded_file
    }
    pub fn ui_from_clipboard_buffer_mut(&mut self) -> &mut String {
        &mut self.ui_from_clipboard_buffer
    }
    pub fn ui_last_clipboard_error(&self) -> &Option<ron::error::SpannedError> {
        &self.ui_last_clipboard_error
    }
    /// Save current program state to the currently loaded file. If the file type does not support
    /// writing, try the optionally provided closure.
    pub fn save(&mut self, file_data: FileData, receiver: Option<impl FnOnce(String) -> ()>) {
        if !self.is_idle() {
            log::warn!("Tried to save but io is busy");
            return;
        }
        match &mut self.loaded_file {
            Some(file) => match file {
                FileSource::LocalFile(file) => {
                    let save_file_state = local_file::save(file, file_data);
                    self.io_state = FileIoState::SavingLocalFile(save_file_state);
                }
                FileSource::Clipboard => {
                    if let Some(receiver) = receiver {
                        if let Ok(data) = file_data.serialize() {
                            receiver(data);
                        } else {
                            log::error!("Could not export to clipboard because data could not be serialized")
                        }
                    } else {
                        log::error!(
                            "Could not export to clipboard because no receiver was provided. The function was probably called incorrectly."
                        );
                    }
                }
            },
            None => log::warn!("Tried to save file but no file is loaded to save to"),
        }
    }
}

/// The different places a file can come from.
pub enum FileSource {
    LocalFile(local_file::FileSource),
    Clipboard,
}

#[derive(Default)]
enum FileIoState {
    #[default]
    Idle,
    SavingLocalFile(local_file::SaveFileState),
    LoadingLocalFile(local_file::LoadFileState),
    SavingAsLocalFile(local_file::SaveAsFileState),
}

/// Contains "solution" for dealing with futures
mod crimes {
    // On WASM, it is illegal to block the main thread and it is illegal to block on a future. This
    // could possibly be solved less awkwardly though so please FIX it at some point.
    //
    // Browser: this application has ONE THREAD. You WILL NOT block on futures
    // YES CHEF
    //
    // This `wasm_bindgen_futures::spawn_local` function runs the future asap as a JavaScript
    // promise.
    #[cfg(not(target_arch = "wasm32"))]
    pub use execute_in_thread as execute_asynchronously;
    #[cfg(target_arch = "wasm32")]
    pub use wasm_bindgen_futures::spawn_local as execute_asynchronously;

    /// On native platforms, spawn a thread which blocks on a future (ie executes it)
    pub fn execute_in_thread(future: impl Future + std::marker::Send + 'static) {
        use pollster::FutureExt as _;
        std::thread::spawn(|| {
            future.block_on();
        });
    }
}
