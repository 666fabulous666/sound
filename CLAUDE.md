# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Quantum Harmonics' Oscillator** is a probability-driven music sequencer built in Rust. It's a real-time audio application with both native (desktop) and WASM (web) targets, featuring a custom GUI built with egui/eframe.

Key concept: The application uses **randomness within constraints** (not AI) to generate music. Users control probabilities, rhythmic rules, harmonic intervals, and synthesis parameters—the engine handles the rest through elegant randomization.

## Build & Run Commands

### Native (Desktop)
```bash
# Build and run in release mode (recommended for audio performance)
cargo run --release

# Build only
cargo build --release

# Development build (slower audio but faster compile)
cargo run
```

### WASM (Web)
```bash
# Install trunk if not already installed
cargo install trunk

# Serve with hot-reload (development)
trunk serve --release

# Build for deployment
trunk build --release
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test <test_name>
```

## Architecture Overview

### Core Components

1. **`src/app/mod.rs` - GuiApp**: The main application state and UI coordinator
   - Manages the `Score` (track tree + generated notes)
   - Handles UI panels (top, property, timeline)
   - Bridges between UI interactions and audio engine
   - Contains save/load logic for project files

2. **`src/engine/score/` - Music Generation Engine**
   - `mod.rs`: `Score` struct - central data structure holding the track tree and generated notes
   - `track_node.rs`: `TrackNode` enum - hierarchical tree structure (Groups and Sequences)
     - Groups: containers with volume/pan/mute, can nest infinitely
     - Sequences: individual tracks with rhythm rules, harmony rules, synthesis parameters
   - `sequence.rs`: `Sequence` struct - the core track type with all musical parameters
   - `note.rs`: `Note` struct - individual note events (time, duration, pitch interval, volume)
   - `default_params.rs`: Default values for all parameters

3. **`src/stream.rs` - Real-time Audio Callback**
   - Uses `cpal` for cross-platform audio I/O
   - Runs on audio thread, reads from `Arc<ArcSwap<Vec<NotesGroup>>>`
   - Generates samples by iterating active notes and calling `generate_wave`
   - Applies reverb via recursive delay lines (`Reverb` struct)

4. **`src/engine/waves/` - Synthesis**
   - `mod.rs`: `generate_wave()` - the main synthesis function
     - Handles waveform generation (sine, square, triangle, sawtooth)
     - Applies envelope (attack/decay), bend, vibrato, chorus (unison detune), power-factor distortion
     - Low-pass filter with envelope
   - `drums.rs`: Specialized drum synthesis (kick, snare, hi-hat, ride, darbuka)

5. **`src/app/property_panel.rs` - Parameter UI**
   - Left-side panel showing parameters for the selected track
   - Edits `Sequence` fields or `TrackNode::Group` fields
   - Regenerates notes when parameters change via `app.edit_node_at()`

6. **`src/app/timeline_panel.rs` - Track Timeline UI**
   - Visual representation of tracks and their notes over time
   - Tree view with expand/collapse for Groups
   - Selection, navigation (H/J/K/L keys), drag-to-reorder (planned)

### Key Architectural Patterns

**Hierarchical Track Tree (`TrackNode`)**:
- Root is always a `TrackNode::Group` (enforced by `GuiApp`)
- Groups can contain Sequences and other Groups (unlimited nesting)
- Navigation uses path-based indexing: `Vec<usize>` (e.g., `[0, 2, 1]` = first child → third child → second child)
- Key operations: `get()`, `get_mut()`, `remove_at()`, `insert_at()` (see `track_node.rs`)

**Generative Notes System**:
- `Sequence::draw()` generates `Note` events based on:
  - **Rhythm rules**: inclusion/exclusion generators (deterministic or random) applied to time quantums
  - **Harmony rules**: interval selection (tempered steps + random variations), optional following of previous notes within tolerance
  - **Repetition**: each sequence loops `repeat` times within its `loop_len`
- Generated notes are stored in `Score.notes` as `Vec<NotesGroup>`
- Notes are regenerated when:
  - Time advances past `not_generate_until` threshold
  - User modifies sequence parameters
  - Notes are retained only if `time + NOTE_LINGER_TIME >= now`

**Thread-Safe Audio Bridge**:
- Main thread: GUI updates `Score.notes`, then stores to `Arc<ArcSwap<Vec<NotesGroup>>>`
- Audio thread: Loads latest notes via `note_queue.load()`, no locks during sample generation
- Same pattern for reverb delays: `Arc<ArcSwap<(Vec<f64>, Vec<f64>)>>`

**Time Representation**:
- `Time(f64)` - seconds (newtype wrapper in `src/time_freq/mod.rs`)
- `Freq(f64)` - Hertz
- Clock: `Arc<AtomicU64>` counting samples, converted to `Time` via sample rate

### Platform Differences (Native vs WASM)

- Native: `cpal` with default host, `pollster` for async, maximized window
- WASM: `cpal` with `wasm-bindgen`, manual canvas creation, file loading via `rfd::AsyncFileDialog` or File System Access API
- Entry points:
  - Native: `src/main.rs` (calls `eframe::run_native`)
  - WASM: `src/lib.rs::start()` (WASM bindgen entry, calls `eframe::WebRunner`)

### Persistence (Save/Load)

- Format: JSON via `serde_json` (`GuiState` struct)
- Saved data: track tree (`TrackNode` hierarchy), reverb delays
- Backward compatibility: `deserialize_sequences_compat` handles old formats (flat `Vec<Sequence>` or `Vec<TrackNode>`)
- Load process:
  1. Deserialize `GuiState`
  2. Call `app.replace_root_with()` to swap in new tree
  3. Regenerate all notes via recursive `draw_node()`

## Common Patterns & Gotchas

### When Editing Sequences
- Always call `app.edit_node_at(new_node, path)` instead of direct mutation
  - This ensures notes are regenerated for the edited sequence **and all following sequences in preorder**
  - Important for harmony following (later sequences may follow earlier ones)

### Token System
- Every `Sequence` has a unique `Token` (from `TokenGen`)
- Tokens identify which `NotesGroup` belongs to which sequence
- When cloning a sequence, must assign a fresh token via `score.last_token.next()`

### Volume Chain
- Each `TrackNode` (Group or Seq) has its own volume
- Final volume = product of all volumes from root to leaf
- Use `Score::volume_chain_product(path)` to compute this

### Navigation Keys (default shortcuts)
- `J`/`K`: Select down/up (visible tracks only)
- `H`/`L`: Parent/first child
- `Ctrl+J`/`Ctrl+K`: Move track within group (reorder)
- `Shift+J`/`Shift+K`: Move into adjacent group
- `Shift+L`: Wrap in new group
- `Ctrl+L`: Promote (move up one tree level)
- `Shift+H`: Dissolve group (flatten into parent)

### Rhythm System
- **Time quantum**: `(p, q)` means base unit = `p/q` seconds
- **Inclusion generators**: A beat at time `t` is included if `floor(t / quantum)` is divisible by any generator
- **Exclusion generators**: Same, but applied to `(t + 1)` to keep the first beat
- **Random mode**: Randomly pick `n` generators from `[1, N]` (or `[2, N+1]` for exclusions)
- **Groove offset**: Shifts the grid by `offset * quantum` seconds

### Harmony System (Non-Drum Waves)

The harmony system provides two complementary approaches for generating melodic/harmonic content:

**1. Basic Interval Selection (Random Walk)**:
- **Interval**: Base octave + random walk over selected semitone steps
- **Tolerance**: `(before, after)` in seconds - window to search for notes to follow
- **Following logic**: If previous notes exist within tolerance, pick a random note and perform a random walk using allowed interval steps; else use base interval
- This approach creates the foundational **chord chart** or harmonic structure

**2. Harmoniser (Tension-Based Selection)**:
- **Purpose**: Build on top of the established chord chart by selecting notes that minimize harmonic tension
- **Mechanism**:
  - Computes tension between candidate note(s) and existing notes within tolerance window
  - Selects the candidate(s) with minimum tension
  - Field: `harmoniser: [[u32; 7]; 2]` in `Sequence` struct
- **Typical workflow**: Use basic interval selection on initial tracks to establish harmonic foundation, then enable harmoniser on subsequent tracks to create consonant relationships

The harmoniser enables creating more coherent harmonic progressions by favoring notes that blend well with the existing musical context, rather than purely random selection.

## Important Constants

- `F0` (440 Hz): Base frequency for interval calculations
- `NOTE_LINGER_TIME` (12s): How long to retain generated notes in memory
- `GENERATE_EARLY` (1s): How far ahead to generate notes
- `REVERB_BUFFER_LEN` (44100): Reverb delay line buffer size
- `DEFAULT_LOOP_LEN` (4s): Default sequence loop length

## File Locations

- Entry points: `src/main.rs` (native), `src/lib.rs::start()` (WASM)
- Core state: `src/app/mod.rs::GuiApp`
- Audio engine: `src/engine/score/` (data) + `src/stream.rs` (rendering)
- UI panels: `src/app/top_panel.rs`, `property_panel.rs`, `timeline_panel.rs`, `start_page.rs`
- Synthesis: `src/engine/waves/mod.rs` + `drums.rs`
- Default grooves: Embedded in `src/lib.rs::GROOVE_DEFAULTS` from `assets/*.json`

## License

PolyForm Noncommercial License 1.0.0 - non-commercial use only. See LICENSE and README.md for details.
