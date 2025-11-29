use egui::Key;

pub const DELETE: Key = Key::D;
pub const CLONE: Key = Key::I;
pub const SELECT_UP: Key = Key::ArrowUp;
pub const SELECT_DOWN: Key = Key::ArrowDown;
pub const PARENT: Key = Key::ArrowLeft;
pub const FIRST_CHILD: Key = Key::ArrowRight;
pub const WRAP: Key = Key::W;
pub const GROUP: Key = Key::G;
pub const COLLAPSE: Key = Key::C;
pub const MUTE: Key = Key::M;
pub const SOLO: Key = Key::S;

pub fn shortcut(key: Key) -> String {
    format!("Shortcut: {}", key.symbol_or_name())
}

pub struct Shortcut {
    pub keys: &'static [Key],
}

impl Shortcut {
    pub const fn new(keys: &'static [Key]) -> Self {
        Self { keys }
    }

    pub fn name(&self) -> String {
        self.keys
            .iter()
            .map(|k| format!("{:?}", k))
            .collect::<Vec<_>>()
            .join(" / ")
    }

    pub fn pressed(&self, i: &egui::InputState) -> bool {
        self.keys.iter().any(|&k| i.key_pressed(k))
    }
}
