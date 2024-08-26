use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType};
use crossterm::{queue, Command};
use std::io::{stdout, Error, Write};
/// Setting the terminal size and position to usize
/// This also handles edge cases
/// Handles the ambiguity between what crossterm accepts accross different methods
#[derive(Copy, Clone, Default)]
pub struct Size {
    pub height: usize,
    pub width: usize,
}

#[derive(Copy, Clone, Default)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Copy, Clone, Default)]
pub struct Location {
    pub x: usize,
    pub y: usize,
}

pub struct Terminal;

impl Terminal {
    pub fn initialize() -> Result<(), Error> {
        enable_raw_mode()?;
        Self::clear_screen()?;
        Self::execute()?;
        Ok(())
    }

    pub fn terminate() -> Result<(), Error> {
        Self::execute()?;
        disable_raw_mode()?;
        Ok(())
    }

    pub fn clear_screen() -> Result<(), Error> {
        Self::queue_command(Clear(ClearType::All))?;
        Ok(())
    }

    pub fn clear_line() -> Result<(), Error> {
        Self::queue_command(Clear(ClearType::CurrentLine))?;
        Ok(())
    }
    pub fn move_cursor_to(position: Position) -> Result<(), Error> {
        #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
        Self::queue_command(MoveTo(position.x as u16, position.y as u16))?;
        Ok(())
    }

    ///Returns the size of the terminal
    ///When usize < u16, defaults to usize
    pub fn size() -> Result<Size, std::io::Error> {
        let (width, height) = size()?;
        #[allow(clippy::as_conversions)]
        Ok(Size {
            height: height as usize,
            width: width as usize,
        })
    }
    pub fn print(output: &str) -> Result<(), Error> {
        Self::queue_command(Print(output))?;
        Ok(())
    }

    pub fn execute() -> Result<(), Error> {
        stdout().flush()?;
        Ok(())
    }

    pub fn hide_cursor() -> Result<(), Error> {
        Self::queue_command(Hide)?;
        Ok(())
    }

    pub fn show_cursor() -> Result<(), Error> {
        Self::queue_command(Show)?;
        Ok(())
    }

    pub fn queue_command<T: Command>(command: T) -> Result<(), Error> {
        queue!(stdout(), command)?;
        Ok(())
    }
}
