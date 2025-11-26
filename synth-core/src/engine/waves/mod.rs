use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::{
    engine::score::{ChorusParams, HarmonicsParams},
    engine::time_varying::TimeVarying,
    sign_f,
    time_freq::{Freq, Time},
};
pub mod drums;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterType {
    Lowpass,
    Highpass,
    Bandpass,
}

impl Default for FilterType {
    fn default() -> Self {
        FilterType::Lowpass
    }
}

impl ToString for FilterType {
    fn to_string(&self) -> String {
        match self {
            FilterType::Lowpass => "Lowpass".into(),
            FilterType::Highpass => "Highpass".into(),
            FilterType::Bandpass => "Bandpass".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
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

fn apply_filter(
    mut signal: f64,
    filter_enabled: bool,
    filter_type: FilterType,
    filter_order: u32,
    memory: &mut [f64; 10],
    cutoff_freq: Freq,
    sample_rate: Freq,
) -> f64 {
    if !filter_enabled {
        return signal;
    }

    let order = filter_order.min(10) as usize;

    match filter_type {
        FilterType::Lowpass => {
            for i in 0..order {
                signal = lowpass_step_cutoff(signal, &mut memory[i], cutoff_freq, sample_rate);
            }
            signal
        }
        FilterType::Highpass => {
            let input = signal;
            for i in 0..order {
                signal = lowpass_step_cutoff(signal, &mut memory[i], cutoff_freq, sample_rate);
            }
            input - signal
        }
        FilterType::Bandpass => {
            // Bandpass = Highpass followed by Lowpass
            // Use first half of memories for highpass, second half for lowpass
            let hp_order = (order + 1) / 2;
            let lp_order = order / 2;

            // Highpass stage
            let input = signal;
            for i in 0..hp_order {
                signal = lowpass_step_cutoff(signal, &mut memory[i], cutoff_freq, sample_rate);
            }
            signal = input - signal;

            // Lowpass stage
            for i in 0..lp_order {
                signal = lowpass_step_cutoff(
                    signal,
                    &mut memory[hp_order + i],
                    cutoff_freq,
                    sample_rate,
                );
            }
            signal
        }
    }
}

fn base_wave(wave_type: &WaveType, phase: f64) -> f64 {
    match wave_type {
        WaveType::Mute => 0.0,
        WaveType::Sine => phase.sin(),
        WaveType::Square => {
            if phase % (2.0 * PI) < PI {
                1.0
            } else {
                -1.0
            }
        }
        WaveType::Triangle => {
            let t = phase / (2.0 * PI);
            2.0 * (t - (t + 0.75).floor() + 0.25).abs() - 1.0
        }
        WaveType::Sawtooth => {
            let t = phase / (2.0 * PI);
            t - (0.5 + t).floor()
        }
        _ => unreachable!(),
    }
}

fn harmonic_bundle(
    wave_type: &WaveType,
    base_phase: f64,
    harmonics: &HarmonicsParams,
    disto: impl Fn(f64) -> f64 + Copy,
) -> (f64, f64) {
    let mut sum = sign_f(base_wave(wave_type, base_phase), disto);
    let mut weight_norm = 1.0;
    let attenuation = harmonics.attenuation.max(0.0);

    for i in 0..harmonics.harmonics {
        let multiplier = (i + 2) as f64;
        let w = attenuation.powi((i + 1) as i32);
        sum += w * sign_f(base_wave(wave_type, base_phase * multiplier), disto);
        weight_norm += w.abs();
    }
    for i in 0..harmonics.subharmonics {
        let divisor = (i + 2) as f64;
        let w = attenuation.powi((i + 1) as i32);
        sum += w * sign_f(base_wave(wave_type, base_phase / divisor), disto);
        weight_norm += w.abs();
    }

    (sum, weight_norm)
}

fn tonal_wave_sample(
    wave_type: &WaveType,
    phase: f64,
    time: Time,
    chorus: &ChorusParams,
    harmonics: &HarmonicsParams,
    disto: impl Fn(f64) -> f64 + Copy,
    freq: Freq,
) -> f64 {
    let mut norm = 0.0;
    let mut sum = 0.0;
    for k in 0..chorus.voices {
        let d = chorus.delta * (chorus.time_dependency * time).exp2();
        let delta1 = 1.0 + d * (1.0 + chorus.delta_shift);
        let delta2 = 1.0 + d * (1.0 - chorus.delta_shift);
        let sym_pow_k = chorus.sym.powi(k as i32);
        let asym_pow_k = chorus.asym.powi(k as i32);
        let (tmp1, w1) =
            harmonic_bundle(wave_type, phase * delta1.powi(k as i32), harmonics, disto);
        let (tmp2, w2) =
            harmonic_bundle(wave_type, phase / delta2.powi(k as i32), harmonics, disto);
        let factor = sym_pow_k.powi(2) + asym_pow_k.powi(2);
        norm += factor * (w1 + w2);
        sum += sym_pow_k * (tmp1 + tmp2) + asym_pow_k * (tmp1 - tmp2);
    }

    let norm = norm.max(f64::MIN_POSITIVE);
    sum / norm.sqrt() / (freq / Freq(440.0)).sqrt()
}

pub fn generate_wave(
    wave_type: &WaveType,
    freq: Freq,
    freq_glide: Option<Freq>,
    time: Time,
    duration: Time,
    attack_decay: (f64, f64),
    cutoff: &TimeVarying,
    bend: (f64, f64),
    vibrato: (f64, Freq),
    chorus: &ChorusParams,
    harmonics: &HarmonicsParams,
    power: &TimeVarying,
    noise: &TimeVarying,
    filter_enabled: bool,
    filter_type: FilterType,
    filter_order: u32,
    memory: &mut [f64; 10],
    sample_rate: Freq,
    global_time: Time,
) -> f64 {
    let vol_envelope = envelope(attack_decay.0, attack_decay.1, duration)(time);
    let glided_time = if let Some(fg) = freq_glide {
        glide_mid(freq, fg, duration, time)
    } else {
        time
    };
    let bend_vib_time = time_bend_vibrato(glided_time, bend.0, bend.1, vibrato.0, vibrato.1);
    let p = power.evaluate(glided_time, global_time, duration);
    let disto = |x: f64| x.powf(p);
    let noise_alpha = noise.evaluate(glided_time, global_time, duration);
    let noise_multiplier = 1.0 - noise_alpha * rand::random::<f64>();
    let dynamic_multiplier = cutoff.evaluate(glided_time, global_time, duration);
    let cutoff_freq = freq * dynamic_multiplier;
    match wave_type {
        WaveType::HiHat => {
            let signal =
                vol_envelope * sign_f(drums::hi_hat(bend_vib_time), disto) * noise_multiplier;
            return apply_filter(
                signal,
                filter_enabled,
                filter_type,
                filter_order,
                memory,
                cutoff_freq,
                sample_rate,
            );
        }
        WaveType::Kick => {
            let signal =
                vol_envelope * sign_f(drums::kick(bend_vib_time), disto) * noise_multiplier;
            return apply_filter(
                signal,
                filter_enabled,
                filter_type,
                filter_order,
                memory,
                cutoff_freq,
                sample_rate,
            );
        }
        WaveType::Snare => {
            let signal =
                vol_envelope * sign_f(drums::snare(bend_vib_time), disto) * noise_multiplier;
            return apply_filter(
                signal,
                filter_enabled,
                filter_type,
                filter_order,
                memory,
                cutoff_freq,
                sample_rate,
            );
        }
        WaveType::Ride => {
            let signal =
                vol_envelope * sign_f(drums::ride(bend_vib_time), disto) * noise_multiplier;
            return apply_filter(
                signal,
                filter_enabled,
                filter_type,
                filter_order,
                memory,
                cutoff_freq,
                sample_rate,
            );
        }
        WaveType::Darbuka => {
            let signal = vol_envelope
                * sign_f(drums::darbuka(freq, bend_vib_time), disto)
                * noise_multiplier;
            return apply_filter(
                signal,
                filter_enabled,
                filter_type,
                filter_order,
                memory,
                cutoff_freq,
                sample_rate,
            );
        }
        _ => {}
    }
    let phase = freq.phase(bend_vib_time);
    let sum_of_waves = tonal_wave_sample(
        wave_type,
        phase,
        glided_time,
        chorus,
        harmonics,
        disto,
        freq,
    );
    apply_filter(
        vol_envelope * sum_of_waves * noise_multiplier,
        filter_enabled,
        filter_type,
        filter_order,
        memory,
        cutoff_freq,
        sample_rate,
    )
}

pub fn generate_wave_with_phase(
    wave_type: &WaveType,
    freq: Freq,
    freq_glide: Option<Freq>,
    time: Time,
    duration: Time,
    attack_decay: (f64, f64),
    cutoff: &TimeVarying,
    bend: (f64, f64),
    vibrato: (f64, Freq),
    chorus: &ChorusParams,
    harmonics: &HarmonicsParams,
    power: &TimeVarying,
    noise: &TimeVarying,
    filter_enabled: bool,
    filter_type: FilterType,
    filter_order: u32,
    memory: &mut [f64; 10],
    phase: &mut f64,
    sample_rate: Freq,
    global_time: Time,
    sample_step: Time,
) -> f64 {
    let vol_envelope = envelope(attack_decay.0, attack_decay.1, duration)(time);
    let glided_time = if let Some(fg) = freq_glide {
        glide_mid(freq, fg, duration, time)
    } else {
        time
    };
    let next_glided_time = if let Some(fg) = freq_glide {
        glide_mid(freq, fg, duration, time + sample_step)
    } else {
        time + sample_step
    };
    let bend_vib_time = time_bend_vibrato(glided_time, bend.0, bend.1, vibrato.0, vibrato.1);
    let next_bend_vib_time =
        time_bend_vibrato(next_glided_time, bend.0, bend.1, vibrato.0, vibrato.1);
    let p = power.evaluate(glided_time, global_time, duration);
    let disto = |x: f64| x.powf(p);
    let noise_alpha = noise.evaluate(glided_time, global_time, duration);
    let noise_multiplier = 1.0 - noise_alpha * rand::random::<f64>();
    let dynamic_multiplier = cutoff.evaluate(glided_time, global_time, duration);
    let cutoff_freq = freq * dynamic_multiplier;

    match wave_type {
        WaveType::HiHat => {
            let signal =
                vol_envelope * sign_f(drums::hi_hat(bend_vib_time), disto) * noise_multiplier;
            return apply_filter(
                signal,
                filter_enabled,
                filter_type,
                filter_order,
                memory,
                cutoff_freq,
                sample_rate,
            );
        }
        WaveType::Kick => {
            let signal =
                vol_envelope * sign_f(drums::kick(bend_vib_time), disto) * noise_multiplier;
            return apply_filter(
                signal,
                filter_enabled,
                filter_type,
                filter_order,
                memory,
                cutoff_freq,
                sample_rate,
            );
        }
        WaveType::Snare => {
            let signal =
                vol_envelope * sign_f(drums::snare(bend_vib_time), disto) * noise_multiplier;
            return apply_filter(
                signal,
                filter_enabled,
                filter_type,
                filter_order,
                memory,
                cutoff_freq,
                sample_rate,
            );
        }
        WaveType::Ride => {
            let signal =
                vol_envelope * sign_f(drums::ride(bend_vib_time), disto) * noise_multiplier;
            return apply_filter(
                signal,
                filter_enabled,
                filter_type,
                filter_order,
                memory,
                cutoff_freq,
                sample_rate,
            );
        }
        WaveType::Darbuka => {
            let signal = vol_envelope
                * sign_f(drums::darbuka(freq, bend_vib_time), disto)
                * noise_multiplier;
            return apply_filter(
                signal,
                filter_enabled,
                filter_type,
                filter_order,
                memory,
                cutoff_freq,
                sample_rate,
            );
        }
        _ => {}
    }

    let phase_increment = freq.phase(next_bend_vib_time - bend_vib_time);
    let current_phase = *phase;
    *phase += phase_increment;

    let mut norm = 0.0;
    let mut sum = 0.0;
    for k in 0..chorus.voices {
        let d = chorus.delta * (chorus.time_dependency * glided_time).exp2();
        let delta1 = 1.0 + d * (1.0 + chorus.delta_shift);
        let delta2 = 1.0 + d * (1.0 - chorus.delta_shift);
        let sym_pow_k = chorus.sym.powi(k as i32);
        let asym_pow_k = chorus.asym.powi(k as i32);

        let (tmp1, w1) = harmonic_bundle(
            wave_type,
            current_phase * delta1.powi(k as i32),
            harmonics,
            disto,
        );
        let (tmp2, w2) = harmonic_bundle(
            wave_type,
            current_phase / delta2.powi(k as i32),
            harmonics,
            disto,
        );
        let factor = sym_pow_k.powi(2) + asym_pow_k.powi(2);
        norm += factor * (w1 + w2);
        sum += sym_pow_k * (tmp1 + tmp2) + asym_pow_k * (tmp1 - tmp2);
    }

    if norm == 0.0 {
        norm = 1.0;
    }

    let signal =
        vol_envelope * (sum / norm.sqrt()) / (freq / Freq(440.0)).sqrt() * noise_multiplier;
    apply_filter(
        signal,
        filter_enabled,
        filter_type,
        filter_order,
        memory,
        cutoff_freq,
        sample_rate,
    )
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
