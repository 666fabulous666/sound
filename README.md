## License

This project is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE).

You may use, modify, and share this software for personal and non-commercial purposes.  
For commercial licensing, please contact me.

# Quantum Harmonics’ Oscillator

A probability-driven music sequencer.  
No AI — **you** control the rules, the rest is beautiful randomness.

- **Fine-tune** wave, envelope, bend, vibrato, chorus, power-factor…
- **Shape rhythm** with deterministic or random inclusion/exclusion generators.
- **Steer harmony** with tolerant following and randomized tempered intervals.
- **Jam forever**: sequences loop and regenerate on the fly.

> If you’re here for safe & familiar sounds, you may feel out of place.  
> If you’re ready to *hear the unheard*, welcome.

---

## Quick start

- **Default Example** — loads an embedded groove.
- **New Score** — starts blank.
- **Top panel** — Load / Save, plus app-level actions.
- **Select track** — click a block in the timeline.
- **Keyboard** — `↑`/`↓` to move selection between tracks.

---

## Tracks & Property Panel

Select a track to reveal its **Property Panel** on the left.

### Track actions
- `✖` — Delete the selected track  
- `Clone` — Duplicate the track  
- `Up` / `Down` — Reorder tracks

> Actions apply after the UI interaction ends (once the primary mouse button is up).

---

### Mix

- **Volume**: `0.0 … 32.0`
- **Stereo**: `0.0 … 1.0`  
  `0.5` is centered; lower/greater biases L/R.

---

### Wave

Choose the **waveform** (`Mute, Sine, Square, Triangle, Sawtooth, HiHat, Kick, Snare`).  
If you choose a **drum** wave (`HiHat, Kick, Snare`), the **Envelope** defaults to drum-friendly values.

---

### Sequence position

Constrain where in the loop notes may occur.

- **t_min / t_max** — start/end (seconds) within `loop_len`  
  Values are snapped to **time quantum** steps.
- **Time quantum** — `p/q` seconds (rational base step).

---

### Envelope

- **Attack** — `0.01 … 100.0` (log scale). *Double-click* to reset.
- **Decay** — `0.01 … 100.0` (log scale). *Double-click* to reset.

*(Drum waves use drum defaults.)*

---

### Bend

- **Magnitude** — `-200 … 200` (display scaled; internally ×1e-4)  
- **Speed** — `1.0 … 1000.0` (log scale)

*Double-click a control to restore its default.*

---

### Vibrato

- **Magnitude** — `0 … 1000` (display scaled; internally ×1e-6)
- **Frequency** — `1 … 100 Hz` (log scale)

*Double-click a control to restore its default.*

---

### Chorus (Unison Detune)

> Hidden for drum waves.

Adds detuned voices around `f₀` for width & motion.

- **Voice layers** — `1 … 10` (total voices = `1 + 2 × (layers − 1)`)
- **Detune Δf** — `0.0 … 1.0` (log scale)
- **Detune shift** — `-1.0 … 1.0` (asymmetry)
- **Detune over time** — `-5 … 5 Hz` (modulates Δf)
- **Weighting around f₀**  
  - **Even (sym)** — `-2 … 2`  
  - **Odd (asym)** — `-2 … 2`  
  |value| > 1 amplifies outer voices; < 1 attenuates; negative inverts phase.

*Double-click any control to reset its parameter to default.*

---

### Power factor

Introduces distortion/metallic timbre.

- **Initial value** — `0.0 … 1000.0` (log scale)
- **Evolution over time** — `-10 … 10 Hz` (internally squared with sign preserved)

Hints:
- `|value| = 0` → square-ish
- `|value| < 1` → distortion
- `= 1` → unchanged wave
- Very small/large values → metallic

*Double-click to reset.*

---

### Rhythm

#### Time quantum
Base time unit used for divisibility tests in rhythm rules.  
`p/q` where both are integers `1 … 128`.

#### Inclusions
Choose **how beats are allowed**:

- **Random** (`Rd`)  
  - `n` (amount): `0 … N`  
  - `N` (pool): up to `512`  
  Randomly pick `n` generators from `[1, N]`. A beat is included if its time unit is a multiple of any generator.
- **Deterministic** (`Det`)  
  - **Generators**: positive integers (> 1 recommended).  
  Any beat whose time unit is a multiple of one of these is included.

Toggle between **Random** and **Deterministic** with the provided buttons.

#### Exclusions
Choose **how beats are skipped** (applied with a +1 shift so the first beat is kept):

- **Random** (`Rd`)  
  - `n`, `N` like above, but from `[2, N+1]`.  
  A beat is excluded if `(time_unit + 1)` is a multiple of a generator.
- **Deterministic** (`Det`)  
  - **Generators**: integers > 1, applied to `(time_unit + 1)`.

#### Groove offset
- **Groove offset** — integer `0 … 256`  
Shifts the rhythmic grid used for inclusion/exclusion tests (expressed in **time quantum** units).

#### Looping
- **Loop length** — seconds (`0 … 512`)  
- **Repeat** — `1 … 64` times per generation cycle  
Each sequence loops independently.

---

### Harmony

> Hidden for drum waves.

#### Tolerance
- **Tolerance** — `<−4 … 16>, <−4 … 16>` (seconds)  
How far to look **before** and **after** a note when following harmonic context.

#### Interval — *RDTempered*
Randomized tempered steps:

- **Octave** — `−5 … 5` (base placement)
- **Variation steps** — `0 … 16` (max chained random steps)
- **Variation intervals** — checkbox set for semitone steps `−11 … 11`  
Each step shifts by one selected interval; steps can combine and wrap octaves.

---

### Accents

Accents scale note energy across the bar.

- **Magnitude** (base inverse) — `0.01 … 100.0` (log) → internally stored as its inverse.  
- **Generators** (list) — values displayed as inverses for intuitive control; editing updates the internal representation.

Use the `Generators` list UI (`+` to add, right-click a value to remove).

---

## Persistence

- **Load / Save** via top panel.  
- **WASM**: save triggers a browser download or uses the File System Access API (when supported).

---


## Building

```bash
# Native
cargo run --release

# WASM (with trunk)
cargo install trunk
trunk serve            # or trunk build --release
