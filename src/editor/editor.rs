use color_eyre::Report;
use crossterm::{
    cursor,
    event::{self, poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue, style,
    terminal::{self, ClearType},
};
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::{Duration, Instant},
};
use tokio::{runtime::Runtime, sync::oneshot};

use super::Keymap;

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum Mode {
    NORMAL,
    COMMAND,
    INSERT,
    VISUAL,
}

#[derive(Debug)]
pub enum ControlCode {
    CONTINUE,
    QUIT,
}

#[derive(Debug)]
pub struct Editor {
    buffer: Vec<String>,
    dirty: bool,
    stop: bool,

    mode: Mode,

    size: (u16, u16),
    cursor: (u16, u16),
    offset: (u16, u16),

    keymap: Keymap,
    last_key_time: Instant,

    out: io::Stdout,
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = execute!(self.out, style::ResetColor, terminal::LeaveAlternateScreen);
    }
}

impl Editor {
    pub fn new() -> Self {
        let mut keymap = Keymap::new();

        keymap.add_keybind(
            vec![Mode::NORMAL],
            vec![
                KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
                KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
            ],
            |editor| Ok(editor.stop = true),
        );

        keymap.add_keybind(
            vec![Mode::NORMAL],
            vec![
                KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
                KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
                KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
            ],
            |editor| Ok(editor.buffer.push("Double QUIT".to_string())),
        );

        let mut editor = Self {
            buffer: vec![String::new()],
            dirty: true,
            stop: false,

            mode: Mode::NORMAL,

            size: terminal::size().unwrap(),
            cursor: (0, 0),
            offset: (0, 0),

            keymap,
            last_key_time: Instant::now(),

            out: io::stdout(),
        };

        let _ = execute!(editor.out, terminal::EnterAlternateScreen);
        editor
    }

    pub fn load_file(&mut self, filename: &str) {
        let file = match File::open(filename) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Error opening file '{}': {}", filename, e);
                return;
            }
        };

        let reader = BufReader::new(file);
        self.buffer = match reader.lines().collect::<Result<Vec<String>, _>>() {
            Ok(lines) => lines,
            Err(e) => {
                eprintln!("Error reading lines from file '{}': {}", filename, e);
                return;
            }
        };
    }

    pub fn run(&mut self) -> Result<(), Report> {
        let (tx, mut rx) = mpsc::channel::<KeyEvent>();

        let rt = Runtime::new()?;
        rt.block_on(async {
            tokio::spawn(async move {
                Editor::key_event_listener(tx).await;
            });

            while !self.stop {
                self.handle_key_event(&mut rx).await?;
                self.render().await?;
            }

            self.buffer.push("This is outside main loop".to_string());
            self.dirty = true;
            self.render().await?;

            Ok(())
        })
    }

    async fn handle_key_event(&mut self, rx: &mut mpsc::Receiver<KeyEvent>) -> Result<(), Report> {
        if self.last_key_time.elapsed().as_millis() > 1000 && !self.keymap.is_empty() {
            self.buffer.push("TIMEOUT".to_string());
            self.execute_keymap_action()?;
            self.dirty = true;
        }

        let event = match rx.try_recv() {
            Ok(event) => event,
            Err(_) => return Ok(()),
        };

        let mut unresolved = self.keymap.traverse(&self.mode, event)?;
        if unresolved.is_some() {
            self.execute_keymap_action()?;
            unresolved = self.keymap.traverse(&self.mode, event)?;
        }

        if let Some(unresolved) = unresolved {
            self.buffer.push(format!(
                "{} {:?} {:?}",
                unresolved.code, unresolved.modifiers, self.last_key_time
            ));
        }

        if self.keymap.is_leaf() {
            self.execute_keymap_action()?;
        }

        self.last_key_time = Instant::now();
        self.dirty = true;
        Ok(())
    }

    fn execute_keymap_action(&mut self) -> Result<(), Report> {
        if let Some(action) = self.keymap.get_action() {
            action.borrow_mut()(self)?;
        };

        self.keymap.clear();
        Ok(())
    }

    async fn render(&mut self) -> Result<(), Report> {
        if !self.dirty {
            return Ok(());
        }

        queue!(
            self.out,
            style::ResetColor,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        let max_lines = self.size.1 as usize;

        let render = &self.buffer[self.offset.1 as usize..]
            .iter()
            .take(max_lines)
            .collect::<Vec<_>>();

        for line in render {
            queue!(self.out, style::Print(line), cursor::MoveToNextLine(1))?;
        }

        let rendered_lines = render.len();
        if rendered_lines < max_lines {
            let empty_lines = max_lines - rendered_lines;
            for _ in 0..empty_lines {
                queue!(self.out, style::Print("~"), cursor::MoveToNextLine(1))?;
            }
        }

        queue!(self.out, cursor::MoveTo(self.cursor.0, self.cursor.1))?;
        self.out.flush()?;

        self.dirty = false;
        Ok(())
    }

    async fn key_event_listener(tx: mpsc::Sender<KeyEvent>) {
        loop {
            if !poll(Duration::from_millis(10)).unwrap() {
                tokio::time::sleep(Duration::from_millis(10)).await;
                continue;
            }

            if let Event::Key(key_event) = read().unwrap() {
                if tx.send(key_event).is_err() {
                    break;
                }
            }
        }
    }
}
