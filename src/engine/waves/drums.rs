use std::f64::consts::PI;

use crate::{
    sign_f,
    time_freq::{DivByFreq, Freq, Time},
};

/// A simple xorshift64* pseudo‐random number generator
fn xorshift64(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// Generate a deterministic “pseudo‐random” f64 in [0.0, 1.0)
/// from a 64‐bit seed.
fn prng_unit(seed: u64) -> f64 {
    let r = xorshift64(seed);
    // Divide by 2^64 − 1 to get [0,1)
    (r as f64) / (u64::MAX as f64)
}

#[inline]
fn exp_env(t: Time, tau: Time) -> f64 {
    (-t.max(Time::new(0.0)) / tau).exp()
}

#[inline]
fn power_attack_env(t: Time, alpha: f64, tau: Time) -> f64 {
    let t = t.max(Time(0.0));
    t.as_secs().powf(alpha) * (-t / tau).exp()
}

/// Deterministic “noise” used in your code. Some callsites wrap fract into [0,1), some don’t.
/// Set `wrapped=true` to reproduce `(raw.fract() + 1.0).fract()`, otherwise `raw.fract()`.
#[inline]
fn det_noise(t: Time, wrapped: bool) -> f64 {
    let raw = (t * Freq(1e7)).sin() * 1e6;
    let frac = if wrapped {
        (raw.fract() + 1.0).fract()
    } else {
        raw.fract()
    };
    2.0 * frac - 1.0
}

/// Small helper used by darbuka for the subtle initial pitch rise
#[inline]
fn glide(freq: Freq, t: Time, amt: f64, tau: Time) -> f64 {
    // f(t) = f0 * (1 + amt * e^{-t/tau})
    freq.as_hz() * (1.0 + amt * (-t / tau).exp())
}

/// Which drum to synthesize
#[derive(Clone, Copy, Debug)]
pub enum DrumParams {
    HiHat,
    Kick,
    Bell,
    Tom,
    Snare,
    Ride,
    Darbuka,
}

#[derive(Copy, Clone)]
struct Layer {
    env_tau: Time,
    gain: f64,
    signal: f64,
}

#[inline]
fn mix_env_layers(time: Time, layers: &[Layer]) -> f64 {
    layers
        .iter()
        .map(|l| l.gain * exp_env(time, l.env_tau) * l.signal)
        .sum()
}

/// A single entry point that reproduces your per-drum functions exactly
pub fn drum(frequency: Freq, time: Time, params: DrumParams) -> f64 {
    match params {
        DrumParams::HiHat => {
            // === your hi_hat unchanged numerics ===
            let tau = Time(0.020);
            let alpha = 0.2;
            let noise_level = 0.25;

            let env = power_attack_env(time, alpha, tau);

            let mut osc = 0.0;
            let k_partials = 8;
            let freq_bits = frequency.as_hz().to_bits();
            let time_bits = time.as_secs().to_bits();

            for i in 0..k_partials {
                let seed_base =
                    freq_bits.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ time_bits ^ (i as u64);

                // δ_k ∈ [0.1, 1.5]
                let u1 = prng_unit(seed_base);
                let delta = 0.1 + u1 * (1.5 - 0.1);
                let fk = frequency * (1.0 + delta);

                // roll-off
                let ak = 1.0 / (1.0 + 0.5 * ((i + 1) as f64));

                // phase φ_k ∈ [0, 2π)
                let u2 = prng_unit(seed_base ^ 0xDEAD_BEEF_DEAD_BEEF);
                let phase = u2 * 2.0 * PI;

                osc += ak * (fk.phase(time) + phase).sin();
            }

            // NOTE: hi_hat used the *unwrapped* fract
            let noise = det_noise(time, /*wrapped=*/ false);

            env * osc + noise_level * env * noise
        }
        DrumParams::Kick => mix_env_layers(
            time,
            &[
                Layer {
                    env_tau: Time(0.1),
                    gain: 5.0,
                    signal: {
                        let sweep_rate = Freq(10.0);
                        frequency
                            .phase((1.0 - (-sweep_rate * time).exp()).div_by(sweep_rate))
                            .sin()
                    },
                },
                Layer {
                    env_tau: Time(0.1),
                    gain: 5.0,
                    signal: {
                        let mut t = time.as_secs() + 1.0;
                        let mut tt = t;
                        for i in (1..=2).rev() {
                            t = t.sqrt() / i as f64;
                            tt += t;
                        }
                        (frequency * 0.25).phase(Time::new(tt)).sin()
                    },
                },
                Layer {
                    env_tau: Time(0.0025),
                    gain: 0.75,
                    signal: det_noise(time, true),
                },
            ],
        ),
        DrumParams::Tom => {
            5.0 * mix_env_layers(
                time,
                &[
                    Layer {
                        env_tau: Time(0.1),
                        gain: 1.0,
                        signal: {
                            let mut t = time.as_secs();
                            let mut tt = t;
                            for i in (2..=5).rev() {
                                t = t.sqrt() / i as f64;
                                tt += t;
                            }
                            frequency.phase(Time::new(tt)).sin()
                        },
                    },
                    Layer {
                        env_tau: Time(0.0025),
                        gain: 0.15,
                        signal: det_noise(time, true),
                    },
                ],
            )
        }
        DrumParams::Bell => {
            5.0 * mix_env_layers(
                time,
                &[
                    Layer {
                        env_tau: Time(0.1),
                        gain: 1.0,
                        signal: {
                            let mut t = time.as_secs();
                            let mut tt = t;
                            for i in 1..=5 {
                                t = t.sqrt() / i as f64;
                                tt += tt; // WARNING: this += tt (not += t) is a mistake but I don't correct before knowing if it would sound the same
                            }
                            frequency.phase(Time::new(tt)).sin()
                        },
                    },
                    Layer {
                        env_tau: Time(0.0025),
                        gain: 0.15,
                        signal: det_noise(time, true),
                    },
                ],
            )
        }

        DrumParams::Snare => {
            5.0 * mix_env_layers(
                time,
                &[
                    Layer {
                        env_tau: Time(0.2),
                        gain: 0.5,
                        signal: frequency.phase(time).sin(),
                    },
                    Layer {
                        env_tau: Time(0.1),
                        gain: 0.15,
                        signal: det_noise(time, true),
                    },
                ],
            )
        }
        DrumParams::Ride => {
            // === your ride() numerics ===
            // envelopes
            let wash_tau = Time(1.20);
            let wash_alpha = 0.05;
            let wash_env = power_attack_env(time, wash_alpha, wash_tau);

            let ping_tau = Time(0.40);
            let ping_env = exp_env(time, ping_tau);

            let click_tau = Time(0.003);
            let click_env = exp_env(time, click_tau);
            let click_level = 0.10;

            // inharmonic wash
            let base = frequency;
            let k_partials = 64;
            let mut wash = 0.0;
            for i in 0..k_partials {
                let seed = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xC3A5_C85C_97CB_3127;

                let u1 = prng_unit(seed);
                let band = 2.5 + u1 * (14.0 - 2.5);
                let fk = base * band;

                let phase = prng_unit(seed ^ 0xDEAD_BEEF_DEAD_BEEF) * 2.0 * PI;
                let ak = 1.0 / (1.0 + 0.18 * ((i + 1) as f64));

                // wash += ak * powf_sin(fk.phase(time) + phase, 1.0 / (i + 10) as f64);
                wash += ak
                    * sign_f((fk.phase(time) + phase).sin(), |x| {
                        x.powf(1.0 / (i + 10) as f64)
                    });
            }

            // ping
            let n_fs = 4;
            let fs = (0..n_fs).map(|i| Freq(2600.0 + 1400.0 * (i as f64 / n_fs as f64)));
            let cs: Vec<f64> = (1..=fs.len()).map(|c| c as f64).collect();
            let sum_cs = cs.iter().sum::<f64>();
            let ping: f64 = cs
                .into_iter()
                .map(|c| c / sum_cs)
                .zip(fs)
                .map(|(c, f)| c * (f.phase(time)).sin())
                .sum();

            // ride used *wrapped* fract for the click
            let noise = det_noise(time, true);

            let out =
                0.65 * wash_env * wash + 0.45 * ping_env * ping + click_level * click_env * noise;

            0.9 * out
        }

        DrumParams::Darbuka => {
            // === your darbuka() numerics ===
            let doum_tau = Time(0.15);
            let doum_env = exp_env(time, doum_tau);

            let tek_tau = Time(0.02);
            let tek_env = exp_env(time, tek_tau);

            let click_tau = Time(0.0025);
            let click_env = exp_env(time, click_tau);

            // body (membrane-like modes), uses incoming `frequency` like original
            let f0 = frequency;
            let body = {
                let ratios = [(1.00, 1.00), (1.59, 0.85), (2.14, 0.75), (2.30, 0.65)];
                let gains = [1.00, 0.70, 0.50, 0.40];
                let mut s = 0.0;
                for ((r, dscale), g) in ratios.into_iter().zip(gains) {
                    let f = Freq(f0.as_hz() * r);
                    let env = exp_env(time, Time(doum_tau.as_secs() * dscale));
                    s += g * (f.phase(time)).sin() * env;
                }
                s
            };

            // rim / tek
            let tek = {
                let partials = [
                    (2300.0, 0.55, 0.03, 0.03),
                    (3300.0, 0.35, 0.03, 0.03),
                    (4100.0, 0.25, 0.02, 0.02),
                ];
                let mut s = 0.0;
                for &(f, gain, amt, tau) in &partials {
                    let freq = Freq(glide(Freq(f), time, amt, Time(tau)));
                    s += gain * freq.phase(time).sin();
                }
                let tmp = sign_f(s, |x| x * x);
                tmp * tek_env
            };

            // darbuka also used *wrapped* fract
            let noise = det_noise(time, true);

            let out = 0.75 * doum_env * body + 0.60 * tek + 0.10 * click_env * noise;

            (out * 1.2).tanh()
        }
    }
}

pub fn hi_hat(time: Time) -> f64 {
    drum(Freq(110.0), time, DrumParams::HiHat)
}

pub fn kick(time: Time) -> f64 {
    drum(Freq(110.0), time, DrumParams::Kick)
}
pub fn tom(frequency: Freq, time: Time) -> f64 {
    drum(frequency, time, DrumParams::Kick)
}

pub fn bell(time: Time) -> f64 {
    drum(Freq(110.0), time, DrumParams::Bell)
}

pub fn snare(time: Time) -> f64 {
    drum(Freq(110.0), time, DrumParams::Snare)
}

pub fn ride(time: Time) -> f64 {
    drum(Freq(880.0), time, DrumParams::Ride)
}

pub fn darbuka(frequency: Freq, time: Time) -> f64 {
    drum(frequency, time, DrumParams::Darbuka)
}
