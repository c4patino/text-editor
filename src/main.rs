mod rope;

use std::{cmp, io};

use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue, style,
    terminal::{self, ClearType},
};

use clap::Parser;

enum Mode {
    NORMAL,
    INSERT,
    VISUAL,
}

#[allow(dead_code)]
struct Editor<W: io::Write> {
    buffer: Vec<String>,
    command: String,
    out: W,
    cursor: (u16, u16),
    size: (u16, u16),
    mode: Mode,
}

impl<W: io::Write> Editor<W> {
    fn new(out: W) -> Self {
        Self {
            size: terminal::size().unwrap(),
            buffer: Vec::new(),
            command: String::new(),
            out,
            cursor: (0, 0),
            mode: Mode::NORMAL,
        }
    }

    fn setup(&mut self) -> io::Result<()> {
        execute!(self.out, terminal::EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;

        self.buffer.push("".to_string());

        for _ in 1..self.size.0 {
            self.buffer.push("~".to_string());
        }

        Ok(())
    }

    fn teardown(&mut self) -> io::Result<()> {
        execute!(self.out, style::ResetColor, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()
    }

    fn run(&mut self) -> io::Result<()> {
        loop {
            queue!(
                self.out,
                style::ResetColor,
                terminal::Clear(ClearType::CurrentLine),
                cursor::MoveTo(0, 0)
            )?;

            for message in &self.buffer {
                queue!(self.out, style::Print(message), cursor::MoveToNextLine(1))?;
            }

            queue!(self.out, cursor::MoveTo(self.cursor.0, self.cursor.1))?;

            self.out.flush()?;

            match Self::read_key_event()? {
                KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                } => {
                    break;
                }
                KeyEvent {
                    code: KeyCode::Char('j'),
                    ..
                } => {
                    self.cursor = (self.cursor.0, cmp::min(self.cursor.1 + 1, self.size.1 - 1));
                }
                KeyEvent {
                    code: KeyCode::Char('k'),
                    ..
                } => {
                    self.cursor = (self.cursor.0, self.cursor.1.saturating_sub(1));
                }

                KeyEvent {
                    code: KeyCode::Char('h'),
                    ..
                } => {
                    self.cursor = (self.cursor.0.saturating_sub(1), self.cursor.1);
                }
                KeyEvent {
                    code: KeyCode::Char('l'),
                    ..
                } => {
                    self.cursor = (cmp::min(self.cursor.0 + 1, self.size.0 - 1), self.cursor.1);
                }
                KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                } => {
                    self.cursor = (
                        self.cursor.0,
                        cmp::min(self.cursor.1 + self.size.1 / 2, self.size.1 - 1),
                    );
                }
                KeyEvent {
                    code: KeyCode::Char('u'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                } => {
                    self.cursor = (
                        self.cursor.0,
                        cmp::max(self.cursor.1.saturating_sub(self.size.1 / 2), 0),
                    );
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn read_key_event() -> std::io::Result<KeyEvent> {
        loop {
            if let Ok(Event::Key(key_event)) = read() {
                return Ok(key_event);
            }
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    filename: Option<String>,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    println!("{:?}", args);

    let mut editor = Editor::new(io::stdout());

    //editor.setup()?;
    //editor.run()?;
    //editor.teardown()

    Ok(())
}
