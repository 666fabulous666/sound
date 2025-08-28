use std::f64::consts::PI;

pub fn sine_wave(frequency: f64, time: f64) -> f64 {
    let attack_slide = (frequency / 440.0).sqrt();
    let tt = time + attack_slide * (1.66 + 3.0 * time).powi(-10);
    // let tt = tt + 1.0e-2 / frequency * (2.0 * PI * 24.0 * tt).sin();
    let phase = 2.0 * PI * (frequency) * tt;
    phase.sin() / attack_slide
}
pub fn sine(frequency: f64, time: f64) -> f64 {
    let phase = 2.0 * PI * (frequency) * time;
    phase.sin()
}
pub fn cosine(frequency: f64, time: f64) -> f64 {
    let phase = 2.0 * PI * (frequency) * time;
    phase.cos()
}

pub fn droplet_wave(frequency: f64, time: f64) -> f64 {
    let attack_slide = (frequency / 440.0).sqrt();
    let tt = time + attack_slide * (1.66 + 3.0 * time).powi(-10);
    let delta = 5e-3;
    let phase = 2.0 * PI * (frequency) * tt;
    (0..4)
        .map(|k| {
            (0.5f64).powi(k)
                * ((phase * (1.0 + 2.0f64.powi(k) * delta)).sin()
                    + (phase * (1.0 - 2.0f64.powi(k) * delta)).sin())
        })
        .sum::<f64>()
        * 0.5
        / attack_slide
}
pub fn xylophone_wave(frequency: f64, time: f64) -> f64 {
    let delta = 3e-2 / (1.0 + 1e2 * time);
    let phase = 2.0 * PI * (frequency) * time;
    (0..9)
        .map(|k| {
            (0.667f64).powi(k)
                * (0.333 * (phase * (1.0 + 2.0f64.powi(k) * delta)).sin()
                    - 0.667 * (phase * (1.0 - 2.0f64.powi(k) * delta)).sin())
        })
        .sum::<f64>()
        / (frequency / 440.0).sqrt()
}
pub fn mute_wave(_frequency: f64, _time: f64) -> f64 {
    0.0
}
pub fn droplet_oct_wave(frequency: f64, time: f64) -> f64 {
    0.3333
        * (droplet_wave(frequency, time)
            + droplet_wave(2.0 * frequency, time)
            + droplet_wave(0.5 * frequency, time))
}

pub fn square_wave(frequency: f64, time: f64) -> f64 {
    if (frequency * time) % 1.0 < 0.5 {
        0.25
    } else {
        -0.25
    }
}

pub fn triangle_wave(frequency: f64, time: f64) -> f64 {
    let t = frequency * time;
    2.0 * (t - (t + 0.75).floor() + 0.25).abs() - 1.0
}

pub fn sawtooth_wave(frequency: f64, time: f64) -> f64 {
    frequency * time - (0.5 + frequency * time).floor()
}

pub fn dist_org(frequency: f64, time: f64) -> f64 {
    let tmp = 0.1
        * (sine(frequency * 1.005, time) + 2.0 * sine(frequency * 1.002, time)
            - 3.0 * sine(frequency, time)
            + 2.0 * sine(frequency * 0.998, time)
            + sine(frequency * 0.995, time));
    (440.0 / frequency).sqrt()
        * tmp.signum()
        * tmp.abs().powf(
            1.0 + (sine(41.0, time.sqrt())
                * cosine(29.0, time.sqrt())
                * sine(23.0, time.sqrt())
                * cosine(19.0, time.sqrt())
                * sine(17.0, time.sqrt())
                * cosine(13.0, time.sqrt())),
        )
}

pub fn custom2(_frequency: f64, _time: f64) -> f64 {
    todo!()
}

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

/// --- RIDE/BELL HYBRID ---
/// Metallic “bell” tone plus noisy ring modulation.
/// - `frequency`: the fundamental “bell” pitch in Hz (e.g. 440.0 for A4)
/// - `time`: time in seconds
pub fn ride(frequency: f64, time: f64) -> f64 {
    // ——— Envelope parameters ———
    // Bell component: soft attack + long bell decay
    let tau_bell = 0.4; // ≈1.0 s decay
    let alpha_bell = 0.3; // gentle rise
    let bell_env = time.powf(alpha_bell) * (-time / tau_bell).exp();

    // Noise/ring component: moderate decay
    let tau_noise = 0.2; // ≈0.7 s decay
    let alpha_noise = 0.3; // gentle rise
    let noise_env = time.powf(alpha_noise) * (-time / tau_noise).exp();

    // Mix levels
    let bell_level = 0.2;
    let noise_level = 0.4;

    // ——— Bell‐like harmonic partials ———
    let mut bell = 0.0;
    let harmonics = 10;
    let fq_bits = frequency.to_bits();
    let tm_bits = time.to_bits();

    for k in 1..=harmonics {
        // build seed and slight inharmonic detune
        let seed = fq_bits.wrapping_mul(0x9E3779B97F4A7C15) ^ tm_bits ^ (k as u64);
        let detune = 1.0 + (prng_unit(seed) - 0.5) * 0.04; // ±2% detune
        let fk = frequency * (k as f64) * detune;

        // roll‐off amplitude of higher harmonics
        let ak = 1.0 / (1.0 + 0.2 * ((k - 1) as f64));

        // random phase
        let phase = prng_unit(seed ^ 0xDEADBEEF_DEADBEEF) * 2.0 * PI;

        bell += ak * (2.0 * PI * fk * time + phase).sin();
    }

    // ——— Noisy “ring” via ring‐modulated noise ———
    // deterministic noise from time
    let raw = (time * 5e6).sin() * 1e6;
    let frac = (raw.fract() + 1.0).fract();
    let noise = 2.0 * frac - 1.0;
    // ring‐modulate noise at the bell frequency
    let ring = noise * (2.0 * PI * frequency * time).sin();

    // ——— Final mix ———
    bell_level * bell_env * bell + noise_level * noise_env * ring
}
