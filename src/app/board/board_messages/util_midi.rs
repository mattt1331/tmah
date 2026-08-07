//! Types and utilities for representing MIDI messages

/// Represents a subset of MIDI messages which is useful for interacting with the boards
#[derive(Debug, PartialEq)]
pub enum MidiMessage {
    NrpnParameterMSB(U7),
    NrpnParameterLSB(U7),
    NrpnDataMSB(U7),
    NrpnDataLSB(U7),
    /// The data does not contain the leading byte and trailing byte (F0 and F7)
    Sysex(Vec<u8>),
}

impl MidiMessage {
    pub fn to_bytes(self, channel: MidiChannel) -> Vec<u8> {
        match self {
            MidiMessage::NrpnParameterMSB(param_msb) => {
                vec![
                    // Control change + channel
                    0b1011_0000 | channel as u8,
                    // NRPN Parameter MSB
                    0x63,
                    // NRPN Parameter MSB value
                    param_msb.into_u8(),
                ]
            }
            MidiMessage::NrpnParameterLSB(param_lsb) => {
                vec![
                    // Control change + channel
                    0b1011_0000 | channel as u8,
                    // NRPN Parameter LSB
                    0x62,
                    // NRPN Parameter LSB value
                    param_lsb.into_u8(),
                ]
            }
            MidiMessage::NrpnDataMSB(data_msb) => {
                vec![
                    // Control change + channel
                    0b1011_0000 | channel as u8,
                    // NRPN Data MSB
                    0x06,
                    // NRPN Data MSB value
                    data_msb.into_u8(),
                ]
            }
            MidiMessage::NrpnDataLSB(data_lsb) => {
                vec![
                    // Control change + channel
                    0b1011_0000 | channel as u8,
                    // NRPN Data LSB
                    0x26,
                    // NRPN Data LSB value
                    data_lsb.into_u8(),
                ]
            }
            MidiMessage::Sysex(mut data) => {
                data.insert(0, 0xf0);
                data.push(0xf7);
                data
            }
        }
    }
    pub fn from_bytes(bytes: &[u8]) -> Option<(Option<MidiChannel>, MidiMessage)> {
        if bytes.len() == 0 {
            return None;
        }
        match bytes[0] {
            0b1111_0000 => {
                if bytes[bytes.len() - 1] != 0b1111_0111 {
                    log::error!("Tried to parse sysex which is missing the end byte: {:#?}", bytes);
                    return None;
                }
                Some((None, MidiMessage::Sysex(bytes[1..bytes.len()-1].iter().copied().collect())))
            }
            byte => match byte & 0b1111_0000 {
                0b1011_0000 => {
                    // Control Change
                    if bytes.len() < 3 {
                        log::error!("Tried to parse a control change but it is too short: {:#?}", bytes);
                        return None;
                    }
                    let ch = byte & 0b0000_1111;
                    let ch: MidiChannel = ch.into();
                    match bytes[1] {
                        0x63 => Some((Some(ch), MidiMessage::NrpnParameterMSB(bytes[2].into()))), // NRPN param MSB
                        0x62 => Some((Some(ch), MidiMessage::NrpnParameterLSB(bytes[2].into()))), // NRPN param LSB
                        0x06 => Some((Some(ch), MidiMessage::NrpnDataMSB(bytes[2].into()))), // NRPN data MSB
                        0x26 => Some((Some(ch), MidiMessage::NrpnDataLSB(bytes[2].into()))), // NRPN data LSB
                        // Other messages we don't care about
                        _ => None,
                    }
                }
                // Other message we don't care about
                _ => None,
            }
        }
    }
}

/// A wrapper around `u8` which ensures that the topmost bit is always zero.
#[derive(Clone, Debug, PartialEq)]
pub struct U7(u8);
// The `u8` stored in the struct must have the top bit be zero.
impl U7 {
    pub fn from_u8(input: u8) -> Self {
        U7(input & 0b0111_1111)
    }
    pub fn into_u8(self) -> u8 {
        self.0
    }
}
impl From<U7> for u8 {
    fn from(value: U7) -> u8 {
        value.into_u8()
    }
}
impl From<u8> for U7 {
    fn from(value: u8) -> U7 {
        U7::from_u8(value)
    }
}

/// One of the sixteen MIDI channels
pub enum MidiChannel {
    Channel1 = 0x0,
    Channel2 = 0x1,
    Channel3 = 0x2,
    Channel4 = 0x3,
    Channel5 = 0x4,
    Channel6 = 0x5,
    Channel7 = 0x6,
    Channel8 = 0x7,
    Channel9 = 0x8,
    Channel10 = 0x9,
    Channel11 = 0xa,
    Channel12 = 0xb,
    Channel13 = 0xc,
    Channel14 = 0xd,
    Channel15 = 0xe,
    Channel16 = 0xf,
}
impl From<u8> for MidiChannel {
    fn from(value: u8) -> MidiChannel {
        use MidiChannel as M;
        match value {
            0 => M::Channel1,
            1 => M::Channel2,
            2 => M::Channel3,
            3 => M::Channel4,
            4 => M::Channel5,
            5 => M::Channel6,
            6 => M::Channel7,
            7 => M::Channel8,
            8 => M::Channel9,
            9 => M::Channel10,
            10 => M::Channel11,
            11 => M::Channel12,
            12 => M::Channel13,
            13 => M::Channel14,
            14 => M::Channel15,
            _ => M::Channel16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sysex() {
        let Some((_, msg)) = MidiMessage::from_bytes(&[0xf0, 0x11, 0x11, 0x11, 0xf7]) else { panic!("Did not parse sysex")};
        assert_eq!(msg, MidiMessage::Sysex(vec![0x11, 0x11, 0x11]));
    }
    #[test]
    fn parse_nrpn_1() {
        let Some((_, msg)) = MidiMessage::from_bytes(&[0b10110000, 0x63, 0x28]) else {panic!("Did not parse message")};
        assert_eq!(msg, MidiMessage::NrpnParameterMSB(0x28.into()));
    }
    fn parse_nrpn_2() {
        let Some((_, msg)) = MidiMessage::from_bytes(&[0b10110000, 0x62, 0x28]) else {panic!("Did not parse message")};
        assert_eq!(msg, MidiMessage::NrpnParameterLSB(0x28.into()));
    }
    fn parse_nrpn_3() {
        let Some((_, msg)) = MidiMessage::from_bytes(&[0b10110000, 0x06, 0x28]) else {panic!("Did not parse message")};
        assert_eq!(msg, MidiMessage::NrpnDataMSB(0x28.into()));
    }
    fn parse_nrpn_4() {
        let Some((_, msg)) = MidiMessage::from_bytes(&[0b10110000, 0x12, 0x28]) else {panic!("Did not parse message")};
        assert_eq!(msg, MidiMessage::NrpnDataLSB(0x28.into()));
    }
}
