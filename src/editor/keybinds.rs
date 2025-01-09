use super::{Keymap, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn default_keybinds(keymap: &mut Keymap) {
    keymap.add_keybind(
        vec![Mode::NORMAL],
        vec![
            KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
        ],
        |editor| Ok(editor.stop = true),
    );
}
