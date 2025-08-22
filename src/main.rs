use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use notes::{notes_equal, Note, Sequence};
use serde_json as json;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

mod gui;
mod notes;
mod waves;
use waves::{basics::*, WaveType};
mod reverb;
use reverb::Reverb;

use crossterm::event::{poll, read, Event, KeyCode};

// // // // around 1/3 s
// const LEFT_DELAYS: [usize; 5] = [1, 14699, 14713, 14717, 14723];
// const RIGHT_DELAYS: [usize; 5] = [1, 14633, 14651, 14657, 14669];

const LEFT_DELAYS: [usize; 4] = [1, 14699, 22037, 7351];
const RIGHT_DELAYS: [usize; 4] = [1, 14713, 22051, 7349];

fn wait_for_exit_signal() -> bool {
    if poll(Duration::from_millis(100)).unwrap() {
        if let Event::Key(event) = read().unwrap() {
            return matches!(event.code, KeyCode::Char('q') | KeyCode::Esc);
        }
    }
    false
}

fn save_to_wav(filename: &str, sample_rate: f64, samples: &[f64], channels: u16) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: sample_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let path = format!("../audio/{}.wav", filename);
    let mut writer = hound::WavWriter::create(&path, spec).expect("Failed to create WAV file");

    let max_amp = 1.0;

    for &sample in samples {
        let scaled =
            (sample / max_amp * i16::MAX as f64).clamp(i16::MIN as f64, i16::MAX as f64) as i16;
        writer.write_sample(scaled).unwrap();
    }

    writer.finalize().expect("Failed to finalize WAV file");

    println!(
        "✅ Saved '{}' — {:.2} sec, {} channels at {} Hz",
        path,
        samples.len() as f64 / sample_rate / channels as f64,
        channels,
        sample_rate
    );
}

fn read_sequences_from_json(path: &str) -> Vec<Sequence> {
    let file = File::open(path).expect("Failed to open JSON file");
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).expect("Failed to parse JSON")
}

fn envelope(attack: f64, decay: f64, note_duration: f64) -> impl Fn(f64) -> f64 {
    move |time: f64| {
        let time_fraction = time / note_duration;
        0.1 * (time_fraction.powf(1.0 / attack) * (1.0 - time_fraction).powf(1.0 / decay)) as f64
    }
}

fn generate_wave(
    wave_type: &WaveType,
    freq: f64,
    time: f64,
    duration: f64,
    attack_decay: (f64, f64),
) -> f64 {
    let (a, d) = attack_decay;
    match wave_type {
        WaveType::Sine => envelope(a, d, duration)(time) * sine_wave(freq, time),
        WaveType::Square => envelope(a, d, duration)(time) * square_wave(freq, time),
        WaveType::Triangle => envelope(a, d, duration)(time) * triangle_wave(freq, time),
        WaveType::Sawtooth => envelope(a, d, duration)(time) * sawtooth_wave(freq, time),
        WaveType::DistOrg => envelope(a, d, duration)(time) * dist_org(freq, time),
        WaveType::Custom2 => envelope(a, d, duration)(time) * custom2(freq, time),
        WaveType::Droplet => envelope(a, d, duration)(time) * droplet_wave(freq, time),
        WaveType::DropletOct => envelope(a, d, duration)(time) * droplet_oct_wave(freq, time),
        WaveType::HiHat => envelope(a, d, duration)(time) * hi_hat(freq, time),
        WaveType::Kick => envelope(a, d, duration)(time) * kick(freq, time),
        WaveType::Snare => envelope(a, d, duration)(time) * snare(freq, time),
        WaveType::Ride => envelope(a, d, duration)(time) * ride(freq, time),
        WaveType::Mute => envelope(a, d, duration)(time) * mute_wave(freq, time),
        WaveType::Xylo => envelope(a, d, duration)(time) * xylophone_wave(freq, time),
    }
}

fn main() {
    let freq0 = 440.0f64;

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");
    let config = device.default_output_config().unwrap().config();
    let sample_rate = config.sample_rate.0 as f64;
    let sample_duration = 1.0 / sample_rate;
    let channels = config.channels;

    let note_queue: Arc<Mutex<Vec<Note>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded_samples = Arc::new(Mutex::new(Vec::new()));
    let sample_clock = Arc::new(Mutex::new(0f64));
    let running = Arc::new(AtomicBool::new(true));
    let shared_seqs: Arc<Mutex<Vec<Sequence>>> =
        Arc::new(Mutex::new(read_sequences_from_json("notes.json")));

    let mut reverb_left: Reverb<44100> = Reverb::new(0.5, 0.5, &LEFT_DELAYS);
    let mut reverb_right: Reverb<44100> = Reverb::new(0.5, 0.5, &RIGHT_DELAYS);

    // Start persistent audio stream
    let stream = {
        let note_queue = Arc::clone(&note_queue);
        let recorded_samples = Arc::clone(&recorded_samples);
        let sample_clock = Arc::clone(&sample_clock);

        device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut notes = note_queue.lock().unwrap();
                    let mut buffer = recorded_samples.lock().unwrap();
                    let mut clock = sample_clock.lock().unwrap();

                    for frame in data.chunks_mut(channels as usize) {
                        // let elapsed = *clock / sample_rate;
                        let elapsed = *clock;
                        let mut dry = 0.0;

                        notes.retain(|note| {
                            if elapsed < note.t {
                                true
                            } else if elapsed <= note.t + note.d {
                                let t = elapsed - note.t;
                                let volume = note.volume
                                    / (0.5
                                        + (0.5 * note.t).fract()
                                        + (1.2 * note.t).fract()
                                        + (2.5 * note.t).fract()
                                        + (3.0 * note.t).fract());
                                dry += volume
                                    * generate_wave(
                                        &note.w,
                                        freq0 * note.f.compute(),
                                        t,
                                        note.d,
                                        note.attack_decay,
                                    );
                                true
                            } else if elapsed > note.t + note.d + 16.0 {
                                false
                            } else {
                                true
                            }
                        });

                        let left = reverb_left.process(dry);
                        let right = reverb_right.process(dry);

                        if channels >= 2 {
                            frame[0] = left as f32;
                            frame[1] = right as f32;
                        } else {
                            frame[0] = (left + right) as f32 * 0.5;
                        }

                        buffer.push(left);
                        buffer.push(right);
                        *clock += sample_duration;
                    }
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            )
            .unwrap()
    };

    stream.play().unwrap();

    // 1.  Make *new* handles for the scheduler thread
    let note_queue_sched = Arc::clone(&note_queue);
    let sample_clock_sched = Arc::clone(&sample_clock);
    let recorded_samples_sched = Arc::clone(&recorded_samples);
    let shared_seqs_sched = Arc::clone(&shared_seqs);

    let running_sched = Arc::clone(&running);
    let scheduler = std::thread::spawn(move || {
        let mut rng = rand::thread_rng(); // local RNG (Send not required)

        // NEW: track last-seen sequences and all future scheduled notes per sequence
        let mut last_seqs = shared_seqs_sched.lock().unwrap().clone();
        let mut active_by_seq: HashMap<usize, Vec<Note>> = HashMap::new();

        println!("🎵 Press 'q' or 'Esc' in this terminal to quit...");
        let mut batch_index = 0;
        let batch_interval = 64.0;

        while running_sched.load(Ordering::Relaxed) {
            let start_time = batch_interval * batch_index as f64;
            // NEW: detect GUI edits and re-generate remaining notes for changed sequences
            {
                let curr_seqs = shared_seqs_sched.lock().unwrap().clone();
                if curr_seqs != last_seqs {
                    let now = *sample_clock_sched.lock().unwrap();

                    let max_len = curr_seqs.len().max(last_seqs.len());
                    for i in 0..max_len {
                        let old_seq = last_seqs.get(i);
                        let new_seq = curr_seqs.get(i);

                        let changed = match (old_seq, new_seq) {
                            (Some(a), Some(b)) => {
                                // compare by JSON string to avoid adding PartialEq on WaveType/Interval
                                json::to_string(a).ok() != json::to_string(b).ok()
                            }
                            (Some(_), None) => true, // removed
                            (None, Some(_)) => true, // added
                            (None, None) => false,
                        };

                        if !changed {
                            continue;
                        }

                        // 1) Remove all *future* notes for this sequence from the global queue
                        //    (keep already-started notes)
                        if let Some(old_future) = active_by_seq.remove(&i) {
                            let mut q = note_queue_sched.lock().unwrap();
                            q.retain(|n| {
                                !(n.t >= now && old_future.iter().any(|m| notes_equal(n, m)))
                            });
                        }

                        // 2) If the sequence still exists, re-generate the remainder of the *current* batch
                        if let Some(seq) = new_seq {
                            let start_time = (now / batch_interval).floor() * batch_interval;
                            let end_time = start_time + batch_interval;

                            // Build context from other sequences' notes already scheduled in this batch
                            // (convert back to local batch time by subtracting start_time)
                            let mut context: Vec<Note> = active_by_seq
                                .iter()
                                .filter(|(j, _)| **j != i)
                                .flat_map(|(_, ns)| {
                                    ns.iter()
                                        .filter(|n| n.t >= start_time && n.t < end_time)
                                        .map(|n| {
                                            let mut m = n.clone();
                                            m.t -= start_time;
                                            m
                                        })
                                        .collect::<Vec<_>>()
                                })
                                .collect();

                            // Generate new notes for this seq (in local batch time)
                            let before = context.len();
                            seq.clone().draw(&mut context, &mut rng);
                            let mut new_local = context[before..].to_vec();

                            // Shift to absolute time and keep only notes that haven't started yet
                            for n in &mut new_local {
                                n.t += start_time;
                            }
                            let new_future: Vec<Note> =
                                new_local.into_iter().filter(|n| n.t >= now).collect();

                            // Publish to the queue and remember them
                            note_queue_sched.lock().unwrap().extend(new_future.clone());
                            active_by_seq.insert(i, new_future);
                        }
                    }

                    last_seqs = curr_seqs;
                }
            }

            // ----- create notes exactly like before, but keep them grouped per sequence -----
            let instruments = shared_seqs_sched.lock().unwrap().clone();

            // Build per-sequence groups while preserving cross-sequence context.
            // We use a single 'context' Vec<Note> exactly like before so interaction logic remains identical.
            let mut context = Vec::<Note>::new();
            let mut groups: Vec<(usize, Vec<Note>)> = Vec::new();

            for (idx, inst) in instruments.iter().enumerate() {
                let before = context.len();
                inst.draw(&mut context, &mut rng); // writes its notes to 'context'
                let group = context[before..].to_vec(); // slice out *just* this sequence's new notes
                groups.push((idx, group));
            }

            // Shift groups to absolute time, remember them, then flatten to the audio queue.
            let mut flat: Vec<Note> = Vec::new();
            for (idx, mut ns) in groups {
                for n in &mut ns {
                    n.t += start_time;
                }
                active_by_seq.entry(idx).or_default().extend(ns.clone());
                flat.extend(ns);
            }

            note_queue_sched.lock().unwrap().extend(flat);
            batch_index += 1;

            // ----- timing -----
            let now = *sample_clock_sched.lock().unwrap();
            let target = batch_interval * batch_index as f64;
            if target > now {
                std::thread::sleep(std::time::Duration::from_secs_f64(target - now - 1.0));
            }

            // ----- exit? -----
            if wait_for_exit_signal() {
                println!("Exiting. Please enter a filename:");
                let mut name = String::new();
                std::io::stdin().read_line(&mut name).unwrap();
                let name = name.trim();

                let buffer = recorded_samples_sched.lock().unwrap();
                save_to_wav(name, sample_rate, &buffer, channels);
                break; // leave the loop → thread ends
            }
            // manual quit?
            if wait_for_exit_signal() {
                println!("Exiting. Please enter a filename:");
                // … WAV export …
                running_sched.store(false, Ordering::Relaxed); // tell everybody else to stop
                break;
            }
        }
    });

    gui::run_gui(
        Some(Arc::clone(&sample_clock)),
        Some(Arc::clone(&shared_seqs)),
    );

    running.store(false, Ordering::Relaxed); // <- tell the scheduler to finish

    // Wait for the scheduler thread to finish (it will exit automatically
    // if the user already pressed q/Esc; otherwise closing the GUI window
    // doesn’t stop it, so you may want a channel/flag – see below).
    scheduler.join().ok();
}
