use rand::prelude::SliceRandom;
use std::iter::once;

use itertools::Itertools;
use serde::Deserialize;

use crate::waves::WaveType;

#[derive(Deserialize, Clone)]
pub enum Instrument {
    Notes {
        t: f32,
        ns: Vec<Note>,
    },
    Grid {
        scale: (u32, i32, Vec<i32>),
        chords: Vec<(f32, f32, Vec<usize>)>,
        wave_type: WaveType,
    },
    Sequence {
        t_min: f32,
        t_max: f32,
        step: f32,
        skips: Vec<u32>,
        f: Interval,
        w: WaveType,
    },
}

#[derive(Deserialize, Clone)]
pub struct Note {
    pub t: f32,
    pub d: f32,
    pub f: Interval,
    pub w: WaveType,
}

#[derive(Deserialize, Clone)]
pub enum Interval {
    /// (degree, number_of_steps, octave)
    Tempered(i32, u32, i32),
    /// (n rd steps, degree, steps, base, octave)
    RDTempered(u32, i32, u32, Vec<i32>, i32),
}

impl Instrument {
    pub fn draw(&self, notes: &mut Vec<Note>, rng: &mut rand::prelude::ThreadRng) {
        match self {
            Instrument::Notes { t, ns } => {
                ns.iter()
                    .for_each(|n| notes.push(n.draw(notes, rng).shift(*t)));
            }
            Instrument::Grid {
                scale,
                chords,
                wave_type,
            } => {
                chords
                    .iter()
                    .flat_map(|(time, duration, degrees)| {
                        degrees.iter().map(|d| {
                            let tmp = scale.2[d % scale.2.len()];
                            Note {
                                t: *time,
                                d: *duration,
                                f: Interval::Tempered(tmp, scale.0, scale.1),
                                w: *wave_type,
                            }
                        })
                    })
                    .for_each(|n| notes.push(n));
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
                    .map(|i| t_min + i as f32 * step)
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
            Interval::RDTempered(n, degree, steps, base, octave) => {
                let other_notes = notes
                    .iter()
                    .filter(|n| (n.t - self.t).abs() < n.d + self.d)
                    .filter_map(|n| {
                        if let Interval::Tempered(d, _, _) = n.f {
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
                degree %= *steps as i32;
                Self {
                    f: Interval::Tempered(degree, *steps, *octave),
                    ..(*self)
                }
            }
            _ => self.clone(),
        }
    }
    pub fn shift(self, dt: f32) -> Self {
        Self {
            t: self.t + dt,
            ..self
        }
    }
}

impl Interval {
    pub fn compute(self) -> f32 {
        match self {
            Interval::Tempered(degree, steps, octave) => {
                (degree as f32 / steps as f32 + octave as f32).exp2()
            }
            _ => panic!(),
        }
    }
}
