use super::line::Line;
use std::fs::read_to_string;
use std::io::Error;

#[derive(Default)]
pub struct Buffer {
    pub text: Vec<Line>,
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

        Ok(Self { text })
    }
}
