pub(crate) mod colormap;
mod load;
mod preset_io;
mod property_panel;
mod save;
mod start_page;
mod timeline_panel;
mod top_panel;

pub use property_panel::PropertySection;

use self::timeline_panel::{SequenceDragState, TreeDragState};
use crate::{
    engine::{
        score::{
            node_params::ResolvedTrackParams,
            sequence::Sequence,
            track_node::{NodeKind, TrackNode},
            Score,
        },
        waves::WaveType,
    },
    session::SessionState,
    shortcuts::*,
    texts::README_MD,
    time_freq::{Tempo, Time},
    Token, GROOVE_DEFAULTS, NOTE_LINGER_TIME,
};
use cpal::Stream;
use cpal::{traits::DeviceTrait, Device};
use eframe::{egui, App, CreationContext};
use egui::{Color32, Layout, ScrollArea, TextureHandle};
use egui_commonmark::CommonMarkCache;
use instant::Duration;
#[cfg(target_arch = "wasm32")]
use instant::Instant;
use rand::{rngs::ThreadRng, thread_rng};
use std::{
    collections::HashMap,
    ops::DerefMut,
    sync::{atomic::AtomicU64, Arc},
};
#[cfg(not(target_arch = "wasm32"))]
use synth_core::recorder::Recorder;

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
    score: Score,
    clock: Arc<AtomicU64>,
    rng: ThreadRng,
    selected: Option<Vec<usize>>,
    stream: Option<Stream>,
    device: Device,
    sample_rate: f64,
    tree_drag: Option<TreeDragState>,
    sequence_drag: Option<SequenceDragState>,
    #[cfg(target_arch = "wasm32")]
    pub(crate) pending_loaded_bytes: std::rc::Rc<std::cell::RefCell<Option<Vec<u8>>>>,
    #[cfg(target_arch = "wasm32")]
    pending_preset_bytes: std::rc::Rc<std::cell::RefCell<Option<(Vec<u8>, Vec<usize>)>>>,
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
    spectrogram_render_requested: bool,
    show_spectrogram_panel: bool,
    spectrogram_log_freq: bool,
    spectrogram_freq_multiplier: f32,
    show_new_score_confirm: bool,
    show_exit_confirm: bool,
    tempo_popup_open: bool,
    tempo_popup_pos: Option<egui::Pos2>,
    timeline_zoom: f32,
    timeline_view_start: Time,
    fixed_track_height: bool,
    track_lane_height: f32,
    #[cfg(not(target_arch = "wasm32"))]
    recorder: Arc<Recorder>,
    #[cfg(not(target_arch = "wasm32"))]
    record_error: Option<String>,
    spectrogram_previews: HashMap<Token, SpectrogramPreview>,
    preset_name_input: String,
    pub active_property_section: PropertySection,
}

#[derive(Clone)]
pub struct SpectrogramPreview {
    pub texture: TextureHandle,
    pub size: [usize; 2],
    pub sequence: Sequence,
    pub params: ResolvedTrackParams,
    pub background: Color32,
    pub log_freq: bool,
    pub fundamental_freq: f32,
}
impl GuiApp {
    pub fn new(_cc: &CreationContext<'_>, device: Device) -> Self {
        let app = Self {
            selected: None,
            stream: None,
            clock: Arc::new(AtomicU64::new(0)),
            rng: thread_rng(),
            sample_rate: device.default_output_config().unwrap().sample_rate().0 as f64,
            device: device,
            tree_drag: None,
            sequence_drag: None,
            #[cfg(target_arch = "wasm32")]
            pending_loaded_bytes: std::rc::Rc::new(std::cell::RefCell::new(None)),
            #[cfg(target_arch = "wasm32")]
            pending_preset_bytes: std::rc::Rc::new(std::cell::RefCell::new(None)),
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
            spectrogram_render_requested: false,
            show_spectrogram_panel: true,
            spectrogram_log_freq: false,
            spectrogram_freq_multiplier: 16.0,
            show_new_score_confirm: false,
            show_exit_confirm: false,
            tempo_popup_open: false,
            tempo_popup_pos: None,
            timeline_zoom: 1.0,
            timeline_view_start: Time::new(0.0),
            fixed_track_height: false,
            track_lane_height: 120.0,
            score: Score::new(),
            #[cfg(not(target_arch = "wasm32"))]
            recorder: Arc::new(Recorder::new()),
            #[cfg(not(target_arch = "wasm32"))]
            record_error: None,
            spectrogram_previews: HashMap::new(),
            preset_name_input: String::new(),
            active_property_section: PropertySection::default(),
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
                let bytes = include_bytes!("../../../assets/QuantumHarmonicsTmpWhite.png");

                image::load_from_memory(bytes)
                    .expect("Failed to load logo")
                    .to_rgba8()
            } else {
                let bytes = include_bytes!("../../../assets/QuantumHarmonicsTmp.png");
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
    fn t_to_x(rect: egui::Rect, t: Time, view_start: Time, view_span: Time) -> f32 {
        if view_span.as_secs() <= f64::EPSILON {
            return rect.left();
        }
        let normalized =
            ((t.as_secs() - view_start.as_secs()) / view_span.as_secs()).clamp(-10.0, 10.0);
        rect.left() + normalized as f32 * rect.width()
    }

    fn timeline_lengths(&self, tempo: Tempo) -> (Time, Time, Time) {
        let max_loop_len = self
            .score
            .track_root
            .sequences()
            .fold(Time(0.0), |acc, seq| {
                acc.max(tempo.beats_to_time(seq.loop_len))
            });
        let padding = NOTE_LINGER_TIME.max(Time::new(1.0));
        let track_display_length =
            Time::new((max_loop_len.as_secs() + padding.as_secs() * 8.0).max(padding.as_secs()));
        (max_loop_len, padding, track_display_length)
    }

    fn timeline_view_span(&self, total: Time) -> Time {
        let zoom = self.timeline_zoom.max(0.01);
        let span = total.as_secs() / zoom as f64;
        Time::new(span.max(f64::MIN_POSITIVE))
    }

    fn clamp_timeline_view(&mut self, total: Time, span: Time) {
        let max_start = (total.as_secs() - span.as_secs()).max(0.0);
        self.timeline_view_start.0 = self.timeline_view_start.as_secs().clamp(0.0, max_start);
    }

    fn adjust_timeline_zoom(
        &mut self,
        multiplier: f32,
        focus_ratio: f32,
        total: Time,
        rect: egui::Rect,
    ) {
        if !multiplier.is_finite() || multiplier <= 0.0 {
            return;
        }
        let old_zoom = self.timeline_zoom;
        let new_zoom = (old_zoom * multiplier).clamp(0.1, 16.0);
        if (new_zoom - old_zoom).abs() <= f32::EPSILON {
            return;
        }

        let old_span = self.timeline_view_span(total);
        self.timeline_zoom = new_zoom;
        let new_span = self.timeline_view_span(total);

        let focus_ratio = focus_ratio.clamp(0.0, 1.0) as f64;
        let focus_time =
            Time::new(self.timeline_view_start.as_secs() + old_span.as_secs() * focus_ratio);
        let desired_start =
            focus_time.as_secs() - new_span.as_secs() * focus_ratio.clamp(0.0, 1.0) as f64;
        self.timeline_view_start = Time::new(desired_start);
        self.clamp_timeline_view(total, new_span);

        // Prevent invisible zoom when rect is tiny
        if rect.width() <= f32::EPSILON {
            self.timeline_zoom = old_zoom;
            self.timeline_view_start = Time::new(0.0);
        }
    }

    /// Get color for a sequence, combining wave type and custom hue
    fn seq_color(w: &WaveType, custom_hue: f64) -> egui::Color32 {
        let txt = format!("{:?}", w.to_string());
        let base_h = hash32(&txt) % 360;
        // Blend base hue with custom hue - add them and wrap around
        let h = ((base_h as f64 + custom_hue) % 360.0) as f32;
        hsl_to_color32(h, 0.5, 0.5)
    }

    /// Get color for a group using custom hue with low saturation (gray-ish)
    fn group_color(custom_hue: f64) -> egui::Color32 {
        // Use low saturation for gray appearance, can be adjusted by hue
        let h = (custom_hue % 360.0) as f32;
        hsl_to_color32(h, 0.1, 0.5) // Low saturation for subtle color
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
                        match serde_json::from_str::<SessionState>(json) {
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
        self.spectrogram_previews.clear();
        self.timeline_zoom = 1.0;
        self.timeline_view_start = Time::new(0.0);
    }

    fn visit_sequences<F>(node: &TrackNode, f: &mut F)
    where
        F: FnMut(&Sequence),
    {
        match &node.kind {
            NodeKind::Seq(s) => f(s),
            NodeKind::Group { children, .. } => {
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
        match &mut node.kind {
            NodeKind::Seq(s) => f(s),
            NodeKind::Group { children, .. } => {
                for ch in children {
                    Self::visit_sequences_mut(ch, f); // not `&mut f`
                }
            }
        }
    }

    // Add a new node (Seq or Group) to the root: draw it recursively, then insert.
    fn new_node(&mut self, mut node: TrackNode) {
        let now = self.now();
        self.score.draw_node_with(&mut node, &mut self.rng, now);
        self.score.track_root.push_child(node);
    }

    /// Replace the root with `node`.
    /// - Draws `node` first (recursively).
    /// - Ensures the root remains a `Group` by wrapping a lone `Seq` if needed.
    pub fn replace_root_with(&mut self, node: TrackNode) {
        let now = self.now();
        self.score.replace_root_with(node, &mut self.rng, now);
    }

    fn set_tempo_bpm(&mut self, bpm: f64) {
        let bpm = bpm.max(1.0);
        if (self.score.tempo().beats_per_minute() - bpm).abs() <= f64::EPSILON {
            return;
        }
        let anchor = self.now();
        self.score.set_tempo(Tempo::new(bpm), anchor);
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
        // 3) Redraw the whole cloned subtree
        let now = self.now();
        self.score.draw_node_with(&mut cloned, &mut self.rng, now);

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
            self.spectrogram_previews.remove(&tk);
        }
        let now = self.now();
        self.score.draw_node_at_path(path, &mut self.rng, now);
    }
    // Collect paths to *sequences* (preorder)
    fn collect_seq_paths(node: &TrackNode, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        match &node.kind {
            NodeKind::Seq(_) => out.push(cur.clone()),
            NodeKind::Group { children, .. } => {
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
            self.spectrogram_previews.remove(&tk);
        }
        self.score.track_root.remove_at(path);
    }

    fn drain_notes_from_seq(&mut self, tk: Token) {
        self.score.notes.remove(&tk);
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_loaded_state(&mut self) {
        let state = {
            let mut slot = self.pending_loaded_bytes.borrow_mut();
            slot.take()
                .and_then(|bytes| serde_json::from_slice::<SessionState>(&bytes).ok())
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
        let spectrogram_background = ctx.style().visuals.panel_fill;
        let mut save = false;
        let mut load = false;
        let mut exit = false;
        #[cfg(target_arch = "wasm32")]
        self.poll_loaded_state();
        #[cfg(target_arch = "wasm32")]
        self.poll_loaded_preset();
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
        #[cfg(not(target_arch = "wasm32"))]
        {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.show_exit_confirm = true;
            }
        }
        if save {
            self.save_state();
        }
        if load {
            self.load_state(ctx);
        }
        if exit {
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
        if self.show_spectrogram_panel {
            self.update_spectrogram_preview_if_needed(ctx, spectrogram_background);
            self.spectrogram_panel(ctx);
        }
        self.timeline_panel(ctx);
        ctx.request_repaint_after(Duration::from_millis((1000.0 / self.min_fps) as _));
        self.score.advance(self.now(), &mut self.rng);
        self.score.publish_shared_notes();
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
