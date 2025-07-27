use std::f64::consts::PI;

pub fn sine_wave(frequency: f64, time: f64) -> f64 {
    let attack_slide = (frequency / 440.0).sqrt();
    let tt = time + attack_slide * (1.66 + 3.0 * time).powi(-10);
    // let tt = tt + 1.0e-2 / frequency * (2.0 * PI * 24.0 * tt).sin();
    let phase = 2.0 * PI * (frequency) * tt;
    phase.sin() / attack_slide
}

pub fn droplet_wave(frequency: f64, time: f64) -> f64 {
    let attack_slide = (frequency / 440.0).sqrt();
    let tt = time + attack_slide * (1.66 + 3.0 * time).powi(-10);
    let delta = 5e-3;
    let phase = 2.0 * PI * (frequency) * tt;
    (0..5)
        .map(|k| {
            (0.5f64).powi(k)
                * ((phase * (1.0 + 2.0f64.powi(k) * delta)).sin()
                    + (phase * (1.0 - 2.0f64.powi(k) * delta)).sin())
        })
        .sum::<f64>()
        * 0.5
        / attack_slide
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

pub fn custom1(frequency: f64, time: f64) -> f64 {
    let attack_slide = (frequency / 440.0).sinh().min(2.0);
    let tt = time + attack_slide * (1.25 + time).powi(-10);
    // let tt = time;
    let tt = tt + 5e-5 * (32.0 * tt).sin();
    let tmp = sine_wave(frequency, tt);
    tmp.signum() * tmp.abs().powf(1.0 + 0.1 * time)
}

pub fn custom2(frequency: f64, time: f64) -> f64 {
    let attack_slide = (frequency / 880.0).sinh().min(1.0);
    let tt = time + attack_slide * (5.75 + 3.0 * time).powi(-10);
    let tt = tt + 5e-5 * (32.0 * tt).sin();
    let tmp = sine_wave(frequency, tt);
    tmp.signum() * tmp.abs().powf(1.0 / (0.2 + time * time))
}
