# Architecture

This workspace is a probability-driven music sequencer and synth. The core engine lives in `synth-core`, the GUI editor lives in `synth-gui`, and `synth-cli` is an older, unmaintained attempt at a CLI player.

## Workspace layout and crate dependencies

- `synth-core` is the engine crate. It defines the score tree, note generation, synthesis parameters, and audio mixing. It has no GUI dependencies and is reused by the GUI and CLI. Entry points are `synth-core/src/lib.rs` and `synth-core/src/engine/mod.rs`.
- `synth-gui` depends on `synth-core` and provides an egui/eframe editor plus a WASM build. Entry points are `synth-gui/src/lib.rs` and `synth-gui/src/app/mod.rs`.
- `synth-cli` depends on `synth-core` but is not maintained and currently references a non-existent module (`synth_core::engine::delay::Delay`). It should not be treated as a working entry point.

## Core data model (synth-core)

### Time and units

The project uses strong newtypes for time and frequency in `synth-core/src/time_freq/mod.rs`:

- `Time(f64)`, `Freq(f64)`, `Beat(f64)`, and `Tempo(f64)` are the canonical units for seconds, Hz, beats, and bpm.
- `Tempo` is used for converting between `Beat` and `Time` for scheduling and retiming.

These types are used throughout score generation (`synth-core/src/engine/score/*.rs`) and audio rendering (`synth-core/src/stream.rs`, `synth-core/src/engine/waves/mod.rs`).

### Identifiers

- `Token(usize)` in `synth-core/src/lib.rs` identifies a sequence node in the score tree. `TokenGen` generates new tokens.
- `NoteId(u64)` in `synth-core/src/lib.rs` uniquely identifies individual notes during playback. `NoteIdGen` generates them.

`Token` keys the `notes` map inside `Score`, while `NoteId` is used for note-local state in the audio callback (low-pass filter memory and phase tracking).

### Score tree and scheduling

The score is a tree of groups and sequences in `synth-core/src/engine/score/track_node.rs`:

- `TrackNode` stores UI-facing properties like `name`, `volume`, `pan`, `hue`, `proba`, `muted`, `solo`, `overrides`, and `delays` plus its `kind`.
- `Probability` in `synth-core/src/engine/score/probability.rs` wraps a 0..1 chance and is used for `TrackNode::proba` and mix propagation.
- `NodeKind` is either `Group` (tree node) or `Seq(Sequence)`.
- `GroupMode` selects `And` (play all children) or `Or` (pick a child by `or_weight`).
- `AestheticLocks` controls whether child nodes inherit volume, pan, or hue.
- `MixContext` propagates mix state (volume, pan, probability, solo flags) while walking the tree.

Scheduling state lives in `synth-core/src/engine/score/scheduler.rs`:

- `PlaybackScheduler` keeps per-sequence `SequencePlaybackState` with cached notes and a `busy_until` timestamp.
- Caching is used for freezing and for avoiding regeneration until the next loop window.

At the top, `Score` in `synth-core/src/engine/score/mod.rs` owns:

- `track_root: TrackNode` as the tree root.
- `notes: BTreeMap<Token, NotesGroup>` as the current audio-ready note buffers per sequence.
- `tempo: Tempo`, `delays: TrackDelays`, `shared_notes: Arc<ArcSwap<...>>` for audio sharing.
- `note_id_gen` and `scheduler` for runtime state.

`Score::generate_notes` walks the tree by calling `TrackNode::draw_node`, which in turn calls `Sequence::draw` to emit notes. `Score::refresh_notes_for_path` re-applies parameter overrides to existing `NotesGroup` entries when only timbre changes.

### Sequence content and note generation

A `Sequence` describes musical content and lives in `synth-core/src/engine/score/sequence.rs`:

- Rhythm parameters: `t_min`, `t_max`, `time_quantum`, `loop_len`, `loop_offset`, `repeat`, `beat_offset`, `tail_multiplier`, `inclusions`, and `exclusions`.
- Harmony parameters: `interval`, `harmoniser`, `melodiser`, `replicator`, `replicator_steps`, `tolerance`, `glide`, `arpegio`, `chord`, `random_chord`, `reverse_prob`, `shuffle_prob`, `melody_order_affinity`.
- Timbre parameters: `wave_type`, `note_variant`, and `harmonics`.

Rhythm utilities are in `synth-core/src/engine/score/time_quantum.rs` (`TimeQuantum`) and the `Rythm` types (`RdRythm`, `DetRythm`) in `synth-core/src/engine/score/mod.rs`.

A `Note` in `synth-core/src/engine/score/note.rs` is the concrete unit of playback:

- `time`, `duration`, and beat equivalents are stored alongside `interval`, optional `glide`, and `NoteVariant` (`PureTime` vs `PhaseTracked`).
- `Note::draw` expands a base note into harmonized/melodic notes, using context from other `NotesGroup` entries.

### Track parameters and overrides

Synthesis parameters live in `synth-core/src/engine/score/node_params.rs`:

- Base parameter structs include `BendParams`, `VibratoParams`, `EnvelopeParams`, `LowpassParams`, `PowerParams`, `NoiseParams`, `WaveParams`, `HarmonyParams`, and `RhythmParams`.
- `NodeOverrides` holds optional versions of those fields and is attached to every `TrackNode`. Overrides are used to clamp child parameters in groups.
- `ResolvedTrackParams` is the fully resolved parameter set for a given path in the tree. It is computed by walking from root to leaf, keeping the first override encountered at each depth.

`ResolvedTrackParams` are applied in `apply_params_to_notes_group` in `synth-core/src/engine/score/mod.rs` to keep audio buffers in sync with UI edits.

### NotesGroup and delay routing

`NotesGroup` in `synth-core/src/engine/score/mod.rs` is the audio-ready bundle derived from a `Sequence` and its resolved parameters:

- Holds `notes: Vec<Note>`, `wave_type`, `harmonics`, `attack_decay`, `cutoff`, `lowpass` settings, `bend`, `vibrato`, `chorus`, `power`, `noise`, `volume`, and `pan`.
- Includes per-track `delays` in seconds (`Vec<DelayTapSeconds>` for left/right).

Delay configuration is beat-based in `TrackDelays` and `DelayChannel` in `synth-core/src/engine/score/mod.rs`, then converted to seconds based on `Tempo` using `TrackDelays::to_seconds`.

### Wave generation and modulation

Sound generation lives in `synth-core/src/engine/waves/mod.rs`:

- `WaveType` selects the oscillator or drum model.
- `FilterType` selects lowpass/highpass/bandpass.
- `generate_wave` and `generate_wave_with_phase` compute a single sample given a `Note` and its parameters.

Time-dependent modulation uses `TimeVarying` from `synth-core/src/engine/time_varying.rs`, which combines `Relaxation` and `Lfo` components. Lowpass, power, and noise are all modeled as `TimeVarying` values.

Reverb lives in `synth-core/src/engine/reverb.rs` and is applied in the audio stream after per-track mixing.

### Real-time audio pipeline

The audio callback is built in `synth-core/src/stream.rs`:

- The GUI (or CLI) shares the current `BTreeMap<Token, NotesGroup>` via `Arc<ArcSwap<...>>`.
- For each frame, the callback iterates all `NotesGroup` entries and all active notes, produces dry samples, pans them, and applies per-track reverb and delays.
- A global reverb is applied after mixing, and the result is optionally recorded through `Recorder` in `synth-core/src/recorder.rs`.

The render loop depends on `NoteVariant` to choose phase-tracked or pure-time generation.

### Presets

Instrument presets are defined in `synth-core/src/engine/score/preset.rs`:

- `InstrumentPreset` stores only aesthetic parameters (wave, envelope, effects, etc.) and excludes rhythm/harmony.
- `PresetMetadata` carries name and optional tags.

In the GUI, presets are saved/loaded in `synth-gui/src/app/preset_io.rs` and applied to the currently selected node by merging into `NodeOverrides` and then refreshing the affected `NotesGroup` entries.

## GUI data flow (synth-gui)

The GUI owns the engine state in `GuiApp` in `synth-gui/src/app/mod.rs`:

- `score: Score` is the authoritative model for sequences and notes.
- `shared_delays` and `shared_notes` are shared with the audio callback via `ArcSwap`.
- Selection is tracked as a `Vec<usize>` path into the tree (`TrackNode` indices).

Serialization is handled by `GuiState` in `synth-gui/src/app/mod.rs`:

- `seqs: TrackNode` is the serialized tree.
- `delays: TrackDelays` and `tempo_bpm` capture the global mix state.

`load_state` and `save_state` in `synth-gui/src/app/load.rs` and `synth-gui/src/app/save.rs` read/write JSON. A compatibility layer (`deserialize_sequences_compat`) accepts older formats (vector of nodes or sequences) and rebuilds a root group.

## CLI status

`synth-cli` in `synth-cli/src/main.rs` mirrors the GUI JSON format but is unmaintained. It contains outdated imports (`synth_core::engine::delay::Delay`) and should be considered non-functional without fixes.

## Data flow summary

1. The GUI edits a `TrackNode` tree and parameter overrides.
2. `Score::generate_notes` walks the tree, resolves parameters, and produces `NotesGroup` entries keyed by `Token`.
3. `shared_notes` is updated, and the audio callback mixes `NotesGroup` entries into stereo samples with per-track delay/reverb.
4. Optional recording writes the final stereo mix to WAV via `Recorder`.

## Key file map

- Core types and constants: `synth-core/src/lib.rs`
- Units: `synth-core/src/time_freq/mod.rs`
- Score tree and params: `synth-core/src/engine/score/mod.rs`, `synth-core/src/engine/score/track_node.rs`, `synth-core/src/engine/score/node_params.rs`, `synth-core/src/engine/score/sequence.rs`, `synth-core/src/engine/score/note.rs`
- Runtime scheduling: `synth-core/src/engine/score/scheduler.rs`
- Synthesis and effects: `synth-core/src/engine/waves/mod.rs`, `synth-core/src/engine/time_varying.rs`, `synth-core/src/engine/reverb.rs`
- Audio stream and recording: `synth-core/src/stream.rs`, `synth-core/src/recorder.rs`
- GUI app and state IO: `synth-gui/src/app/mod.rs`, `synth-gui/src/app/load.rs`, `synth-gui/src/app/save.rs`, `synth-gui/src/app/preset_io.rs`
