//! Module containing serialization and deserialization of show file

use super::{Cue, State};

/// Data saved in the show file
#[derive(serde::Serialize, serde::Deserialize)]
struct FileData {
    cues: Vec<Cue>,
    // TODO: connections serialization
}

impl State {
    /// Returns the data which will be saved in the show file
    pub fn file_data(&self) -> FileData {
        // Remember to update the load function as well
        // Do that first pls
        FileData {
            cues: self.cues.clone(),
        }
    }
    /// Loads the given data (ie sets state equal to provided values)
    pub fn load_file_data(&mut self, data: FileData) {
        self.cues = data.cues;
    }
    // Brick because they're dumb like bricks and only temprorary im tired
    pub fn brick_save(&self) {
        log::error!("Somebody called the brick save function");
        let data = self.file_data();
        let toml_data = toml::to_string(&data);
        if let Ok(toml_data) = toml_data {
            let _ = std::fs::write("brick.toml", toml_data);
        } else {
            log::error!("Failed to serialize data");
        }
    }
    pub fn brick_load(&mut self) {
        log::error!("Somebody called the brick load function");
        let data = std::fs::read("brick.toml");
        if let Ok(data) = data {
            let data = toml::from_slice::<FileData>(&data);
            if let Ok(data) = data {
                log::info!("Successfully deserialized brick.toml");
                self.load_file_data(data);
            } else {
                log::error!("Failed to deserialize data")
            }
        } else {
            log::error!("Failed to read data")
        }
    }
}
