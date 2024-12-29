use std::io;

use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue, style,
    terminal::{self, ClearType},
};

struct Editor {}

fn run<W>(w: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    execute!(w, terminal::EnterAlternateScreen)?;

    terminal::enable_raw_mode()?;

    let mut messages: Vec<String> = Vec::new();

    loop {
        queue!(
            w,
            style::ResetColor,
            terminal::Clear(ClearType::All),
            cursor::Hide,
            cursor::MoveTo(0, 0)
        )?;

        for message in &messages {
            queue!(w, style::Print(message), cursor::MoveToNextLine(1))?;
        }

        w.flush()?;

        match read_key_event()? {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                break;
            }
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::CONTROL,
                ..
            } if c.is_ascii_alphabetic() => {
                let ctrl_code = (c.to_ascii_uppercase() as u8 - b'A' + 1) as u8;
                messages.push(format!("Ctrl + {} -> {}", c, ctrl_code));
            }
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                ..
            } => {
                messages.push(format!(
                    "Key: '{}' (ASCII: {}) Modifiers: {:?}",
                    c, c as u8, modifiers
                ));
            }
            KeyEvent {
                code, modifiers, ..
            } => {
                messages.push(format!("KeyCode: {:?} Modifiers: {:?}", code, modifiers));
            }
        }

        if messages.len() > 10 {
            messages.remove(0); // Remove the oldest message
        }
    }

    // Reset terminal to normal mode
    execute!(
        w,
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;

    terminal::disable_raw_mode()
}

pub fn read_key_event() -> std::io::Result<KeyEvent> {
    loop {
        if let Ok(Event::Key(key_event)) = read() {
            return Ok(key_event);
        }
    }
}

pub fn buffer_size() -> io::Result<(u16, u16)> {
    terminal::size()
}

fn main() -> std::io::Result<()> {
    let mut stdout = io::stdout();
    run(&mut stdout)
}
