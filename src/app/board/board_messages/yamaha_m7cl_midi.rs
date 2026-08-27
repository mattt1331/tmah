//! An adaptor between `BoardEdit` and the MIDI messages that the Yamaha M7CL sends and receives
//! through the pair of three-pin MIDI ports on its back panel.
//! Important note: This adaptor makes certain assumptions about how the board is configured. See
//! `UI_BOARD_CONFIG_TUTORIAL` for more information. Namely, we receive all messages from the board
//! as SYSEX, because we need to listen for SYSEX anyways (DCA assignment can only be done via
//! SYSEX). Consequently, in order to avoid having two streams of duplicate messages being sent at
//! us, we receive everything via SYSEX.

use super::super::{Channel, Dca};
use super::util_midi::MidiMessage as ParsedMidiMessage;
use super::{BoardEdit, BoardEditAdaptor};

pub const UI_BOARD_CONFIG_TUTORIAL: &str = "\
Note: The board's MIDI settings need to be configured correctly. Ensure that the following are set:
    1. PORT/CH Tx and Rx are set to CH1
    2. CONTROL CHANGE Rx is on
    3. CONTROL CHANGE mode is set to NRPN
    4. PARAMETER CHANGE Tx and Rx are on
Preferably, all of the other Tx, Rx, and ECHO settings should be turned off.";

pub struct YamahaM7CLMidi {
    // DCA names get sent in halves for some reason
    last_dca_name_short_1: Option<(Dca, [u8; 4])>,
    last_dca_name_short_2: Option<(Dca, [u8; 4])>,
}

impl YamahaM7CLMidi {
    /// Create a new instance of this adaptor. Note that this adaptor is stateful because messages
    /// are received in parts.
    pub fn new() -> Self {
        YamahaM7CLMidi {
            last_dca_name_short_1: None,
            last_dca_name_short_2: None,
        }
    }
}

pub type MidiMessage = Vec<u8>;

impl BoardEditAdaptor for YamahaM7CLMidi {
    type Message = MidiMessage;

    fn send_board_edit(
        &mut self,
        edit: BoardEdit,
    ) -> Result<impl Iterator<Item = MidiMessage>, super::SendError> {
        match edit {
            BoardEdit::ChannelMute(ch, mute) => Ok(send_ch_on(ch.index(), !mute).into_iter()),
            BoardEdit::ChannelDcaAssign(ch, dca, assign) => {
                Ok(send_ch_dca(ch.index(), dca.index(), assign).into_iter())
            }
            BoardEdit::ChannelName(ch, name) => {
                Ok(send_channel_name(ch.index(), &name).into_iter())
            }
            BoardEdit::DcaLevel(_dca, _level) => todo!(),
            BoardEdit::DcaName(dca, name) => Ok(send_dca_name(dca.index(), &name).into_iter()),
        }
    }

    /// Parses MIDI messages into `BoardEdits`. Note that this will only parse the `ChannelMute`,
    /// `ChannelDcaAssign`, and `DcaName` variants because at time of writing those are the only
    /// ones that we care about receiving in the codebase.
    // TODO: Implement selecting by MIDI channel
    fn recv_board_message(
        &mut self,
        message: Self::Message,
    ) -> Option<impl Iterator<Item = BoardEdit>> {
        // ChannelMute: kInputOn
        // ChannelDcaAssign: kInputDCA
        // DcaName: kDCAName
        // We receive everything over SYSEX. See module docs for appropriate configuration
        if let Some((_midi_channel, ParsedMidiMessage::Sysex(message))) = ParsedMidiMessage::from_bytes(&message)
            // Parse the sysex content into its various elements
            && let Some((elem, ind, cc, dd)) = recv_prm_sysex(&message)
        {
            match elem {
                // kInputOn
                0x0030 => {
                    // channel is cc CH TABLE 1
                    let channel = Channel::from_index(cc as u8);
                    // data is off, on; 0, 1
                    let mute = dd[0] == 0;
                    return Some(std::iter::once(BoardEdit::ChannelMute(channel, mute)));
                }
                // kInputDCA
                0x003f => {
                    // ind is dca index
                    let dca = Dca::from_index(ind as u8);
                    // cc is channel CH TABLE 1
                    let channel = Channel::from_index(cc as u8);
                    // data is not assign, assign; 0, 1
                    let is_assigned = dd[0] != 0;
                    return Some(std::iter::once(BoardEdit::ChannelDcaAssign(
                        channel,
                        dca,
                        is_assigned,
                    )));
                }
                // kDcaName
                0x007b => {
                    // cc is TABLE #07
                    let dca = Dca::from_index(cc as u8);
                    // dd is midi packed ascii (bruh)
                    let data = [
                        dd[0] << 4 | dd[1] >> 3,
                        dd[1] << 5 | dd[2] >> 2,
                        dd[2] << 6 | dd[3] >> 1,
                        dd[3] << 7 | dd[4],
                    ];
                    // ind is 0 -> kNameShort1, 1 -> kNameShort2
                    if ind == 0 {
                        self.last_dca_name_short_1 = Some((dca.clone(), data));
                    }
                    if ind == 1 {
                        self.last_dca_name_short_2 = Some((dca, data));
                    }
                    if let Some((dca_1, data_1)) = &self.last_dca_name_short_1
                        && let Some((dca_2, data_2)) = &self.last_dca_name_short_2
                        && dca_1 == dca_2
                    {
                        let dca_name = data_1
                            .iter()
                            .cloned()
                            .chain(data_2.iter().cloned())
                            .collect();
                        let dca = dca_1.clone();
                        self.last_dca_name_short_1 = None;
                        self.last_dca_name_short_2 = None;
                        if let Ok(dca_name) = String::from_utf8(dca_name) {
                            return Some(std::iter::once(BoardEdit::DcaName(dca, dca_name)));
                        }
                    }
                }
                // Something else
                _ => (),
            }
        }
        None::<std::iter::Once<_>>
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
    let param_msb = (param >> 8) as u8;
    let param_lsb = param as u8;
    let val_msb = (val >> 8) as u8;
    let val_lsb = val as u8;
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
    let e1 = (elem >> 8) as u8;
    let e2 = elem as u8;
    let i1 = (ind >> 8) as u8;
    let i2 = ind as u8;
    let c1 = (cc >> 8) as u8;
    let c2 = cc as u8;
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
/// Unpack a midi sysex message to change parameter into its arguments.
fn recv_prm_sysex(content: &[u8]) -> Option<(u16, u16, u16, [u8; 5])> {
    if content.is_empty() {
        return None;
    }
    // We accept messages both with and without the leading byte
    let offset = if content[0] == 0xf0 { 1 } else { 0 };
    if content.len() < 16 + offset {
        return None;
    }
    Some((
            (content[5+offset] as u16) << 8 | content[6+offset] as u16,
            (content[7+offset] as u16) << 8 | content[8+offset] as u16,
            (content[9+offset] as u16) << 8 | content[10+offset] as u16,
            content[11+offset..16+offset].try_into().expect("Slice should be the correct size on account of it being sliced right here with the correct size")
            ))
}
