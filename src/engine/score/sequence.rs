use std::iter::once;

use itertools::Itertools;
use rand::seq::index::sample;
use rand::seq::SliceRandom;
use serde::Deserialize;
use serde::Serialize;

use crate::app::NotesGroup;
use crate::engine::score::default_params;
use crate::engine::score::note::Note;
use crate::engine::score::RdRythm;
use crate::time_freq::Freq;

use crate::engine::waves::WaveType;
use crate::Token;
use crate::DEFAULT_LOOP_LEN;
use crate::GLOBAL_VOLUME;

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
    #[serde(default = "default_beat_offset")]
    pub beat_offset: usize,
    #[serde(default = "default_volume")]
    pub volume: f64,
    #[serde(default = "default_mute")]
    pub mute: bool,
    #[serde(default = "default_normalization")]
    pub normalization: f64,
    #[serde(default = "default_attack_decay")]
    pub attack_decay: (f64, f64),
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
    #[serde(default = "default_repeat")]
    pub repeat: usize,
    #[serde(default = "default_accents")]
    pub accents: (f64, Vec<f64>),
    #[serde(default = "default_shuffle")]
    pub shuffle: bool,
    pub not_generate_until: Option<Time>, // TODO: should be accessed through a method
    pub token: Token,
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
        }
    }
    pub fn draw(
        &self,
        notes_buffer: &mut Vec<NotesGroup>,
        rng: &mut rand::prelude::ThreadRng,
        seq_start: Time,
        // tempo: f64,
    ) {
        if self.mute {
            return;
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
        let ts =
            (0..32768) // FIXME: do better
                .filter(|i| inclusions.iter().any(|p| (i - self.beat_offset) % p == 0))
                .filter(|i| {
                    exclusions
                        .iter()
                        .all(|s| (i + 1 - self.beat_offset) % s != 0)
                })
                .map(|i| self.t_min + step_as_time * i as f64)
                .take_while(|t| *t < self.t_max.min(self.loop_len));
        let ds = ts
            .clone()
            .chain(once(self.t_max))
            .tuple_windows()
            .map(|(t1, t2)| t2 - t1)
            .collect::<Vec<_>>();
        let mut tmp = ts.zip(ds.iter()).collect::<Vec<_>>();
        if self.shuffle {
            tmp.shuffle(rng);
        }
        tmp.into_iter()
            .map(|(t, d)| Note {
                time: t + seq_start,
                duration: *d,
                interval: self.interval.clone(),
                volume: GLOBAL_VOLUME / self.normalization
                    * (self.accents.0 + 0.5 * self.accents.1.iter().sum::<f64>())
                    / (self.accents.0
                        + self
                            .accents
                            .1
                            .iter()
                            .map(|a| ((t + seq_start) * *a).as_secs().fract())
                            .sum::<f64>()),
            })
            .for_each(|n| {
                let to_push = n.draw(&notes_buffer, rng);
                if let Some(NotesGroup { notes, .. }) = notes_buffer
                    .iter_mut()
                    .find(|NotesGroup { token, .. }| *token == self.token)
                {
                    for p in (0..self.repeat).map(|i| {
                        let tmp = to_push.clone();
                        Note {
                            time: tmp.time + self.loop_len * i as f64,
                            ..tmp
                        }
                    }) {
                        notes.push(p);
                    }
                    // v.push(to_push);
                } else {
                    for p in (0..self.repeat).map(|i| {
                        let tmp = to_push.clone();
                        Note {
                            time: tmp.time + self.loop_len * i as f64,
                            ..tmp
                        }
                    }) {
                        notes_buffer.push(NotesGroup {
                            token: self.token,
                            bend: self.bend,
                            vibrato: self.vibrato,
                            notes: vec![p],
                            wave_type: self.wave_type.clone(),
                            chorus: self.chorus.clone(),
                            attack_decay: self.attack_decay,
                            pow_fact: self.pow_fact,
                            spacial: self.spacial,
                            volume: self.volume,
                            tolerance: self.tolerance,
                        });
                    }
                }
            });
    }
}
