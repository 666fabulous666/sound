//! Musical score representation and generation.
//!
//! This module defines the data structures for representing musical compositions:
//! - `Score`: Top-level container for a musical score
//! - `TrackNode`: Tree structure organizing sequences into groups
//! - `Sequence`: Individual musical sequence with rhythm and harmony rules
//! - `Note`: Individual note with timing and pitch information
//! - `NotesGroup`: Collection of notes for audio rendering
//!
//! The score system uses a tree structure where:
//! - The root is always a Group
//! - Groups can contain other Groups or Sequences
//! - Sequences generate notes based on their parameters
//! - Volume is multiplicative down the tree (parent volume × child volume)

pub mod default_params;
pub mod node_params;
pub mod note;
pub mod preset;
pub mod probability;
pub mod scheduler;
pub mod sequence;
pub mod time_quantum;
pub mod track_node;

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{
    engine::{
        score::{
            note::Note,
            probability::Probability,
            scheduler::PlaybackScheduler,
            track_node::{AestheticLocks, GroupMode, NodeKind, TrackNode},
        },
        waves::{FilterType, WaveType},
    },
    time_freq::{Freq, Tempo, Time},
    NoteIdGen, Token, TokenGen, NOTE_LINGER_TIME,
};
use arc_swap::ArcSwap;
use default_params::*;
use rand::rngs::ThreadRng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RdRythm {
    pub amount: usize,
    pub length: usize,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DetRythm {
    pub generators: Vec<usize>,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum Rythm {
    Rd(RdRythm),
    Det(DetRythm),
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct LowpassRelaxation {
    pub start: f64,
    pub end: f64,
    pub rate: f64,
}

impl Default for LowpassRelaxation {
    fn default() -> Self {
        Self {
            start: 1.0,
            end: 1.0,
            rate: 1.0,
        }
    }
}

impl LowpassRelaxation {
    pub fn factor(&self, normalized_time: f64) -> f64 {
        let alpha = normalized_time.clamp(0.0, 1.0);
        let exp = (-self.rate.max(0.0) * alpha).exp();
        self.end + (self.start - self.end) * exp
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct LowpassLfo {
    pub magnitude: f64,
    pub frequency: Freq,
    pub sync_with_clock: bool,
}

impl Default for LowpassLfo {
    fn default() -> Self {
        Self {
            magnitude: 0.0,
            frequency: Freq(0.5),
            sync_with_clock: false,
        }
    }
}

impl LowpassLfo {
    pub fn contribution(&self, reference_time: Time) -> f64 {
        if self.magnitude == 0.0 {
            0.0
        } else {
            self.magnitude * (self.frequency.phase(reference_time)).sin()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ChorusParams {
    pub voices: usize,
    pub delta: f64,
    #[serde(default = "default_delta_shift")]
    pub delta_shift: f64,
    pub sym: f64,
    pub asym: f64,
    pub time_dependency: Freq,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct HarmonicsParams {
    pub harmonics: usize,
    pub subharmonics: usize,
    pub attenuation: f64,
}

impl Default for HarmonicsParams {
    fn default() -> Self {
        Self {
            harmonics: 0,
            subharmonics: 0,
            attenuation: default_params::default_harmonics_attenuation(),
        }
    }
}
impl ChorusParams {
    pub fn new(
        number_of_heads: usize,
        delta: f64,
        delta_noise: f64,
        sym: f64,
        asym: f64,
        time_dependency: Freq,
    ) -> Self {
        Self {
            voices: number_of_heads,
            delta,
            delta_shift: delta_noise,
            sym,
            asym,
            time_dependency,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct DelayTap {
    pub beat: f64,
    #[serde(default = "default_weight")]
    pub weight: f64,
}

const fn default_weight() -> f64 {
    1.0
}

impl DelayTap {
    fn key(&self) -> u64 {
        self.beat.to_bits()
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct DelayTapSeconds {
    pub seconds: f64,
    pub weight: f64,
}

impl DelayTapSeconds {
    pub fn steps(&self, sample_rate: f64) -> usize {
        (self.seconds * sample_rate).round() as usize
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct DelayChannel {
    #[serde(default)]
    pub dry: Option<f64>,
    #[serde(default)]
    pub taps: Vec<DelayTap>,
}

impl Default for DelayChannel {
    fn default() -> Self {
        Self {
            dry: Some(0.5),
            taps: Vec::new(),
        }
    }
}

impl DelayChannel {
    fn merge_with_parent(&self, parent: &DelayChannel) -> DelayChannel {
        DelayChannel {
            dry: self.dry.or(parent.dry),
            taps: merge_taps(&parent.taps, &self.taps),
        }
    }

    fn to_seconds(&self, seconds_per_beat: f64, fallback_dry: f64) -> Vec<DelayTapSeconds> {
        let dry = self.dry.unwrap_or(fallback_dry).clamp(0.0, 1.0);
        let merged = merge_taps(&[], &self.taps);
        let total_weight: f64 = merged.iter().map(|t| t.weight.max(0.0)).sum();
        let scale = if total_weight > 0.0 {
            (1.0 - dry) / total_weight
        } else {
            0.0
        };
        merged
            .into_iter()
            .map(|t| {
                let weight = (t.weight.max(0.0)) * scale;
                DelayTapSeconds {
                    seconds: (t.beat.max(0.0)) * seconds_per_beat,
                    weight,
                }
            })
            .collect()
    }
}

fn merge_taps(parent: &[DelayTap], child: &[DelayTap]) -> Vec<DelayTap> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<u64, f64> = BTreeMap::new();

    for tap in parent.iter().chain(child.iter()) {
        let key = tap.key();
        let entry = map.entry(key).or_insert(0.0);
        *entry += tap.weight.max(0.0);
    }

    map.into_iter()
        .map(|(bits, weight)| DelayTap {
            beat: f64::from_bits(bits).max(0.0),
            weight,
        })
        .collect()
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Interval {
    /// (degree, octave)
    Tempered(i32, i32),
    /// (n rd steps, base, octave)
    RDTempered(u32, Vec<i32>, i32),
}

impl Interval {
    pub fn compute(&self) -> f64 {
        match self {
            Interval::Tempered(degree, octave) => (*degree as f64 / 12.0 + *octave as f64).exp2(),
            _ => unreachable!(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TrackDelays {
    pub left: DelayChannel,
    pub right: DelayChannel,
}

impl Default for TrackDelays {
    fn default() -> Self {
        Self {
            left: DelayChannel::default(),
            right: DelayChannel::default(),
        }
    }
}

impl TrackDelays {
    pub fn to_seconds(
        &self,
        tempo: Tempo,
        fallback_dry_left: f64,
        fallback_dry_right: f64,
    ) -> (Vec<DelayTapSeconds>, Vec<DelayTapSeconds>) {
        let seconds_per_beat = tempo.seconds_per_beat();
        (
            self.left.to_seconds(seconds_per_beat, fallback_dry_left),
            self.right.to_seconds(seconds_per_beat, fallback_dry_right),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.left.taps.is_empty() && self.right.taps.is_empty()
    }

    pub fn merge_with_parent(&self, parent: &TrackDelays) -> TrackDelays {
        TrackDelays {
            left: self.left.merge_with_parent(&parent.left),
            right: self.right.merge_with_parent(&parent.right),
        }
    }

    pub fn normalise(&mut self) {
        self.left.taps = merge_taps(&[], &self.left.taps);
        self.right.taps = merge_taps(&[], &self.right.taps);
    }
}

#[derive(Clone)]
pub struct NotesGroup {
    pub attack_decay: (f64, f64),
    pub lp_attack_decay: (f64, f64),
    pub cutoff: crate::engine::time_varying::TimeVarying,
    pub lowpass_enabled: bool,
    pub lp_order: u32,
    pub filter_type: FilterType,
    pub bend: (f64, f64),
    pub chorus: ChorusParams,
    pub harmonics: HarmonicsParams,
    pub notes: Vec<Note>,
    pub power: crate::engine::time_varying::TimeVarying,
    pub noise: crate::engine::time_varying::TimeVarying,
    pub pan: f64,
    pub tolerance: (Time, Time),
    pub vibrato: (f64, Freq),
    pub volume: f64,
    pub wave_type: WaveType,
    pub delays: (Vec<DelayTapSeconds>, Vec<DelayTapSeconds>),
}

pub struct Score {
    pub notes: BTreeMap<Token, NotesGroup>,
    pub track_root: TrackNode,
    pub last_token: TokenGen,
    pub delays: TrackDelays,
    pub tempo: Tempo,
    pub shared_notes: Arc<ArcSwap<BTreeMap<Token, NotesGroup>>>,
    note_id_gen: NoteIdGen,
    scheduler: PlaybackScheduler,
}
impl Score {
    pub fn new() -> Self {
        let mut last_token = TokenGen(0);
        let default_delays = default_delays();
        Self {
            notes: BTreeMap::new(),
            track_root: TrackNode::new_root(&mut last_token),
            last_token,
            delays: default_delays.clone(),
            tempo: default_tempo(),
            shared_notes: Arc::new(ArcSwap::from_pointee(BTreeMap::new())),
            note_id_gen: NoteIdGen::default(),
            scheduler: PlaybackScheduler::default(),
        }
        .init_root_delays(default_delays)
    }
    pub fn tempo(&self) -> Tempo {
        self.tempo
    }

    pub fn set_tempo(&mut self, tempo: Tempo, anchor_time: Time) {
        let old = self.tempo;
        if (old.beats_per_minute() - tempo.beats_per_minute()).abs() <= f64::EPSILON {
            return;
        }
        self.retime_notes(old, tempo, anchor_time);
        self.tempo = tempo;
    }

    fn init_root_delays(mut self, default_delays: TrackDelays) -> Self {
        self.track_root.delays = default_delays.clone();
        self.delays = default_delays;
        self
    }

    pub fn draw_node_with(
        &mut self,
        node: &mut TrackNode,
        rng: &mut rand::rngs::ThreadRng,
        now: Time,
    ) {
        node.draw_node(
            &mut self.notes,
            &mut self.scheduler,
            rng,
            now,
            self.tempo,
            &mut self.note_id_gen,
            track_node::MixContext::default(),
            node_params::ResolvedTrackParams::default(),
            0,
            TrackDelays::default(),
        );
        self.scheduler.tick(now);
    }

    pub fn draw_node_at_path(
        &mut self,
        path: &[usize],
        rng: &mut rand::rngs::ThreadRng,
        now: Time,
    ) {
        let inherited = if path.is_empty() {
            node_params::ResolvedTrackParams::default()
        } else {
            self.track_root
                .resolved_params_for_path(&path[..path.len() - 1])
        };
        let mix = if path.is_empty() {
            track_node::MixContext::default()
        } else {
            self.track_root
                .mix_context_for_path(&path[..path.len() - 1])
        };
        let accumulated_delays = delays_for_path(&self.track_root, path);
        if let Some(node) = self.track_root.get_mut(path) {
            node.draw_node(
                &mut self.notes,
                &mut self.scheduler,
                rng,
                now,
                self.tempo,
                &mut self.note_id_gen,
                mix,
                inherited,
                path.len(),
                accumulated_delays,
            );
        }
    }

    fn retime_notes(&mut self, old: Tempo, new: Tempo, anchor_time: Time) {
        let beat_anchor = old.time_to_beats(anchor_time);
        for notes_group in self.notes.values_mut() {
            for note in notes_group.notes.iter_mut() {
                let beat_offset = note.beat_time - beat_anchor;
                note.time = anchor_time + new.beats_to_time(beat_offset);
                note.duration = new.beats_to_time(note.beat_duration);
            }
        }
        self.track_root.for_each_sequence_mut(|seq| {
            if let Some(until) = seq.not_generate_until {
                let beat_until = old.time_to_beats(until);
                let beat_offset = beat_until - beat_anchor;
                seq.not_generate_until = Some(anchor_time + new.beats_to_time(beat_offset));
            }
        });
    }
    /// Product of volumes from the root down to (and including) the node at `path`.
    ///
    /// Computes the cumulative volume by multiplying all ancestor volumes from root to the node.
    /// Ensures the result is always finite (clamps invalid values to 0.0).
    ///
    /// # Arguments
    /// * `path` - Index path from root to target node
    ///
    /// # Returns
    /// * `Some(volume)` - The volume chain product (always finite, non-negative)
    /// * `None` - If the path is invalid (doesn't exist in tree)
    pub fn volume_chain_product(&self, path: &[usize]) -> Option<f64> {
        use crate::engine::score::track_node::TrackNode;

        // Helper: get a node's own volume, ensuring it's finite
        fn node_volume(node: &TrackNode) -> f64 {
            let v = node.volume();
            if v.is_finite() && v >= 0.0 {
                v
            } else {
                0.0 // Safe fallback for invalid volumes
            }
        }

        let mut node = &self.track_root;
        let mut product = node_volume(node); // include root

        for &idx in path {
            match &node.kind {
                NodeKind::Group { children, .. } => {
                    node = children.get(idx)?;
                    product *= node_volume(node);

                    // Ensure product stays finite
                    if !product.is_finite() {
                        product = 0.0;
                    }
                }
                NodeKind::Seq(_) => {
                    // Tried to go deeper under a Seq: invalid path
                    return None;
                }
            }
        }

        Some(product)
    }

    pub fn delays_for_path(&self, path: &[usize]) -> TrackDelays {
        delays_for_path(&self.track_root, path)
    }
    /// Return a new path pointing to the next sibling. If `wrap` is false and we’re
    /// at the last sibling, returns `None`.
    pub fn next_sibling(&self, path: &[usize], wrap: bool) -> Option<Vec<usize>> {
        if path.is_empty() {
            return None; // root has no siblings
        }
        let parent_path = &path[..path.len() - 1];
        let idx = *path.last().unwrap();

        let parent = self.track_root.get(parent_path)?;
        let n = parent.child_count();
        if n == 0 {
            return None;
        }

        let next = if idx + 1 < n {
            idx + 1
        } else if wrap {
            0
        } else {
            return None;
        };

        let mut new_path = path.to_vec();
        *new_path.last_mut().unwrap() = next;
        Some(new_path)
    }

    /// Return a new path pointing to the previous sibling. If `wrap` is false and we’re
    /// at the first sibling, returns `None`.
    pub fn prev_sibling(&self, path: &[usize], wrap: bool) -> Option<Vec<usize>> {
        if path.is_empty() {
            return None; // root has no siblings
        }
        let parent_path = &path[..path.len() - 1];
        let idx = *path.last().unwrap();

        let parent = self.track_root.get(parent_path)?;
        let n = parent.child_count();
        if n == 0 {
            return None;
        }

        let prev = if idx > 0 {
            idx - 1
        } else if wrap {
            n.saturating_sub(1)
        } else {
            return None;
        };

        let mut new_path = path.to_vec();
        *new_path.last_mut().unwrap() = prev;
        Some(new_path)
    }

    /// Optional: select parent (None if already at root).
    pub fn parent_of(&self, path: &[usize]) -> Option<Vec<usize>> {
        if path.is_empty() {
            return None;
        }
        let mut p = path.to_vec();
        p.pop();
        Some(p)
    }

    /// Optional: select first child (if any).
    pub fn first_child_of(&self, path: &[usize]) -> Option<Vec<usize>> {
        let node = self.track_root.get(path)?;
        if node.child_count() == 0 {
            return None;
        }
        let mut p = path.to_vec();
        p.push(0);
        Some(p)
    }

    /// Optional: select last child (if any).
    pub fn last_child_of(&self, path: &[usize]) -> Option<Vec<usize>> {
        let node = self.track_root.get(path)?;
        let n = node.child_count();
        if n == 0 {
            return None;
        }
        let mut p = path.to_vec();
        p.push(n - 1);
        Some(p)
    }
    pub fn swap_with_prev(&mut self, path: &[usize]) -> Option<Vec<usize>> {
        if path.is_empty() {
            return None;
        }
        let i = *path.last().unwrap();
        if i == 0 {
            return None;
        }
        let parent_path = &path[..path.len() - 1];

        let parent = self.track_root.get_mut(parent_path)?;
        if let NodeKind::Group { children, .. } = &mut parent.kind {
            children.swap(i, i - 1);
            let mut np = path.to_vec();
            *np.last_mut().unwrap() = i - 1;
            Some(np)
        } else {
            None
        }
    }

    pub fn swap_with_next(&mut self, path: &[usize]) -> Option<Vec<usize>> {
        if path.is_empty() {
            return None;
        }
        let i = *path.last().unwrap();
        let parent_path = &path[..path.len() - 1];

        let parent = self.track_root.get_mut(parent_path)?;
        if let NodeKind::Group { children, .. } = &mut parent.kind {
            if i + 1 >= children.len() {
                return None;
            }
            children.swap(i, i + 1);
            let mut np = path.to_vec();
            *np.last_mut().unwrap() = i + 1;
            Some(np)
        } else {
            None
        }
    }
    /// Wrap the node at `path` into a new Group inserted at the same index.
    /// Returns the path of the newly created group.
    pub fn wrap_into_group_at(&mut self, path: &[usize], name: String) -> Option<Vec<usize>> {
        if path.is_empty() {
            // Don't wrap the root
            return None;
        }
        let parent_path = &path[..path.len() - 1];
        let idx = *path.last().unwrap();

        // 1) Take the node out
        let node = self.track_root.remove_at(path)?;

        // 2) Build the group (adapt fields to your TrackNode::Group)
        let group = TrackNode {
            name,
            proba: Probability::default(),
            volume: 1.0,
            pan: 0.5,
            hue: 0.0,
            or_weight: 1.0,
            overrides: node_params::NodeOverrides::default(),
            delays: TrackDelays::default(),
            kind: NodeKind::Group {
                id: self.last_token.next(), // or Token(0) if you don't need unique ids
                muted: false,
                collapsed: false,
                children: vec![node],
                not_generate_until: None,
                mode: GroupMode::And,
                aesthetic: AestheticLocks::default(),
            },
        };

        // 3) Insert group back at the same position
        let mut new_path = parent_path.to_vec();
        new_path.push(idx);
        let ok = self.track_root.insert_at(&new_path, group);
        if ok {
            Some(new_path)
        } else {
            None
        }
    }
}
impl Score {
    /// Core helper: move `path` into an adjacent sibling group.
    /// `dir = -1` → previous sibling; `dir = +1` → next sibling.
    /// Returns the new path of the moved node (inside the target group).
    fn move_into_adjacent_group(&mut self, path: &[usize], dir: isize) -> Option<Vec<usize>> {
        if path.is_empty() {
            return None;
        }
        let parent = path[..path.len() - 1].to_vec();
        let idx = *path.last().unwrap();

        // Sibling bounds
        let siblings = self.track_root.get(&parent)?.child_count();
        if dir == -1 {
            if idx == 0 {
                return None;
            }
        } else if dir == 1 {
            if idx + 1 >= siblings {
                return None;
            }
        } else {
            return None; // only -1 or +1 supported
        }

        // Path to target sibling BEFORE removal
        let mut target_before = parent.clone();
        let target_idx_before = if dir == -1 { idx - 1 } else { idx + 1 };
        target_before.push(target_idx_before);

        // Must be a group
        let is_group = self
            .track_root
            .get(&target_before)
            .map(|node| node.is_group())
            .unwrap_or(false);
        if !is_group {
            return None;
        }

        // Remove the source node
        let moved = self.track_root.remove_at(path)?;

        // After removal:
        //  - moving to previous: target index unchanged (idx-1 stays idx-1)
        //  - moving to next: the old "next" shifts left by 1 → now at `idx`
        let mut target_after = parent.clone();
        let target_idx_after = if dir == -1 { idx - 1 } else { idx };
        target_after.push(target_idx_after);

        // Push moved into that group
        if let Some(target_node) = self.track_root.get_mut(&target_after) {
            if let NodeKind::Group { children, .. } = &mut target_node.kind {
                children.push(moved);
                let new_leaf = children.len() - 1;
                let mut new_path = target_after;
                new_path.push(new_leaf);
                return Some(new_path);
            }
        }

        // Shouldn't happen; best-effort restore
        let mut restore = parent;
        restore.push(idx.min(self.track_root.get(&restore)?.child_count()));
        let _ = self.track_root.insert_at(&restore, moved);
        None
    }

    /// Move the node at `path` into the previous sibling group.
    pub fn move_into_prev_group(&mut self, path: &[usize]) -> Option<Vec<usize>> {
        self.move_into_adjacent_group(path, -1)
    }

    /// Move the node at `path` into the next sibling group.
    pub fn move_into_next_group(&mut self, path: &[usize]) -> Option<Vec<usize>> {
        self.move_into_adjacent_group(path, 1)
    }

    /// Move the node at `path` up one rank in the tree:
    /// - Remove it from its parent
    /// - Insert it into the grandparent, right *after* the parent
    /// Returns the new path of the moved node.
    pub fn promote_one_rank(&mut self, path: &[usize]) -> Option<Vec<usize>> {
        // Need at least grandparent/parent/current: [.., parent_idx, child_idx]
        if path.len() < 2 {
            return None; // already at root -> no-op
        }

        let parent_path = &path[..path.len() - 1];
        let grand_path = &path[..path.len() - 2];
        let parent_idx = *parent_path.last().unwrap();

        // Remove the node from its parent
        let node = self.track_root.remove_at(path)?;

        // Insert into grandparent right after the parent
        let insert_idx_in_grand = parent_idx + 1;
        let mut new_path = grand_path.to_vec();
        new_path.push(insert_idx_in_grand);

        // Insert the moved node
        let _ok = self.track_root.insert_at(&new_path, node);

        Some(new_path)
    }
    /// Dissolve the group at `path`:
    /// - If `path` points to a `Group`, remove that group and splice all its children
    ///   into the parent at the same index (preserving order).
    /// - If it's not a `Group`, do nothing.
    /// Returns the path of the first inserted child (for convenient selection),
    /// or `None` if the group was empty / invalid path / not a group.
    pub fn dissolve_group_at(&mut self, path: &[usize]) -> Option<Vec<usize>> {
        // Root cannot be dissolved and must have at least a parent/index
        if path.is_empty() {
            return None;
        }

        let parent_path = &path[..path.len() - 1];
        let idx = *path.last().unwrap();

        // Get mutable access to the parent group's children
        let parent = self.track_root.get_mut(parent_path)?;
        let NodeKind::Group {
            children: parent_children,
            ..
        } = &mut parent.kind
        else {
            // Parent must be a group/root-group to splice into
            return None;
        };

        // Bounds check
        if idx >= parent_children.len() {
            return None;
        }

        // Take the node at `idx`
        let mut node = parent_children.remove(idx);

        // Only act if it is a Group
        if let NodeKind::Group { children, .. } = &mut node.kind {
            if children.is_empty() {
                // Nothing to splice; group was empty
                return None;
            }

            // Splice its children back into parent at the original index, preserving order
            let first_insert = idx;
            for (k, ch) in children.drain(..).enumerate() {
                parent_children.insert(first_insert + k, ch);
            }

            // Return the path to the first child now at `idx`
            let mut new_path = parent_path.to_vec();
            new_path.push(first_insert);
            Some(new_path)
        } else {
            // Not a group: put it back where it was (no-op externally)
            parent_children.insert(idx, node);
            None
        }
    }

    /// Move the node located at `source_path` into `target_parent` at `target_index`.
    /// Returns the new path of the moved node on success.
    pub fn move_node_to(
        &mut self,
        source_path: &[usize],
        target_parent: &[usize],
        target_index: usize,
    ) -> Option<Vec<usize>> {
        if source_path.is_empty() || target_parent.starts_with(source_path) {
            // Never move the root or into one of our own descendants.
            return None;
        }

        let src_parent: Vec<usize> = source_path[..source_path.len() - 1].to_vec();
        let src_idx = *source_path.last().unwrap();

        // Dropping immediately before/after itself is a no-op.
        if target_parent == src_parent && (target_index == src_idx || target_index == src_idx + 1) {
            return None;
        }

        let mut insert_parent = target_parent.to_vec();
        adjust_path_for_removal(&mut insert_parent, source_path);

        let node = self.track_root.remove_at(source_path)?;

        let parent = self.track_root.get_mut(&insert_parent)?;
        let NodeKind::Group { children, .. } = &mut parent.kind else {
            return None;
        };

        let insert_idx = target_index.min(children.len());
        children.insert(insert_idx, node);
        let mut new_path = insert_parent;
        new_path.push(insert_idx);
        Some(new_path)
    }
    // /// Draw the node at `path` (recursively if it's a Group).
    // pub fn draw_node_at(&mut self, path: &[usize], now: Time, rng: &mut ThreadRng) {
    //     let volume_opt = self.volume_chain_product(path);
    //     if let Some(node) = self.track_root.get_mut(path) {
    //         node.draw_node(&mut self.notes, rng, now, true, volume_opt.unwrap());
    //     }
    // }
    pub fn retain_notes(&mut self, now: Time) {
        self.notes
            .values_mut()
            .for_each(|ng| ng.notes.retain(|n| n.time + NOTE_LINGER_TIME >= now));
    }

    pub fn refresh_notes_for_path(&mut self, path: &[usize]) {
        let inherited = if path.is_empty() {
            node_params::ResolvedTrackParams::default()
        } else {
            self.track_root
                .resolved_params_for_path(&path[..path.len() - 1])
        };
        if let Some(node) = self.track_root.get(path) {
            let mut updates = Vec::new();
            node.collect_sequences_with_params(inherited, path.len(), &mut updates);
            for (token, params) in updates {
                if let Some(group) = self.notes.get_mut(&token) {
                    apply_params_to_notes_group(group, &params);
                }
            }
        }
    }

    pub fn generate_notes(&mut self, now: Time, rng: &mut ThreadRng) {
        self.track_root.draw_node(
            &mut self.notes,
            &mut self.scheduler,
            rng,
            now,
            self.tempo,
            &mut self.note_id_gen,
            track_node::MixContext::default(),
            node_params::ResolvedTrackParams::default(),
            0,
            TrackDelays::default(),
        );
    }
}

fn adjust_path_for_removal(path: &mut Vec<usize>, removed: &[usize]) {
    if removed.is_empty() {
        return;
    }
    let parent_depth = removed.len() - 1;
    if path.len() <= parent_depth {
        return;
    }
    if path[..parent_depth] != removed[..parent_depth] {
        return;
    }
    if path[parent_depth] > removed[parent_depth] {
        path[parent_depth] -= 1;
    }
}

fn apply_params_to_notes_group(ng: &mut NotesGroup, params: &node_params::ResolvedTrackParams) {
    ng.bend = (params.bend.magnitude, params.bend.speed);
    ng.vibrato = (params.vibrato.magnitude, params.vibrato.frequency);
    ng.chorus = params.chorus.clone();
    ng.harmonics = params.harmonics.clone();
    ng.attack_decay = (params.envelope.attack, params.envelope.decay);
    ng.lp_attack_decay = params.lowpass.envelope;
    ng.power = params.power.power;
    ng.noise = params.noise.noise;
    ng.wave_type = params.wave.wave;
    ng.cutoff = params.lowpass.cutoff;
    ng.lowpass_enabled = params.lowpass.enabled;
    ng.lp_order = params.lowpass.order;
}

fn delays_for_path(root: &TrackNode, path: &[usize]) -> TrackDelays {
    let mut delays = TrackDelays::default().merge_with_parent(&root.delays);
    let mut node = root;
    for &idx in path {
        match &node.kind {
            track_node::NodeKind::Group { children, .. } => {
                if let Some(ch) = children.get(idx) {
                    delays = ch.delays.merge_with_parent(&delays);
                    node = ch;
                } else {
                    break;
                }
            }
            track_node::NodeKind::Seq(_) => break,
        }
    }
    delays
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::score::node_params::BendParams;
    use crate::engine::score::sequence::Sequence;

    #[test]
    fn track_delays_default_empty_for_track_nodes() {
        let mut gen = TokenGen::default();
        let node = TrackNode::from_sequence(Sequence::new(gen.next()));
        assert!(node.delays.is_empty(), "Track delays should start empty");
    }

    #[test]
    fn default_delays_convert_with_tempo() {
        let delays = default_params::default_delays();
        let tempo = Tempo::new(120.0);
        let seconds = delays.to_seconds(tempo, 0.5, 0.5);
        // 0.031 beats at 120 BPM = 0.0155s
        let first = seconds.0.first().map(|tap| tap.seconds).unwrap_or(0.0);
        assert!((first - 0.0155).abs() < 1e-4);
    }

    #[test]
    fn test_volume_chain_product_root() {
        let score = Score::new();
        // Root path (empty) should return root volume
        let vol = score.volume_chain_product(&[]);
        assert!(vol.is_some());
        let vol = vol.unwrap();
        assert!(vol.is_finite(), "Root volume should be finite");
        assert!(vol > 0.0, "Root volume should be positive");
    }

    #[test]
    fn test_volume_chain_product_invalid_path() {
        let score = Score::new();
        // Invalid path should return None
        let vol = score.volume_chain_product(&[99]);
        assert!(vol.is_none(), "Invalid path should return None");
    }

    #[test]
    fn test_volume_chain_product_always_finite() {
        let mut score = Score::new();

        // Add a child to root
        let seq = Sequence::new(score.last_token.next());
        score.track_root.push_child(TrackNode::from_sequence(seq));

        // Test path [0] (first child)
        let vol = score.volume_chain_product(&[0]);
        assert!(vol.is_some());
        let vol = vol.unwrap();
        assert!(
            vol.is_finite(),
            "Volume chain product should always be finite"
        );
        assert!(vol >= 0.0, "Volume should be non-negative");
    }

    #[test]
    fn test_score_new_has_valid_state() {
        let score = Score::new();

        // Check that initial state is valid
        assert!(score.notes.is_empty(), "New score should have no notes");
        assert_eq!(score.delays.left.taps.len(), 3, "Should have 3 left delays");
        assert_eq!(
            score.delays.right.taps.len(),
            3,
            "Should have 3 right delays"
        );

        // All delays should be positive and finite with non-negative weights
        for delay in &score.delays.left.taps {
            assert!(delay.beat.is_finite() && delay.beat > 0.0);
            assert!(delay.weight.is_finite() && delay.weight >= 0.0);
        }
        for delay in &score.delays.right.taps {
            assert!(delay.beat.is_finite() && delay.beat > 0.0);
            assert!(delay.weight.is_finite() && delay.weight >= 0.0);
        }
    }

    #[test]
    fn test_parameter_inheritance_prefers_parent_override() {
        let mut score = Score::new();
        let mut parent = TrackNode::new_node(&mut score.last_token, "group");
        parent.overrides.bend = Some(BendParams {
            magnitude: 0.75,
            speed: 12.0,
        });
        let seq = Sequence::new(score.last_token.next());
        let child = TrackNode::from_sequence(seq);
        if let NodeKind::Group { children, .. } = &mut parent.kind {
            children.push(child);
        }
        score.track_root = parent;

        let resolution = score.track_root.resolve_bend(&[0]);
        assert_eq!(resolution.source_depth, Some(0));
        assert!((resolution.value.magnitude - 0.75).abs() < f64::EPSILON);
        assert!((resolution.value.speed - 12.0).abs() < f64::EPSILON);
    }
}
