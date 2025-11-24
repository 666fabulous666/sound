//! Instrument preset system for saving and loading aesthetic parameters.
//!
//! This module provides functionality to save and load "instrument presets" -
//! collections of aesthetic/synthesis parameters (wave type, envelope, effects, etc.)
//! that define the timbre and character of a sound, independent of musical content
//! (rhythm and harmony).

use serde::{Deserialize, Serialize};

use super::{
    node_params::{
        BendParams, EnvelopeParams, LowpassParams, NodeOverrides,
        PowerParams, VibratoParams, WaveParams,
    },
    ChorusParams, HarmonicsParams,
};

/// Metadata for an instrument preset
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct PresetMetadata {
    /// Name of the preset (e.g., "Warm Pad", "Plucky Bass")
    pub name: String,

    /// Optional description of the sound or use case
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,

    /// Optional author/creator name
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub author: Option<String>,

    /// Optional tags for categorization (e.g., ["bass", "synth", "aggressive"])
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub tags: Option<Vec<String>>,

    /// Version of the preset format (for future compatibility)
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    1
}

impl Default for PresetMetadata {
    fn default() -> Self {
        Self {
            name: "Untitled Preset".to_string(),
            description: None,
            author: None,
            tags: None,
            version: 1,
        }
    }
}

/// Instrument preset containing aesthetic/synthesis parameters
///
/// This struct stores only the parameters that affect the timbre and character
/// of the sound (wave type, envelope, effects), excluding musical parameters
/// (rhythm, harmony, intervals) which are specific to the composition.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct InstrumentPreset {
    /// Metadata about the preset
    pub metadata: PresetMetadata,

    /// Pitch bend parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub bend: Option<BendParams>,

    /// Vibrato parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub vibrato: Option<VibratoParams>,

    /// Chorus/unison detuning parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub chorus: Option<ChorusParams>,

    /// Amplitude envelope (attack/decay)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub envelope: Option<EnvelopeParams>,

    /// Lowpass filter parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub lowpass: Option<LowpassParams>,

    /// Power factor distortion
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub power: Option<PowerParams>,

    /// Harmonics and subharmonics
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub harmonics: Option<HarmonicsParams>,

    /// Waveform type
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub wave: Option<WaveParams>,
}

impl InstrumentPreset {
    /// Create a new preset with the given name and no parameters
    pub fn new(name: String) -> Self {
        Self {
            metadata: PresetMetadata {
                name,
                ..Default::default()
            },
            bend: None,
            vibrato: None,
            chorus: None,
            envelope: None,
            lowpass: None,
            power: None,
            harmonics: None,
            wave: None,
        }
    }

    /// Extract aesthetic parameters from NodeOverrides
    ///
    /// Creates a preset from a node's overrides, copying only the aesthetic
    /// parameters (excluding harmony and rhythm which are musical/compositional).
    pub fn from_overrides(name: String, overrides: &NodeOverrides) -> Self {
        Self {
            metadata: PresetMetadata {
                name,
                ..Default::default()
            },
            bend: overrides.bend.clone(),
            vibrato: overrides.vibrato.clone(),
            chorus: overrides.chorus.clone(),
            envelope: overrides.envelope.clone(),
            lowpass: overrides.lowpass.clone(),
            power: overrides.power.clone(),
            harmonics: overrides.harmonics.clone(),
            wave: overrides.wave.clone(),
        }
    }

    /// Apply preset parameters to NodeOverrides
    ///
    /// Copies the preset's parameters into the target overrides, preserving
    /// any existing harmony/rhythm parameters. Only non-None parameters from
    /// the preset are applied.
    pub fn apply_to_overrides(&self, overrides: &mut NodeOverrides) {
        if let Some(ref bend) = self.bend {
            overrides.bend = Some(bend.clone());
        }
        if let Some(ref vibrato) = self.vibrato {
            overrides.vibrato = Some(vibrato.clone());
        }
        if let Some(ref chorus) = self.chorus {
            overrides.chorus = Some(chorus.clone());
        }
        if let Some(ref envelope) = self.envelope {
            overrides.envelope = Some(envelope.clone());
        }
        if let Some(ref lowpass) = self.lowpass {
            overrides.lowpass = Some(lowpass.clone());
        }
        if let Some(ref power) = self.power {
            overrides.power = Some(power.clone());
        }
        if let Some(ref harmonics) = self.harmonics {
            overrides.harmonics = Some(harmonics.clone());
        }
        if let Some(ref wave) = self.wave {
            overrides.wave = Some(wave.clone());
        }
    }

    /// Serialize preset to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize preset from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for InstrumentPreset {
    fn default() -> Self {
        Self::new("Default Preset".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::waves::WaveType;

    #[test]
    fn test_preset_roundtrip_empty() {
        let preset = InstrumentPreset::new("Test".to_string());
        let json = preset.to_json().unwrap();
        let loaded = InstrumentPreset::from_json(&json).unwrap();
        assert_eq!(preset, loaded);
    }

    #[test]
    fn test_preset_roundtrip_with_params() {
        let mut preset = InstrumentPreset::new("Test".to_string());
        preset.wave = Some(WaveParams { wave: WaveType::Square });
        preset.bend = Some(BendParams { magnitude: 0.5, speed: 2.0 });

        let json = preset.to_json().unwrap();
        let loaded = InstrumentPreset::from_json(&json).unwrap();
        assert_eq!(preset, loaded);
    }

    #[test]
    fn test_extract_from_overrides() {
        let mut overrides = NodeOverrides::default();
        overrides.wave = Some(WaveParams { wave: WaveType::Sine });
        overrides.bend = Some(BendParams { magnitude: 0.3, speed: 1.5 });

        let preset = InstrumentPreset::from_overrides("Test".to_string(), &overrides);
        assert_eq!(preset.wave, overrides.wave);
        assert_eq!(preset.bend, overrides.bend);
    }

    #[test]
    fn test_apply_to_overrides() {
        let mut preset = InstrumentPreset::new("Test".to_string());
        preset.wave = Some(WaveParams { wave: WaveType::Triangle });
        preset.vibrato = Some(VibratoParams {
            magnitude: 0.1,
            frequency: crate::time_freq::Freq(5.0),
        });

        let mut overrides = NodeOverrides::default();
        preset.apply_to_overrides(&mut overrides);

        assert_eq!(overrides.wave, preset.wave);
        assert_eq!(overrides.vibrato, preset.vibrato);
    }

    #[test]
    fn test_apply_preserves_harmony_rhythm() {
        let mut preset = InstrumentPreset::new("Test".to_string());
        preset.wave = Some(WaveParams { wave: WaveType::Sawtooth });

        let mut overrides = NodeOverrides::default();
        // Set some harmony/rhythm params
        overrides.harmony = Some(super::super::node_params::HarmonyParams::default());
        overrides.rhythm = Some(super::super::node_params::RhythmParams::default());

        let harmony_backup = overrides.harmony.clone();
        let rhythm_backup = overrides.rhythm.clone();

        preset.apply_to_overrides(&mut overrides);

        // Wave should be applied
        assert_eq!(overrides.wave, preset.wave);
        // Harmony and rhythm should be preserved
        assert_eq!(overrides.harmony, harmony_backup);
        assert_eq!(overrides.rhythm, rhythm_backup);
    }
}
