use crossterm::cursor::{Hide, MoveTo, SetCursorStyle, Show};
use crossterm::style::{Color, Print, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{self, disable_raw_mode, enable_raw_mode, size, Clear, ClearType};
use crossterm::{queue, Command};
use std::io::{stdout, Error, Write};
/// Setting the terminal size and position to usize
/// This also handles edge cases
/// Handles the ambiguity between what crossterm accepts accross different methods

#[derive(Copy, Clone, Default, PartialEq)]
pub struct Size {
    pub height: usize,
    pub width: usize,
}

#[derive(Copy, Clone, Default, PartialEq)]
pub struct Position {
    pub width: usize,
    pub height: usize,
}

impl Position {
    pub fn diff_height(&self, other: &Position) -> usize {
        if self.height > other.height {
            return self.height - other.height;
        }
        other.height - self.height
    }

    pub fn set_position(&mut self, new: Position) {
        self.set_height(new.height);
        self.set_width(new.width);
    }

    /*
    pub fn diff_width(&self, other: &Position) -> usize {
        if self.height > other.height {
            return self.width - other.width;
        }
        other.width - self.width
    }
    */

    pub fn view_height(&self, offset: &Position) -> Position {
        Position {
            height: self.height.saturating_sub(offset.height),
            width: self.width,
        }
    }

    pub fn right_of_view(&self, offset: &Position, size: &Size) -> bool {
        if self.width > offset.width + size.width {
            return true;
        }
        false
    }

    pub fn left_of_view(&self, offset: &Position) -> bool {
        if self.width < offset.width {
            return true;
        }
        false
    }

    pub fn width_in_view(&self, offset: &Position, size: &Size) -> bool {
        if self.left_of_view(offset) | self.right_of_view(offset, size) {
            return false;
        }
        true
    }

    pub fn height_in_view(&self, offset: &Position, size: &Size, size_offset: usize) -> bool {
        if self.above_view(offset) | self.below_view(offset, size, size_offset) {
            return false;
        }
        true
    }

    pub fn above_view(&self, offset: &Position) -> bool {
        if self.height < offset.height {
            return true;
        }
        false
    }

    pub fn below_view(&self, offset: &Position, size: &Size, size_height_offset: usize) -> bool {
        if self.height >= offset.height + size.height.saturating_sub(size_height_offset) {
            return true;
        }
        false
    }

    pub fn set_width(&mut self, val: usize) {
        self.width = val;
    }

    pub fn set_height(&mut self, val: usize) {
        self.height = val;
    }

    pub fn left(&mut self, delta: usize) {
        self.width = self.width.saturating_sub(delta);
    }

    pub fn up(&mut self, delta: usize) {
        self.height = self.height.saturating_sub(delta);
    }

    pub fn right(&mut self, delta: usize, max: usize) {
        self.width = std::cmp::min(self.width.saturating_add(delta), max);
    }

    pub fn down(&mut self, delta: usize, max: usize) {
        self.height = std::cmp::min(self.height.saturating_add(delta), max);
    }

    pub fn resolve_width(&mut self, max: usize) {
        self.width = std::cmp::min(self.width, max);
    }

    pub fn page_up(&mut self) {
        self.height = 0;
    }

    pub fn page_down(&mut self, max: usize) {
        self.height = max;
    }

    pub fn at_max_width(&mut self, max_width: usize) -> bool {
        self.width == max_width
    }

    pub fn at_max_height(&mut self, max_height: usize) -> bool {
        self.height == max_height
    }

    pub fn at_top(&mut self) -> bool {
        self.height == 0
    }

    pub fn at_left_edge(&mut self) -> bool {
        self.width == 0
    }

    pub fn snap_right(&mut self, new_width: usize) {
        self.width = new_width;
    }

    pub fn snap_left(&mut self) {
        self.width = 0;
    }
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
        Self::enter_alternate_screen()?;
        Self::clear_screen()?;
        Self::execute()?;
        Ok(())
    }

    pub fn terminate() -> Result<(), Error> {
        Self::leave_alternate_screen()?;
        Self::show_cursor()?;
        Self::set_cursor_style(SetCursorStyle::DefaultUserShape)?;
        Self::execute()?;
        disable_raw_mode()?;
        Ok(())
    }

    pub fn set_cursor_style(style: SetCursorStyle) -> Result<(), Error> {
        Self::queue_command(style)?;
        Ok(())
    }

    pub fn set_background_color(color: Color) -> Result<(), Error> {
        Self::queue_command(SetBackgroundColor(color))?;
        Ok(())
    }

    pub fn set_foreground_color(color: Color) -> Result<(), Error> {
        Self::queue_command(SetForegroundColor(color))?;
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
        Self::queue_command(MoveTo(position.width as u16, position.height as u16))?;
        Ok(())
    }

    ///Returns the size of the terminal
    ///When usize < u16, defaults to usize
    pub fn size() -> Result<Size, Error> {
        let (width, height) = size()?;
        #[allow(clippy::as_conversions)]
        Ok(Size {
            height: height as usize,
            width: width as usize,
        })
    }

    pub fn render_line<T: std::fmt::Display>(row: usize, line: T) -> Result<(), Error> {
        Terminal::move_cursor_to(Position {
            width: 0,
            height: row,
        })?;
        Terminal::clear_line()?;
        Terminal::print(line)?;
        Ok(())
    }

    pub fn print<T: std::fmt::Display>(output: T) -> Result<(), Error> {
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

    fn enter_alternate_screen() -> Result<(), Error> {
        Self::queue_command(terminal::EnterAlternateScreen)?;
        Ok(())
    }

    fn leave_alternate_screen() -> Result<(), Error> {
        Self::queue_command(terminal::LeaveAlternateScreen)?;
        Ok(())
    }
}
