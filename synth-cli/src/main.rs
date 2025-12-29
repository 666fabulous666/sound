use anyhow::{bail, Context, Result};
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait};
use std::path::PathBuf;
use std::sync::{atomic::AtomicU64, Arc};
use std::thread;
use std::time::Duration;
use synth_core::{
    engine::{
        reverb::Reverb,
        score::Score,
    },
    session::SessionState,
    recorder::Recorder,
    stream::stream,
    F0, REVERB_BUFFER_LEN,
};

/// Command-line JSON player for Quantum Harmonics' Oscillator
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// JSON file to play
    #[arg(value_name = "FILE")]
    file: PathBuf,

    /// Duration to play in seconds (0 = play forever)
    #[arg(short, long, default_value = "0")]
    duration: u64,

    /// Precompute mode: render audio to buffer first, then play
    #[arg(short, long)]
    precompute: bool,

    /// Record output to WAV file
    #[arg(long)]
    record: Option<PathBuf>,
}

/// Precompute audio and play from buffer
fn precompute_and_play(
    mut score: Score,
    device: &cpal::Device,
    sample_rate: f64,
    duration_secs: u64,
) -> Result<()> {
    use synth_core::{engine::waves::generate_wave, time_freq::Time};

    println!("Precomputing {} seconds of audio...", duration_secs);

    let mut rng = rand::thread_rng();
    let total_samples = (sample_rate * duration_secs as f64) as usize;
    let mut audio_buffer = vec![0.0f32; total_samples * 2]; // stereo

    // Generate all notes for the duration
    for t in 0..10 {
        let now = Time(t as f64 * 0.5);
        score.generate_notes(now, &mut rng);
    }
    score.update_mix_from_tree();

    println!("Rendering {} samples...", total_samples);

    // Render audio
    for i in 0..total_samples {
        let now = Time((i as f64) / sample_rate);
        let mut left = 0.0;
        let mut right = 0.0;

        for (_token, ng) in &score.notes {
            for note in &ng.notes {
                if note.time <= now && now <= note.time + note.duration {
                    let t = (now - note.time).rem_euclid(note.duration);
                    let volume = 0.1 * ng.volume * note.volume;
                    let mut memory = [0.0; 10];

                    let dry = volume
                        * generate_wave(
                            &ng.wave_type,
                            synth_core::F0 * note.interval.compute(),
                            note.glide.as_ref().map(|g| synth_core::F0 * g.compute()),
                            t,
                            note.duration,
                            ng.attack_decay,
                            &ng.cutoff,
                            ng.bend,
                            ng.vibrato,
                            &ng.chorus,
                            &ng.harmonics,
                            &ng.power,
                            &ng.noise,
                            ng.lowpass_enabled,
                            ng.filter_type,
                            ng.lp_order,
                            &mut memory,
                            synth_core::time_freq::Freq(sample_rate),
                            now,
                        );

                    left += (1.0 - ng.pan) * dry;
                    right += ng.pan * dry;
                }
            }
        }

        audio_buffer[i * 2] = left as f32;
        audio_buffer[i * 2 + 1] = right as f32;
    }

    println!("Playing precomputed audio...");

    // Play buffer
    let buffer_arc = Arc::new(audio_buffer);
    let buffer_pos = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let buffer_clone = buffer_arc.clone();
    let pos_clone = buffer_pos.clone();

    let stream = {
        let callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(2) {
                let pos = pos_clone.fetch_add(2, std::sync::atomic::Ordering::Relaxed);
                if pos < buffer_clone.len() {
                    frame[0] = buffer_clone[pos];
                    frame[1] = buffer_clone[pos + 1];
                } else {
                    frame[0] = 0.0;
                    frame[1] = 0.0;
                }
            }
        };

        device
            .build_output_stream(
                &device.default_output_config()?.config(),
                callback,
                |err| eprintln!("Stream error: {}", err),
                None,
            )
            .context("Failed to build output stream")?
    };

    use cpal::traits::StreamTrait;
    stream.play().context("Failed to start stream")?;

    println!("Playing for {} seconds", duration_secs);
    thread::sleep(Duration::from_secs(duration_secs));

    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    // Load JSON file
    let json_content = std::fs::read_to_string(&args.file)
        .with_context(|| format!("Failed to read file: {:?}", args.file))?;

    let state: SessionState =
        serde_json::from_str(&json_content).with_context(|| "Failed to parse JSON file")?;

    println!("Loaded: {:?}", args.file);

    // Setup audio device
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .context("No output device available")?;

    println!("Using audio device: {}", device.name().unwrap_or_default());

    // Create score and apply session state
    let mut score = Score::new();
    let mut rng = rand::thread_rng();
    score.apply_session_state(&state, synth_core::time_freq::Time(0.0), &mut rng);

    // Get sample rate
    let config = device
        .default_output_config()
        .context("Failed to get output config")?;
    let sample_rate = config.sample_rate().0 as f64;

    if args.precompute {
        if args.record.is_some() {
            bail!("Recording is not available in precompute mode");
        }
        return precompute_and_play(score, &device, sample_rate, args.duration.max(5));
    }

    // Setup audio stream

    let clock = Arc::new(AtomicU64::new(0));

    // Use Score's built-in shared_notes and create shared_delays (like GUI)
    let shared_delays = Arc::new(arc_swap::ArcSwap::from_pointee(
        score.root_delays_seconds(0.0, 0.0),
    ));

    let recorder = if let Some(path) = &args.record {
        let recorder = Arc::new(Recorder::new());
        recorder
            .start(path, sample_rate as u32)
            .with_context(|| format!("failed to start recording to {}", path.display()))?;
        Some(recorder)
    } else {
        None
    };

    let _stream = stream(
        F0,
        &device,
        clock.clone(),
        score.shared_notes.clone(), // Use Score's shared_notes
        (
            Reverb::<REVERB_BUFFER_LEN>::new(1.0, 1.0, sample_rate),
            Reverb::<REVERB_BUFFER_LEN>::new(1.0, 1.0, sample_rate),
        ),
        shared_delays.clone(),
        recorder.clone(),
    );

    println!("Playing... (Press Ctrl+C to stop)");

    score.advance(synth_core::time_freq::Time(0.0), &mut rng);
    score.publish_shared_notes();
    shared_delays.store(Arc::new(score.root_delays_seconds(0.0, 0.0)));

    // Spawn background thread to continuously regenerate notes
    let clock_clone = clock.clone();
    thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let update_interval = Duration::from_millis(100); // Update at 10 Hz
        let mut last_gen_time = synth_core::time_freq::Time(-1.0);

        loop {
            thread::sleep(update_interval);

            // Get current playback time
            let sample_count = clock_clone.load(std::sync::atomic::Ordering::Relaxed);
            let now = synth_core::time_freq::Time((sample_count as f64) / sample_rate);

            // Only regenerate if enough time has passed (at least 0.05 seconds)
            // This reduces unnecessary updates
            if (now - last_gen_time).as_secs() < 0.05 {
                continue;
            }

            score.advance(now, &mut rng);
            score.publish_shared_notes();
            shared_delays.store(Arc::new(score.root_delays_seconds(0.0, 0.0)));

            last_gen_time = now;
        }
    });

    // Play for specified duration or forever
    if args.duration > 0 {
        println!("Playing for {} seconds", args.duration);
        thread::sleep(Duration::from_secs(args.duration));
    } else {
        // Play forever until interrupted
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }

    if let Some(recorder) = recorder {
        recorder.stop().context("Failed to finalize recording")?;
    }

    Ok(())
}
