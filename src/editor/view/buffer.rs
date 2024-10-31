use super::line::{GraphemeWidth, Line, TextFragment};
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

    pub fn search(&self, search_str: &str, indicies: &mut Vec<usize>) {
        indicies.clear();

        for (i, line) in self.text.iter().enumerate().rev() {
            if line.raw_string.contains(search_str) {
                indicies.push(i);
            }
        }
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

    pub fn insert_tab(&mut self, line_index: usize, width_index: usize) {
        if self.is_empty() {
            let new_line = Line {
                string: Vec::new(),
                raw_string: String::new(),
            };
            self.text.push(new_line);
        }

        for _ in width_index..width_index.saturating_add(4) {
            self.text
                .get_mut(line_index)
                .expect("Out of bounds")
                .string
                .push(TextFragment::try_from(" ").expect("Error generating new fragment"));
        }

        self.text
            .get_mut(line_index)
            .expect("Out of bounds error")
            .generate_raw_string();
    }

    pub fn update_line_insert(
        &mut self,
        line_index: usize,
        width_index: usize,
        insert_char: char,
    ) -> usize {
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
                .get_mut(line_index)
                .expect("Error getting mut line")
                .string
                .insert(width_index, new_fragment);
        } else {
            self.text.push(Line::from(insert_char.to_string().as_str()));
        }
        self.text
            .get_mut(line_index)
            .expect("Out of bounds error")
            .generate_raw_string();
        self.is_saved = false;
        move_width
    }

    pub fn update_line_delete(&mut self, line_index: usize, width_index: usize) -> usize {
        // pop out the char we want to removed
        // return the render_width of that char
        if self.is_tab(line_index, width_index) {
            for i in (width_index.saturating_sub(4)..width_index).rev() {
                self.text
                    .get_mut(line_index)
                    .expect("Out of bounds error")
                    .string
                    .remove(i);
            }
            return 4;
        }
        let removed_char = self
            .text
            .get_mut(line_index)
            .expect("Out of bounds error")
            .string
            .remove(width_index.saturating_sub(1));
        self.text
            .get_mut(line_index)
            .expect("Out of bounds error")
            .generate_raw_string();
        self.is_saved = false;
        match removed_char.render_width {
            GraphemeWidth::Half => 1,
            GraphemeWidth::Full => 2,
        }
    }

    fn is_tab(&self, line_index: usize, width_index: usize) -> bool {
        if width_index < 4 {
            return false;
        }
        let fragments_to_check = &self
            .text
            .get(line_index)
            .expect("Out of bounds")
            .string
            .get(width_index.saturating_sub(4)..width_index)
            .expect("Out of bounds error");
        for fragment in fragments_to_check.iter().rev() {
            if fragment.grapheme != " ".to_string() {
                return false;
            }
        }

        return true;
    }

    pub fn new_line(&mut self, line_index: usize) {
        self.text.insert(
            line_index.saturating_add(1),
            Line {
                string: Vec::new(),
                raw_string: String::new(),
            },
        );
        self.is_saved = false;
    }

    pub fn split_line(&mut self, line_index: usize, width_index: usize) {
        let new_line = self
            .text
            .get(line_index)
            .expect("Out of bounds error")
            .string
            .get(width_index..)
            .expect("Out of bounds error");
        self.text.insert(
            line_index.saturating_add(1),
            Line {
                string: new_line.to_vec(),
                raw_string: String::new(),
            },
        );
        self.text
            .get_mut(line_index)
            .expect("Out of bounds error")
            .string
            .truncate(width_index);

        self.text
            .get_mut(line_index.saturating_add(1))
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
}
