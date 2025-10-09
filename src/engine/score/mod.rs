pub mod default_params;
pub mod note;
pub mod sequence;
pub mod track_node;

use std::sync::Arc;

use crate::{
    engine::{
        score::{note::Note, track_node::TrackNode},
        waves::WaveType,
    },
    time_freq::{Freq, Time},
    Token, TokenGen,
};
use arc_swap::ArcSwap;
use default_params::*;
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

#[derive(Serialize, Deserialize, Clone, PartialEq)]
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
    pub bend: (f64, f64),
    pub chorus: ChorusParams,
    pub notes: Vec<Note>,
    pub pow_fact: (f64, Freq),
    pub spacial: f64,
    pub token: Token,
    pub tolerance: (Time, Time),
    pub vibrato: (f64, Freq),
    pub volume: f64,
    pub wave_type: WaveType,
}

pub struct Score {
    pub notes: Vec<NotesGroup>,
    pub track_root: TrackNode,
    pub last_token: TokenGen,
    pub delays: (Vec<f64>, Vec<f64>),
    pub shared_notes: Arc<ArcSwap<Vec<NotesGroup>>>,
}
impl Score {
    pub fn new() -> Self {
        let mut last_token = TokenGen(0);
        Self {
            notes: Vec::new(),
            track_root: TrackNode::new_root(&mut last_token),
            last_token,
            delays: default_delays(),
            shared_notes: Arc::new(ArcSwap::from_pointee(Vec::new())),
        }
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
        if let TrackNode::Group { children, .. } = parent {
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
        if let TrackNode::Group { children, .. } = parent {
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
        let group = TrackNode::Group {
            id: self.last_token.next(), // or Token(0) if you don't need unique ids
            name,
            muted: false,
            volume: 1.0,
            spacial: 0.5,
            collapsed: false,
            children: vec![node],
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
