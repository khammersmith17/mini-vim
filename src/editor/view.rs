use super::terminal::{Size, Terminal};
mod buffer;
use buffer::Buffer;

const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
const PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct View {
    pub buffer: Buffer,
    pub needs_redraw: bool,
    pub size: Size,
}

impl Default for View {
    fn default() -> Self {
        Self {
            buffer: Buffer::default(),
            needs_redraw: true,
            size: Terminal::size().unwrap_or_default(),
        }
    }
}

impl View {
    pub fn render(&mut self) {
        if self.size.width == 0 || self.size.height == 0 {
            return;
        }
        #[allow(clippy::integer_division)]
        let welcome_row = self.size.height / 3;
        for current_row in 0..self.size.height {
            if let Some(line) = self.buffer.text.get(current_row) {
                let print_line = if line.len() > self.size.width {
                    self.truncate_line(line)
                } else {
                    line.to_string()
                };
                Self::render_line(current_row, &print_line);
            } else if self.buffer.is_empty() && current_row == welcome_row {
                Self::render_line(current_row, &self.get_welcome_message());
            } else {
                Self::render_line(current_row, "~");
            }
        }

        self.needs_redraw = false;
    }

    fn render_line(row: usize, line: &str) {
        let result = Terminal::print_line(row, line);
        debug_assert!(result.is_ok(), "Failed to render line");
    }

    fn truncate_line(&self, line: &str) -> String {
        line[0..self.size.width].to_string()
    }

    pub fn resize(&mut self, size: Size) {
        self.size = size;
    }

    pub fn load(&mut self, filename: &str) {
        if let Ok(buffer) = Buffer::load(filename) {
            self.buffer = buffer;
        }
    }

    fn get_welcome_message(&self) -> String {
        let mut welcome_message = format!("{PROGRAM_NAME} editor -- version {PROGRAM_VERSION}");
        let width = self.size.width;
        let len = welcome_message.len();
        #[allow(clippy::integer_division)]
        let padding = (width.saturating_sub(len)) / 2;

        let spaces = " ".repeat(padding.saturating_sub(1));
        welcome_message = format!("~{spaces}{welcome_message}");
        welcome_message.truncate(width);
        welcome_message
    }
}
