use crate::editor::{
    terminal::{Position, Size, Terminal},
    view::Buffer,
};
use crossterm::style::{Attribute, Color, Print, PrintStyledContent, StyledContent, Stylize};
use std::cmp::min;
use std::collections::HashSet;

pub struct Search {
    pub render_search: bool,
    pub search_index: usize,
    pub string: String,
    pub previous_position: Position,
    pub previous_offset: Position,
    pub stack: Vec<Vec<Position>>,
    pub line_indicies: HashSet<usize>,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            render_search: false,
            search_index: 0,
            string: String::new(),
            previous_position: Position::default(),
            previous_offset: Position::default(),
            stack: Vec::new(),
            line_indicies: HashSet::new(),
        }
    }
}

enum SearchResolver {
    Left,
    Mid,
    Right,
}

impl Search {
    pub fn find_relative_start(&self, curr_height: &usize) -> Option<usize> {
        // binary search to find the closest search result to pre search cursor position
        // returns Some when there is a search result
        // returns None otherwise
        // None is a catch all, we should always have a closest position
        let current_positions: Vec<Position> =
            match self.stack.get(self.stack.len().saturating_sub(1)) {
                Some(positions) => positions.to_vec(),
                None => return None,
            };
        let mut l: usize = 0;
        let mut r: usize = current_positions.len().saturating_sub(1);
        if r <= l {
            return None;
        }

        let mut m = (r - l) / 2 + l;
        while l < r {
            if (current_positions[m].height == *curr_height)
                | ((current_positions[m.saturating_sub(1)].height < *curr_height)
                    & (current_positions[min(m + 1, current_positions.len() - 1)].height
                        > *curr_height))
            {
                match Self::resolve_closest(
                    *curr_height,
                    current_positions[m - 1].height,
                    current_positions[m].height,
                    current_positions[m + 1].height,
                ) {
                    SearchResolver::Left => return Some(m - 1),
                    SearchResolver::Mid => return Some(m),
                    SearchResolver::Right => return Some(m + 1),
                }
            } else if current_positions[m].height > *curr_height {
                r = m.saturating_sub(1);
            } else {
                l = m + 1;
            }
            m = (r - l) / 2 + l;
        }
        return None;
    }

    fn resolve_closest(curr: usize, left: usize, mid: usize, right: usize) -> SearchResolver {
        // resolves the closests search position to cursor
        // within the range on values returned in binary search
        if curr > mid {
            let res = if right - curr < curr - mid {
                SearchResolver::Right
            } else {
                SearchResolver::Mid
            };
            return res;
        } else {
            let res = if curr - left < mid - curr {
                SearchResolver::Left
            } else {
                SearchResolver::Mid
            };
            return res;
        }
    }

    pub fn render_search_string(&self, size: &Size) {
        let result = Terminal::render_line(
            size.height.saturating_sub(2),
            &format!("Search: {}", self.string),
        );

        debug_assert!(result.is_ok(), "Failed to render line")
    }

    pub fn set_line_indicies(&mut self) {
        // getting the line indexes where there are search hits
        let curr_positions = self
            .stack
            .get(self.stack.len().saturating_sub(1))
            .expect("Stack is empty");

        self.line_indicies.clear();

        for position in curr_positions.iter() {
            self.line_indicies.insert(position.height);
        }
    }

    pub fn clean_up_search(&mut self) {
        self.string.clear();
        self.stack.clear();
        self.line_indicies.clear();
    }

    pub fn render_search_line(
        &mut self,
        line: usize,
        buffer: &Buffer,
        offset: &Position,
        size: &Size,
        search_highlight: Color,
        search_text: Color,
    ) {
        //grab the current lint
        //style the search hit
        //render the search hits and plain text
        let styled_search: StyledContent<String> = self
            .string
            .clone()
            .with(search_text)
            .on(search_highlight)
            .attribute(Attribute::Bold);

        Terminal::move_cursor_to(Position {
            height: line.saturating_sub(offset.height),
            width: 0,
        })
        .unwrap();
        Terminal::clear_line().unwrap();

        let full_line = &buffer.text.get(line).unwrap().raw_string;
        let start = offset.width;
        let end = min(offset.width.saturating_add(size.width), full_line.len());
        let current_line = full_line.get(start..end).unwrap();
        let mut split = current_line.split(&self.string);

        if let Some(first) = split.next() {
            if !current_line.starts_with(&self.string) {
                Terminal::queue_command(Print(first)).unwrap();
            }
        };

        while let Some(text) = split.next() {
            Terminal::queue_command(PrintStyledContent(styled_search.clone())).unwrap();
            Terminal::queue_command(Print(text)).unwrap();
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
            })
        }
        search.stack = vec![positions];
        let mut pos = search.find_relative_start(&10);
        assert_eq!(pos.unwrap(), 1);
        pos = search.find_relative_start(&15);
        assert_eq!(pos.unwrap(), 2);
        pos = search.find_relative_start(&25);
        assert_eq!(pos.unwrap(), 3);
        pos = search.find_relative_start(&40);
        assert_eq!(pos.unwrap(), 4);
    }
}
