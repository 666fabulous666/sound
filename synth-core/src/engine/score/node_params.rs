use serde::{Deserialize, Serialize};

use crate::{
    engine::score::{
        default_params::*,
        track_node::{TrackNode, NodeKind},
        ChorusParams, LowpassLfo, LowpassRelaxation,
    },
    time_freq::Freq,
    rescale_factor,
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
        }
    }
}

#[derive(Clone)]
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
        self
    }
}

#[derive(Clone)]
pub struct ParamResolution<T> {
    pub value: T,
    pub source_depth: Option<usize>,
}

impl<T> ParamResolution<T> {
    pub fn locked_for_depth(&self, depth: usize) -> bool {
        self.source_depth
            .map(|src| src < depth)
            .unwrap_or(false)
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
