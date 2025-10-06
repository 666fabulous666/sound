use serde::{Deserialize, Serialize};

use crate::{engine::score::sequence::Sequence, Token, TokenGen};

#[derive(Clone, Serialize, Deserialize)]
pub enum TrackNode {
    // Group {
    //     id: Token,
    //     name: String,
    //     muted: bool,
    //     volume: f64,  // mix gain multiplier (>= 0.0)
    //     spacial: f64, // pan 0.0..=1.0 (0 = L, 0.5 = C, 1 = R)
    //     children: Vec<TrackNode>,
    // },
    Seq(Sequence),
}

impl TrackNode {
    // ---------- Constructors ----------
    // /// Root group with safe defaults.
    // pub fn new_root(gen: &mut TokenGen) -> Self {
    //     Self::new_group(gen, "Root")
    // }

    // /// New group with a fresh Token id and safe defaults.
    // pub fn new_group(gen: &mut TokenGen, name: impl Into<String>) -> Self {
    //     TrackNode::Group {
    //         id: gen.next(),
    //         name: name.into(),
    //         muted: false,
    //         volume: 1.0,
    //         spacial: 0.5,
    //         children: Vec::new(),
    //     }
    // }

    /// Wrap a Sequence node.
    pub fn from_sequence(seq: Sequence) -> Self {
        TrackNode::Seq(seq)
    }

    // // ---------- Queries ----------
    // pub fn is_group(&self) -> bool {
    //     matches!(self, TrackNode::Group { .. })
    // }

    pub fn name(&self) -> &str {
        match self {
            // TrackNode::Group { name, .. } => name,
            TrackNode::Seq(s) => &s.name,
        }
    }

    pub fn child_count(&self) -> usize {
        match self {
            // TrackNode::Group { children, .. } => children.len(),
            TrackNode::Seq(_) => 0,
        }
    }

    // ---------- Safe mutations on groups ----------
    /// Push a child at the end; returns its index.
    pub fn push_child(&mut self, child: TrackNode) -> Option<usize> {
        match self {
            // TrackNode::Group { children, .. } => {
            //     children.push(child);
            //     Some(children.len() - 1)
            // }
            _ => None,
        }
    }

    /// Insert a child at index; returns true on success.
    pub fn insert_child(&mut self, index: usize, child: TrackNode) -> bool {
        match self {
            // TrackNode::Group { children, .. } => {
            //     if index <= children.len() {
            //         children.insert(index, child);
            //         return true;
            //     }
            // }
            _ => (),
        }
        false
    }

    /// Clamp mix params to sane ranges.
    pub fn clamp_mix(&mut self) {
        match self {
            // TrackNode::Group {
            //     volume, spacial, ..
            // } => {
            //     *volume = volume.max(0.0);
            //     *spacial = spacial.clamp(0.0, 1.0);
            // }
            _ => (),
        }
    }

    // ---------- Path-based navigation & edits ----------
    pub fn get<'a>(&'a self, path: &[usize]) -> Option<&'a TrackNode> {
        let mut cur = self;
        for &ix in path {
            match cur {
                // TrackNode::Group { children, .. } => cur = children.get(ix)?,
                TrackNode::Seq(_) => return None,
            }
        }
        Some(cur)
    }

    pub fn get_mut<'a>(&'a mut self, path: &[usize]) -> Option<&'a mut TrackNode> {
        let mut cur = self;
        for &ix in path {
            match cur {
                // TrackNode::Group { children, .. } => cur = children.get_mut(ix)?,
                TrackNode::Seq(_) => return None,
            }
        }
        Some(cur)
    }

    pub fn remove_at(&mut self, path: &[usize]) -> Option<TrackNode> {
        match (path.split_first(), self) {
            // (Some((&ix, rest)), TrackNode::Group { children, .. }) if rest.is_empty() => {
            //     if ix < children.len() {
            //         Some(children.remove(ix))
            //     } else {
            //         None
            //     }
            // }
            // (Some((&ix, rest)), TrackNode::Group { children, .. }) => {
            //     children.get_mut(ix)?.remove_at(rest)
            // }
            _ => None,
        }
    }

    pub fn insert_at(&mut self, path: &[usize], node: TrackNode) -> bool {
        let Some((&ix, parent_path)) = path.split_last() else {
            return false;
        };
        let parent: &mut TrackNode = if parent_path.is_empty() {
            self
        } else {
            match self.get_mut(parent_path) {
                Some(p) => p,
                None => return false,
            }
        };
        match parent {
            // TrackNode::Group { children, .. } => {
            //     if ix <= children.len() {
            //                 children.insert(ix, node);
            //                 return true;
            //             }
            // }
            _ => (),
        }
        false
    }
}

// Flatten for engine playback/mix
pub fn collect_sequences<'a>(
    node: &'a TrackNode,
    acc: &mut Vec<(&'a Sequence, f64, f64)>, // (seq, vol, spacial)
    parent_vol: f64,
    parent_spacial: f64,
) {
    match node {
        TrackNode::Seq(s) => acc.push((s, parent_vol, parent_spacial)),
        // TrackNode::Group {
        //     muted,
        //     volume,
        //     spacial,
        //     children,
        //     ..
        // } => {
        //     if *muted {
        //         return;
        //     }
        //     let vol = parent_vol * *volume;
        //     let pan = parent_spacial + parent_spacial.min(1.0 - parent_spacial) * spacial; //NOTE: might not be the good approach
        //     for ch in children {
        //         collect_sequences(ch, acc, vol, pan);
        //     }
        // }
    }
}
