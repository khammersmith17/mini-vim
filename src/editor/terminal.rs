use crate::editor::view::{PROGRAM_NAME, PROGRAM_VERSION};
use crossterm::cursor::{Hide, MoveTo, SetCursorStyle, Show};
use crossterm::style::{Color, Print, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{self, disable_raw_mode, enable_raw_mode, size, Clear, ClearType};
use crossterm::{queue, Command};
use std::io::{stdout, Error, Write};

/// Setting the terminal size and position to usize
/// This also handles edge cases
/// Handles the ambiguity between what crossterm accepts accross different methods

#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
pub struct Size {
    pub height: usize,
    pub width: usize,
}

#[derive(Copy, Clone, Default, Debug)]
pub struct ScreenPosition {
    pub height: usize,
    pub width: usize,
}

//trait for 'Position' and 'ScreenOffset' struct
//for structs that represent some sort of coordinate
pub trait Coordinate {
    fn set_width(&mut self, _val: usize) {}

    fn set_height(&mut self, _val: usize) {}

    fn page_up(&mut self) {}

    fn page_down(&mut self, _max: usize) {}

    fn at_max_width(&self, _max_width: usize) -> bool {
        false
    }

    fn at_max_height(&self, _max_height: usize) -> bool {
        false
    }

    fn at_top(&self) -> bool {
        false
    }

    fn at_left_edge(&self) -> bool {
        false
    }

    fn snap_right(&mut self, _new_width: usize) {}

    fn snap_left(&mut self) {}
}

#[derive(Copy, Clone, Default, Eq, Debug)]
pub struct Position {
    pub width: usize,
    pub height: usize,
    pub max_width: usize,
}

impl PartialEq for Position {
    fn eq(&self, rhs: &Position) -> bool {
        // only evaluate the position data not the max_width
        self.width == rhs.width && self.height == rhs.height
    }
}

// inlining methods here as they will be hot as the cursor move should be a common instruction
impl Coordinate for Position {
    #[inline]
    fn set_width(&mut self, val: usize) {
        self.width = val;
    }

    #[inline]
    fn set_height(&mut self, val: usize) {
        self.height = val;
    }

    fn page_up(&mut self) {
        self.height = 0;
    }

    fn page_down(&mut self, max: usize) {
        self.height = max;
    }

    fn at_max_width(&self, max_width: usize) -> bool {
        self.width == max_width
    }

    fn at_max_height(&self, max_height: usize) -> bool {
        self.height == max_height
    }

    fn at_top(&self) -> bool {
        self.height == 0
    }

    fn at_left_edge(&self) -> bool {
        self.width == 0
    }

    fn snap_right(&mut self, new_width: usize) {
        self.width = new_width;
    }

    fn snap_left(&mut self) {
        self.width = 0;
    }
}

impl Position {
    pub fn to_screen_position(self) -> ScreenPosition {
        ScreenPosition {
            height: self.height,
            width: self.width,
        }
    }

    pub fn diff_height(&self, other: &Position) -> usize {
        if self.height > other.height {
            return self.height.saturating_sub(other.height);
        }
        other.height.saturating_sub(self.height)
    }

    #[inline]
    pub fn left(&mut self, delta: usize) {
        self.width = self.width.saturating_sub(delta);
        self.max_width = self.max_width.saturating_sub(1);
    }

    #[inline]
    pub fn right(&mut self, delta: usize, max: usize) {
        self.width = std::cmp::min(self.width.saturating_add(delta), max);
        self.max_width = self.max_width.saturating_add(1);
    }

    #[inline]
    pub fn up(&mut self, delta: usize) {
        self.height = self.height.saturating_sub(delta);
    }

    #[inline]
    pub fn down(&mut self, delta: usize, max: usize) {
        self.height = std::cmp::min(self.height.saturating_add(delta), max);
    }

    pub fn set_position(&mut self, new: Position) {
        self.set_height(new.height);
        self.set_width(new.width);
    }

    #[inline]
    pub fn max_displacement_from_view(
        &self,
        offset: &ScreenOffset,
        size: &Size,
        reserved_lines: usize,
    ) -> usize {
        let width_displacement: usize = if self.width < offset.width {
            offset.width.saturating_sub(self.width)
        } else if self.width >= offset.width.saturating_add(size.width) {
            self.width
                .saturating_sub(offset.width)
                .saturating_add(size.width)
        } else {
            0_usize
        };

        let cutoff: usize = offset
            .height
            .saturating_add(size.height)
            .saturating_sub(reserved_lines);
        let height_displacement: usize = if self.height < offset.height {
            offset.height.saturating_sub(self.height)
        } else if self.height >= cutoff {
            self.height.saturating_sub(cutoff)
        } else {
            0_usize
        };

        std::cmp::max(height_displacement, width_displacement)
    }

    #[inline] //this is called on every render, very hot
    pub fn relative_view_position(&self, offset: &ScreenOffset) -> ScreenPosition {
        ScreenPosition {
            height: self.height.saturating_sub(offset.height),
            width: self.width.saturating_sub(offset.width),
        }
    }

    pub fn right_of_view(&self, offset: &ScreenOffset, size: &Size) -> bool {
        self.width > offset.width.saturating_add(size.width)
    }

    pub fn left_of_view(&self, offset: &ScreenOffset) -> bool {
        self.width < offset.width
    }

    pub fn above_view(&self, offset: &ScreenOffset) -> bool {
        self.height < offset.height
    }

    pub fn below_view(&self, offset: &ScreenOffset, size: &Size, reserved_lines: usize) -> bool {
        self.height
            >= offset
                .height
                .saturating_add(size.height.saturating_sub(reserved_lines))
    }

    #[inline]
    pub fn resolve_width(&mut self, max: usize) {
        self.width = std::cmp::min(self.max_width, max);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScreenOffset {
    pub height: usize,
    pub width: usize,
}

impl ScreenOffset {
    pub fn to_position(self) -> ScreenPosition {
        ScreenPosition {
            height: self.height,
            width: self.width,
        }
    }

    pub fn handle_offset_screen_snap(
        &mut self,
        pos: &Position,
        size: &Size,
        reserved: usize,
        buffer_len: usize,
    ) {
        // updates the offset when offset adjustment is > 1
        if pos.below_view(self, size, reserved) {
            self.set_height(std::cmp::min(
                buffer_len
                    .saturating_sub(size.height)
                    .saturating_add(reserved), // leave space for the file info line
                pos.height
                    .saturating_sub(size.height)
                    .saturating_add(reserved),
            ));
            if reserved > 1 {
                self.set_height(self.height.saturating_add(1));
            }
        } else if pos.above_view(self) {
            self.set_height(pos.height.saturating_sub(1));
        }

        if pos.at_top() {
            self.page_up();
        }

        if pos.at_left_edge() {
            self.snap_left();
        }

        if pos.width >= size.width.saturating_add(self.width) {
            self.width = pos.width.saturating_sub(size.width).saturating_add(1);
        } else if pos.width < self.width {
            self.width = pos.width;
        }
    }

    pub fn update_offset_single_move(&mut self, pos: &Position, size: &Size, reserved: usize) {
        //if cursor moves beyond height + offset -> increment height offset
        if pos.below_view(self, size, reserved) {
            self.set_height(std::cmp::min(
                self.height.saturating_add(1),
                pos.height
                    .saturating_sub(size.height)
                    .saturating_add(reserved), // space for file info line
            ));
        }
        // if height moves less than the offset -> decrement height
        if pos.above_view(self) {
            self.set_height(pos.height);
        }
        //if widith less than offset -> decerement width
        if pos.left_of_view(self) {
            self.set_width(pos.width);
        }
        //if width moves outside view by 1 increment
        if pos.right_of_view(self, size) {
            //self.screen_offset.width = self.screen_offset.width.saturating_sub(1);
            self.width = self.width.saturating_add(1);
        }
    }
}

// inlining all methods here as they are straight forward computations
impl Coordinate for ScreenOffset {
    fn set_width(&mut self, val: usize) {
        self.width = val;
    }

    fn set_height(&mut self, val: usize) {
        self.height = val;
    }

    fn page_up(&mut self) {
        self.height = 0;
    }

    fn page_down(&mut self, max: usize) {
        self.height = max;
    }

    fn at_max_width(&self, max_width: usize) -> bool {
        self.width == max_width
    }

    fn at_max_height(&self, max_height: usize) -> bool {
        self.height == max_height
    }

    fn at_top(&self) -> bool {
        self.height == 0
    }

    fn at_left_edge(&self) -> bool {
        self.width == 0
    }

    fn snap_right(&mut self, new_width: usize) {
        self.width = new_width;
    }

    fn snap_left(&mut self) {
        self.width = 0;
    }
}

pub enum Mode {
    Insert,
    Vim,
    Search,
    Highlight,
}

impl Mode {
    pub fn to_string(&self) -> &str {
        match *self {
            Self::Insert => "Insert",
            Self::Vim => "Vim",
            Self::Search => "Search",
            Self::Highlight => "Highlight",
        }
    }
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
    pub fn move_cursor_to(position: ScreenPosition) -> Result<(), Error> {
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
        Terminal::move_cursor_to(ScreenPosition {
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

    #[inline]
    pub fn render_status_line(
        mode: &Mode,
        saved: bool,
        size: &Size,
        filename: Option<&str>,
        line_pos: Option<(usize, usize)>,
    ) -> Result<(), Error> {
        let saved = if saved { "saved" } else { "modified" };
        let filename = filename.unwrap_or("-");
        let mut render_message = if let Some((line, len)) = line_pos {
            format!(
                "Mode: {} | Filename: {filename} | Status: {saved} | Line: {line} / {len}",
                mode.to_string()
            )
        } else {
            format!(
                "Mode: {} | Filename: {filename} | Status: {saved} | Line: -",
                mode.to_string()
            )
        };
        render_message.truncate(size.width);
        Self::render_line(size.height.saturating_sub(1), render_message)?;
        Ok(())
    }

    #[inline]
    pub fn get_welcome_message(size: &Size, screen_offset: &ScreenOffset) -> String {
        let mut welcome_message = format!("{PROGRAM_NAME} editor -- version {PROGRAM_VERSION}");
        let width = size.width;
        let len = welcome_message.len();
        #[allow(clippy::integer_division)]
        let padding = (width.saturating_sub(len)) / 2;

        let spaces = " ".repeat(padding.saturating_sub(1));
        welcome_message = format!("~{spaces}{welcome_message}");
        welcome_message.truncate(width);
        let range = screen_offset.width
            ..std::cmp::min(
                screen_offset.width.saturating_add(size.width),
                welcome_message.len(),
            );
        welcome_message = match welcome_message.get(range) {
            Some(text) => text.to_string(),
            None => String::new(),
        };
        welcome_message
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_pos_in_view() {
        //testing a position in the view
        let size1 = Size {
            height: 20,
            width: 20,
        };
        let offset1 = ScreenOffset {
            height: 1,
            width: 1,
        };
        let pos1 = Position {
            height: 12,
            width: 12,
            max_width: usize::default(),
        };

        assert_eq!(pos1.max_displacement_from_view(&offset1, &size1, 1), 0);
    }

    #[test]
    fn test_displacement_height_1() {
        //testing a position in the view
        let size1 = Size {
            height: 20,
            width: 20,
        };
        let offset1 = ScreenOffset {
            height: 1,
            width: 1,
        };
        let pos1 = Position {
            height: 0,
            width: 12,

            max_width: usize::default(),
        };

        assert_eq!(pos1.max_displacement_from_view(&offset1, &size1, 1), 1);
    }

    #[test]
    fn test_displacement_height_2() {
        //testing a position in the view
        let size1 = Size {
            height: 20,
            width: 20,
        };
        let offset1 = ScreenOffset {
            height: 2,
            width: 2,
        };
        let pos1 = Position {
            height: 0,
            width: 12,

            max_width: usize::default(),
        };

        assert_eq!(pos1.max_displacement_from_view(&offset1, &size1, 1), 2);
    }

    #[test]
    fn test_displacement_width_1() {
        //testing a position in the view
        let size1 = Size {
            height: 20,
            width: 20,
        };
        let offset1 = ScreenOffset {
            height: 1,
            width: 0,
        };
        let pos1 = Position {
            height: 0,
            width: 12,

            max_width: usize::default(),
        };

        assert_eq!(pos1.max_displacement_from_view(&offset1, &size1, 1), 1);
    }

    #[test]
    fn test_displacement_width_2() {
        //testing a position in the view
        let size1 = Size {
            height: 20,
            width: 20,
        };
        let offset1 = ScreenOffset {
            height: 1,
            width: 2,
        };
        let pos1 = Position {
            height: 0,
            width: 0,
            max_width: usize::default(),
        };

        assert_eq!(pos1.max_displacement_from_view(&offset1, &size1, 1), 2);
    }

    #[test]
    fn test_displacement_height_and_width() {
        //testing a position in the view
        let size1 = Size {
            height: 20,
            width: 20,
        };
        let offset1 = ScreenOffset {
            height: 9,
            width: 8,
        };
        let pos1 = Position {
            height: 0,
            width: 6,
            max_width: usize::default(),
        };

        assert_eq!(pos1.max_displacement_from_view(&offset1, &size1, 1), 9);
    }
}
