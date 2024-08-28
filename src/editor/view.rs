use super::terminal::{Position, Size, Terminal};
use std::io::Error;
mod buffer;
use buffer::Buffer;
use std::cmp::min;

const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
const PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
pub struct View {
    pub buffer: Buffer,
    pub needs_redraw: bool,
    pub size: Size,
}

impl View {
    pub fn render(&mut self) -> Result<(), Error> {
        if self.buffer.is_empty() {
            Self::render_welcome_message(self.size.height)?;
        } else {
            self.render_file(self.size.height, self.size.width)?;
        }
        self.needs_redraw = false;
        Ok(())
    }

    fn render_file(&mut self, height: usize, width: usize) -> Result<(), Error> {
        let mut skip_rows: usize = 0;
        for current_row in 0..height {
            if skip_rows > 0 {
                skip_rows -= 1;
                continue;
            }
            Terminal::clear_line()?;
            if let Some(line) = self.buffer.text.get(current_row) {
                if line.len() > width {
                    let line_len = line.len();
                    while line_len > skip_rows * width {
                        let start = width * skip_rows;
                        let end = min(width * (skip_rows + 1), line_len);
                        let text = line
                            .get(start..end)
                            .expect("Length of line less than width");
                        Terminal::clear_line()?;
                        Terminal::print(text)?;
                        skip_rows = skip_rows.saturating_add(1);
                        Terminal::move_cursor_to(Position {
                            x: 0,
                            y: current_row.saturating_add(skip_rows),
                        })?;
                    }
                } else {
                    Terminal::print(&line)?;
                }
            } else {
                Self::draw_empty_row()?;
            }
            Terminal::move_cursor_to(Position {
                x: 0,
                y: current_row.saturating_add(1),
            })?;
        }
        Ok(())
    }

    fn render_welcome_message(height: usize) -> Result<(), Error> {
        for current_row in 0..height {
            Terminal::clear_line()?;
            #[allow(clippy::integer_division)]
            if current_row == height / 3 {
                Self::draw_welcome_message()?;
            } else {
                Self::draw_empty_row()?;
            }
            if current_row.saturating_add(1) < height {
                Terminal::move_cursor_to(Position {
                    x: 0,
                    y: current_row.saturating_add(1),
                })?;
            }
        }
        Ok(())
    }

    pub fn load(&mut self, filename: &str) {
        if let Ok(buffer) = Buffer::load(filename) {
            self.buffer = buffer;
        }
    }

    fn draw_empty_row() -> Result<(), Error> {
        Terminal::print("~")?;
        Ok(())
    }

    fn draw_welcome_message() -> Result<(), Error> {
        let mut welcome_message = format!("{PROGRAM_NAME} editor -- version {PROGRAM_VERSION}");
        let width = Terminal::size()?.width;
        let len = welcome_message.len();
        #[allow(clippy::integer_division)]
        let padding = (width.saturating_sub(len)) / 2;

        let spaces = " ".repeat(padding.saturating_sub(1));
        welcome_message = format!("~{spaces}{welcome_message}");
        welcome_message.truncate(width);
        Terminal::print(&welcome_message)?;
        Ok(())
    }
}
