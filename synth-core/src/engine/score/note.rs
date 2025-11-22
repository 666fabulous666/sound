use std::cmp::Ordering;
use std::collections::BTreeMap;

use itertools::Itertools;
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

use super::Interval;

use crate::{
    engine::score::HarmonicsParams,
    engine::score::NotesGroup,
    time_freq::{Beat, Time},
    NoteId, NoteIdGen, Token,
};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoteVariant {
    #[default]
    PhaseTracked,
    PureTime,
}

#[derive(Deserialize, Clone)]
pub struct Note {
    #[serde(default)]
    pub id: NoteId,
    pub time: Time,
    pub duration: Time,
    #[serde(default)]
    pub beat_time: Beat,
    #[serde(default)]
    pub beat_duration: Beat,
    pub interval: Interval,
    pub glide: Option<Interval>,
    pub volume: f64,
    pub chord: usize,
    pub random_chord: bool,
    pub reverse_prob: f64,
    #[serde(default)]
    pub shuffle_prob: f64,
    #[serde(default)]
    pub variant: NoteVariant,
    #[serde(default)]
    pub harmonics: HarmonicsParams,
}
impl Note {
    pub fn draw(
        &self,
        context: &BTreeMap<Token, NotesGroup>,
        self_ctx: &mut Vec<Note>,
        rng: &mut rand::prelude::ThreadRng,
        note_id_gen: &mut NoteIdGen,
        harmonise: bool,
        harmoniser: [u32; 7],
        melodise: bool,
        melodiser: [i32; 7],
        melody_order_affinity: i32,
        tolerance: (Time, Time),
        step_as_time: Time,
        step_in_beats: Beat,
        arpegio: f64,
    ) -> Vec<Self> {
        match &self.interval {
            Interval::RDTempered(n_rd_steps, base, octave) => {
                // 1. Collect contextual notes from external context
                let mut others = context
                    .values()
                    .flat_map(
                        |NotesGroup {
                             notes, tolerance, ..
                         }| {
                            notes
                                .iter()
                                .filter(|n| {
                                    // self.time < n.time + n.duration + tolerance.0
                                    //     && n.time < self.time + self.duration + tolerance.1
                                    self.time - n.time < n.duration + tolerance.0
                                        && n.time - self.time < self.duration + tolerance.1
                                    // self.time < n.time + tolerance.0
                                    //     && n.time < self.time + tolerance.1
                                })
                                .map(|n| {
                                    (
                                        n,
                                        // true overlap only if timing overlaps
                                        // self.time < n.time + n.duration
                                        //     && n.time < self.time + self.duration,
                                        // (self.time - n.time).as_secs(),
                                        (self.time + self.duration - n.time)
                                            .max(n.time + n.duration - self.time)
                                            .as_secs(),
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

                let (prev_note, prev_prev_note) = if harmonise && melodise {
                    melodic_previous_notes(self_ctx, self.time, tolerance)
                } else {
                    (None, None)
                };

                // 3. Choose seed degree
                // let seed = others.choose(rng).unwrap_or(&(0, false)).0;
                let seed = others.choose(rng).unwrap_or(&(0, 0.0)).0;

                // 4. Compute new degrees
                let mut degree = if harmonise {
                    (-11..12i32)
                        .combinations(if self.random_chord {
                            rng.gen_range(1..=self.chord)
                        } else {
                            self.chord
                        })
                        .min_by_key(|d| {
                            let tension = tension_family(
                                others.iter().cloned(),
                                d.iter().cloned(),
                                harmoniser,
                            );
                            let affinity = if melodise {
                                melodic_affinity(
                                    d.iter().copied(),
                                    prev_note,
                                    prev_prev_note,
                                    melodiser,
                                    melody_order_affinity,
                                ) as f64
                            } else {
                                0.0
                            };

                            ((tension - affinity) * 1024.0) as i64
                        })
                        .unwrap()
                } else {
                    vec![(0..*n_rd_steps).fold(seed, |acc, _| acc + base.choose(rng).unwrap()) % 12]
                };
                if rng.gen_bool(self.reverse_prob) {
                    degree.reverse();
                }
                if rng.gen_bool(self.shuffle_prob) {
                    degree.shuffle(rng);
                }

                // 5. Build new notes and push into self_ctx
                let new_notes = degree
                    .iter()
                    .enumerate()
                    .map(|(n, d)| {
                        let delta_time = step_as_time * arpegio * n as f64;
                        let delta_beats = step_in_beats * arpegio * n as f64;
                        Self {
                            id: if n == 0 { self.id } else { note_id_gen.next() },
                            interval: Interval::Tempered(*d, *octave),
                            glide: self.glide.clone(),
                            time: self.time + delta_time,
                            beat_time: self.beat_time + delta_beats,
                            ..*self
                        }
                    })
                    .collect::<Vec<_>>();

                self_ctx.extend(new_notes.iter().cloned());

                new_notes
            }
            _ => vec![self.clone()],
        }
    }
}
fn tension_family(
    // ns: impl IntoIterator<Item = (i32, bool)>,
    ns: impl IntoIterator<Item = (i32, f64)>,
    c: impl IntoIterator<Item = i32>,
    harmoniser: [u32; 7],
) -> f64 {
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
                .map(|d| tension2(n, d, harmoniser) / (overlap.powi(2) + 1.0))
                .sum::<f64>()
        })
        .sum::<f64>();

    // --- 2. sum pairwise tensions within c itself ---
    for i in 0..combo_vals.len() {
        for j in (i + 1)..combo_vals.len() {
            total += tension2(combo_vals[i], combo_vals[j], harmoniser);
        }
    }

    total
}
fn tension2(n1: i32, n2: i32, harmoniser: [u32; 7]) -> f64 {
    let d = dist12(n1, n2);
    (match d {
        0 => harmoniser[0],
        1 => harmoniser[1],
        2 => harmoniser[2],
        3 => harmoniser[3],
        4 => harmoniser[4],
        5 => harmoniser[5],
        6 => harmoniser[6],
        _ => unreachable!(),
    }) as f64
}
fn dist12(n1: i32, n2: i32) -> i32 {
    let d = ((n1 - n2) % 12).abs();
    d.min(12 - d)
}

fn melodic_previous_notes(
    self_ctx: &[Note],
    current_time: Time,
    tolerance: (Time, Time),
) -> (Option<i32>, Option<i32>) {
    let mut recent = self_ctx
        .iter()
        .filter(|n| n.time <= current_time && current_time - n.time < n.duration + tolerance.0)
        .filter_map(|n| {
            if let Interval::Tempered(d, _) = n.interval {
                Some((n.time, d))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    recent.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

    let mut iter = recent.into_iter().rev();
    let prev = iter.next().map(|(_, d)| d);
    let prev_prev = iter.next().map(|(_, d)| d);
    (prev, prev_prev)
}

fn melodic_affinity(
    combo: impl IntoIterator<Item = i32>,
    prev_note: Option<i32>,
    prev_prev_note: Option<i32>,
    melodiser: [i32; 7],
    order_affinity: i32,
) -> i32 {
    let Some(prev) = prev_note else {
        return 0;
    };

    let previous_direction = prev_prev_note
        .map(|v| prev.cmp(&v))
        .filter(|d| *d != Ordering::Equal);

    let mut total = 0;
    for n in combo {
        let d = dist12(prev, n) as usize;
        total += melodiser[d];

        if let Some(direction) = previous_direction {
            let keeps_direction = match direction {
                Ordering::Greater => n > prev,
                Ordering::Less => n < prev,
                Ordering::Equal => false,
            };
            if keeps_direction {
                total += order_affinity;
            }
        }
    }

    total
}
