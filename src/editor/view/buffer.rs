use super::line::{GraphemeWidth, Line, TextFragment};
use crate::editor::view::{Coordinate, Position};
use std::fs::{read_to_string, OpenOptions};
use std::io::{Error, LineWriter, Write};

#[derive(Default, Clone)]
pub struct Buffer {
    pub text: Vec<Line>,
    pub filename: Option<String>,
    pub is_saved: bool,
}

impl Buffer {
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn add_text_from_clipboard(&mut self, paste_text: &str, pos: &mut Position) {
        let mut buff_len = if self.is_empty() {
            self.len().saturating_sub(1)
        } else {
            0
        };
        for (i, line_str) in paste_text.lines().enumerate() {
            if i != 0 {
                pos.height = pos.height.saturating_add(1);
                pos.width = 0;
            }

            if pos.height > buff_len {
                self.text.push(Line::from(line_str));
                buff_len = buff_len.saturating_add(1);
                continue;
            }
            for c in line_str.chars() {
                self.update_line_insert(pos, c);
            }
        }
    }

    pub fn load_named_empty(filename: &str) -> Buffer {
        Self {
            text: Vec::new(),
            filename: Some(filename.to_string()),
            is_saved: false,
        }
    }

    pub fn load(filename: &str) -> Result<Buffer, Error> {
        let file_contents = read_to_string(filename)?;
        let mut text = Vec::new();
        for line in file_contents.lines() {
            text.push(Line::from(line));
        }

        Ok(Self {
            text,
            filename: Some(filename.to_string()),
            is_saved: true,
        })
    }

    pub fn search(&self, search_str: &str) -> Vec<Position> {
        //change to return a vector of positions of search results
        let mut positions: Vec<Position> = Vec::new();

        for (i, line) in self.text.iter().enumerate() {
            if line.raw_string.contains(search_str) {
                let resulting_widths = self.find_search_widths(search_str, i);
                for width in resulting_widths {
                    positions.push(Position { width, height: i });
                }
            }
        }
        positions
    }

    pub fn find_prev_word(&self, position: &mut Position) {
        // find the prev word
        // start at current line
        // if the prev word is only the following line,
        // then jump to the prev  word on the following line
        // use line to find the cursor width
        if self.is_empty() {
            return;
        }

        if let Some(new_width) = self.text[position.height].get_prev_word(position.width) {
            position.width = new_width;
            return;
        }
        while position.height > 0 {
            position.height = position.height.saturating_sub(1);
            if let Some(new_width) = self.text[position.height].get_prev_word_spillover() {
                position.width = new_width;
                return;
            }
        }

        position.width = 0;
    }

    pub fn find_next_word(&self, position: &mut Position) {
        // find the next word
        // start at current line
        // if the next word is only the following line,
        // then jump to the next word on the following line
        // use line to find the cursor width
        if self.is_empty() {
            return;
        }

        if let Some(new_width) = self.text[position.height].get_next_word(position.width) {
            position.width = new_width;
            return;
        }

        // here look for the next char following a space
        // go to next line until we reach EOF
        while position.height < self.text.len().saturating_sub(1) {
            position.height = position.height.saturating_add(1);

            if let Some(new_width) = self.text[position.height].next_word_spillover() {
                position.width = new_width;
                return;
            }
        }
        position.width = self.text[position.height].grapheme_len();
    }

    fn find_search_widths(&self, search_str: &str, line_index: usize) -> Vec<usize> {
        let mut string_split = self
            .text
            .get(line_index)
            .expect("Out of bounds error")
            .raw_string
            .split(search_str);
        let search_len = search_str.len();
        let first = string_split.next().expect("No split results");
        let mut running_len = first.len();
        let mut widths: Vec<usize> = vec![running_len];
        for slice in string_split {
            let current_len = slice.len();
            running_len = running_len
                .saturating_add(search_len)
                .saturating_add(current_len);
            widths.push(running_len);
        }
        widths.pop();
        widths
    }

    pub fn assume_file_name(&mut self, filename: String) {
        self.filename = Some(filename);
    }

    pub fn save(&mut self) {
        //write buffer to disk
        let Some(filename) = &self.filename else {
            panic!("Trying to save without filename being set")
        };
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(filename)
            .expect("Error opening file");
        let mut file = LineWriter::new(file);
        for line in &self.text {
            let text_line = line.to_string();
            file.write_all(text_line.as_bytes())
                .expect("Error on write");
            file.write_all(b"\n").expect("Error entering new line");
        }
        self.is_saved = true;
    }

    pub fn insert_tab(&mut self, pos: &Position, num_tabs: usize) {
        if self.is_empty() {
            let new_line = Line {
                string: Vec::new(),
                raw_string: String::new(),
            };
            self.text.push(new_line);
        }

        for _ in pos.width..pos.width.saturating_add(num_tabs * 4) {
            self.text
                .get_mut(pos.height)
                .expect("Out of bounds")
                .string
                .push(TextFragment::try_from(" ").expect("Error generating new fragment"));
        }

        self.text
            .get_mut(pos.height)
            .expect("Out of bounds error")
            .generate_raw_string();
    }

    pub fn update_line_insert(&mut self, pos: &mut Position, insert_char: char) {
        //take current vec<TextFragment> at height
        //insert new char
        //generate a new vec<TextFragment from new string
        //replace current with new vec
        //return the cursor position update
        let new_fragment: TextFragment = TextFragment::try_from(insert_char.to_string().as_str())
            .expect("Error getting new fragment");
        let move_width = match new_fragment.render_width {
            GraphemeWidth::Half => 1,
            GraphemeWidth::Full => 2,
        };
        if self.is_empty() {
            self.text.push(Line::from(insert_char.to_string().as_str()));
        } else {
            self.text
                .get_mut(pos.height)
                .expect("Error getting mut line")
                .string
                .insert(pos.width, new_fragment);
        }
        self.text
            .get_mut(pos.height)
            .expect("Out of bounds error")
            .generate_raw_string();
        self.is_saved = false;
        pos.width = pos.width.saturating_add(move_width);
    }

    pub fn update_line_delete(&mut self, pos: &mut Position) {
        // pop out the char we want to removed
        // return the render_width of that char
        if self.is_tab(pos) {
            for i in (pos.width.saturating_sub(4)..pos.width).rev() {
                self.text
                    .get_mut(pos.height)
                    .expect("Out of bounds error")
                    .string
                    .remove(i);
            }
            pos.left(4);
            return;
        }
        let removed_char = self
            .text
            .get_mut(pos.height)
            .expect("Out of bounds error")
            .string
            .remove(pos.width.saturating_sub(1));
        self.text
            .get_mut(pos.height)
            .expect("Out of bounds error")
            .generate_raw_string();
        self.is_saved = false;
        let diff = match removed_char.render_width {
            GraphemeWidth::Half => 1,
            GraphemeWidth::Full => 2,
        };
        pos.left(diff);
    }

    pub fn is_tab(&self, pos: &Position) -> bool {
        if pos.width < 4 {
            return false;
        }
        let fragments_to_check = &self
            .text
            .get(pos.height)
            .expect("Out of bounds")
            .string
            .get(pos.width.saturating_sub(4)..pos.width);
        match fragments_to_check {
            Some(frags) => {
                for fragment in frags.iter().rev() {
                    if fragment.grapheme != *" ".to_string() {
                        return false;
                    }
                }
            }
            None => return false,
        }

        true
    }

    pub fn num_tabs(&self, index: usize) -> usize {
        let bytes = self.text[index].raw_string.as_bytes();
        let len = bytes.len();
        let mut i = 5;
        while i < len && bytes[i] == 32 {
            i += 1;
        }

        let i = i >> 2; //cheaper divide by 4 and get integer divide free
        i
    }

    pub fn new_line(&mut self, line_index: usize) {
        if self.is_empty() {
            self.text.push(Line {
                string: Vec::new(),
                raw_string: String::new(),
            });
        }
        self.text.insert(
            line_index.saturating_add(1),
            Line {
                string: Vec::new(),
                raw_string: String::new(),
            },
        );

        if self.is_tab(&Position {
            height: line_index,
            width: 4,
        }) {
            let num_tabs = self.num_tabs(line_index);
            self.insert_tab(
                &Position {
                    height: line_index.saturating_add(1),
                    width: 0,
                },
                num_tabs,
            );
        }

        self.is_saved = false;
    }

    pub fn split_line(&mut self, pos: &Position) {
        let new_line = self
            .text
            .get(pos.height)
            .expect("Out of bounds error")
            .string
            .get(pos.width..)
            .expect("Out of bounds error");

        self.text.insert(
            pos.height.saturating_add(1),
            Line {
                string: new_line.to_vec(),
                raw_string: String::new(),
            },
        );

        self.text
            .get_mut(pos.height)
            .expect("Out of bounds error")
            .string
            .truncate(pos.width);

        self.text
            .get_mut(pos.height.saturating_add(1))
            .expect("Out of bounds error")
            .generate_raw_string();

        self.is_saved = false;
    }

    pub fn join_line(&mut self, line_index: usize) {
        let mut current_line = self
            .text
            .get(line_index)
            .expect("Out of bounds error")
            .string
            .clone();

        self.text.remove(line_index);

        self.text
            .get_mut(line_index.saturating_sub(1))
            .expect("Out of bounds error")
            .string
            .append(&mut current_line);

        self.is_saved = false;

        self.text
            .get_mut(line_index.saturating_sub(1))
            .expect("Out of bounds error")
            .generate_raw_string();
    }

    pub fn delete_segment(&mut self, left_pos: &Position, right_pos: &mut Position) {
        //delete from right to left
        right_pos.width = right_pos.width.saturating_add(1);
        while right_pos.width > left_pos.width {
            self.update_line_delete(right_pos);
        }
    }

    #[inline]
    pub fn pop_line(&mut self, line_index: usize) {
        self.text.remove(line_index);
    }

    pub fn begining_of_current_word(&self, pos: &mut Position) {
        if self.is_empty() {
            return;
        }
        if let Some(new) = self.text[pos.height].begining_of_current_word(pos.width) {
            pos.width = new;
            return;
        }

        while pos.height >= 1 {
            pos.height = pos.height.saturating_sub(1);
            if let Some(new) = self.text[pos.height].begining_of_current_word_spillover() {
                pos.width = new;
                return;
            }
        }

        pos.height = 0;
        pos.width = 0;
    }

    pub fn begining_of_next_word(&self, pos: &mut Position) {
        if self.is_empty() {
            return;
        }
        if let Some(new) = self.text[pos.height].begining_of_next_word(pos.width) {
            pos.width = new;
            return;
        }

        let max = self.len().saturating_sub(1);
        while pos.height < max {
            pos.height = pos.height.saturating_add(1);
            if let Some(new) = self.text[pos.height].begining_of_next_word_spillover() {
                pos.width = new;
                return;
            }
        }
        // if we are here we are at the end
        pos.width = self.text.last().unwrap().grapheme_len();
    }

    pub fn end_of_current_word(&self, pos: &mut Position) {
        if self.is_empty() {
            return;
        }
        if let Some(new) = self.text[pos.height].end_of_current_word(pos.width) {
            pos.width = new;
            return;
        }

        let max_height = self.len().saturating_sub(1);
        while pos.height < max_height {
            pos.height = pos.height.saturating_add(1);
            if let Some(thing) = self.text[pos.height].end_of_current_word_spillover() {
                pos.width = thing;
                return;
            }
        }

        pos.width = self.text.last().unwrap().grapheme_len().saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_of_current_word() {
        let line1 = Line::from("I have a bunch of text");
        let line2 = Line::from("This is a bunch more text");
        let lines = vec![line1, line2];
        let buff = Buffer {
            text: lines,
            filename: None,
            is_saved: true,
        };

        let mut pos = Position {
            height: 0,
            width: 3,
        };

        buff.end_of_current_word(&mut pos);

        assert_eq!(
            pos,
            Position {
                height: 0,
                width: 5
            }
        );
    }

    #[test]
    fn end_of_current_word_spillover() {
        let line1 = Line::from("I have a bunch of text ");
        let line2 = Line::from("This is a bunch more text");
        let lines = vec![line1, line2];
        let buff = Buffer {
            text: lines,
            filename: None,
            is_saved: true,
        };

        let mut pos = Position {
            height: 0,
            width: 22,
        };
        buff.end_of_current_word(&mut pos);

        assert_eq!(
            pos,
            Position {
                height: 1,
                width: 3
            }
        );
    }

    #[test]
    fn end_of_current_word_end() {
        let line1 = Line::from("I have a bunch of text ");
        let line2 = Line::from("This is a bunch more text");
        let lines = vec![line1, line2];
        let buff = Buffer {
            text: lines,
            filename: None,
            is_saved: true,
        };

        let mut pos = Position {
            height: 1,
            width: 22,
        };
        buff.end_of_current_word(&mut pos);

        assert_eq!(
            pos,
            Position {
                height: 1,
                width: 24
            }
        );
    }

    #[test]
    fn begining_of_current_word() {
        let line1 = Line::from("I have a bunch of text ");
        let line2 = Line::from("This is a bunch more text");
        let lines = vec![line1, line2];
        let buff = Buffer {
            text: lines,
            filename: None,
            is_saved: true,
        };

        let mut pos = Position {
            height: 0,
            width: 4,
        };
        buff.begining_of_current_word(&mut pos);

        assert_eq!(
            pos,
            Position {
                height: 0,
                width: 2
            }
        );
    }

    #[test]
    fn begining_of_current_word_origin() {
        let line1 = Line::from("  I have a bunch of text ");
        let line2 = Line::from("This is a bunch more text");
        let lines = vec![line1, line2];
        let buff = Buffer {
            text: lines,
            filename: None,
            is_saved: true,
        };

        let mut pos = Position {
            height: 0,
            width: 2,
        };
        buff.begining_of_current_word(&mut pos);

        assert_eq!(
            pos,
            Position {
                height: 0,
                width: 0
            }
        );
    }

    #[test]
    fn num_tabs() {
        let line1 = Line::from("              I have a bunch of text ");
        let line2 = Line::from("This is a bunch more text");
        let lines = vec![line1, line2];
        let buff = Buffer {
            text: lines,
            filename: None,
            is_saved: true,
        };
        assert_eq!(buff.num_tabs(0), 3);
    }
}
