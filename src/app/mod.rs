//! Contains the program state and UI

mod board;
mod file;
mod ui;

use crate::dB;
use board::{Channel, Connectable};

/// Top level of program state
pub struct State {
    // Actual program state
    cues: Vec<Cue>,
    ch_names: ChannelNames,
    connection: Box<dyn board::Connectable>,

    // UI state etc
    ui_screen: UiScreen,
    /// The connection selected in the dropdown on the board screen
    connection_ui: board::Connections,
    /// The currently active connection to difference with above
    connection_ui_prev: board::Connections,
}

impl Default for State {
    fn default() -> Self {
        State {
            cues: vec![
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
                Cue::default(),
            ],
            ch_names: ChannelNames::default(),
            connection: Box::new(board::NoConnection::new()),

            ui_screen: UiScreen::default(),
            connection_ui: board::Connections::default(),
            connection_ui_prev: board::Connections::default(),
        }
    }
}

impl State {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        Self::default()
    }
    pub fn num_dcas(&self) -> u8 {
        self.connection.num_dcas()
    }
    pub fn cues(&self) -> &Vec<Cue> {
        &self.cues
    }
    pub fn channel_names(&self) -> &ChannelNames {
        &self.ch_names
    }
    pub fn connection(&self) -> &Box<dyn board::Connectable> {
        &self.connection
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
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Cue {
    name: String,
    dcas: Vec<DcaState>,
}

impl Default for Cue {
    fn default() -> Self {
        Cue {
            name: "def cue name".to_string(),
            dcas: vec![
                DcaState::default(),
                DcaState::default(),
                DcaState::default(),
                DcaState::default(),
                DcaState::default(),
                DcaState::default(),
                DcaState::default(),
                DcaState::default(),
            ],
        }
    }
}

impl Cue {
    fn name(&self) -> &str {
        &self.name
    }
    fn dcas(&self) -> &Vec<DcaState> {
        &self.dcas
    }
    /// Takes this cue and has the provided `top` cue override any parameters that `top` sets.
    /// Currently this just returns `top` but when EQs and crap get added this will make much more
    /// sense as Cue is currently used to record the state of the board (M7CL). This could also be
    /// terrible but idk
    fn superimpose(&self, top: &Cue) -> Cue {
        top.clone()
    }
}

/// The state of a DCA, which can be realized by calling a cue
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct DcaState {
    assigned: Vec<Channel>,
    name: Option<String>,
    level: Option<dB>,
}

impl DcaState {
    fn name(&self, ch_names: &ChannelNames) -> String {
        if let Some(name) = &self.name {
            name.clone()
        } else {
            let mut name = "".to_string();
            for ch in &self.assigned {
                let ch_name = if let Some(ch_name) = ch_names.get_name(&ch) {
                    ch_name
                } else {
                    format!("Ch {}", ch.number())
                };
                name = format!("{name}, {}", ch_name)
            }
            format!("{name}")
        }
    }
    fn assigned(&self) -> &Vec<Channel> {
        &self.assigned
    }
}

/// Contains the names of each channel
#[derive(Default)]
struct ChannelNames {
    // Keys are the index of the channel
    names: std::collections::HashMap<u8, String>,
}
impl ChannelNames {
    fn set_name(&mut self, ch: &Channel, name: String) {
        self.names.insert(ch.index(), name);
    }
    fn get_name(&self, ch: &Channel) -> Option<String> {
        self.names.get(&ch.index()).map(|name| format!("{name}"))
    }
}
