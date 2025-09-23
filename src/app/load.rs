use crate::app::GuiApp;

impl GuiApp {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_state(&mut self) {
        use rfd::FileDialog;
        use std::fs;
        if let Some(path) = FileDialog::new()
            .set_title("Load session from JSON")
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            use crate::app::GuiState;

            match fs::read_to_string(&path) {
                Ok(text) => match serde_json::from_str::<GuiState>(&text) {
                    Ok(state) => {
                        use crate::{Token, TokenGen};
                        self.new_score();
                        for seq in &state.seqs {
                            self.new_seq(seq.clone());
                        }
                        self.last_token = TokenGen(
                            self.sequences
                                .iter()
                                .map(|s| s.token)
                                .max()
                                .unwrap_or(Token(0))
                                .saturating_add(1),
                        );
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
