use super::line::{GraphemeWidth, Line, TextFragment};
use crate::editor::Position;
use std::fs::{read_to_string, OpenOptions};
use std::io::prelude::*;
use std::io::{Error, LineWriter};

#[derive(Default)]
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

    pub fn add_text_from_clipboard(&mut self, paste_text: String, pos: &mut Position) {
        let mut buff_len = if !self.is_empty() { self.len() - 1 } else { 0 };
        for (i, line_str) in paste_text.lines().enumerate() {
            if i != 0 {
                pos.height += 1;
                pos.width = 0;
            }

            if pos.height > buff_len {
                self.text.push(Line::from(line_str));
                buff_len += 1;
                continue;
            }
            for c in line_str.chars() {
                self.update_line_insert(pos, c);
            }
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
                for width in resulting_widths.iter() {
                    positions.push(Position {
                        width: *width,
                        height: i as usize,
                    })
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
            position.height -= 1;
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
            position.height += 1;

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
        let mut widths: Vec<usize> = vec![running_len.clone()];
        for slice in string_split {
            let current_len = slice.len();
            running_len = running_len
                .saturating_add(search_len)
                .saturating_add(current_len);
            widths.push(running_len.clone());
        }
        widths.pop();
        widths
    }

    pub fn assume_file_name(&mut self, filename: String) {
        self.filename = Some(filename);
    }

    pub fn save(&mut self) {
        //write buffer to disk
        let filename = match &self.filename {
            Some(name) => name,
            None => panic!("Trying to save without filename being set"),
        };
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(filename)
            .expect("Error opening file");
        let mut file = LineWriter::new(file);
        for line in self.text.iter() {
            let text_line = line.to_string();
            file.write_all(text_line.as_bytes())
                .expect("Error on write");
            file.write_all(b"\n").expect("Error entering new line");
        }
        self.is_saved = true;
    }

    pub fn insert_tab(&mut self, pos: &Position) {
        if self.is_empty() {
            let new_line = Line {
                string: Vec::new(),
                raw_string: String::new(),
            };
            self.text.push(new_line);
        }

        for _ in pos.width..pos.width.saturating_add(4) {
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
        if !self.is_empty() {
            self.text
                .get_mut(pos.height)
                .expect("Error getting mut line")
                .string
                .insert(pos.width, new_fragment);
        } else {
            self.text.push(Line::from(insert_char.to_string().as_str()));
        }
        self.text
            .get_mut(pos.height)
            .expect("Out of bounds error")
            .generate_raw_string();
        self.is_saved = false;
        pos.width += move_width;
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
                    if fragment.grapheme != " ".to_string() {
                        return false;
                    }
                }
            }
            None => return false,
        }

        return true;
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
            self.insert_tab(&Position {
                height: line_index.saturating_add(1),
                width: 0,
            });
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
        right_pos.width += 1;
        while right_pos.width > left_pos.width {
            self.update_line_delete(right_pos);
        }
    }

    pub fn pop_line(&mut self, line_index: usize) {
        self.text.remove(line_index);
    }
}
