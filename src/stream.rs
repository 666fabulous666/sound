use arc_swap::ArcSwap;
use core::panic;
use cpal::traits::{DeviceTrait, StreamTrait};
use std::sync::{atomic::AtomicU64, Arc};

use crate::{
    engine::{notes::Note, reverb::Reverb, waves::generate_wave},
    Token, REVERB_BUFFER_LEN,
};

pub fn stream(
    freq0: f64,
    device: &cpal::Device,
    clock: Arc<AtomicU64>,
    note_queue: Arc<ArcSwap<Vec<(Token, Vec<Note>)>>>,
    (mut reverb_left, mut reverb_right): (Reverb<REVERB_BUFFER_LEN>, Reverb<REVERB_BUFFER_LEN>), // FIXME: should be dynamically shared with the callback
    delays: Arc<ArcSwap<(Vec<f64>, Vec<f64>)>>,
) -> cpal::Stream {
    let config = device.default_output_config().unwrap();
    if config.sample_format() != cpal::SampleFormat::F32 {
        log::log!(log::Level::Error, "not f32");
        panic!("not f32")
    }
    let config = config.config();
    let sample_rate = config.sample_rate.0 as f64;
    // println!("sample rate from callback: {sample_rate}");
    let channels = config.channels;
    let stream = {
        let callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let notes = note_queue.load();
            let delays = delays.load();
            let channels_usize = channels as usize;
            let frames = data.len() / channels_usize;

            let mut now = clock.load(std::sync::atomic::Ordering::Relaxed) as f64 / sample_rate;

            for frame in data.chunks_mut(channels_usize) {
                let mut dry_left = 0.0;
                let mut dry_right = 0.0;

                for (_, notes_from_seq) in notes.iter() {
                    for note in notes_from_seq {
                        if note.time < now && now <= note.time + note.duration {
                            let t = now - note.time;
                            let volume = note.volume;
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
                    }
                }

                let left = reverb_left.process(dry_left, &delays.0);
                let right = reverb_right.process(dry_right, &delays.1);

                if channels_usize >= 2 {
                    frame[0] = left as f32;
                    frame[1] = right as f32;
                } else {
                    frame[0] = (left + right) as f32;
                }

                now += 1.0 / sample_rate;
            }

            clock.fetch_add(frames as u64, std::sync::atomic::Ordering::Relaxed);
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
