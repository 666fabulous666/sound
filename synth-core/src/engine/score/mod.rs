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
pub mod note;
pub mod sequence;
pub mod track_node;

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{
    engine::{
        score::{
            note::Note,
            track_node::{NodeKind, TrackNode},
        },
        waves::WaveType,
    },
    time_freq::{Freq, Time},
    Token, TokenGen, NOTE_LINGER_TIME,
};
use arc_swap::ArcSwap;
use default_params::*;
use rand::rngs::ThreadRng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct RdRythm {
    pub amount: usize,
    pub length: usize,
}
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct DetRythm {
    pub generators: Vec<usize>,
}
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Rythm {
    Rd(RdRythm),
    Det(DetRythm),
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ChorusParams {
    pub voices: usize,
    pub delta: f64,
    #[serde(default = "default_delta_shift")]
    pub delta_shift: f64,
    pub sym: f64,
    pub asym: f64,
    pub time_dependency: Freq,
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
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

#[derive(Clone)]
pub struct NotesGroup {
    pub attack_decay: (f64, f64),
    pub lp_attack_decay: (f64, f64),
    pub cutoff_multiplier: f64,
    pub lowpass_enabled: bool,
    pub lp_order: u32,
    pub bend: (f64, f64),
    pub chorus: ChorusParams,
    pub notes: Vec<Note>,
    pub pow_fact: (f64, Freq),
    pub pan: f64,
    pub tolerance: (Time, Time),
    pub vibrato: (f64, Freq),
    pub volume: f64,
    pub wave_type: WaveType,
}

pub struct Score {
    pub notes: BTreeMap<Token, NotesGroup>,
    pub track_root: TrackNode,
    pub last_token: TokenGen,
    pub delays: (Vec<f64>, Vec<f64>),
    pub shared_notes: Arc<ArcSwap<BTreeMap<Token, NotesGroup>>>,
}
impl Score {
    pub fn new() -> Self {
        let mut last_token = TokenGen(0);
        Self {
            notes: BTreeMap::new(),
            track_root: TrackNode::new_root(&mut last_token),
            last_token,
            delays: default_delays(),
            shared_notes: Arc::new(ArcSwap::from_pointee(BTreeMap::new())),
        }
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
            proba: 1.0,
            volume: 1.0,
            pan: 0.5,
            hue: 0.0,
            kind: NodeKind::Group {
                id: self.last_token.next(), // or Token(0) if you don't need unique ids
                muted: false,
                collapsed: false,
                children: vec![node],
                not_generate_until: None,
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
    // /// Draw the node at `path` (recursively if it's a Group).
    // pub fn draw_node_at(&mut self, path: &[usize], now: Time, rng: &mut ThreadRng) {
    //     let volume_opt = self.volume_chain_product(path);
    //     if let Some(node) = self.track_root.get_mut(path) {
    //         node.draw_node(&mut self.notes, rng, now, true, volume_opt.unwrap());
    //     }
    // }
    pub fn retain_notes(&mut self, now: Time) {
        self.notes.values_mut().for_each(|ng| {
            ng.notes.retain(|n| n.time + NOTE_LINGER_TIME >= now)
        });
    }

    pub fn generate_notes(&mut self, now: Time, rng: &mut ThreadRng) {
        self.track_root.draw_node(&mut self.notes, rng, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::score::sequence::Sequence;

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
        assert!(vol.is_finite(), "Volume chain product should always be finite");
        assert!(vol >= 0.0, "Volume should be non-negative");
    }

    #[test]
    fn test_score_new_has_valid_state() {
        let score = Score::new();

        // Check that initial state is valid
        assert!(score.notes.is_empty(), "New score should have no notes");
        assert_eq!(score.delays.0.len(), 3, "Should have 3 left delays");
        assert_eq!(score.delays.1.len(), 3, "Should have 3 right delays");

        // All delays should be positive and finite
        for &delay in &score.delays.0 {
            assert!(delay.is_finite() && delay > 0.0, "Delay should be positive and finite");
        }
        for &delay in &score.delays.1 {
            assert!(delay.is_finite() && delay > 0.0, "Delay should be positive and finite");
        }
    }
}

