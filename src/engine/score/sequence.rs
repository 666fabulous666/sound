use std::iter::once;

use itertools::Itertools;
use rand::seq::index::sample;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;

use crate::engine::score::default_params;
use crate::engine::score::note::Note;
use crate::engine::score::NotesGroup;
use crate::engine::score::RdRythm;
use crate::time_freq::Freq;

use crate::engine::waves::WaveType;
use crate::Token;
use crate::DEFAULT_LOOP_LEN;
use crate::GENERATE_EARLY;

use super::Interval;

use super::Rythm;

use super::ChorusParams;

use crate::time_freq::Time;
use default_params::*;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Sequence {
    pub t_min: Time,
    pub t_max: Time,
    pub chorus: ChorusParams,
    pub inclusions: Rythm,
    pub exclusions: Rythm,
    pub interval: Interval,
    pub wave_type: WaveType,
    #[serde(default = "default_time_quantum")]
    pub time_quantum: (usize, usize),
    #[serde(default = "default_proba")]
    pub proba: f64,
    #[serde(default = "default_harmoniser")]
    pub harmoniser: [[u32; 7]; 2],
    #[serde(default = "default_beat_offset")]
    pub beat_offset: i32,
    #[serde(default = "default_volume")]
    pub volume: f64,
    #[serde(default = "default_glide")]
    pub glide: bool,
    #[serde(default = "default_mute")]
    pub mute: bool,
    #[serde(default = "default_normalization")]
    pub normalization: f64,
    #[serde(default = "default_attack_decay")]
    pub attack_decay: (f64, f64),
    #[serde(default = "default_attack_decay")]
    pub lp_attack_decay: (f64, f64),
    #[serde(default = "default_cutoff_multiplier")]
    pub cutoff_multiplier: f64,
    #[serde(default = "default_bend")]
    pub bend: (f64, f64),
    #[serde(default = "default_vibrato")]
    pub vibrato: (f64, Freq),
    #[serde(default = "default_pow_fact")]
    pub pow_fact: (f64, Freq),
    #[serde(default = "default_loop_len")]
    pub loop_len: Time,
    #[serde(default = "default_spacial")]
    pub spacial: f64,
    #[serde(default = "default_tolerance")]
    pub tolerance: (Time, Time),
    #[serde(default = "default_harmonise")]
    pub harmonise: bool,
    #[serde(default = "default_repeat")]
    pub repeat: usize,
    #[serde(default = "default_accents")]
    pub accents: (f64, Vec<f64>),
    #[serde(default = "default_shuffle")]
    pub shuffle: bool,
    #[serde(default = "default_tension")]
    pub chord: usize,
    #[serde(default = "default_random_chord")]
    pub random_chord: bool,
    pub not_generate_until: Option<Time>, // TODO: should be accessed through a method
    pub token: Token,
    #[serde(default = "default_name")]
    pub name: String,
}

impl Sequence {
    pub fn new(token: Token) -> Self {
        Sequence {
            t_min: Time(0.0),
            t_max: DEFAULT_LOOP_LEN,
            time_quantum: default_time_quantum(),
            exclusions: Rythm::Rd(RdRythm::default()),
            inclusions: Rythm::Rd(RdRythm::default()),
            beat_offset: default_beat_offset(),
            interval: Interval::RDTempered(2, vec![-7, 0, 7], 0),
            wave_type: WaveType::Sine,
            volume: default_volume(),
            mute: default_mute(),
            attack_decay: default_attack_decay(),
            lp_attack_decay: default_attack_decay(),
            token,
            not_generate_until: None,
            bend: default_bend(),
            vibrato: default_vibrato(),
            chorus: ChorusParams::default(),
            pow_fact: default_pow_fact(),
            loop_len: default_loop_len(),
            spacial: default_spacial(),
            tolerance: default_tolerance(),
            repeat: default_repeat(),
            accents: default_accents(),
            normalization: default_normalization(),
            shuffle: default_shuffle(),
            name: format!("seq {}", token.to_string()),
            harmonise: default_harmonise(),
            harmoniser: default_harmoniser(),
            proba: default_proba(),
            glide: default_glide(),
            cutoff_multiplier: default_cutoff_multiplier(),
            chord: default_tension(),
            random_chord: default_random_chord(),
        }
    }
    pub fn draw(
        &self,
        notes_buffer: &mut Vec<NotesGroup>,
        rng: &mut rand::prelude::ThreadRng,
        seq_start: Time,
        mut volume: f64,
    ) {
        if self.mute {
            volume = 0.0;
        }
        let inclusions = match &self.inclusions {
            Rythm::Rd(rd_rythm) => sample(rng, rd_rythm.length, rd_rythm.amount)
                .into_iter()
                .map(|k| k + 1)
                .collect(),
            Rythm::Det(det_rythm) => det_rythm.generators.clone(), // TODO: remove this clone if possible
        }
        .into_iter()
        .collect_vec();
        let exclusions = match &self.exclusions {
            Rythm::Rd(rd_rythm) => sample(rng, rd_rythm.length, rd_rythm.amount)
                .into_iter()
                .map(|k| k + 2)
                .collect(),
            Rythm::Det(det_rythm) => det_rythm.generators.clone(), // TODO: remove this clone if possible
        }
        .into_iter()
        .collect_vec();
        let step_as_time = Time(self.time_quantum.0 as f64 / self.time_quantum.1 as f64);
        let ts = (0..32768) // FIXME: do better
            .filter(|i| {
                inclusions
                    .iter()
                    .any(|p| (i - self.beat_offset) % (*p as i32) == 0)
            })
            .filter(|i| {
                exclusions
                    .iter()
                    .all(|s| (i + 1 - self.beat_offset) % (*s as i32) != 0)
            })
            .map(|i| self.t_min + step_as_time * i as f64)
            .take_while(|t| *t < self.t_max.min(self.loop_len))
            .collect_vec();
        let mut ds = ts
            .clone()
            .into_iter()
            .chain(once(self.t_max))
            .tuple_windows()
            .map(|(t1, t2)| t2 - t1)
            .collect::<Vec<_>>();
        ds.last_mut().map(|d| *d * 3.0); // WARNING: harcoded 3.0 ...
        let mut tmp = ts.into_iter().zip(ds.iter()).collect::<Vec<_>>();
        if self.shuffle {
            tmp.shuffle(rng);
        }
        let tmp = tmp.into_iter().enumerate().map(|(n, (t, d))| Note {
            time: t + seq_start, // + Time(1e-3 * ((7.3 * n as f64) % 5.0)),
            duration: *d,
            interval: self.interval.clone(),
            glide: None,
            volume: volume / self.normalization
                * (self.accents.0 + 0.5 * self.accents.1.iter().sum::<f64>())
                / (self.accents.0
                    + self
                        .accents
                        .1
                        .iter()
                        .map(|a| ((t + seq_start) * *a).as_secs().fract())
                        .sum::<f64>()),
            tension: self.chord,
            random_chord: self.random_chord,
        });
        let mut self_ctx = vec![];
        let tmp: Vec<Note> = tmp
            .map(|n| {
                n.draw(
                    &notes_buffer,
                    &mut self_ctx,
                    rng,
                    self.harmonise,
                    self.harmoniser,
                    step_as_time,
                )
            })
            .flat_map(|to_push| {
                (0..self.repeat).flat_map(move |i| {
                    // let tmp = to_push.clone();
                    to_push
                        .clone()
                        .iter()
                        .map(move |tmp| Note {
                            time: tmp.time + self.loop_len * i as f64,
                            ..tmp.clone()
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let tmp = if self.glide {
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
        if let Some(NotesGroup { notes, .. }) = notes_buffer
            .iter_mut()
            .find(|NotesGroup { token, .. }| *token == self.token)
        {
            notes.extend(tmp);
        } else {
            notes_buffer.push(NotesGroup {
                token: self.token,
                bend: self.bend,
                vibrato: self.vibrato,
                notes: tmp,
                wave_type: self.wave_type.clone(),
                chorus: self.chorus.clone(),
                attack_decay: self.attack_decay,
                lp_attack_decay: self.lp_attack_decay,
                pow_fact: self.pow_fact,
                spacial: self.spacial,
                volume: self.volume,
                tolerance: self.tolerance,
                cutoff_multiplier: self.cutoff_multiplier,
            });
        }
    }

    /// Core drawing for a single sequence.
    pub fn draw_sequence_core(
        &mut self,
        notes: &mut Vec<NotesGroup>,
        rng: &mut rand::rngs::ThreadRng,
        now: Time,
        anticipate: bool,
        volume: f64,
    ) {
        let base = if anticipate {
            now + GENERATE_EARLY
        } else {
            now
        };
        let start = self.loop_len * (base / self.loop_len).floor();

        if self
            .not_generate_until
            .as_ref()
            .map_or(true, |until| now >= *until)
        {
            if rng.gen_bool(self.proba) {
                self.draw(notes, rng, start, volume);
            }
            self.not_generate_until =
                Some(start + self.t_min + self.loop_len * self.repeat as f64 - GENERATE_EARLY);
        }
    }
}
