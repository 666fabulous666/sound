use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::{
    engine::score::{ChorusParams, LowpassLfo, LowpassRelaxation},
    sign_f,
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
    freq_glide: Option<Freq>,
    time: Time,
    duration: Time,
    attack_decay: (f64, f64),
    cutoff_multiplier: f64,
    lp_relaxation: LowpassRelaxation,
    lp_lfo: LowpassLfo,
    bend: (f64, f64),
    vibrato: (f64, Freq),
    chorus: &ChorusParams,
    pow_fact: (f64, Freq),
    lowpass_enabled: bool,
    lp_order: u32,
    memory: &mut [f64; 5],
    sample_rate: Freq,
    global_time: Time,
) -> f64 {
    let vol_envelope = envelope(attack_decay.0, attack_decay.1, duration)(time);
    let time = if let Some(fg) = freq_glide {
        glide_mid(freq, fg, duration, time)
    } else {
        time
    };
    let bend_vib_time = time_bend_vibrato(time, bend.0, bend.1, vibrato.0, vibrato.1);
    let p = pow_fact.0 * (pow_fact.1 * time).exp2();
    let disto = |x: f64| x.powf(p);
    let dynamic_multiplier = compute_cutoff_multiplier(
        cutoff_multiplier,
        &lp_relaxation,
        &lp_lfo,
        time,
        global_time,
        duration,
    );
    match wave_type {
        WaveType::HiHat => {
            let mut signal = vol_envelope * sign_f(drums::hi_hat(bend_vib_time), disto);
            if lowpass_enabled {
                for i in 0..lp_order.min(5) as usize {
                    signal = lowpass_step_cutoff(
                        signal,
                        &mut memory[i],
                        freq * dynamic_multiplier,
                        sample_rate,
                    );
                }
            }
            return signal;
        }
        WaveType::Kick => {
            let mut signal = vol_envelope * sign_f(drums::kick(bend_vib_time), disto);
            if lowpass_enabled {
                for i in 0..lp_order.min(5) as usize {
                    signal = lowpass_step_cutoff(
                        signal,
                        &mut memory[i],
                        freq * dynamic_multiplier,
                        sample_rate,
                    );
                }
            }
            return signal;
        }
        WaveType::Snare => {
            let mut signal = vol_envelope * sign_f(drums::snare(bend_vib_time), disto);
            if lowpass_enabled {
                for i in 0..lp_order.min(5) as usize {
                    signal = lowpass_step_cutoff(
                        signal,
                        &mut memory[i],
                        freq * dynamic_multiplier,
                        sample_rate,
                    );
                }
            }
            return signal;
        }
        WaveType::Ride => {
            let mut signal = vol_envelope * sign_f(drums::ride(bend_vib_time), disto);
            if lowpass_enabled {
                for i in 0..lp_order.min(5) as usize {
                    signal = lowpass_step_cutoff(
                        signal,
                        &mut memory[i],
                        freq * dynamic_multiplier,
                        sample_rate,
                    );
                }
            }
            return signal;
        }
        WaveType::Darbuka => {
            let mut signal = vol_envelope * sign_f(drums::darbuka(freq, bend_vib_time), disto);
            if lowpass_enabled {
                for i in 0..lp_order.min(5) as usize {
                    signal = lowpass_step_cutoff(
                        signal,
                        &mut memory[i],
                        freq * dynamic_multiplier,
                        sample_rate,
                    );
                }
            }
            return signal;
        }
        _ => {}
    }
    let f = |t: f64| match wave_type {
        WaveType::Mute => 0.0,
        WaveType::Sine => t.sin(),
        WaveType::Square => {
            if t % (2.0 * PI) < PI {
                1.0
            } else {
                -1.0
            }
        }
        WaveType::Triangle => {
            let t = t / (2.0 * PI);
            2.0 * (t - (t + 0.75).floor() + 0.25).abs() - 1.0
        }
        WaveType::Sawtooth => {
            let t = t / (2.0 * PI);
            t - (0.5 + t).floor()
        }
        _ => unreachable!(),
    };
    let phase = freq.phase(bend_vib_time);
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
            let tmp1 = sign_f(tmp1, disto);
            let tmp2 = sign_f(tmp2, disto);
            let factor = sym_pow_k.powi(2) + asym_pow_k.powi(2);
            norm += factor;
            let tmp = sym_pow_k * (tmp1 + tmp2) + asym_pow_k * (tmp1 - tmp2);
            tmp
        })
        .sum::<f64>()
        / norm.sqrt()
        / (freq / Freq(440.0)).sqrt();
    let mut tmp = vol_envelope * sum_of_waves;
    if lowpass_enabled {
        for i in 0..lp_order.min(5) as usize {
            tmp = lowpass_step_cutoff(tmp, &mut memory[i], freq * dynamic_multiplier, sample_rate);
        }
    }
    tmp
}

fn compute_cutoff_multiplier(
    base: f64,
    relaxation: &LowpassRelaxation,
    lfo: &LowpassLfo,
    note_time: Time,
    global_time: Time,
    duration: Time,
) -> f64 {
    let normalized = if duration.as_secs() <= 0.0 {
        0.0
    } else {
        (note_time / duration).clamp(0.0, 1.0)
    };
    let relax_factor = relaxation.factor(normalized);
    let lfo_time = if lfo.sync_with_clock {
        global_time
    } else {
        note_time
    };
    let mut multiplier = base * relax_factor + lfo.contribution(lfo_time);
    if !multiplier.is_finite() {
        multiplier = 0.0;
    }
    multiplier.max(0.0)
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

/// Polynomial time mapping for a frequency glide:
/// p(t) = t + ((f2 - f1) / (2 * d * f1)) * t²
///
/// f1: start frequency (Hz)
/// f2: end frequency (Hz)
/// d:  glide duration (seconds)
/// t:  current time (seconds)
pub fn help_glide(f1: Freq, f2: Freq, d: Time, t: Time) -> Time {
    t + t * ((f2 - f1) * t) / (d * f1 * 2.0)
}

/// Middle-only glide time-warp:
/// - [0, d/3]:      f_inst = f1  (p'(t) = 1)
/// - [d/3, 2d/3]:   linear glide f1 -> f2 (p'(t) ramps linearly)
/// - [2d/3, d]:     f_inst = f2  (p'(t) = f2/f1)
///
/// Returns p(t) so your sample is: (2.0*PI*f1*glide_mid(f1,f2,d,t)).sin()
pub fn glide_mid(f1: Freq, f2: Freq, d: Time, t: Time) -> Time {
    let third = d / 3.0;

    if t <= third {
        // Constant f1: p(t) = t
        t
    } else if t <= third * 2.0 {
        // Middle third: quadratic time-warp giving linear freq ramp
        // p(t) = t + k * (t - d/3)^2, with k = 3(f2 - f1) / (2 d f1)
        let k = (f2 - f1) * 3.0 / (d * 2.0 * f1);
        let tau = t - third;
        t + tau * (k * tau)
    } else {
        // Last third: keep instantaneous freq = f2  => p'(t) = f2/f1
        // p(t) = p(2d/3) + (t - 2d/3) * (f2/f1)
        let k = (f2 - f1) * 3.0 / (d * 2.0 * f1);
        let p_2thirds = third * 2.0 + third * (third * k); // p at 2d/3
        p_2thirds + (t - third * 2.0) * (f2 / f1)
    }
}

/// One step of a 1-pole low-pass filter with a frequency cutoff.
///
/// Arguments:
/// - `x`:        current raw sample
/// - `memory`:   previous output sample (will be updated in place)
/// - `cutoff`:   desired cutoff frequency in Hz
/// - `sample_rate`: audio sample rate in Hz
///
/// Returns the filtered sample y[n].
///
/// Behavior:
/// - higher `cutoff`  -> brighter, follows input more quickly
/// - lower `cutoff`   -> darker, slower/smoother
/// - very low cutoff  -> almost DC (holdy / laggy)
///
/// This is stable for 0 < cutoff < sample_rate/2.
pub fn lowpass_step_cutoff(x: f64, memory: &mut f64, cutoff: Freq, sample_rate: Freq) -> f64 {
    // Convert to Hz as f64
    let fc = cutoff.as_hz().max(0.0);
    let fs = sample_rate.as_hz().max(1.0); // avoid div-by-zero

    // Compute smoothing coeff alpha
    // alpha = 1 - exp(-2π fc / fs)
    let alpha = 1.0 - (-2.0 * std::f64::consts::PI * fc / fs).exp();

    // Do the standard 1-pole low-pass update:
    // y[n] = alpha*x[n] + (1-alpha)*y[n-1]
    *memory = alpha * x + (1.0 - alpha) * *memory;

    *memory
}
