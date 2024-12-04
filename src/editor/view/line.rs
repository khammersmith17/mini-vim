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
                            .map(|char| char.is_control())
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

#[derive(Clone)]
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

impl Line {
    pub fn generate_raw_string(&mut self) {
        self.raw_string = self.to_string();
    }

    pub fn grapheme_len(&self) -> usize {
        if self.string.len() == 0 {
            return 0;
        }

        let len = self
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
                                    .map(|char| char.is_control())
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
        let end = std::cmp::min(range.end, self.string.len());
        let new_line = self
            .string
            .get(range.start..end)
            .expect("Out of bounds error");

        return Line {
            string: new_line.to_vec(),
            raw_string: String::new(),
        };
    }

    pub fn is_empty(&self) -> bool {
        if self.string.len() == 0 {
            return true;
        } else {
            return false;
        }
    }
}
