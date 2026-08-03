//! Module containing state for saving and loading files. 

use std::collections::BTreeMap;
use super::{CueNumber, Cue, ChannelNames};

// Saving to and loading from a local file
mod local_file;
// Saving to and loading from a google sheet
mod google_sheet;

/// Represents what file we have loaded, if any, and the state of loading a new file if we are doing
/// that.
#[derive(Default)]
pub struct FileState {
    loaded_file: Option<FileSource>,
    io_state: FileIoState,
}
impl FileState {
    pub fn loaded_file(&self) -> &Option<FileSource> {
        &self.loaded_file
    }
    /// Updates IO state and returns whether IO is idle
    pub fn tick_io_and_is_idle(&mut self) -> bool {
        match &mut self.io_state {
            FileIoState::Idle => true,
            FileIoState::Saving(save_state) => {
                let is_done = save_state.poll_saved();
                if is_done {
                    self.io_state = FileIoState::Idle;
                }
                is_done
            }
        }
    }
    /// Save current program state to the currently loaded file
    pub fn save(&mut self, file_data: FileData) {
        if !self.tick_io_and_is_idle() {
            log::warn!("Tried to save but io is busy");
            return;
        }
        match &mut self.loaded_file {
            Some(file) => match file {
                FileSource::LocalFile(file) => {
                    let save_file_state = local_file::save(&file, file_data);
                    self.io_state = FileIoState::Saving(SaveFileState::SavingLocalFile(save_file_state));
                }
                FileSource::GoogleSheet(file) => {
                    let save_file_state = google_sheet::save(&file, file_data);
                    self.io_state = FileIoState::Saving(SaveFileState::SavingGoogleSheet(save_file_state));
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
    Saving(SaveFileState),
}

enum SaveFileState {
    SavingLocalFile(local_file::SaveFileState),
    SavingGoogleSheet(google_sheet::SaveFileState),
}
impl SaveFileState {
    /// Check if we are done saving.
    fn poll_saved(&mut self) -> bool {
        match self {
            SaveFileState::SavingLocalFile(save_state) => save_state.poll_saved(),
            SaveFileState::SavingGoogleSheet(save_state) => save_state.poll_saved(),
        }
    }
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
