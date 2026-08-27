//! Contains interfaces and implementations for each board
//!
//! To create a connectable board, you only need to implement the `Connectable` trait (and add the
//! board to the ui dropdown). In addition to this trait, there are several submodules containing
//! useful tools for implementing the connection.
//! - `data`: Contains types commonly used throughout the application to represent things related
//!   to boards.
//! - `board_messages`: Contains the `BoardEditAdaptor` trait and implementations. `BoardEdit` is
//!   an enumeration of all edits to the board that we support and each implementation of the
//!   `BoardEditAdaptor` trait knows how to convert that edit into a message for each particular
//!   board. This setup is nice because it allows us to reuse connection logic and keep it separate
//!   from the details of the board's protocol.
//! - `board_state_cache`: Contains the `BoardStateCache` type, which lets us track a board's state
//!   and knows how to use that knowledge to efficiently bring the board to a desired state without
//!   sending unnecessary messages.

// Types used by this module to represent data common to all boards
mod data;
pub use data::{BoardEdit, Channel, Dca, Decibels};
// Library of `BoardEdit` to message and vice versa (low level api)
mod board_messages;
// System for tracking the state of the board
mod board_state_cache;

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

    /// An optional method that is called every frame to allow the connection to do some work
    fn heartbeat(&mut self) {
        // Optional, do nothing by default
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
                board_messages::YamahaM7CLMidi::new(),
                board_messages::YAMAHA_M7CL_MIDI_UI_BOARD_CONFIG_TUTORIAL.to_string(),
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
