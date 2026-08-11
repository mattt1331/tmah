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

#[derive(Clone, Debug, PartialEq)]
/// Represents all supported board edit operations.
pub enum BoardEdit {
    /// Edit to a channel's mute. If true, the channel is now muted.
    ChannelMute(Channel, bool),
    /// Edit to a channel's DCA assignment. If true, the channel is now assigned to the DCA.
    ChannelDcaAssign(Channel, Dca, bool),
    /// Edit to a channel's name. The name is now set to the given value.
    ChannelName(Channel, String),
    /// Edit to a DCA's level. The level is now set to the given value.
    DcaLevel(Dca, Decibels),
    /// Edit to a DCA's name. The name is now set to the given value.
    DcaName(Dca, String),
}

pub type Decibels = f32;
