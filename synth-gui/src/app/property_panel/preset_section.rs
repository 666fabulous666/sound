use egui::TextEdit;

pub struct PresetAction {
    pub save: bool,
    pub load: bool,
    pub preset_name: String,
}

impl Default for PresetAction {
    fn default() -> Self {
        Self {
            save: false,
            load: false,
            preset_name: String::new(),
        }
    }
}

pub fn preset_section(ui: &mut egui::Ui, preset_name: &mut String) -> PresetAction {
    let mut action = PresetAction::default();

    egui::CollapsingHeader::new("Presets")
        .default_open(false)
        .show(ui, |ui| {
            ui.label("Instrument presets save and load aesthetic parameters (wave, envelope, effects) while preserving musical settings (rhythm, harmony).");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Preset name:");
                ui.add(
                    TextEdit::singleline(preset_name)
                        .hint_text("Enter preset name...")
                        .desired_width(150.0),
                );
            });

            ui.add_space(3.0);

            ui.horizontal(|ui| {
                if ui.button("💾 Save Preset")
                    .on_hover_text("Save current aesthetic parameters as a preset file")
                    .clicked()
                {
                    action.save = true;
                }

                if ui.button("📂 Load Preset")
                    .on_hover_text("Load aesthetic parameters from a preset file")
                    .clicked()
                {
                    action.load = true;
                }
            });

            ui.add_space(3.0);
            ui.label(egui::RichText::new("Tip: Give your preset a descriptive name before saving").weak().italics());
        });

    action.preset_name = preset_name.clone();
    action
}
