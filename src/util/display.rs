use color_eyre::Report;
use crossterm::{
    cursor, execute, queue, style,
    terminal::{self, ClearType},
};
use std::io::{self, Write};

use crate::editor::Mode;

pub struct Display {
    size: (u16, u16),
    offset: (u16, u16),

    pub(crate) cursor: Cursor,

    pub(crate) out: io::Stdout,
}

pub struct Cursor {
    pub(crate) position: (u16, u16),
    pub(crate) max_column: u16,
}

impl Cursor {
    pub fn new() -> Self {
        Self { position: (0, 0), max_column: 0 }
    }

    fn move_by(&mut self, delta: (i16, i16), buffer: &Vec<String>) {
        let saturate = |pos: u16, delta: i16| {
            if delta.is_negative() {
                pos.saturating_sub(delta.abs() as u16)
            } else {
                pos.saturating_add(delta as u16)
            }
        };

        let (mut x, mut y) = self.position;
        let (dx, dy) = delta;

        if dx != 0 {
            x = saturate(x, dx);
            self.max_column = x;
        }

        if dy != 0 {
            y = saturate(y, dy);
        }

        self.position = (x, y);
        self.validate_cursor(buffer);
    }

    fn move_x(&mut self, new_x: u16, buffer: &Vec<String>) {
        self.position.0 = new_x;
        self.max_column = new_x;

        self.validate_cursor(buffer);
    }

    fn move_y(&mut self, new_y: u16, buffer: &Vec<String>) {
        self.position.1 = new_y;
        self.validate_cursor(buffer);
    }

    fn validate_cursor(&mut self, buffer: &Vec<String>) {
        let (_x, y) = self.position;

        if y >= buffer.len() as u16 {
            self.position.1 = buffer.len().saturating_sub(1) as u16;
        }

        let line_len = buffer[self.position.1 as usize].len() as u16;
        self.position.0 = self.max_column.min(line_len);
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        let _ = execute!(self.out, style::ResetColor, terminal::LeaveAlternateScreen);
    }
}

impl Display {
    pub fn new() -> Self {
        let mut display =
            Self { size: terminal::size().unwrap(), offset: (0, 0), cursor: Cursor::new(), out: io::stdout() };

        let _ = execute!(display.out, terminal::EnterAlternateScreen);

        display
    }

    pub fn render(
        &mut self,
        buffer: &Vec<String>,
        command: &String,
        error: &Option<String>,
        mode: &Mode,
    ) -> Result<(), Report> {
        queue!(self.out, style::ResetColor, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

        let mut max_lines = self.size.1 as usize;
        if *mode == Mode::COMMAND {
            max_lines -= 1;
        }
        if let Some(error) = error {
            max_lines -= error.matches('\n').count();
        }

        let max_columns = self.size.0 as usize;

        let cursor_line = self.cursor.position.1 as usize;
        let render = buffer[self.offset.1 as usize..]
            .iter()
            .enumerate()
            .take(max_lines)
            .map(|(i, line)| {
                let rendering_line = self.offset.1 as usize + i;
                let relative_number = if rendering_line == cursor_line {
                    self.cursor.position.1.to_string()
                } else {
                    (cursor_line as isize - rendering_line as isize).abs().to_string()
                };

                let padded_number = format!("{:>4}  ", relative_number);

                let trimmed_line = if line.len() > self.cursor.position.0 as usize {
                    let start = self.offset.0 as usize;
                    let end = (start + max_columns).min(line.len());
                    &line[start..end]
                } else {
                    ""
                };

                format!("{}{}", padded_number, trimmed_line)
            })
            .collect::<Vec<_>>();

        for line in &render {
            queue!(self.out, style::Print(line), cursor::MoveToNextLine(1))?;
        }

        let rendered_lines = render.len();
        if rendered_lines < max_lines {
            let empty_lines = max_lines - rendered_lines;
            for _ in 0..empty_lines {
                queue!(self.out, style::Print("   ~ "), cursor::MoveToNextLine(1))?;
            }
        }

        if let Some(error) = error {
            for line in error.split("\n") {
                queue!(
                    self.out,
                    style::SetAttribute(style::Attribute::Italic),
                    style::Print(line),
                    style::SetAttribute(style::Attribute::Reset),
                    cursor::MoveToNextLine(1)
                )?;
            }
        }

        match mode {
            Mode::INSERT => queue!(
                self.out,
                cursor::SetCursorStyle::BlinkingBar,
                cursor::MoveTo(self.cursor.position.0 - self.offset.0 + 6, self.cursor.position.1 - self.offset.1),
            )?,
            Mode::COMMAND => queue!(
                self.out,
                style::SetAttribute(style::Attribute::Bold),
                style::Print(format!(":{}", command)),
                style::SetAttribute(style::Attribute::Reset),
                cursor::SetCursorStyle::BlinkingBar,
                cursor::MoveTo(command.len() as u16 + 1, self.size.1)
            )?,
            _ => queue!(
                self.out,
                cursor::SetCursorStyle::DefaultUserShape,
                cursor::MoveTo(self.cursor.position.0 - self.offset.0 + 6, self.cursor.position.1 - self.offset.1)
            )?,
        }

        self.out.flush()?;
        Ok(())
    }

    pub fn cursor_move_by(&mut self, delta: (i16, i16), buffer: &Vec<String>) {
        self.cursor.move_by(delta, buffer);
        self.validate_offset();
    }

    pub fn cursor_move_x(&mut self, new_x: u16, buffer: &Vec<String>) {
        self.cursor.move_x(new_x, buffer);
        self.validate_offset();
    }

    pub fn cursor_move_y(&mut self, new_y: u16, buffer: &Vec<String>) {
        self.cursor.move_y(new_y, buffer);
        self.validate_offset();
    }

    fn validate_offset(&mut self) {
        if self.cursor.position.0 >= self.offset.0 + self.size.0 {
            self.offset.1 = self.cursor.position.1 - self.size.0 + 1;
        }
        if self.cursor.position.1 >= self.offset.1 + self.size.1 {
            self.offset.1 = self.cursor.position.1 - self.size.1 + 1;
        }
        if self.cursor.position.0 < self.offset.0 {
            self.offset.0 = self.cursor.position.0;
        }
        if self.cursor.position.1 < self.offset.1 {
            self.offset.1 = self.cursor.position.1;
        }
    }
}
