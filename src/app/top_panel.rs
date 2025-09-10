use crate::{
    app::GuiApp,
    engine::{notes::Sequence, scheduler::Message},
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
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New score").clicked() {
                    self.messages.send(Message::NewScore).unwrap();
                    self.selected = None;
                }
                if ui.button("Add track").clicked() {
                    let idx = len;
                    self.last_token
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let seq =
                        Sequence::new(self.last_token.load(std::sync::atomic::Ordering::Relaxed));
                    self.messages.send(Message::NewSequence(seq)).unwrap();
                    self.selected = Some(idx);
                }
                *save = ui.button("Save…").clicked();
                *load = ui.button("Load…").clicked();
                *exit = ui.button("Exit").clicked();
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
}
