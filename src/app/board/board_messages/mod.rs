//! A set of adaptors between `BoardEdit` and a board's native message format.
use super::BoardEdit;

/// A common interface to all of the adaptors.
pub trait BoardEditAdaptor {
    /// The messages the board sends/recieves that this adaptor converts to/from `BoardEdit`.
    type Message;
    /// Converts a `BoardEdit` to a `Message` to be sent to the board.
    fn send_board_edit(&mut self, edit: BoardEdit) -> Result<Self::Message, SendError>;
    /// Receives a message from the board, possibly converting it to a `BoardEdit` if appropriate
    #[allow(dead_code)] //FIX: delete this once we use this the warning is annoying me
    fn recv_board_message(&mut self, message: Self::Message) -> Option<BoardEdit>;
}

/// An error which may arise when attempting to convert a `BoardEdit` to a board message.
#[derive(Debug)]
pub enum SendError {
    /// This variant of `BoardEdit` is not supported by the implementation for this board.
    NotSupported,
}

mod ah_sq7_midi;
mod yamaha_m7cl_midi;

pub use ah_sq7_midi::AHSQ7Midi;
pub use yamaha_m7cl_midi::YamahaM7CLMidi;
