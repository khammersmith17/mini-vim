use std::convert::TryFrom;
use std::fmt;
use std::ops::{Range, RangeInclusive};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const UPPERCASE_ASCII_RANGE: RangeInclusive<u8> = 65..=90;
const LOWERCASE_ASCII_RANGE: RangeInclusive<u8> = 97..=122;

fn is_alpha(val: u8) -> bool {
    // including underscores here
    UPPERCASE_ASCII_RANGE.contains(&val) || LOWERCASE_ASCII_RANGE.contains(&val) || val == 95
}

#[derive(PartialEq, Clone, Debug)]
pub enum GraphemeWidth {
    Half,
    Full,
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
                if trimmed == "\t" {
                    Some(' ')
                } else {
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
            _ => None,
        };

        Ok(Self {
            grapheme: new_item.to_owned(),
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
                GraphemeWidth::Full => 2usize,
                GraphemeWidth::Half => 1usize,
            })
            .sum::<usize>();

        len
    }

    pub fn get_next_word(&self, start: usize) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        let mut space_pos = start;

        for (i, c) in self.raw_string.as_bytes().iter().skip(start).enumerate() {
            space_pos = i.saturating_add(start);
            if !is_alpha(*c) {
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
            if !is_alpha(*c) {
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
        if !is_alpha(chars[0]) {
            return Some(0);
        }

        for (i, c) in chars.iter().skip(1).enumerate() {
            if !is_alpha(*c) {
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
            if !is_alpha(*c) {
                break;
            }
        }

        if (pos == start) | (pos == 0_usize) {
            return None;
        }

        pos = pos.saturating_sub(1);
        for c in self.raw_string.as_bytes()[..pos].iter().rev() {
            pos = pos.saturating_sub(1);
            if !is_alpha(*c) {
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
        if !is_alpha(bytes[len]) {
            return Some(len);
        }
        for (i, c) in bytes.iter().rev().enumerate().skip(1) {
            if !is_alpha(*c) {
                return Some(i.saturating_sub(1));
            }
        }

        None
    }

    pub fn begining_of_next_word(&self, pos: usize) -> Option<usize> {
        // if we are one a non alpha char (_ included)
        // then we find the next alpha char
        // if we are on an alpha char
        // we find the next non alpha char
        // then find the next alpha char
        // if neither are satisfied -> None
        let bytes = self.raw_string.as_bytes();
        if is_alpha(bytes[pos]) {
            // currently at alpha char
            // find next non alpha char
            let Some(next) = Self::forward_from_alpha(pos, bytes) else {
                return None;
            };

            let new = Self::forward_from_non_alpha(next, bytes);
            if new.is_some() {
                new
            } else {
                None
            }
        } else {
            let Some(next) = Self::forward_from_non_alpha(pos, bytes) else {
                return None;
            };

            let new = Self::forward_from_alpha(next, bytes);
            if new.is_some() {
                new
            } else {
                None
            }
        }
    }

    pub fn begining_of_next_word_spillover(&self) -> Option<usize> {
        let bytes = self.raw_string.as_bytes();
        if self.is_empty() {
            return None;
        }
        if is_alpha(bytes[0]) {
            return Some(0);
        }

        if let Some(new) = Self::forward_from_non_alpha(0, bytes) {
            return Some(new.saturating_sub(1));
        }
        None
    }

    #[inline]
    fn forward_from_alpha(pos: usize, str_bytes: &[u8]) -> Option<usize> {
        // find next non alpha char
        // if not found -> None
        for (i, c) in str_bytes[pos..].iter().enumerate() {
            if !is_alpha(*c) {
                return Some(pos.saturating_add(i));
            }
        }
        None
    }

    #[inline]
    fn forward_from_non_alpha(pos: usize, str_bytes: &[u8]) -> Option<usize> {
        for (i, c) in str_bytes[pos..].iter().enumerate() {
            if is_alpha(*c) {
                return Some(pos.saturating_add(i));
            }
        }
        None
    }

    pub fn begining_of_current_word_spillover(&self) -> Option<usize> {
        // if the end is alpha
        // then find next alpha
        // if end is not alpha
        // find the next alpha then find the next non alpha
        let len: usize = self.raw_string.len().saturating_sub(1);
        let bytes = self.raw_string.as_bytes();
        if is_alpha(bytes[len]) {
            if let Some(new) = Self::backward_from_alpha(len, bytes) {
                return Some(new.saturating_add(1));
            }
        } else if let Some(new) = Self::backward_from_non_alpha(len, bytes) {
            return Some(new.saturating_add(1));
        }

        None
    }

    pub fn begining_of_current_word(&self, pos: usize) -> Option<usize> {
        // if char is alpha and the char to the left is not
        // look for next alpha twice
        // if char is non alpha
        // look for next alpha then next non alpha
        // if char is alpha and char to the left is alpha
        // look for next non alpha
        if self.is_empty() {
            return None;
        }
        let bytes = self.raw_string.as_bytes();
        // making sure we are not at the begining of the line
        // if the current pos an alphabet char and is the char to the lest if a current alphabet
        // char
        match (
            is_alpha(bytes[pos]),
            pos > 0 && is_alpha(bytes[pos.saturating_sub(1)]),
        ) {
            (true, true) => {
                // look for next non alpha char
                if let Some(new) = Self::backward_from_alpha(pos, bytes) {
                    return Some(new.saturating_add(1));
                }
                None
            }
            (true, false) => {
                // find next alpha twice
                // start at 1 left
                // find the next alpha char
                let Some(next_left_alpha) =
                    Self::backward_from_non_alpha(pos.saturating_sub(1), bytes)
                else {
                    return None;
                };

                if let Some(new_pos) = Self::backward_from_alpha(next_left_alpha, bytes) {
                    return Some(new_pos.saturating_add(1));
                }
                if is_alpha(bytes[0]) {
                    Some(0)
                } else {
                    None
                }
            }
            _ => {
                // find next alpha
                // find next non alpha
                let Some(temp) = Self::backward_from_non_alpha(pos, bytes) else {
                    return None;
                };
                if let Some(new) = Self::backward_from_alpha(pos, bytes) {
                    return Some(new.saturating_add(1));
                }
                Some(temp)
            }
        }
    }

    pub fn end_of_current_word(&self, pos: usize) -> Option<usize> {
        // if pos is alpha and right is alpha
        // look for next non alpha and return -1
        // if pos is alpha and pos + 1 is not alpha
        // start at pos + 1 then find next alpha
        // then find next alpha again
        let len = self.raw_string.len().saturating_sub(1);
        if pos == len {
            return None;
        }
        let bytes = self.raw_string.as_bytes();
        // making sure we are not ad the current end
        // if the current char is an alphabet char and checking the char to the right
        let new: Option<usize> = match (
            is_alpha(bytes[pos]),
            pos < self.raw_string.len().saturating_sub(1) && is_alpha(bytes[pos.saturating_add(1)]),
        ) {
            (true, true) => {
                // find next non alpha and return pos - 1
                Self::forward_from_alpha(pos, bytes)
            }
            (true, false) => {
                // find next alpha from pos + 1, then next non alpha
                if let Some(temp) = Self::forward_from_non_alpha(pos.saturating_add(1), bytes) {
                    Self::forward_from_alpha(temp, bytes)
                } else {
                    None
                }
            }
            (false, true) => Self::forward_from_alpha(pos.saturating_add(1), bytes),
            _ => {
                if let Some(temp) = Self::forward_from_non_alpha(pos, bytes) {
                    Self::forward_from_alpha(temp, bytes)
                } else {
                    None
                }
            }
        };

        if let Some(new_pos) = new {
            return Some(new_pos.saturating_sub(1));
        }

        None
    }

    pub fn end_of_current_word_spillover(&self) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        let bytes = self.raw_string.as_bytes();
        let mut pos = 0;

        if !is_alpha(bytes[0]) {
            if let Some(new) = Self::forward_from_non_alpha(pos, bytes) {
                pos = new;
            } else {
                return None;
            }
        }
        if let Some(new) = Self::forward_from_alpha(pos, bytes) {
            return Some(new.saturating_sub(1));
        }
        None
    }

    #[inline]
    fn backward_from_alpha(pos: usize, str_bytes: &[u8]) -> Option<usize> {
        // iterate backward to find the first non alpha char left
        for (i, c) in str_bytes[..pos].iter().rev().enumerate() {
            if !is_alpha(*c) {
                return Some(pos.saturating_sub(i).saturating_sub(1));
            }
        }
        None
    }

    #[inline]
    fn backward_from_non_alpha(pos: usize, str_bytes: &[u8]) -> Option<usize> {
        // iterate backward to find first
        for (i, c) in str_bytes[..pos].iter().rev().enumerate() {
            if is_alpha(*c) {
                return Some(pos.saturating_sub(i).saturating_sub(1));
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
                        if trimmed == "\t" {
                            Some(' ')
                        } else {
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
                    _ => None,
                };
                TextFragment {
                    grapheme: grapheme.to_owned(),
                    render_width: grapheme_width,
                    replacement_text: replacement,
                }
            })
            .collect();

        Self {
            string: line,
            raw_string: line_str.to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_helper() {
        let line = Line::from("I have a bunch: of text. variable_name too");
        let bytes = line.raw_string.as_bytes();
        // testing next non alpha
        // starting at u, trying to get to ':'
        assert_eq!(Line::forward_from_alpha(10, bytes).unwrap(), 14);
        // starting at : and trying to get to t
        assert_eq!(Line::forward_from_non_alpha(14, bytes).unwrap(), 16);
    }

    #[test]
    fn test_begining_of_next_word() {
        let line = Line::from("I have a bunch: of text. variable_name too");
        // starting at a end at b
        assert_eq!(line.begining_of_next_word(7).unwrap(), 9);
        // testing word with _
        assert_eq!(line.begining_of_next_word(25).unwrap(), 39);
        // testing None
        assert!(line.begining_of_next_word(39).is_none());
    }

    #[test]
    fn backward_alpha_helper() {
        let line = Line::from("I have a bunch: of text. variable_name too");
        let bytes = line.raw_string.as_bytes();

        assert_eq!(Line::backward_from_alpha(12, bytes), Some(8));
        assert_eq!(Line::backward_from_non_alpha(15, bytes), Some(13));
    }

    #[test]
    fn begining_of_current_word() {
        let line = Line::from("I have a bunch: of text. variable_name too");

        assert_eq!(line.begining_of_current_word(5), Some(2));
        // testing spillover
        assert_eq!(line.begining_of_current_word(0), None);
        // testing from non alpha start
        // 14, 9
        assert_eq!(line.begining_of_current_word(14), Some(9));
        //assert_eq!(line.begining_of_current_word(2), None);
        let line2 = Line::from("  I have a bunch: of text. variable_name too");
        assert_eq!(line2.begining_of_current_word(2), None);
    }

    #[test]
    fn begining_of_current_word_spillover() {
        let line = Line::from("I have a bunch: of text. variable_name too");
        let len = line.raw_string.len().saturating_sub(1);
        assert_eq!(
            line.begining_of_current_word_spillover(),
            Some(len.saturating_sub(2))
        );
    }
}
