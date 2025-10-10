use egui::Key;

pub const DELETE: Key = Key::D;
pub const CLONE: Key = Key::I;
pub const SELECT_UP: Key = Key::K;
pub const SELECT_DOWN: Key = Key::J;
pub const SWAP_UP: Key = Key::U;
pub const SWAP_DOWN: Key = Key::D;
pub const WRAP: Key = Key::W;
pub const GROUP: Key = Key::G;
pub const COLLAPSE: Key = Key::C;
pub const MUTE: Key = Key::M;

pub fn shortcut(key: Key) -> String {
    format!("Shortcut: {}", key.symbol_or_name())
}
