// ===================== audio.rs =====================
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::theory::{envelope, osc_sample, RenderNote};

pub fn play_notes(notes: Vec<RenderNote>) -> anyhow::Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No default output device"))?;
    let config = device.default_output_config()?.config();

    let sample_rate = config.sample_rate.0 as f32;

    let err_fn = |err| eprintln!("Stream error: {err}");

    let sample_clock = Arc::new(Mutex::new(0f32));
    let notes = Arc::new(notes);

    let stream = device.build_output_stream(
        &config,
        {
            let sample_clock = Arc::clone(&sample_clock);
            let notes = Arc::clone(&notes);
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                let mut clock = sample_clock.lock().unwrap();
                for sample in data.iter_mut() {
                    let elapsed = *clock / sample_rate;
                    let mut value = 0.0f32;
                    for n in notes.iter() {
                        if elapsed >= n.start && elapsed <= n.start + n.duration {
                            let t = elapsed - n.start;
                            let env = envelope(n.env_attack, n.env_decay, n.duration)(t);
                            value += env * osc_sample(n.wave, n.freq, t);
                        }
                    }
                    *sample = value;
                    *clock += 1.0;
                }
            }
        },
        err_fn,
        None,
    )?;

    stream.play()?;

    let total_duration = notes
        .iter()
        .map(|n| n.start + n.duration)
        .fold(0.0f32, f32::max);
    thread::sleep(Duration::from_secs_f32(total_duration + 0.5));
    Ok(())
}
