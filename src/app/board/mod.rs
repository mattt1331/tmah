//! Contains interfaces and implementations for each board

// Types used by this module to represent data common to all boards
mod data;
pub use data::{BoardEdit, Channel, Dca, Decibels};
// Library of `BoardEdit` to message and vice versa (low level api)
mod board_messages;

mod generic_generic_midi;

/// A generic connection. Implemented by each board. This, in combination with `Connections`, is
/// this module's public api.
pub trait Connectable {
    /// The number of channels this board has
    fn num_channels(&self) -> u8;
    /// The number of DCAs this board has
    fn num_dcas(&self) -> u8;

    /// Are we actually connected?
    fn connected(&self) -> bool {
        false
    }
    /// Draw the UI specific to the connection in the board screen
    fn ui(&mut self, ui: &mut eframe::egui::Ui) {
        ui.heading("somebody forgot to implement this ui :(");
    }

    /// Fire the provided cue
    fn fire_cue(&mut self, cue: &super::Cue);
    /// Set the channel names on the board to match the specified names
    fn fire_channel_names(&mut self, names: &super::ChannelNames);
}

/// A list of all of our boards. Use `Connenctions::construct` to pick one from the list.
// Remember to add new entries to the dropdown
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Connections {
    #[default]
    None,
    YamahaM7CLMidi,
}

impl Connections {
    /// Returns the actual connection object
    pub fn construct(self) -> Box<dyn Connectable> {
        match self {
            Connections::None => Box::new(NoConnection::new()),
            Connections::YamahaM7CLMidi => Box::new(generic_generic_midi::GenericGenericMidi::new(
                board_messages::YamahaM7CLMidi,
            )),
        }
    }
}

pub struct NoConnection {
    num_dcas: u8,
    num_channels: u8,

    num_dcas_ui: String,
    num_channels_ui: String,
}
impl NoConnection {
    pub fn new() -> Self {
        NoConnection {
            num_dcas: 8,
            num_channels: 20,
            num_dcas_ui: "8".to_string(),
            num_channels_ui: "20".to_string(),
        }
    }
}
impl Connectable for NoConnection {
    fn num_channels(&self) -> u8 {
        self.num_channels
    }
    fn num_dcas(&self) -> u8 {
        self.num_dcas
    }
    fn fire_cue(&mut self, _cue: &super::Cue) {
        log::warn!("Cue was fired with no connection")
    }
    fn fire_channel_names(&mut self, _: &super::ChannelNames) {
        log::warn!("Channel names fired with no connection")
    }
    fn ui(&mut self, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Number of DCAs (for testing):");
            let response = ui.add(eframe::egui::TextEdit::singleline(&mut self.num_dcas_ui));
            if response.changed() {
                let new_val = self.num_dcas_ui.parse::<u8>();
                match new_val {
                    Ok(val) => {
                        if val > 0 {
                            self.num_dcas = val;
                        } else {
                            self.num_dcas = 1;
                            self.num_dcas_ui = "1".to_string();
                        }
                    }
                    Err(_) => self.num_dcas_ui = self.num_dcas.to_string(),
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Number of channels (for testing):");
            let response = ui.add(eframe::egui::TextEdit::singleline(
                &mut self.num_channels_ui,
            ));
            if response.changed() {
                let new_val = self.num_channels_ui.parse::<u8>();
                match new_val {
                    Ok(val) => {
                        if val > 0 {
                            self.num_channels = val;
                        } else {
                            self.num_channels = 1;
                            self.num_channels_ui = "1".to_string();
                        }
                    }
                    Err(_) => self.num_channels_ui = self.num_channels.to_string(),
                }
            }
        });
    }
}
