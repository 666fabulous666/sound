use crate::{app::NotesGroup, engine::waves::WaveType, Token, DEFAULT_LOOP_LEN, GLOBAL_VOLUME};
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
impl Default for DetRythm {
    fn default() -> Self {
        Self { generators: vec![] }
    }
}
impl Default for RdRythm {
    fn default() -> Self {
        Self {
            amount: 5,
            length: 10,
        }
    }
}
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Rythm {
    Rd(RdRythm),
    Det(DetRythm),
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Sequence {
    pub t_min: f64,
    pub t_max: f64,
    pub time_quantum: (usize, usize), // TODO: use proper fractions
    pub inclusions: Rythm,
    pub exclusions: Rythm,
    #[serde(default = "default_beat_offset")]
    pub beat_offset: usize, // WARNING: relative to step
    pub interval: Interval,
    pub wave_type: WaveType,
    #[serde(default = "default_volume")]
    pub volume: f64,
    #[serde(default = "default_attack_decay")]
    pub attack_decay: (f64, f64),
    pub bend: (f64, f64),
    pub vibrato: (f64, f64),
    pub chorus: ChorusParams,
    pub pow_fact: (f64, f64),
    pub token: Token,
    pub not_generate_until: Option<f64>, // TODO: should be accessed through a method
    pub loop_len: f64,
    #[serde(default = "default_spacial")]
    pub spacial: f64,
    #[serde(default = "default_tolerance")]
    pub tolerance: (f64, f64),
    pub repeat: usize,
    #[serde(default = "default_accents")]
    pub accents: (f64, Vec<f64>),
}

pub fn default_pow_fact() -> (f64, f64) {
    (1.0, 0.0)
}

pub fn default_attack_decay() -> (f64, f64) {
    (4.0, 0.3333)
}

pub fn default_drum_attack_decay() -> (f64, f64) {
    (100.0, 100.0)
}

pub fn default_bend() -> (f64, f64) {
    (0.0, 32.0)
}

pub fn default_vibrato() -> (f64, f64) {
    (0.0, 12.0)
}

fn default_accents() -> (f64, Vec<f64>) {
    (1.0, vec![0.5, 1.2, 2.5, 3.0])
}

fn default_spacial() -> f64 {
    0.5
}

fn default_tolerance() -> (f64, f64) {
    (1.0, 0.0)
}

fn default_beat_offset() -> usize {
    0
}

fn default_volume() -> f64 {
    5.0
}
fn default_delta_shift() -> f64 {
    0.0
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ChorusParams {
    pub voices: usize,
    pub delta: f64,
    #[serde(default = "default_delta_shift")]
    pub delta_shift: f64,
    pub sym: f64,
    pub asym: f64,
    pub time_dependency: f64,
}
impl Default for ChorusParams {
    fn default() -> Self {
        Self {
            voices: 1,
            delta: 0.02,
            delta_shift: 0.15,
            sym: 0.5,
            asym: 0.0,
            time_dependency: 0.0,
        }
    }
}

impl ChorusParams {
    pub fn new(
        number_of_heads: usize,
        delta: f64,
        delta_noise: f64,
        sym: f64,
        asym: f64,
        time_dependency: f64,
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
    pub time: f64,
    pub duration: f64,
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
            t_min: 0.0,
            t_max: DEFAULT_LOOP_LEN,
            time_quantum: (1, 6),
            exclusions: Rythm::Rd(RdRythm::default()),
            inclusions: Rythm::Rd(RdRythm::default()),
            beat_offset: default_beat_offset(),
            // interval: Interval::RDTempered(2, vec![-7, 0, 7], 0),
            interval: Interval::RDTempered(2, vec![0, 5, 7], 0),
            wave_type: WaveType::Sine,
            volume: default_volume(),
            attack_decay: default_attack_decay(),
            token,
            not_generate_until: None,
            bend: default_bend(),
            vibrato: default_vibrato(),
            chorus: ChorusParams::default(),
            pow_fact: default_pow_fact(),
            loop_len: DEFAULT_LOOP_LEN,
            spacial: default_spacial(),
            tolerance: default_tolerance(),
            repeat: 1,
            accents: default_accents(),
        }
    }
    pub fn draw(
        &self,
        notes_buffer: &mut Vec<NotesGroup>,
        rng: &mut rand::prelude::ThreadRng,
        seq_start: f64,
    ) {
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
        let step_as_time = self.time_quantum.0 as f64 / self.time_quantum.1 as f64;
        let ts =
            (0..32768) // FIXME: do better
                .filter(|i| inclusions.iter().any(|p| (i - self.beat_offset) % p == 0))
                .filter(|i| {
                    exclusions
                        .iter()
                        .all(|s| (i + 1 - self.beat_offset) % s != 0)
                })
                .map(|i| self.t_min + i as f64 * step_as_time)
                .take_while(|t| *t < self.t_max.min(self.loop_len));
        let ds = ts
            .clone()
            .chain(once(self.t_max))
            .tuple_windows()
            .map(|(t1, t2)| t2 - t1)
            .collect::<Vec<_>>();
        ts.zip(ds.iter())
            .map(|(t, d)| Note {
                time: t + seq_start,
                duration: *d,
                interval: self.interval.clone(),
                volume: GLOBAL_VOLUME
                    * self.volume
                    * (self.accents.0 + 0.5 * self.accents.1.iter().sum::<f64>())
                    / (self.accents.0
                        + self
                            .accents
                            .1
                            .iter()
                            .map(|a| (a * (t + seq_start)).fract())
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
                            time: tmp.time + i as f64 * self.loop_len,
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
                            time: tmp.time + i as f64 * self.loop_len,
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
