use super::terminal::{Size, Terminal};
use std::io::Error;
mod buffer;
use buffer::Buffer;

const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
const PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
pub struct View {
    pub buffer: Buffer,
}

impl View {
    pub fn render(&self) -> Result<(), Error> {
        let Size { height, .. } = Terminal::size()?;
        let buffer_len = self.buffer.text.len().saturating_sub(1);
        for current_row in 0..height {
            Terminal::clear_line()?;
            #[allow(clippy::integer_division)]
            if current_row <= buffer_len {
                Terminal::print(&self.buffer.text[current_row])?;
            } else if current_row == height / 3 {
                Self::draw_welcome_message()?;
            } else {
                Self::draw_empty_row()?;
            }
            if current_row.saturating_add(1) < height {
                Terminal::print("\r\n")?;
            }
        }
        Ok(())
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
