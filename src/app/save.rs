use std::fs;

use rfd::FileDialog;

use crate::app::{GuiApp, GuiState};

impl GuiApp {
    pub fn save_state(&self) {
        // Choose where to save
        if let Some(path) = FileDialog::new()
            .set_title("Save session as JSON")
            .add_filter("JSON", &["json"])
            .save_file()
        {
            // Snapshot current state
            let seqs = self.seqs.lock().unwrap().clone();
            let state = GuiState {
                seqs,
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
}
