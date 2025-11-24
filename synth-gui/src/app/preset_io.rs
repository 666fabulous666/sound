use crate::app::GuiApp;
use synth_core::engine::score::preset::InstrumentPreset;

impl GuiApp {
    /// Save the currently selected node's aesthetic parameters as a preset
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_preset(&self, preset_name: String) {
        use rfd::FileDialog;
        use std::fs;

        // Get the selected node
        let Some(ref path) = self.selected else {
            eprintln!("[save_preset] No node selected");
            return;
        };

        let Some(node) = self.score.track_root.get(path) else {
            eprintln!("[save_preset] Selected path is invalid");
            return;
        };

        // Create preset from node's overrides
        let preset = InstrumentPreset::from_overrides(preset_name.clone(), &node.overrides);

        // Open save dialog
        if let Some(save_path) = FileDialog::new()
            .set_title("Save Preset")
            .set_file_name(&format!("{}.json", preset_name.replace(' ', "_")))
            .add_filter("JSON", &["json"])
            .save_file()
        {
            match preset.to_json() {
                Ok(text) => {
                    if let Err(e) = fs::write(&save_path, text) {
                        eprintln!("[save_preset] Failed to write file: {e}");
                    }
                }
                Err(e) => eprintln!("[save_preset] Failed to serialize preset: {e}"),
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn save_preset(&self, preset_name: String) {
        use rfd::AsyncFileDialog;
        use wasm_bindgen_futures::spawn_local;

        // Get the selected node
        let Some(ref path) = self.selected else {
            eprintln!("[save_preset] No node selected");
            return;
        };

        let Some(node) = self.score.track_root.get(path) else {
            eprintln!("[save_preset] Selected path is invalid");
            return;
        };

        // Create preset from node's overrides
        let preset = InstrumentPreset::from_overrides(preset_name.clone(), &node.overrides);

        let Ok(text) = preset.to_json() else {
            eprintln!("[save_preset] Failed to serialize preset to JSON");
            return;
        };

        let bytes: Vec<u8> = text.into_bytes();
        let filename = format!("{}.json", preset_name.replace(' ', "_"));

        spawn_local(async move {
            if let Some(file) = AsyncFileDialog::new()
                .set_title("Save Preset")
                .set_file_name(&filename)
                .add_filter("JSON", &["json"])
                .save_file()
                .await
            {
                if let Err(e) = file.write(&bytes).await {
                    eprintln!("[save_preset] Failed to write file: {e}");
                }
            }
        });
    }

    /// Load a preset and apply it to the currently selected node
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_preset(&mut self, _ctx: &egui::Context) {
        use rfd::FileDialog;
        use std::fs;

        // Get the selected node path
        let Some(ref path) = self.selected.clone() else {
            eprintln!("[load_preset] No node selected");
            return;
        };

        if let Some(file_path) = FileDialog::new()
            .set_title("Load Preset")
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            match fs::read_to_string(&file_path) {
                Ok(text) => match InstrumentPreset::from_json(&text) {
                    Ok(preset) => self.apply_preset(preset, path),
                    Err(e) => eprintln!("[load_preset] Failed to parse JSON: {e}"),
                },
                Err(e) => eprintln!("[load_preset] Failed to read file: {e}"),
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_preset(&mut self, ctx: &egui::Context) {
        use rfd::AsyncFileDialog;
        use wasm_bindgen_futures::spawn_local;

        // Store the path for later use
        let Some(ref path) = self.selected.clone() else {
            eprintln!("[load_preset] No node selected");
            return;
        };

        // Create a slot to store the loaded preset bytes and path
        let slot = self.pending_preset_bytes.clone();
        let ctx_clone = ctx.clone();
        let path_clone = path.clone();

        spawn_local(async move {
            if let Some(file) = AsyncFileDialog::new()
                .set_title("Load Preset")
                .add_filter("JSON", &["json"])
                .pick_file()
                .await
            {
                let bytes = file.read().await;
                *slot.borrow_mut() = Some((bytes, path_clone));
                ctx_clone.request_repaint();
            }
        });
    }

    /// Poll for loaded preset bytes (WASM only)
    #[cfg(target_arch = "wasm32")]
    pub fn poll_loaded_preset(&mut self) {
        let preset_data = {
            let mut slot = self.pending_preset_bytes.borrow_mut();
            slot.take()
        };

        if let Some((bytes, path)) = preset_data {
            match std::str::from_utf8(&bytes) {
                Ok(text) => match InstrumentPreset::from_json(text) {
                    Ok(preset) => self.apply_preset(preset, &path),
                    Err(e) => eprintln!("[poll_loaded_preset] Failed to parse JSON: {e}"),
                },
                Err(e) => eprintln!("[poll_loaded_preset] Invalid UTF-8: {e}"),
            }
        }
    }

    /// Apply a loaded preset to the node at the given path
    pub fn apply_preset(&mut self, preset: InstrumentPreset, path: &[usize]) {
        // Get mutable reference to the selected node
        let Some(node) = self.score.track_root.get_mut(path) else {
            eprintln!("[apply_preset] Selected path is invalid");
            return;
        };

        // Apply preset to the node's overrides
        preset.apply_to_overrides(&mut node.overrides);

        // Regenerate notes for the affected node and following sequences
        self.score.refresh_notes_for_path(path);
    }
}
