//! An adaptor between `BoardEdit` and the MIDI messages that the Yamaha M7CL sends and recieves
//! through the pair of three-pin MIDI ports on its back panel.
//! Multiple MIDI messages may need to be produced for a single `BoardEdit`. Consequently, the
//! adaptor produces a `Vec` of `MidiMessage`s, which are themselves `Vec<u8>`. This is not optimal
//! (FIX pls, the messages cannot be combined into one stream because some implementations (eg alsa)
//! do not function correctly if multiple messages are `send`ed in one go).

use super::{BoardEdit, BoardEditAdaptor};

pub struct YamahaM7CLMidi;

pub type MidiMessage = Vec<u8>;

impl BoardEditAdaptor for YamahaM7CLMidi {
    type Message = Vec<MidiMessage>;

    fn send_board_edit(&mut self, edit: BoardEdit) -> Result<Self::Message, super::SendError> {
        match edit {
            BoardEdit::ChannelMute(ch, mute) => Ok(send_ch_on(ch.index(), !mute)),
            BoardEdit::ChannelDcaAssign(ch, dca, assign) => {
                Ok(send_ch_dca(ch.index(), dca.index(), assign))
            }
            BoardEdit::ChannelName(ch, name) => Ok(send_channel_name(ch.index(), &name)),
            BoardEdit::DcaLevel(_dca, _level) => todo!(),
            BoardEdit::DcaName(dca, name) => Ok(send_dca_name(dca.index(), &name)),
        }
    }

    fn recv_board_message(&mut self, _message: Self::Message) -> Option<BoardEdit> {
        todo!()
    }
}

/// The MIDI channel index (ch1 = 0) to send on. MUST BE LESS THAN 16
const MIDI_CHANNEL_IND: u8 = 0;

/// Send the messages to turn on/off the given channel
fn send_ch_on(ch_ind: u8, on: bool) -> Vec<MidiMessage> {
    let val = if on {
        0b1111_1111_1111_1111
    } else {
        0b0000_0000_0000_0000
    };
    send_nrpn(0x05b6 + (ch_ind as u16), val)
}
/// Send the messages to assign/unassign the given channel from the given dca
fn send_ch_dca(ch_ind: u8, dca_ind: u8, assigned: bool) -> Vec<MidiMessage> {
    let data = if assigned { 0x01 } else { 0x00 };
    vec![send_prm_sysex(
        0x003f,
        dca_ind as u16,
        ch_ind as u16,
        [0x00, 0x00, 0x00, 0x00, data],
    )]
}
/// Send the messages to set the name of the given DCA to the given value. The name must be
/// ASCII and can be at most eight characters.
fn send_dca_name(dca_ind: u8, name: &str) -> Vec<MidiMessage> {
    let name = name.as_bytes();
    let data_1: [u8; 5] = [
        0x00,
        *name.first().unwrap_or(&0x00),
        *name.get(1).unwrap_or(&0x00),
        *name.get(2).unwrap_or(&0x00),
        *name.get(3).unwrap_or(&0x00),
    ];
    let data_2: [u8; 5] = [
        0x00,
        *name.get(4).unwrap_or(&0x00),
        *name.get(5).unwrap_or(&0x00),
        *name.get(6).unwrap_or(&0x00),
        *name.get(7).unwrap_or(&0x00),
    ];
    vec![
        // kDCAName kNameShort1
        send_prm_sysex(0x007b, 0x0000, dca_ind.into(), data_1),
        // kDCAName kNameShort2
        send_prm_sysex(0x007b, 0x0001, dca_ind.into(), data_2),
    ]
}
/// Send the messages to set the name of the given channel to the given value. The name must be
/// ASCII and can be at most eight characters.
fn send_channel_name(channel_ind: u8, name: &str) -> Vec<MidiMessage> {
    let name = name.as_bytes();
    let data_1: [u8; 5] = [
        0x00,
        *name.first().unwrap_or(&0x00),
        *name.get(1).unwrap_or(&0x00),
        *name.get(2).unwrap_or(&0x00),
        *name.get(3).unwrap_or(&0x00),
    ];
    let data_2: [u8; 5] = [
        0x00,
        *name.get(4).unwrap_or(&0x00),
        *name.get(5).unwrap_or(&0x00),
        *name.get(6).unwrap_or(&0x00),
        *name.get(7).unwrap_or(&0x00),
    ];
    vec![
        // kDCAName kNameShort1
        send_prm_sysex(0x0113, 0x0000, channel_ind.into(), data_1),
        // kDCAName kNameShort2
        send_prm_sysex(0x0113, 0x0001, channel_ind.into(), data_2),
    ]
}
/// Send the sequence of midi messages which corresponds to the given NRPN control change
/// Note: takes normal, not midi, bytes
fn send_nrpn(param: u16, val: u16) -> Vec<MidiMessage> {
    let (param_msb, param_lsb) = two_byte_midi_pack(param);
    let (val_msb, val_lsb) = two_byte_midi_pack(val);
    vec![
        vec![
            // Control change + channel
            0b1011_0000 | MIDI_CHANNEL_IND,
            // NRPN Parameter MSB
            0x63,
            // NRPN Parameter MSB value
            param_msb,
        ],
        vec![
            // Control change + channel
            0b1011_0000 | MIDI_CHANNEL_IND,
            // NRPN Parameter LSB
            0x62,
            // NRPN Parameter LSB value
            param_lsb,
        ],
        vec![
            // Control change + channel
            0b1011_0000 | MIDI_CHANNEL_IND,
            // NRPN Data MSB
            0x06,
            // NRPN Data MSB value
            val_msb,
        ],
        vec![
            // Control change + channel
            0b1011_0000 | MIDI_CHANNEL_IND,
            // NRPN Data LSB
            0x26,
            // NRPN Data LSB value
            val_lsb,
        ],
    ]
}
/// Send the midi sysex message to change parameter as given
/// Note: takes normal, not midi, bytes
fn send_prm_sysex(elem: u16, ind: u16, cc: u16, dd: [u8; 5]) -> MidiMessage {
    let (e1, e2) = two_byte_midi_pack(elem);
    let (i1, i2) = two_byte_midi_pack(ind);
    let (c1, c2) = two_byte_midi_pack(cc);
    vec![
        // Sysex
        0xf0,
        // Manufacturer id (Yamaha)
        0x43,
        // Sub status + channel
        0b0001_0000 | MIDI_CHANNEL_IND,
        // Group id (Yamaha: digital mixer)
        0x3e,
        // Model id (M7CL)
        0x11,
        // Data category (parameters, not dump or lib or meter or whatever)
        0x01,
        // Element
        e1,
        e2,
        // Index
        i1,
        i2,
        // Channel
        c1,
        c2,
        // Data
        ((dd[0] << 4) | (dd[1] >> 4)) & 0b0111_1111,
        ((dd[1] << 3) | (dd[2] >> 5)) & 0b0111_1111,
        ((dd[2] << 2) | (dd[3] >> 6)) & 0b0111_1111,
        ((dd[3] << 1) | (dd[4] >> 7)) & 0b0111_1111,
        dd[4] & 0b0111_1111,
        // End sysex
        0xf7,
    ]
}
/// Takes two normal bytes and packs them into two midi bytes. Note that this is lossy as midi
/// bytes are 7 bits.
fn two_byte_midi_pack(input: u16) -> (u8, u8) {
    (
        ((input >> 7) & 0b0111_1111) as u8,
        (input & 0b0111_1111) as u8,
    )
}
