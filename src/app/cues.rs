//! This module contains methods on `State` which correspond to UI actions in the cues screen and
//! types for the data associated with cues. This is here so as to not clutter up the main file.

use super::*;

/// Cues UI functions
impl super::State {
    /// Adds the given undo action to the top of the undo stack.
    pub fn cues_stack_undo_action(&mut self, action: Box<CuesEditAction>) {
        self.cues_undo_stack.push(action);
    }
    /// Executes the undo action at the top of the stack and pops it off of the stack.
    pub fn cues_do_undo(&mut self) {
        if let Some(action) = self.cues_undo_stack.pop() {
            action(self);
        }
    }
    /// Commences the given edit action. If another edit action is already active, ends that
    /// action.
    pub fn cues_do_edit_action(&mut self, action: ui::CuesUiMode) {
        if !matches!(self.cues_ui_mode, ui::CuesUiMode::None) {
            self.cues_end_edit_action();
        }
        self.cues_ui_mode = action;
    }
    /// Ends any active cues edit action. For actions which edit state continuously (eg dca assign
    /// popup), stacks an undo action.
    pub fn cues_end_edit_action(&mut self) {
        match self.cues_ui_mode {
            ui::CuesUiMode::None => return,
            ui::CuesUiMode::EditChannelNames => {
                log::warn!("Please implement undo for edit channel names")
            }
            ui::CuesUiMode::EditDcaAssign {
                cue_ind,
                dca_ind,
                ref original_dca_name,
                ref original_assignment,
            } => {
                let original_dca_name = original_dca_name.clone();
                let original_assignment = original_assignment.clone();
                self.cues_stack_undo_action(Box::new(move |state| {
                    let dca = state.cues_mut().values_mut().nth(cue_ind)
                        .map(|cue| &mut cue.dcas[dca_ind]);
                    if let Some(dca) = dca {
                        dca.name = original_dca_name;
                        dca.set_assigned(original_assignment)
                    } else {
                        log::warn!("Could not find DCA at index {dca_ind} in cue at index {cue_ind} for undoing dca assign popup edits");
                    }
                }));
            }
            ui::CuesUiMode::EditCueDesc { cue_ind: _ } => {
                log::warn!("Please implement undo for editing cue desc")
            }
            ui::CuesUiMode::RenumberCue {
                cue_ind: _,
                input_text: _,
            } => {
                // Renumbering cues is a one-shot and undo is implemented elsewhere
            }
        }
        self.cues_ui_mode = ui::CuesUiMode::None;
    }
}
/// Functions used by the cues ui.
impl super::State {
    /// Adds the given cue at the given number.
    pub fn cues_add_cue(&mut self, cue: Cue, number: CueNumber) {
        // Check that there isn't already a cue at this number
        if self.cues().contains_key(&number) {
            log::warn!("Did not insert cue because this number is already occupied");
            return;
        }
        self.cues_data.add_cue(cue.clone(), number.clone());
        let num = number.clone();
        let undo = move |data: &mut CuesData| {
            let cue = cue.clone();
            let number = num;
            data.add_cue(cue.clone(), number.clone());
        };
        let redo = move |data: &mut CuesData| {
            let number = number;
            data.remove_cue(&number);
        };
        self.cues_stack_action(Box::new(undo), Box::new(redo));
    }
    /// Deletes the cue at the specified number.
    pub fn cues_delete_cue(&mut self, number: &CueNumber) {
        if let Some(removed_cue) = self.cues_data.remove_cue(number) {
            let num = number.clone();
            let undo = move |data: &mut CuesData| {
                let cue = removed_cue.clone();
                let number = num;
                data.add_cue(cue.clone(), number.clone());
            };
            let num = number.clone();
            let redo = move |data: &mut CuesData| {
                let number = num;
                data.remove_cue(&number);
            };
            self.cues_stack_action(Box::new(undo), Box::new(redo));
            // Update index
            if let Some(sel_ind) = self.cues_selected_cue_ind {
                if sel_ind > 0 {
                    self.cues_set_selected(Some(sel_ind - 1));
                } else {
                    self.cues_set_selected(None);
                }
            }
        }
    }
    /// Move the cue at `start_index` to `end_number`.
    pub fn cues_renumber_cue(&mut self, start_num: CueNumber, end_num: CueNumber) {
        // Check that there isn't already a cue with `end_number`
        if !self.cues().contains_key(&end_num) {
            self.cues_data.renumber_cue(start_num.clone(), end_num.clone());
            let start = end_num.clone();
            let end = start_num.clone();
            let undo = move |data: &mut CuesData| {
                let start_num = start;
                let end_num = end;
                data.renumber_cue(start_num, end_num);
            };
            let start = end_num.clone();
            let end = start_num.clone();
            let redo = move |data: &mut CuesData| {
                let start_num = start;
                let end_num = end;
                data.renumber_cue(start_num, end_num);
            };
            self.cues_stack_action(Box::new(undo), Box::new(redo));
        } else {
            log::warn!("Could not renumber cue because new number already exists");
        }
    }
    /// Sets the selected cue to the given index, validating the index. If `None` is given instead,
    /// deselect any selected cue.
    pub fn cues_set_selected(&mut self, cue_ind: Option<usize>) {
        if let Some(ind) = cue_ind
            && ind < self.cues().len()
        {
            self.cues_selected_cue_ind = cue_ind;
        } else {
            self.cues_selected_cue_ind = None;
        }
    }
    /// Checks whether we can undo right now.
    pub fn cues_can_undo(&self) -> bool {
        self.cues_action_stack.len() - self.cues_action_stack_backtracks > 0
    }
    /// Executes the undo action to take us back a step and increment the backtrack counter.
    pub fn cues_undo(&mut self) {
        if self.cues_can_undo() {
            (self.cues_action_stack
                [self.cues_action_stack.len() - self.cues_action_stack_backtracks - 1]
                .0.box_clone())(&mut self.cues_data);
            self.cues_action_stack_backtracks += 1;
        }
    }
    /// Checks whether we can redo right now.
    pub fn cues_can_redo(&self) -> bool {
        self.cues_action_stack_backtracks > 0
    }
    /// Executes the redo action to move us forwards a step and decrement the backtrack counter.
    pub fn cues_redo(&mut self) {
        if self.cues_can_redo() {
            (self.cues_action_stack
                [self.cues_action_stack.len() - self.cues_action_stack_backtracks - 1]
                .1.box_clone())(&mut self.cues_data);
            self.cues_action_stack_backtracks -= 1;
        }
    }
    /// Adds the given undo/redo action to the top of the stack, discarding any actions which could
    /// be redone from this point.
    pub fn cues_stack_action(
        &mut self,
        backwards_action: Box<CuesEditActionPrime>,
        forwards_action: Box<CuesEditActionPrime>,
    ) {
        self.cues_action_stack
            .truncate(self.cues_action_stack.len() - self.cues_action_stack_backtracks);
        self.cues_action_stack
            .push((backwards_action, forwards_action));
        self.cues_action_stack_backtracks = 0;
    }
}

pub type CuesEditAction = dyn FnOnce(&mut State);
/// Some action which edits a `CuesData`
pub type CuesEditActionPrime = dyn CloneableClosure;
/// This trait denotes and is automatically implemented for closures which can be cloned and which
/// we can move values into.
/// Because you could undo, redo, undo, redo the same action multiple times, we use these closures
/// to store edit actions so that we can clone them to apply them multiple times. Is this a good
/// idea? Probably not, but neither is storing closures instead of doing this a normal way and I'm
/// too lazy to implement this correctly.
pub trait CloneableClosure: for<'a> FnOnce(&'a mut CuesData) -> () {
    fn box_clone(&self) -> Box<dyn CloneableClosure>;
}
impl<F> CloneableClosure for F 
where
    F: for<'a> FnOnce(&'a mut CuesData) + Clone + 'static
{
    fn box_clone(&self) -> Box<dyn CloneableClosure> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CuesData {
    cues: BTreeMap<CueNumber, Cue>,
    ch_names: ChannelNames,
}
impl CuesData {
    pub fn cues(&self) -> &BTreeMap<CueNumber, Cue> {
        &self.cues
    }
    pub fn cues_mut(&mut self) -> &mut BTreeMap<CueNumber, Cue> {
        &mut self.cues
    }
    pub fn ch_names(&self) -> &ChannelNames {
        &self.ch_names
    }
    pub fn ch_names_mut(&mut self) -> &mut ChannelNames {
        &mut self.ch_names
    }
    /// Adds the given cue at the given number.
    pub fn add_cue(&mut self, cue: Cue, number: CueNumber) {
        // Check that there isn't already a cue at this number
        if self.cues().contains_key(&number) {
            log::warn!("Did not insert cue because this number is already occupied");
            return;
        }
        self.cues_mut().insert(number, cue);
    }
    /// Deletes the cue at the specified number and returns it.
    pub fn remove_cue(&mut self, number: &CueNumber) -> Option<Cue> {
        let removed_cue = self.cues_mut().remove(number);
        removed_cue
    }
    /// Move the cue at `start_index` to `end_number`.
    pub fn renumber_cue(&mut self, start_num: CueNumber, end_num: CueNumber) {
        // Check that there isn't already a cue with `end_number`
        if !self.cues().contains_key(&end_num) {
            // Get the cue we are renumbering
            if let Some(cue) = self.cues_mut().remove(&start_num) {
                self.cues_mut().insert(end_num.clone(), cue);
            } else {
                log::warn!("Could not renumber cue because getting the cue failed");
            }
        } else {
            log::warn!("Could not renumber cue because new number already exists");
        }
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
    pub fn parse(input: &str) -> Result<CueNumber, ()> {
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
    pub fn increment_lowest(&mut self) {
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
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn edit_name(&mut self) -> &mut String {
        &mut self.name
    }
    pub fn dcas(&self) -> &Vec<DcaState> {
        &self.dcas
    }
    pub fn dcas_mut(&mut self) -> &mut Vec<DcaState> {
        &mut self.dcas
    }
}

/// The state of a DCA, which can be realized by calling a cue
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DcaState {
    assigned: BTreeSet<Channel>,
    name: Option<String>,
    level: Option<Decibels>,
}

impl DcaState {
    pub fn name(&self, ch_names: &ChannelNames) -> String {
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
    pub fn assigned_name(&self) -> &Option<String> {
        &self.name
    }
    pub fn edit_name(&mut self) -> &mut Option<String> {
        &mut self.name
    }
    pub fn level(&self) -> &Option<Decibels> {
        &self.level
    }
    pub fn assigned(&self) -> &BTreeSet<Channel> {
        &self.assigned
    }
    pub fn set_assigned(&mut self, assignment: BTreeSet<Channel>) {
        self.assigned = assignment;
    }
    /// Assigns the given channel to this DCA
    pub fn assign(&mut self, ch: Channel) {
        self.assigned.insert(ch);
    }
    /// Unassigns the given channel from this DCA
    pub fn unassign(&mut self, ch: Channel) {
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
    pub fn edit_name(&mut self, ch: &Channel) -> &mut String {
        self.names.entry(ch.index()).or_insert("".to_string())
    }
    /// Returns the name of the channel, if set
    pub fn get_name(&self, ch: &Channel) -> Option<String> {
        self.names.get(&ch.index()).map(|name| name.to_string())
    }
    /// Returns an iterator over (ch_ind, name) for the channels whose names are set.
    pub fn iterator(&self) -> std::collections::hash_map::Iter<'_, u8, String> {
        self.names.iter()
    }
}
