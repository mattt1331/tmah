//! A data structure which holds the current (known) state of the board so that we can find the
//! smallest number of messages needed to reach a desired state

use super::super::{ChannelNames, Cue};
use super::{BoardEdit, Channel, Dca};
use std::collections::{BTreeMap, BTreeSet};

/// The data structure which holds the board's known state. This is probably not very efficient
/// considering we have BTrees containing BTrees.
// TODO: Actually check how much memory/time this data structure uses and ensure it isn't slowing us
// down too much
#[derive(Default)]
pub struct BoardStateCache {
    channel_mute: BTreeMap<Channel, bool>,
    dca_assignments: BTreeMap<Dca, BTreeSet<Channel>>,
    dca_names: BTreeMap<Dca, String>,
}

impl BoardStateCache {
    /// Update the cache with the change represented by `message` in order to keep the cache in sync
    /// with what is going on.
    pub fn apply_board_message(&mut self, message: &BoardEdit) {
        match message {
            BoardEdit::ChannelMute(ch, mute) => {
                self.channel_mute.insert(ch.clone(), *mute);
            }
            BoardEdit::ChannelDcaAssign(ch, dca, assign) => {
                if *assign {
                    self.dca_assignments
                        .entry(dca.clone())
                        .or_default()
                        .insert(ch.clone());
                } else {
                    self.dca_assignments
                        .entry(dca.clone())
                        .or_default()
                        .remove(ch);
                }
            }
            BoardEdit::DcaName(dca, name) => {
                self.dca_names.insert(dca.clone(), name.clone());
            }
            // Don't care because we don't track this
            BoardEdit::ChannelName(..) => (),
            BoardEdit::DcaLevel(..) => (),
        }
    }
    /// Generate a list of edits that need to be made in order to bring the state of the board in
    /// line with what is specified in the cue. Some things which are not cached will always be
    /// sent.
    // TODO: Move assignments, mutes, levels, and names into their own separate helpers
    fn cue_diff(
        &self,
        cue: &Cue,
        channel_names: &ChannelNames,
        num_channels: u8,
        num_dcas: u8,
    ) -> Vec<BoardEdit> {
        // DCA assignments
        let mut channel_assigns: Vec<BoardEdit> = Vec::new();
        let mut channel_unassigns: Vec<BoardEdit> = Vec::new();
        for (dca_ind, new_dca) in cue.dcas().iter().enumerate() {
            // Don't do anything with extra dcas that we aren't supposed to touch
            if dca_ind >= num_dcas.into() {
                break;
            }
            if let Some(current_assignments) =
                self.dca_assignments.get(&Dca::from_index(dca_ind as u8))
            {
                // We know the current state of this dca
                // Channels to add: cue - state
                // Channels to remove: state - cue
                let assigns = new_dca
                    .assigned()
                    .difference(current_assignments)
                    .filter_map(|channel| {
                        if channel.index() < num_channels {
                            Some(BoardEdit::ChannelDcaAssign(
                                channel.clone(),
                                Dca::from_index(dca_ind as u8),
                                true,
                            ))
                        } else {
                            None
                        }
                    });
                channel_assigns.extend(assigns);
                let unassigns = current_assignments
                    .difference(new_dca.assigned())
                    .filter_map(|channel| {
                        if channel.index() < num_channels {
                            Some(BoardEdit::ChannelDcaAssign(
                                channel.clone(),
                                Dca::from_index(dca_ind as u8),
                                false,
                            ))
                        } else {
                            None
                        }
                    });
                channel_unassigns.extend(unassigns);
            } else {
                // We do not know the current state of this DCA. Fire every channel.
                for ch_ind in 0..num_channels {
                    let ch = Channel::from_index(ch_ind);
                    if matches!(new_dca.assigned().get(&ch), Some(_)) {
                        // Channel is assigned to DCA
                        channel_assigns.push(BoardEdit::ChannelDcaAssign(
                            ch,
                            Dca::from_index(dca_ind as u8),
                            true,
                        ));
                    } else {
                        // Channel is unassigned from DCA
                        channel_unassigns.push(BoardEdit::ChannelDcaAssign(
                            ch,
                            Dca::from_index(dca_ind as u8),
                            false,
                        ));
                    }
                }
            }
        }
        // Channel mutes
        let mut channel_mutes: Vec<BoardEdit> = Vec::new();
        for ch_ind in 0..num_channels {
            if ch_ind >= num_channels.into() {
                break;
            }
            let ch = Channel::from_index(ch_ind);
            // If we are in any DCA, we should be unmuted
            let mut is_channel_muted = true;
            for (dca_ind, new_dca) in cue.dcas().iter().enumerate() {
                if dca_ind >= num_dcas.into() {
                    break;
                }
                if let Some(_) = new_dca.assigned().get(&ch) {
                    is_channel_muted = false;
                    break;
                }
            }
            if let Some(mute_state) = self.channel_mute.get(&ch) {
                // We know the state of the channel mute. Only send if it is wrong.
                if *mute_state != is_channel_muted {
                    channel_mutes.push(BoardEdit::ChannelMute(ch, is_channel_muted));
                }
            } else {
                // We do not know the state of the channel mute. Send it.
                channel_mutes.push(BoardEdit::ChannelMute(ch, is_channel_muted));
            }
        }
        // DCA levels
        let mut dca_levels: Vec<BoardEdit> = Vec::new();
        for (dca_ind, new_dca) in cue.dcas().iter().enumerate() {
            if dca_ind >= num_dcas.into() {
                break;
            }
            if let Some(level) = new_dca.level() {
                dca_levels.push(BoardEdit::DcaLevel(Dca::from_index(dca_ind as u8), *level));
            }
        }
        // DCA names
        let mut dca_names: Vec<BoardEdit> = Vec::new();
        for (dca_ind, new_dca) in cue.dcas().iter().enumerate() {
            if dca_ind >= num_dcas.into() {
                break;
            }
            let new_name = new_dca.name(channel_names);
            if let Some(current_name) = self.dca_names.get(&Dca::from_index(dca_ind as u8)) {
                // We know the current name, send it if it's wrong
                if *current_name != new_name {
                    dca_names.push(BoardEdit::DcaName(Dca::from_index(dca_ind as u8), new_name));
                }
            } else {
                // We do not know the current name, send it
                dca_names.push(BoardEdit::DcaName(Dca::from_index(dca_ind as u8), new_name));
            }
        }

        let mut out = channel_assigns;
        out.append(&mut channel_mutes);
        out.append(&mut channel_unassigns);
        out.append(&mut dca_levels);
        out.append(&mut dca_names);
        out
    }
    //pub fn cue_diff_and_apply(&mut self, cue: &Cue)
}
