//! Contains the program state and UI

mod board;
mod file;
mod ui;

pub use board::Decibels;
use board::{Channel, Connectable};

use std::collections::HashSet;

/// Top level of program state
pub struct State {
    // Actual program state
    cues: Vec<Cue>,
    ch_names: ChannelNames,
    connection: Box<dyn board::Connectable>,

    /// State related to loading show files
    file_state: file::FileState,

    // UI state etc
    ui_screen: ui::UiScreen,
    /// What kind of edit are we in-progress of? eg DCA assignments, cue names, etc
    cues_edit_action: ui::CuesEditAction,
    cues_selected_cue_ind: Option<usize>,
    // FIXME: Refactor this atrocity.
    /// An objectively terrible implementation, true, but it's funny. A stack of undo actions. When
    /// undo is pressed, pop off the last one and run it on `State`. When an undoable action
    /// occurs, add the undo action to the stack.
    cues_undo_stack: Vec<Box<UndoAction>>,
    /// The connection selected in the dropdown on the board screen
    connection_ui: board::Connections,
    /// The currently active connection to difference with above
    connection_ui_prev: board::Connections,
}

type UndoAction = dyn FnOnce(&mut State);

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
            ],
            ch_names: ChannelNames::default(),
            connection: Box::new(board::NoConnection::new()),

            file_state: file::FileState::default(),

            ui_screen: ui::UiScreen::default(),
            cues_edit_action: ui::CuesEditAction::default(),
            cues_selected_cue_ind: None,
            cues_undo_stack: Vec::default(),
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
    pub fn add_cue(&mut self, cue: Cue, index: usize, no_undo: bool) {
        let index = std::cmp::min(index, self.cues.len());
        self.cues.insert(index, cue);
        if !no_undo {
            self.do_cues_edit_action(ui::CuesEditAction::EditCueDesc { cue_ind: index });
            self.cues_stack_undo(Box::new(move |state| {
                state.delete_cue(index, true);
            }));
        }
    }
    pub fn delete_cue(&mut self, index: usize, no_undo: bool) {
        if index < self.cues.len() {
            let removed_cue = self.cues.remove(index);
            if !no_undo {
                self.cues_stack_undo(Box::new(move |state| {
                    state.add_cue(removed_cue, index, true);
                }));
            }
            // Update index
            if let Some(sel_ind) = self.cues_selected_cue_ind {
                if sel_ind > 0 {
                    self.set_selected_cue(Some(sel_ind - 1));
                } else {
                    self.set_selected_cue(None);
                }
            }
        }
    }
    /// Move the cue at `start_index` to `end_index`. If `end_index` is greater than or equal to
    /// the number of cues, the cue will be moved to the end of the list.
    pub fn renumber_cue(&mut self, start_index: usize, end_index: usize, no_undo: bool) {
        // Check that start_index is in range
        if start_index > self.cues.len() - 1 {
            return;
        }
        // Clamp end_index to be range
        let end_index = std::cmp::min(end_index, self.cues.len() - 1);
        if end_index == start_index {
            return;
        }
        let cue = self.cues.remove(start_index);
        self.cues.insert(end_index, cue);
        if !no_undo {
            self.cues_stack_undo(Box::new(move |state| {
                state.renumber_cue(end_index, start_index, true);
            }));
        }
    }
    pub fn cues_stack_undo(&mut self, action: Box<UndoAction>) {
        self.cues_undo_stack.push(action);
    }
    pub fn cues_do_undo(&mut self) {
        if let Some(action) = self.cues_undo_stack.pop() {
            action(self);
        }
    }
    pub fn channel_names(&self) -> &ChannelNames {
        &self.ch_names
    }
    pub fn channel_names_mut(&mut self) -> &mut ChannelNames {
        &mut self.ch_names
    }
    pub fn cues_edit_action(&self) -> &ui::CuesEditAction {
        &self.cues_edit_action
    }
    pub fn cues_edit_action_mut(&mut self) -> &mut ui::CuesEditAction {
        &mut self.cues_edit_action
    }
    pub fn do_cues_edit_action(&mut self, action: ui::CuesEditAction) {
        self.cues_edit_action = action;
    }
    pub fn clear_cues_edit_action(&mut self) {
        self.cues_edit_action = ui::CuesEditAction::None;
    }
    pub fn selected_cue(&self) -> Option<usize> {
        self.cues_selected_cue_ind
    }
    pub fn set_selected_cue(&mut self, cue_ind: Option<usize>) {
        if let Some(ind) = cue_ind
            && ind < self.cues.len()
        {
            self.cues_selected_cue_ind = cue_ind;
        } else {
            self.cues_selected_cue_ind = None;
        }
    }
    pub fn fire_selected_cue(&mut self) {
        if let Some(ind) = self.cues_selected_cue_ind {
            self.connection.fire_cue(&self.cues[ind]);
        }
    }
    pub fn fire_next_cue(&mut self) {
        if let Some(ind) = self.cues_selected_cue_ind {
            self.set_selected_cue(Some(ind + 1));
            self.fire_selected_cue();
        } else {
            self.set_selected_cue(Some(0));
            self.fire_selected_cue();
        }
    }
    pub fn fire_channel_names(&mut self) {
        self.connection.fire_channel_names(&self.ch_names);
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
    /// Takes this cue and has the provided `top` cue override any parameters that `top` sets.
    /// Currently this just returns `top` but when EQs and crap get added this will make much more
    /// sense as Cue is currently used to record the state of the board (M7CL). This could also be
    /// terrible but idk
    // FIXME: this is not correct anymore. Use diff
    fn superimpose(&self, top: &Cue) -> Cue {
        top.clone()
    }
    /// Finds the difference between the two given cues and produces a list of board edits to go
    /// from `prev` to `next`.
    fn diff(prev: &Cue, next: &Cue) -> Vec<board::BoardEdit> {
        let mut diff: Vec<board::BoardEdit> = Vec::new();
        // Get the diffs for each individual DCA
        for (i, next_dca) in next.dcas.iter().enumerate() {
            if i < prev.dcas.len() {
                diff.append(&mut DcaState::diff(
                    &prev.dcas[i],
                    next_dca,
                    board::Dca::from_index(i as u8),
                ));
            } else {
                // If somehow the second cue has more dcas listed than the first
                diff.append(&mut next.dcas[i].full_send(board::Dca::from_index(i as u8)));
            }
        }
        // Deconflict unmutes: if someone is exiting one dca and entering another, DO NOT send the
        // mute.
        // FIX: this is not the best way to implement this.
        let mut unmutes = Vec::new();
        for edit in &diff {
            if matches!(edit, board::BoardEdit::ChannelMute(_, false)) {
                unmutes.push(edit.clone());
            }
        }
        for unmute in unmutes {
            let board::BoardEdit::ChannelMute(unmute_channel, _) = unmute else {
                unreachable!(
                    "All elements of `unmutes` should be `BoardEdit::ChannelMute` because of above"
                );
            };
            diff = diff
                .into_iter()
                .filter(|edit| {
                    match edit {
                        board::BoardEdit::ChannelMute(channel, true) => {
                            *channel != unmute_channel
                        }
                        _ => true,
                    }
                })
                .collect();
        }
        diff
    }
}

/// The state of a DCA, which can be realized by calling a cue
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct DcaState {
    assigned: HashSet<Channel>,
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
                if !name.is_empty() {
                    name = ch_name;
                } else {
                    name = format!("{name}, {}", ch_name)
                }
            }
            name.to_string()
        }
    }
    fn edit_name(&mut self) -> &mut Option<String> {
        &mut self.name
    }
    fn assigned(&self) -> &HashSet<Channel> {
        &self.assigned
    }
    /// Assigns the given channel to this DCA
    fn assign(&mut self, ch: Channel) {
        self.assigned.insert(ch);
    }
    /// Unassigns the given channel from this DCA
    fn unassign(&mut self, ch: Channel) {
        self.assigned.remove(&ch);
    }
    /// Find the differences between the two given `DcaState`s. The differences will be given as
    /// board edits so that they can be applied to go from `prev` to `next`. The diff includes
    /// mute/unmute operations so that channels which are unassigned are muted and channels which
    /// are newly assigned are unmuted.
    fn diff(prev: &DcaState, next: &DcaState, dca: board::Dca) -> Vec<board::BoardEdit> {
        let unassigned = prev.assigned.difference(&next.assigned);
        let assigned = next.assigned.difference(&prev.assigned);
        let mut diff = Vec::new();
        for ch in unassigned {
            diff.push(board::BoardEdit::ChannelDcaAssign(
                ch.clone(),
                dca.clone(),
                false,
            ));
            diff.push(board::BoardEdit::ChannelMute(ch.clone(), true));
        }
        for ch in assigned {
            diff.push(board::BoardEdit::ChannelDcaAssign(
                ch.clone(),
                dca.clone(),
                true,
            ));
            diff.push(board::BoardEdit::ChannelMute(ch.clone(), false));
        }
        if prev.name != next.name
            && let Some(name) = &next.name
        {
            diff.push(board::BoardEdit::DcaName(dca.clone(), name.clone()));
        }
        if prev.level != next.level
            && let Some(level) = next.level
        {
            diff.push(board::BoardEdit::DcaLevel(dca.clone(), level));
        }
        diff
    }
    /// Returns a diff, as board edit actions, which applies all of the values stored in the
    /// `DcaState`.
    fn full_send(&self, dca: board::Dca) -> Vec<board::BoardEdit> {
        let mut diff = Vec::new();
        for ch in &self.assigned {
            diff.push(board::BoardEdit::ChannelDcaAssign(
                ch.clone(),
                dca.clone(),
                true,
            ));
        }
        if let Some(name) = &self.name {
            diff.push(board::BoardEdit::DcaName(dca.clone(), name.clone()));
        }
        if let Some(level) = self.level {
            diff.push(board::BoardEdit::DcaLevel(dca.clone(), level));
        }
        diff
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
