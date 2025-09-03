use rand::{prelude::SliceRandom, seq::index::sample};
// NEW (used for light-weight fingerprints)
use std::iter::once;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{waves::WaveType, DEFAULT_LOOP_LEN};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Sequence {
    pub t_min: f64,
    pub t_max: f64,
    pub step: (usize, usize),
    pub skips: (usize, usize),
    #[serde(default = "default_beat_offset")]
    pub beat_offset: usize, // WARNING: relative to step
    pub interval: Interval,
    pub wave_type: WaveType,
    #[serde(default = "default_volume")]
    pub volume: f64,
    #[serde(default = "default_attack_decay")]
    pub attack_decay: (f64, f64),
    pub attack_freq_modulation: (f64, f64),
    pub vibrato: (f64, f64),
    pub chorus: ChorusParams,
    pub pow_fact: f64,
    pub token: usize,
    pub not_generate_until: Option<f64>,
    pub loop_len: f64,
    #[serde(default = "default_spacial")]
    pub spacial: f64,
}

fn default_spacial() -> f64 {
    0.5
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
    pub attack_freq_modulation: (f64, f64),
    pub vibrato: (f64, f64),
    pub chorus: ChorusParams,
    pub pow_fact: f64,
    pub spacial: f64,
    // loop_len: f64,
    // seq_start: f64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Interval {
    /// (degree, octave)
    Tempered(i32, i32),
    /// (n rd steps, base, octave)
    RDTempered(u32, Vec<i32>, i32),
}

impl Sequence {
    pub fn new(token: usize) -> Self {
        Sequence {
            t_min: 0.0,
            t_max: DEFAULT_LOOP_LEN,
            step: (1, 6),
            skips: (5, 10),
            beat_offset: default_beat_offset(),
            interval: Interval::RDTempered(2, vec![-7, 0, 7], 0),
            wave_type: WaveType::Sine,
            volume: default_volume(),
            attack_decay: default_attack_decay(),
            token,
            not_generate_until: None,
            attack_freq_modulation: (0.0, 32.0),
            vibrato: (0.0, 32.0),
            chorus: ChorusParams::new(1, 1e-2, 0.5, 0.0, 0.0),
            pow_fact: 0.0,
            loop_len: DEFAULT_LOOP_LEN,
            spacial: 0.5,
        }
    }
    pub fn draw(
        &self,
        notes: &mut Vec<(usize, Vec<Note>)>,
        rng: &mut rand::prelude::ThreadRng,
        seq_start: f64,
    ) {
        let skips = sample(rng, self.skips.1, self.skips.0)
            .into_iter()
            .map(|k| k + 2)
            .collect_vec();
        let step_as_time = self.step.0 as f64 / self.step.1 as f64;
        let ts = (0..)
            .filter(|i| skips.iter().all(|s| (i + 1 - self.beat_offset) % s != 0))
            .map(|i| self.t_min + i as f64 * step_as_time)
            .take_while(|t| *t <= self.t_max);
        let ds = ts
            .clone()
            .chain(once(self.t_max))
            .tuple_windows()
            .map(|(t1, t2)| t2 - t1)
            .collect::<Vec<_>>();
        let notes_from_seq: Vec<Note> = ts
            .zip(ds.iter())
            .map(|(t, d)| Note {
                time: t + seq_start,
                duration: *d,
                interval: self.interval.clone(),
                wave_type: self.wave_type,
                volume: self.volume,
                attack_decay: self.attack_decay,
                attack_freq_modulation: self.attack_freq_modulation,
                vibrato: self.vibrato,
                chorus: self.chorus.clone(),
                pow_fact: self.pow_fact,
                spacial: self.spacial,
                // loop_len: self.loop_len,
                // seq_start,
            })
            .map(|n| n.draw(notes, rng))
            .collect();
        notes.push((self.token, notes_from_seq));
    }
}

impl Note {
    pub fn draw(&self, notes: &[(usize, Vec<Note>)], rng: &mut rand::prelude::ThreadRng) -> Self {
        match &self.interval {
            Interval::RDTempered(degree, base, octave) => {
                // println!();
                let other_notes = notes
                    .iter()
                    .flat_map(|(_, n)| n)
                    .filter(|n| {
                        let condition =
                            (n.time - self.time).abs() < n.duration + self.duration + 1.0;
                        // println!("{}, {}, {}", n.time, self.time, condition);
                        condition
                    })
                    // .filter(|n| {
                    //     ((n.t % n.loop_len) - (self.t % self.loop_len)).abs() < n.d + self.d
                    // })
                    .filter_map(|n| {
                        if let Interval::Tempered(d, _) = n.interval {
                            Some(d)
                        } else {
                            None
                        }
                    })
                    .collect_vec();

                let tmp = other_notes.choose(rng).unwrap_or(&0);
                // let tmp = other_notes.choose(rng).unwrap_or_else(|| {
                //     println!("{}, {:.2}", self.time, self.spacial);
                //     &0
                // });
                let degree = (0..*degree).fold(*tmp, |acc, _| acc + base.choose(rng).unwrap()) % 12;
                // println!("---------------");
                Self {
                    interval: Interval::Tempered(degree, *octave),
                    chorus: self.chorus.clone(),
                    ..(*self)
                }
            }
            _ => self.clone(),
        }
    }
}

impl Interval {
    pub fn compute(&self) -> f64 {
        match self {
            Interval::Tempered(degree, octave) => (*degree as f64 / 12.0 + *octave as f64).exp2(),
            _ => panic!(),
        }
    }
}
// notes.rs
impl Sequence {
    pub fn draw_with_context(
        &self,
        out: &mut Vec<(usize, Vec<Note>)>,
        rng: &mut rand::prelude::ThreadRng,
        seq_start: f64,
        context: &[(usize, Vec<Note>)],
    ) {
        let skips = sample(rng, self.skips.1, self.skips.0)
            .into_iter()
            .map(|k| k + 2)
            .collect_vec();
        let step_as_time = self.step.0 as f64 / self.step.1 as f64;
        let ts = (0..)
            .filter(|i| skips.iter().all(|s| (i + 1 - self.beat_offset) % s != 0))
            .map(|i| self.t_min + i as f64 * step_as_time)
            .take_while(|t| *t <= self.t_max);
        let ds = ts
            .clone()
            .chain(once(self.t_max))
            .tuple_windows()
            .map(|(t1, t2)| t2 - t1)
            .collect::<Vec<_>>();
        let notes_from_seq: Vec<Note> = ts
            .zip(ds.iter())
            .map(|(t, d)| Note {
                time: t + seq_start,
                duration: *d,
                interval: self.interval.clone(),
                wave_type: self.wave_type,
                volume: self.volume,
                attack_decay: self.attack_decay,
                attack_freq_modulation: self.attack_freq_modulation,
                vibrato: self.vibrato,
                chorus: self.chorus.clone(),
                pow_fact: self.pow_fact,
                spacial: self.spacial,
            })
            .map(|n| n.draw_with_context(context, rng))
            .collect();
        out.push((self.token, notes_from_seq));
    }
}

impl Note {
    pub fn draw_with_context(
        &self,
        context: &[(usize, Vec<Note>)],
        rng: &mut rand::prelude::ThreadRng,
    ) -> Self {
        match &self.interval {
            Interval::RDTempered(degree, base, octave) => {
                const EPS: f64 = 1e-0;
                let others = context
                    .iter()
                    .flat_map(|(_, ns)| ns)
                    // Proper interval overlap test:
                    .filter(|n| {
                        n.time < self.time + self.duration + EPS
                            && self.time < n.time + n.duration + EPS
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
                let degree = (0..*degree).fold(seed, |acc, _| acc + base.choose(rng).unwrap()) % 12;

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
