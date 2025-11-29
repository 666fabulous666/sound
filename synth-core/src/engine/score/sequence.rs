use itertools::Itertools;
use rand::seq::index::sample;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::ser::Serializer;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::convert::TryFrom;

use crate::engine::score::default_params;
use crate::engine::score::note::{Note, NoteVariant};
use crate::engine::score::probability::Probability;
use crate::engine::score::time_quantum::TimeQuantum;
use crate::engine::score::RdRythm;
use crate::engine::score::{
    node_params::{HarmonyParams, ResolvedTrackParams, RhythmParams},
    DelayTapSeconds, HarmonicsParams, LowpassLfo, LowpassRelaxation, NotesGroup,
};
use crate::time_freq::{Beat, Freq, Tempo, Time};

use crate::engine::waves::WaveType;
use crate::DEFAULT_LOOP_LEN;
use crate::GENERATE_EARLY;
use crate::{NoteIdGen, Token};

use super::Interval;

use super::Rythm;

use super::ChorusParams;

use default_params::*;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct IntervalAffinity {
    pub interval: u8,
    pub affinity: i32,
}

impl Default for IntervalAffinity {
    fn default() -> Self {
        Self {
            interval: 0,
            affinity: 0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct ReplicatorStep {
    pub distance: u32,
    #[serde(
        default = "default_interval_affinities",
        deserialize_with = "deserialize_interval_affinities"
    )]
    pub affinities: Vec<IntervalAffinity>,
}

impl Default for ReplicatorStep {
    fn default() -> Self {
        Self {
            distance: default_replicator_distance(),
            affinities: default_interval_affinities(),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AffinityInput {
    Pairs(Vec<IntervalAffinity>),
    Fixed([i32; MAX_MELODIC_INTERVAL]),
    Flexible(Vec<i32>),
}

pub(crate) fn deserialize_interval_affinities<'de, D>(
    deserializer: D,
) -> Result<Vec<IntervalAffinity>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let input = AffinityInput::deserialize(deserializer)?;
    let from_array = |vals: Vec<i32>| {
        vals.into_iter()
            .enumerate()
            .filter(|(_, v)| *v != 0)
            .map(|(i, v)| IntervalAffinity {
                interval: i as u8,
                affinity: v,
            })
            .collect()
    };
    let res = match input {
        AffinityInput::Pairs(p) => p,
        AffinityInput::Fixed(a) => from_array(a.to_vec()),
        AffinityInput::Flexible(v) => from_array(v),
    };
    Ok(res)
}

pub(crate) fn default_interval_affinities() -> Vec<IntervalAffinity> {
    Vec::new()
}

pub(crate) fn default_replicator_steps() -> Vec<ReplicatorStep> {
    vec![ReplicatorStep::default()]
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Sequence {
    pub t_min: Beat,
    pub t_max: Beat,
    pub inclusions: Rythm,
    pub exclusions: Rythm,
    pub interval: Interval,
    pub wave_type: WaveType,
    #[serde(skip)]
    pub note_variant: NoteVariant,
    #[serde(default)]
    pub harmonics: HarmonicsParams,
    #[serde(default = "default_time_quantum")]
    pub time_quantum: TimeQuantum,
    #[serde(default = "default_harmoniser")]
    pub harmoniser: [u32; 7],
    #[serde(default = "default_skip_harmonised")]
    pub skip_harmonised: usize,
    #[serde(
        default = "default_interval_affinities",
        deserialize_with = "deserialize_interval_affinities"
    )]
    pub melodiser: Vec<IntervalAffinity>,
    #[serde(default = "default_replicator_enabled")]
    pub replicator: bool,
    #[serde(default = "default_replicator_steps")]
    pub replicator_steps: Vec<ReplicatorStep>,
    #[serde(
        default = "default_interval_affinities",
        deserialize_with = "deserialize_interval_affinities",
        skip_serializing,
        rename = "replicator_affinity"
    )]
    pub replicator_affinity_legacy: Vec<IntervalAffinity>,
    #[serde(default, skip_serializing, rename = "replicator_distance")]
    pub replicator_distance_legacy: Option<u32>,
    #[serde(default = "default_beat_offset")]
    pub beat_offset: i32,
    #[serde(default = "default_glide")]
    pub glide: bool,
    #[serde(default = "default_loop_len")]
    pub loop_len: Beat,
    #[serde(default = "default_tail_multiplier")]
    pub tail_multiplier: f64,
    #[serde(default = "default_arpegio")]
    pub arpegio: f64,
    #[serde(default = "default_tolerance")]
    pub tolerance: (Time, Time),
    #[serde(default = "default_harmonise")]
    pub harmonise: bool,
    #[serde(default = "default_melodise")]
    pub melodise: bool,
    #[serde(default = "default_melody_order_affinity")]
    pub melody_order_affinity: i32,
    #[serde(default = "default_repeat")]
    pub repeat: usize,
    #[serde(default = "default_loop_offset")]
    pub loop_offset: Beat,
    #[serde(default = "default_accents")]
    pub accents: (f64, Vec<f64>),
    #[serde(default = "default_shuffle")]
    pub shuffle: bool,
    #[serde(default = "default_tension")]
    pub chord: usize,
    #[serde(default = "default_random_chord")]
    pub random_chord: bool,
    #[serde(default = "default_reverse_prob")]
    pub reverse_prob: f64,
    #[serde(default = "default_shuffle_prob")]
    pub shuffle_prob: f64,
    #[serde(default, flatten)]
    legacy_params: LegacySequenceParams,
    pub not_generate_until: Option<Time>, // TODO: should be accessed through a method
    pub token: Token,
}

impl Sequence {
    pub fn new(token: Token) -> Self {
        Sequence {
            t_min: Beat(0.0),
            t_max: Beat(DEFAULT_LOOP_LEN.as_secs()),
            time_quantum: default_time_quantum(),
            exclusions: Rythm::Rd(RdRythm::default()),
            inclusions: Rythm::Rd(RdRythm::default()),
            beat_offset: default_beat_offset(),
            interval: Interval::RDTempered(2, vec![-7, 0, 7], 0),
            wave_type: WaveType::Sine,
            note_variant: NoteVariant::default(),
            harmonics: HarmonicsParams::default(),
            token,
            not_generate_until: None,
            loop_len: default_loop_len(),
            tail_multiplier: default_tail_multiplier(),
            tolerance: default_tolerance(),
            repeat: default_repeat(),
            loop_offset: default_loop_offset(),
            accents: default_accents(),
            shuffle: default_shuffle(),
            harmonise: default_harmonise(),
            harmoniser: default_harmoniser(),
            skip_harmonised: default_skip_harmonised(),
            melodiser: default_interval_affinities(),
            replicator: default_replicator_enabled(),
            replicator_steps: default_replicator_steps(),
            replicator_affinity_legacy: default_interval_affinities(),
            replicator_distance_legacy: None,
            glide: default_glide(),
            melodise: default_melodise(),
            melody_order_affinity: default_melody_order_affinity(),
            chord: default_tension(),
            random_chord: default_random_chord(),
            arpegio: default_arpegio(),
            reverse_prob: default_reverse_prob(),
            shuffle_prob: default_shuffle_prob(),
            legacy_params: LegacySequenceParams::default(),
        }
    }
    pub fn draw(
        &self,
        notes_buffer: &mut BTreeMap<Token, NotesGroup>,
        rng: &mut rand::prelude::ThreadRng,
        seq_start: Time,
        pan: f64,
        tempo: Tempo,
        note_id_gen: &mut NoteIdGen,
        params: &ResolvedTrackParams,
        rhythm: &RhythmParams,
        harmony: &HarmonyParams,
        wave: WaveType,
        delays_seconds: &(Vec<DelayTapSeconds>, Vec<DelayTapSeconds>),
    ) {
        // Volume is always 1.0 for note generation - actual volume is applied separately
        let volume = 1.0;
        let inclusions = match &rhythm.inclusions {
            Rythm::Rd(rd_rythm) => sample(rng, rd_rythm.length, rd_rythm.amount)
                .into_iter()
                .map(|k| k + 1)
                .collect(),
            Rythm::Det(det_rythm) => det_rythm.generators.clone(), // TODO: remove this clone if possible
        }
        .into_iter()
        .collect_vec();
        let exclusions = match &rhythm.exclusions {
            Rythm::Rd(rd_rythm) => sample(rng, rd_rythm.length, rd_rythm.amount)
                .into_iter()
                .map(|k| k + 2)
                .collect(),
            Rythm::Det(det_rythm) => det_rythm.generators.clone(), // TODO: remove this clone if possible
        }
        .into_iter()
        .collect_vec();
        let step_in_beats = rhythm.time_quantum.beat_step();
        let limit_in_beats = rhythm.t_max.min(rhythm.loop_len);
        let mut windows: Vec<(Beat, Beat)> = Vec::new();
        let step_value = step_in_beats.as_beats();
        let available = limit_in_beats - rhythm.t_min;
        if step_value.is_finite() && step_value > 0.0 && available.as_beats() > 0.0 {
            let ratio = available.as_beats() / step_in_beats.as_beats();
            let estimated_steps = (ratio.ceil() as usize).saturating_add(1);
            let mut starts = Vec::new();
            starts.reserve(estimated_steps.min(4096));
            for raw_idx in 0..estimated_steps {
                let step_idx = match i32::try_from(raw_idx) {
                    Ok(value) => value,
                    Err(_) => break,
                };
                let beat_time = rhythm.t_min + step_in_beats * raw_idx as f64;
                if beat_time >= limit_in_beats {
                    break;
                }
                if !inclusions
                    .iter()
                    .any(|p| (step_idx - rhythm.beat_offset) % (*p as i32) == 0)
                {
                    continue;
                }
                if exclusions
                    .iter()
                    .any(|s| (step_idx + 1 - rhythm.beat_offset) % (*s as i32) == 0)
                {
                    continue;
                }
                starts.push(beat_time);
            }
            if !starts.is_empty() {
                windows.reserve(starts.len());
                for idx in 0..starts.len() {
                    let current = starts[idx];
                    let next = if idx + 1 < starts.len() {
                        starts[idx + 1]
                    } else {
                        rhythm.t_max
                    };
                    let mut duration = next - current;
                    if idx + 1 == starts.len() {
                        duration *= rhythm.tail_multiplier;
                    }
                    windows.push((current, duration));
                }
            }
        }
        if harmony.shuffle {
            windows.shuffle(rng);
        }
        let step_as_time = tempo.beats_to_time(step_in_beats);
        let seq_start_beats = tempo.time_to_beats(seq_start);
        let loop_len_time = tempo.beats_to_time(rhythm.loop_len);
        let base_triggers: Vec<Note> = windows
            .into_iter()
            .map(|(t, d)| {
                let time_offset = tempo.beats_to_time(t);
                let duration_time = tempo.beats_to_time(d);
                let note_start = seq_start + time_offset;
                Note {
                    id: note_id_gen.next(),
                    time: note_start,
                    duration: duration_time,
                    beat_time: seq_start_beats + t,
                    beat_duration: d,
                    interval: harmony.interval.clone(),
                    glide: None,
                    volume: volume / params.envelope.normalization
                        * (self.accents.0 + 0.5 * self.accents.1.iter().sum::<f64>())
                        / (self.accents.0
                            + self
                                .accents
                                .1
                                .iter()
                                .map(|a| (note_start * *a).as_secs().fract())
                                .sum::<f64>()),
                    chord: harmony.chord,
                    random_chord: harmony.random_chord,
                    reverse_prob: harmony.reverse_prob,
                    shuffle_prob: harmony.shuffle_prob,
                    variant: self.note_variant,
                    harmonics: params.harmonics.clone(),
                }
            })
            .collect();
        let mut self_ctx = vec![];
        let mut legacy_steps = Vec::new();
        if self.replicator_steps.is_empty() && !self.replicator_affinity_legacy.is_empty() {
            legacy_steps.push(ReplicatorStep {
                distance: self
                    .replicator_distance_legacy
                    .unwrap_or_else(default_replicator_distance),
                affinities: self.replicator_affinity_legacy.clone(),
            });
        }
        let replicator_steps = if !legacy_steps.is_empty() {
            &legacy_steps
        } else {
            &self.replicator_steps
        };
        let base_notes: Vec<Note> = base_triggers
            .into_iter()
            .flat_map(|n| {
                n.draw(
                    &notes_buffer,
                    &mut self_ctx,
                    rng,
                    note_id_gen,
                    self.harmonise,
                    self.harmoniser,
                    self.skip_harmonised,
                    self.melodise,
                    &self.melodiser,
                    self.replicator,
                    replicator_steps,
                    self.melody_order_affinity,
                    self.tolerance,
                    step_as_time,
                    step_in_beats,
                    self.arpegio,
                )
            })
            .collect();

        let mut tmp: Vec<Note> = Vec::new();
        for note in base_notes {
            tmp.push(note.clone());
            for i in 1..rhythm.repeat {
                let mut clone = note.clone();
                clone.id = note_id_gen.next();
                clone.time += loop_len_time * i as f64;
                clone.beat_time += rhythm.loop_len * i as f64;
                tmp.push(clone);
            }
        }
        let tmp = if harmony.glide {
            let mut tmp = tmp;
            if tmp.len() > 1 {
                for i in 0..tmp.len() - 1 {
                    let next_interval = tmp[i + 1].interval.clone();
                    tmp[i].glide = Some(next_interval);
                }
            }
            tmp
        } else {
            tmp
        };
        if let Some(ng) = notes_buffer.get_mut(&self.token) {
            // Update all parameters every time we regenerate to ensure Sequence is the source of truth.
            // Volume is NOT set here - it will be computed separately based on tree structure.
            ng.notes.extend(tmp);
            ng.pan = pan;
            ng.bend = (params.bend.magnitude, params.bend.speed);
            ng.vibrato = (params.vibrato.magnitude, params.vibrato.frequency);
            ng.wave_type = wave;
            ng.chorus = params.chorus.clone();
            ng.attack_decay = (params.envelope.attack, params.envelope.decay);
            ng.lp_attack_decay = params.lowpass.envelope;
            ng.power = params.power.power;
            ng.tolerance = harmony.tolerance;
            ng.cutoff = params.lowpass.cutoff;
            ng.lowpass_enabled = params.lowpass.enabled;
            ng.lp_order = params.lowpass.order;
            ng.filter_type = params.lowpass.filter_type;
            ng.harmonics = params.harmonics.clone();
            ng.delays = delays_seconds.clone();
        } else {
            notes_buffer.insert(
                self.token,
                NotesGroup {
                    bend: (params.bend.magnitude, params.bend.speed),
                    vibrato: (params.vibrato.magnitude, params.vibrato.frequency),
                    notes: tmp,
                    wave_type: wave,
                    chorus: params.chorus.clone(),
                    harmonics: params.harmonics.clone(),
                    attack_decay: (params.envelope.attack, params.envelope.decay),
                    lp_attack_decay: params.lowpass.envelope,
                    power: params.power.power,
                    noise: params.noise.noise,
                    pan,
                    volume: 1.0, // Initial volume - will be updated by volume application mechanism
                    tolerance: harmony.tolerance,
                    cutoff: params.lowpass.cutoff,
                    lowpass_enabled: params.lowpass.enabled,
                    lp_order: params.lowpass.order,
                    filter_type: params.lowpass.filter_type,
                    delays: delays_seconds.clone(),
                },
            );
        }
    }

    /// Core drawing for a single sequence.
    /// Volume is NOT passed here - notes are generated at 1.0.
    pub fn draw_sequence_core(
        &mut self,
        notes: &mut BTreeMap<Token, NotesGroup>,
        rng: &mut rand::rngs::ThreadRng,
        now: Time,
        pan: f64,
        proba: Probability,
        tempo: Tempo,
        note_id_gen: &mut NoteIdGen,
        params: &ResolvedTrackParams,
        rhythm: &RhythmParams,
        harmony: &HarmonyParams,
        wave: WaveType,
        delays_seconds: &(Vec<DelayTapSeconds>, Vec<DelayTapSeconds>),
    ) -> Option<Time> {
        let base = now + GENERATE_EARLY;
        let loop_len_time = tempo.beats_to_time(rhythm.loop_len);
        let loop_offset_time = tempo.beats_to_time(rhythm.loop_offset);
        // Adjust base for the offset, then calculate which loop iteration we're in
        let adjusted_base = base - loop_offset_time;
        let start = loop_offset_time + loop_len_time * (adjusted_base / loop_len_time).floor();

        if self
            .not_generate_until
            .as_ref()
            .map_or(true, |until| now >= *until)
        {
            let generated = if rng.gen_bool(proba.as_f64()) {
                self.draw(
                    notes,
                    rng,
                    start,
                    pan,
                    tempo,
                    note_id_gen,
                    params,
                    rhythm,
                    harmony,
                    wave,
                    delays_seconds,
                );
                true
            } else {
                false
            };
            self.not_generate_until = Some(
                start + tempo.beats_to_time(rhythm.t_min) + loop_len_time * rhythm.repeat as f64
                    - GENERATE_EARLY,
            );
            if generated {
                return Some(start + loop_len_time - GENERATE_EARLY);
            }
        }
        None
    }

    pub(crate) fn take_legacy_params(&mut self) -> LegacySequenceParams {
        std::mem::take(&mut self.legacy_params)
    }
}

#[derive(Deserialize, Clone, Default, PartialEq)]
pub(crate) struct LegacySequenceParams {
    #[serde(default)]
    pub bend: Option<(f64, f64)>,
    #[serde(default)]
    pub vibrato: Option<(f64, Freq)>,
    #[serde(default)]
    pub chorus: Option<ChorusParams>,
    #[serde(default)]
    pub attack_decay: Option<(f64, f64)>,
    #[serde(default)]
    pub lp_attack_decay: Option<(f64, f64)>,
    #[serde(default)]
    pub cutoff_multiplier: Option<f64>,
    #[serde(default)]
    pub lp_relaxation: Option<LowpassRelaxation>,
    #[serde(default)]
    pub lp_lfo: Option<LowpassLfo>,
    #[serde(default)]
    pub lowpass_enabled: Option<bool>,
    #[serde(default)]
    pub lp_order: Option<u32>,
    #[serde(default)]
    pub pow_fact: Option<(f64, Freq)>,
    #[serde(default)]
    pub normalization: Option<f64>,
}

impl Serialize for LegacySequenceParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let map = serializer.serialize_map(Some(0))?;
        map.end()
    }
}
