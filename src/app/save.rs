use crate::app::GuiApp;

impl GuiApp {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_state(&self) {
        use rfd::FileDialog;
        use std::fs;
        if let Some(path) = FileDialog::new()
            .set_title("Save session as JSON")
            .add_filter("JSON", &["json"])
            .save_file()
        {
            use crate::app::GuiState;
            let state = GuiState {
                seqs: self.sequences.clone(),
                selected: self.selected,
            };

            match serde_json::to_string_pretty(&state) {
                Ok(text) => {
                    if let Err(e) = fs::write(&path, text) {
                        eprintln!("[save_state] Failed to write file: {e}");
                    }
                }
                Err(e) => eprintln!("[save_state] Failed to serialize: {e}"),
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    pub fn save_state(&self) {
        todo!()
    }
}
