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
                seqs: self.score.sequences.clone(),
                selected: self.selected,
                delays: self.score.delays.clone(),
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
        use rfd::AsyncFileDialog;
        use wasm_bindgen_futures::spawn_local;

        // Build the state the same way as native
        let state = crate::app::GuiState {
            seqs: self.score.sequences.clone(),
            selected: self.selected,
            delays: self.score.delays.clone(),
        };

        let Ok(text) = serde_json::to_string_pretty(&state) else {
            eprintln!("[save_state] Failed to serialize state to JSON");
            return;
        };
        // Move the bytes into the async task
        let bytes: Vec<u8> = text.into_bytes();

        spawn_local(async move {
            if let Some(file) = AsyncFileDialog::new()
                .set_title("Save session as JSON")
                .set_file_name("session.json")
                .add_filter("JSON", &["json"])
                .save_file()
                .await
            {
                if let Err(e) = file.write(&bytes).await {
                    eprintln!("[save_state] Failed to write file: {e}");
                }
            }
            // else: user canceled; do nothing
        });
    }
}
