use color_eyre::Report;
use crossterm::{
    cursor::{self, SetCursorStyle},
    execute, queue,
    style::{self, Stylize},
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

    fn move_x(&mut self, new_x: u16) {
        self.position.0 = new_x;
    }

    fn move_y(&mut self, new_y: u16) {
        self.position.1 = new_y;
    }

    fn validate_cursor(&mut self, buffer: &Vec<String>) {
        let (_x, y) = self.position;

        if y >= buffer.len() as u16 {
            self.move_y(buffer.len().saturating_sub(1) as u16);
        }

        let line_len = buffer[self.position.1 as usize].len() as u16;
        self.move_x(self.max_column.min(line_len));
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

    pub fn move_cursor(&mut self, delta: (i16, i16), buffer: &Vec<String>) {
        self.cursor.move_by(delta, buffer);
    }

    pub fn render(&mut self, buffer: &Vec<String>, command: &String, mode: &Mode) -> Result<(), Report> {
        queue!(self.out, style::ResetColor, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

        let mut max_lines = self.size.1 as usize;
        if !command.is_empty() {
            max_lines -= 1;
        }

        let max_columns = self.size.0 as usize;

        let render = buffer[self.offset.1 as usize..]
            .iter()
            .take(max_lines)
            .map(|line| {
                if line.len() > self.offset.0 as usize {
                    let start = self.offset.0 as usize;
                    let end = (start + max_columns).min(line.len());
                    &line[start..end]
                } else {
                    ""
                }
            })
            .collect::<Vec<_>>();

        for line in &render {
            queue!(self.out, style::Print(line), cursor::MoveToNextLine(1))?;
        }

        let rendered_lines = render.len();
        if rendered_lines < max_lines {
            let empty_lines = max_lines - rendered_lines;
            for _ in 0..empty_lines {
                queue!(self.out, style::Print("~"), cursor::MoveToNextLine(1))?;
            }
        }

        if !command.is_empty() {
            queue!(
                self.out,
                style::SetAttribute(style::Attribute::Bold),
                style::Print(command),
                style::SetAttribute(style::Attribute::Reset),
                cursor::MoveToNextLine(1)
            )?;
        }

        match mode {
            Mode::INSERT => queue!(self.out, cursor::SetCursorStyle::BlinkingBar)?,
            _ => queue!(self.out, cursor::SetCursorStyle::DefaultUserShape)?,
        }

        queue!(self.out, cursor::MoveTo(self.cursor.position.0, self.cursor.position.1))?;
        self.out.flush()?;
        Ok(())
    }
}