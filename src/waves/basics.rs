pub fn sine_wave(frequency: f32, time: f32) -> f32 {
    let attack_slide = (frequency / 440.0).powf(0.5);
    // let attack_slide = (frequency / 220.0).sinh().min(2.0);
    let tt = time + attack_slide * (1.66 + 3.0 * time).powi(-10);
    // let tt = tt + 2.5e-4 * (128.0 * tt).sin();
    (2.0 * std::f32::consts::PI * frequency * tt).sin() / attack_slide
}

pub fn square_wave(frequency: f32, time: f32) -> f32 {
    if (frequency * time) % 1.0 < 0.5 {
        0.25
    } else {
        -0.25
    }
}

pub fn triangle_wave(frequency: f32, time: f32) -> f32 {
    let t = frequency * time;
    2.0 * (t - (t + 0.75).floor() + 0.25).abs() - 1.0
}

pub fn sawtooth_wave(frequency: f32, time: f32) -> f32 {
    frequency * time - (0.5 + frequency * time).floor()
}

pub fn custom1(frequency: f32, time: f32) -> f32 {
    let attack_slide = (frequency / 440.0).sinh().min(2.0);
    let tt = time + attack_slide * (1.25 + time).powi(-10);
    // let tt = time;
    let tt = tt + 5e-5 * (32.0 * tt).sin();
    let tmp = sine_wave(frequency, tt);
    tmp.signum() * tmp.abs().powf(1.0 + 0.1 * time)
}

pub fn custom2(frequency: f32, time: f32) -> f32 {
    let attack_slide = (frequency / 880.0).sinh().min(1.0);
    let tt = time + attack_slide * (5.75 + 3.0 * time).powi(-10);
    let tt = tt + 5e-5 * (32.0 * tt).sin();
    let tmp = sine_wave(frequency, tt);
    tmp.signum() * tmp.abs().powf(1.0 / (0.2 + time * time))
}
