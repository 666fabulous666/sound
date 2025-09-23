use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::engine::notes::ChorusParams;
pub mod drums;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum WaveType {
    Mute,
    Sine,
    Square,
    Triangle,
    Sawtooth,
    HiHat,
    Kick,
    Snare,
}

impl ToString for &WaveType {
    fn to_string(&self) -> String {
        match self {
            WaveType::Mute => "Mute".into(),
            WaveType::Sine => "Sine".into(),
            WaveType::Square => "Square".into(),
            WaveType::Triangle => "Triangle".into(),
            WaveType::Sawtooth => "Sawtooth".into(),
            WaveType::HiHat => "HiHat".into(),
            WaveType::Kick => "Kick".into(),
            WaveType::Snare => "Snare".into(),
        }
    }
}

pub fn generate_wave(
    wave_type: &WaveType,
    freq: f64,
    time: f64,
    duration: f64,
    attack_decay: (f64, f64),
    bend: (f64, f64),
    vibrato: (f64, f64),
    chorus: &ChorusParams,
    pow_fact: (f64, f64),
) -> f64 {
    let bend_vib_time = time_bend_vibrato(time, bend.0, bend.1, vibrato.0, vibrato.1);
    let f = |x: f64| match wave_type {
        WaveType::Mute => 0.0,
        WaveType::Sine => x.sin(),
        WaveType::Square => {
            if x % (2.0 * PI) < PI {
                0.25
            } else {
                -0.25
            }
        }
        WaveType::Triangle => {
            let t = x / (2.0 * PI);
            2.0 * (t - (t + 0.75).floor() + 0.25).abs() - 1.0
        }
        WaveType::Sawtooth => {
            let t = x / (2.0 * PI);
            0.5 * (t - (0.5 + t).floor())
        }
        WaveType::HiHat => drums::hi_hat(freq, bend_vib_time),
        WaveType::Kick => drums::kick(freq, bend_vib_time),
        WaveType::Snare => drums::snare(freq, bend_vib_time),
    };
    let phase = 2.0 * PI * freq * bend_vib_time;
    let p = pow_fact.0 * (pow_fact.1 * time).exp();
    // let mut norm = 1.0;
    // let tmp = once({
    //     let wave = f(phase);
    //     wave.signum() * wave.abs().min(1.0).powf(p)
    // })
    // .chain((1..chorus.voices).map(|k| {
    //     let delta = chorus.delta * (chorus.time_dependency * time).exp2();
    //     let two_pow_k = 2f64.powi(k as i32);
    //     let sym_pow_k = chorus.sym.powi(k as i32);
    //     let asym_pow_k = chorus.asym.powi(k as i32);
    //     let wave1 = f(phase * (1.0 + two_pow_k * delta));
    //     let wave2 = f(phase * (1.0 - two_pow_k * delta));
    //     let pow_wave1 = wave1.signum() * wave1.abs().min(1.0).powf(p);
    //     let pow_wave2 = wave2.signum() * wave2.abs().min(1.0).powf(p);
    //     let factor1 = sym_pow_k + asym_pow_k;
    //     let factor2 = sym_pow_k - asym_pow_k;
    //     norm += 0.5 * (factor1.abs() + factor2.abs());
    //     let tmp = factor1 * pow_wave1 + factor2 * pow_wave2;
    //     tmp
    // }))
    let mut norm = 0.0;
    let sum_of_waves = (0..chorus.voices)
        .map(|k| {
            let d = chorus.delta * (chorus.time_dependency * time).exp2();
            let delta1 = 1.0 + d * (1.0 + chorus.delta_shift);
            let delta2 = 1.0 + d * (1.0 - chorus.delta_shift);
            // let two_pow_k = 2f64.powi(k as i32);
            let sym_pow_k = chorus.sym.powi(k as i32);
            let asym_pow_k = chorus.asym.powi(k as i32);
            // let tmp1 = f(phase
            //     * (1.0 + chorus.delta * (chorus.time_dependency * time).exp2()).powi(k as i32));
            let tmp1 = f(phase * delta1.powi(k as i32));
            let tmp2 = f(phase / delta2.powi(k as i32));
            let tmp1 = tmp1.signum() * tmp1.abs().min(1.0).powf(p);
            let tmp2 = tmp2.signum() * tmp2.abs().min(1.0).powf(p);
            let factor = sym_pow_k.powi(2) + asym_pow_k.powi(2);
            norm += factor;
            let tmp = sym_pow_k * (tmp1 + tmp2) + asym_pow_k * (tmp1 - tmp2);
            tmp
        })
        .sum::<f64>()
        / norm.sqrt()
        / (freq / 440.0).sqrt();
    envelope(attack_decay.0, attack_decay.1, duration)(time) * sum_of_waves
}
pub fn envelope(attack: f64, decay: f64, note_duration: f64) -> impl Fn(f64) -> f64 {
    move |time: f64| {
        let time_fraction = time / note_duration;
        0.1 * (time_fraction.powf(1.0 / attack) * (1.0 - time_fraction).powf(1.0 / decay)) as f64
    }
}
fn time_bend_vibrato(
    time: f64,
    bend_mag: f64,
    bend_speed: f64,
    vibrato_mag: f64,
    vibrato_freq: f64,
) -> f64 {
    time + bend_mag * (1.0 + time).powf(-bend_speed)
        + vibrato_mag * (2.0 * PI * time * vibrato_freq).sin()
}

/// Deterministically "hash" an f64 into a pseudo-random f64 in [-1.0, 1.0).
/// - `-0.0` is treated as `0.0`
/// - All NaNs map to the same output for stability
/// - Infinities are handled like any other bit pattern
pub fn hash_f64_to_minus1_1(x: f64) -> f64 {
    // Canonicalize special cases so that -0.0 == 0.0, and all NaNs share one seed.
    let seed_bits: u64 = if x == 0.0 {
        0
    } else if x.is_nan() {
        // A canonical quiet-NaN payload
        0x7ff8_0000_0000_0000
    } else {
        x.to_bits()
    };

    // SplitMix64 mixer
    fn splitmix64(mut z: u64) -> u64 {
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    // Mix the seed
    let mixed = splitmix64(seed_bits);

    // Use the high 53 bits to construct a uniform in [0, 1)
    // (53 is the mantissa precision of f64)
    let u01 = ((mixed >> 11) as f64) * (1.0 / (1u64 << 53) as f64);

    // Map [0,1) -> [-1,1)
    2.0 * u01 - 1.0
}
