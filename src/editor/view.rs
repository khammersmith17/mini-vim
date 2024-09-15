use super::terminal::{Position, Size, Terminal};
mod buffer;
use buffer::Buffer;
use crossterm::event::KeyCode;
use std::cmp::max;

const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
const PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct View {
    pub buffer: Buffer,
    pub needs_redraw: bool,
    pub size: Size,
    pub cursor_position: Position,
    pub screen_offset: Position,
}

impl Default for View {
    fn default() -> Self {
        Self {
            buffer: Buffer::default(),
            needs_redraw: true,
            size: Terminal::size().unwrap_or_default(),
            cursor_position: Position::default(),
            screen_offset: Position::default(),
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
        for current_row in self.screen_offset.height..self.screen_offset.height + self.size.height {
            let relative_row = current_row - self.screen_offset.height;
            if let Some(line) = self.buffer.text.get(current_row) {
                self.render_line(relative_row, &line);
            } else if self.buffer.is_empty() && current_row == welcome_row {
                self.render_line(relative_row, &self.get_welcome_message());
            } else {
                self.render_line(relative_row, "~");
            }
        }

        self.needs_redraw = false;
    }

    fn render_line(&self, row: usize, line: &str) {
        let screen_text = if line.len() < self.screen_offset.width {
            ""
        } else if line.len() < self.screen_offset.width + self.size.width {
            &line[self.screen_offset.width..]
        } else {
            &line[self.screen_offset.width..(self.screen_offset.width + self.size.width)]
        };
        let result = Terminal::print_line(row, screen_text);
        debug_assert!(result.is_ok(), "Failed to render line");
    }
    /*
        fn truncate_line(&self, line: &str) -> String {
            line[self.screen_offset.width..self.size.width + self.screen_offset.width].to_string()
        }
    */
    pub fn resize(&mut self, size: Size) {
        self.size = size;
        let Size { height, width } = size;
        self.update_offset(height, width);
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

    pub fn move_cursor(&mut self, key_code: KeyCode) {
        let Size { height, width } = Terminal::size().unwrap_or_default();
        match key_code {
            KeyCode::Down => {
                self.cursor_position.height = self.cursor_position.height.saturating_add(1);
            }
            KeyCode::Up => {
                self.cursor_position.height = max(self.cursor_position.height.saturating_sub(1), 0);
            }
            KeyCode::Left => {
                self.cursor_position.width = max(self.cursor_position.width.saturating_sub(1), 0);
            }
            KeyCode::Right => {
                self.cursor_position.width = self.cursor_position.width.saturating_add(1);
            }
            KeyCode::PageDown => {
                self.cursor_position.height = height + self.screen_offset.height;
            }
            KeyCode::PageUp => {
                self.cursor_position.height = 0;
            }
            KeyCode::End => {
                self.cursor_position.width = 0;
            }
            KeyCode::Home => {
                self.cursor_position.width = self.cursor_position.width + self.screen_offset.width;
            }
            _ => {}
        }
        self.update_offset(height, width);
        self.needs_redraw = true;
    }

    fn update_offset(&mut self, height: usize, width: usize) {
        if self.cursor_position.height > height + self.screen_offset.height {
            self.screen_offset.height = self.screen_offset.height.saturating_add(1);
        }
        if self.cursor_position.height < self.screen_offset.height {
            self.screen_offset.height = self.cursor_position.height;
        }
        if self.cursor_position.width < self.screen_offset.width {
            self.screen_offset.width = self.cursor_position.width;
        }
        if self.cursor_position.width > width + self.screen_offset.width {
            self.screen_offset.width = self.screen_offset.width.saturating_add(1);
        }
    }
}
