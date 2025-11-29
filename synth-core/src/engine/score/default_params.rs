use crate::engine::score::{
    probability::Probability, time_quantum::TimeQuantum, ChorusParams, DetRythm, LowpassLfo,
    LowpassRelaxation, RdRythm, TrackDelays,
};
use crate::time_freq::{Beat, Freq, Tempo, Time};
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
pub fn default_cutoff_multiplier() -> f64 {
    4.0
}
pub fn default_lp_relaxation() -> LowpassRelaxation {
    LowpassRelaxation::default()
}
pub fn default_lp_lfo() -> LowpassLfo {
    LowpassLfo::default()
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
    false
}

pub fn default_name() -> String {
    String::new()
}

pub fn default_spacial() -> f64 {
    0.5
}

pub fn default_pan() -> f64 {
    0.5
}

pub fn default_arpegio() -> f64 {
    0.0
}

pub fn default_tolerance() -> (Time, Time) {
    (Time(1.0), Time(0.0))
}

pub fn default_harmonise() -> bool {
    false
}

pub fn default_melodise() -> bool {
    false
}

pub const MAX_MELODIC_INTERVAL: usize = 24;

pub fn default_melody_order_affinity() -> i32 {
    0
}

pub fn default_replicator_enabled() -> bool {
    false
}

pub fn default_replicator_distance() -> u32 {
    1
}

pub fn default_beat_offset() -> i32 {
    0
}

pub fn default_tension() -> usize {
    1
}
pub fn default_random_chord() -> bool {
    false
}

pub fn default_volume() -> f64 {
    1.0
}
pub fn default_glide() -> bool {
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
pub fn default_loop_len() -> Beat {
    Beat(DEFAULT_LOOP_LEN.as_secs())
}
pub fn default_loop_offset() -> Beat {
    Beat(0.0)
}
pub fn default_tail_multiplier() -> f64 {
    1.0
}
pub fn default_time_quantum() -> TimeQuantum {
    TimeQuantum::default()
}
pub fn default_proba() -> Probability {
    Probability::default()
}
pub fn default_reverse_prob() -> f64 {
    0.0
}
pub fn default_shuffle_prob() -> f64 {
    0.0
}
pub fn default_lowpass_enabled() -> bool {
    false
}
pub fn default_lp_order() -> u32 {
    1
}
pub fn default_harmonics_attenuation() -> f64 {
    0.5
}
pub fn default_tempo() -> Tempo {
    Tempo::new(60.0)
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
pub fn default_delays() -> TrackDelays {
    // Defaults converted from the previous millisecond values at 60 BPM (1s per beat).
    // Using beats keeps the perceived delay length stable across tempo changes.
    TrackDelays {
        left: crate::engine::score::DelayChannel {
            dry: Some(0.5),
            taps: vec![
                crate::engine::score::DelayTap {
                    beat: 0.031,
                    weight: 1.0,
                },
                crate::engine::score::DelayTap {
                    beat: 0.063,
                    weight: 1.0,
                },
                crate::engine::score::DelayTap {
                    beat: 0.128,
                    weight: 1.0,
                },
            ],
        },
        right: crate::engine::score::DelayChannel {
            dry: Some(0.5),
            taps: vec![
                crate::engine::score::DelayTap {
                    beat: 0.033,
                    weight: 1.0,
                },
                crate::engine::score::DelayTap {
                    beat: 0.061,
                    weight: 1.0,
                },
                crate::engine::score::DelayTap {
                    beat: 0.124,
                    weight: 1.0,
                },
            ],
        },
    }
}
pub fn default_harmoniser() -> [u32; 7] {
    [10, 11, 11, 4, 3, 0, 8]
}
pub fn default_skip_harmonised() -> usize {
    0
}
