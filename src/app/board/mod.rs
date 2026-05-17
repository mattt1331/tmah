//! Contains interfaces and implementations for each board

mod m7cl_midi;

/// A generic connection. Implemented by each board
pub trait Connectable {
    /// Create a new connection object of this type
    fn new() -> Self
    where
        Self: Sized;
    /// Are we actually connected?
    fn connected(&self) -> bool {
        false
    }
    /// Draw the UI for the connection in the board screen
    fn ui(&self, ui: &mut eframe::egui::Ui) {}
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
}

/// Represents a single channel-like object (eg channel, bus, DCA, etc)
pub trait ChannelLike {}

/// Represents a single channel-like object which can be assigned to a DCA
pub trait DcaAssignable: ChannelLike {}
