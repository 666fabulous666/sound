use arc_swap::ArcSwap;
use core::panic;
use cpal::traits::{DeviceTrait, StreamTrait};
use std::sync::{Arc, Mutex};

use crate::{
    engine::{notes::Note, reverb::Reverb, waves::generate_wave},
    NOTE_LINGER_TIME, REVERB_BUFFER_LEN,
};

pub fn stream(
    freq0: f64,
    device: &cpal::Device,
    sample_clock: Arc<Mutex<f64>>,
    note_queue: Arc<ArcSwap<Vec<(usize, Vec<Note>)>>>,
    (mut reverb_left, mut reverb_right): (Reverb<REVERB_BUFFER_LEN>, Reverb<REVERB_BUFFER_LEN>),
) -> cpal::Stream {
    let config = device.default_output_config().unwrap();
    if config.sample_format() != cpal::SampleFormat::F32 {
        log::log!(log::Level::Error, "not f32");
        panic!("not f32")
    }
    let config = config.config();
    let sample_rate = config.sample_rate.0 as f64;
    let sample_duration = 1.0 / sample_rate;
    let channels = config.channels;
    let stream = {
        // let recorded_samples = recorded_samples.clone();
        let sample_clock = sample_clock.clone();

        let callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let note_queue = note_queue.load();
            // let mut recorded_samples = recorded_samples.lock().unwrap();
            let mut sample_clock = sample_clock.lock().unwrap();

            for frame in data.chunks_mut(channels as usize) {
                let elapsed = *sample_clock;
                let mut dry_left = 0.0;
                let mut dry_right = 0.0;

                for (_, notes_from_seq) in note_queue.iter() {
                    // FIXME: no longer removes outdated notes
                    notes_from_seq.iter().for_each(|note| {
                        if note.time < elapsed && elapsed <= note.time + note.duration {
                            let t = elapsed - note.time;
                            let volume = note.volume // TODO: make this parameters
                                    / (1.5
                                        + (0.5 * note.time).fract()
                                        + (1.2 * note.time).fract()
                                        + (2.5 * note.time).fract()
                                        + (3.0 * note.time).fract());
                            let dry = volume
                                * generate_wave(
                                    &note.wave_type,
                                    freq0 * note.interval.compute(),
                                    t,
                                    note.duration,
                                    note.attack_decay,
                                    note.bend,
                                    note.vibrato,
                                    &note.chorus,
                                    note.pow_fact,
                                );
                            dry_left += (1.0 - note.spacial) * dry;
                            dry_right += note.spacial * dry;
                        }
                    })
                }

                let left = reverb_left.process(dry_left);
                let right = reverb_right.process(dry_right);

                if channels >= 2 {
                    frame[0] = left as f32;
                    frame[1] = right as f32;
                } else {
                    frame[0] = (left + right) as f32;
                }
                // recorded_samples.push(left);
                // recorded_samples.push(right);
                *sample_clock += sample_duration;
            }
        };

        let stream = match device.build_output_stream(
            &config,
            callback,
            |err| eprintln!("Stream error: {}", err),
            None,
        ) {
            Err(err) => {
                log::log!(log::Level::Error, "{err}");
                panic!("{err}")
            }
            Ok(stream) => stream,
        };
        if let Err(err) = stream.play() {
            log::log!(log::Level::Error, "{err}");
            panic!("{err}")
        }
        stream
    };
    stream
}
