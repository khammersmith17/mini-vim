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

impl GraphemeWidth {
    fn saturating_add(&self, other: usize) -> usize {
        match self {
            Self::Half => other.saturating_add(1),
            Self::Full => other.saturating_add(2),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextFragment {
    grapheme: String,
    pub render_width: GraphemeWidth,
    replacement_text: Option<char>,
}

#[derive(Debug, Clone)]
struct TextFragmentError;

impl fmt::Display for TextFragmentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error using TextFragment")
    }
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
}

impl Line {
    pub fn to_string(&self) -> String {
        let mut return_string = String::new();

        for fragment in self.string.iter() {
            match fragment.replacement_text {
                Some(char) => return_string.push(char),
                None => return_string.push_str(&fragment.grapheme),
            };
        }
        return_string
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

        Self { string: line }
    }

    pub fn get(&self, range: Range<usize>) -> String {
        if range.start >= range.end {
            return String::new();
        }

        let mut result_string = String::new();
        let mut current_position = 0;
        for fragment in &self.string {
            let end = fragment.render_width.saturating_add(current_position);
            if current_position > range.end {
                break;
            }

            if end > range.start {
                if end > range.end || current_position < range.start {
                    result_string.push('~');
                } else if let Some(char) = fragment.replacement_text {
                    result_string.push(char);
                } else {
                    result_string.push_str(&fragment.grapheme)
                }
            }

            current_position = end;
        }

        result_string
    }

    pub fn is_empty(&self) -> bool {
        if self.string.len() == 0 {
            return true;
        } else {
            return false;
        }
    }
}
