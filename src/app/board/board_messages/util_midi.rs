//! Types and utilities for representing MIDI messages

/// Represents a subset of MIDI messages which is useful for interacting with the boards
pub enum MidiMessage {
    NrpnParameterMSB(U7),
    NrpnParameterLSB(U7),
    NrpnDataMSB(U7),
    NrpnDataLSB(U7),
    /// The data does not contain the leading byte and trailing byte (F0 and F7)
    Sysex(Vec<u8>)
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
        todo!()
    }
}

/// A wrapper around `u8` which ensures that the topmost bit is always zero.
#[derive(Clone)]
pub struct U7(u8);
// The `u8` stored in the struct need not have the top bit be zero, but ensure that if you are
// getting a u8 out of it, the u8 must have the top bit be zero.
impl U7 {
    pub fn from_u8(input: u8) -> Self {
        U7(input)
    }
    pub fn into_u8(self) -> u8 {
        self.0 & 0b0111_1111
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
