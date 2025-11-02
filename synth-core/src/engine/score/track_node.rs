use crate::engine::score::default_proba;
use core::marker::PhantomData;
use rand::Rng;
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use crate::{
    engine::score::{sequence::Sequence, NotesGroup},
    time_freq::Time,
    Token, TokenGen,
};

fn default_hue() -> f64 {
    0.0
}

fn default_pan() -> f64 {
    0.5
}

#[derive(Clone, Serialize, Deserialize)]
pub enum NodeKind {
    Group {
        id: Token,
        muted: bool,
        collapsed: bool,
        children: Vec<TrackNode>,
        #[serde(default)]
        not_generate_until: Option<Time>,
    },
    Seq(Sequence),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TrackNode {
    pub name: String,
    #[serde(default = "default_proba")]
    pub proba: f64,
    pub volume: f64, // mix gain multiplier (>= 0.0)
    #[serde(default = "default_pan", alias = "spacial")]
    pub pan: f64, // pan 0.0..=1.0 (0 = L, 0.5 = C, 1 = R)
    #[serde(default = "default_hue")]
    pub hue: f64, // HSL hue 0.0..=360.0
    #[serde(flatten)]
    pub kind: NodeKind,
}

impl TrackNode {
    pub fn as_seq(&self) -> Option<&Sequence> {
        if let NodeKind::Seq(s) = &self.kind {
            Some(s)
        } else {
            None
        }
    }
    pub fn as_seq_mut(&mut self) -> Option<&mut Sequence> {
        if let NodeKind::Seq(s) = &mut self.kind {
            Some(s)
        } else {
            None
        }
    }

    // ---------- Constructors ----------
    /// Root group with safe defaults.
    pub fn new_root(gen: &mut TokenGen) -> Self {
        Self::new_node(gen, "Root")
    }

    /// New group with a fresh Token id and safe defaults.
    pub fn new_node(gen: &mut TokenGen, name: impl Into<String>) -> Self {
        TrackNode {
            name: name.into(),
            proba: 1.0,
            volume: 1.0,
            pan: 0.5,
            hue: 0.0,
            kind: NodeKind::Group {
                id: gen.next(),
                muted: false,
                collapsed: false,
                children: Vec::new(),
                not_generate_until: None,
            },
        }
    }

    /// Wrap a Sequence node.
    pub fn from_sequence(seq: Sequence) -> Self {
        TrackNode {
            name: String::new(),
            proba: 1.0,
            volume: 1.0,
            pan: 0.5,
            hue: 0.0,
            kind: NodeKind::Seq(seq),
        }
    }

    // // ---------- Queries ----------
    pub fn is_group(&self) -> bool {
        matches!(self.kind, NodeKind::Group { .. })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    pub fn volume(&self) -> f64 {
        self.volume
    }

    pub fn volume_mut(&mut self) -> &mut f64 {
        &mut self.volume
    }

    pub fn pan(&self) -> f64 {
        self.pan
    }

    pub fn pan_mut(&mut self) -> &mut f64 {
        &mut self.pan
    }

    pub fn proba(&self) -> f64 {
        self.proba
    }

    pub fn proba_mut(&mut self) -> &mut f64 {
        &mut self.proba
    }

    pub fn child_count(&self) -> usize {
        match &self.kind {
            NodeKind::Group { children, .. } => children.len(),
            NodeKind::Seq(_) => 0,
        }
    }

    // ---------- Safe mutations on groups ----------
    /// Push a child at the end; returns its index.
    pub fn push_child(&mut self, child: TrackNode) -> Option<usize> {
        match &mut self.kind {
            NodeKind::Group { children, .. } => {
                children.push(child);
                Some(children.len() - 1)
            }
            _ => None,
        }
    }

    /// Insert a child at index; returns true on success.
    pub fn insert_child(&mut self, index: usize, child: TrackNode) -> bool {
        match &mut self.kind {
            NodeKind::Group { children, .. } => {
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
        let volume = self.volume_mut();
        *volume = volume.max(0.0);
        let pan = self.pan_mut();
        *pan = pan.clamp(0.0, 1.0);
    }

    // ---------- Path-based navigation & edits ----------
    pub fn get<'a>(&'a self, path: &[usize]) -> Option<&'a TrackNode> {
        let mut cur = self;
        for &ix in path {
            match &cur.kind {
                NodeKind::Group { children, .. } => cur = children.get(ix)?,
                NodeKind::Seq(_) => return None,
            }
        }
        Some(cur)
    }

    pub fn get_mut<'a>(&'a mut self, path: &[usize]) -> Option<&'a mut TrackNode> {
        let mut cur = self;
        for &ix in path {
            match &mut cur.kind {
                NodeKind::Group { children, .. } => cur = children.get_mut(ix)?,
                NodeKind::Seq(_) => return None,
            }
        }
        Some(cur)
    }

    pub fn remove_at(&mut self, path: &[usize]) -> Option<TrackNode> {
        match (path.split_first(), &mut self.kind) {
            (Some((&ix, rest)), NodeKind::Group { children, .. }) if rest.is_empty() => {
                if ix < children.len() {
                    Some(children.remove(ix))
                } else {
                    None
                }
            }
            (Some((&ix, rest)), NodeKind::Group { children, .. }) => {
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
        match &mut parent.kind {
            NodeKind::Group { children, .. } => {
                if ix <= children.len() {
                    children.insert(ix, node);
                    return true;
                }
            }
            _ => (),
        }
        false
    }

    pub fn toggle_mute(&mut self) {
        match &mut self.kind {
            NodeKind::Group { ref mut muted, .. } => *muted ^= true,
            NodeKind::Seq(sequence) => sequence.mute ^= true,
        }
    }

    pub fn is_mute(&self) -> bool {
        match &self.kind {
            NodeKind::Group { muted, .. } => *muted,
            NodeKind::Seq(sequence) => sequence.mute,
        }
    }
    /// Recursively draw all sequences under this node.
    /// Volume is NOT computed here - notes are generated at 1.0 and volume is applied separately.
    pub fn draw_node(
        &mut self,
        notes: &mut BTreeMap<Token, NotesGroup>,
        rng: &mut rand::rngs::ThreadRng,
        now: Time,
    ) {
        match &mut self.kind {
            NodeKind::Seq(seq) => {
                seq.draw_sequence_core(notes, rng, now, self.pan, self.proba);
            }
            NodeKind::Group {
                children,
                muted,
                not_generate_until,
                ..
            } => {
                if *muted {
                    return; // Don't generate anything if muted
                }
                if not_generate_until.map_or(true, |until| now >= until) {
                    if rng.gen_bool(self.proba) {
                        for ch in children {
                            ch.draw_node(notes, rng, now);
                        }
                    }
                }
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
            match &node.kind {
                NodeKind::Seq(s) => return Some(s),
                NodeKind::Group { children, .. } => {
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
            match &mut node.kind {
                NodeKind::Seq(s) => f(s),
                NodeKind::Group { children, .. } => {
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
            match &mut node.kind {
                NodeKind::Seq(s) => out.push(s as *mut Sequence),
                NodeKind::Group { children, .. } => {
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
    /// Returns `true` if the node at `path` should be visible,
    /// i.e. none of its *ancestor* groups are collapsed.
    pub fn path_visible(&self, path: &[usize]) -> bool {
        let mut node = self;

        for (depth, &idx) in path.iter().enumerate() {
            match &node.kind {
                NodeKind::Group {
                    collapsed,
                    children,
                    ..
                } => {
                    // If an *ancestor* is collapsed, hide descendants.
                    if *collapsed && depth < path.len() {
                        return false;
                    }
                    node = match children.get(idx) {
                        Some(n) => n,
                        None => return false, // invalid path
                    };
                }
                NodeKind::Seq(_) => {
                    // Can't have children under a sequence; only valid if this is the last step.
                    return depth + 1 == path.len();
                }
            }
        }
        true
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
            match &node.kind {
                NodeKind::Seq(seq) => {
                    // Leaf: yield (path, &Sequence)
                    return Some((path, seq));
                }
                NodeKind::Group { children, .. } => {
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

    match &node.kind {
        NodeKind::Seq(seq) => {
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
        NodeKind::Group {
            children, muted, ..
        } => {
            // Use a box emoji unless ascii_only
            if !opts.ascii_only {
                s.push_str("📦 ");
            }
            s.push_str("Group");
            if !node.name.is_empty() {
                let _ = write!(s, " \"{}\"", node.name);
            }
            if opts.show_details {
                let _ = write!(s, " ({})", children.len());
                // small status hints
                if *muted {
                    s.push_str(" [muted]");
                }
                if (node.volume - 1.0).abs() > f64::EPSILON {
                    let _ = write!(s, " vol={:.2}", node.volume);
                }
                if (node.pan - 0.5).abs() > f64::EPSILON {
                    let _ = write!(s, " pan={:.2}", node.pan);
                }
            }
        }
    }

    s
}

/// Borrow children if this is a group.
fn children_of(node: &TrackNode) -> Option<&[TrackNode]> {
    match &node.kind {
        NodeKind::Group { children, .. } => Some(children.as_slice()),
        _ => None,
    }
}
fn collect_nodes_with_paths<'a>(
    node: &'a TrackNode,
    cur: &mut Vec<usize>,
    out: &mut Vec<(Vec<usize>, &'a TrackNode)>,
) {
    out.push((cur.clone(), node));
    if let NodeKind::Group { children, .. } = &node.kind {
        for (i, ch) in children.iter().enumerate() {
            cur.push(i);
            collect_nodes_with_paths(ch, cur, out);
            cur.pop();
        }
    }
}

pub fn nodes_with_paths(root: &TrackNode) -> Vec<(Vec<usize>, &TrackNode)> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    collect_nodes_with_paths(root, &mut cur, &mut out);
    out
}
fn collect_all_paths(node: &TrackNode, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    out.push(cur.clone());
    if let NodeKind::Group { children, .. } = &node.kind {
        for (i, ch) in children.iter().enumerate() {
            cur.push(i);
            collect_all_paths(ch, cur, out);
            cur.pop();
        }
    }
}

pub fn all_paths(root: &TrackNode) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    collect_all_paths(root, &mut cur, &mut out);
    out
}
