use crate::{
    app::{GuiApp, GuiState},
    engine::scheduler::Message,
};

impl GuiApp {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_state(&mut self) {
        use rfd::FileDialog;
        use std::{fs, sync::atomic::Ordering};

        // Pick a file to open
        if let Some(path) = FileDialog::new()
            .set_title("Load session from JSON")
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            match fs::read_to_string(&path) {
                Ok(text) => match serde_json::from_str::<GuiState>(&text) {
                    Ok(state) => {
                        // Clear current score on the audio side
                        let _ = self.messages.send(Message::NewScore);

                        // Send each sequence to the scheduler
                        for seq in &state.seqs {
                            let _ = self.messages.send(Message::NewSequence(seq.clone()));
                        }

                        // Update the shared mirror immediately so the UI reflects it right away
                        if let Ok(mut shared) = self.seqs.lock() {
                            *shared = state.seqs.clone();
                        }

                        // Keep tokens monotonic for future "Add track"
                        let next_token = state
                            .seqs
                            .iter()
                            .map(|s| s.token)
                            .max()
                            .unwrap_or(0)
                            .saturating_add(1);
                        self.last_token.store(next_token, Ordering::Relaxed);

                        // Restore selection (clamped)
                        self.selected = state.selected.and_then(|i| {
                            if i < state.seqs.len() {
                                Some(i)
                            } else {
                                None
                            }
                        });
                    }
                    Err(e) => eprintln!("[load_state] Failed to parse JSON: {e}"),
                },
                Err(e) => eprintln!("[load_state] Failed to read file: {e}"),
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    pub fn load_state(&self) {
        todo!()
    }
}
