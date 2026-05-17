//! Contains interfaces implemented for each board

/// Represents a single channel-like object (eg channel, bus, DCA, etc)
pub trait ChannelLike {}

/// Represents a single channel-like object which can be assigned to a DCA
pub trait DcaAssignable: ChannelLike {}
