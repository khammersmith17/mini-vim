use super::terminal::{Position, Size, Terminal};
use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::event::read;
use std::time::Instant;
mod buffer;
use super::editorcommands::{
    Direction, EditorCommand, FileNameCommand, HighlightCommand, JumpCommand, SearchCommand,
    VimModeCommands,
};
use buffer::Buffer;
use std::cmp::min;
pub mod line;
mod theme;
use theme::Theme;
mod search;
use search::Search;
pub mod help;
use help::Help;
mod highlight;
use highlight::{Highlight, HighlightOrientation};

const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
const PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

/// the core logic
pub struct View {
    pub buffer: Buffer,
    pub needs_redraw: bool,
    pub size: Size,
    pub cursor_position: Position,
    pub screen_offset: Position,
    help_indicator: Help,
    search: Search,
    theme: Theme,
    clipboard: ClipboardContext,
    highlight: Highlight,
}

impl Default for View {
    fn default() -> Self {
        Self {
            buffer: Buffer::default(),
            needs_redraw: true,
            size: Terminal::size().unwrap_or_default(),
            cursor_position: Position::default(),
            screen_offset: Position::default(),
            help_indicator: Help::default(),
            search: Search::default(),
            theme: Theme::default(),
            clipboard: ClipboardProvider::new().unwrap(),
            highlight: Highlight::default(),
        }
    }
}

impl View {
    pub fn render(&mut self) {
        if (self.size.width == 0) | (self.size.height == 0) {
            return;
        }
        let screen_cut = if self.help_indicator.render_help | self.search.render_search {
            2
        } else {
            1
        };

        #[allow(clippy::integer_division)]
        for current_row in
            self.screen_offset.height..self.screen_offset.height + self.size.height - screen_cut
        {
            let relative_row = current_row - self.screen_offset.height;
            if self.search.render_search && self.search.line_indicies.contains(&current_row) {
                self.search.render_search_line(
                    current_row,
                    &self.buffer,
                    &self.screen_offset,
                    &self.size,
                    self.theme.search_highlight,
                    self.theme.search_text,
                );
                continue;
            }

            if self.highlight.render & self.highlight.line_range.contains(&current_row) {
                // going to handle rendering these lines with the highlight range
                // want to skip this so we do not render twice
                continue;
            }

            if let Some(line) = self.buffer.text.get(current_row) {
                self.render_line(
                    relative_row,
                    line.get_line_subset(
                        self.screen_offset.width
                            ..self.screen_offset.width.saturating_add(self.size.width),
                    ),
                );
            } else if self.buffer.is_empty() & (current_row == self.size.height / 3) {
                self.render_line(relative_row, &self.get_welcome_message());
            } else {
                self.render_line(relative_row, "~");
            }
        }

        if self.search.render_search {
            self.search.render_search_string(&self.size);
        }

        if self.help_indicator.render_help {
            self.render_help_line(self.size.height, self.size.width);
        }

        // TODO:
        // when in highlight mode consider end
        // when in normal mode consider the position
        if self.highlight.render {
            self.render_file_info(
                self.highlight.end.height - self.screen_offset.height + self.size.height,
            );
        } else {
            self.render_file_info(
                self.cursor_position.height - self.screen_offset.height + self.size.height,
            );
        }

        self.needs_redraw = false;
    }

    fn render_help_line(&mut self, height: usize, width: usize) {
        if self.help_indicator.render_help
            & (Instant::now()
                .duration_since(self.help_indicator.time_began)
                .as_secs()
                < 5)
        {
            let mut render_message = format!(
                "HELP: {} | {} | {} | {} | {} | {}",
                "Ctrl-w = save",
                "Ctrl-q = quit",
                "Ctrl-j = jump-to",
                "Ctrl-f = search",
                "Ctrl-u = snap-up",
                "Ctrl-d = snap-down"
            );
            render_message.truncate(width);
            self.render_line(height.saturating_sub(2), &render_message);
        } else {
            self.help_indicator.render_help = false;
        }
    }

    fn render_file_info(&mut self, height: usize) {
        let saved = if !self.buffer.is_saved {
            "modified"
        } else {
            "saved"
        };
        let filename = match &self.buffer.filename {
            Some(file) => file,
            None => "-",
        };
        let render_message = if !self.buffer.is_empty() {
            format!(
                "Filename: {} | Status: {} | Line: {} / {}",
                filename,
                saved,
                self.cursor_position.height.saturating_add(1),
                self.buffer.len()
            )
        } else {
            format!("Filename: {} | Status: {} | Line: -", filename, saved)
        };

        self.render_line(height.saturating_sub(1), &render_message);
    }

    pub fn render_line<T: std::fmt::Display>(&self, row: usize, line: T) {
        let result = Terminal::render_line(row, line);
        debug_assert!(result.is_ok(), "Failed to render line")
    }
    pub fn resize(&mut self, size: Size) {
        self.size = size;
        self.handle_offset_screen_snap();
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
            match key_code {
                //if not on last line, move down
                //if the next line is shorter, snap to the end of that line
                Direction::Down => {
                    self.cursor_position
                        .down(1, self.buffer.len().saturating_sub(1));
                    self.cursor_position.resolve_width(
                        self.buffer.text[self.cursor_position.height].grapheme_len(),
                    );
                }
                //if we are not in row 0, move up
                //if the line above is shorter than the previous line, snap to the end
                Direction::Up => {
                    self.cursor_position.up(1);
                    self.cursor_position.resolve_width(
                        self.buffer.text[self.cursor_position.height].grapheme_len(),
                    );
                }
                //move left
                //if we are at 0,0 no action
                //if we are at width 0, snap to the right end of the previous line
                //else move left 1
                Direction::Left => {
                    match (
                        self.cursor_position.at_left_edge(),
                        self.cursor_position.at_top(),
                    ) {
                        (true, false) => {
                            self.cursor_position.up(1);
                            self.cursor_position.snap_right(
                                self.buffer.text[self.cursor_position.height].grapheme_len(),
                            );
                            snap = true;
                        }
                        _ => {
                            self.cursor_position.left(1);
                        }
                    }
                }
                //if we are on the last line at the -1 position of the text, do nothing
                //if we are at the end of the line, snap to position 0 on the next line
                //else move right 1 char
                Direction::Right => {
                    let grapheme_len = self.buffer.text[self.cursor_position.height].grapheme_len();
                    let text_height = self.buffer.len().saturating_sub(1);

                    match (
                        self.cursor_position.at_max_width(grapheme_len),
                        self.cursor_position.at_max_height(text_height),
                    ) {
                        (true, false) => {
                            self.cursor_position.down(1, text_height);
                            self.cursor_position.snap_left();
                            snap = true;
                        }
                        _ => self.cursor_position.right(1, grapheme_len),
                    };
                }
                //move to last line, cursor width will stay the same
                Direction::PageDown => {
                    self.cursor_position
                        .page_down(self.buffer.len().saturating_sub(1));
                    snap = true;
                }
                //move to the first line, cursor width stays the same
                Direction::PageUp => {
                    self.cursor_position.page_up();
                    snap = true;
                }
                //move to end of current line
                Direction::End => {
                    self.cursor_position
                        .snap_right(self.buffer.text[self.cursor_position.height].grapheme_len());
                    snap = true;
                }
                //move to start of current line
                Direction::Home => {
                    self.cursor_position.snap_left();
                    snap = true;
                }
            }
            if snap {
                self.handle_offset_screen_snap();
            } else {
                self.update_offset_single_move();
            }
        } else {
            self.cursor_position.page_up();
            self.cursor_position.snap_left();
        }
        self.needs_redraw = true;
    }

    fn insert_char(&mut self, insert_char: char) {
        self.buffer
            .update_line_insert(&mut self.cursor_position, insert_char);

        self.buffer.is_saved = false;
    }

    fn insert_tab(&mut self) {
        self.buffer.insert_tab(&self.cursor_position);
        self.cursor_position.width = self.cursor_position.width.saturating_add(4);
    }

    fn delete_char(&mut self) {
        //get the width of the char being deleted to update the cursor position
        self.buffer.update_line_delete(&mut self.cursor_position);
    }

    pub fn get_file_name(&mut self) {
        // clear_screen and render screen to get file name
        let mut filename_buffer = String::new();
        let mut curr_position: usize = 10;
        self.render_filename_screen(&filename_buffer, curr_position);
        loop {
            /*
                        let read_event = match read() {
                            Ok(event) => event,
                            Err(_) => continue,
                        };

            */
            let Ok(read_event) = read() else { continue };

            match FileNameCommand::try_from(read_event) {
                Ok(event) => match event {
                    FileNameCommand::Insert(c) => {
                        filename_buffer.push(c);
                        curr_position += 1;
                    }
                    FileNameCommand::BackSpace => {
                        filename_buffer.pop();
                        curr_position = std::cmp::max(10, curr_position.saturating_sub(1));
                    }
                    FileNameCommand::SaveFileName => break,
                    FileNameCommand::NoAction => continue,
                },
                _ => continue,
            }

            self.render_filename_screen(&filename_buffer, curr_position);
        }

        self.buffer.assume_file_name(filename_buffer);
        self.needs_redraw = true;
    }

    fn render_filename_screen(&self, curr_filename: &str, curr_position: usize) {
        Terminal::hide_cursor().unwrap();
        Terminal::move_cursor_to(Position {
            height: 0,
            width: 0,
        })
        .unwrap();
        Terminal::clear_screen().unwrap();
        self.render_line(0, &format!("Filename: {}", &curr_filename));
        Terminal::move_cursor_to(Position {
            height: 0,
            width: curr_position,
        })
        .unwrap();
        Terminal::show_cursor().unwrap();
        Terminal::execute().unwrap();
    }

    pub fn handle_event(&mut self, command: EditorCommand) {
        //match the event to the enum value and handle the event accrodingly
        match command {
            EditorCommand::Move(direction) => self.move_cursor(direction),
            EditorCommand::JumpWord(direction) => self.jump_word(direction),
            EditorCommand::Resize(size) => {
                self.resize(size);
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
                let paste_text = self.clipboard.get_contents().unwrap();
                self.buffer
                    .add_text_from_clipboard(paste_text, &mut self.cursor_position);
            }
            EditorCommand::Highlight => {
                self.highlight.render = true;
                self.handle_highlight();
            }
            EditorCommand::Search => {
                if self.help_indicator.render_help {
                    self.help_indicator.render_help = false;
                }
                self.search.render_search = true;
                self.handle_search();
            }
            EditorCommand::Insert(char) => {
                self.insert_char(char);
                self.update_offset_single_move();
            }
            EditorCommand::Tab => self.insert_tab(),
            EditorCommand::JumpLine => self.jump_cursor(),
            EditorCommand::Delete => self.deletion(),
            EditorCommand::NewLine => {
                self.new_line();
                self.handle_offset_screen_snap();
            }
            EditorCommand::Help => {
                self.help_indicator.render_help = true;
                self.help_indicator.time_began = Instant::now();
            }
            EditorCommand::VimMode => self.enter_vim_mode(),
            _ => {}
        }
        self.needs_redraw = true;
    }

    fn enter_vim_mode(&mut self) {
        loop {
            let Ok(read_event) = read() else { continue }; //skipping an error on read cursor action

            match VimModeCommands::try_from(read_event) {
                Ok(event) => match event {
                    VimModeCommands::Move(dir) => match dir {
                        Direction::Right | Direction::Left | Direction::Up | Direction::Down => {
                            self.move_cursor(dir);
                        }
                        _ => continue,
                    },
                    VimModeCommands::Exit => return,
                    VimModeCommands::NoAction => continue, // skipping other
                },
                Err(_) => continue, //ignoring error
            }
            self.render();
        }
    }

    fn new_line(&mut self) {
        let grapheme_len = if !self.buffer.is_empty() {
            self.buffer
                .text
                .get(self.cursor_position.height)
                .expect("Out of bounds error")
                .grapheme_len()
        } else {
            0
        };

        if self.cursor_position.width == grapheme_len {
            self.buffer.new_line(self.cursor_position.height);
        } else {
            self.buffer
                .split_line(self.cursor_position.height, self.cursor_position.width);
        }

        self.cursor_position
            .down(1, self.buffer.len().saturating_sub(1));
        self.cursor_position.width = if self.buffer.is_tab(&Position {
            height: self.cursor_position.height,
            width: 4,
        }) {
            4
        } else {
            0
        };
    }

    fn deletion(&mut self) {
        if self.buffer.is_empty() {
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
                    self.handle_offset_screen_snap();
                }
                _ => {
                    self.buffer.join_line(self.cursor_position.height);
                    self.cursor_position.up(1);
                    self.cursor_position
                        .set_width(self.buffer.text[self.cursor_position.height].grapheme_len());
                }
            },
            _ => {
                self.delete_char();
            }
        };
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
        Terminal::render_line(neg_2, format!("{}", render_string)).unwrap();
        Terminal::execute().unwrap();

        loop {
            let Ok(read_event) = read() else { continue }; //skipping errors here
            match JumpCommand::try_from(read_event) {
                Ok(command) => match command {
                    JumpCommand::Enter(digit) => line = line * 10 + digit,
                    JumpCommand::Delete => line = if line > 9 { line / 10 } else { 0 },
                    JumpCommand::Move => {
                        // if line > buffer.len(), give buffer len
                        if line < self.buffer.len() {
                            self.cursor_position.height = line.saturating_sub(1);
                        } else {
                            self.move_cursor(Direction::PageDown);
                        };

                        if (self.cursor_position.height
                            > self.size.height + self.screen_offset.height)
                            | (self.cursor_position.height < self.screen_offset.height)
                        {
                            self.handle_offset_screen_snap();
                        }
                    }
                    JumpCommand::Exit => return,
                    JumpCommand::NoAction => continue,
                },
                Err(_) => continue,
            }

            match line {
                0 => {
                    let _ = Terminal::render_line(neg_2, &format!("{}", render_string));
                }
                _ => {
                    let _ = Terminal::render_line(neg_2, &format!("{}{}", render_string, line));
                }
            }
            let _ = Terminal::execute();
        }
    }

    fn handle_offset_screen_snap(&mut self) {
        // updates the offset when offset adjustment is > 1
        if self
            .cursor_position
            .below_view(&self.screen_offset, &self.size, 1)
        {
            self.screen_offset.set_height(min(
                self.buffer
                    .text
                    .len()
                    .saturating_sub(self.size.height)
                    .saturating_add(2), // leave space for the file info line
                self.cursor_position
                    .height
                    .saturating_sub(self.size.height)
                    .saturating_add(2),
            ));
            if self.search.render_search | self.help_indicator.render_help {
                self.screen_offset
                    .set_height(self.screen_offset.height.saturating_add(1));
            }
        } else if self.cursor_position.above_view(&self.screen_offset) {
            self.screen_offset
                .set_height(self.cursor_position.height.saturating_sub(1));
        }

        if self.cursor_position.at_top() {
            self.screen_offset.page_up();
        }

        if self.cursor_position.at_left_edge() {
            self.screen_offset.snap_left();
        }

        if self.cursor_position.width >= self.size.width + self.screen_offset.width {
            self.screen_offset.width = self
                .cursor_position
                .width
                .saturating_sub(self.size.width)
                .saturating_add(1);
        } else if self.cursor_position.width < self.screen_offset.width {
            self.screen_offset.left(1);
        }
    }

    fn update_offset_single_move(&mut self) {
        //if cursor moves beyond height + offset -> increment height offset
        if self
            .cursor_position
            .below_view(&self.screen_offset, &self.size, 1)
        {
            self.screen_offset.set_height(min(
                self.screen_offset.height.saturating_add(1),
                self.cursor_position
                    .height
                    .saturating_sub(self.size.height)
                    .saturating_add(2), // space for file info line
            ));
        }
        // if height moves less than the offset -> decrement height
        if self.cursor_position.above_view(&self.screen_offset) {
            self.screen_offset.set_height(self.cursor_position.height);
        }
        //if widith less than offset -> decerement width
        if self.cursor_position.left_of_view(&self.screen_offset) {
            self.screen_offset.set_width(self.cursor_position.width);
        }
        //if width moves outside view by 1 increment
        if self
            .cursor_position
            .right_of_view(&self.screen_offset, &self.size)
        {
            //self.screen_offset.width = self.screen_offset.width.saturating_sub(1);
            self.screen_offset.width = self.screen_offset.width.saturating_add(1);
        }
    }

    fn handle_search(&mut self) {
        self.search
            .set_previous(&self.cursor_position, &self.screen_offset);

        // keep a stack of search positions so
        // only need to compute the positions when the user
        // adds to a search string
        // when the user removes from the search string
        // pop the stack

        loop {
            // on errors or events that dont matter in this context
            // skip and continue
            self.render_search();
            let Ok(read_event) = read() else { continue }; //skipping errors here

            match SearchCommand::try_from(read_event) {
                Ok(event) => match event {
                    SearchCommand::Insert(c) => {
                        // add char to search query
                        self.search.string.push(c);
                        self.search
                            .stack
                            .push(self.buffer.search(&self.search.string));
                        self.search.search_index = match self
                            .search
                            .find_relative_start(&self.search.previous_position.height)
                        {
                            Some(ind) => ind,
                            None => 0,
                        };
                        self.search.set_line_indicies();
                    }
                    SearchCommand::Next => {
                        //snap to next result
                        if !self.search.stack.is_empty() {
                            let curr_results =
                                self.search.stack.get(self.search.stack.len() - 1).unwrap();
                            self.search.search_index = if curr_results.len().saturating_sub(1)
                                > self.search.search_index
                            {
                                self.search.search_index.saturating_add(1)
                            } else {
                                0
                            };
                        }
                    }
                    SearchCommand::Previous => {
                        //snap to previous result
                        if !self.search.stack.is_empty() {
                            let curr_results =
                                self.search.stack.get(self.search.stack.len() - 1).unwrap();
                            self.search.search_index = if self.search.search_index > 0 {
                                self.search.search_index.saturating_sub(1)
                            } else {
                                curr_results.len().saturating_sub(1)
                            };
                        }
                    }
                    SearchCommand::RevertState => {
                        //return to pre search screen state
                        self.revert_screen_state();
                        break;
                    }
                    SearchCommand::AssumeState => {
                        //assume current state on screen after search
                        break;
                    }
                    SearchCommand::BackSpace => {
                        // remove char from search query
                        if !self.search.string.is_empty() {
                            self.search.string.pop();
                            self.search.stack.pop();
                            self.search.search_index = match self
                                .search
                                .find_relative_start(&self.search.previous_position.height)
                            {
                                Some(ind) => ind,
                                None => 0,
                            };
                            self.search.set_line_indicies();
                        }
                    }
                    SearchCommand::Resize(size) => self.resize(size),
                    SearchCommand::NoAction => continue,
                },
                Err(_) => continue,
            }

            // no search query
            if self.search.stack.is_empty() {
                self.revert_screen_state();
                continue;
            }

            // no matches on current search query
            if self.search.stack[self.search.stack.len() - 1].is_empty() {
                self.revert_screen_state();
                continue;
            }

            //grab the latest search results from the stack
            //get the search index position
            self.cursor_position =
                self.search.stack[self.search.stack.len() - 1][self.search.search_index].clone();

            // if the search position is out of current screen bounds
            if !self
                .cursor_position
                .height_in_view(&self.screen_offset, &self.size, 2)
                | !self
                    .cursor_position
                    .width_in_view(&self.screen_offset, &self.size)
            {
                self.handle_offset_screen_snap();
            }
        }
        self.search.clean_up_search();
        self.render();
    }

    fn revert_screen_state(&mut self) {
        self.cursor_position = self.search.previous_position;
        self.screen_offset = self.search.previous_offset;
    }

    fn render_search(&mut self) {
        // this largely is the same logic as Editor::refresh_screen
        // maybe that logic should be called out of view to not reproduce code
        Terminal::hide_cursor().unwrap();
        Terminal::move_cursor_to(self.screen_offset).unwrap();
        Terminal::clear_screen().unwrap();
        self.render();
        Terminal::move_cursor_to(self.cursor_position.view_height(&self.screen_offset)).unwrap();
        Terminal::show_cursor().unwrap();
        Terminal::execute().unwrap();
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

    fn handle_highlight(&mut self) {
        //if buffer is empty -> nothing to highlight
        if self.buffer.is_empty() {
            return;
        }
        self.highlight.end = self.cursor_position.clone();
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
                                    self.highlight
                                        .end
                                        .set_height(min(self.highlight.end.height + 1, max_height));
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
                    self.theme.search_highlight,
                    self.theme.search_text,
                );
            } else {
                self.highlight.multi_line_render(
                    &self.cursor_position,
                    &self.screen_offset,
                    &self.size,
                    &self.buffer,
                    self.theme.search_highlight,
                    self.theme.search_text,
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
            self.copy_text_to_clipboard(copy_string);
        }
        self.highlight.clean_up();
    }

    fn copy_text_to_clipboard(&mut self, content: String) {
        self.clipboard.set_contents(content).unwrap();
    }

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
                        (self.cursor_position.height + 1..self.highlight.end.height).rev()
                    }
                    HighlightOrientation::EndFirst => {
                        (self.highlight.end.height + 1..self.cursor_position.height).rev()
                    }
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
                            width: self.buffer.text[self.cursor_position.height].len() - 1,
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
                            width: self.buffer.text[self.highlight.end.height].len() - 1,
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
}
