use std::convert::TryFrom;
use std::fmt;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq, Clone, Debug)]
pub enum GraphemeWidth {
    Half,
    Full,
}

#[derive(Debug, Clone)]
struct TextFragmentError;

impl fmt::Display for TextFragmentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error using TextFragment")
    }
}

#[derive(Debug, Clone)]
pub struct TextFragment {
    pub grapheme: String,
    pub render_width: GraphemeWidth,
    replacement_text: Option<char>,
}

impl TryFrom<&str> for TextFragment {
    type Error = String;
    fn try_from(new_item: &str) -> Result<Self, Self::Error> {
        let width = new_item.width();
        let fragment_width = match width {
            0 | 1 => GraphemeWidth::Half,
            _ => GraphemeWidth::Full,
        };

        let replacement = match width {
            0 => {
                let trimmed = new_item.trim();
                match trimmed {
                    "\t" => Some(' '),
                    _ => {
                        let control = trimmed
                            .chars()
                            .map(char::is_control)
                            .reduce(|a, b| a | b)
                            .expect("Error in reduction");
                        let replace_val = if control {
                            '|'
                        } else if trimmed.is_empty() {
                            '*'
                        } else {
                            '.'
                        };
                        Some(replace_val)
                    }
                }
            }
            _ => None,
        };

        Ok(Self {
            grapheme: new_item.to_string(),
            render_width: fragment_width,
            replacement_text: replacement,
        })
    }
}

#[derive(Clone, Default)]
pub struct Line {
    pub string: Vec<TextFragment>,
    pub raw_string: String,
}

impl fmt::Display for Line {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let result: String = self
            .string
            .iter()
            .map(|fragment| match fragment.replacement_text {
                Some(char) => char.to_string().clone(),
                None => fragment.grapheme.clone(),
            })
            .collect();

        write!(formatter, "{result}")
    }
}

/*
impl Default for Line {
    fn default() -> Line {
        Line {
            string: Vec::new(),
            raw_string: String::new(),
        }
    }
}
*/

impl Line {
    pub fn generate_raw_string(&mut self) {
        self.raw_string = self.to_string();
    }

    pub fn len(&self) -> usize {
        self.string.len()
    }

    pub fn grapheme_len(&self) -> usize {
        if self.string.is_empty() {
            return 0;
        }

        let len: usize = self
            .string
            .iter()
            .map(|fragment| match fragment.render_width {
                GraphemeWidth::Full => 2,
                GraphemeWidth::Half => 1,
            })
            .reduce(|a, b| a + b)
            .expect("Error in reduce");

        len
    }

    pub fn get_next_word(&self, start: usize) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        let mut space_pos = start;

        for (i, c) in self.raw_string.as_bytes().iter().skip(start).enumerate() {
            space_pos = i.saturating_add(start);
            if *c == 32 {
                break;
            }
        }
        space_pos = space_pos.saturating_add(1);

        for (i, c) in self
            .raw_string
            .as_bytes()
            .iter()
            .skip(space_pos)
            .enumerate()
        {
            if *c != 32 {
                return Some(i.saturating_add(space_pos));
            }
        }

        None
    }

    pub fn next_word_spillover(&self) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        let chars = self.raw_string.as_bytes();
        if chars[0] != 32 {
            return Some(0);
        }

        for (i, c) in chars.iter().skip(1).enumerate() {
            if *c != 32 {
                return Some(i.saturating_add(1));
            }
        }

        None
    }

    pub fn get_prev_word(&self, start: usize) -> Option<usize> {
        if self.is_empty() {
            return None;
        }

        let mut pos = start;
        for c in self.raw_string.as_bytes()[..start].iter().rev() {
            pos = pos.saturating_sub(1);
            if *c == 32 {
                break;
            }
        }

        if (pos == start) | (pos == 0_usize) {
            return None;
        }

        pos = pos.saturating_sub(1);
        for c in self.raw_string.as_bytes()[..pos].iter().rev() {
            pos = pos.saturating_sub(1);
            if *c != 32 {
                return Some(pos.saturating_add(1));
            }
        }

        None
    }

    pub fn get_prev_word_spillover(&self) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        let len = self.raw_string.len().saturating_sub(1);
        let bytes = self.raw_string.as_bytes();
        if bytes[len] != 32 {
            return Some(len);
        }
        for (i, c) in bytes.iter().rev().enumerate().skip(1) {
            if *c != 32 {
                return Some(i.saturating_sub(1));
            }
        }

        None
    }

    pub fn from(line_str: &str) -> Self {
        let line = line_str
            .graphemes(true)
            .map(|grapheme| {
                let line_width = grapheme.width();
                let grapheme_width = match line_width {
                    0 | 1 => GraphemeWidth::Half,
                    _ => GraphemeWidth::Full,
                };
                let replacement = match line_width {
                    0 => {
                        let trimmed = grapheme.trim();
                        match trimmed {
                            "\t" => Some(' '),
                            _ => {
                                let control = trimmed
                                    .chars()
                                    .map(char::is_control)
                                    .reduce(|a, b| a | b)
                                    .expect("Error in reduction");
                                let replace_val = if control {
                                    '|'
                                } else if trimmed.is_empty() {
                                    '*'
                                } else {
                                    '.'
                                };
                                Some(replace_val)
                            }
                        }
                    }
                    _ => None,
                };
                TextFragment {
                    grapheme: grapheme.to_string(),
                    render_width: grapheme_width,
                    replacement_text: replacement,
                }
            })
            .collect();

        Self {
            string: line,
            raw_string: line_str.to_string(),
        }
    }

    pub fn get_line_subset(&self, range: Range<usize>) -> Line {
        if range.start > self.grapheme_len() {
            return Line::default();
        }
        let end = std::cmp::min(range.end, self.string.len());
        let new_line = self
            .string
            .get(range.start..end)
            .expect("Out of bounds error");

        Line {
            string: new_line.to_vec(),
            raw_string: String::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.string.len() == 0
    }
}
