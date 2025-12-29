use arc_swap::ArcSwap;
use core::panic;
use cpal::traits::{DeviceTrait, StreamTrait};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{atomic::AtomicU64, Arc},
};

use crate::{
    engine::{
        reverb::Reverb,
        score::{note::NoteVariant, NotesGroup},
        waves::{generate_wave, generate_wave_with_phase},
    },
    recorder::Recorder,
    time_freq::{DivByFreq, Freq, Time},
    NoteId, Token, REVERB_BUFFER_LEN,
};

struct NoteMemory {
    start_time: Time,
    lowpass_state: [f64; 10],
    phase: f64,
}

pub fn stream(
    freq0: Freq,
    device: &cpal::Device,
    clock: Arc<AtomicU64>,
    note_queue: Arc<ArcSwap<BTreeMap<Token, NotesGroup>>>,
    recorder: Option<Arc<Recorder>>,
) -> cpal::Stream {
    let config = device.default_output_config().unwrap();
    if config.sample_format() != cpal::SampleFormat::F32 {
        log::log!(log::Level::Error, "not f32");
        panic!("not f32")
    }
    let config = config.config();
    let sample_rate = Freq(config.sample_rate.0 as f64);
    // println!("sample rate from callback: {sample_rate}");
    let channels = config.channels;
    let stream = {
        let mut lp_memories: HashMap<NoteId, NoteMemory> = HashMap::new();
        let mut track_reverbs: HashMap<
            Token,
            (Reverb<REVERB_BUFFER_LEN>, Reverb<REVERB_BUFFER_LEN>),
        > = HashMap::new();
        let mut last_cleanup = crate::time_freq::Time(0.0);
        let sample_rate_hz = sample_rate.as_hz();
        let recorder = recorder.clone();
        let callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let note_groups = note_queue.load();
            let channels_usize = channels as usize;
            let frames = data.len() / channels_usize;
            let sample_step = 1.0.div_by(sample_rate);

            let mut now =
                (clock.load(std::sync::atomic::Ordering::Relaxed) as f64).div_by(sample_rate);

            for frame in data.chunks_mut(channels_usize) {
                let mut mixed_left = 0.0;
                let mut mixed_right = 0.0;

                for (
                    token,
                    NotesGroup {
                        notes: notes_from_seq,
                        bend,
                        vibrato,
                        wave_type,
                        chorus,
                        attack_decay,
                        cutoff,
                        power,
                        noise,
                        pan,
                        volume,
                        lowpass_enabled,
                        lp_order,
                        filter_type,
                        harmonics,
                        delays: track_delays,
                        ..
                    },
                ) in note_groups.iter()
                {
                    let mut track_left = 0.0;
                    let mut track_right = 0.0;

                    for note in notes_from_seq.iter() {
                        let memory = lp_memories.entry(note.id).or_insert_with(|| NoteMemory {
                            start_time: note.time,
                            lowpass_state: [0.0; 10],
                            phase: 0.0,
                        });
                        if note.time <= now && now <= note.time + note.duration {
                            let t = (now - note.time).rem_euclid(note.duration);
                            let volume = 0.1 * volume * note.volume;
                            let dry = volume
                                * match note.variant {
                                    NoteVariant::PureTime => generate_wave(
                                        wave_type,
                                        freq0 * note.interval.compute(),
                                        note.glide.as_ref().map(|g| freq0 * g.compute()),
                                        t,
                                        note.duration,
                                        *attack_decay,
                                        cutoff,
                                        *bend,
                                        *vibrato,
                                        chorus,
                                        harmonics,
                                        power,
                                        noise,
                                        *lowpass_enabled,
                                        *filter_type,
                                        *lp_order,
                                        &mut memory.lowpass_state,
                                        sample_rate,
                                        now,
                                    ),
                                    NoteVariant::PhaseTracked => generate_wave_with_phase(
                                        wave_type,
                                        freq0 * note.interval.compute(),
                                        note.glide.as_ref().map(|g| freq0 * g.compute()),
                                        t,
                                        note.duration,
                                        *attack_decay,
                                        cutoff,
                                        *bend,
                                        *vibrato,
                                        chorus,
                                        harmonics,
                                        power,
                                        noise,
                                        *lowpass_enabled,
                                        *filter_type,
                                        *lp_order,
                                        &mut memory.lowpass_state,
                                        &mut memory.phase,
                                        sample_rate,
                                        now,
                                        sample_step,
                                    ),
                                };
                            track_left += (1.0 - pan) * dry;
                            track_right += pan * dry;
                        }
                    }

                    let entry = track_reverbs.entry(*token).or_insert_with(|| {
                        (
                            Reverb::new(0.5, 0.5, sample_rate_hz),
                            Reverb::new(0.5, 0.5, sample_rate_hz),
                        )
                    });
                    let processed_left = entry.0.process(track_left, &track_delays.0);
                    let processed_right = entry.1.process(track_right, &track_delays.1);
                    mixed_left += processed_left;
                    mixed_right += processed_right;
                }

                // Periodic cleanup of old low-pass filter memories to prevent unbounded growth
                const CLEANUP_INTERVAL: f64 = 10.0;
                if (now - last_cleanup).as_secs() > CLEANUP_INTERVAL {
                    let cutoff_time = now - crate::NOTE_LINGER_TIME - Time(5.0);
                    lp_memories.retain(|_, state| state.start_time >= cutoff_time);
                    track_reverbs.retain(|tk, _| note_groups.contains_key(tk));
                    last_cleanup = now;
                }

                let left = mixed_left;
                let right = mixed_right;

                if let Some(rec) = recorder.as_ref() {
                    rec.write_frame(left as f32, right as f32);
                }

                if channels_usize >= 2 {
                    frame[0] = left as f32;
                    frame[1] = right as f32;
                } else {
                    frame[0] = (left + right) as f32;
                }

                now += sample_step;
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
