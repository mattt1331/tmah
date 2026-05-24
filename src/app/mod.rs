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
    /// What kind of edit are we in-progress of? eg DCA assignments, cue names, etc
    cues_edit_action: ui::CuesEditAction,
    cues_selected_cue_ind: Option<usize>,
    // FIXME: Refactor this atrocity.
    /// An objectively terrible implementation, true, but it's funny. A stack of undo actions. When
    /// undo is pressed, pop off the last one and run it on `State`. When an undoable action
    /// occurs, add the undo action to the stack.
    cues_undo_stack: Vec<Box<dyn FnOnce(&mut State)>>,
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
            ],
            ch_names: ChannelNames::default(),
            connection: Box::new(board::NoConnection::new()),

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
    pub fn cues_stack_undo(&mut self, action: Box<dyn FnOnce(&mut State)>) {
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
    pub fn cues_edit_action(&self) -> &ui::CuesEditAction {
        &self.cues_edit_action
    }
    pub fn do_cues_edit_action(&mut self, action: ui::CuesEditAction) {
        self.cues_edit_action = action;
    }
    pub fn clear_cues_edit_action(&mut self) {
        self.cues_edit_action = ui::CuesEditAction::None;
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
    fn assigned(&self) -> &Vec<Channel> {
        &self.assigned
    }
    /// Assigns the given channel to this DCA
    fn assign(&mut self, ch: Channel) {
        if !self.assigned.contains(&ch) {
            self.assigned.push(ch);
            self.assigned.sort();
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
            self.assigned.remove(ind);
        }
    }
}

/// Contains the names of each channel
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChannelNames {
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
        self.names.get(&ch.index()).map(|name| name.to_string())
    }
}
