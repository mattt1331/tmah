//! Contains the program state and UI

mod board;
mod ui;

use board::DcaAssignable;
use crate::dB;

/// Top level of program state
pub struct State {
    // Actual program state
    num_dcas: u8,
    cues: Vec<Cue>,

    // UI state etc
    ui_screen: UiScreen,
}

impl Default for State {
    fn default() -> Self {
        State {
            num_dcas: 100,
            cues: vec![Cue::default(), Cue::default(), Cue::default()],

            ui_screen: UiScreen::default(),
        }
    }
}

impl State {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        Self::default()
    }
}

/// The different screens of the ui, like the cues, board connection, etc
#[derive(Default, PartialEq)]
enum UiScreen {
    #[default]
    Cues,
    File,
    Board,
}

/// One singular cue aka scene
struct Cue {
    name: String,
    dcas: Vec<DcaState>,
}

impl Default for Cue {
    fn default() -> Self {
        Cue {
            name: "def cue name".to_string(),
            dcas: vec![DcaState::default(), DcaState::default(), DcaState::default(), DcaState::default(), DcaState::default(), DcaState::default(), DcaState::default(), DcaState::default()],
        }
    }
}

/// The state of a DCA, which can be realized by calling a cue
struct DcaState {
    assigned: Vec<Box<dyn DcaAssignable>>,
    level: Option<dB>,
}

impl Default for DcaState {
    fn default() -> Self {
        DcaState {
            assigned: Vec::new(),
            level: None,
        }
    }
}

impl DcaState {
    fn name(&self) -> String {
        "DCA names not impl".to_string()
    }
}
