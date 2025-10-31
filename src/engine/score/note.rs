use std::iter::once;

use itertools::{Combinations, Itertools};
use rand::{seq::SliceRandom, Rng};
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
    pub tension: usize,
    pub random_chord: bool,
    pub reverse_prob: f64,
}
impl Note {
    pub fn draw(
        &self,
        context: &[NotesGroup],
        self_ctx: &mut Vec<Note>,
        rng: &mut rand::prelude::ThreadRng,
        harmonise: bool,
        harmoniser: [[u32; 7]; 2],
        step_as_time: Time,
        arpegio: f64,
    ) -> Vec<Self> {
        match &self.interval {
            Interval::RDTempered(n_rd_steps, base, octave) => {
                // 1. Collect contextual notes from external context
                let mut others = context
                    .iter()
                    .flat_map(
                        |NotesGroup {
                             notes, tolerance, ..
                         }| {
                            notes
                                .iter()
                                .filter(|n| {
                                    // self.time < n.time + n.duration + tolerance.0
                                    //     && n.time < self.time + self.duration + tolerance.1
                                    self.time < n.time + tolerance.0
                                        && n.time < self.time + tolerance.1
                                })
                                .map(|n| {
                                    (
                                        n,
                                        // true overlap only if timing overlaps
                                        // self.time < n.time + n.duration
                                        //     && n.time < self.time + self.duration,
                                        (self.time - n.time).as_secs().abs(),
                                    )
                                })
                        },
                    )
                    .filter_map(|(n, overlap)| {
                        if let Interval::Tempered(d, _) = n.interval {
                            Some((d, overlap))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                // 2. Add notes from self_ctx (always non-overlapping)
                for n in self_ctx.iter() {
                    if let Interval::Tempered(d, _) = n.interval {
                        // others.push((d, false));
                        others.push((d, (self.time - n.time).as_secs().powi(2)));
                    }
                }

                // 3. Choose seed degree
                // let seed = others.choose(rng).unwrap_or(&(0, false)).0;
                let seed = others.choose(rng).unwrap_or(&(0, 0.0)).0;

                // 4. Compute new degrees
                let mut degree = if harmonise {
                    (-11..12i32)
                        .combinations(if self.random_chord {
                            rng.gen_range(1..=self.tension)
                        } else {
                            self.tension
                        })
                        .min_by_key(|d| {
                            tension_family(others.iter().cloned(), d.iter().cloned(), harmoniser)
                        })
                        .unwrap()
                } else {
                    vec![(0..*n_rd_steps).fold(seed, |acc, _| acc + base.choose(rng).unwrap()) % 12]
                };
                // degree.shuffle(rng);
                if rng.gen_bool(self.reverse_prob) {
                    // let degree = degree.into_iter().rev().collect_vec();
                    degree.reverse();
                }

                // 5. Build new notes and push into self_ctx
                let new_notes = degree
                    .iter()
                    .enumerate()
                    .map(|(n, d)| Self {
                        interval: Interval::Tempered(*d, *octave),
                        glide: self.glide.clone(),
                        time: self.time + (step_as_time * arpegio * n as f64),
                        ..*self
                    })
                    .collect::<Vec<_>>();

                self_ctx.extend(new_notes.iter().cloned());

                new_notes
            }
            _ => vec![self.clone()],
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
fn tension_family(
    // ns: impl IntoIterator<Item = (i32, bool)>,
    ns: impl IntoIterator<Item = (i32, f64)>,
    c: impl IntoIterator<Item = i32>,
    harmoniser: [[u32; 7]; 2],
) -> u32 {
    // Collect combinations so we can reuse them
    let combo_vals: Vec<i32> = c.into_iter().collect();

    // --- 1. sum tensions between ns and c ---
    let mut total = ns
        .into_iter()
        .map(|(n, overlap)| {
            combo_vals
                .iter()
                .copied()
                // .map(|d| tension2(n, d, overlap, harmoniser))
                .map(|d| (16 * tension2(n, d, true, harmoniser)) / (overlap as u32 + 1))
                .sum::<u32>()
        })
        .sum::<u32>();

    // --- 2. sum pairwise tensions within c itself ---
    for i in 0..combo_vals.len() {
        for j in (i + 1)..combo_vals.len() {
            total += tension2(combo_vals[i], combo_vals[j], true, harmoniser);
        }
    }

    total
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
