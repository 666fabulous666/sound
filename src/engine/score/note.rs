use rand::seq::SliceRandom;
use serde::Deserialize;

use super::Interval;

use crate::{engine::score::NotesGroup, time_freq::Time};

#[derive(Deserialize, Clone)]
pub struct Note {
    pub time: Time,
    pub duration: Time,
    pub interval: Interval,
    pub glide: Option<Interval>,
    pub volume: f64,
}
impl Note {
    pub fn draw(
        &self,
        context: &[NotesGroup],
        rng: &mut rand::prelude::ThreadRng,
        harmonise: bool,
        harmoniser: [[u32; 7]; 2],
    ) -> Self {
        match &self.interval {
            Interval::RDTempered(n_rd_steps, base, octave) => {
                let others = context
                    .iter()
                    .map(
                        |NotesGroup {
                             notes, tolerance, ..
                         }| {
                            notes
                                .iter()
                                .filter(|n| {
                                    self.time < n.time + n.duration + tolerance.0
                                        && n.time < self.time + self.duration + tolerance.1
                                })
                                .map(|n| {
                                    (
                                        n,
                                        self.time < n.time + n.duration
                                            && n.time < self.time + self.duration,
                                    )
                                })
                        },
                    )
                    .flatten()
                    .filter_map(|(n, overlap)| {
                        if let Interval::Tempered(d, _) = n.interval {
                            Some((d, overlap))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                let seed = others.choose(rng).unwrap_or(&(0, false)).0;

                let degree = if harmonise {
                    (0..12)
                        .min_by_key(|d| tension(others.iter().cloned(), *d, harmoniser))
                        .unwrap()
                } else {
                    (0..*n_rd_steps).fold(seed, |acc, _| acc + base.choose(rng).unwrap()) % 12
                };

                Self {
                    interval: Interval::Tempered(degree, *octave),
                    glide: self.glide.clone(),
                    ..*self
                }
            }
            _ => self.clone(),
        }
    }
}
fn tension(
    ns: impl Iterator<Item = (i32, bool)> + Clone,
    d: i32,
    harmoniser: [[u32; 7]; 2],
) -> u32 {
    ns.into_iter()
        .map(|(n, overlap)| tension2(n, d, overlap, harmoniser))
        .sum()
}
fn tension2(n1: i32, n2: i32, overlap: bool, harmoniser: [[u32; 7]; 2]) -> u32 {
    let d = dist12(n1, n2);
    if overlap {
        match d {
            0 => harmoniser[0][0],
            1 => harmoniser[0][1],
            2 => harmoniser[0][2],
            3 => harmoniser[0][3],
            4 => harmoniser[0][4],
            5 => harmoniser[0][5],
            6 => harmoniser[0][6],
            _ => unreachable!(),
        }
    } else {
        match d {
            0 => harmoniser[1][0],
            1 => harmoniser[1][1],
            2 => harmoniser[1][2],
            3 => harmoniser[1][3],
            4 => harmoniser[1][4],
            5 => harmoniser[1][5],
            6 => harmoniser[1][6],
            _ => unreachable!(),
        }
    }
}
fn dist12(n1: i32, n2: i32) -> i32 {
    let d = ((n1 - n2) % 12).abs();
    d.min(12 - d)
}
