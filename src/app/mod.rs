//! Contains the program state and UI

mod cues;
mod board;
mod file;
mod ui;

use board::Channel;
pub use board::Decibels;

use std::collections::{BTreeMap, BTreeSet};

/// Top level of program state
// TODO: Rename/reorganize all of these elements to have the correct prefixes (cues_, file_, board_)
pub struct State {
    ui_screen: ui::UiScreen,


    // Cues
    cues: BTreeMap<CueNumber, Cue>,
    cues_ch_names: ChannelNames,
    /// What kind of edit are we in-progress of? eg DCA assignments, cue names, etc
    cues_edit_action: ui::CuesEditAction,
    cues_selected_cue_ind: Option<usize>,
    // FIXME: Refactor this atrocity.
    /// An objectively terrible implementation, true, but it's funny. A stack of undo actions. When
    /// undo is pressed, pop off the last one and run it on `State`. When an undoable action
    /// occurs, add the undo action to the stack.
    cues_undo_stack: Vec<Box<UndoAction>>,


    // Board
    board_connection: Box<dyn board::Connectable>,
    /// The connection selected in the dropdown on the board screen
    board_connection_ui: board::Connections,
    /// The currently active connection to difference with above
    board_connection_ui_prev: board::Connections,


    // File
    file_state: file::FileState,
}

type UndoAction = dyn FnOnce(&mut State);

impl Default for State {
    fn default() -> Self {
        State {
            cues: BTreeMap::new(),
            cues_ch_names: ChannelNames::default(),
            board_connection: Box::new(board::NoConnection::new()),

            file_state: file::FileState::default(),

            ui_screen: ui::UiScreen::default(),
            cues_edit_action: ui::CuesEditAction::default(),
            cues_selected_cue_ind: None,
            cues_undo_stack: Vec::default(),
            board_connection_ui: board::Connections::default(),
            board_connection_ui_prev: board::Connections::default(),
        }
    }
}

/// Basic getters and setters
impl State {
    pub fn num_dcas(&self) -> u8 {
        self.board_connection.num_dcas()
    }
    pub fn num_channels(&self) -> u8 {
        self.board_connection.num_channels()
    }
    pub fn cues(&self) -> &BTreeMap<CueNumber, Cue> {
        &self.cues
    }
    pub fn cues_mut(&mut self) -> &mut BTreeMap<CueNumber, Cue> {
        &mut self.cues
    }
    pub fn cues_channel_names(&self) -> &ChannelNames {
        &self.cues_ch_names
    }
    pub fn cues_channel_names_mut(&mut self) -> &mut ChannelNames {
        &mut self.cues_ch_names
    }
    pub fn cues_edit_action(&self) -> &ui::CuesEditAction {
        &self.cues_edit_action
    }
    pub fn cues_edit_action_mut(&mut self) -> &mut ui::CuesEditAction {
        &mut self.cues_edit_action
    }
    pub fn cues_selected_cue(&self) -> Option<usize> {
        self.cues_selected_cue_ind
    }
    pub fn board_connection(&self) -> &Box<dyn board::Connectable> {
        &self.board_connection
    }
}
/// Methods with additional logic
impl State {
    /// The key for eframe's persistent storage where we will autosave the file to
    const PERSISTANT_STORAGE_KEY: &str = "miq_v2.file_data";
    /// Create a new `State` and load any persisted values
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut out = Self::default();
        if let Some(storage) = cc.storage
            && let Some(persist_data) = storage.get_string(Self::PERSISTANT_STORAGE_KEY)
        {
            match file::FileData::deserialize(persist_data.as_bytes()) {
                Ok(file_data) => out.file_load_data(file_data),
                Err(err) => log::error!("Failed to load persisted data: {err}"),
            }
        }
        out
    }
    /// Should get called every frame.
    pub fn each_frame(&mut self) {
        // Each connection is allowed to do some work each frame
        self.board_connection.heartbeat();
    }
    /// Fires the selected cue.
    pub fn fire_selected_cue(&mut self) {
        if let Some(ind) = self.cues_selected_cue_ind {
            if let Some(cue) = self.cues().values().nth(ind) {
                // FIX: There has to be a better way to do this. Maybe switch to selected cue
                // number, not index?
                let cue = cue.clone();
                self.board_connection.fire_cue(&cue);
            } else {
                log::error!(
                    "Fire cue function called but the selected cue at index {ind} could not be found"
                );
            }
        }
    }
    /// Increments selection index and fires the newly selected cue.
    pub fn fire_next_cue(&mut self) {
        if let Some(ind) = self.cues_selected_cue_ind {
            self.cues_set_selected(Some(ind + 1));
            self.fire_selected_cue();
        } else {
            self.cues_set_selected(Some(0));
            self.fire_selected_cue();
        }
    }
    /// Sends the channel names to the board.
    pub fn fire_channel_names(&mut self) {
        self.board_connection.fire_channel_names(&self.cues_ch_names);
    }
}

#[derive(
    Clone, Default, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize, serde::Deserialize,
)]
/// Represents a cue's number. Cues can optionally be nested up to three levels deep. However, the
/// second two levels are optional. Logically, if the third level is present the second one must be
/// as well. Note that the numbers should be displayed as one plus their value. Thus, (0, None) is
/// cue 1, and (0, Some((0, None))) is cue 1.1.
pub struct CueNumber(usize, Option<(usize, Option<usize>)>);
impl CueNumber {
    /// Try to parse a `CueNumber` from an `&str` inputted by the user
    fn parse(input: &str) -> Result<CueNumber, ()> {
        let mut input = input
            .split(['.', '-', ' '])
            .filter_map(|num| num.parse().ok());
        match input.next() {
            Some(first_num) => match input.next() {
                Some(second_num) => Ok(CueNumber(first_num, Some((second_num, input.next())))),
                None => Ok(CueNumber(first_num, None)),
            },
            None => Err(()),
        }
    }
    /// Returns the number after this one, incrementing the lowest level of numbers that has been
    /// set
    fn increment_lowest(&mut self) {
        if let Some((ref mut b, mut c)) = self.1 {
            if let Some(ref mut c) = c {
                *c += 1;
            } else {
                *b += 1;
            }
        } else {
            self.0 += 1;
        }
    }
}
impl std::fmt::Display for CueNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self.1 {
            Some((b, c)) => match c {
                Some(c) => write!(f, "{}.{}.{}", self.0, b, c),
                None => write!(f, "{}.{}", self.0, b),
            },
            None => write!(f, "{}", self.0),
        }?;
        Ok(())
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
            name: "".to_string(),
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
    fn edit_name(&mut self) -> &mut String {
        &mut self.name
    }
    fn dcas(&self) -> &Vec<DcaState> {
        &self.dcas
    }
    fn dcas_mut(&mut self) -> &mut Vec<DcaState> {
        &mut self.dcas
    }
}

/// The state of a DCA, which can be realized by calling a cue
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct DcaState {
    assigned: BTreeSet<Channel>,
    name: Option<String>,
    level: Option<Decibels>,
}

impl DcaState {
    fn name(&self, ch_names: &ChannelNames) -> String {
        if let Some(name) = &self.name {
            name.clone()
        } else {
            let mut name = "".to_string();
            for ch in &self.assigned {
                let ch_name = if let Some(ch_name) = ch_names.get_name(ch) {
                    if !ch_name.is_empty() {
                        ch_name
                    } else {
                        format!("Ch {}", ch.number())
                    }
                } else {
                    format!("Ch {}", ch.number())
                };
                if name.is_empty() {
                    name = ch_name;
                } else {
                    name = format!("{name}, {}", ch_name);
                }
            }
            name.to_string()
        }
    }
    fn edit_name(&mut self) -> &mut Option<String> {
        &mut self.name
    }
    fn level(&self) -> &Option<Decibels> {
        &self.level
    }
    fn assigned(&self) -> &BTreeSet<Channel> {
        &self.assigned
    }
    fn set_assigned(&mut self, assignment: BTreeSet<Channel>) {
        self.assigned = assignment;
    }
    /// Assigns the given channel to this DCA
    fn assign(&mut self, ch: Channel) {
        self.assigned.insert(ch);
    }
    /// Unassigns the given channel from this DCA
    fn unassign(&mut self, ch: Channel) {
        self.assigned.remove(&ch);
    }
}

/// Contains the names of each channel
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChannelNames {
    // Keys are the index of the channel
    names: std::collections::HashMap<u8, String>,
}
impl ChannelNames {
    /// Returns a mutable reference to the given channel's name
    fn edit_name(&mut self, ch: &Channel) -> &mut String {
        self.names.entry(ch.index()).or_insert("".to_string())
    }
    /// Returns the name of the channel, if set
    fn get_name(&self, ch: &Channel) -> Option<String> {
        self.names.get(&ch.index()).map(|name| name.to_string())
    }
    /// Returns an iterator over (ch_ind, name) for the channels whose names are set.
    fn iterator(&self) -> std::collections::hash_map::Iter<'_, u8, String> {
        self.names.iter()
    }
}
