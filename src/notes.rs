use rand::{prelude::SliceRandom, seq::index::sample};
use std::iter::once;

use itertools::Itertools;
use serde::Deserialize;

use crate::waves::WaveType;

#[derive(Deserialize, Clone)]
pub struct Sequence {
    pub t_min: f64,
    pub t_max: f64,
    pub step: [usize; 2],
    pub skips: (usize, usize),

    #[serde(default = "default_beat_offset")]
    pub beat_offset: usize, // WARNING: relative to step

    pub f: Interval,
    pub w: WaveType,
}

fn default_beat_offset() -> usize {
    0
}
#[derive(Deserialize, Clone)]
pub struct Note {
    pub t: f64,
    pub d: f64,
    pub f: Interval,
    pub w: WaveType,
}

#[derive(Deserialize, Clone)]
pub enum Interval {
    /// (degree, octave)
    Tempered(i32, i32),
    /// (n rd steps, degree, base, octave)
    RDTempered(u32, i32, Vec<i32>, i32),
}

impl Sequence {
    pub fn draw(&self, notes: &mut Vec<Note>, rng: &mut rand::prelude::ThreadRng) {
        let skips = sample(rng, self.skips.1, self.skips.0)
            .into_iter()
            .map(|k| k + 2)
            .collect_vec();
        // let skips = self.skips.clone();
        // println!("{skips:?}");
        let step_as_time = self.step[0] as f64 / self.step[1] as f64;
        let ts = (0..)
            .filter(|i| skips.iter().all(|s| (i + 1 + self.beat_offset) % s != 0))
            .map(|i| self.t_min + i as f64 * step_as_time)
            .take_while(|t| *t <= self.t_max);
        let ds = ts
            .clone()
            .chain(once(self.t_max))
            .tuple_windows()
            .map(|(t1, t2)| t2 - t1)
            .collect::<Vec<_>>();
        ts.zip(ds.iter())
            .map(|(t, d)| Note {
                t,
                d: *d,
                f: self.f.clone(),
                w: self.w,
            })
            .for_each(|n| notes.push(n.draw(notes, rng)));
    }
}

impl Note {
    pub fn draw(&self, notes: &[Note], rng: &mut rand::prelude::ThreadRng) -> Self {
        match &self.f {
            Interval::RDTempered(n, degree, base, octave) => {
                let other_notes = notes
                    .iter()
                    .filter(|n| ((n.t - self.t).abs() < n.d + self.d + 2.0))
                    .filter_map(|n| {
                        if let Interval::Tempered(d, _) = n.f {
                            Some(d)
                        } else {
                            None
                        }
                    })
                    .collect_vec();

                let mut degree = if other_notes.is_empty() {
                    (0..*n).fold(*degree, |acc, _| acc + base.choose(rng).unwrap())
                } else {
                    let tmp = other_notes.choose(rng).unwrap();
                    (0..*n).fold(*tmp, |acc, _| acc + base.choose(rng).unwrap())
                };
                degree %= 12;
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
