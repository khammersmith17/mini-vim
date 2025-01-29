use super::editorcommands::{Direction, EditorCommand, FileNameCommand, JumpCommand};
use super::terminal::{Coordinate, Mode, Position, ScreenOffset, Size, Terminal};
use crossterm::event::read;
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

pub const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
pub const PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

const ORIGIN_POSITION: Position = Position {
    height: 0_usize,
    width: 0_usize,
};

/// the core logic
pub struct View {
    pub size: Size,
    pub cursor_position: Position,
    pub screen_offset: ScreenOffset,
    theme: Theme,
    pub needs_redraw: bool,
    pub buffer: Buffer,
}

impl Default for View {
    fn default() -> Self {
        Self {
            buffer: Buffer::default(),
            needs_redraw: true,
            size: Terminal::size().unwrap_or_default(),
            cursor_position: Position::default(),
            screen_offset: ScreenOffset::default(),
            theme: Theme::default(),
        }
    }
}

impl View {
    pub fn render(&mut self) {
        if (self.size.width == 0) | (self.size.height == 0) {
            return;
        }

        #[allow(clippy::integer_division)]
        for current_row in self.screen_offset.height
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

        Terminal::render_status_line(
            Mode::Insert,
            self.buffer.is_saved,
            &self.size,
            self.buffer.filename.as_deref(),
            Some((
                self.cursor_position.height.saturating_add(1),
                self.buffer.len(),
            )),
        )
        .unwrap();

        self.needs_redraw = false;
    }

    #[inline]
    pub fn render_line<T: std::fmt::Display>(row: usize, line: T) {
        let result = Terminal::render_line(row, line);
        debug_assert!(result.is_ok(), "Failed to render line");
    }

    pub fn resize(&mut self, size: Size) {
        self.size = size;
        self.screen_offset.handle_offset_screen_snap(
            &self.cursor_position,
            &self.size,
            1,
            self.buffer.len(),
        );
    }

    pub fn load(&mut self, filename: &str) {
        if let Ok(buffer) = Buffer::load(filename) {
            self.buffer = buffer;
            self.needs_redraw = true;
        } else {
            self.buffer = Buffer::load_named_empty(filename);
        }
    }

    // inlining because it is a rather straight forward computation
    #[inline]
    pub fn move_cursor(&mut self, key_code: Direction) {
        if self.buffer.is_empty() {
            self.cursor_position = ORIGIN_POSITION;
        } else {
            key_code.move_cursor(&mut self.cursor_position, &self.buffer);
        }
    }

    fn insert_char(&mut self, insert_char: char) {
        self.buffer
            .update_line_insert(&mut self.cursor_position, insert_char);

        self.buffer.is_saved = false;
    }

    #[inline]
    fn insert_tab(&mut self) {
        self.buffer.insert_tab(&self.cursor_position);
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
        self.needs_redraw = true;
    }

    fn render_filename_screen(curr_filename: &str, curr_position: usize) {
        Terminal::hide_cursor().unwrap();
        Terminal::move_cursor_to(Position {
            height: 0,
            width: 0,
        })
        .unwrap();
        Terminal::clear_screen().unwrap();
        Self::render_line(0, format!("Filename: {}", &curr_filename));
        Terminal::move_cursor_to(Position {
            height: 0,
            width: curr_position,
        })
        .unwrap();
        Terminal::show_cursor().unwrap();
        Terminal::execute().unwrap();
    }

    pub fn handle_event(&mut self, command: EditorCommand) -> bool {
        //match the event to the enum value and handle the event accrodingly
        //return true to continue false to quit
        //so we can propogate up quit from vim mode
        let mut continue_status: bool = true;
        match command {
            EditorCommand::Move(direction) => {
                self.move_cursor(direction);
                self.check_offset();
            }
            EditorCommand::JumpWord(direction) => self.jump_word(direction),
            EditorCommand::Resize(size) => {
                self.resize(size);
                self.check_offset(); // offset may no longer be on screen
            }
            EditorCommand::Save => {
                if self.buffer.filename.is_none() {
                    self.get_file_name();
                }
                self.buffer.save();
            }
            EditorCommand::Theme => {
                self.theme.set_theme();
            }
            EditorCommand::Paste => {
                let Ok(paste_text) = ClipboardUtils::get_text_from_clipboard() else {
                    return true;
                };
                self.buffer
                    .add_text_from_clipboard(&paste_text, &mut self.cursor_position);
            }
            EditorCommand::Highlight => {
                let mut highlight = Highlight::new(
                    &mut self.cursor_position,
                    self.screen_offset,
                    &mut self.size,
                    &mut self.buffer,
                );
                highlight.run(self.theme.highlight, self.theme.text);
                self.check_offset() // making sure the offset is correct on a delete
            }
            EditorCommand::Search => {
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
            EditorCommand::Insert(char) => {
                self.insert_char(char);
                self.check_offset();
            }
            EditorCommand::Tab => self.insert_tab(),
            EditorCommand::JumpLine => self.jump_cursor(),
            EditorCommand::Delete => self.deletion(),
            EditorCommand::NewLine => {
                self.new_line();
                self.check_offset(); //may need to snap back
            }
            EditorCommand::Help => {
                Help::render_help(&mut self.size);
            }
            EditorCommand::VimMode => {
                let mut vim_mode = VimMode::new(
                    self.cursor_position,
                    self.screen_offset,
                    self.size,
                    &mut self.buffer,
                );
                continue_status = vim_mode.run(
                    &mut self.cursor_position,
                    &mut self.screen_offset,
                    &mut self.size,
                );
            }
            EditorCommand::Quit => return false,
            _ => {}
        }
        self.needs_redraw = true;
        continue_status
    }

    fn new_line(&mut self) {
        let grapheme_len = if self.buffer.is_empty() {
            0
        } else {
            self.buffer.text[self.cursor_position.height].grapheme_len()
        };

        // if at end of current line -> new blank line
        // otherwise move all text right of cursor to new line
        if self.cursor_position.width == grapheme_len {
            self.buffer.new_line(self.cursor_position.height);
        } else {
            self.buffer.split_line(&self.cursor_position);
        }

        self.cursor_position
            .down(1, self.buffer.len().saturating_sub(1));
        // if prev line starts with a tab -> this line starts with a tab
        // TODO:
        // get number of spaces that the prev line starts with
        // floor divide by 4 -> new line starts with this many tabs
        // up to some ceiling
        self.cursor_position.width = if self.buffer.is_tab(&Position {
            height: self.cursor_position.height,
            width: 4,
        }) {
            4
        } else {
            0
        };
        self.check_offset();
    }

    #[inline]
    fn check_offset(&mut self) {
        match self
            .cursor_position
            .max_displacement_from_view(&self.screen_offset, &self.size, 1)
        {
            0 => (),
            1 => self
                .screen_offset
                .update_offset_single_move(&self.cursor_position, &self.size, 1),
            _ => self.screen_offset.handle_offset_screen_snap(
                &self.cursor_position,
                &self.size,
                1,
                self.buffer.len(),
            ),
        }
    }

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
                    // TODO:
                    // figure out where to set the cursor
                    // currently there is odd behavior when shifting the screen
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
        self.check_offset();
    }

    fn jump_cursor(&mut self) {
        let neg_2 = self.size.height.saturating_sub(2);
        let render_string: String = "Jump to: ".into();
        let mut line = 0_usize;
        Terminal::move_cursor_to(Position {
            height: neg_2,
            width: 0,
        })
        .unwrap();
        Terminal::render_line(neg_2, render_string.to_string()).unwrap();
        Terminal::execute().unwrap();

        loop {
            let Ok(read_event) = read() else { continue }; //skipping errors here
            match JumpCommand::try_from(read_event) {
                Ok(command) => match command {
                    JumpCommand::Enter(digit) => {
                        line = line.saturating_mul(10).saturating_sub(digit);
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
                        }
                        return;
                    }
                    JumpCommand::Exit => return,
                    JumpCommand::NoAction => continue,
                },
                Err(_) => continue,
            }

            match line {
                0 => {
                    let _ = Terminal::render_line(neg_2, &render_string);
                }
                _ => {
                    let _ = Terminal::render_line(neg_2, &format!("{render_string}{line}"));
                }
            }
            let _ = Terminal::execute();
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

    /*
    fn handle_highlight(&mut self) {
        //if buffer is empty -> nothing to highlight
        if self.buffer.is_empty() {
            return;
        }
        self.highlight.end = self.cursor_position;
        let max_height = self.buffer.len().saturating_sub(1);
        let previous_offset = self.screen_offset;

        loop {
            let Ok(read_event) = read() else { continue }; //skipping errors here
            match HighlightCommand::try_from(read_event) {
                Ok(event) => match event {
                    HighlightCommand::Move(dir) => match dir {
                        Direction::Right => {
                            let max_width = self.buffer.text[self.highlight.end.height]
                                .grapheme_len()
                                .saturating_sub(1);
                            let at_height = self.highlight.end.at_max_height(max_height);
                            let at_width = self.highlight.end.at_max_width(max_width);
                            match (at_height, at_width) {
                                (true, false) => {
                                    self.highlight.end.set_height(min(
                                        self.highlight.end.height.saturating_add(1),
                                        max_height,
                                    ));
                                    self.highlight.end.snap_left();
                                }
                                (false, false) => self.highlight.end.right(1, max_width),
                                _ => {
                                    // at last possible position
                                }
                            }
                        }
                        Direction::Left => {
                            if self.highlight.end.at_left_edge() & !self.highlight.end.at_top() {
                                self.highlight.end.up(1);
                                self.highlight.end.set_width(
                                    self.buffer.text[self.highlight.end.height]
                                        .grapheme_len()
                                        .saturating_sub(1),
                                );
                            } else {
                                self.highlight.end.left(1);
                            }
                        }
                        Direction::Down => {
                            self.highlight.end.down(1, max_height);
                            if self.highlight.end.at_max_height(max_height) {
                                continue;
                            }
                            self.highlight.end.resolve_width(
                                self.buffer.text[self.highlight.end.height]
                                    .grapheme_len()
                                    .saturating_sub(1),
                            );
                        }
                        Direction::Up => {
                            if self.highlight.end.height == 0 {
                                continue;
                            }
                            self.highlight.end.up(1);
                            self.highlight.end.set_width(min(
                                self.highlight.end.width,
                                self.buffer.text[self.highlight.end.height]
                                    .grapheme_len()
                                    .saturating_sub(1),
                            ));
                        }
                        _ => continue,
                    },
                    HighlightCommand::Copy => {
                        break;
                    }
                    HighlightCommand::Resize(size) => self.resize(size),
                    HighlightCommand::RevertState => {
                        self.screen_offset = previous_offset;
                        self.highlight.clean_up();
                        return;
                    }
                    HighlightCommand::Delete => {
                        if self.cursor_position != self.highlight.end {
                            self.batch_delete();
                        }
                        self.highlight.clean_up();
                        return;
                    }
                    HighlightCommand::NoAction => continue,
                },
                Err(_) => continue,
            }

            self.highlight
                .update_offset(&mut self.screen_offset, &self.size);
            self.highlight.resolve_orientation(&self.cursor_position);
            self.highlight.adjust_range(&self.cursor_position);
            Terminal::hide_cursor().unwrap();
            Terminal::clear_screen().unwrap();
            self.render();

            if self.cursor_position.height == self.highlight.end.height {
                self.highlight.render_single_line(
                    &self.cursor_position,
                    &self.buffer,
                    self.theme.highlight,
                    self.theme.text,
                );
            } else {
                self.highlight.multi_line_render(
                    &self.cursor_position,
                    &self.screen_offset,
                    &self.size,
                    &self.buffer,
                    self.theme.highlight,
                    self.theme.text,
                );
            }

            Terminal::move_cursor_to(self.highlight.end.view_height(&self.screen_offset)).unwrap();
            Terminal::show_cursor().unwrap();
            Terminal::execute().unwrap();
        }

        let copy_string = self
            .highlight
            .generate_copy_str(&self.buffer, &self.cursor_position);

        if !copy_string.is_empty() {
            let res = self.copy_text_to_clipboard(copy_string);
            debug_assert!(res.is_ok());
        }
        self.highlight.clean_up();
    }
    */

    /*
    #[inline]
    fn copy_text_to_clipboard(&mut self, content: String) -> Result<(), Box<dyn Error>> {
        ClipboardUtils::copy_text_to_clipboard(content)?;
        Ok(())
    }
    */

    /*
    fn batch_delete(&mut self) {
        self.highlight.resolve_orientation(&self.cursor_position);

        if self.cursor_position.diff_height(&self.highlight.end) == 0 {
            match self.highlight.or {
                HighlightOrientation::StartFirst => self
                    .buffer
                    .delete_segment(&self.cursor_position, &mut self.highlight.end),
                HighlightOrientation::EndFirst => self
                    .buffer
                    .delete_segment(&self.highlight.end, &mut self.cursor_position),
            }
        } else {
            if self.cursor_position.diff_height(&self.highlight.end) > 1 {
                let range_iter = match self.highlight.or {
                    HighlightOrientation::StartFirst => {
                        (self.cursor_position.height.saturating_add(1)..self.highlight.end.height)
                            .rev()
                    }
                    HighlightOrientation::EndFirst => (self.highlight.end.height.saturating_add(1)
                        ..self.cursor_position.height)
                        .rev(),
                };

                for line in range_iter {
                    self.buffer.pop_line(line);
                }
            }

            //delete everything left of bottom position
            //delete everything right of top position
            //join the lines
            match self.highlight.or {
                HighlightOrientation::StartFirst => {
                    self.highlight
                        .end
                        .set_height(self.cursor_position.height.saturating_add(1));
                    self.buffer.delete_segment(
                        &Position {
                            width: 0,
                            height: self.highlight.end.height,
                        },
                        &mut self.highlight.end,
                    );
                    self.buffer.delete_segment(
                        &self.cursor_position,
                        &mut Position {
                            width: self.buffer.text[self.cursor_position.height]
                                .len()
                                .saturating_sub(1),
                            height: self.cursor_position.height,
                        },
                    );
                    self.buffer.join_line(self.highlight.end.height);
                }
                HighlightOrientation::EndFirst => {
                    self.cursor_position
                        .set_height(self.highlight.end.height.saturating_add(1));
                    self.buffer.delete_segment(
                        &self.cursor_position,
                        &mut Position {
                            width: self.buffer.text[self.highlight.end.height]
                                .len()
                                .saturating_sub(1),
                            height: self.highlight.end.height,
                        },
                    );
                    self.buffer.delete_segment(
                        &Position {
                            width: 0,
                            height: self.cursor_position.height,
                        },
                        &mut self.cursor_position,
                    );
                    self.buffer.join_line(self.cursor_position.height);
                    self.cursor_position.set_position(self.highlight.end);
                }
            }
        }
    }
    */
}
