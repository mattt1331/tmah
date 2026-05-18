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

    /// Are we actually connected?
    fn connected(&self) -> bool {
        false
    }
    /// Draw the UI for the connection in the board screen
    fn ui(&self, ui: &mut eframe::egui::Ui) {}

    /// Fire the provided cue
    fn fire_cue(&mut self, cue: &super::Cue);
}

/// A list of all of our boards. Use `Connenctions::construct` to pick one from the list.
#[derive(Default)]
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

pub struct NoConnection {}
impl Connectable for NoConnection {
    fn new() -> Self {
        NoConnection {}
    }
    fn num_channels(&self) -> u8 {
        0
    }
    fn fire_cue(&mut self, _cue: &super::Cue) {}
}

/// Represents a channel, which can be assigned to a DCA
#[derive(Clone)]
pub struct Channel {
    index: u8,
}

impl Channel {
    /// Returns the zero-indexed index of the channel (eg Ch1 returns 0)
    fn index(&self) -> u8 {
        self.index
    }
    /// Returns the number of the channel (eg Ch1 returns 1)
    fn number(&self) -> u8 {
        self.index + 1
    }
}
