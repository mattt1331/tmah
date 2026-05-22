//! Contains interfaces and implementations for each board

mod m7cl_midi;

/// A generic connection. Implemented by each board
pub trait Connectable {
    /// Create a new connection object of this type
    fn new() -> Self
    where
        Self: Sized;

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
}

/// A list of all of our boards. Use `Connenctions::construct` to pick one from the list.
// Remember to add new entries to the dropdown
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Connections {
    #[default]
    None,
    M7CLMidi,
}

impl Connections {
    /// Returns the actual connection object
    pub fn construct(self) -> Box<dyn Connectable> {
        match self {
            Connections::None => Box::new(NoConnection::new()),
            Connections::M7CLMidi => Box::new(m7cl_midi::M7CLMidi::new()),
        }
    }
}

pub struct NoConnection {
    num_dcas: u8,
    num_channels: u8,

    num_dcas_ui: String,
    num_channels_ui: String,
}
impl Connectable for NoConnection {
    fn new() -> Self {
        NoConnection {
            num_dcas: 8,
            num_channels: 20,
            num_dcas_ui: "8".to_string(),
            num_channels_ui: "20".to_string(),
        }
    }
    fn num_channels(&self) -> u8 {
        self.num_channels
    }
    fn num_dcas(&self) -> u8 {
        self.num_dcas
    }
    fn fire_cue(&mut self, _cue: &super::Cue) {
        log::warn!("Cue was fired with no connection")
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

/// Represents a channel, which can be assigned to a DCA
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Channel {
    index: u8,
}

impl Channel {
    /// Returns a `Channel` with the specified index (eg `ind` = 0 -> Ch1)
    pub fn from_index(ind: u8) -> Self {
        Channel { index: ind }
    }
    /// Returns a `Channel` with the specified number (eg `num` = 1 -> Ch1). Returns None if `num` is
    /// zero.
    pub fn from_number(num: u8) -> Option<Channel> {
        if num == 0 {
            None
        } else {
            Some(Channel { index: num - 1 })
        }
    }
    /// Returns the zero-indexed index of the channel (eg Ch1 returns 0)
    pub fn index(&self) -> u8 {
        self.index
    }
    /// Returns the number of the channel (eg Ch1 returns 1)
    pub fn number(&self) -> u8 {
        self.index + 1
    }
}
