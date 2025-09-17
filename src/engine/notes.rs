use crate::{engine::waves::WaveType, DEFAULT_LOOP_LEN};
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
        Self {
            generators: vec![2],
        }
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
    pub pow_fact: f64,
    pub token: usize,
    pub not_generate_until: Option<f64>,
    pub loop_len: f64,
    #[serde(default = "default_spacial")]
    pub spacial: f64,
    #[serde(default = "default_tolerance")]
    pub tolerance: (f64, f64),
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

fn default_attack_decay() -> (f64, f64) {
    (4.0, 0.3333)
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ChorusParams {
    pub number_of_heads: usize,
    pub delta: f64,
    pub sym: f64,
    pub asym: f64,
    pub time_dependency: f64,
}

impl ChorusParams {
    pub fn new(
        number_of_heads: usize,
        delta: f64,
        sym: f64,
        asym: f64,
        time_dependency: f64,
    ) -> Self {
        Self {
            number_of_heads,
            delta,
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
    pub wave_type: WaveType,
    pub volume: f64,
    pub attack_decay: (f64, f64),
    pub bend: (f64, f64),
    pub vibrato: (f64, f64),
    pub chorus: ChorusParams,
    pub pow_fact: f64,
    pub spacial: f64,
    pub tolerance: (f64, f64),
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
    pub fn new(token: usize) -> Self {
        Sequence {
            t_min: 0.0,
            t_max: DEFAULT_LOOP_LEN,
            time_quantum: (1, 6),
            exclusions: Rythm::Rd(RdRythm::default()),
            inclusions: Rythm::Rd(RdRythm::default()),
            beat_offset: default_beat_offset(),
            interval: Interval::RDTempered(2, vec![-7, 0, 7], 0),
            wave_type: WaveType::Sine,
            volume: default_volume(),
            attack_decay: default_attack_decay(),
            token,
            not_generate_until: None,
            bend: (0.0, 32.0),
            vibrato: (0.0, 32.0),
            chorus: ChorusParams::new(1, 1e-2, 0.5, 0.0, 0.0),
            pow_fact: 0.0,
            loop_len: DEFAULT_LOOP_LEN,
            spacial: default_spacial(),
            tolerance: default_tolerance(),
        }
    }
    pub fn draw(
        &self,
        out: &mut Vec<(usize, Vec<Note>)>,
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
        let ts = (0..)
            .filter(|i| inclusions.iter().any(|p| (i - self.beat_offset) % p == 0))
            .filter(|i| {
                exclusions
                    .iter()
                    .all(|s| (i + 1 - self.beat_offset) % s != 0)
            })
            .map(|i| self.t_min + i as f64 * step_as_time)
            .take_while(|t| *t <= self.t_max.min(self.loop_len));
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
                wave_type: self.wave_type,
                volume: self.volume,
                attack_decay: self.attack_decay,
                bend: self.bend,
                vibrato: self.vibrato,
                chorus: self.chorus.clone(),
                pow_fact: self.pow_fact,
                spacial: self.spacial,
                tolerance: self.tolerance,
            })
            .for_each(|n| {
                let to_push = n.draw(&out, rng);
                if let Some((_, v)) = out.iter_mut().find(|(token, _)| *token == self.token) {
                    v.push(to_push);
                } else {
                    out.push((self.token, vec![to_push]));
                }
            });
    }
}

impl Note {
    pub fn draw(&self, context: &[(usize, Vec<Note>)], rng: &mut rand::prelude::ThreadRng) -> Self {
        match &self.interval {
            Interval::RDTempered(degree, base, octave) => {
                let others = context
                    .iter()
                    .flat_map(|(_, ns)| ns)
                    // Proper interval overlap test:
                    .filter(|n| {
                        self.time < n.time + n.duration + self.tolerance.0
                            && n.time < self.time + self.duration + self.tolerance.1
                    })
                    .filter_map(|n| {
                        if let Interval::Tempered(d, _) = n.interval {
                            Some(d)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                let seed = *others.choose(rng).unwrap_or(&0);
                let degree =
                    (((0..*degree).fold(seed, |acc, _| acc + base.choose(rng).unwrap()) % 12) + 12)
                        % 12;

                Self {
                    interval: Interval::Tempered(degree, *octave),
                    chorus: self.chorus.clone(),
                    ..*self
                }
            }
            _ => self.clone(),
        }
    }
}
