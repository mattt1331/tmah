//! Contains the program state and UI

mod cues;
mod board;
mod file;
mod ui;

use board::Channel;
pub use board::Decibels;
pub use cues::{Cue, CueNumber, ChannelNames, UndoAction};

use std::collections::{BTreeMap, BTreeSet};

/// Top level of program state
pub struct State {
    ui_screen: ui::UiScreen,


    // Cues
    cues: BTreeMap<CueNumber, Cue>,
    cues_ch_names: ChannelNames,
    /// What kind of edit are we in-progress of? eg DCA assignments, cue names, etc
    cues_ui_mode: ui::CuesUiMode,
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

impl Default for State {
    fn default() -> Self {
        State {
            cues: BTreeMap::new(),
            cues_ch_names: ChannelNames::default(),
            board_connection: Box::new(board::NoConnection::new()),

            file_state: file::FileState::default(),

            ui_screen: ui::UiScreen::default(),
            cues_ui_mode: ui::CuesUiMode::default(),
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
    pub fn cues_ui_mode(&self) -> &ui::CuesUiMode {
        &self.cues_ui_mode
    }
    pub fn cues_ui_mode_mut(&mut self) -> &mut ui::CuesUiMode {
        &mut self.cues_ui_mode
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

