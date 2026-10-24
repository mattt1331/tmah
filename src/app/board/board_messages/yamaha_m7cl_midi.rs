//! An adaptor between `BoardEdit` and the MIDI messages that the Yamaha M7CL sends and receives
//! through the pair of three-pin MIDI ports on its back panel.
//! Important note: This adaptor makes certain assumptions about how the board is configured. See
//! `UI_BOARD_CONFIG_TUTORIAL` for more information. Namely, we receive all messages from the board
//! as SYSEX, because we need to listen for SYSEX anyways (DCA assignment can only be done via
//! SYSEX). Consequently, in order to avoid having two streams of duplicate messages being sent at
//! us, we receive everything via SYSEX.

use super::super::{Channel, Dca, Pan};
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
            BoardEdit::ChannelPan(ch, pan) => Ok(send_ch_pan(ch.index(), pan).into_iter()),
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
/// Send the messages to set the pan of the given channel
fn send_ch_pan(ch_ind: u8, pan: Pan) -> Vec<MidiMessage> {
    let param: u16 = 0x2096 + ch_ind as u16;
    // The M7CL wants the pan as an integer in [-63, 63]. I assume this is meant to be represented
    // as an eight-bit two's complement integer (`i8`).
    let value = ((63.0 * pan.value()) as i8).clamp(-63, 63) as u8;
    let value = value as u16;
    send_nrpn(param, value)
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
/// Pack the two given (8 bit) bytes lossily into two MIDI (7 bit) bytes
fn two_byte_midi_pack(input: u16) -> (u8, u8) {
    let lsb = input & 0b0111_1111;
    let msb = (input >> 7) & 0b0111_1111;
    (msb as u8, lsb as u8)
}

/// Round the provided fader value to the closest discrete value that the M7CL can store and return
/// its index in PRM TABLE #03.
fn fader_value_to_index(value: f32) -> Result<usize, ()> {
    // we have binary search at home
    let mut lower_bound = 0;
    let mut upper_bound = FADER_TABLE.len() - 1;
    // Guard against brokenness
    match value.classify() {
        std::num::FpCategory::Nan => return Err(()),
        std::num::FpCategory::Infinite => {
            if value.is_sign_negative() {
                return Ok(lower_bound);
            } else {
                return Ok(upper_bound);
            }
        }
        // Proceed to search
        _ => (),
    }
    // static upper loop bound
    for i in (0..FADER_TABLE.len()) {
        let search_ind = lower_bound + (upper_bound - lower_bound)/2;
        if value > FADER_TABLE[search_ind] {
            lower_bound = search_ind;
        } else {
            upper_bound = search_ind;
        }
        if upper_bound - lower_bound == 1 {
            break;
        }
    }
    let lower_diff = (value - FADER_TABLE[lower_bound]).abs();
    let upper_diff = (value - FADER_TABLE[upper_bound]).abs();
    if lower_diff > upper_diff {
        Ok(upper_bound)
    } else {
        Ok(lower_bound)
    }
}

/// The M7CL stores fader values internally using a number in [0, 1023] (ie discretely with 1024
/// steps). This is a map of table index to fader value. (PRM TABLE #03)
const FADER_TABLE: [f32; 1024] = [
    // Technically, this first value is -inf but for algorithm convenience it'll be this in our
    // table
    -141.00, -138.00, -135.00, -132.00, -129.00, -126.00, -123.00, -120.00, -117.00, -114.00,
    -111.00, -108.00, -105.00, -102.00, -99.00, -96.00, -95.00, -94.00, -93.00, -92.00, -91.00,
    -90.00, -89.00, -88.00, -87.00, -86.00, -85.00, -84.00, -83.00, -82.00, -81.00, -80.00, -79.00,
    -78.00, -77.80, -77.60, -77.40, -77.20, -77.00, -76.80, -76.60, -76.40, -76.20, -76.00, -75.80,
    -75.60, -75.40, -75.20, -75.00, -74.80, -74.60, -74.40, -74.20, -74.00, -73.80, -73.60, -73.40,
    -73.20, -73.00, -72.80, -72.60, -72.40, -72.20, -72.00, -71.80, -71.60, -71.40, -71.20, -71.00,
    -70.80, -70.60, -70.40, -70.20, -70.00, -69.80, -69.60, -69.40, -69.20, -69.00, -68.80, -68.60,
    -68.40, -68.20, -68.00, -67.80, -67.60, -67.40, -67.20, -67.00, -66.80, -66.60, -66.40, -66.20,
    -66.00, -65.80, -65.60, -65.40, -65.20, -65.00, -64.80, -64.60, -64.40, -64.20, -64.00, -63.80,
    -63.60, -63.40, -63.20, -63.00, -62.80, -62.60, -62.40, -62.20, -62.00, -61.80, -61.60, -61.40,
    -61.20, -61.00, -60.80, -60.60, -60.40, -60.20, -60.00, -59.80, -59.60, -59.40, -59.20, -59.00,
    -58.80, -58.60, -58.40, -58.20, -58.00, -57.80, -57.60, -57.40, -57.20, -57.00, -56.80, -56.60,
    -56.40, -56.20, -56.00, -55.80, -55.60, -55.40, -55.20, -55.00, -54.80, -54.60, -54.40, -54.20,
    -54.00, -53.80, -53.60, -53.40, -53.20, -53.00, -52.80, -52.60, -52.40, -52.20, -52.00, -51.80,
    -51.60, -51.40, -51.20, -51.00, -50.80, -50.60, -50.40, -50.20, -50.00, -49.80, -49.60, -49.40,
    -49.20, -49.00, -48.80, -48.60, -48.40, -48.20, -48.00, -47.80, -47.60, -47.40, -47.20, -47.00,
    -46.80, -46.60, -46.40, -46.20, -46.00, -45.80, -45.60, -45.40, -45.20, -45.00, -44.80, -44.60,
    -44.40, -44.20, -44.00, -43.80, -43.60, -43.40, -43.20, -43.00, -42.80, -42.60, -42.40, -42.20,
    -42.00, -41.80, -41.60, -41.40, -41.20, -41.00, -40.80, -40.60, -40.40, -40.20, -40.00, -39.90,
    -39.80, -39.70, -39.60, -39.50, -39.40, -39.30, -39.20, -39.10, -39.00, -38.90, -38.80, -38.70,
    -38.60, -38.50, -38.40, -38.30, -38.20, -38.10, -38.00, -37.90, -37.80, -37.70, -37.60, -37.50,
    -37.40, -37.30, -37.20, -37.10, -37.00, -36.90, -36.80, -36.70, -36.60, -36.50, -36.40, -36.30,
    -36.20, -36.10, -36.00, -35.90, -35.80, -35.70, -35.60, -35.50, -35.40, -35.30, -35.20, -35.10,
    -35.00, -34.90, -34.80, -34.70, -34.60, -34.50, -34.40, -34.30, -34.20, -34.10, -34.00, -33.90,
    -33.80, -33.70, -33.60, -33.50, -33.40, -33.30, -33.20, -33.10, -33.00, -32.90, -32.80, -32.70,
    -32.60, -32.50, -32.40, -32.30, -32.20, -32.10, -32.00, -31.90, -31.80, -31.70, -31.60, -31.50,
    -31.40, -31.30, -31.20, -31.10, -31.00, -30.90, -30.80, -30.70, -30.60, -30.50, -30.40, -30.30,
    -30.20, -30.10, -30.00, -29.90, -29.80, -29.70, -29.60, -29.50, -29.40, -29.30, -29.20, -29.10,
    -29.00, -28.90, -28.80, -28.70, -28.60, -28.50, -28.40, -28.30, -28.20, -28.10, -28.00, -27.90,
    -27.80, -27.70, -27.60, -27.50, -27.40, -27.30, -27.20, -27.10, -27.00, -26.90, -26.80, -26.70,
    -26.60, -26.50, -26.40, -26.30, -26.20, -26.10, -26.00, -25.90, -25.80, -25.70, -25.60, -25.50,
    -25.40, -25.30, -25.20, -25.10, -25.00, -24.90, -24.80, -24.70, -24.60, -24.50, -24.40, -24.30,
    -24.20, -24.10, -24.00, -23.90, -23.80, -23.70, -23.60, -23.50, -23.40, -23.30, -23.20, -23.10,
    -23.00, -22.90, -22.80, -22.70, -22.60, -22.50, -22.40, -22.30, -22.20, -22.10, -22.00, -21.90,
    -21.80, -21.70, -21.60, -21.50, -21.40, -21.30, -21.20, -21.10, -21.00, -20.90, -20.80, -20.70,
    -20.60, -20.50, -20.40, -20.30, -20.20, -20.10, -20.00, -19.95, -19.90, -19.85, -19.80, -19.75,
    -19.70, -19.65, -19.60, -19.55, -19.50, -19.45, -19.40, -19.35, -19.30, -19.25, -19.20, -19.15,
    -19.10, -19.05, -19.00, -18.95, -18.90, -18.85, -18.80, -18.75, -18.70, -18.65, -18.60, -18.55,
    -18.50, -18.45, -18.40, -18.35, -18.30, -18.25, -18.20, -18.15, -18.10, -18.05, -18.00, -17.95,
    -17.90, -17.85, -17.80, -17.75, -17.70, -17.65, -17.60, -17.55, -17.50, -17.45, -17.40, -17.35,
    -17.30, -17.25, -17.20, -17.15, -17.10, -17.05, -17.00, -16.95, -16.90, -16.85, -16.80, -16.75,
    -16.70, -16.65, -16.60, -16.55, -16.50, -16.45, -16.40, -16.35, -16.30, -16.25, -16.20, -16.15,
    -16.10, -16.05, -16.00, -15.95, -15.90, -15.85, -15.80, -15.75, -15.70, -15.65, -15.60, -15.55,
    -15.50, -15.45, -15.40, -15.35, -15.30, -15.25, -15.20, -15.15, -15.10, -15.05, -15.00, -14.95,
    -14.90, -14.85, -14.80, -14.75, -14.70, -14.65, -14.60, -14.55, -14.50, -14.45, -14.40, -14.35,
    -14.30, -14.25, -14.20, -14.15, -14.10, -14.05, -14.00, -13.95, -13.90, -13.85, -13.80, -13.75,
    -13.70, -13.65, -13.60, -13.55, -13.50, -13.45, -13.40, -13.35, -13.30, -13.25, -13.20, -13.15,
    -13.10, -13.05, -13.00, -12.95, -12.90, -12.85, -12.80, -12.75, -12.70, -12.65, -12.60, -12.55,
    -12.50, -12.45, -12.40, -12.35, -12.30, -12.25, -12.20, -12.15, -12.10, -12.05, -12.00, -11.95,
    -11.90, -11.85, -11.80, -11.75, -11.70, -11.65, -11.60, -11.55, -11.50, -11.45, -11.40, -11.35,
    -11.30, -11.25, -11.20, -11.15, -11.10, -11.05, -11.00, -10.95, -10.90, -10.85, -10.80, -10.75,
    -10.70, -10.65, -10.60, -10.55, -10.50, -10.45, -10.40, -10.35, -10.30, -10.25, -10.20, -10.15,
    -10.10, -10.05, -10.00, -9.95, -9.90, -9.85, -9.80, -9.75, -9.70, -9.65, -9.60, -9.55, -9.50,
    -9.45, -9.40, -9.35, -9.30, -9.25, -9.20, -9.15, -9.10, -9.05, -9.00, -8.95, -8.90, -8.85,
    -8.80, -8.75, -8.70, -8.65, -8.60, -8.55, -8.50, -8.45, -8.40, -8.35, -8.30, -8.25, -8.20,
    -8.15, -8.10, -8.05, -8.00, -7.95, -7.90, -7.85, -7.80, -7.75, -7.70, -7.65, -7.60, -7.55,
    -7.50, -7.45, -7.40, -7.35, -7.30, -7.25, -7.20, -7.15, -7.10, -7.05, -7.00, -6.95, -6.90,
    -6.85, -6.80, -6.75, -6.70, -6.65, -6.60, -6.55, -6.50, -6.45, -6.40, -6.35, -6.30, -6.25,
    -6.20, -6.15, -6.10, -6.05, -6.00, -5.95, -5.90, -5.85, -5.80, -5.75, -5.70, -5.65, -5.60,
    -5.55, -5.50, -5.45, -5.40, -5.35, -5.30, -5.25, -5.20, -5.15, -5.10, -5.05, -5.00, -4.95,
    -4.90, -4.85, -4.80, -4.75, -4.70, -4.65, -4.60, -4.55, -4.50, -4.45, -4.40, -4.35, -4.30,
    -4.25, -4.20, -4.15, -4.10, -4.05, -4.00, -3.95, -3.90, -3.85, -3.80, -3.75, -3.70, -3.65,
    -3.60, -3.55, -3.50, -3.45, -3.40, -3.35, -3.30, -3.25, -3.20, -3.15, -3.10, -3.05, -3.00,
    -2.95, -2.90, -2.85, -2.80, -2.75, -2.70, -2.65, -2.60, -2.55, -2.50, -2.45, -2.40, -2.35,
    -2.30, -2.25, -2.20, -2.15, -2.10, -2.05, -2.00, -1.95, -1.90, -1.85, -1.80, -1.75, -1.70,
    -1.65, -1.60, -1.55, -1.50, -1.45, -1.40, -1.35, -1.30, -1.25, -1.20, -1.15, -1.10, -1.05,
    -1.00, -0.95, -0.90, -0.85, -0.80, -0.75, -0.70, -0.65, -0.60, -0.55, -0.50, -0.45, -0.40,
    -0.35, -0.30, -0.25, -0.20, -0.15, -0.10, -0.05, 0.00, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30,
    0.35, 0.40, 0.45, 0.50, 0.55, 0.60, 0.65, 0.70, 0.75, 0.80, 0.85, 0.90, 0.95, 1.00, 1.05, 1.10,
    1.15, 1.20, 1.25, 1.30, 1.35, 1.40, 1.45, 1.50, 1.55, 1.60, 1.65, 1.70, 1.75, 1.80, 1.85, 1.90,
    1.95, 2.00, 2.05, 2.10, 2.15, 2.20, 2.25, 2.30, 2.35, 2.40, 2.45, 2.50, 2.55, 2.60, 2.65, 2.70,
    2.75, 2.80, 2.85, 2.90, 2.95, 3.00, 3.05, 3.10, 3.15, 3.20, 3.25, 3.30, 3.35, 3.40, 3.45, 3.50,
    3.55, 3.60, 3.65, 3.70, 3.75, 3.80, 3.85, 3.90, 3.95, 4.00, 4.05, 4.10, 4.15, 4.20, 4.25, 4.30,
    4.35, 4.40, 4.45, 4.50, 4.55, 4.60, 4.65, 4.70, 4.75, 4.80, 4.85, 4.90, 4.95, 5.00, 5.05, 5.10,
    5.15, 5.20, 5.25, 5.30, 5.35, 5.40, 5.45, 5.50, 5.55, 5.60, 5.65, 5.70, 5.75, 5.80, 5.85, 5.90,
    5.95, 6.00, 6.05, 6.10, 6.15, 6.20, 6.25, 6.30, 6.35, 6.40, 6.45, 6.50, 6.55, 6.60, 6.65, 6.70,
    6.75, 6.80, 6.85, 6.90, 6.95, 7.00, 7.05, 7.10, 7.15, 7.20, 7.25, 7.30, 7.35, 7.40, 7.45, 7.50,
    7.55, 7.60, 7.65, 7.70, 7.75, 7.80, 7.85, 7.90, 7.95, 8.00, 8.05, 8.10, 8.15, 8.20, 8.25, 8.30,
    8.35, 8.40, 8.45, 8.50, 8.55, 8.60, 8.65, 8.70, 8.75, 8.80, 8.85, 8.90, 8.95, 9.00, 9.05, 9.10,
    9.15, 9.20, 9.25, 9.30, 9.35, 9.40, 9.45, 9.50, 9.55, 9.60, 9.65, 9.70, 9.75, 9.80, 9.85, 9.90,
    9.95, 10.00,
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_fader_value_to_index() {
        assert_eq!(fader_value_to_index(f32::NEG_INFINITY).unwrap(), 0);
        assert_eq!(fader_value_to_index(f32::INFINITY).unwrap(), 1023);
        assert_eq!(fader_value_to_index(-138.0).unwrap(), 1);
        assert_eq!(fader_value_to_index(-138.1).unwrap(), 1);
        assert_eq!(fader_value_to_index(-137.9).unwrap(), 1);
        assert_eq!(fader_value_to_index(0.0).unwrap(), 823);
        assert_eq!(fader_value_to_index(-29.7).unwrap(), 326);
        assert_eq!(fader_value_to_index(-5.0).unwrap(), 723);
        assert_eq!(fader_value_to_index(-5.005).unwrap(), 723);
        assert_eq!(fader_value_to_index(-4.95).unwrap(), 724);
        assert_eq!(fader_value_to_index(-10.0).unwrap(), 623);
    }
}
