use crate::app::GuiApp;
use synth_core::session::SessionState;

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
            match fs::read_to_string(&path) {
                Ok(text) => match serde_json::from_str::<SessionState>(&text) {
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

    pub fn apply_loaded_state(&mut self, state: SessionState) {
        self.new_score();
        self.score
            .apply_session_state(&state, self.now(), &mut self.rng);
        self.score.publish_shared_notes();
    }
}
