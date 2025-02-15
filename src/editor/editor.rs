use color_eyre::Report;
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;

use crate::util::{Display, Keymap};

#[derive(Eq, PartialEq, Hash)]
pub enum Mode {
    NORMAL,
    COMMAND,
    INSERT,
    VISUAL,
}

pub struct Editor {
    pub(crate) buffer: Vec<String>,
    pub(crate) error: Option<String>,
    pub(crate) command: String,

    pub(crate) filename: Option<String>,

    pub(crate) dirty: bool,
    pub(crate) stop: bool,

    pub(crate) mode: Mode,

    pub(crate) display: Display,

    pub(crate) keymap: Keymap,
    pub(crate) last_key_time: Instant,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: vec![String::new()],
            error: None,
            command: String::new(),

            filename: None,

            dirty: true,
            stop: false,

            mode: Mode::NORMAL,

            display: Display::new(),

            keymap: Keymap::new(),
            last_key_time: Instant::now(),
        }
    }

    pub fn load_file(&mut self, filename: &str) -> Result<(), Report> {
        self.filename = Some(filename.to_string());
        if !Path::new(filename).exists() {
            return Ok(());
        }

        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        self.buffer = reader.lines().collect::<Result<Vec<String>, _>>()?;

        Ok(())
    }

    pub fn save_file(&mut self, filename: &str) -> Result<(), Report> {
        let mut file = File::create(filename)?;
        file.write_all(self.buffer.join("\n").as_bytes())?;

        self.filename = Some(filename.to_string());
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Report> {
        let (tx, mut rx) = mpsc::channel::<KeyEvent>();

        let rt = Runtime::new()?;
        rt.block_on(async {
            tokio::spawn(async move {
                Editor::key_event_listener(tx).await;
            });
        });

        while !self.stop {
            if let Err(err) = self.handle_key_event(&mut rx) {
                self.error = Some(err.to_string());
                self.dirty = true;
            }

            if self.dirty {
                self.display.render(&self.buffer, &self.command, &self.error, &self.mode)?;
                self.dirty = false;
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, rx: &mut mpsc::Receiver<KeyEvent>) -> Result<(), Report> {
        if self.last_key_time.elapsed().as_millis() > 1000 && !self.keymap.is_empty() {
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

        if self.keymap.is_leaf() {
            self.execute_keymap_action()?;
        }

        if let Some(unresolved) = unresolved {
            if !unresolved.modifiers.intersects(KeyModifiers::ALT | KeyModifiers::CONTROL) {
                self.handle_unresolved_key_event(unresolved);
            }
        }

        self.last_key_time = Instant::now();
        self.dirty = true;
        Ok(())
    }

    fn handle_unresolved_key_event(&mut self, unresolved: KeyEvent) {
        match self.mode {
            Mode::COMMAND => {
                if let KeyCode::Char(c) = unresolved.code {
                    self.command.push(c);
                } else if unresolved.code == KeyCode::Backspace {
                    self.command.pop();
                }
            }
            Mode::INSERT => match unresolved.code {
                KeyCode::Char(c) => {
                    let (x, y) = self.display.cursor.position;
                    self.buffer[y as usize].insert(x as usize, c);
                    self.display.cursor_move_by((1, 0), &self.buffer);
                }
                KeyCode::Enter => {
                    let (x, y) = self.display.cursor.position;
                    let remaining = self.buffer[y as usize].split_off(x as usize);
                    self.buffer.insert((y + 1) as usize, remaining);
                    self.display.cursor_move_by((-(x as i16), 1), &self.buffer)
                }
                KeyCode::Delete => {
                    let (x, y) = self.display.cursor.position;
                    if x < self.buffer[y as usize].len() as u16 {
                        self.buffer[y as usize].remove(x as usize);
                    } else if y + 1 < self.buffer.len() as u16 {
                        let next_line = self.buffer.remove((y + 1) as usize);
                        self.buffer[y as usize].push_str(&next_line);
                    }
                }
                KeyCode::Backspace => {
                    let (x, y) = self.display.cursor.position;
                    if x > 0 {
                        self.buffer[y as usize].remove((x - 1) as usize);
                        self.display.cursor_move_by((-1, 0), &self.buffer);
                    } else if y > 0 {
                        let prev_line_len = self.buffer[(y - 1) as usize].len() as u16;
                        let current_line = self.buffer.remove(y as usize);
                        self.buffer[(y - 1) as usize].push_str(&current_line);
                        self.display.cursor_move_by((prev_line_len as i16, -1), &self.buffer);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn execute_keymap_action(&mut self) -> Result<(), Report> {
        if let Some(action) = self.keymap.get_action() {
            for _ in 0..self.keymap.repeats() {
                action.borrow_mut()(self)?;
            }
        };

        self.keymap.clear();
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
