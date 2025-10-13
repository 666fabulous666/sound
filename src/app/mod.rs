mod load;
mod property_panel;
mod save;
mod start_page;
mod timeline_panel;
mod top_panel;

use crate::{
    engine::{
        score::{
            default_params::{default_delays, default_tempo},
            sequence::Sequence,
            track_node::TrackNode,
            NotesGroup, Score,
        },
        waves::WaveType,
    },
    shortcuts::*,
    texts::README_MD,
    time_freq::Time,
    Token, GLOBAL_VOLUME, GROOVE_DEFAULTS,
};
use arc_swap::ArcSwap;
use cpal::Stream;
use cpal::{traits::DeviceTrait, Device};
use eframe::{egui, App, CreationContext};
use egui::{Color32, Layout, ScrollArea, TextureHandle};
use egui_commonmark::CommonMarkCache;
use instant::Duration;
#[cfg(target_arch = "wasm32")]
use instant::Instant;
use rand::{rngs::ThreadRng, thread_rng};
use serde::{Deserialize, Serialize};
use std::{
    ops::DerefMut,
    sync::{atomic::AtomicU64, Arc},
};

const ALL_WAVES: [WaveType; 10] = [
    WaveType::Mute,
    WaveType::Sine,
    WaveType::Square,
    WaveType::Triangle,
    WaveType::Sawtooth,
    WaveType::HiHat,
    WaveType::Kick,
    WaveType::Snare,
    WaveType::Ride,
    WaveType::Darbuka,
];

const DRUM_WAVES: [WaveType; 5] = [
    WaveType::HiHat,
    WaveType::Kick,
    WaveType::Snare,
    WaveType::Ride,
    WaveType::Darbuka,
];

pub struct GuiApp {
    tempo: f64,
    score: Score,
    clock: Arc<AtomicU64>,
    rng: ThreadRng,
    selected: Option<Vec<usize>>,
    stream: Option<Stream>,
    device: Device,
    sample_rate: f64,
    shared_delays: Arc<ArcSwap<(Vec<f64>, Vec<f64>)>>,
    #[cfg(target_arch = "wasm32")]
    pub(crate) pending_loaded_bytes: std::rc::Rc<std::cell::RefCell<Option<Vec<u8>>>>,
    #[cfg(target_arch = "wasm32")]
    instant: Instant,
    #[cfg(target_arch = "wasm32")]
    fps: f64,
    min_fps: f64,
    show_start: bool,
    show_doc: bool,
    markdown_cache: CommonMarkCache,
    show_default_picker: bool,
    default_pick_idx: usize,
    logo: Option<TextureHandle>,
    property_panel_width: f32,
}

use serde::Deserializer;

#[derive(Deserialize)]
#[serde(untagged)]
enum SequencesCompat {
    Root(TrackNode),       // new format: a single root node
    Nodes(Vec<TrackNode>), // old format: Vec<TrackNode>
    Seqs(Vec<Sequence>),   // optional: very old format: Vec<Sequence>
}

fn deserialize_sequences_compat<'de, D>(de: D) -> Result<TrackNode, D::Error>
where
    D: Deserializer<'de>,
{
    let compat = SequencesCompat::deserialize(de)?;
    Ok(match compat {
        SequencesCompat::Root(root) => root,
        SequencesCompat::Nodes(children) => TrackNode::Group {
            id: Token(0), // placeholder if Group needs an id
            name: "Root".into(),
            muted: false,
            volume: 1.0,
            spacial: 0.5,
            collapsed: false,
            children,
        },
        SequencesCompat::Seqs(seqs) => TrackNode::Group {
            id: Token(0),
            name: "Root".into(),
            muted: false,
            volume: 1.0,
            spacial: 0.5,
            collapsed: false,
            children: seqs.into_iter().map(TrackNode::Seq).collect(),
        },
    })
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GuiState {
    #[serde(deserialize_with = "deserialize_sequences_compat")]
    pub seqs: TrackNode,
    #[serde(default = "default_delays")]
    pub delays: (Vec<f64>, Vec<f64>),
}

impl GuiApp {
    pub fn new(_cc: &CreationContext<'_>, device: Device) -> Self {
        let app = Self {
            tempo: default_tempo(),
            selected: None,
            stream: None,
            shared_delays: Arc::new(ArcSwap::from_pointee((Vec::new(), Vec::new()))),
            clock: Arc::new(AtomicU64::new(0)),
            rng: thread_rng(),
            sample_rate: device.default_output_config().unwrap().sample_rate().0 as f64,
            device: device,
            #[cfg(target_arch = "wasm32")]
            pending_loaded_bytes: std::rc::Rc::new(std::cell::RefCell::new(None)),
            #[cfg(target_arch = "wasm32")]
            instant: Instant::now(),
            #[cfg(target_arch = "wasm32")]
            fps: 60.0,
            min_fps: 60.0,
            show_start: true,
            show_doc: false,
            markdown_cache: egui_commonmark::CommonMarkCache::default(),
            show_default_picker: false,
            default_pick_idx: 0,
            logo: None,
            property_panel_width: 270.0,
            score: Score::new(),
        };
        app
    }
    pub fn score(&self) -> &Score {
        &self.score
    }
    pub fn score_mut(&mut self) -> &mut Score {
        &mut self.score
    }
    pub fn load_logo(&mut self, ctx: &egui::Context) {
        if self.logo.is_none() {
            let image = if ctx.style().visuals.dark_mode {
                let bytes = include_bytes!("../../assets/QuantumHarmonicsTmpWhite.png");

                image::load_from_memory(bytes)
                    .expect("Failed to load logo")
                    .to_rgba8()
            } else {
                let bytes = include_bytes!("../../assets/QuantumHarmonicsTmp.png");
                image::load_from_memory(bytes)
                    .expect("Failed to load logo")
                    .to_rgba8()
            };
            let size = [image.width() as usize, image.height() as usize];

            let pixels = image.into_vec();
            let texture = ctx.load_texture(
                "logo",
                egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                Default::default(),
            );
            self.logo = Some(texture);
        }
    }

    fn doc_page(&mut self, ctx: &egui::Context) {
        use egui_commonmark::CommonMarkViewer;

        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.heading("README");
                ui.separator();

                CommonMarkViewer::new().show(ui, &mut self.markdown_cache, README_MD);

                ui.add_space(12.0);
                if ui.button("Exit README").clicked() {
                    self.show_doc = false;
                }
            });
        });
    }
    fn t_to_x(rect: egui::Rect, t: Time, loop_len: Time) -> f32 {
        rect.left() + t.as_secs() as f32 / loop_len.as_secs() as f32 * rect.width()
    }

    fn hash_color(w: &WaveType) -> egui::Color32 {
        let txt = format!("{:?}", w.to_string());
        let h = hash32(&txt) % 360;
        hsl_to_color32(h as _, 0.5, 0.5)
    }

    fn now(&self) -> Time {
        Time(self.clock.load(std::sync::atomic::Ordering::Relaxed) as f64 / self.sample_rate)
    }

    fn exit(&self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    fn edit_vec<T: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        mut vec: impl DerefMut<Target = Vec<T>>,
        // label: Option<impl Into<WidgetText>>,
        default_value: T,
        layout: Layout,
    ) {
        ui.vertical(|ui| {
            // if let Some(label) = label {
            //     ui.label(label);
            // }
            ui.horizontal(|ui| {
                ui.with_layout(layout, |ui| {
                    vec.retain_mut(|d| !ui.add(egui::DragValue::new(d)).secondary_clicked()); // TODO: return true
                    if ui
                        .button("+")
                        .on_hover_text("Right click an item to remove it.")
                        .clicked()
                    {
                        vec.push(default_value);
                    } else {
                    }
                });
            });
        });
    }
    fn default_picker_window(&mut self, ctx: &egui::Context) {
        use egui::{Align, Layout, RichText};

        egui::Window::new("Choose a default groove")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_min_width(420.0);

                ui.label("Select a preset to load:");
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        for (i, (name, _json)) in GROOVE_DEFAULTS.iter().enumerate() {
                            let selected = self.default_pick_idx == i;
                            if ui.selectable_label(selected, *name).clicked() {
                                self.default_pick_idx = i;
                            }
                        }
                    });

                ui.add_space(8.0);
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_default_picker = false;
                    }

                    ui.add_space(8.0);

                    // Primary action
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Load preset").strong())
                                .min_size(egui::vec2(140.0, 28.0)),
                        )
                        .clicked()
                    {
                        let (_name, json) = GROOVE_DEFAULTS[self.default_pick_idx];
                        match serde_json::from_str::<GuiState>(json) {
                            Ok(state) => {
                                self.apply_loaded_state(state);
                                self.show_default_picker = false;
                                self.show_start = false;
                            }
                            Err(e) => {
                                eprintln!("[default_picker] Failed to parse preset: {e}");
                            }
                        }
                    }
                });
            });
    }

    fn new_score(&mut self) {
        self.score = Score::new();
        self.score.notes.clear();
        self.clock.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    fn visit_sequences<F>(node: &TrackNode, f: &mut F)
    where
        F: FnMut(&Sequence),
    {
        match node {
            TrackNode::Seq(s) => f(s),
            TrackNode::Group { children, .. } => {
                for ch in children {
                    Self::visit_sequences(ch, f); // not `&mut f`
                }
            }
        }
    }

    fn visit_sequences_mut<F>(node: &mut TrackNode, f: &mut F)
    where
        F: FnMut(&mut Sequence),
    {
        match node {
            TrackNode::Seq(s) => f(s),
            TrackNode::Group { children, .. } => {
                for ch in children {
                    Self::visit_sequences_mut(ch, f); // not `&mut f`
                }
            }
        }
    }

    // Add a new node (Seq or Group) to the root: draw it recursively, then insert.
    fn new_node(&mut self, mut node: TrackNode) {
        let now = self.now();
        let volume = match self.score.track_root {
            TrackNode::Group { volume, .. } => volume * GLOBAL_VOLUME,
            TrackNode::Seq(_) => unreachable!(),
        };
        node.draw_node(&mut self.score.notes, &mut self.rng, now, true, volume);
        self.score.track_root.push_child(node);
    }

    // Clone the subtree at `path`, retokenize all sequences, redraw, insert after original.
    fn clone_node(&mut self, path: &[usize]) {
        if path.is_empty() {
            return;
        }

        // 1) Deep-clone the subtree
        let mut cloned = match self.score.track_root.get(path).cloned() {
            Some(n) => n,
            None => return,
        };

        // 2) Assign fresh tokens to every Sequence in the clone
        Self::visit_sequences_mut(&mut cloned, &mut |seq: &mut Sequence| {
            seq.token = self.score.last_token.next();
        });
        // 3) Redraw the whole cloned subtree (no anticipation)
        let now = self.now();
        let volume = self.score.volume_chain_product(path).unwrap();
        cloned.draw_node(&mut self.score.notes, &mut self.rng, now, true, volume);

        // 4) Insert clone right after the original
        let insert_idx = path[path.len() - 1] + 1;
        let mut full_insert_path = path[..path.len() - 1].to_vec();
        full_insert_path.push(insert_idx);
        let _ok = self.score.track_root.insert_at(&full_insert_path, cloned);
    }

    // (optional) Handy by-path variant
    fn regen_node_at(&mut self, path: &[usize]) {
        // collect tokens first
        let tokens: Vec<Token> = {
            let mut out = Vec::new();
            if let Some(n) = self.score.track_root.get(path) {
                Self::visit_sequences(n, &mut |s: &Sequence| {
                    out.push(s.token);
                });
            }
            out
        };
        for tk in tokens {
            self.drain_notes_from_seq(tk);
        }
        let now = self.now();
        let volume = self.score.volume_chain_product(path).unwrap();
        if let Some(n) = self.score.track_root.get_mut(path) {
            n.draw_node(&mut self.score.notes, &mut self.rng, now, true, volume);
        }
    }
    // Collect paths to *sequences* (preorder)
    fn collect_seq_paths(node: &TrackNode, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        match node {
            TrackNode::Seq(_) => out.push(cur.clone()),
            TrackNode::Group { children, .. } => {
                for (i, ch) in children.iter().enumerate() {
                    cur.push(i);
                    Self::collect_seq_paths(ch, cur, out);
                    cur.pop();
                }
            }
        }
    }

    /// Replace the node at `path` (Seq or Group), then regenerate that sequence
    /// and all that follow it in preorder traversal. If a Group is inserted,
    /// regeneration starts from the first sequence inside that group; if the
    /// group has no sequences, it starts from the first sequence after the group.
    fn edit_node_at(&mut self, node: TrackNode, path: &[usize]) {
        // 1) Replace node at path
        {
            if let Some(slot) = self.score.track_root.get_mut(path) {
                *slot = node;
            } else {
                return; // invalid path
            }
        }

        // 2) Compute the preorder list of all *sequence* paths
        let seq_paths: Vec<Vec<usize>> = {
            let mut out = Vec::new();
            let mut cur = Vec::new();
            Self::collect_seq_paths(&self.score.track_root, &mut cur, &mut out);
            out
        };

        // 3) Find where to start regenerating:
        //    - If the edited path is exactly a sequence, start there.
        //    - Else (a Group), start at the first sequence *inside* the group.
        //    - If the group is empty, start at the first sequence *after* the group.
        let start_idx = if let Some(pos) = seq_paths.iter().position(|p| p.as_slice() == path) {
            Some(pos)
        } else if let Some(pos) = seq_paths.iter().position(|p| starts_with(p, path)) {
            Some(pos)
        } else {
            // No sequence inside the group — pick the first sequence whose path is lexicographically > path
            seq_paths
                .iter()
                .enumerate()
                .find(|(_, p)| path_lex_gt(p, path))
                .map(|(i, _)| i)
        };

        // 4) Regenerate from that point onward
        if let Some(pos) = start_idx {
            for p in &seq_paths[pos..] {
                self.regen_node_at(p);
            }
        }
        // else: nothing to regenerate (e.g., no sequences at/after this path)
    }

    // --- delete --------------------------------------------------------------
    fn del_node(&mut self, path: &[usize]) {
        let tokens: Vec<Token> = {
            let mut out = Vec::new();
            if let Some(node) = self.score.track_root.get(path) {
                let mut collect = |seq: &Sequence| {
                    out.push(seq.token);
                };
                Self::visit_sequences(node, &mut collect);
            }
            out
        };
        for tk in tokens {
            self.drain_notes_from_seq(tk);
        }
        self.score.track_root.remove_at(path);
    }

    fn drain_notes_from_seq(&mut self, tk: Token) {
        self.score
            .notes
            .retain(|NotesGroup { token, .. }| *token != tk);
    }
    #[cfg(target_arch = "wasm32")]
    fn poll_loaded_state(&mut self) {
        let state = {
            let mut slot = self.pending_loaded_bytes.borrow_mut();
            slot.take()
                .and_then(|bytes| serde_json::from_slice::<crate::app::GuiState>(&bytes).ok())
        };
        if let Some(state) = state {
            self.apply_loaded_state(state);
        }
    }
    fn try_load_default(&mut self, _ctx: &egui::Context) {
        self.show_default_picker = true;
    }
}

impl App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.load_logo(ctx);

        let mut style: egui::Style = (*ctx.style()).clone();
        style.interaction.tooltip_delay = 0.01;
        ctx.set_style(style);
        let mut save = false;
        let mut load = false;
        let mut exit = false;
        #[cfg(target_arch = "wasm32")]
        self.poll_loaded_state();
        if self.score.track_root.child_count() != 0 {
            // if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, SELECT_UP)) {
            //     if let Some(ref path) = self.selected {
            //         self.selected = self.score.prev_sibling(path, /*wrap=*/ false);
            //     }
            // }
            // if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, SELECT_DOWN)) {
            //     if let Some(ref path) = self.selected {
            //         self.selected = self.score.next_sibling(path, /*wrap=*/ false);
            //     }
            // }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, GROUP)) {
                if let Some(ref path) = self.selected {
                    let new_selected = self.score.first_child_of(path);
                    if new_selected.is_some() {
                        self.selected = new_selected;
                    }
                }
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, GROUP)) {
                if let Some(ref path) = self.selected {
                    let new_selected = self.score.parent_of(path);
                    if new_selected.as_ref().is_some_and(|path| path.len() > 0) {
                        self.selected = new_selected
                    };
                }
            }
        }
        if !self.show_start {
            self.top_panel(ctx, &mut save, &mut load, &mut exit);
        }
        if save {
            self.save_state();
        }
        if load {
            self.load_state(ctx);
        }
        if exit || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.exit(ctx);
        }
        if self.show_default_picker {
            self.default_picker_window(ctx);
        }
        if self.show_doc {
            self.doc_page(ctx);
            if self.show_doc {
                return;
            };
        }
        if self.show_start || self.score.track_root.child_count() == 0 {
            self.start_page(ctx);
            if self.show_start {
                return;
            };
        }
        self.property_panel(ctx);
        self.timeline_panel(ctx);
        ctx.request_repaint_after(Duration::from_millis((1000.0 / self.min_fps) as _));
        self.score.generate_notes(self.now(), &mut self.rng);
        self.score.retain_notes(self.now());
        self.score
            .shared_notes
            .store(Arc::new(self.score.notes.clone()));
        self.shared_delays
            .store(Arc::new(self.score.delays.clone()));
    }
}

/// Convert HSL (0.0–360.0, 0.0–1.0, 0.0–1.0) to `Color32`.
pub fn hsl_to_color32(h: f32, s: f32, l: f32) -> Color32 {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());

    let (r1, g1, b1) = match h_prime as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        5 => (c, 0.0, x),
        _ => (0.0, 0.0, 0.0),
    };

    let m = l - c / 2.0;
    let (r, g, b) = (r1 + m, g1 + m, b1 + m);

    Color32::from_rgb(
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

/// simple deterministic hash for colour
fn hash32(s: &str) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish() as u32
}
#[inline]
fn starts_with(a: &[usize], prefix: &[usize]) -> bool {
    a.starts_with(prefix)
}

#[inline]
fn path_lex_gt(a: &[usize], b: &[usize]) -> bool {
    use std::cmp::Ordering;
    matches!(a.cmp(b), Ordering::Greater)
}
