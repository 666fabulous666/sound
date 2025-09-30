use crate::engine::notes::{ChorusParams, DetRythm, RdRythm};
use crate::time_freq::{Freq, Time};
use crate::{rescale_factor, DEFAULT_LOOP_LEN};

pub fn default_repeat() -> usize {
    1
}

pub fn default_pow_fact() -> (f64, Freq) {
    (1.0, Freq(0.0))
}

pub fn default_attack_decay() -> (f64, f64) {
    (4.0, 0.3333)
}

pub fn default_drum_attack_decay() -> (f64, f64) {
    (5.0, 0.5)
}

pub fn default_bend() -> (f64, f64) {
    (0.0, 32.0)
}

pub fn default_vibrato() -> (f64, Freq) {
    (0.0, Freq(12.0))
}

pub fn default_accents() -> (f64, Vec<f64>) {
    (1.0, vec![0.5, 1.2, 2.5, 3.0])
}

pub fn default_shuffle() -> bool {
    true
}

pub fn default_spacial() -> f64 {
    0.5
}

pub fn default_tolerance() -> (Time, Time) {
    (Time(1.0), Time(0.0))
}

pub fn default_beat_offset() -> usize {
    0
}

pub fn default_volume() -> f64 {
    5.0
}
pub fn default_mute() -> bool {
    false
}
pub fn default_normalization() -> f64 {
    let (attack, decay) = default_attack_decay();
    rescale_factor(1.0 / attack, 1.0 / decay)
}
pub fn default_drum_normalization() -> f64 {
    let (attack, decay) = default_attack_decay();
    rescale_factor(1.0 / attack, 1.0 / decay)
}
pub fn default_delta_shift() -> f64 {
    0.0
}
pub fn default_loop_len() -> Time {
    DEFAULT_LOOP_LEN
}
pub fn default_time_quantum() -> (usize, usize) {
    (1, 8)
}
impl Default for ChorusParams {
    fn default() -> Self {
        Self {
            voices: 1,
            delta: 0.02,
            delta_shift: default_delta_shift(),
            sym: 0.5,
            asym: 0.0,
            time_dependency: Freq(0.0),
        }
    }
}
impl Default for DetRythm {
    fn default() -> Self {
        Self { generators: vec![] }
    }
}
impl Default for RdRythm {
    fn default() -> Self {
        Self {
            amount: 5,
            length: 10,
        }
    }
}
