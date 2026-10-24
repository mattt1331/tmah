//! Contains types used by `mod board` to represent data common to all boards

/// Represents a channel
// FIXME: Make Channel and Dca be Copy, not Clone
#[derive(
    Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Channel {
    index: u8,
}

impl Channel {
    /// Returns a `Channel` with the specified index (eg `ind` = 0 -> Ch1)
    pub fn from_index(ind: u8) -> Self {
        Channel { index: ind }
    }
    #[allow(dead_code)] // for completeness
    /// Returns a `Channel` with the specified number (eg `num` = 1 -> Ch1). Returns None if `num` is
    /// zero.
    pub fn from_number(num: u8) -> Option<Channel> {
        if num == 0 {
            None
        } else {
            Some(Channel { index: num - 1 })
        }
    }
    /// Returns the zero-indexed index of the channel (eg Ch1 returns 0)
    pub fn index(&self) -> u8 {
        self.index
    }
    /// Returns the number of the channel (eg Ch1 returns 1)
    pub fn number(&self) -> u8 {
        self.index + 1
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
/// Represents a DCA
pub struct Dca {
    index: u8,
}

impl Dca {
    /// Returns a `Dca` with the specified index (eg `ind` = 0 -> DCA 1)
    pub fn from_index(ind: u8) -> Self {
        Dca { index: ind }
    }
    #[allow(dead_code)] // for completeness
    /// Returns a `Channel` with the specified number (eg `num` = 1 -> DCA 1). Returns None if `num` is
    /// zero.
    pub fn from_number(num: u8) -> Option<Dca> {
        if num == 0 {
            None
        } else {
            Some(Dca { index: num - 1 })
        }
    }
    /// Returns the zero-indexed index of the DCA (eg DCA 1 returns 0)
    pub fn index(&self) -> u8 {
        self.index
    }
    #[allow(dead_code)] // for completeness
    /// Returns the number of the DCA (eg DCA 1 returns 1)
    pub fn number(&self) -> u8 {
        self.index + 1
    }
}

/// A channel's pan value, as a float from -1.0 to 1.0, where -1.0 is all the way left and 1.0 is
/// all the way right.
/// The pan value is stored as an `f32`. The infinities are clamped, and NaN is treated as
/// zero.
#[derive(Clone, Copy, Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pan {
    value: f32,
}

impl Pan {
    /// Take the provided value and transform it to be valid and in the range [-1.0, 1.0].
    fn normalize(input: f32) -> f32 {
        use std::num::FpCategory;

        match input.classify() {
            FpCategory::Zero => input,
            FpCategory::Normal | FpCategory::Subnormal | FpCategory::Infinite => {
                input.clamp(-1.0, 1.0)
            }
            FpCategory::Nan => 0.0,
        }
    }
    pub fn new(value: f32) -> Self {
        Pan {
            value: Self::normalize(value),
        }
    }
    pub fn value(&self) -> f32 {
        Self::normalize(self.value)
    }
    pub fn set_value(&mut self, value: f32) {
        self.value = Self::normalize(value);
    }
}
impl PartialEq for Pan {
    fn eq(&self, other: &Self) -> bool {
        // Ensure that eg a Pan with value NaN and a pan with value 0.0 are equal
        self.value() == other.value()
    }
}
impl Eq for Pan {}

#[derive(Clone, Debug, PartialEq)]
/// Represents all supported board edit operations.
pub enum BoardEdit {
    /// Edit to a channel's mute. If true, the channel is now muted.
    ChannelMute(Channel, bool),
    /// Edit to a channel's DCA assignment. If true, the channel is now assigned to the DCA.
    ChannelDcaAssign(Channel, Dca, bool),
    /// Edit to a channel's name. The name is now set to the given value.
    ChannelName(Channel, String),
    /// Edit to a channel's pan value. The pan is now set to the given value.
    ChannelPan(Channel, Pan),
    /// Edit to a DCA's level. The level is now set to the given value.
    DcaLevel(Dca, Decibels),
    /// Edit to a DCA's name. The name is now set to the given value.
    DcaName(Dca, String),
}

pub type Decibels = f32;
