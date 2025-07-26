use rand::prelude::SliceRandom;
use std::iter::once;

use itertools::Itertools;
use serde::Deserialize;

use crate::waves::WaveType;

#[derive(Deserialize, Clone)]
pub enum Instrument {
    Notes {
        t: f64,
        ns: Vec<Note>,
    },
    Sequence {
        t_min: f64,
        t_max: f64,
        step: f64,
        skips: Vec<u32>,
        f: Interval,
        w: WaveType,
    },
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

impl Instrument {
    pub fn draw(&self, notes: &mut Vec<Note>, rng: &mut rand::prelude::ThreadRng) {
        match self {
            Instrument::Notes { t, ns } => {
                ns.iter()
                    .for_each(|n| notes.push(n.draw(notes, rng).shift(*t)));
            }
            Instrument::Sequence {
                t_min,
                t_max,
                step,
                skips,
                f,
                w,
            } => {
                let ts = (0..)
                    .filter(|i| skips.iter().all(|s| (i + 1) % s != 0))
                    .map(|i| t_min + i as f64 * step)
                    .take_while(|t| t <= t_max);
                let ds = ts
                    .clone()
                    .chain(once(*t_max))
                    .tuple_windows()
                    .map(|(t1, t2)| t2 - t1)
                    .collect::<Vec<_>>();
                ts.zip(ds.iter())
                    .map(|(t, d)| Note {
                        t,
                        d: *d,
                        f: f.clone(),
                        w: *w,
                    })
                    .for_each(|n| notes.push(n.draw(notes, rng)));
            }
        }
    }
}

impl Note {
    pub fn draw(&self, notes: &[Note], rng: &mut rand::prelude::ThreadRng) -> Self {
        match &self.f {
            Interval::RDTempered(n, degree, base, octave) => {
                let other_notes = notes
                    .iter()
                    .filter(|n| (n.t - self.t).abs() < n.d + self.d - 0.0)
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
    pub fn shift(self, dt: f64) -> Self {
        Self {
            t: self.t + dt,
            ..self
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
