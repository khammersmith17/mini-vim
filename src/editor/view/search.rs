use crate::editor::{
    terminal::{Position, Terminal},
    view::Buffer,
};
use crossterm::style::{Attribute, Color, Print, PrintStyledContent, StyledContent, Stylize};
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

impl Search {
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
    pub fn render_search_line(&mut self, line: usize, buffer: &Buffer) {
        //grab the current lint
        //style the search hit
        //render the search hits and plain text
        let styled_search: StyledContent<String> = self
            .string
            .clone()
            .with(Color::Yellow)
            .on(Color::Blue)
            .attribute(Attribute::Bold);

        Terminal::move_cursor_to(Position {
            height: line,
            width: 0,
        })
        .expect("Error moving cursor");

        let current_line = &buffer.text.get(line).expect("Out of bounds").raw_string;

        let mut split = current_line.split(&self.string);
        if let Some(first) = split.next() {
            if !current_line.starts_with(&self.string) {
                Terminal::queue_command(Print(first)).expect("Error queuing command");
            }
        };

        while let Some(text) = split.next() {
            Terminal::queue_command(PrintStyledContent(styled_search.clone()))
                .expect("Error queuing command");
            Terminal::queue_command(Print(text)).expect("Error queuing command");
        }
    }
}
