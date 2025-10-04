pub mod default_params;
use crate::{
    app::NotesGroup,
    engine::waves::WaveType,
    time_freq::{Freq, Time},
    Token, DEFAULT_LOOP_LEN, GLOBAL_VOLUME,
};
use default_params::*;
use itertools::Itertools;
use rand::{prelude::SliceRandom, seq::index::sample};
use serde::{Deserialize, Serialize};
use std::iter::once;

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

#[derive(Deserialize, Clone)]
pub struct Note {
    pub time: Time,
    pub duration: Time,
    pub interval: Interval,
    pub volume: f64,
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
            _ => panic!(),
        }
    }
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
            // interval: Interval::RDTempered(2, vec![0, 5, 7], 0),
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

impl Note {
    pub fn draw(&self, context: &[NotesGroup], rng: &mut rand::prelude::ThreadRng) -> Self {
        match &self.interval {
            Interval::RDTempered(degree, base, octave) => {
                let others = context
                    .iter()
                    .map(
                        |NotesGroup {
                             notes, tolerance, ..
                         }| {
                            notes.iter().filter(|n| {
                                self.time < n.time + n.duration + tolerance.0
                                    && n.time < self.time + self.duration + tolerance.1
                            })
                        },
                    )
                    .flatten()
                    // Proper interval overlap test:
                    .filter_map(|n| {
                        if let Interval::Tempered(d, _) = n.interval {
                            Some(d)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                let seed = *others.choose(rng).unwrap_or(&0);
                // let degree =
                //     (((0..*degree).fold(seed, |acc, _| acc + base.choose(rng).unwrap()) % 12) + 12)
                //         % 12;
                let degree = (0..*degree).fold(seed, |acc, _| acc + base.choose(rng).unwrap()) % 12;

                Self {
                    interval: Interval::Tempered(degree, *octave),
                    ..*self
                }
            }
            _ => self.clone(),
        }
    }
}
