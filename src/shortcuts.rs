use egui::Key;

pub const DELETE: Key = Key::Delete;
pub const CLONE: Key = Key::Insert;
pub const SELECT_UP: Key = Key::ArrowUp;
pub const SELECT_DOWN: Key = Key::ArrowDown;
pub const SWAP_UP: Key = Key::U;
pub const SWAP_DOWN: Key = Key::D;
pub const MUTE: Key = Key::M;

pub fn shortcut(key: Key) -> String {
    format!("Shortcut: {}", key.symbol_or_name())
}
