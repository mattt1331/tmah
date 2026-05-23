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
    ui_screen: ui::UiScreen,
    cues_popup: ui::CuesPopup,
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

            ui_screen: ui::UiScreen::default(),
            cues_popup: ui::CuesPopup::default(),
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
    pub fn num_channels(&self) -> u8 {
        self.connection.num_channels()
    }
    pub fn cues(&self) -> &Vec<Cue> {
        &self.cues
    }
    pub fn cues_mut(&mut self) -> &mut Vec<Cue> {
        &mut self.cues
    }
    pub fn channel_names(&self) -> &ChannelNames {
        &self.ch_names
    }
    pub fn channel_names_mut(&mut self) -> &mut ChannelNames {
        &mut self.ch_names
    }
    pub fn open_cues_popup(&mut self, popup: ui::CuesPopup) {
        self.cues_popup = popup;
    }
    pub fn clear_cues_popup(&mut self) {
        self.cues_popup = ui::CuesPopup::None;
    }
    pub fn connection(&self) -> &Box<dyn board::Connectable> {
        &self.connection
    }
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
    fn dcas_mut(&mut self) -> &mut Vec<DcaState> {
        &mut self.dcas
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
                    if ch_name != "" {
                        ch_name
                    } else {
                        format!("Ch {}", ch.number())
                    }
                } else {
                    format!("Ch {}", ch.number())
                };
                if name == "" {
                    name = ch_name;
                } else {
                    name = format!("{name}, {}", ch_name)
                }
            }
            format!("{name}")
        }
    }
    fn edit_name(&mut self) -> &mut Option<String> {
        &mut self.name
    }
    fn assigned(&self) -> &Vec<Channel> {
        &self.assigned
    }
    /// Assigns the given channel to this DCA
    fn assign(&mut self, ch: Channel) {
        if !self.assigned.contains(&ch) {
            self.assigned.push(ch);
        }
    }
    /// Unassigns the given channel from this DCA
    fn unassign(&mut self, ch: Channel) {
        let mut ind = None;
        for (i, channel) in self.assigned.iter().enumerate() {
            if *channel == ch {
                ind = Some(i);
            }
        }
        if let Some(ind) = ind {
            // `swap_remove` for performance (absolutely crucial here)
            self.assigned.swap_remove(ind);
        }
    }
}

/// Contains the names of each channel
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct ChannelNames {
    // Keys are the index of the channel
    names: std::collections::HashMap<u8, String>,
}
impl ChannelNames {
    /// Set the name of the given channel
    fn set_name(&mut self, ch: &Channel, name: String) {
        self.names.insert(ch.index(), name);
    }
    /// Returns a mutable reference to the given channel's name
    fn edit_name(&mut self, ch: &Channel) -> &mut String {
        self.names.entry(ch.index()).or_insert("".to_string())
    }
    /// Returns the name of the channel, if set
    fn get_name(&self, ch: &Channel) -> Option<String> {
        self.names.get(&ch.index()).map(|name| format!("{name}"))
    }
}
