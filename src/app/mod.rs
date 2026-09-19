//! Contains the program state and UI

mod board;
mod cues;
mod file;
mod ui;

use board::Channel;
pub use board::Decibels;
pub use cues::{ChannelNames, Cue, CueNumber, CuesData, CuesEditAction, DcaState};

use std::collections::{BTreeMap, BTreeSet};

/// Top level of program state
pub struct State {
    ui_screen: ui::UiScreen,

    // Cues
    cues_data: CuesData,
    /// What kind of edit are we in-progress of? eg DCA assignments, cue names, etc
    cues_ui_mode: ui::CuesUiMode,
    cues_ui_safety: ui::CuesUiSafety,
    cues_selected_cue_ind: Option<usize>,
    cues_copied_dca: Option<DcaState>,
    cues_copied_cue: Option<Cue>,
    // FIXME: Refactor this atrocity.
    /// An objectively terrible implementation, true, but it's funny. A stack of undo/redo actions.
    /// When undo is pressed, the backtrack counter is increased, and when redo is pressed, it is
    /// decreased.
    cues_action_stack: Vec<(Box<CuesEditAction>, Box<CuesEditAction>)>,
    cues_action_stack_backtracks: usize,

    // Board
    board_connection: Box<dyn board::Connectable>,
    /// The connection selected in the dropdown on the board screen
    board_connection_ui: board::Connections,
    /// The currently active connection to difference with above
    board_connection_ui_prev: board::Connections,

    // File
    file_state: file::FileState,
}

impl Default for State {
    fn default() -> Self {
        State {
            cues_data: CuesData::default(),
            board_connection: Box::new(board::NoConnection::new()),

            file_state: file::FileState::default(),

            ui_screen: ui::UiScreen::default(),
            cues_ui_mode: ui::CuesUiMode::default(),
            cues_ui_safety: ui::CuesUiSafety::default(),
            cues_selected_cue_ind: None,
            cues_copied_dca: None,
            cues_copied_cue: None,
            cues_action_stack: Vec::default(),
            cues_action_stack_backtracks: usize::default(),
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
    pub fn cues_data(&self) -> &CuesData {
        &self.cues_data
    }
    pub fn cues_data_mut(&mut self) -> &mut CuesData {
        &mut self.cues_data
    }
    pub fn cues(&self) -> &BTreeMap<CueNumber, Cue> {
        self.cues_data.cues()
    }
    pub fn cues_mut(&mut self) -> &mut BTreeMap<CueNumber, Cue> {
        self.cues_data.cues_mut()
    }
    pub fn cues_ch_names(&self) -> &ChannelNames {
        self.cues_data.ch_names()
    }
    pub fn cues_ch_names_mut(&mut self) -> &mut ChannelNames {
        self.cues_data.ch_names_mut()
    }
    pub fn cues_ui_mode(&self) -> &ui::CuesUiMode {
        &self.cues_ui_mode
    }
    pub fn cues_ui_mode_mut(&mut self) -> &mut ui::CuesUiMode {
        &mut self.cues_ui_mode
    }
    pub fn cues_is_editing(&self) -> bool {
        matches!(self.cues_ui_safety, ui::CuesUiSafety::Edit)
    }
    pub fn cues_is_show(&self) -> bool {
        matches!(self.cues_ui_safety, ui::CuesUiSafety::Show)
    }
    pub fn cues_ui_safety_mut(&mut self) -> &mut ui::CuesUiSafety {
        &mut self.cues_ui_safety
    }
    pub fn cues_selected_cue(&self) -> Option<usize> {
        self.cues_selected_cue_ind
    }
    pub fn cues_copied_dca(&self) -> &Option<DcaState> {
        &self.cues_copied_dca
    }
    pub fn cues_copied_dca_mut(&mut self) -> &mut Option<DcaState> {
        &mut self.cues_copied_dca
    }
    pub fn cues_copied_cue(&self) -> &Option<Cue> {
        &self.cues_copied_cue
    }
    pub fn cues_copied_cue_mut(&mut self) -> &mut Option<Cue> {
        &mut self.cues_copied_cue
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
        // FIX: why are we cloning here
        let ch_names = self.cues_ch_names().clone();
        self.board_connection.fire_channel_names(&ch_names);
    }
}
