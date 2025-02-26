use crate::editor::editorcommands::SearchCommand;
use crate::editor::{
    terminal::{Coordinate, Mode, Position, ScreenOffset, ScreenPosition, Size, Terminal},
    view::Buffer,
};
use crossterm::event::read;
use crossterm::style::{Attribute, Color, Print, PrintStyledContent, StyledContent, Stylize};
use std::cmp::min;
use std::collections::HashSet;

pub struct Search {
    index: usize, // index of search positions we are currently on
    cursor_position: Position,
    screen_offset: ScreenOffset,
    highlight: Color,
    text: Color,
    stack: Vec<Vec<Position>>,
    string: String,
    line_indicies: HashSet<usize>,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            index: 0,
            string: String::new(),
            cursor_position: Position::default(),
            screen_offset: ScreenOffset::default(),
            highlight: Color::DarkBlue,
            text: Color::White,
            stack: Vec::new(),
            line_indicies: HashSet::new(),
        }
    }
}

enum IndexResolver {
    Left,
    Mid,
    Right,
}

impl Search {
    pub fn new(pos: Position, offset: ScreenOffset, highlight: Color, text: Color) -> Self {
        Self {
            index: 0,
            string: String::new(),
            cursor_position: pos,
            screen_offset: offset,
            highlight,
            text,
            stack: Vec::new(),
            line_indicies: HashSet::new(),
        }
    }

    // entry
    // no changes to buffer -> immutable reference
    pub fn run(
        &mut self,
        prev_pos: &mut Position,
        prev_offset: &mut ScreenOffset,
        size: &mut Size,
        buffer: &Buffer,
    ) {
        loop {
            // on errors or events that dont matter in this context
            // skip and continue
            self.render(buffer, size);
            let Ok(read_event) = read() else { continue }; //skipping errors here

            match SearchCommand::try_from(read_event) {
                Ok(event) => match event {
                    SearchCommand::Insert(c) => {
                        // add char to search query
                        self.string.push(c);
                        self.stack.push(buffer.search(&self.string));
                        self.index = self.find_relative_start(prev_pos.height).unwrap_or(0);
                        self.set_line_indicies();
                    }
                    SearchCommand::Next => {
                        //snap to next result
                        if !self.stack.is_empty() {
                            let curr_results = self.stack.last().unwrap();
                            self.index = if curr_results.len().saturating_sub(1) > self.index {
                                self.index.saturating_add(1)
                            } else {
                                0
                            };
                        }
                    }
                    SearchCommand::Previous => {
                        //snap to previous result
                        if !self.stack.is_empty() {
                            let curr_results = self.stack.last().unwrap();
                            self.index = if self.index > 0 {
                                self.index.saturating_sub(1)
                            } else {
                                curr_results.len().saturating_sub(1)
                            };
                        }
                    }
                    SearchCommand::RevertState => {
                        //return to pre search screen state
                        self.revert_screen_state(prev_pos, prev_offset);
                        break;
                    }
                    SearchCommand::AssumeState => {
                        //assume current state on screen after search
                        *prev_pos = self.cursor_position;
                        *prev_offset = self.screen_offset;
                        break;
                    }
                    SearchCommand::BackSpace => {
                        // remove char from search query
                        if !self.string.is_empty() {
                            self.string.pop();
                            self.stack.pop();
                            self.index = self.find_relative_start(prev_pos.height).unwrap_or(0);
                            self.set_line_indicies();
                        }
                    }
                    SearchCommand::Resize(new_size) => *size = new_size,
                    SearchCommand::NoAction => continue,
                },
                Err(_) => continue,
            }

            // no search query
            if self.stack.is_empty() {
                self.revert_screen_state(prev_pos, prev_offset);
                continue;
            }

            // no matches on current search query
            if self.stack[self.stack.len().saturating_sub(1)].is_empty() {
                self.revert_screen_state(prev_pos, prev_offset);
                continue;
            }

            //grab the latest search results from the stack
            //get the search index position
            //self.cursor_position = self.stack[self.stack.len() - 1][self.index].clone();
            self.cursor_position = self.stack.last().unwrap()[self.index];

            // if the search position is out of current screen bounds
            // if out width is within 0 - size
            // snap offset left
            if self.cursor_position.width < size.width {
                self.screen_offset.snap_left();
            }
            match self
                .cursor_position
                .max_displacement_from_view(&self.screen_offset, &size, 3)
            {
                0_usize => {}
                1_usize => {
                    self.screen_offset
                        .update_offset_single_move(&self.cursor_position, &size, 3)
                }
                _ => self.screen_offset.handle_offset_screen_snap(
                    &self.cursor_position,
                    &size,
                    3,
                    buffer.len(),
                ),
            }
            /*
                        if !self
                            .cursor_position
                            .height_in_view(&self.screen_offset, size, 2)
                            | !self
                                .cursor_position
                                .width_in_view(&self.screen_offset, size)
                        {
                            self.screen_offset.handle_offset_screen_snap(
                                &self.cursor_position,
                                size,
                                3,
                                buffer.len(),
                            );
                        }
            */
        }
        self.render(buffer, size);
    }

    fn render(&self, buffer: &Buffer, size: &Size) {
        // this largely is the same logic as Editor::refresh_screen
        // maybe that logic should be called out of view to not reproduce code
        if (size.width == 0) | (size.height == 0) {
            return;
        }
        Terminal::hide_cursor().expect("Terminal error");
        Terminal::move_cursor_to(self.screen_offset.to_position()).expect("Terminal error");
        Terminal::clear_screen().expect("Terminal error");

        #[allow(clippy::integer_division)]
        for current_row in self.screen_offset.height
            ..self
                .screen_offset
                .height
                .saturating_add(size.height)
                .saturating_sub(2)
        {
            let relative_row = current_row.saturating_sub(self.screen_offset.height);
            if self.line_indicies.contains(&current_row) {
                self.render_search_line(current_row, buffer, size, self.highlight, self.text);
                continue;
            }

            // buffer should not be empty here
            if let Some(line) = buffer.text.get(current_row) {
                Terminal::render_line(
                    relative_row,
                    line.get_line_subset(
                        self.screen_offset.width
                            ..self.screen_offset.width.saturating_add(size.width),
                    ),
                )
                .expect("Terminal Error");
            } else {
                Terminal::render_line(relative_row, "~").expect("Terminal error");
            }
        }

        self.render_search_string(size);
        Terminal::render_status_line(
            &Mode::Search,
            buffer.is_saved,
            size,
            buffer.filename.as_deref(),
            Some((self.cursor_position.height.saturating_add(1), buffer.len())),
        )
        .expect("Terminal Error");

        Terminal::move_cursor_to(
            self.cursor_position
                .relative_view_position(&self.screen_offset),
        )
        .expect("Terminal Error");
        Terminal::show_cursor().expect("Terminal Error");
        Terminal::execute().expect("Terminal Error");
    }

    #[inline]
    fn revert_screen_state(&mut self, pos: &Position, offset: &ScreenOffset) {
        self.cursor_position = *pos;
        self.screen_offset = *offset;
    }

    fn find_relative_start(&self, curr_height: usize) -> Option<usize> {
        // in the next verion, change this to have better cach locality for the search
        // in most cases this probably does not matter
        let current_positions: Vec<Position> =
            match self.stack.get(self.stack.len().saturating_sub(1)) {
                Some(positions) => positions.clone(),
                None => return None,
            };
        let mut l: usize = 0;
        let mut r: usize = current_positions.len().saturating_sub(1);
        if r <= l {
            return None;
        }

        #[allow(clippy::integer_division)]
        let mut m = (r - l) / 2 + l;
        while l < r {
            if (current_positions[m].height == curr_height)
                | ((current_positions[m.saturating_sub(1)].height < curr_height)
                    & (current_positions[min(
                        m.saturating_add(1),
                        current_positions.len().saturating_sub(1),
                    )]
                    .height
                        > curr_height))
            {
                match Self::resolve_closest(
                    curr_height,
                    current_positions[m.saturating_sub(1)].height,
                    current_positions[m].height,
                    current_positions[min(
                        m.saturating_add(1),
                        current_positions.len().saturating_sub(1),
                    )]
                    .height,
                ) {
                    IndexResolver::Left => return Some(m.saturating_sub(1)),
                    IndexResolver::Mid => return Some(m),
                    IndexResolver::Right => return Some(m.saturating_add(1)),
                }
            } else if current_positions[m].height > curr_height {
                r = m.saturating_sub(1);
            } else {
                l = m.saturating_add(1);
            }

            m = (r - l) / 2 + l;
        }
        None
    }

    fn resolve_closest(curr: usize, left: usize, mid: usize, right: usize) -> IndexResolver {
        // resolves the closests search position to cursor
        // within the range on values returned in binary search
        if curr > mid {
            if right.saturating_sub(curr) < curr.saturating_sub(mid) {
                IndexResolver::Right
            } else {
                IndexResolver::Mid
            }
        } else if curr.saturating_sub(left) < mid.saturating_sub(curr) {
            IndexResolver::Left
        } else {
            IndexResolver::Mid
        }
    }

    #[inline]
    fn render_search_string(&self, size: &Size) {
        let result = Terminal::render_line(
            size.height.saturating_sub(2),
            format!("Search: {}", self.string),
        );

        debug_assert!(result.is_ok(), "Failed to render line");
    }

    fn set_line_indicies(&mut self) {
        if self.stack.is_empty() {
            return;
        }

        self.line_indicies.clear();

        // iter through search hits for current query
        for position in &self.stack[self.stack.len().saturating_sub(1)] {
            self.line_indicies.insert(position.height);
        }
    }

    #[inline]
    fn render_search_line(
        &self,
        line: usize,
        buffer: &Buffer,
        size: &Size,
        search_highlight: Color,
        search_text: Color,
    ) {
        let styled_search: StyledContent<String> = self
            .string
            .clone()
            .with(search_text)
            .on(search_highlight)
            .attribute(Attribute::Bold);

        Terminal::move_cursor_to(ScreenPosition {
            height: line.saturating_sub(self.screen_offset.height),
            width: 0,
        })
        .expect("Terminal Error");
        Terminal::clear_line().expect("Terminal Error");

        let full_line = &buffer.text[line].raw_string;
        let start = self.screen_offset.width;
        let end = min(
            self.screen_offset.width.saturating_add(size.width),
            full_line.len(),
        );
        let current_line = match full_line.get(start..end) {
            Some(text) => text,
            None => return,
        };
        let mut split = current_line.split(&self.string);

        if let Some(first) = split.next() {
            if !current_line.starts_with(&self.string) {
                Terminal::queue_command(Print(first)).expect("Terminal Error");
            }
        };

        for text in split {
            Terminal::queue_command(PrintStyledContent(styled_search.clone()))
                .expect("Terminal Error");
            Terminal::queue_command(Print(text)).expect("Terminal Error");
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn search_find_closest_position() {
        let mut search = Search::default();
        let heights: Vec<usize> = vec![4, 9, 12, 30, 39, 45, 56, 63];
        let mut positions = Vec::new();
        for i in heights.iter() {
            positions.push(Position {
                height: *i,
                width: 0,
                max_width: usize::default(),
            })
        }
        search.stack = vec![positions];
        let mut pos = search.find_relative_start(10);
        assert_eq!(pos.unwrap(), 1);
        pos = search.find_relative_start(15);
        assert_eq!(pos.unwrap(), 2);
        pos = search.find_relative_start(25);
        assert_eq!(pos.unwrap(), 3);
        pos = search.find_relative_start(40);
        assert_eq!(pos.unwrap(), 4);
    }
}
