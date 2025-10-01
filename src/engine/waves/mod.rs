use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::{
    engine::notes::ChorusParams,
    time_freq::{Freq, Time},
};
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
    Ride,
    Darbuka,
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
            WaveType::Ride => "Ride".into(),
            WaveType::Darbuka => "Darbuka".into(),
        }
    }
}

pub fn generate_wave(
    wave_type: &WaveType,
    freq: Freq,
    time: Time,
    duration: Time,
    attack_decay: (f64, f64),
    bend: (f64, f64),
    vibrato: (f64, Freq),
    chorus: &ChorusParams,
    pow_fact: (f64, Freq),
) -> f64 {
    let envelope = envelope(attack_decay.0, attack_decay.1, duration)(time);
    let bend_vib_time = time_bend_vibrato(time, bend.0, bend.1, vibrato.0, vibrato.1);
    match wave_type {
        WaveType::HiHat => return envelope * drums::hi_hat(freq, time), // WARNING: put back bend_vib_time instead of freq if it changed something
        WaveType::Kick => return envelope * drums::kick(time),
        WaveType::Snare => return envelope * drums::snare(time),
        WaveType::Ride => return envelope * drums::ride(freq, time),
        WaveType::Darbuka => return envelope * drums::darbuka(time),
        _ => {}
    }
    let f = |t: f64| match wave_type {
        WaveType::Mute => 0.0,
        WaveType::Sine => t.sin(),
        WaveType::Square => {
            if t % (2.0 * PI) < PI {
                0.25
            } else {
                -0.25
            }
        }
        WaveType::Triangle => {
            let t = t / (2.0 * PI);
            2.0 * (t - (t + 0.75).floor() + 0.25).abs() - 1.0
        }
        WaveType::Sawtooth => {
            let t = t / (2.0 * PI);
            0.5 * (t - (0.5 + t).floor())
        }
        _ => unreachable!(),
    };
    let phase = freq.phase(bend_vib_time);
    let p = pow_fact.0 * (pow_fact.1 * time).exp2();
    let mut norm = 0.0;
    let sum_of_waves = (0..chorus.voices)
        .map(|k| {
            let d = chorus.delta * (chorus.time_dependency * time).exp2();
            let delta1 = 1.0 + d * (1.0 + chorus.delta_shift);
            let delta2 = 1.0 + d * (1.0 - chorus.delta_shift);
            let sym_pow_k = chorus.sym.powi(k as i32);
            let asym_pow_k = chorus.asym.powi(k as i32);
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
        / (freq / Freq(440.0)).sqrt();
    envelope * sum_of_waves
}
pub fn envelope(attack: f64, decay: f64, note_duration: Time) -> impl Fn(Time) -> f64 {
    move |time: Time| {
        let time_fraction = time / note_duration;
        (time_fraction.powf(1.0 / attack) * (1.0 - time_fraction).powf(1.0 / decay)) as f64
    }
}
fn time_bend_vibrato(
    time: Time,
    bend_mag: f64,
    bend_speed: f64,
    vibrato_mag: f64,
    vibrato_freq: Freq,
) -> Time {
    time + Time(bend_mag * (Time(1.0) + time).as_secs().powf(-bend_speed))
        + Time(vibrato_mag * (vibrato_freq.phase(time)).sin())
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
