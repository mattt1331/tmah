//! A set of adaptors between `BoardEdit` and a board's native message format.
use super::BoardEdit;

/// A common interface to all of the adaptors.
pub trait BoardEditAdaptor {
    /// The messages the board sends/recieves that this adaptor converts to/from `BoardEdit`.
    type Message;
    // FIX: For both send and recv, take and return references instead of the owned values because
    // performance.
    /// Converts a `BoardEdit` to a `Message` to be sent to the board.
    fn send_board_edit(
        &mut self,
        edit: BoardEdit,
    ) -> Result<impl Iterator<Item = Self::Message>, SendError>;
    /// Receives a message from the board, possibly converting it to a `BoardEdit` if appropriate
    fn recv_board_message(
        &mut self,
        message: Self::Message,
    ) -> Option<impl Iterator<Item = BoardEdit>>;
}

/// An error which may arise when attempting to convert a `BoardEdit` to a board message.
#[derive(Debug)]
pub enum SendError {
    /// This variant of `BoardEdit` is not supported by the implementation for this board.
    NotSupported,
}

mod yamaha_m7cl_midi;

pub use yamaha_m7cl_midi::UI_BOARD_CONFIG_TUTORIAL as YAMAHA_M7CL_MIDI_UI_BOARD_CONFIG_TUTORIAL;
pub use yamaha_m7cl_midi::YamahaM7CLMidi;

mod util_midi;
