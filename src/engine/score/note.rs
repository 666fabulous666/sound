use std::iter::once;

use itertools::Itertools;
use rand::seq::SliceRandom;
use serde::Deserialize;

use super::Interval;

use crate::{engine::score::NotesGroup, time_freq::Time};

#[derive(Deserialize, Clone)]
pub struct Note {
    pub time: Time,
    pub duration: Time,
    pub interval: Interval,
    pub volume: f64,
}
impl Note {
    pub fn draw(
        &self,
        context: &[NotesGroup],
        rng: &mut rand::prelude::ThreadRng,
        harmonise: bool,
    ) -> Self {
        match &self.interval {
            Interval::RDTempered(n_rd_steps, base, octave) => {
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
                    .filter_map(|n| {
                        if let Interval::Tempered(d, _) = n.interval {
                            Some(d)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                let seed = *others.choose(rng).unwrap_or(&0);

                let degree = if harmonise {
                    (0..12)
                        .min_by_key(|d| tension(others.iter().cloned().chain(once(*d))))
                        .unwrap()
                } else {
                    (0..*n_rd_steps).fold(seed, |acc, _| acc + base.choose(rng).unwrap()) % 12
                };

                Self {
                    interval: Interval::Tempered(degree, *octave),
                    ..*self
                }
            }
            _ => self.clone(),
        }
    }
}
fn tension(ns: impl Iterator<Item = i32> + Clone) -> u32 {
    ns.tuple_combinations()
        .map(|(n1, n2)| tension2(n1, n2))
        .sum()
}
fn tension2(n1: i32, n2: i32) -> u32 {
    let d = dist12(n1, n2);
    match d {
        // 0 => 10,
        // 1 => 11,
        // 2 => 9,
        // 3 => 4,
        // 4 => 3,
        // 5 => 0,
        // 6 => 8,
        // _ => unreachable!(),
        0 => 100,
        1 => 101,
        2 => 99,
        3 => 50,
        4 => 45,
        5 => 0,
        6 => 98,
        _ => unreachable!(),
    }
}
fn dist12(n1: i32, n2: i32) -> i32 {
    let d = ((n1 - n2) % 12).abs();
    d.min(12 - d)
}
