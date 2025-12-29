use serde::{Deserialize, Serialize};

use crate::engine::score::{
    default_params::{default_delays, default_tempo},
    node_params::NodeOverrides,
    probability::Probability,
    track_node::{AestheticLocks, GroupMode, NodeKind, TrackNode},
    TrackDelays,
};
use crate::Token;

use serde::Deserializer;

#[derive(Deserialize)]
#[serde(untagged)]
enum SequencesCompat {
    Root(TrackNode),       // new format: a single root node
    Nodes(Vec<TrackNode>), // old format: Vec<TrackNode>
    Seqs(Vec<crate::engine::score::sequence::Sequence>), // very old format
}

fn deserialize_sequences_compat<'de, D>(de: D) -> Result<TrackNode, D::Error>
where
    D: Deserializer<'de>,
{
    let compat = SequencesCompat::deserialize(de)?;
    let mut root = match compat {
        SequencesCompat::Root(root) => root,
        SequencesCompat::Nodes(children) => TrackNode {
            name: "Root".into(),
            proba: Probability::default(),
            volume: 1.0,
            muted: false,
            solo: false,
            pan: 0.5,
            hue: 0.0,
            or_weight: 1.0,
            overrides: NodeOverrides::default(),
            delays: TrackDelays::default(),
            kind: NodeKind::Group {
                id: Token(0),
                collapsed: false,
                children,
                not_generate_until: None,
                mode: GroupMode::And,
                aesthetic: AestheticLocks::default(),
            },
        },
        SequencesCompat::Seqs(seqs) => TrackNode {
            name: "Root".into(),
            proba: Probability::default(),
            volume: 1.0,
            muted: false,
            solo: false,
            pan: 0.5,
            hue: 0.0,
            or_weight: 1.0,
            overrides: NodeOverrides::default(),
            delays: TrackDelays::default(),
            kind: NodeKind::Group {
                id: Token(0),
                collapsed: false,
                children: seqs.into_iter().map(TrackNode::from_sequence).collect(),
                not_generate_until: None,
                mode: GroupMode::And,
                aesthetic: AestheticLocks::default(),
            },
        },
    };
    root.migrate_legacy_overrides();
    Ok(root)
}

fn default_tempo_bpm() -> f64 {
    default_tempo().beats_per_minute()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionState {
    #[serde(deserialize_with = "deserialize_sequences_compat")]
    pub seqs: TrackNode,
    #[serde(default = "default_delays", rename = "delays_beats")]
    pub delays: TrackDelays,
    #[serde(default = "default_tempo_bpm")]
    pub tempo_bpm: f64,
}

impl SessionState {
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}
