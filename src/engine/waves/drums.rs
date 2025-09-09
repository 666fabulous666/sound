use std::f64::consts::PI;

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
pub fn hi_hat(frequency: f64, time: f64) -> f64 {
    // Envelope params
    let tau = 0.020; // decay ≈ 20 ms
    let alpha = 0.2; // attack shape
    let noise_level = 0.25; // amount of noise mixed in

    // Amplitude envelope: E(t) = t^α * exp(-t / τ)
    let env = time.powf(alpha) * (-time / tau).exp();

    // Sum of inharmonic resonant partials
    let mut osc = 0.0;
    let k_partials = 8;
    let freq_bits = frequency.to_bits();
    let time_bits = time.to_bits();

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

        osc += ak * (2.0 * PI * fk * time + phase).sin();
    }

    // Deterministic “noise” component (as before)
    let raw = (time * 1e7).sin() * 1e6;
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
pub fn kick(_frequency: f64, time: f64) -> f64 {
    let frequency = 220.0;
    // ——— Envelope parameters ———
    let amp_tau = 0.1; // main amplitude decay ≈ 200 ms
    let sweep_rate = 10.0; // how quickly the pitch sweeps downward
    let noise_level = 0.15; // level of click‐noise on the attack
    let click_tau = 0.0025; // click‐noise decay ≈ 5 ms

    // 1) Amplitude envelope: exponential decay
    //    E_amp(t) = exp(−t / amp_tau)
    let env = (-time / amp_tau).exp();

    // 2) Frequency‐sweep oscillator:
    //    instantaneous phase = 2π ∫₀ᵗ f(t') dt'
    //    with f(t) = frequency * exp(−sweep_rate * t)
    //    ⇒ ∫₀ᵗ f exp(−s t) dt = frequency * (1 − exp(−sweep_rate·t)) / sweep_rate
    let phase = PI * frequency * (1.0 - (-sweep_rate * time).exp()) / sweep_rate;
    let osc = phase.sin();

    // 3) Deterministic “click” noise on attack
    let raw = (time * 1e7).sin() * 1e6;
    // fract can be negative, so wrap it into [0,1)
    let frac = (raw.fract() + 1.0).fract();
    let noise = 2.0 * frac - 1.0;
    let click_env = (-time / click_tau).exp();

    // 4) Mix oscillator and click under their respective envelopes
    5.0 * (env * osc + noise_level * click_env * noise)
}

/// --- SNARE DRUM ---
/// Combines a noisy “crack” with a pitched “body”
/// - `frequency`: tuned pitch for the snare body (e.g. 200–300 Hz)
/// - `time`: time in seconds
pub fn snare(_frequency: f64, time: f64) -> f64 {
    // Envelope time‐constants
    let noise_tau = 0.1; // noise decay ≈ 150 ms
    let tone_tau = 0.2; // body decay ≈ 50 ms

    // Levels
    let noise_level = 0.15;
    let tone_level = 0.5;

    // Envelopes
    let env_noise = (-time / noise_tau).exp();
    let env_tone = (-time / tone_tau).exp();

    // ===== NOISE “CRACK” =====
    // deterministic pseudo‐noise from time
    let raw = (time * 1e7).sin() * 1e6;
    let frac = (raw.fract() + 1.0).fract();
    let noise = 2.0 * frac - 1.0;

    // ===== TONAL “BODY” =====
    // simple sine at fixed frequency, you could add a slight pitch-drop if desired
    let tone = (2.0 * PI * 110.0 * time).sin();

    5.0 * (env_noise * noise_level * noise + env_tone * tone_level * tone)
        * (1.0 + 0.25 * kick(110.0, (time * 0.25).sqrt()))
}
