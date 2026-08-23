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
    pub fn apply_board_edit(&mut self, edit: &BoardEdit) {
        match edit {
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
    /// Update the cache with the changes represented by `message` in order to keep the cache in
    /// sync with what is going on.
    pub fn apply_board_edits(&mut self, edits: &[BoardEdit]) {
        for edit in edits {
            self.apply_board_edit(edit);
        }
    }
    /// Generate a list of edits that need to be made in order to bring the channel DCA assignments
    /// in line with what is specified in the cue. The first returned `Vec` is all of the _assign_
    /// edits, and the second is all of the _unassign_ edits. They are separate so that they can be
    /// ordered correctly.
    fn cue_diff_dca_assign(
        &self,
        cue: &Cue,
        num_channels: u8,
        num_dcas: u8,
    ) -> (Vec<BoardEdit>, Vec<BoardEdit>) {
        let mut channel_assigns: Vec<BoardEdit> = Vec::new();
        let mut channel_unassigns: Vec<BoardEdit> = Vec::new();
        for dca_ind in 0..num_dcas {
            // If the cue does not contain a dca at this index, treat it as a dca with nothing in it
            let new_dca_assign = match cue.dcas().get(dca_ind as usize) {
                Some(dca) => dca.assigned(),
                None => &BTreeSet::new(),
            };
            if let Some(current_assignments) =
                self.dca_assignments.get(&Dca::from_index(dca_ind as u8))
            {
                // We know the current state of this dca
                // Channels to add: cue - state
                // Channels to remove: state - cue
                let assigns =
                    new_dca_assign
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
                let unassigns =
                    current_assignments
                        .difference(new_dca_assign)
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
                    if matches!(new_dca_assign.get(&ch), Some(_)) {
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
        (channel_assigns, channel_unassigns)
    }
    /// Generate a list of edits that need to be made in order to set the channel mutes to what they
    /// should be in the cue.
    fn cue_diff_channel_mutes(&self, cue: &Cue, num_channels: u8, num_dcas: u8) -> Vec<BoardEdit> {
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
        channel_mutes
    }
    /// Generate a list of edits that need to be made in order to bring the mutes and DCA
    /// assignments on the board to what is specified in the cue.
    fn cue_diff_core(&self, cue: &Cue, num_channels: u8, num_dcas: u8) -> Vec<BoardEdit> {
        let (channel_assigns, mut channel_unassigns) =
            self.cue_diff_dca_assign(cue, num_channels, num_dcas);
        // Channel mutes
        let mut channel_mutes = self.cue_diff_channel_mutes(cue, num_channels, num_dcas);

        let mut out = channel_assigns;
        out.append(&mut channel_mutes);
        out.append(&mut channel_unassigns);
        out
    }
    /// Generate a list of edits that need to be made in order to bring the state of the board in
    /// line with what is specified in the cue. Some things which are not cached will always be
    /// sent.
    fn cue_diff(
        &self,
        cue: &Cue,
        channel_names: Option<&ChannelNames>,
        num_channels: u8,
        num_dcas: u8,
    ) -> Vec<BoardEdit> {
        let mute_and_assign = self.cue_diff_core(cue, num_channels, num_dcas);
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
        // FIX: We should not have to have this optional, I think.
        let mut dca_names: Vec<BoardEdit> = Vec::new();
        if let Some(channel_names) = channel_names {
            for dca_ind in 0..num_dcas {
                let new_name = match cue.dcas().get(dca_ind as usize) {
                    Some(dca) => dca.name(channel_names),
                    None => "".to_string(),
                };
                if let Some(current_name) = self.dca_names.get(&Dca::from_index(dca_ind as u8)) {
                    // We know the current name, send it if it's wrong
                    if *current_name != new_name {
                        dca_names
                            .push(BoardEdit::DcaName(Dca::from_index(dca_ind as u8), new_name));
                    }
                } else {
                    // We do not know the current name, send it
                    dca_names.push(BoardEdit::DcaName(Dca::from_index(dca_ind as u8), new_name));
                }
            }
        }

        let mut out = mute_and_assign;
        out.append(&mut dca_levels);
        out.append(&mut dca_names);
        out
    }
    pub fn cue_diff_and_apply(
        &mut self,
        cue: &Cue,
        channel_names: Option<&ChannelNames>,
        num_channels: u8,
        num_dcas: u8,
    ) -> Vec<BoardEdit> {
        let edits = self.cue_diff(cue, channel_names, num_channels, num_dcas);
        self.apply_board_edits(&edits);
        edits
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::DcaState;
    use super::*;

    /// Returns a `DcaState` with the channels given by the provided indices assigned
    fn dca_with_chs(chs: &[u8]) -> DcaState {
        let mut dca = DcaState::default();
        for ind in chs {
            dca.assign(Channel::from_index(*ind));
        }
        dca
    }
    /// Returns a `Cue` with the given DCAs
    fn cue_with_dcas(dcas: Vec<DcaState>) -> Cue {
        let mut cue = Cue::default();
        cue.dcas = dcas;
        cue
    }
    impl BoardStateCache {
        /// Cue diff core and apply
        fn cdcaa(&mut self, cue: &Cue, num_channels: u8, num_dcas: u8) -> Vec<BoardEdit> {
            let diff = self.cue_diff_core(cue, num_channels, num_dcas);
            self.apply_board_edits(&diff);
            diff
        }
    }
    /// Returns the channel with the given index
    fn ch(ind: u8) -> Channel {
        Channel::from_index(ind)
    }
    /// Returns the DCA with the given index
    fn dca(ind: u8) -> Dca {
        Dca::from_index(ind)
    }
    /// Returns a cache which knows that nothing is assigned anywhere and everything is muted
    fn warm_cache(num_channels: u8, num_dcas: u8) -> BoardStateCache {
        let mut cache = BoardStateCache::default();
        let mydca = dca_with_chs(&[]);
        let mycue = cue_with_dcas(vec![mydca; num_dcas.into()]);
        let _: Vec<BoardEdit> = cache.cdcaa(&mycue, num_channels, num_dcas);
        cache
    }

    #[test]
    fn one_ch_one_dca_cold() {
        let mut cache = BoardStateCache::default();
        let mydca = dca_with_chs(&[0]);
        let mycue = cue_with_dcas(vec![mydca]);
        let edits: Vec<BoardEdit> = cache.cdcaa(&mycue, 1, 1);
        assert_eq!(
            edits,
            vec![
                BoardEdit::ChannelDcaAssign(ch(0), dca(0), true),
                BoardEdit::ChannelMute(ch(0), false),
            ]
        );
    }
    #[test]
    fn one_ch_two_dca_cold() {
        let mut cache = BoardStateCache::default();
        let mydca = dca_with_chs(&[0]);
        let mycue = cue_with_dcas(vec![mydca]);
        let edits: Vec<BoardEdit> = cache.cdcaa(&mycue, 1, 2);
        assert_eq!(
            edits,
            vec![
                BoardEdit::ChannelDcaAssign(ch(0), dca(0), true),
                BoardEdit::ChannelMute(ch(0), false),
                BoardEdit::ChannelDcaAssign(ch(0), dca(1), false),
            ]
        );
    }
    #[test]
    fn two_ch_one_dca_cold() {
        let mut cache = BoardStateCache::default();
        let mydca = dca_with_chs(&[0]);
        let mycue = cue_with_dcas(vec![mydca]);
        let edits: Vec<BoardEdit> = cache.cdcaa(&mycue, 2, 1);
        assert_eq!(
            edits,
            vec![
                BoardEdit::ChannelDcaAssign(ch(0), dca(0), true),
                BoardEdit::ChannelMute(ch(0), false),
                BoardEdit::ChannelMute(ch(1), true),
                BoardEdit::ChannelDcaAssign(ch(1), dca(0), false),
            ]
        );
    }
    #[test]
    fn two_ch_two_dca_cold() {
        let mut cache = BoardStateCache::default();
        let dca1 = dca_with_chs(&[0]);
        let dca2 = dca_with_chs(&[1]);
        let mycue = cue_with_dcas(vec![dca1, dca2]);
        let edits: Vec<BoardEdit> = cache.cdcaa(&mycue, 2, 2);
        assert_eq!(
            edits,
            vec![
                BoardEdit::ChannelDcaAssign(ch(0), dca(0), true),
                BoardEdit::ChannelDcaAssign(ch(1), dca(1), true),
                BoardEdit::ChannelMute(ch(0), false),
                BoardEdit::ChannelMute(ch(1), false),
                BoardEdit::ChannelDcaAssign(ch(1), dca(0), false),
                BoardEdit::ChannelDcaAssign(ch(0), dca(1), false),
            ]
        );
    }
    #[test]
    fn move_to_new_dca() {
        let mut cache = BoardStateCache::default();
        let dca11 = dca_with_chs(&[0]);
        let dca12 = dca_with_chs(&[]);
        let dca21 = dca_with_chs(&[]);
        let dca22 = dca_with_chs(&[0]);
        let cue1 = cue_with_dcas(vec![dca11, dca12]);
        let cue2 = cue_with_dcas(vec![dca21, dca22]);
        let _: Vec<BoardEdit> = cache.cdcaa(&cue1, 1, 2);
        let edits: Vec<BoardEdit> = cache.cdcaa(&cue2, 1, 2);
        assert_eq!(
            edits,
            vec![
                BoardEdit::ChannelDcaAssign(ch(0), dca(1), true),
                BoardEdit::ChannelDcaAssign(ch(0), dca(0), false),
            ]
        );
    }
    #[test]
    fn one_ch_one_dca_warm() {
        let mut cache = warm_cache(1, 1);
        let mydca = dca_with_chs(&[0]);
        let mycue = cue_with_dcas(vec![mydca]);
        let edits: Vec<BoardEdit> = cache.cdcaa(&mycue, 1, 1);
        assert_eq!(
            edits,
            vec![
                BoardEdit::ChannelDcaAssign(ch(0), dca(0), true),
                BoardEdit::ChannelMute(ch(0), false),
            ]
        );
    }
    #[test]
    fn two_ch_two_dca_warm() {
        let mut cache = warm_cache(2, 2);
        let mydca = dca_with_chs(&[0]);
        let mycue = cue_with_dcas(vec![mydca]);
        let edits: Vec<BoardEdit> = cache.cdcaa(&mycue, 2, 2);
        assert_eq!(
            edits,
            vec![
                BoardEdit::ChannelDcaAssign(ch(0), dca(0), true),
                BoardEdit::ChannelMute(ch(0), false),
            ]
        );
    }
    #[test]
    fn five_ch_two_dca_warm() {
        let mut cache = warm_cache(5, 2);
        let dca1 = dca_with_chs(&[0, 1]);
        let dca2 = dca_with_chs(&[2, 3]);
        let mycue = cue_with_dcas(vec![dca1, dca2]);
        let edits: Vec<BoardEdit> = cache.cdcaa(&mycue, 5, 2);
        assert_eq!(
            edits,
            vec![
                BoardEdit::ChannelDcaAssign(ch(0), dca(0), true),
                BoardEdit::ChannelDcaAssign(ch(1), dca(0), true),
                BoardEdit::ChannelDcaAssign(ch(2), dca(1), true),
                BoardEdit::ChannelDcaAssign(ch(3), dca(1), true),
                BoardEdit::ChannelMute(ch(0), false),
                BoardEdit::ChannelMute(ch(1), false),
                BoardEdit::ChannelMute(ch(2), false),
                BoardEdit::ChannelMute(ch(3), false),
            ]
        );
    }
    #[test]
    fn name_caching() {
        let mut cache = BoardStateCache::default();
        let names = ChannelNames::default();
        let mut dca1 = dca_with_chs(&[0]);
        *dca1.edit_name() = Some("Name".to_string());
        let mut dca2 = dca_with_chs(&[]);
        *dca2.edit_name() = Some("Name".to_string());
        let cue1 = cue_with_dcas(vec![dca1]);
        let cue2 = cue_with_dcas(vec![dca2]);
        let _: Vec<BoardEdit> = cache.cue_diff_and_apply(&cue1, Some(&names), 1, 1);
        let edits: Vec<BoardEdit> = cache.cue_diff_and_apply(&cue2, Some(&names), 1, 1);
        assert_eq!(
            edits,
            vec![
                BoardEdit::ChannelMute(ch(0), true),
                BoardEdit::ChannelDcaAssign(ch(0), dca(0), false),
            ]
        );
    }
    #[test]
    fn name_caching_cue_missing_dcas() {
        let mut cache = BoardStateCache::default();
        let names = ChannelNames::default();
        let mut dca1 = dca_with_chs(&[0]);
        *dca1.edit_name() = Some("Name".to_string());
        let mut dca2 = dca_with_chs(&[]);
        *dca2.edit_name() = Some("Name".to_string());
        let cue1 = cue_with_dcas(vec![dca1, dca2.clone()]);
        let cue2 = cue_with_dcas(vec![dca2]);
        let _: Vec<BoardEdit> = cache.cue_diff_and_apply(&cue1, Some(&names), 1, 2);
        let edits: Vec<BoardEdit> = cache.cue_diff_and_apply(&cue2, Some(&names), 1, 2);
        assert_eq!(
            edits,
            vec![
                BoardEdit::ChannelMute(ch(0), true),
                BoardEdit::ChannelDcaAssign(ch(0), dca(0), false),
                BoardEdit::DcaName(dca(1), "".to_string())
            ]
        );
    }
}
