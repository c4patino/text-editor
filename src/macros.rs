use color_eyre::eyre::eyre;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::mem::take;

use crate::editor::{Editor, Mode};

macro_rules! add_keybind {
    ($editor:expr, $modes:expr, $sequence:expr, $action:expr) => {
        $editor.keymap.add_keybind(
            add_keybind!(@parse_modes $modes),
            add_keybind!(@parse_keys $sequence),
            $action,
        );
    };

    (@parse_keys $keyseq:expr) => {{
        let mut keys = Vec::new();
        let mut chars = $keyseq.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '<' {
                let mut key_name = String::new();
                while let Some(&peek) = chars.peek() {
                    if peek == '>' {
                        chars.next();
                        break;
                    }
                    key_name.push(chars.next().unwrap());
                }

                let (modifiers, key) = if let Some(split_idx) = key_name.find('-') {
                    let (modifier, key) = key_name.split_at(split_idx);
                    (modifier, &key[1..].to_string())
                } else {
                    ("", &key_name)
                };

                let modifiers = match modifiers {
                    "C" => KeyModifiers::CONTROL,
                    "S" => KeyModifiers::SHIFT,
                    "A" => KeyModifiers::ALT,
                    _ => KeyModifiers::NONE,
                };

                let key_event = match key.as_str() {
                    "BS" => KeyEvent::new(KeyCode::Backspace, modifiers),
                    "Tab" => KeyEvent::new(KeyCode::Tab, modifiers),
                    "CR" | "Enter" | "Return" => KeyEvent::new(KeyCode::Enter, modifiers),
                    "Esc" => KeyEvent::new(KeyCode::Esc, modifiers),
                    "Space" => KeyEvent::new(KeyCode::Char(' '), modifiers),
                    "Up" => KeyEvent::new(KeyCode::Up, modifiers),
                    "Down" => KeyEvent::new(KeyCode::Down, modifiers),
                    "Left" => KeyEvent::new(KeyCode::Left, modifiers),
                    "Right" => KeyEvent::new(KeyCode::Right, modifiers),
                    "Insert" => KeyEvent::new(KeyCode::Insert, modifiers),
                    "Del" => KeyEvent::new(KeyCode::Delete, modifiers),
                    "Home" => KeyEvent::new(KeyCode::Home, modifiers),
                    "End" => KeyEvent::new(KeyCode::End, modifiers),
                    "PageUp" => KeyEvent::new(KeyCode::PageUp, modifiers),
                    "PageDown" => KeyEvent::new(KeyCode::PageDown, modifiers),
                    key if key.starts_with('F') && key[1..].parse::<u8>().is_ok() => {
                        let f_num = key[1..].parse::<u8>().unwrap();
                        KeyEvent::new(KeyCode::F(f_num), modifiers)
                    }
                    key if key.len() == 1 => {
                        KeyEvent::new(KeyCode::Char(key.chars().next().unwrap()), modifiers)
                    }
                    _ => panic!("Unknown key: {}", key),
                };

                keys.push(key_event);
            } else {
                keys.push(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
        }

        keys
    }};

    (@parse_modes $modes:expr) => {{
        $modes
            .chars()
            .filter_map(|c| match c {
                'n' => Some(Mode::NORMAL),
                'v' => Some(Mode::VISUAL),
                'c' => Some(Mode::COMMAND),
                'i' => Some(Mode::INSERT),
                _ => None,
            })
            .collect::<Vec<_>>()
    }};
}

pub fn default_keybinds(editor: &mut Editor) {
    add_keybind!(editor, "n", "k", |e| {
        e.display.cursor.move_by((0, -1), &e.buffer);
        Ok(())
    });

    add_keybind!(editor, "n", "j", |e| {
        e.display.cursor.move_by((0, 1), &e.buffer);
        Ok(())
    });

    add_keybind!(editor, "n", "h", |e| {
        e.display.cursor.move_by((-1, 0), &e.buffer);
        Ok(())
    });

    add_keybind!(editor, "n", "l", |e| {
        e.display.cursor.move_by((1, 0), &e.buffer);
        Ok(())
    });

    add_keybind!(editor, "n", "i", |e| {
        e.mode = Mode::INSERT;
        Ok(())
    });

    add_keybind!(editor, "n", ":", |e| {
        e.mode = Mode::COMMAND;
        Ok(())
    });

    add_keybind!(editor, "ic", "<Esc>", |e| {
        e.mode = Mode::NORMAL;
        e.command.clear();
        Ok(())
    });

    add_keybind!(editor, "n", "<CR>", |e| {
        if e.error.is_some() {
            e.error = None;
        }

        Ok(())
    });

    add_keybind!(editor, "n", "$", |e| {
        let (_x, y) = e.display.cursor.position;
        let line_len = e.buffer[y as usize].len() as u16 - 1;
        e.display.cursor.move_x(line_len, &e.buffer);
        Ok(())
    });

    add_keybind!(editor, "n", "_", |e| {
        let current_line = &e.buffer[e.display.cursor.position.1 as usize];
        if let Some((index, _)) = current_line.char_indices().find(|&(_, c)| !c.is_whitespace()) {
            e.display.cursor.move_x(index as u16, &e.buffer);
        }

        Ok(())
    });

    add_keybind!(editor, "n", "gg", |e| {
        e.display.cursor.move_y(0, &e.buffer);
        Ok(())
    });

    add_keybind!(editor, "n", "G", |e| {
        e.display.cursor.move_y(e.buffer.len() as u16, &e.buffer);
        Ok(())
    });

    add_keybind!(editor, "n", "o", |e| {
        e.buffer.insert(e.display.cursor.position.1 as usize + 1, String::new());
        e.display.cursor.move_by((0, 1), &e.buffer);
        e.mode = Mode::INSERT;
        Ok(())
    });

    add_keybind!(editor, "n", "O", |e| {
        e.buffer.insert(e.display.cursor.position.1 as usize, String::new());
        e.display.cursor.move_by((0, 0), &e.buffer);
        e.mode = Mode::INSERT;
        Ok(())
    });

    add_keybind!(editor, "c", "<CR>", |e| {
        if e.command.is_empty() {
            e.mode = Mode::NORMAL;
            return Ok(());
        }

        let command = take(&mut e.command);
        match command.split_whitespace().next() {
            Some("q") => e.stop = true,
            Some("e") => {
                if let Some(filename) = command.split_whitespace().nth(1) {
                    e.load_file(filename)?;
                } else {
                    return Err(eyre!("No filename specified"));
                }
            }
            Some("w") => {
                if let Some(filename) = command.split_whitespace().nth(1).or(e.filename.clone().as_deref()) {
                    e.save_file(filename)?;
                } else {
                    return Err(eyre!("No filename specified"));
                }
            }
            Some("wq") => {
                if let Some(filename) = command.split_whitespace().nth(1).or(e.filename.clone().as_deref()) {
                    e.save_file(filename)?;
                    e.stop = true;
                } else {
                    return Err(eyre!("No filename specified"));
                }
            }
            _ => {}
        }

        e.mode = Mode::NORMAL;
        e.command.clear();
        Ok(())
    });
}
