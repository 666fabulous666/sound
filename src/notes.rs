use rand::{prelude::SliceRandom, seq::index::sample};
// NEW (used for light-weight fingerprints)
use std::iter::once;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{waves::WaveType, LOOP_LEN};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Sequence {
    pub t_min: f64,
    pub t_max: f64,
    pub step: (usize, usize),
    pub skips: (usize, usize),
    #[serde(default = "default_beat_offset")]
    pub beat_offset: usize, // WARNING: relative to step
    pub f: Interval,
    pub w: WaveType,
    #[serde(default = "default_volume")]
    pub volume: f64,
    #[serde(default = "default_attack_decay")]
    pub attack_decay: (f64, f64),
    pub attack_freq_modulation: (f64, f64),
    pub vibrato: (f64, f64),
    pub chorus: (usize, f64, f64),
    pub token: usize,
    pub not_generate_until: Option<f64>,
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

#[derive(Deserialize, Clone)]
pub struct Note {
    pub t: f64,
    pub d: f64,
    pub f: Interval,
    pub w: WaveType,
    pub volume: f64,
    pub attack_decay: (f64, f64),
    pub attack_freq_modulation: (f64, f64),
    pub vibrato: (f64, f64),
    pub chorus: (usize, f64, f64),
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
            t_max: LOOP_LEN,
            step: (1, 6),
            skips: (5, 10),
            beat_offset: default_beat_offset(),
            f: Interval::RDTempered(2, vec![-7, 0, 7], 0),
            w: WaveType::Xylo,
            volume: default_volume(),
            attack_decay: default_attack_decay(),
            token,
            not_generate_until: None,
            attack_freq_modulation: (0.0, 32.0),
            vibrato: (0.0, 32.0),
            chorus: (1, 1e-3, 0.5),
        }
    }
    pub fn draw(
        &self,
        notes: &mut Vec<(usize, Vec<Note>)>,
        rng: &mut rand::prelude::ThreadRng,
        start_time: f64,
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
                t: t + start_time,
                d: *d,
                f: self.f.clone(),
                w: self.w,
                volume: self.volume,
                attack_decay: self.attack_decay,
                attack_freq_modulation: self.attack_freq_modulation,
                vibrato: self.vibrato,
                chorus: self.chorus,
            })
            .map(|n| n.draw(notes, rng))
            .collect();
        notes.push((self.token, notes_from_seq));
    }
}

impl Note {
    pub fn draw(&self, notes: &[(usize, Vec<Note>)], rng: &mut rand::prelude::ThreadRng) -> Self {
        match &self.f {
            Interval::RDTempered(n, base, octave) => {
                let other_notes = notes
                    .iter()
                    .flat_map(|(_, n)| n)
                    .filter(|n| ((n.t - self.t).abs() < n.d + self.d + 2.0))
                    .filter_map(|n| {
                        if let Interval::Tempered(d, _) = n.f {
                            Some(d)
                        } else {
                            None
                        }
                    })
                    .collect_vec();

                let tmp = other_notes.choose(rng).unwrap_or(&0);
                let degree = (0..*n).fold(*tmp, |acc, _| acc + base.choose(rng).unwrap()) % 12;
                Self {
                    f: Interval::Tempered(degree, *octave),
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
