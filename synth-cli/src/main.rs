use anyhow::{Context, Result};
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait};
use std::path::PathBuf;
use std::sync::{atomic::AtomicU64, Arc};
use std::thread;
use std::time::Duration;
use synth_core::{
    engine::score::Score,
    session::SessionState,
    stream::stream,
    F0,
};

/// Command-line JSON player for Quantum Harmonics' Oscillator
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// JSON file to play
    #[arg(value_name = "FILE")]
    file: PathBuf,

    // Minimal example: default output device only.
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

    // Setup audio stream

    let clock = Arc::new(AtomicU64::new(0));

    let _stream = stream(
        F0,
        &device,
        clock.clone(),
        score.shared_notes.clone(), // Use Score's shared_notes
        None,
    );

    println!("Playing... (Press Ctrl+C to stop)");

    score.advance(synth_core::time_freq::Time(0.0), &mut rng);
    score.publish_shared_notes();

    // Spawn background thread to continuously regenerate notes
    let clock_clone = clock.clone();
    thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let update_interval = Duration::from_millis(50);

        loop {
            thread::sleep(update_interval);

            // Get current playback time
            let sample_count = clock_clone.load(std::sync::atomic::Ordering::Relaxed);
            let now = synth_core::time_freq::Time((sample_count as f64) / sample_rate);

            score.advance(now, &mut rng);
            score.publish_shared_notes();
        }
    });

    thread::park();

    Ok(())
}
