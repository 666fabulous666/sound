use anyhow::{Context, Result};
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait};
use std::path::PathBuf;
use std::sync::{atomic::AtomicU64, Arc};
use std::thread;
use std::time::Duration;
use synth_core::{
    engine::{
        reverb::Reverb,
        score::{track_node::TrackNode, Score},
    },
    stream::stream,
    Token, TokenGen, F0, REVERB_BUFFER_LEN,
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
}

#[derive(serde::Deserialize)]
struct ScoreState {
    seqs: TrackNode,
    #[serde(default)]
    delays: (Vec<f64>, Vec<f64>),
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    // Load JSON file
    let json_content = std::fs::read_to_string(&args.file)
        .with_context(|| format!("Failed to read file: {:?}", args.file))?;

    let state: ScoreState = serde_json::from_str(&json_content)
        .with_context(|| "Failed to parse JSON file")?;

    println!("Loaded: {:?}", args.file);

    // Setup audio device
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .context("No output device available")?;

    println!("Using audio device: {}", device.name().unwrap_or_default());

    // Create score
    let mut score = Score::new();
    score.track_root = state.seqs;
    score.delays = state.delays;

    // Initialize token generator based on existing tokens
    score.last_token = TokenGen(
        score
            .track_root
            .sequences()
            .map(|s| s.token)
            .max()
            .unwrap_or(Token(0))
            .saturating_add(1),
    );

    // Generate initial notes
    let mut rng = rand::thread_rng();
    let now = synth_core::time_freq::Time(0.0);
    score.generate_notes(now, &mut rng);

    // Setup audio stream
    let config = device
        .default_output_config()
        .context("Failed to get output config")?;
    let sample_rate = config.sample_rate().0 as f64;

    let clock = Arc::new(AtomicU64::new(0));
    let note_queue = Arc::new(arc_swap::ArcSwap::from_pointee(score.notes.clone()));
    let reverb_left = Reverb::<REVERB_BUFFER_LEN>::new(0.5, 0.5, sample_rate);
    let reverb_right = Reverb::<REVERB_BUFFER_LEN>::new(0.5, 0.5, sample_rate);
    let delays_arc = Arc::new(arc_swap::ArcSwap::from_pointee(score.delays.clone()));

    let _stream = stream(
        F0,
        &device,
        clock.clone(),
        note_queue.clone(),
        (reverb_left, reverb_right),
        delays_arc.clone(),
    );

    println!("Playing... (Press Ctrl+C to stop)");

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

    Ok(())
}
