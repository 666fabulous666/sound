use crate::{
    app::{GuiApp, GuiState},
    time_freq::{Tempo, Time},
};

impl GuiApp {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_state(&mut self, _ctx: &egui::Context) {
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
                    Ok(state) => self.apply_loaded_state(state),
                    Err(e) => eprintln!("[load_state] Failed to parse JSON: {e}"),
                },
                Err(e) => eprintln!("[load_state] Failed to read file: {e}"),
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_state(&mut self, ctx: &egui::Context) {
        use rfd::AsyncFileDialog;
        use wasm_bindgen_futures::spawn_local;
        let slot = self.pending_loaded_bytes.clone();
        let ctx = ctx.clone();

        spawn_local(async move {
            if let Some(file) = AsyncFileDialog::new()
                .set_title("Load session from JSON")
                .add_filter("JSON", &["json"])
                .pick_file()
                .await
            {
                let bytes = file.read().await; // Vec<u8>
                *slot.borrow_mut() = Some(bytes);
                ctx.request_repaint(); // wake the UI to process result next frame
            }
        });
    }

    pub fn apply_loaded_state(&mut self, state: GuiState) {
        use crate::{Token, TokenGen};

        self.new_score();
        self.score.set_tempo(Tempo::new(state.tempo_bpm), Time(0.0));
        self.replace_root_with(state.seqs);
        self.score_mut().last_token = TokenGen(
            self.score
                .track_root
                .sequences()
                .map(|s| s.token)
                .max()
                .unwrap_or(Token(0))
                .saturating_add(1),
        );
        self.score_mut()
            .track_root
            .for_each_sequence_mut(|s| s.not_generate_until = None);
        self.score.generate_notes(self.now(), &mut self.rng);
        self.update_all_volumes_from_tree(); // Apply volumes based on tree structure
                                             // self.selected = state.selected.filter(|&p| state.seqs.get(p).is_some());
        self.score.delays = state.delays;
    }
}
