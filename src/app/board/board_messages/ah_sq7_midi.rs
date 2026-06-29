//! An adaptor between `BoardEdit` and the MIDI messages that the Allen and Heath SQ7 sends and
//! recieves through the USB port on its back panel.
//! See the M7CL for disscussion about the choice of types here.

use super::{BoardEditAdaptor, BoardEdit};

pub struct AHSQ7Midi;

pub type MidiMessage = Vec<u8>;

impl BoardEditAdaptor for AHSQ7Midi {
    type Message = Vec<MidiMessage>;

    fn send_board_edit(&mut self, _edit: BoardEdit) -> Result<Self::Message, super::SendError> {
        todo!()
    }

    fn recv_board_message(&mut self, _message: Self::Message) -> Option<BoardEdit> {
        todo!()
    }
}
