use crate::PRESET_DEFAULTS;
use egui::{RichText, TextEdit, Ui};

pub struct PresetAction {
    pub save: bool,
    pub load: bool,
    pub preset_name: String,
    /// Index of the built-in preset to apply, if any
    pub apply_builtin: Option<usize>,
}

impl Default for PresetAction {
    fn default() -> Self {
        Self {
            save: false,
            load: false,
            preset_name: String::new(),
            apply_builtin: None,
        }
    }
}

pub fn preset_section(ui: &mut Ui, preset_name: &mut String) -> PresetAction {
    let mut action = PresetAction::default();

    ui.heading("Presets");
    ui.label("Instrument presets save and load aesthetic parameters (wave, envelope, effects) while preserving musical settings (rhythm, harmony).");
    ui.add_space(5.0);

    // Built-in presets section
    ui.label(RichText::new("Built-in presets:").strong());
    ui.add_space(3.0);

    for (i, (name, _json)) in PRESET_DEFAULTS.iter().enumerate() {
        if ui.button(*name).clicked() {
            action.apply_builtin = Some(i);
        }
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(5.0);

    // User presets section
    ui.label(RichText::new("Custom presets:").strong());
    ui.add_space(3.0);

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
        if ui
            .button("Save Preset")
            .on_hover_text("Save current aesthetic parameters as a preset file")
            .clicked()
        {
            action.save = true;
        }

        if ui
            .button("Load Preset")
            .on_hover_text("Load aesthetic parameters from a preset file")
            .clicked()
        {
            action.load = true;
        }
    });

    ui.add_space(3.0);
    ui.label(
        RichText::new("Tip: Give your preset a descriptive name before saving")
            .weak()
            .italics(),
    );

    action.preset_name = preset_name.clone();
    action
}
