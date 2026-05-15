//! Contains types representing the program state

mod board;

use board::DcaAssignable;
use crate::dB;

/// Top level of program state
struct State {
    cues: Vec<Cue>,
}

/// One singular cue aka scene
struct Cue {
    dcas: Vec<DcaState>,
}

/// The state of a DCA, which can be realized by calling a cue
struct DcaState {
    assigned: Vec<Box<dyn DcaAssignable>>,
    level: Option<dB>,
}
