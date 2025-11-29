## License

This project is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE).

You may use, modify, and share this software for personal and non-commercial purposes.  
For commercial licensing, please contact me.

# Quantum Harmonics’ Oscillator

A probability-driven music sequencer.  
No AI — **you** set the rules, the rest is beautiful randomness.

- **Fine-tune** oscillators, envelopes, bends, vibrato, chorus, power-factor, filters, noise.
- **Shape rhythm** with deterministic or random inclusion/exclusion generators.
- **Steer harmony** with tolerant following, melodic affinities, chords, and randomized tempered intervals.
- **Jam forever**: sequences loop, repeat, and regenerate on the fly.

> If you’re here for safe & familiar sounds, you may feel out of place.  
> If you’re ready to *hear the unheard*, welcome.

---

## Quick Start

- Start screen: **Examples** (loads an embedded groove), **New Score**, **Read Full README**, **Open GitHub**.
- Top toolbar: add tracks, load/save, play/pause, tempo popup, spectrogram toggle, record (desktop), README popup.
- Click a lane in the timeline to select a track; its properties appear in the left panel.
- Play/pause at any time with the **Space bar**.

---

## Global UI Map

### Top Toolbar
- **New Score** – clears the current project (asks to save first).
- **Add Track** – inserts a new sequence (auto-starts audio if needed).
- **Examples** – reloads the built-in groove.
- **Load… / Save…** – serialize/restore the full state (WASM triggers download/File System Access).
- **▶ / ⏸ (Play/Pause)** – also **Space bar**.
- **README** – show this document in-app.
- **⏺ / ⏹ (Record)** – desktop-only: record master output to WAV.
- **🕛 Reset elapsed time** – zeroes the transport clock.
- **💓 Tempo** – toggle the BPM popup (20–240 BPM, continuous slider).
- **📡 Spectrogram** – show/hide per-sequence previews (first open requests a render).
- **Delay matrices** – edit left/right recursive delay taps (ms) to sculpt stereo reverb.
- **❌ Exit** – desktop only, confirms before closing; also reachable with **Escape**.

### Timeline & Tree
- Left band shows the **track tree** (groups + sequences), with connectors and collapse bullets.
- Main area shows **loop windows**, repeats, and note envelopes in a “laser strip” visualization.
- Handles on each block let you **resize start/end** or **shift the window** (snaps to the sequence time quantum).
- A vertical playhead shows the current audio time; bar markers label repeats (e.g., `x3`, `2/4`).
- Drag-and-drop in the tree band reorders or reparents tracks; drop targets highlight valid slots.

### Mouse Interaction
- Click a lane to select it; click the bullet beside a group name to expand/collapse.
- Drag the left/right handles of a block to change `t_min` / `t_max`; drag the lower strip to offset the window.
- Drag a lane in the tree band to move or reparent; release to commit once a drop slot is highlighted.
- Right-click (or use the context menu) on most sliders to **reset to default**.

### Keyboard Shortcuts
- **Space** – Play/Pause.
- **ArrowUp / ArrowDown** – Select previous/next visible track.
- **ArrowLeft / ArrowRight** – Go to parent / first child.
- **Shift+ArrowUp / Shift+ArrowDown** – Move the selection into the previous/next sibling group.
- **Shift+ArrowLeft** – Dissolve the current group (if selected).
- **Cmd/Ctrl+ArrowUp / Cmd/Ctrl+ArrowDown** – Move the track up/down within its group.
- **Cmd/Ctrl+ArrowRight** – Wrap selection into a new group.
- **Cmd/Ctrl+ArrowLeft** – Promote the selection one level up in the tree.
- **M** – Mute/Unmute selection.
- **D** – Delete selection.
- **I** – Clone selection.
- **C** – Collapse/Expand (groups only).
- **+ / -** – Adjust volume (hold Shift for bigger steps).
- **Escape** – Exit confirmation (desktop).

---

## Property Panel (Left)

Select any track to reveal controls. Group nodes show child overrides; sequences show full synthesis parameters. All sliders support right-click reset. Changes that alter timing or structure regenerate notes after the pointer is released.

- **Name & Hue** – rename tracks and pick a hue for the lane/laser color.
- **Mix** – `Volume` (0–32, `+`/`-` shortcuts), `Pan` (0–1), `Proba` (chance to generate on the next cycle).
- **Wave & Phase** – choose oscillator (`Mute, Sine, Square, Triangle, Sawtooth, HiHat, Kick, Snare, Ride, Darbuka`) and note variant (Pure time or Phase tracked). Drum waves auto-load drum envelopes.
- **Envelope** – Attack/Decay (0.01–100, log scale; drum defaults for drums).
- **Filter** – Enable/disable; Lowpass/Highpass/Bandpass, order, cutoff base; relaxation (start/end/rate), LFO magnitude/frequency, sync to clock.
- **Bend** – Depth (-200..200 display, scaled internally ×1e-4) and speed (1–1000, log).
- **Vibrato** – Magnitude (0–1000, scaled ×1e-6) and frequency (0.01–100 Hz, log).
- **Harmonics** – Add harmonics/subharmonics with attenuation control.
- **Chorus (Unison detune)** – Hidden for drums. Voices (1–10), Δf, shift/asymmetry, time modulation, weighting around `f₀`.
- **Power factor** – Distortion/metallic timbre: base (0.01–10, log) plus relaxation and LFO modulation.
- **Noise** – Random attenuation amount with relaxation/LFO; optional clock sync.
- **Rhythm** – Time quantum (`p/q` beats), inclusions/exclusions (random `n` from `[1..N]` or deterministic generators), beat offset, loop length, tail multiplier (last window stretch), repeats.
- **Harmony (non-drum)** – Glide toggle; tolerance window; RDTempered interval (variation steps + allowed intervals + octave).  
  When **Harmonise** is on: chord size, skip-most-harmonious (tension), arpeggio offset, reverse/shuffle probabilities, random chord selection. Melody tools: affinities per interval, keep-direction affinity, replicator steps that look back N time quanta, and harmoniser weights for tension scoring.
- **Accents** – Base inverse magnitude (log) and generator list (values shown as inverses; add with `+`, remove with right-click).
- **Selection weight** – Used when a parent group is in OR mode.
- **Spectrogram preview** – When enabled globally, regenerating parameters schedules a fresh preview for the selected sequence.

### Groups & Overrides
- Each group can run in **AND** (play all children) or **OR** (pick one child weighted by “Selection weight”).
- Group actions: wrap selection into a group, promote, dissolve, move into neighbor groups, collapse/expand, mute.
- Group panels expose **child overrides** for envelope, filter, bend, vibrato, chorus, power, noise, harmonics, wave type, rhythm, and harmony. Enabling an override locks descendants until you clear it.
- Group volumes multiply down the tree; muting a group mutes all descendants.

---

## Persistence & I/O

- **Load / Save** from the toolbar (WASM triggers download / File System Access where supported).
- **Recording (desktop)** writes the master bus to WAV via a file dialog.
- **State resets**: “Reset elapsed time” zeroes the transport clock; “New Score” starts blank while keeping audio running.

---

## How Rhythm & Notes Are Generated

### Short Version
- A **time quantum** defines the grid; **inclusion** and **exclusion** generators carve windows inside `t_min..t_max`.
- Each window spawns a base note; tempo converts beats to seconds and applies **accents** for per-hit energy.
- Harmony picks tempered intervals using context (nearby notes within tolerance), chord tension weights, melody affinities, and optional arpeggio/reverse/shuffle tweaks.
- Notes inherit timbre (wave, envelope, filter, bend, vibrato, chorus, power, noise, harmonics), are repeated `repeat` times per loop, and glide to the next interval if enabled.

### Detailed Path
1) **Scheduling** – `Score::generate_notes` walks the track tree. Each sequence only regenerates after `not_generate_until` expires, so notes are drawn once per loop+repeat span.  
2) **Rhythm windows** – From `time_quantum` (`p/q` beats), compute step indices inside `t_min..min(t_max, loop_len)`.  
   - *Random mode*: sample `n` inclusion generators from `[1..N]`; a step is kept if `(idx - beat_offset) % gen == 0`.  
   - *Exclusions*: same idea but tested on `(idx + 1 - beat_offset)` so the first step stays.  
   - Remaining starts become windows; the final window is lengthened by `tail_multiplier`. Optional shuffle randomizes window order.  
3) **Tempo & accents** – Windows become Time using the global tempo. Each note’s energy is scaled by the envelope normalization and the accent formula (base inverse plus periodic generators). Loop repeats clone notes every `loop_len` beats.  
4) **Harmony resolution** – For RDTempered intervals, the engine gathers overlapping external notes (within `tolerance`) plus already-drawn notes in this sequence.  
   - *Harmonise on*: search chord combinations (size `chord`, optionally random subset) that minimize tension using `harmoniser` weights; skip the most consonant combos via `skip_harmonised`. Melody affinities bias keeping or flipping contour; **replicator** affinities bias repeating intervals seen `distance × time_quantum` earlier.  
   - *Harmonise off*: walk `variation_steps`, summing random intervals from the allowed set, wrapping octaves.  
   - Arpeggio offsets later notes by `arpegio × time_quantum`; `reverse_prob` and `shuffle_prob` optionally flip or shuffle the chosen degrees.  
5) **Glide & repeats** – If glide is enabled, each note stores the next interval as its glide target. `repeat` duplicates the pattern across the loop span; phase mode (Pure time vs Phase tracked) is stored per note.  
6) **Mixing & overrides** – Notes land in a `NotesGroup` keyed by sequence token. Timbre parameters are refreshed every generation; volumes are applied later by multiplying every ancestor group’s volume (and muted state) before rendering/recording.

---

## Building

```bash
# Native
cargo run --release

# WASM (with trunk)
cargo install trunk
trunk serve --release
```
