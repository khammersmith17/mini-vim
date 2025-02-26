use super::editorcommands::{
    parse_highlight_normal_mode, Direction, EditorCommand, FileNameCommand, JumpCommand,
};
use super::terminal::{Coordinate, Mode, Position, ScreenOffset, ScreenPosition, Size, Terminal};
use crossterm::event::read;
use std::{error::Error, path::Path};
pub mod buffer;
use buffer::Buffer;
pub mod line;
mod theme;
use theme::Theme;
mod search;
use search::Search;
pub mod help;
use help::Help;
mod highlight;
use highlight::Highlight;
mod vim_mode;
use vim_mode::VimMode;
mod clipboard_interface;
use clipboard_interface::ClipboardUtils;

enum ScreenUpdateType {
    FullScreen,
    MultiLineRender,
    SingleLineRender,
    DefaultAction,
}

pub const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
pub const PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

const ORIGIN_POSITION: Position = Position {
    height: 0_usize,
    width: 0_usize,
    max_width: 0_usize,
};

/// the core logic
pub struct View {
    pub size: Size,
    pub cursor_position: Position,
    pub screen_offset: ScreenOffset,
    pub theme: Theme,
    pub buffer: Buffer,
}

impl Default for View {
    fn default() -> Self {
        Self {
            buffer: Buffer::default(),
            size: Terminal::size().unwrap_or_default(),
            cursor_position: Position::default(),
            screen_offset: ScreenOffset::default(),
            theme: Theme::default(),
        }
    }
}

impl View {
    pub fn start(&self) -> Result<(), Box<dyn Error>> {
        self.full_screen_render()?;
        self.set_cursor_and_status()?;
        Terminal::execute()?;
        Ok(())
    }

    pub fn render(&self, full_screen: bool) {
        let start = if full_screen {
            self.screen_offset.height
        } else {
            self.cursor_position.height.saturating_sub(1)
        };
        #[allow(clippy::integer_division)]
        for current_row in start
            ..self
                .screen_offset
                .height
                .saturating_add(self.size.height)
                .saturating_sub(1)
        {
            let relative_row = current_row.saturating_sub(self.screen_offset.height);

            if let Some(line) = self.buffer.text.get(current_row) {
                Self::render_line(
                    relative_row,
                    line.get_line_subset(
                        self.screen_offset.width
                            ..self.screen_offset.width.saturating_add(self.size.width),
                    ),
                );
            } else if self.buffer.is_empty() & (current_row == self.size.height / 3) {
                Self::render_line(
                    relative_row,
                    Terminal::get_welcome_message(&self.size, &self.screen_offset),
                );
            } else {
                Self::render_line(relative_row, "~");
            }
        }
    }

    #[inline] // this should be very hot
    fn evaluate_view_state_change(&mut self) -> ScreenUpdateType {
        let view_delta = self.check_offset();
        if view_delta == 0 {
            ScreenUpdateType::SingleLineRender
        } else {
            ScreenUpdateType::FullScreen
        }
    }

    fn full_screen_render(&self) -> Result<(), Box<dyn Error>> {
        Terminal::hide_cursor()?;
        Terminal::move_cursor_to(self.screen_offset.to_position())?;
        Terminal::clear_screen()?;
        self.render(true);
        Terminal::move_cursor_to(
            self.cursor_position
                .relative_view_position(&self.screen_offset),
        )?;
        Terminal::show_cursor()?;
        Terminal::execute()?;
        Ok(())
    }

    #[inline]
    fn render_line<T: std::fmt::Display>(row: usize, line: T) {
        let result = Terminal::render_line(row, line);
        debug_assert!(result.is_ok(), "Failed to render line");
    }

    fn resize(&mut self, size: Size) {
        self.size = size;
        self.screen_offset.handle_offset_screen_snap(
            &self.cursor_position,
            &self.size,
            1,
            self.buffer.len(),
        );
    }

    pub fn load(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        let path = Path::new(filename);
        if path.is_dir() {
            return Err(format!("{filename} is a directory").into());
        }
        if let Ok(buffer) = Buffer::load(filename) {
            self.buffer = buffer;
        } else {
            self.buffer = Buffer::load_named_empty(filename);
        }

        Ok(())
    }

    // inlining because it is a rather straight forward computation
    #[inline]
    fn move_cursor(&mut self, key_code: Direction) {
        if self.buffer.is_empty() {
            self.cursor_position = ORIGIN_POSITION;
        } else {
            key_code.move_cursor(&mut self.cursor_position, &self.buffer);
        }
    }

    #[inline]
    fn insert_char(&mut self, insert_char: char) {
        self.buffer
            .update_line_insert(&mut self.cursor_position, insert_char);

        self.buffer.is_saved = false;
    }

    fn insert_tab(&mut self) {
        self.buffer.insert_tab(&self.cursor_position, 1);
        self.cursor_position.width = self.cursor_position.width.saturating_add(4);
    }

    #[inline]
    fn delete_char(&mut self) {
        //get the width of the char being deleted to update the cursor position
        self.buffer.update_line_delete(&mut self.cursor_position);
    }

    pub fn get_file_name(&mut self) {
        // clear_screen and render screen to get file name
        let mut filename_buffer = String::new();
        let mut curr_position: usize = 10;
        Self::render_filename_screen(&filename_buffer, curr_position);
        loop {
            let Ok(read_event) = read() else { continue };

            match FileNameCommand::try_from(read_event) {
                Ok(event) => match event {
                    FileNameCommand::Insert(c) => {
                        filename_buffer.push(c);
                        curr_position = curr_position.saturating_add(1);
                    }
                    FileNameCommand::BackSpace => {
                        filename_buffer.pop();
                        curr_position = std::cmp::max(10, curr_position.saturating_sub(1));
                    }
                    FileNameCommand::SaveFileName => break,
                    FileNameCommand::NoAction => continue,
                    FileNameCommand::Quit => return,
                },
                _ => continue,
            }

            Self::render_filename_screen(&filename_buffer, curr_position);
        }

        self.buffer.assume_file_name(filename_buffer);
    }

    fn render_filename_screen(curr_filename: &str, curr_position: usize) {
        Terminal::hide_cursor().unwrap();
        Terminal::move_cursor_to(ScreenPosition {
            height: 0,
            width: 0,
        })
        .unwrap();
        Terminal::clear_screen().unwrap();
        Self::render_line(0, format!("Filename: {}", &curr_filename));
        Terminal::move_cursor_to(ScreenPosition {
            height: 0,
            width: curr_position,
        })
        .unwrap();
        Terminal::show_cursor().unwrap();
        Terminal::execute().unwrap();
    }

    fn set_cursor_and_status(&self) -> Result<(), Box<dyn Error>> {
        Terminal::render_status_line(
            &Mode::Insert,
            self.buffer.is_saved,
            &self.size,
            self.buffer.filename.as_deref(),
            Some((
                self.cursor_position.height.saturating_add(1),
                self.buffer.len(),
            )),
        )?;
        Terminal::move_cursor_to(
            self.cursor_position
                .relative_view_position(&self.screen_offset),
        )?;
        Terminal::show_cursor()?;
        Ok(())
    }

    fn save(&mut self) -> Result<(), Box<dyn Error>> {
        // no need to render here
        if self.buffer.filename.is_none() {
            self.get_file_name();
        }
        self.buffer.save();
        // onyl status line needs to change
        Terminal::render_status_line(
            &Mode::Insert,
            self.buffer.is_saved,
            &self.size,
            self.buffer.filename.as_deref(),
            Some((
                self.cursor_position.height.saturating_add(1),
                self.buffer.len(),
            )),
        )?;
        Terminal::move_cursor_to(
            self.cursor_position
                .relative_view_position(&self.screen_offset),
        )?;
        Ok(())
    }

    fn paste_text(&mut self) -> Option<bool> {
        // render always
        let Ok(paste_text) = ClipboardUtils::get_text_from_clipboard() else {
            return Some(true); // handling an error here
        };
        self.buffer
            .add_text_from_clipboard(&paste_text, &mut self.cursor_position);
        None
    }

    fn enter_vim_mode(&mut self) -> bool {
        let mut vim_mode = VimMode::new(
            self.cursor_position,
            self.screen_offset,
            self.size,
            &mut self.buffer,
        );
        vim_mode.run(
            &mut self.cursor_position,
            &mut self.screen_offset,
            &mut self.size,
            self.theme.highlight,
            self.theme.text,
        )
    }

    fn enter_highlight_mode(&mut self) {
        let mut highlight = Highlight::new(
            &mut self.cursor_position,
            self.screen_offset,
            &mut self.size,
            &mut self.buffer,
        );
        highlight.run(
            self.theme.highlight,
            self.theme.text,
            parse_highlight_normal_mode,
        );
    }

    fn enter_search_mode(&mut self) {
        let mut search = Search::new(
            self.cursor_position,
            self.screen_offset,
            self.theme.highlight,
            self.theme.text,
        );
        search.run(
            &mut self.cursor_position,
            &mut self.screen_offset,
            &mut self.size,
            &self.buffer,
        );
    }

    pub fn handle_event(&mut self, command: EditorCommand) -> Result<bool, Box<dyn Error>> {
        let mut continue_status: bool = true;
        let mut render_type: ScreenUpdateType = ScreenUpdateType::DefaultAction;
        match command {
            EditorCommand::Move(direction) => {
                // if offset changes, render the entire screen
                self.move_cursor(direction);
                let view_delta = self.check_offset();
                render_type = if view_delta == 0 {
                    ScreenUpdateType::DefaultAction
                } else {
                    ScreenUpdateType::FullScreen
                };
            }
            EditorCommand::Insert(char) => {
                self.insert_char(char);
                render_type = self.evaluate_view_state_change();
            }
            EditorCommand::Delete => {
                self.deletion();
            }
            EditorCommand::Tab => {
                self.insert_tab();
                render_type = self.evaluate_view_state_change();
            }
            EditorCommand::NewLine => {
                self.new_line();
                render_type = ScreenUpdateType::MultiLineRender;
            }
            EditorCommand::JumpWord(direction) => self.jump_word(direction),
            EditorCommand::Save => self.save()?,
            EditorCommand::Resize(size) => {
                // render always
                self.resize(size);
                self.check_offset(); // cursor may no longer be on screen
                render_type = ScreenUpdateType::FullScreen;
            }

            EditorCommand::Paste => {
                // self.render(true)?;
                if let Some(_failed_paste) = self.paste_text() {
                    return Ok(true);
                }
                render_type = ScreenUpdateType::FullScreen;
            }
            EditorCommand::VimMode => {
                self.enter_vim_mode();
                render_type = ScreenUpdateType::FullScreen;
            }
            EditorCommand::Highlight => {
                self.enter_highlight_mode();
                let _ = self.check_offset(); // making sure the offset is correct on a delete
                render_type = ScreenUpdateType::FullScreen;
            }
            EditorCommand::Search => {
                self.enter_search_mode();
                render_type = ScreenUpdateType::FullScreen;
            }
            EditorCommand::JumpLine => {
                render_type = self.jump_cursor()?;
            }
            EditorCommand::Help => {
                Help::render_help(&mut self.size, self.theme.highlight, self.theme.text);
                self.full_screen_render()?;
            }

            EditorCommand::Quit => continue_status = false,
            EditorCommand::Theme => {
                self.theme.set_theme();
                render_type = ScreenUpdateType::FullScreen;
            }
            EditorCommand::None => {}
        }
        self.eval_screen_update(&render_type)?;
        self.set_cursor_and_status()?;
        Terminal::execute()?;
        Ok(continue_status)
    }

    fn eval_screen_update(&self, update_t: &ScreenUpdateType) -> Result<(), Box<dyn Error>> {
        match update_t {
            ScreenUpdateType::FullScreen => self.full_screen_render()?,
            ScreenUpdateType::SingleLineRender => {
                Self::render_line(
                    self.cursor_position
                        .height
                        .saturating_sub(self.screen_offset.height),
                    self.buffer.text[self.cursor_position.height].get_line_subset(
                        self.screen_offset.width
                            ..self.screen_offset.width.saturating_add(self.size.width),
                    ),
                );
            }
            ScreenUpdateType::MultiLineRender => {
                self.render(false);
            }
            ScreenUpdateType::DefaultAction => {
                // no additional action is required
            }
        }
        Ok(())
    }

    fn new_line(&mut self) -> ScreenUpdateType {
        self.buffer.add_new_line(&mut self.cursor_position);
        // handling if the new line is currently off screen
        let view_delta = self.check_offset();
        if view_delta > 1 {
            ScreenUpdateType::FullScreen
        } else {
            ScreenUpdateType::MultiLineRender
        }
    }

    #[inline]
    fn check_offset(&mut self) -> usize {
        let view_delta =
            self.cursor_position
                .max_displacement_from_view(&self.screen_offset, &self.size, 2);
        match view_delta {
            0 => (),
            1 => self
                .screen_offset
                .update_offset_single_move(&self.cursor_position, &self.size, 2),
            _ => self.screen_offset.handle_offset_screen_snap(
                &self.cursor_position,
                &self.size,
                1,
                self.buffer.len(),
            ),
        }
        view_delta
    }

    #[inline]
    fn deletion(&mut self) {
        if self.buffer.is_empty() || self.cursor_position == ORIGIN_POSITION {
            return;
        }
        match self.cursor_position.width {
            0 => match (
                self.cursor_position.at_top(),
                self.buffer.text[self.cursor_position.height].is_empty(),
            ) {
                (true, true) => return,
                (false, true) => {
                    self.buffer.text.remove(self.cursor_position.height);
                    self.cursor_position.up(1);
                    self.cursor_position
                        .set_width(self.buffer.text[self.cursor_position.height].grapheme_len());
                }
                _ => {
                    // get length of 1 line above
                    // this will be new width after join line operation
                    let prev_line_width = self.buffer.text
                        [self.cursor_position.height.saturating_sub(1)]
                    .grapheme_len();
                    self.buffer.join_line(self.cursor_position.height);
                    self.cursor_position.up(1);
                    self.cursor_position.set_width(prev_line_width);
                }
            },
            _ => {
                self.delete_char();
            }
        };

        // evaluate how much of the screen we need to render
        let view_delta = self.check_offset();
        self.render(view_delta > 0);
        let move_res = Terminal::move_cursor_to(
            self.cursor_position
                .relative_view_position(&self.screen_offset),
        );
        debug_assert!(move_res.is_ok());
    }

    fn overwrite_last_line(&self) -> Result<(), Box<dyn Error>> {
        let l = self
            .screen_offset
            .height
            .saturating_add(self.size.height)
            .saturating_sub(2);

        if self.buffer.len() >= l {
            Terminal::render_line(
                self.size.height.saturating_sub(2),
                self.buffer.text[l].get_line_subset(
                    self.screen_offset.width
                        ..self.screen_offset.width.saturating_add(self.size.width),
                ),
            )?;
        } else {
            Terminal::render_line(self.size.height.saturating_sub(1), "~")?;
        }
        Terminal::execute()?;
        Ok(())
    }

    fn jump_cursor(&mut self) -> Result<ScreenUpdateType, Box<dyn Error>> {
        let neg_2 = self.size.height.saturating_sub(2);
        let render_string: &str = "Jump to: ";
        let mut line = 0_usize;
        Terminal::move_cursor_to(ScreenPosition {
            height: neg_2,
            width: 0,
        })?;

        loop {
            let render_line = if line != 0 {
                format!("{render_string}{line}")
            } else {
                format!("{render_string}")
            };
            Terminal::render_line(neg_2, render_line)?;
            Terminal::execute()?;
            let Ok(read_event) = read() else { continue }; //skipping errors here
            match JumpCommand::try_from(read_event) {
                Ok(command) => match command {
                    JumpCommand::Enter(digit) => {
                        line = std::cmp::max(1, line.saturating_mul(10).saturating_sub(digit));
                    }
                    #[allow(clippy::integer_division)]
                    JumpCommand::Delete => line = if line > 9 { line / 10 } else { 0 },
                    JumpCommand::Move => {
                        // if line > buffer.len(), give buffer len
                        if line < self.buffer.len() {
                            self.cursor_position.height = line.saturating_sub(1);
                        } else {
                            self.move_cursor(Direction::PageDown);
                        };

                        if (self.cursor_position.height
                            > self.size.height.saturating_add(self.screen_offset.height))
                            | (self.cursor_position.height < self.screen_offset.height)
                        {
                            self.screen_offset.handle_offset_screen_snap(
                                &self.cursor_position,
                                &self.size,
                                1,
                                self.buffer.len(),
                            );
                            return Ok(ScreenUpdateType::FullScreen);
                        }

                        self.overwrite_last_line()?;
                        return Ok(ScreenUpdateType::DefaultAction);
                    }
                    JumpCommand::Exit => {
                        self.overwrite_last_line()?;
                        return Ok(ScreenUpdateType::DefaultAction);
                    }
                    JumpCommand::NoAction => continue,
                },
                Err(_) => continue,
            }

            match line {
                0 => {
                    Terminal::render_line(neg_2, &render_string)?;
                }
                _ => {
                    Terminal::render_line(neg_2, &format!("{render_string}{line}"))?;
                }
            }
            Terminal::execute()?;
        }
    }

    fn jump_word(&mut self, dir: Direction) {
        match dir {
            Direction::Right => self.buffer.find_next_word(&mut self.cursor_position),
            Direction::Left => self.buffer.find_prev_word(&mut self.cursor_position),
            _ => {
                #[cfg(debug_assertions)]
                panic!("Invalid direction in jump word");
            } //direction should only be left or right at this point
        };
    }
}
