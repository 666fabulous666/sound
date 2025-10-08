pub mod default_params;
pub mod note;
pub mod sequence;
pub mod track_node;

use std::sync::Arc;

use crate::{
    engine::{
        score::{note::Note, track_node::TrackNode},
        waves::WaveType,
    },
    time_freq::{Freq, Time},
    Token, TokenGen,
};
use arc_swap::ArcSwap;
use default_params::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct RdRythm {
    pub amount: usize,
    pub length: usize,
}
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct DetRythm {
    pub generators: Vec<usize>,
}
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Rythm {
    Rd(RdRythm),
    Det(DetRythm),
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ChorusParams {
    pub voices: usize,
    pub delta: f64,
    #[serde(default = "default_delta_shift")]
    pub delta_shift: f64,
    pub sym: f64,
    pub asym: f64,
    pub time_dependency: Freq,
}
impl ChorusParams {
    pub fn new(
        number_of_heads: usize,
        delta: f64,
        delta_noise: f64,
        sym: f64,
        asym: f64,
        time_dependency: Freq,
    ) -> Self {
        Self {
            voices: number_of_heads,
            delta,
            delta_shift: delta_noise,
            sym,
            asym,
            time_dependency,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Interval {
    /// (degree, octave)
    Tempered(i32, i32),
    /// (n rd steps, base, octave)
    RDTempered(u32, Vec<i32>, i32),
}

impl Interval {
    pub fn compute(&self) -> f64 {
        match self {
            Interval::Tempered(degree, octave) => (*degree as f64 / 12.0 + *octave as f64).exp2(),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub struct NotesGroup {
    pub attack_decay: (f64, f64),
    pub bend: (f64, f64),
    pub chorus: ChorusParams,
    pub notes: Vec<Note>,
    pub pow_fact: (f64, Freq),
    pub spacial: f64,
    pub token: Token,
    pub tolerance: (Time, Time),
    pub vibrato: (f64, Freq),
    pub volume: f64,
    pub wave_type: WaveType,
}

pub struct Score {
    pub notes: Vec<NotesGroup>,
    pub sequences: TrackNode,
    pub last_token: TokenGen,
    pub delays: (Vec<f64>, Vec<f64>),
    pub shared_notes: Arc<ArcSwap<Vec<NotesGroup>>>,
}
impl Score {
    pub fn new() -> Self {
        let mut last_token = TokenGen(0);
        Self {
            notes: Vec::new(),
            sequences: TrackNode::new_root(&mut last_token),
            last_token,
            delays: default_delays(),
            shared_notes: Arc::new(ArcSwap::from_pointee(Vec::new())),
        }
    }
}
