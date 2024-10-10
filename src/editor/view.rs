use super::terminal::{Position, Size, Terminal};
use crossterm::event::{read, Event, KeyCode, KeyEvent};
mod buffer;
use super::editorcommands::{Direction, EditorCommand};
use buffer::Buffer;
use std::cmp::{max, min};
mod line;

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
        for current_row in self.screen_offset.height..self.screen_offset.height + self.size.height {
            let relative_row = current_row - self.screen_offset.height;
            if let Some(line) = self.buffer.text.get(current_row) {
                self.render_line(
                    relative_row,
                    &line.get(
                        self.screen_offset.width
                            ..self.screen_offset.width.saturating_add(self.size.width),
                    ),
                );
            } else if self.buffer.is_empty() && current_row == self.size.height / 3 {
                self.render_line(relative_row, &self.get_welcome_message());
            } else {
                self.render_line(relative_row, "~");
            }
        }

        self.needs_redraw = false;
    }

    fn render_line(&self, row: usize, line: &str) {
        let result = Terminal::print_line(row, line);
        debug_assert!(result.is_ok(), "Failed to render line");
    }
    pub fn resize(&mut self, size: Size) {
        self.size = size;
        let Size { height, width } = size;
        self.handle_offset_screen_snap(height, width);
    }

    pub fn load(&mut self, filename: &str) {
        if let Ok(buffer) = Buffer::load(filename) {
            self.buffer = buffer;
            self.needs_redraw = true;
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
        let range = self.screen_offset.width
            ..min(
                self.screen_offset.width.saturating_add(self.size.width),
                welcome_message.len(),
            );
        welcome_message = match welcome_message.get(range) {
            Some(text) => text.to_string(),
            None => "".to_string(),
        };
        welcome_message
    }

    pub fn move_cursor(&mut self, key_code: Direction) {
        if !self.buffer.is_empty() {
            let mut snap = false;
            let Size { height, width } = Terminal::size().unwrap();
            match key_code {
                //if not on last line, move down
                //if the next line is shorter, snap to the end of that line
                Direction::Down => {
                    self.cursor_position.height = min(
                        self.cursor_position.height.saturating_add(1),
                        self.buffer.text.len().saturating_sub(1),
                    );
                    self.cursor_position.width = min(
                        self.cursor_position.width,
                        self.buffer
                            .text
                            .get(self.cursor_position.height)
                            .expect("Out of bounds error")
                            .grapheme_len(),
                    );
                }
                //if we are not in row 0, move up
                //if the line above is shorter than the previous line, snap to the end
                Direction::Up => {
                    self.cursor_position.height =
                        max(self.cursor_position.height.saturating_sub(1), 0);
                    self.cursor_position.width = min(
                        self.cursor_position.width,
                        self.buffer
                            .text
                            .get(self.cursor_position.height)
                            .expect("Out of bounds error")
                            .grapheme_len(),
                    );
                }
                //move left
                //if we are at 0,0 no action
                //if we are at width 0, snap to the right end of the previous line
                //else move left 1
                Direction::Left => {
                    if self.cursor_position.width == 0 && self.cursor_position.height > 0 {
                        self.cursor_position.height =
                            max(self.cursor_position.height.saturating_sub(1), 0);
                        self.cursor_position.width = self
                            .buffer
                            .text
                            .get(self.cursor_position.height)
                            .expect("Out of bounds error")
                            .grapheme_len();
                        snap = true;
                    } else {
                        self.cursor_position.width = self.cursor_position.width.saturating_sub(1);
                    }
                }
                //if we are on the last line at the -1 position of the text, do nothing
                //if we are at the end of the line, snap to position 0 on the next line
                //else move right 1 char
                Direction::Right => {
                    let grapheme_len = self
                        .buffer
                        .text
                        .get(self.cursor_position.height)
                        .expect("Out of bounds error")
                        .grapheme_len();

                    let text_height = self.buffer.text.len().saturating_sub(1);

                    if self.cursor_position.width == grapheme_len
                        && self.cursor_position.height < text_height
                    {
                        self.cursor_position.height = self.cursor_position.height.saturating_add(1);
                        self.cursor_position.width = 0;
                        snap = true;
                    } else {
                        self.cursor_position.width =
                            min(self.cursor_position.width.saturating_add(1), grapheme_len);
                    }
                }
                //move to last line, cursor width will stay the same
                Direction::PageDown => {
                    self.cursor_position.height = self.buffer.text.len().saturating_sub(1);
                    snap = true;
                }
                //move to the first line, cursor width stays the same
                Direction::PageUp => {
                    self.cursor_position.height = 0;
                    snap = true;
                }
                //move to end of current line
                Direction::End => {
                    self.cursor_position.width = self
                        .buffer
                        .text
                        .get(self.cursor_position.height)
                        .expect("index Error")
                        .grapheme_len();
                    snap = true;
                }
                //move to start of current line
                Direction::Home => {
                    self.cursor_position.width = 0;
                    snap = true;
                }
            }
            if snap {
                self.handle_offset_screen_snap(height, width);
            } else {
                self.update_offset_single_move(height, width);
            }
        } else {
            self.cursor_position.width = 0;
            self.cursor_position.height = 0;
        }
        self.needs_redraw = true;
    }

    fn insert_char(&mut self, insert_char: char) {
        let new_char_width = self.buffer.update_line_insert(
            self.cursor_position.height,
            self.cursor_position.width,
            insert_char,
        );

        self.cursor_position.width = self.cursor_position.width.saturating_add(new_char_width);
    }

    fn delete_char(&mut self) {
        //get the width of the char being deleted to update the cursor position
        let removed_char_width = self
            .buffer
            .update_line_delete(self.cursor_position.height, self.cursor_position.width);

        self.cursor_position.width = self
            .cursor_position
            .width
            .saturating_sub(removed_char_width);
    }

    pub fn get_file_name(&mut self) {
        //clear the screen
        //move the cursor to 0,0
        //start a repl loop
        //grab test user inputs to be file name
        //when the user presses enter
        let mut filename_buffer = Vec::new();
        loop {
            Terminal::hide_cursor().expect("Error hiding cursor");
            Terminal::move_cursor_to(Position {
                height: 0,
                width: 0,
            })
            .expect("Error moving cursor to start");
            Terminal::clear_screen().expect("Error clearing screen");

            let current_filename: String = filename_buffer.iter().collect();
            let to_render = format!("Filename: {}", &current_filename);
            let new_position = to_render.len();

            match read() {
                Ok(event) => {
                    match event {
                        Event::Key(KeyEvent { code, .. }) => match code {
                            KeyCode::Char(letter) => {
                                filename_buffer.push(letter);
                            }
                            KeyCode::Backspace => {
                                filename_buffer.pop();
                            }
                            KeyCode::Enter => break,
                            _ => {
                                //skipping all other keycode events
                            }
                        },
                        _ => {
                            //skipping all other events
                        }
                    }
                }

                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not handle event: {err}");
                    }
                }
            }

            self.render_line(0, &to_render);
            Terminal::move_cursor_to(Position {
                height: 0,
                width: new_position,
            })
            .expect("Error moving cursor");
            Terminal::show_cursor().expect("Error showing cursor");
            Terminal::execute().expect("Error flushing std buffer");
        }
        let filename: String = filename_buffer.iter().collect();

        self.buffer.assume_file_name(filename);
        self.needs_redraw = true;
    }

    pub fn handle_event(&mut self, command: EditorCommand) {
        //match the event to the enum value and handle the event accrodingly
        let Size { height, width } = Terminal::size().expect("Error getting size");
        match command {
            EditorCommand::Move(direction) => self.move_cursor(direction),
            EditorCommand::Resize(size) => {
                self.resize(size);
            }
            EditorCommand::Save => {
                if self.buffer.filename.is_none() {
                    self.get_file_name();
                }
                self.buffer.save();
            }
            EditorCommand::Insert(char) => {
                self.insert_char(char);
                self.update_offset_single_move(height, width);
            }
            EditorCommand::Delete => {
                //todo add logic for when a line is empty
                match self.cursor_position.width {
                    0 => {
                        if self.cursor_position.height == 0 {
                        } else if self
                            .buffer
                            .text
                            .get(self.cursor_position.height)
                            .expect("Out of bounds error")
                            .is_empty()
                        {
                            self.buffer.text.remove(self.cursor_position.height);
                            self.cursor_position.height =
                                self.cursor_position.height.saturating_sub(1);
                            self.cursor_position.width = self
                                .buffer
                                .text
                                .get(self.cursor_position.height)
                                .expect("Out of bounds error")
                                .grapheme_len();
                            self.handle_offset_screen_snap(height, width);
                        } else {
                            let new_width = self
                                .buffer
                                .text
                                .get(self.cursor_position.height.saturating_sub(1))
                                .expect("Out of bounds error")
                                .grapheme_len();
                            self.buffer.join_line(self.cursor_position.height);
                            self.cursor_position.height =
                                self.cursor_position.height.saturating_sub(1);
                            self.cursor_position.width = new_width;
                        }
                    }
                    _ => {
                        self.delete_char();
                    }
                };
            }
            EditorCommand::NewLine => {
                let grapheme_len = self
                    .buffer
                    .text
                    .get(self.cursor_position.height)
                    .expect("Out of bounds error")
                    .grapheme_len();
                if self.cursor_position.width == grapheme_len {
                    self.buffer.new_line(self.cursor_position.height);
                } else {
                    self.buffer
                        .split_line(self.cursor_position.height, self.cursor_position.width);
                }

                self.cursor_position.height = self.cursor_position.height.saturating_add(1);
                self.cursor_position.width = 0;
                self.handle_offset_screen_snap(height, width);
            }
            _ => {}
        }
        self.needs_redraw = true;
    }

    fn handle_offset_screen_snap(&mut self, height: usize, width: usize) {
        if self.cursor_position.height >= height + self.screen_offset.height {
            self.screen_offset.height = min(
                self.buffer
                    .text
                    .len()
                    .saturating_sub(height)
                    .saturating_add(1),
                self.cursor_position
                    .height
                    .saturating_sub(height)
                    .saturating_add(1),
            );
        }

        if self.cursor_position.height == 0 {
            self.screen_offset.height = 0;
        }

        if self.cursor_position.width == 0 {
            self.screen_offset.width = 0;
        }

        if self.cursor_position.width >= width + self.screen_offset.width {
            self.screen_offset.width = self
                .cursor_position
                .width
                .saturating_sub(width)
                .saturating_add(1);
        }
    }
    fn update_offset_single_move(&mut self, height: usize, width: usize) {
        //if cursor moves beyond height + offset -> increment height
        if self.cursor_position.height >= height + self.screen_offset.height {
            self.screen_offset.height = min(
                self.screen_offset.height.saturating_add(1),
                self.cursor_position
                    .height
                    .saturating_sub(height)
                    .saturating_add(1),
            );
        }
        // if height moves less than the offset -> decrement height
        if self.cursor_position.height <= self.screen_offset.height {
            self.screen_offset.height = self.cursor_position.height;
        }
        //if widith less than offset -> decerement width
        if self.cursor_position.width < self.screen_offset.width {
            self.screen_offset.width = self.cursor_position.width;
        }
        // if new position is greater than offset, offset gets current_width - screen width
        // this better handles snapping the cursor to the end of the line
        if self.cursor_position.width >= width + self.screen_offset.width {
            //self.screen_offset.width = self.screen_offset.width.saturating_sub(1);
            self.screen_offset.width = self.screen_offset.width.saturating_add(1);
        }
    }
}
