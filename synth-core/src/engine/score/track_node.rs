use crate::engine::score::{
    default_proba, probability::Probability, scheduler::PlaybackScheduler, TrackDelays,
};
use core::marker::PhantomData;
use rand::Rng;
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use crate::{
    engine::score::{
        node_params::{
            BendParams, EnvelopeParams, HarmonyParams, LowpassParams, NodeOverrides, PowerParams,
            ResolvedTrackParams, RhythmParams, VibratoParams,
        },
        sequence::Sequence,
        NotesGroup,
    },
    engine::waves::WaveType,
    rescale_factor,
    time_freq::{Tempo, Time},
    NoteIdGen, Token, TokenGen,
};

fn default_hue() -> f64 {
    0.0
}

fn default_pan() -> f64 {
    0.5
}

fn default_or_weight() -> f64 {
    1.0
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupMode {
    And,
    Or,
}

impl Default for GroupMode {
    fn default() -> Self {
        GroupMode::And
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct AestheticLocks {
    #[serde(default)]
    pub lock_volume: bool,
    #[serde(default)]
    pub lock_pan: bool,
    #[serde(default)]
    pub lock_hue: bool,
}

#[derive(Clone, Copy)]
pub struct MixContext {
    pub volume: f64,
    pub pan: Option<f64>,
    pub proba: Probability,
    /// True if any track in the tree has solo enabled.
    pub solo_active: bool,
    /// True if this node or any ancestor has solo enabled (children of solo tracks play).
    pub in_solo_chain: bool,
}

impl Default for MixContext {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pan: None,
            proba: Probability::certainty(),
            solo_active: false,
            in_solo_chain: false,
        }
    }
}

impl MixContext {
    pub fn with_solo_active(mut self, solo_active: bool) -> Self {
        self.solo_active = solo_active;
        self
    }

    pub fn propagate(&self, node: &TrackNode) -> (MixContext, f64) {
        // Determine if we're in a solo chain (this node or ancestor is solo)
        let in_solo_chain = self.in_solo_chain || node.solo;

        // Don't apply solo zeroing here - just multiply volumes normally.
        // Solo zeroing is applied only at the sequence level when setting NotesGroup volume.
        let volume = sanitize_volume(self.volume * node.volume());

        let proba = self.proba.mul(node.proba);
        let pan_lock_here =
            matches!(node.kind, NodeKind::Group { ref aesthetic, .. } if aesthetic.lock_pan);
        let effective_pan = self.pan.unwrap_or(node.pan);
        let next_pan = if let Some(pan) = self.pan {
            Some(pan)
        } else if pan_lock_here {
            Some(node.pan)
        } else {
            None
        };

        (
            MixContext {
                volume,
                pan: next_pan,
                proba,
                solo_active: self.solo_active,
                in_solo_chain,
            },
            effective_pan,
        )
    }

    /// Returns the effective volume for a sequence, considering solo mode.
    /// Call this when setting the actual volume on a NotesGroup.
    pub fn effective_volume(&self) -> f64 {
        if self.solo_active && !self.in_solo_chain {
            0.0
        } else {
            self.volume
        }
    }
}

fn sanitize_volume(value: f64) -> f64 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum NodeKind {
    Group {
        id: Token,
        collapsed: bool,
        children: Vec<TrackNode>,
        #[serde(default)]
        not_generate_until: Option<Time>,
        #[serde(default)]
        mode: GroupMode,
        #[serde(default)]
        aesthetic: AestheticLocks,
    },
    Seq(Sequence),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TrackNode {
    pub name: String,
    #[serde(default = "default_proba")]
    pub proba: Probability,
    pub volume: f64, // mix gain multiplier (>= 0.0)
    #[serde(default)]
    pub muted: bool, // when true, effective volume is 0
    #[serde(default)]
    pub solo: bool, // when true and any track is solo, only solo tracks produce sound
    #[serde(default = "default_pan", alias = "spacial")]
    pub pan: f64, // pan 0.0..=1.0 (0 = L, 0.5 = C, 1 = R)
    #[serde(default = "default_hue")]
    pub hue: f64, // HSL hue 0.0..=360.0
    #[serde(default = "default_or_weight")]
    pub or_weight: f64,
    #[serde(default)]
    pub overrides: NodeOverrides,
    #[serde(default, skip_serializing_if = "TrackDelays::is_empty")]
    pub delays: TrackDelays,
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
            proba: Probability::default(),
            volume: 1.0,
            muted: false,
            solo: false,
            pan: 0.5,
            hue: 0.0,
            or_weight: 1.0,
            overrides: NodeOverrides::default(),
            delays: TrackDelays::default(),
            kind: NodeKind::Group {
                id: gen.next(),
                collapsed: false,
                children: Vec::new(),
                not_generate_until: None,
                mode: GroupMode::And,
                aesthetic: AestheticLocks::default(),
            },
        }
    }

    /// Wrap a Sequence node.
    pub fn from_sequence(seq: Sequence) -> Self {
        TrackNode {
            name: String::new(),
            proba: Probability::default(),
            volume: 1.0,
            muted: false,
            solo: false,
            pan: 0.5,
            hue: 0.0,
            or_weight: 1.0,
            overrides: NodeOverrides::sequence_defaults(),
            delays: TrackDelays::default(),
            kind: NodeKind::Seq(seq),
        }
    }

    pub fn migrate_legacy_overrides(&mut self) {
        match &mut self.kind {
            NodeKind::Seq(seq) => {
                let legacy = seq.take_legacy_params();
                if let Some((magnitude, speed)) = legacy.bend {
                    self.overrides.bend = Some(BendParams { magnitude, speed });
                }
                if let Some((magnitude, frequency)) = legacy.vibrato {
                    self.overrides.vibrato = Some(VibratoParams {
                        magnitude,
                        frequency,
                    });
                }
                if let Some(chorus) = legacy.chorus {
                    self.overrides.chorus = Some(chorus);
                }
                if let Some((attack, decay)) = legacy.attack_decay {
                    let normalization = legacy
                        .normalization
                        .unwrap_or_else(|| rescale_factor(1.0 / attack, 1.0 / decay));
                    self.overrides.envelope = Some(EnvelopeParams {
                        attack,
                        decay,
                        normalization,
                    });
                }
                let mut lowpass = LowpassParams::default();
                let mut lowpass_touched = false;
                if let Some(envelope) = legacy.lp_attack_decay {
                    lowpass.envelope = envelope;
                    lowpass_touched = true;
                }
                if let Some(cutoff) = legacy.cutoff_multiplier {
                    lowpass.cutoff.base = cutoff;
                    lowpass_touched = true;
                }
                if let Some(relax) = legacy.lp_relaxation {
                    lowpass.cutoff.relaxation = crate::engine::time_varying::Relaxation {
                        start: relax.start,
                        end: relax.end,
                        rate: relax.rate,
                    };
                    lowpass_touched = true;
                }
                if let Some(lfo) = legacy.lp_lfo {
                    lowpass.cutoff.lfo = crate::engine::time_varying::Lfo {
                        magnitude: lfo.magnitude,
                        frequency: lfo.frequency,
                        sync_with_clock: lfo.sync_with_clock,
                    };
                    lowpass_touched = true;
                }
                if let Some(enabled) = legacy.lowpass_enabled {
                    lowpass.enabled = enabled;
                    lowpass_touched = true;
                }
                if let Some(order) = legacy.lp_order {
                    lowpass.order = order;
                    lowpass_touched = true;
                }
                if lowpass_touched {
                    self.overrides.lowpass = Some(lowpass);
                }
                if let Some((_initial, _evolution)) = legacy.pow_fact {
                    // Legacy power factor used exponential evolution, which is incompatible
                    // with the new TimeVarying system. We just use the default.
                    self.overrides.power = Some(PowerParams::default());
                }
            }
            NodeKind::Group { children, .. } => {
                for child in children {
                    child.migrate_legacy_overrides();
                }
            }
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

    /// Returns the effective volume (0.0 if muted, otherwise the stored volume).
    pub fn volume(&self) -> f64 {
        if self.muted {
            0.0
        } else {
            self.volume
        }
    }

    /// Returns a mutable reference to the stored volume value.
    /// Note: This is the stored value, not the effective volume.
    pub fn volume_mut(&mut self) -> &mut f64 {
        &mut self.volume
    }

    /// Returns the stored volume value, ignoring mute state.
    pub fn volume_raw(&self) -> f64 {
        self.volume
    }

    pub fn pan(&self) -> f64 {
        self.pan
    }

    pub fn pan_mut(&mut self) -> &mut f64 {
        &mut self.pan
    }

    pub fn proba(&self) -> Probability {
        self.proba
    }

    pub fn proba_mut(&mut self) -> &mut Probability {
        &mut self.proba
    }

    pub fn set_proba(&mut self, value: f64) {
        self.proba.set(value);
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

    /// Compute the mix context contributed by all ancestors of `path`.
    /// The node at `path` itself is not applied; the returned context should be
    /// passed when drawing that node so it can apply its own mix.
    pub fn mix_context_for_path(&self, path: &[usize]) -> MixContext {
        let mut mix = MixContext::default();
        let mut node = self;
        for &idx in path {
            let (next_mix, _) = mix.propagate(node);
            match &node.kind {
                NodeKind::Group { children, .. } => {
                    node = match children.get(idx) {
                        Some(ch) => ch,
                        None => break,
                    };
                    mix = next_mix;
                }
                NodeKind::Seq(_) => break,
            }
        }
        mix
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
        self.muted = !self.muted;
    }

    pub fn is_mute(&self) -> bool {
        self.muted
    }

    pub fn toggle_solo(&mut self) {
        self.solo = !self.solo;
    }

    pub fn is_solo(&self) -> bool {
        self.solo
    }

    /// Check if any node in the tree has solo enabled.
    pub fn has_any_solo(&self) -> bool {
        if self.solo {
            return true;
        }
        match &self.kind {
            NodeKind::Seq(_) => false,
            NodeKind::Group { children, .. } => children.iter().any(|ch| ch.has_any_solo()),
        }
    }
    /// Recursively draw all sequences under this node.
    /// Volume is NOT computed here - notes are generated at 1.0 and volume is applied separately.
    pub fn draw_node(
        &mut self,
        notes: &mut BTreeMap<Token, NotesGroup>,
        scheduler: &mut PlaybackScheduler,
        rng: &mut rand::rngs::ThreadRng,
        now: Time,
        tempo: Tempo,
        note_id_gen: &mut NoteIdGen,
        mix: MixContext,
        inherited_params: ResolvedTrackParams,
        depth: usize,
        inherited_delays: TrackDelays,
    ) {
        let (current_mix, effective_pan) = mix.propagate(self);
        let current_params = inherited_params.with_overrides(&self.overrides, depth);
        let merged_delays = self.delays.merge_with_parent(&inherited_delays);
        match &mut self.kind {
            NodeKind::Seq(seq) => {
                if !scheduler.sequence_state_mut(seq.token).is_idle(now) {
                    return;
                }
                let rhythm_context = if current_params.has_rhythm_override() {
                    current_params.rhythm.clone()
                } else {
                    RhythmParams::from_sequence(seq)
                };
                let harmony_context = if current_params.has_harmony_override() {
                    current_params.harmony.clone()
                } else {
                    HarmonyParams::from_sequence(seq)
                };
                let wave_context: WaveType = if current_params.has_wave_override() {
                    current_params.wave.wave
                } else {
                    seq.wave_type
                };
                let delays_seconds = merged_delays.to_seconds(tempo, 0.5, 0.5);
                if let Some(release) = seq.draw_sequence_core(
                    notes,
                    rng,
                    now,
                    effective_pan,
                    current_mix.proba,
                    tempo,
                    note_id_gen,
                    &current_params,
                    &rhythm_context,
                    &harmony_context,
                    wave_context,
                    &delays_seconds,
                ) {
                    if let Some(ng) = notes.get(&seq.token) {
                        scheduler.register_sequence_snapshot(seq, ng, rhythm_context.loop_len);
                    }
                    scheduler.sequence_state_mut(seq.token).busy_until = release;
                }
            }
            NodeKind::Group {
                children,
                not_generate_until,
                mode,
                ..
            } => {
                // Muted nodes will have 0 volume via MixContext propagation
                if not_generate_until.map_or(true, |until| now >= until) {
                    if rng.gen_bool(current_mix.proba.as_f64()) {
                        match mode {
                            GroupMode::And => {
                                for ch in children {
                                    ch.draw_node(
                                        notes,
                                        scheduler,
                                        rng,
                                        now,
                                        tempo,
                                        note_id_gen,
                                        current_mix,
                                        current_params.clone(),
                                        depth + 1,
                                        merged_delays.clone(),
                                    );
                                }
                            }
                            GroupMode::Or => {
                                let busy_threshold = children
                                    .iter()
                                    .map(|child| child_busy_until(child, scheduler))
                                    .reduce(Time::max)
                                    .unwrap_or(Time(0.0));
                                if busy_threshold > now {
                                    return;
                                }
                                let candidates: Vec<usize> = children
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, child)| child_idle(child, scheduler, now))
                                    .map(|(idx, _)| idx)
                                    .collect();
                                if let Some(idx) = select_or_child(children, &candidates, rng) {
                                    if let Some(child) = children.get_mut(idx) {
                                        child.draw_node(
                                            notes,
                                            scheduler,
                                            rng,
                                            now,
                                            tempo,
                                            note_id_gen,
                                            current_mix,
                                            current_params.clone(),
                                            depth + 1,
                                            merged_delays.clone(),
                                        );
                                        let busy_until = child_busy_until(child, scheduler);
                                        for (child_idx, sibling) in children.iter_mut().enumerate()
                                        {
                                            if child_idx != idx {
                                                force_busy_until(sibling, scheduler, busy_until);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn select_or_child(
    children: &[TrackNode],
    candidates: &[usize],
    rng: &mut rand::rngs::ThreadRng,
) -> Option<usize> {
    let mut total = 0.0;
    for idx in candidates {
        total += children[*idx].or_weight.max(0.0);
    }
    if total <= f64::EPSILON {
        return None;
    }
    let mut pick = rng.gen_range(0.0..total);
    for idx in candidates {
        let weight = children[*idx].or_weight.max(0.0);
        if pick < weight {
            return Some(*idx);
        }
        pick -= weight;
    }
    None
}

fn child_idle(child: &TrackNode, scheduler: &PlaybackScheduler, now: Time) -> bool {
    match &child.kind {
        NodeKind::Seq(seq) => scheduler
            .sequence_state(&seq.token)
            .map(|state| state.is_idle(now))
            .unwrap_or(true),
        NodeKind::Group { children, .. } => {
            children.iter().all(|ch| child_idle(ch, scheduler, now))
        }
    }
}

fn child_busy_until(child: &TrackNode, scheduler: &PlaybackScheduler) -> Time {
    match &child.kind {
        NodeKind::Seq(seq) => scheduler
            .sequence_state(&seq.token)
            .map(|state| state.busy_until)
            .unwrap_or(Time(0.0)),
        NodeKind::Group { children, .. } => children
            .iter()
            .map(|ch| child_busy_until(ch, scheduler))
            .max()
            .unwrap_or(Time(0.0)),
    }
}

fn force_busy_until(child: &TrackNode, scheduler: &mut PlaybackScheduler, until: Time) {
    match &child.kind {
        NodeKind::Seq(seq) => {
            let state = scheduler.sequence_state_mut(seq.token);
            if state.busy_until < until {
                state.busy_until = until;
            }
        }
        NodeKind::Group { children, .. } => {
            for ch in children {
                force_busy_until(ch, scheduler, until);
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

impl TrackNode {
    pub fn collect_sequences_with_params(
        &self,
        inherited: ResolvedTrackParams,
        depth: usize,
        out: &mut Vec<(Token, ResolvedTrackParams)>,
    ) {
        let current = inherited.with_overrides(&self.overrides, depth);
        match &self.kind {
            NodeKind::Seq(seq) => {
                let mut final_params = current;
                if !final_params.has_wave_override() {
                    final_params.wave.wave = seq.wave_type;
                }
                out.push((seq.token, final_params));
            }
            NodeKind::Group { children, .. } => {
                for child in children {
                    child.collect_sequences_with_params(current.clone(), depth + 1, out);
                }
            }
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
                let loop_len = seq.loop_len.as_beats();
                let _ = write!(s, " token={} loop={:.2} beats", seq.token.0, loop_len);
                // If you like: wave type / repeat etc.
                // let _ = write!(s, " repeat={}", seq.repeat);
                // let _ = write!(s, " wave={}", seq.wave_type.to_string());
            }
        }
        NodeKind::Group { children, .. } => {
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
                if node.muted {
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
