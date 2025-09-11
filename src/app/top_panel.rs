use std::sync::{Arc, Mutex};

use crate::{
    app::GuiApp,
    engine::{notes::Sequence, reverb::Reverb, scheduler::Message},
    stream::stream,
};

impl GuiApp {
    pub fn top_panel(
        &mut self,
        ctx: &egui::Context,
        len: usize,
        save: &mut bool,
        load: &mut bool,
        exit: &mut bool,
    ) {
        let clock = self.clock.clone();
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New score").clicked() {
                    self.sender.send(Message::NewScore).unwrap();
                    self.selected = None;
                }
                if ui.button("Add track").clicked() {
                    let idx = len;
                    self.last_token
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let seq =
                        Sequence::new(self.last_token.load(std::sync::atomic::Ordering::Relaxed));
                    self.sender.send(Message::NewSequence(seq)).unwrap();
                    self.selected = Some(idx);
                    self.start_stream(clock.clone());
                }
                *save = ui.button("Save…").clicked();
                *load = if ui.button("Load…").clicked() {
                    self.stream = None;
                    self.is_playing = false;
                    true
                } else {
                    false
                };
                *exit = ui.button("Exit").clicked();
                if ui
                    .add(egui::Button::new(if self.is_playing {
                        "⏸"
                    } else {
                        "▶"
                    }))
                    .clicked()
                {
                    if self.stream.is_some() {
                        self.stream = None
                    } else {
                        self.start_stream(clock.clone());
                    }
                    self.is_playing = !self.is_playing;
                }

                if self.stream.is_some() {
                    // TODO: when removing it, replace is_playing with just testing for self.stream.is_some()
                    ui.label("stream");
                }
            });
            ui.separator();
            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    ui.label("Left Dealys");
                    let mut delays = self.score_params.delays.0.lock().unwrap();
                    ui.horizontal(|ui| {
                        delays.retain_mut(|d| !ui.add(egui::DragValue::new(d)).secondary_clicked());
                        if ui.button("add").clicked() {
                            delays.push(1);
                        }
                    });
                });
                cols[1].vertical(|ui| {
                    ui.label("Right Dealys");
                    let mut delays = self.score_params.delays.1.lock().unwrap();
                    ui.horizontal(|ui| {
                        delays.retain_mut(|d| !ui.add(egui::DragValue::new(d)).secondary_clicked());
                        if ui.button("add").clicked() {
                            delays.push(1);
                        }
                    });
                });
            });
        });
    }

    fn start_stream(&mut self, clock: Option<std::sync::Arc<std::sync::Mutex<f64>>>) {
        self.stream = Some(stream(
            440.0,
            &self.device,
            clock.unwrap_or(Arc::new(Mutex::new(0.0))),
            self.notes.clone(),
        ))
    }
}
