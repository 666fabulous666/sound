# Synth Architecture Plan

## Overview
Splitting the monolithic synth application into a clean architecture with three components:
- **synth-core**: Pure audio engine library
- **synth-cli**: Command-line player
- **synth-gui**: GUI editor (current functionality)

## Current Issues
1. ❌ No separation between engine and GUI
2. ❌ Heavy use of `.unwrap()` and `.expect()` - crashes on errors
3. ❌ No error logging/display for users
4. ❌ No unit tests
5. ❌ Limited documentation

## Proposed Structure

### synth-core (Library Crate)
**Purpose**: Pure audio synthesis engine with no GUI dependencies

**Contents**:
- `engine/` - Audio generation
  - `score/` - Score data structures (Score, Sequence, TrackNode, Note)
  - `waves/` - Waveform generation
  - `reverb.rs` - Reverb effect
- `time_freq/` - Time and Frequency types
- `error.rs` - Error types
- Core utilities (Token, rescale_factor)

**Public API**:
```rust
pub fn load_score(json: &str) -> Result<Score, SynthError>;
pub fn generate_audio(score: &Score) -> Result<AudioBuffer, SynthError>;
```

**Dependencies**: serde, serde_json, rand, itertools, hound

### synth-cli (Binary Crate)
**Purpose**: Command-line JSON player

**Features**:
- Load JSON file
- Play audio
- Optional: export to WAV

**Usage**: `synth-cli play song.json`

**Dependencies**: synth-core, cpal, clap

### synth-gui (Binary Crate)
**Purpose**: Interactive GUI editor (current main functionality)

**Contents**:
- `app/` - All GUI code
- `stream.rs` - Audio streaming
- `shortcuts.rs` - Keyboard shortcuts
- `texts/` - Help text

**Dependencies**: synth-core, egui, eframe, cpal, rfd

## Error Handling Strategy

### Error Types
```rust
pub enum SynthError {
    Json(serde_json::Error),
    InvalidParameter { param: String, value: f64 },
    AudioDevice(String),
    IoError(std::io::Error),
}
```

### Usage Pattern
Replace:
```rust
let value = some_operation().unwrap();
```

With:
```rust
let value = some_operation()
    .map_err(|e| SynthError::from(e))?;
```

## Testing Strategy

### Unit Tests
1. **Math functions**:
   - `rescale_factor` - no NaN, no infinity
   - Volume chain products - no NaN

2. **Parameter validation**:
   - Volume: [0.0, ∞) - no NaN
   - Pan: [0.0, 1.0] - clamped
   - Frequencies: positive - no NaN

3. **Serialization**:
   - Load/save Score roundtrip
   - Backward compatibility

4. **Audio generation**:
   - No NaN in output
   - No silence when volume > 0

### Integration Tests
- Load example JSON files
- Generate audio without panics
- Verify output is finite

## Documentation Plan

### Module-level docs
- Explain purpose of each module
- Show usage examples

### Function docs
- All public functions
- Parameter constraints
- Error conditions

## Implementation Order

1. ✅ Create ARCHITECTURE.md (this file)
2. Create error types
3. Create workspace Cargo.toml
4. Extract synth-core
5. Create synth-cli
6. Refactor synth-gui
7. Add Result-based error handling
8. Add log panel to GUI
9. Write unit tests
10. Add documentation

## File Migration

### To synth-core/src/
- engine/ (entire directory)
- time_freq/ (entire directory)
- lib.rs → lib.rs (partial - just core exports)
- error.rs (new)

### To synth-cli/src/
- main.rs (new CLI implementation)

### To synth-gui/src/
- app/ (entire directory)
- stream.rs
- shortcuts.rs
- texts/
- range_slider.rs
- main.rs (current main.rs)

## Benefits

1. ✅ **Testable core** - Unit test without GUI
2. ✅ **Reusable engine** - Use in other projects
3. ✅ **Better errors** - User-facing error messages
4. ✅ **Safer code** - No unwrap() panics
5. ✅ **Clear separation** - Engine vs presentation
