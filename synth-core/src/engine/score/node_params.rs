use serde::{Deserialize, Serialize};

use crate::{
    engine::score::{
        default_params::*,
        sequence::Sequence,
        time_quantum::TimeQuantum,
        track_node::{NodeKind, TrackNode},
        ChorusParams, Interval, LowpassLfo, LowpassRelaxation, RdRythm, Rythm,
    },
    engine::waves::WaveType,
    rescale_factor,
    time_freq::{Beat, Freq, Time},
    DEFAULT_LOOP_LEN,
};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct BendParams {
    pub magnitude: f64,
    pub speed: f64,
}

impl Default for BendParams {
    fn default() -> Self {
        let (magnitude, speed) = default_bend();
        Self { magnitude, speed }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct VibratoParams {
    pub magnitude: f64,
    pub frequency: Freq,
}

impl Default for VibratoParams {
    fn default() -> Self {
        let (magnitude, frequency) = default_vibrato();
        Self {
            magnitude,
            frequency,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct EnvelopeParams {
    pub attack: f64,
    pub decay: f64,
    pub normalization: f64,
}

impl Default for EnvelopeParams {
    fn default() -> Self {
        let (attack, decay) = default_attack_decay();
        Self {
            attack,
            decay,
            normalization: rescale_factor(1.0 / attack, 1.0 / decay),
        }
    }
}

impl EnvelopeParams {
    pub fn rescale(&mut self) {
        let a = 1.0 / self.attack;
        let b = 1.0 / self.decay;
        let factor = rescale_factor(a, b);
        if factor.is_normal() {
            self.normalization = factor;
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct LowpassParams {
    pub envelope: (f64, f64),
    pub cutoff_multiplier: f64,
    pub relaxation: LowpassRelaxation,
    pub lfo: LowpassLfo,
    pub enabled: bool,
    pub order: u32,
}

impl Default for LowpassParams {
    fn default() -> Self {
        Self {
            envelope: default_attack_decay(),
            cutoff_multiplier: default_cutoff_multiplier(),
            relaxation: default_lp_relaxation(),
            lfo: default_lp_lfo(),
            enabled: default_lowpass_enabled(),
            order: default_lp_order(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct PowerParams {
    pub initial: f64,
    pub evolution: Freq,
}

impl Default for PowerParams {
    fn default() -> Self {
        let (initial, evolution) = default_pow_fact();
        Self { initial, evolution }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct WaveParams {
    pub wave: WaveType,
}

impl Default for WaveParams {
    fn default() -> Self {
        Self {
            wave: WaveType::Sine,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct HarmonyParams {
    pub glide: bool,
    pub harmonise: bool,
    pub tolerance: (Time, Time),
    pub interval: Interval,
    pub shuffle: bool,
    pub harmoniser: [u32; 7],
    pub arpegio: f64,
    pub chord: usize,
    pub random_chord: bool,
    pub reverse_prob: f64,
    pub shuffle_prob: f64,
}

impl Default for HarmonyParams {
    fn default() -> Self {
        Self {
            glide: default_glide(),
            harmonise: default_harmonise(),
            tolerance: default_tolerance(),
            interval: Interval::RDTempered(2, vec![-7, 0, 7], 0),
            shuffle: default_shuffle(),
            harmoniser: default_harmoniser(),
            arpegio: default_arpegio(),
            chord: default_tension(),
            random_chord: default_random_chord(),
            reverse_prob: default_reverse_prob(),
            shuffle_prob: default_shuffle_prob(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct RhythmParams {
    pub time_quantum: TimeQuantum,
    pub t_min: Beat,
    pub t_max: Beat,
    pub loop_len: Beat,
    pub tail_multiplier: f64,
    pub inclusions: Rythm,
    pub exclusions: Rythm,
    pub beat_offset: i32,
    pub repeat: usize,
}

impl Default for RhythmParams {
    fn default() -> Self {
        Self {
            time_quantum: default_time_quantum(),
            t_min: Beat(0.0),
            t_max: Beat(DEFAULT_LOOP_LEN.as_secs()),
            loop_len: default_loop_len(),
            tail_multiplier: default_tail_multiplier(),
            inclusions: Rythm::Rd(RdRythm::default()),
            exclusions: Rythm::Rd(RdRythm::default()),
            beat_offset: default_beat_offset(),
            repeat: default_repeat(),
        }
    }
}

impl WaveParams {
    pub fn from_sequence(seq: &Sequence) -> Self {
        Self {
            wave: seq.wave_type,
        }
    }
}

impl HarmonyParams {
    pub fn from_sequence(seq: &Sequence) -> Self {
        Self {
            glide: seq.glide,
            harmonise: seq.harmonise,
            tolerance: seq.tolerance,
            interval: seq.interval.clone(),
            shuffle: seq.shuffle,
            harmoniser: seq.harmoniser,
            arpegio: seq.arpegio,
            chord: seq.chord,
            random_chord: seq.random_chord,
            reverse_prob: seq.reverse_prob,
            shuffle_prob: seq.shuffle_prob,
        }
    }

    pub fn apply_to_sequence(&self, seq: &mut Sequence) {
        seq.glide = self.glide;
        seq.harmonise = self.harmonise;
        seq.tolerance = self.tolerance;
        seq.interval = self.interval.clone();
        seq.shuffle = self.shuffle;
        seq.harmoniser = self.harmoniser;
        seq.arpegio = self.arpegio;
        seq.chord = self.chord;
        seq.random_chord = self.random_chord;
        seq.reverse_prob = self.reverse_prob;
        seq.shuffle_prob = self.shuffle_prob;
    }
}

impl RhythmParams {
    pub fn from_sequence(seq: &Sequence) -> Self {
        Self {
            time_quantum: seq.time_quantum,
            t_min: seq.t_min,
            t_max: seq.t_max,
            loop_len: seq.loop_len,
            tail_multiplier: seq.tail_multiplier,
            inclusions: seq.inclusions.clone(),
            exclusions: seq.exclusions.clone(),
            beat_offset: seq.beat_offset,
            repeat: seq.repeat,
        }
    }

    pub fn apply_to_sequence(&self, seq: &mut Sequence) {
        seq.time_quantum = self.time_quantum;
        seq.t_min = self.t_min;
        seq.t_max = self.t_max;
        seq.loop_len = self.loop_len;
        seq.tail_multiplier = self.tail_multiplier;
        seq.inclusions = self.inclusions.clone();
        seq.exclusions = self.exclusions.clone();
        seq.beat_offset = self.beat_offset;
        seq.repeat = self.repeat;
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct NodeOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub bend: Option<BendParams>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub vibrato: Option<VibratoParams>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub chorus: Option<ChorusParams>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub envelope: Option<EnvelopeParams>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub lowpass: Option<LowpassParams>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub power: Option<PowerParams>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub wave: Option<WaveParams>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub harmony: Option<HarmonyParams>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub rhythm: Option<RhythmParams>,
}

impl NodeOverrides {
    pub fn sequence_defaults() -> Self {
        Self {
            bend: Some(BendParams::default()),
            vibrato: Some(VibratoParams::default()),
            chorus: Some(ChorusParams::default()),
            envelope: Some(EnvelopeParams::default()),
            lowpass: Some(LowpassParams::default()),
            power: Some(PowerParams::default()),
            wave: None,
            harmony: None,
            rhythm: None,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ResolvedTrackParams {
    pub bend: BendParams,
    bend_depth: Option<usize>,
    pub vibrato: VibratoParams,
    vibrato_depth: Option<usize>,
    pub chorus: ChorusParams,
    chorus_depth: Option<usize>,
    pub envelope: EnvelopeParams,
    envelope_depth: Option<usize>,
    pub lowpass: LowpassParams,
    lowpass_depth: Option<usize>,
    pub power: PowerParams,
    power_depth: Option<usize>,
    pub wave: WaveParams,
    wave_depth: Option<usize>,
    pub harmony: HarmonyParams,
    harmony_depth: Option<usize>,
    pub rhythm: RhythmParams,
    rhythm_depth: Option<usize>,
}

impl Default for ResolvedTrackParams {
    fn default() -> Self {
        Self {
            bend: BendParams::default(),
            bend_depth: None,
            vibrato: VibratoParams::default(),
            vibrato_depth: None,
            chorus: ChorusParams::default(),
            chorus_depth: None,
            envelope: EnvelopeParams::default(),
            envelope_depth: None,
            lowpass: LowpassParams::default(),
            lowpass_depth: None,
            power: PowerParams::default(),
            power_depth: None,
            wave: WaveParams::default(),
            wave_depth: None,
            harmony: HarmonyParams::default(),
            harmony_depth: None,
            rhythm: RhythmParams::default(),
            rhythm_depth: None,
        }
    }
}

impl ResolvedTrackParams {
    pub fn with_overrides(mut self, overrides: &NodeOverrides, depth: usize) -> Self {
        if let Some(bend) = &overrides.bend {
            if self.bend_depth.is_none() {
                self.bend = bend.clone();
                self.bend_depth = Some(depth);
            }
        }
        if let Some(vibrato) = &overrides.vibrato {
            if self.vibrato_depth.is_none() {
                self.vibrato = vibrato.clone();
                self.vibrato_depth = Some(depth);
            }
        }
        if let Some(chorus) = &overrides.chorus {
            if self.chorus_depth.is_none() {
                self.chorus = chorus.clone();
                self.chorus_depth = Some(depth);
            }
        }
        if let Some(envelope) = &overrides.envelope {
            if self.envelope_depth.is_none() {
                self.envelope = envelope.clone();
                self.envelope_depth = Some(depth);
            }
        }
        if let Some(lowpass) = &overrides.lowpass {
            if self.lowpass_depth.is_none() {
                self.lowpass = lowpass.clone();
                self.lowpass_depth = Some(depth);
            }
        }
        if let Some(power) = &overrides.power {
            if self.power_depth.is_none() {
                self.power = power.clone();
                self.power_depth = Some(depth);
            }
        }
        if let Some(wave) = &overrides.wave {
            if self.wave_depth.is_none() {
                self.wave = wave.clone();
                self.wave_depth = Some(depth);
            }
        }
        if let Some(harmony) = &overrides.harmony {
            if self.harmony_depth.is_none() {
                self.harmony = harmony.clone();
                self.harmony_depth = Some(depth);
            }
        }
        if let Some(rhythm) = &overrides.rhythm {
            if self.rhythm_depth.is_none() {
                self.rhythm = rhythm.clone();
                self.rhythm_depth = Some(depth);
            }
        }
        self
    }

    pub fn has_wave_override(&self) -> bool {
        self.wave_depth.is_some()
    }

    pub fn has_harmony_override(&self) -> bool {
        self.harmony_depth.is_some()
    }

    pub fn has_rhythm_override(&self) -> bool {
        self.rhythm_depth.is_some()
    }
}

#[derive(Clone)]
pub struct ParamResolution<T> {
    pub value: T,
    pub source_depth: Option<usize>,
}

impl<T> ParamResolution<T> {
    pub fn locked_for_depth(&self, depth: usize) -> bool {
        self.source_depth.map(|src| src < depth).unwrap_or(false)
    }

    pub fn provided_here(&self, depth: usize) -> bool {
        self.source_depth == Some(depth)
    }
}

impl<T: Default> Default for ParamResolution<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            source_depth: None,
        }
    }
}

impl TrackNode {
    pub fn resolve_bend(&self, path: &[usize]) -> ParamResolution<BendParams> {
        resolve_param(self, path, |o| o.bend.as_ref())
    }

    pub fn resolve_vibrato(&self, path: &[usize]) -> ParamResolution<VibratoParams> {
        resolve_param(self, path, |o| o.vibrato.as_ref())
    }

    pub fn resolve_chorus(&self, path: &[usize]) -> ParamResolution<ChorusParams> {
        resolve_param(self, path, |o| o.chorus.as_ref())
    }

    pub fn resolve_envelope(&self, path: &[usize]) -> ParamResolution<EnvelopeParams> {
        resolve_param(self, path, |o| o.envelope.as_ref())
    }

    pub fn resolve_lowpass(&self, path: &[usize]) -> ParamResolution<LowpassParams> {
        resolve_param(self, path, |o| o.lowpass.as_ref())
    }

    pub fn resolve_power(&self, path: &[usize]) -> ParamResolution<PowerParams> {
        resolve_param(self, path, |o| o.power.as_ref())
    }

    pub fn resolve_wave(&self, path: &[usize]) -> ParamResolution<WaveParams> {
        resolve_param(self, path, |o| o.wave.as_ref())
    }

    pub fn resolve_harmony(&self, path: &[usize]) -> ParamResolution<HarmonyParams> {
        resolve_param(self, path, |o| o.harmony.as_ref())
    }

    pub fn resolve_rhythm(&self, path: &[usize]) -> ParamResolution<RhythmParams> {
        resolve_param(self, path, |o| o.rhythm.as_ref())
    }

    pub fn resolved_params_for_path(&self, path: &[usize]) -> ResolvedTrackParams {
        let mut params = ResolvedTrackParams::default().with_overrides(&self.overrides, 0);
        let mut node = self;
        let mut depth = 0usize;
        for &idx in path {
            match &node.kind {
                NodeKind::Group { children, .. } => {
                    if let Some(child) = children.get(idx) {
                        depth += 1;
                        params = params.with_overrides(&child.overrides, depth);
                        node = child;
                    } else {
                        break;
                    }
                }
                NodeKind::Seq(_) => break,
            }
        }
        params
    }
}

fn resolve_param<'a, T, F>(root: &'a TrackNode, path: &[usize], getter: F) -> ParamResolution<T>
where
    T: Default + Clone + 'a,
    F: Fn(&'a NodeOverrides) -> Option<&'a T>,
{
    let mut node = root;
    let mut depth = 0usize;
    if let Some(value) = getter(&node.overrides) {
        return ParamResolution {
            value: value.clone(),
            source_depth: Some(depth),
        };
    }
    for &idx in path {
        match &node.kind {
            NodeKind::Group { children, .. } => {
                if let Some(child) = children.get(idx) {
                    depth += 1;
                    if let Some(value) = getter(&child.overrides) {
                        return ParamResolution {
                            value: value.clone(),
                            source_depth: Some(depth),
                        };
                    }
                    node = child;
                } else {
                    break;
                }
            }
            NodeKind::Seq(_) => break,
        }
    }
    ParamResolution {
        value: T::default(),
        source_depth: None,
    }
}
