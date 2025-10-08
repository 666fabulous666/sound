use core::marker::PhantomData;
use serde::{Deserialize, Serialize};

use crate::{engine::score::sequence::Sequence, Token, TokenGen};

#[derive(Clone, Serialize, Deserialize)]
pub enum TrackNode {
    Group {
        id: Token,
        name: String,
        muted: bool,
        volume: f64,  // mix gain multiplier (>= 0.0)
        spacial: f64, // pan 0.0..=1.0 (0 = L, 0.5 = C, 1 = R)
        children: Vec<TrackNode>,
    },
    Seq(Sequence),
}

impl TrackNode {
    pub fn as_seq(&self) -> Option<&Sequence> {
        if let TrackNode::Seq(s) = self {
            Some(s)
        } else {
            None
        }
    }
    pub fn as_seq_mut(&mut self) -> Option<&mut Sequence> {
        if let TrackNode::Seq(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn seq_unchecked(&self) -> &Sequence {
        self.as_seq().expect("TrackNode::Seq expected")
    }
    pub fn seq_mut_unchecked(&mut self) -> &mut Sequence {
        self.as_seq_mut().expect("TrackNode::Seq expected")
    }

    // ---------- Constructors ----------
    /// Root group with safe defaults.
    pub fn new_root(gen: &mut TokenGen) -> Self {
        Self::new_group(gen, "Root")
    }

    /// New group with a fresh Token id and safe defaults.
    pub fn new_group(gen: &mut TokenGen, name: impl Into<String>) -> Self {
        TrackNode::Group {
            id: gen.next(),
            name: name.into(),
            muted: false,
            volume: 1.0,
            spacial: 0.5,
            children: Vec::new(),
        }
    }

    /// Wrap a Sequence node.
    pub fn from_sequence(seq: Sequence) -> Self {
        TrackNode::Seq(seq)
    }

    // // ---------- Queries ----------
    pub fn is_group(&self) -> bool {
        matches!(self, TrackNode::Group { .. })
    }

    pub fn name(&self) -> &str {
        match self {
            TrackNode::Group { name, .. } => name,
            TrackNode::Seq(s) => &s.name,
        }
    }

    pub fn child_count(&self) -> usize {
        match self {
            TrackNode::Group { children, .. } => children.len(),
            TrackNode::Seq(_) => 0,
        }
    }

    // ---------- Safe mutations on groups ----------
    /// Push a child at the end; returns its index.
    pub fn push_child(&mut self, child: TrackNode) -> Option<usize> {
        match self {
            TrackNode::Group { children, .. } => {
                children.push(child);
                Some(children.len() - 1)
            }
            _ => None,
        }
    }

    /// Insert a child at index; returns true on success.
    pub fn insert_child(&mut self, index: usize, child: TrackNode) -> bool {
        match self {
            TrackNode::Group { children, .. } => {
                if index <= children.len() {
                    children.insert(index, child);
                    return true;
                }
            }
            _ => (),
        }
        false
    }

    /// Clamp mix params to sane ranges.
    pub fn clamp_mix(&mut self) {
        match self {
            TrackNode::Group {
                volume, spacial, ..
            } => {
                *volume = volume.max(0.0);
                *spacial = spacial.clamp(0.0, 1.0);
            }
            _ => (),
        }
    }

    // ---------- Path-based navigation & edits ----------
    pub fn get<'a>(&'a self, path: &[usize]) -> Option<&'a TrackNode> {
        let mut cur = self;
        for &ix in path {
            match cur {
                TrackNode::Group { children, .. } => cur = children.get(ix)?,
                TrackNode::Seq(_) => return None,
            }
        }
        Some(cur)
    }

    pub fn get_mut<'a>(&'a mut self, path: &[usize]) -> Option<&'a mut TrackNode> {
        let mut cur = self;
        for &ix in path {
            match cur {
                TrackNode::Group { children, .. } => cur = children.get_mut(ix)?,
                TrackNode::Seq(_) => return None,
            }
        }
        Some(cur)
    }

    pub fn remove_at(&mut self, path: &[usize]) -> Option<TrackNode> {
        match (path.split_first(), self) {
            (Some((&ix, rest)), TrackNode::Group { children, .. }) if rest.is_empty() => {
                if ix < children.len() {
                    Some(children.remove(ix))
                } else {
                    None
                }
            }
            (Some((&ix, rest)), TrackNode::Group { children, .. }) => {
                children.get_mut(ix)?.remove_at(rest)
            }
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
            TrackNode::Group { children, .. } => {
                if ix <= children.len() {
                    children.insert(ix, node);
                    return true;
                }
            }
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
        TrackNode::Group {
            muted,
            volume,
            spacial,
            children,
            ..
        } => {
            if *muted {
                return;
            }
            let vol = parent_vol * *volume;
            let pan = parent_spacial + parent_spacial.min(1.0 - parent_spacial) * spacial; //NOTE: might not be the good approach
            for ch in children {
                collect_sequences(ch, acc, vol, pan);
            }
        }
    }
}

// =====================
// Read-only iterator
// =====================

impl TrackNode {
    /// Depth-first iterator over all leaf Sequences.
    pub fn sequences(&self) -> SequencesIter<'_> {
        SequencesIter { stack: vec![self] }
    }
}

pub struct SequencesIter<'a> {
    stack: Vec<&'a TrackNode>,
}

impl<'a> Iterator for SequencesIter<'a> {
    type Item = &'a Sequence;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match node {
                TrackNode::Seq(s) => return Some(s),
                TrackNode::Group { children, .. } => {
                    // push in reverse so we visit children in original order
                    for ch in children.iter().rev() {
                        self.stack.push(ch);
                    }
                }
            }
        }
        None
    }
}

// =====================
// Mutable traversal
// =====================

impl TrackNode {
    /// Apply a function to every Sequence (leaf) in depth-first order.
    /// Entirely safe, no `unsafe` needed.
    pub fn for_each_sequence_mut(&mut self, mut f: impl FnMut(&mut Sequence)) {
        fn walk(node: &mut TrackNode, f: &mut impl FnMut(&mut Sequence)) {
            match node {
                TrackNode::Seq(s) => f(s),
                TrackNode::Group { children, .. } => {
                    for ch in children {
                        walk(ch, f);
                    }
                }
            }
        }
        walk(self, &mut f);
    }

    /// Depth-first iterator over all leaf Sequences (mutable).
    ///
    /// Internally collects raw pointers and yields them immediately as `&mut Sequence`.
    /// This is fine for immediate iteration (don’t store the returned references).
    pub fn sequences_mut(&mut self) -> SequencesIterMut<'_> {
        let mut ptrs: Vec<*mut Sequence> = Vec::new();

        fn collect(node: &mut TrackNode, out: &mut Vec<*mut Sequence>) {
            match node {
                TrackNode::Seq(s) => out.push(s as *mut Sequence),
                TrackNode::Group { children, .. } => {
                    for ch in children {
                        collect(ch, out);
                    }
                }
            }
        }

        collect(self, &mut ptrs);
        SequencesIterMut {
            inner: ptrs.into_iter(),
            _marker: PhantomData,
        }
    }
}

pub struct SequencesIterMut<'a> {
    inner: std::vec::IntoIter<*mut Sequence>,
    _marker: PhantomData<&'a mut Sequence>,
}

impl<'a> Iterator for SequencesIterMut<'a> {
    type Item = &'a mut Sequence;

    fn next(&mut self) -> Option<Self::Item> {
        let p = self.inner.next()?;
        // SAFETY: All pointers were collected from a unique &mut TrackNode,
        // and this iterator yields each exactly once during this call site.
        // Do not store the returned references beyond the borrow of `&mut self`.
        unsafe { Some(&mut *p) }
    }
}
impl TrackNode {
    /// Depth-first iterator over all leaf `Sequence`s, yielding `(path, &Sequence)`.
    /// `path` is the list of child indices from the root to that leaf.
    pub fn sequences_with_paths(&self) -> SequencesWithPathIter<'_> {
        SequencesWithPathIter {
            // start with the root at the empty path
            stack: vec![(self, Vec::new())],
        }
    }
}

/// DFS iterator over `(path, &Sequence)`.
pub struct SequencesWithPathIter<'a> {
    // Stack of nodes to visit next, each with its path from the root.
    // We push children in reverse order so the leftmost child is visited first.
    stack: Vec<(&'a TrackNode, Vec<usize>)>,
}

impl<'a> Iterator for SequencesWithPathIter<'a> {
    type Item = (Vec<usize>, &'a Sequence);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node, path)) = self.stack.pop() {
            match node {
                TrackNode::Seq(seq) => {
                    // Leaf: yield (path, &Sequence)
                    return Some((path, seq));
                }
                TrackNode::Group { children, .. } => {
                    // Internal: push children (right-to-left) with extended paths
                    for (i, child) in children.iter().enumerate().rev() {
                        let mut child_path = path.clone();
                        child_path.push(i);
                        self.stack.push((child, child_path));
                    }
                    // loop to pop the next item
                }
            }
        }
        None
    }
}
use std::fmt::Write as _;

/// Customize how the tree is printed.
#[derive(Clone, Copy)]
pub struct TreePrintOptions {
    /// Use only ASCII characters (`+--`, `|  `) instead of Unicode (`├──`, `│  `).
    pub ascii_only: bool,
    /// Show the index path like `[0,1,2]` for each node.
    pub show_path: bool,
    /// Show extra details for sequences/groups.
    pub show_details: bool,
}

impl Default for TreePrintOptions {
    fn default() -> Self {
        Self {
            ascii_only: true,
            show_path: true,
            show_details: true,
        }
    }
}

/// Return a string that looks like the `tree` command.
///
/// Example:
/// ```text
/// Root (3)
/// ├── Group "Drums" (2)
/// │   ├── Seq token=7 loop=4.00s
/// │   └── Seq token=8 loop=4.00s
/// └── Seq token=12 loop=8.00s
/// ```
pub fn tracknode_to_tree(root: &TrackNode, opts: TreePrintOptions) -> String {
    let mut out = String::new();

    // Top line (no connector)
    write!(out, "{}", node_label(root, &[], opts)).ok();
    out.push('\n');

    if let Some(children) = children_of(root) {
        for (i, child) in children.iter().enumerate() {
            let last = i + 1 == children.len();
            let mut path = vec![i];
            fmt_tree_rec(child, &mut out, &mut path, "", last, opts);
        }
    }

    out
}

/// Print directly to stdout (convenience).
pub fn print_tracknode_tree(root: &TrackNode, opts: TreePrintOptions) {
    println!("{}", tracknode_to_tree(root, opts));
}

// -------------------- internals --------------------

fn fmt_tree_rec(
    node: &TrackNode,
    out: &mut String,
    path: &mut Vec<usize>,
    parent_prefix: &str,
    is_last: bool,
    opts: TreePrintOptions,
) {
    let (tee, elb, bar, sp) = if opts.ascii_only {
        ("+-- ", "'-- ", "|   ", "    ")
    } else {
        ("├── ", "└── ", "│   ", "    ")
    };

    // Line prefix + connector
    out.push_str(parent_prefix);
    out.push_str(if is_last { elb } else { tee });

    // Label
    out.push_str(&node_label(node, path, opts));
    out.push('\n');

    // Recurse
    if let Some(children) = children_of(node) {
        let new_prefix = if opts.ascii_only {
            format!("{}{}", parent_prefix, if is_last { sp } else { bar })
        } else {
            format!("{}{}", parent_prefix, if is_last { sp } else { bar })
        };

        for (i, child) in children.iter().enumerate() {
            path.push(i);
            fmt_tree_rec(child, out, path, &new_prefix, i + 1 == children.len(), opts);
            path.pop();
        }
    }
}

/// Render the line label for a node.
fn node_label(node: &TrackNode, path: &[usize], opts: TreePrintOptions) -> String {
    let mut s = String::new();

    if opts.show_path && !path.is_empty() {
        // Render path like [0,1,2]
        s.push('[');
        for (i, idx) in path.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let _ = write!(s, "{}", idx);
        }
        s.push(']');
        s.push(' ');
    }

    match node {
        TrackNode::Seq(seq) => {
            // Basic label
            s.push_str("Seq");
            if opts.show_details {
                // Adjust fields to your actual `Sequence` struct
                let loop_len = seq.loop_len.as_secs();
                let _ = write!(s, " token={} loop={:.2}s", seq.token.0, loop_len);
                // If you like: wave type / repeat etc.
                // let _ = write!(s, " repeat={}", seq.repeat);
                // let _ = write!(s, " wave={}", seq.wave_type.to_string());
            }
        }
        TrackNode::Group {
            name,
            children,
            muted,
            volume,
            spacial,
            ..
        } => {
            // Use a box emoji unless ascii_only
            if !opts.ascii_only {
                s.push_str("📦 ");
            }
            s.push_str("Group");
            if !name.is_empty() {
                let _ = write!(s, " \"{}\"", name);
            }
            if opts.show_details {
                let _ = write!(s, " ({})", children.len());
                // small status hints
                if *muted {
                    s.push_str(" [muted]");
                }
                if (*volume - 1.0).abs() > f64::EPSILON {
                    let _ = write!(s, " vol={:.2}", volume);
                }
                if (*spacial - 0.5).abs() > f64::EPSILON {
                    let _ = write!(s, " pan={:.2}", spacial);
                }
            }
        }
    }

    s
}

/// Borrow children if this is a group.
fn children_of(node: &TrackNode) -> Option<&[TrackNode]> {
    match node {
        TrackNode::Group { children, .. } => Some(children.as_slice()),
        _ => None,
    }
}
