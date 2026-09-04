//! This module contains methods on `State` which correspond to UI actions in the cues screen. They
//! are here so as not to clutter up the main file.

use super::*;

/// Cues UI functions
impl super::State {
    /// Adds the given cue at the given number.
    /// UNDO: If `no_undo` is false, stacks an undo action.
    pub fn cues_add_cue(&mut self, cue: Cue, number: CueNumber, no_undo: bool) {
        // Check that there isn't already a cue at this number
        if self.cues.contains_key(&number) {
            log::warn!("Did not insert cue because this number is already occupied");
            return;
        }
        self.cues.insert(number.clone(), cue);
        if !no_undo && let Some(cue_ind) = self.cues.keys().position(|c| *c == number) {
            self.cues_do_edit_action(ui::CuesEditAction::EditCueDesc { cue_ind });
            self.cues_stack_undo_action(Box::new(move |state| {
                state.cues_delete_cue(&number, true);
            }));
        }
    }
    /// Deletes the cue at the specified number.
    /// UNDO: If `no_undo` is false, stacks an undo action.
    pub fn cues_delete_cue(&mut self, number: &CueNumber, no_undo: bool) {
        let removed_cue = self.cues.remove(number);
        if !no_undo && let Some(removed_cue) = removed_cue {
            let number = number.clone();
            self.cues_stack_undo_action(Box::new(move |state| {
                state.cues_add_cue(removed_cue, number, true);
            }));
        }
        // Update index
        if let Some(sel_ind) = self.cues_selected_cue_ind {
            if sel_ind > 0 {
                self.cues_set_selected(Some(sel_ind - 1));
            } else {
                self.cues_set_selected(None);
            }
        }
    }
    /// Move the cue at `start_index` to `end_number`.
    /// UNDO: If `no_undo` is false, stacks an undo action.
    pub fn cues_renumber_cue(&mut self, start_num: CueNumber, end_num: CueNumber, no_undo: bool) {
        // Check that there isn't already a cue with `end_number`
        if !self.cues.contains_key(&end_num) {
            // Get the cue we are renumbering
            if let Some(cue) = self.cues.remove(&start_num) {
                self.cues.insert(end_num.clone(), cue);

                if !no_undo {
                    self.cues_stack_undo_action(Box::new(move |state| {
                        state.cues_renumber_cue(end_num, start_num, true);
                    }));
                }
            } else {
                log::warn!("Could not renumber cue because getting the cue failed");
            }
        } else {
            log::warn!("Could not renumber cue because new number already exists");
        }
    }
    /// Adds the given undo action to the top of the undo stack.
    pub fn cues_stack_undo_action(&mut self, action: Box<UndoAction>) {
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
    pub fn cues_do_edit_action(&mut self, action: ui::CuesEditAction) {
        if !matches!(self.cues_edit_action, ui::CuesEditAction::None) {
            self.cues_end_edit_action();
        }
        self.cues_edit_action = action;
    }
    /// Ends any active cues edit action. For actions which edit state continuously (eg dca assign
    /// popup), stacks an undo action.
    pub fn cues_end_edit_action(&mut self) {
        match self.cues_edit_action {
            ui::CuesEditAction::None => return,
            ui::CuesEditAction::EditChannelNames => {
                log::warn!("Please implement undo for edit channel names")
            }
            ui::CuesEditAction::EditDcaAssign {
                cue_ind,
                dca_ind,
                ref original_dca_name,
                ref original_assignment,
            } => {
                let original_dca_name = original_dca_name.clone();
                let original_assignment = original_assignment.clone();
                self.cues_stack_undo_action(Box::new(move |state| {
                    let dca = state.cues.values_mut().nth(cue_ind)
                        .map(|cue| &mut cue.dcas[dca_ind]);
                    if let Some(dca) = dca {
                        dca.name = original_dca_name;
                        dca.set_assigned(original_assignment)
                    } else {
                        log::warn!("Could not find DCA at index {dca_ind} in cue at index {cue_ind} for undoing dca assign popup edits");
                    }
                }));
            }
            ui::CuesEditAction::EditCueDesc { cue_ind: _ } => {
                log::warn!("Please implement undo for editing cue desc")
            }
            ui::CuesEditAction::RenumberCue {
                cue_ind: _,
                input_text: _,
            } => {
                // Renumbering cues is a one-shot and undo is implemented elsewhere
            }
        }
        self.cues_edit_action = ui::CuesEditAction::None;
    }
    /// Sets the selected cue to the given index, validating the index. If `None` is given instead,
    /// deselect any selected cue.
    pub fn cues_set_selected(&mut self, cue_ind: Option<usize>) {
        if let Some(ind) = cue_ind
            && ind < self.cues.len()
        {
            self.cues_selected_cue_ind = cue_ind;
        } else {
            self.cues_selected_cue_ind = None;
        }
    }
}
