use std::f64::consts::PI;

use crate::time_freq::{DivByFreq, Freq, Time};

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

/// Hi‐hat synthesis purely in direct time‐domain,
/// using a deterministic PRNG based on frequency, time, and partial index.
pub fn hi_hat(frequency: Freq, time: Time) -> f64 {
    // Envelope params
    let tau = Time(0.020); // decay ≈ 20 ms
    let alpha = 0.2; // attack shape
    let noise_level = 0.25; // amount of noise mixed in

    // Amplitude envelope: E(t) = t^α * exp(-t / τ)
    let env = time.as_secs().powf(alpha) * (-time / tau).exp();

    // Sum of inharmonic resonant partials
    let mut osc = 0.0;
    let k_partials = 8;
    let freq_bits = frequency.as_hz().to_bits();
    let time_bits = time.as_secs().to_bits();

    for i in 0..k_partials {
        // Build a unique seed for this partial
        let seed_base = freq_bits
            .wrapping_mul(0x9E3779B97F4A7C15)  // golden‐ratio prime salt
            ^ time_bits
            ^ (i as u64);

        // 1) δₖ ∈ [0.1, 1.5]
        let u1 = prng_unit(seed_base);
        let delta = 0.1 + u1 * (1.5 - 0.1);
        let fk = frequency * (1.0 + delta);

        // 2) amplitude roll‐off
        let ak = 1.0 / (1.0 + 0.5 * ((i + 1) as f64));

        // 3) phase φₖ ∈ [0, 2π)
        let u2 = prng_unit(seed_base ^ 0xDEADBEEF_DEADBEEF);
        let phase = u2 * 2.0 * PI;

        osc += ak * (fk.phase(time) + phase).sin();
    }

    // Deterministic “noise” component (as before)
    let raw = (time * Freq(1e7)).sin() * 1e6;
    let frac = raw.fract();
    let noise = 2.0 * frac - 1.0;

    // Mix resonators + noise under the same short envelope
    env * osc + noise_level * env * noise
}

/// Generate a kick‐drum–style sample at a given base frequency and time.
///
/// # Parameters
/// - `frequency`: the “tuned” pitch in Hz (e.g. 50–60 Hz) that the kick will drop towards.
/// - `time`: the time (t) in seconds at which to evaluate the kick.
///
/// # Returns
/// A single audio sample (f64) producing a short, punchy kick‐drum sound.
pub fn kick(time: Time) -> f64 {
    let frequency = Freq(110.0);
    // ——— Envelope parameters ———
    let amp_tau = Time(0.1); // main amplitude decay ≈ 200 ms
    let sweep_rate = Freq(10.0); // how quickly the pitch sweeps downward
    let noise_level = 0.15; // level of click‐noise on the attack
    let click_tau = 0.0025; // click‐noise decay ≈ 5 ms

    // 1) Amplitude envelope: exponential decay
    //    E_amp(t) = exp(−t / amp_tau)
    let env = (-time / amp_tau).exp();

    // 2) Frequency‐sweep oscillator:
    //    instantaneous phase = 2π ∫₀ᵗ f(t') dt'
    //    with f(t) = frequency * exp(−sweep_rate * t)
    //    ⇒ ∫₀ᵗ f exp(−s t) dt = frequency * (1 − exp(−sweep_rate·t)) / sweep_rate
    let phase = frequency.phase((1.0 - (-sweep_rate * time).exp()).div_by(sweep_rate));
    // let phase = PI * frequency * (1.0 - (-sweep_rate * time).exp()) / sweep_rate;
    let osc = phase.sin();

    // 3) Deterministic “click” noise on attack
    let raw = (time * Freq(1e7)).sin() * 1e6;
    // fract can be negative, so wrap it into [0,1)
    let frac = (raw.fract() + 1.0).fract();
    let noise = 2.0 * frac - 1.0;
    let click_env = (-time / Time(click_tau)).exp();

    // 4) Mix oscillator and click under their respective envelopes
    5.0 * (env * osc + noise_level * click_env * noise)
}

/// --- SNARE DRUM ---
/// Combines a noisy “crack” with a pitched “body”
/// - `frequency`: tuned pitch for the snare body (e.g. 200–300 Hz)
/// - `time`: time in seconds
pub fn snare(time: Time) -> f64 {
    // Envelope time‐constants
    let noise_tau = Time(0.1); // noise decay ≈ 150 ms
    let tone_tau = Time(0.2); // body decay ≈ 50 ms

    // Levels
    let noise_level = 0.15;
    let tone_level = 0.5;

    // Envelopes
    let env_noise = (-time / noise_tau).exp();
    let env_tone = (-time / tone_tau).exp();

    // ===== NOISE “CRACK” =====
    // deterministic pseudo‐noise from time
    let raw = (time * Freq(1e7)).sin() * 1e6;
    let frac = (raw.fract() + 1.0).fract();
    let noise = 2.0 * frac - 1.0;

    // ===== TONAL “BODY” =====
    // simple sine at fixed frequency, you could add a slight pitch-drop if desired
    let tone = Freq(110.0).phase(time).sin();

    5.0 * (env_noise * noise_level * noise + env_tone * tone_level * tone)
    // * (1.0 + 0.25 * kick(110.0, (time * 0.25).sqrt())) // WARNING: uncomment it if snare changed
}
/// --- RIDE CYMBAL ---
/// Metallic, sustained wash with a mid–high stick ping.
/// Purely time-domain, deterministic (no RNG state).
pub fn ride(frequency: Freq, time: Time) -> f64 {
    // ===== GLOBAL ENVELOPES =====
    // Long metallic decay for the wash:
    let wash_tau = Time(1.20); // ~1.2 s tail
    let wash_alpha = 0.05; // very fast micro-attack
    let wash_env = time.as_secs().powf(wash_alpha) * (-time / wash_tau).exp();

    // Mid–high "ping" that dies faster than the wash:
    let ping_tau = Time(0.40); // ~400 ms
    let ping_env = (-time / ping_tau).exp();

    // Tiny attack click (very short):
    let click_tau = Time(0.003); // ~3 ms
    let click_env = (-time / click_tau).exp();
    let click_level = 0.10;

    // ===== INHARMONIC WASH (sum of many partials) =====
    // We generate a set of inharmonic partials in a cymbal-like band.
    // Frequencies are around a few hundred Hz up to several kHz.
    let base = Freq(880.0) - frequency.rem_euclid(Freq::new(55.0)); // base band anchor (not an audible fundamental)
                                                                    // let base = frequency;
    let k_partials = 64; // more partials → denser wash

    let mut wash = 0.0;
    for i in 0..k_partials {
        // Stable, deterministic seeds per partial:
        let seed = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xC3A5_C85C_97CB_3127;

        // Inharmonic offset: spread partials roughly ×(2.5 .. 14) over base
        let u1 = prng_unit(seed); // [0,1)
        let band = 2.5 + u1 * (14.0 - 2.5); // ~ 2.5x .. 14x
        let fk = base * band;

        // Random phase per partial (but stable over time):
        let phase = prng_unit(seed ^ 0xDEAD_BEEF_DEAD_BEEF) * 2.0 * PI;

        // Smooth amplitude roll-off so higher partials contribute less:
        let ak = 1.0 / (1.0 + 0.18 * ((i + 1) as f64));

        wash += ak * (fk.phase(time) + phase).sin();
    }

    // ===== STICK "PING" =====
    // A focused tone around 2–4 kHz with a little second partial.
    // let ping: f64 = (0..12)
    //     .map(|i| {
    //         let seed = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xC3A5_C85C_97CB_3127;
    //         let u = 1.0 + prng_unit(seed);
    //         2000.0 * u as f64
    //     })
    //     .sum::<f64>()
    //     / 12.0;
    let ping_f1 = Freq(2611.0);
    let ping_f2 = Freq(2813.0);
    let ping_f3 = Freq(3223.0);
    let ping_f4 = Freq(3999.0);
    let ping = 0.4 * (ping_f1.phase(time)).sin()
        + 0.3 * (ping_f2.phase(time)).sin()
        + 0.2 * (ping_f3.phase(time)).sin()
        + 0.1 * (ping_f4.phase(time)).sin();

    // ===== ATTACK CLICK (deterministic “noise”) =====
    let raw = (time * Freq(1e7)).sin() * 1e6;
    let frac = (raw.fract() + 1.0).fract(); // wrap into [0,1)
    let noise = 2.0 * frac - 1.0;

    // ===== MIX =====
    // Balance: wash dominates, ping sits on top, tiny click on the transient.
    let out = 0.65 * wash_env * wash     // long metallic bed
            + 0.45 * ping_env * ping     // stick definition
            + click_level * click_env * noise;

    // Gentle overall gain
    0.9 * out
}
/// --- DARBUKA (DERBOUKA) ---
/// Hybrid hit: low "doum" body + bright "tek" rim + short attack click.
/// Purely time-domain and deterministic.
pub fn darbuka(frequency: Freq, time: Time) -> f64 {
    // ===== ENVELOPES =====
    // Low body "doum": slower decay
    let doum_tau = Time(0.15);
    let doum_env = (-time / doum_tau).exp();

    // Bright rim "tek": faster decay
    let tek_tau = Time(0.02);
    let tek_env = (-time / tek_tau).exp();

    // Very short attack click
    let click_tau = Time(0.0025);
    let click_env = (-time / click_tau).exp();

    // Subtle initial pitch rise on the rim (perceptual “snap”)
    fn glide(freq: Freq, t: Time, amt: f64, tau: Time) -> f64 {
        //  f(t) = f0 * (1 + amt * e^{-t/tau})
        freq.as_hz() * (1.0 + amt * (-t / tau).exp())
    }

    // ===== LOW BODY (membrane-like modes near ~150 Hz) =====
    // Ratios approximate circular membrane partials
    // let f0 = Freq(440.0);
    let f0 = frequency;
    let body = {
        let ratios = [
            (1.00, 1.00), // (ratio, relative decay scale)
            (1.59, 0.85),
            (2.14, 0.75),
            (2.30, 0.65),
        ];
        let gains = [1.00, 0.70, 0.50, 0.40];
        let mut s = 0.0;
        for ((r, dscale), g) in ratios.into_iter().zip(gains) {
            let f = Freq(f0.as_hz() * r);
            let env = (-time / Time(doum_tau.as_secs() * dscale)).exp();
            s += g * (f.phase(time)).sin() * env;
        }
        s
    };

    // ===== BRIGHT RIM / TEK (mid-high partials 1.8–5 kHz) =====
    let tek = {
        let base = Freq(2300.0);
        let g1 = 0.55 * (Freq(glide(base, time, 0.03, Time(0.03))).phase(time)).sin();
        let g2 = 0.35 * (Freq(glide(Freq(3300.0), time, 0.03, Time(0.03))).phase(time)).sin();
        let g3 = 0.25 * (Freq(glide(Freq(4100.0), time, 0.02, Time(0.02))).phase(time)).sin();
        let tmp = g1 + g2 + g3;
        let tmp = tmp.signum() * tmp.powi(2);
        tmp * tek_env
    };

    // ===== DETERMINISTIC ATTACK NOISE =====
    let raw = (time * Freq(1e7)).sin() * 1e6;
    let frac = (raw.fract() + 1.0).fract();
    let noise = 2.0 * frac - 1.0;

    // ===== MIX =====
    let out = 0.75 * doum_env * body   // body resonance
            + 0.60 * tek               // rim brightness
            + 0.10 * click_env * noise; // attack click

    // Gentle saturation to tame peaks
    (out * 1.2).tanh()
}
