use std::collections::{BTreeMap, VecDeque};

use crate::{
    engine::score::{sequence::Sequence, NotesGroup},
    time_freq::{Beat, Time},
    Token,
};

/// Captures a copy of the generated notes so we can reuse them when freezing.
#[derive(Clone, Default)]
pub struct CachedNotes {
    pub notes: Vec<crate::engine::score::note::Note>,
    pub loop_len: Beat,
}

/// Runtime status for a sequence (leaf).
#[derive(Clone, Default)]
pub struct SequencePlaybackState {
    pub cached: Option<CachedNotes>,
    pub next_window_start: Time,
    pub frozen: bool,
    pub busy_until: Time,
}

impl SequencePlaybackState {
    pub fn is_idle(&self, now: Time) -> bool {
        now >= self.busy_until
    }
}

/// Runtime status for a group node.
#[derive(Clone, Default)]
pub struct GroupPlaybackState {
    pub pending_children: VecDeque<usize>,
}

/// Central scheduler placeholder.
#[derive(Default)]
pub struct PlaybackScheduler {
    pub sequence_states: BTreeMap<Token, SequencePlaybackState>,
    pub group_states: BTreeMap<Token, GroupPlaybackState>,
}

impl PlaybackScheduler {
    pub fn tick(&mut self, _now: Time) {
        // placeholder for future state machine updates
    }

    pub fn sequence_state_mut(&mut self, token: Token) -> &mut SequencePlaybackState {
        self.sequence_states
            .entry(token)
            .or_insert_with(SequencePlaybackState::default)
    }

    pub fn sequence_state(&self, token: &Token) -> Option<&SequencePlaybackState> {
        self.sequence_states.get(token)
    }

    pub fn register_sequence_snapshot(&mut self, sequence: &Sequence, notes_group: &NotesGroup) {
        let state = self
            .sequence_states
            .entry(sequence.token)
            .or_insert_with(SequencePlaybackState::default);
        state.cached = Some(CachedNotes {
            notes: notes_group.notes.clone(),
            loop_len: sequence.loop_len,
        });
    }

    pub fn clear_sequence(&mut self, token: Token) {
        self.sequence_states.remove(&token);
    }
}
