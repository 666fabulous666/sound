use rand::seq::SliceRandom;
use serde::Deserialize;

use super::Interval;

use crate::{app::NotesGroup, time_freq::Time};

#[derive(Deserialize, Clone)]
pub struct Note {
    pub time: Time,
    pub duration: Time,
    pub interval: Interval,
    pub volume: f64,
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
