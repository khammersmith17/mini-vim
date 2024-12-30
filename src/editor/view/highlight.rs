use crate::editor::{view::Buffer, Position, Terminal};
use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::style::{Color, Print, PrintStyledContent, StyledContent, Stylize};
use std::ops::RangeInclusive;

/// type to identify the direction the highlight goes in
#[derive(Copy, Clone)]
pub enum HighlightOrientation {
    StartFirst,
    EndFirst,
}

impl Default for HighlightOrientation {
    fn default() -> HighlightOrientation {
        HighlightOrientation::StartFirst
    }
}

/// seperate the logic for highlight a partial and full line
pub enum HighlightLineType {
    Middle,
    Leading,
    Trailing,
    All,
}

/// type to handle the highlighting and copy logic
pub struct Highlight {
    pub render: bool,
    pub line_range: RangeInclusive<usize>,
    pub or: HighlightOrientation,
}

impl Default for Highlight {
    fn default() -> Highlight {
        Highlight {
            render: false,
            line_range: 0..=0,
            or: HighlightOrientation::default(),
        }
    }
}

impl Highlight {
    pub fn adjust_range(&mut self, pos1: &Position, pos2: &Position) {
        match self.or {
            HighlightOrientation::StartFirst => {
                self.line_range = pos1.height..=pos2.height;
            }
            HighlightOrientation::EndFirst => {
                self.line_range = pos2.height..=pos1.height;
            }
        }
    }
    pub fn resolve_orientation(&mut self, pos1: &Position, pos2: &Position) {
        if pos1.height == pos2.height {
            if pos1.width <= pos2.width {
                self.or = HighlightOrientation::StartFirst;
            } else {
                self.or = HighlightOrientation::EndFirst;
            }
            return;
        }
        if pos1.height < pos2.height {
            self.or = HighlightOrientation::StartFirst;
        } else {
            self.or = HighlightOrientation::EndFirst;
        }
    }
    pub fn generate_copy_str(buffer: &Buffer, start: &Position, end: &Position) -> String {
        let mut copy_string = String::new();
        if start.height == end.height {
            let line_len = buffer.text[start.height].raw_string.len() - 1;
            let line_string = &buffer.text[start.height].raw_string;
            let slice: String = if end.width == line_len {
                line_string[start.width..].to_string()
            } else {
                line_string[start.width..end.width].to_string()
            };
            copy_string.push_str(&slice);
        } else {
            copy_string.push_str(&buffer.text[start.height].raw_string[start.width..]);
            copy_string.push_str("\n");
            for h in start.height + 1..end.height {
                copy_string.push_str(&buffer.text[h].raw_string);
                copy_string.push_str("\n");
            }
            copy_string.push_str(&buffer.text[end.height].raw_string[..end.width]);
        }
        copy_string
    }

    pub fn render_highlight_line(
        line: &str,
        height: usize,
        h_range: RangeInclusive<usize>,
        ctx: HighlightLineType,
        h_color: Color,
        t_color: Color,
    ) {
        Terminal::move_cursor_to(Position { height, width: 0 }).unwrap();
        Terminal::clear_line().unwrap();

        let segment_to_highlight: String = line[h_range.clone()].to_string();
        let highlight_seg: StyledContent<String> =
            segment_to_highlight.clone().with(t_color).on(h_color);

        // order in which elements are rendered
        // on the line based on line type
        match ctx {
            HighlightLineType::All => {
                Terminal::queue_command(PrintStyledContent(highlight_seg)).unwrap();
            }
            HighlightLineType::Leading => {
                Terminal::queue_command(PrintStyledContent(highlight_seg)).unwrap();
                Terminal::queue_command(Print(&line[(h_range.end() + 1)..])).unwrap();
            }
            HighlightLineType::Trailing => {
                Terminal::queue_command(Print(&line[..*h_range.start()])).unwrap();
                Terminal::queue_command(PrintStyledContent(highlight_seg)).unwrap();
            }
            HighlightLineType::Middle => {
                Terminal::queue_command(Print(&line[..*h_range.start()])).unwrap();
                Terminal::queue_command(PrintStyledContent(highlight_seg)).unwrap();
                Terminal::queue_command(Print(&line[(h_range.end() + 1)..])).unwrap();
            }
        }
    }

    pub fn copy_text_to_clipboard(ctx: &mut ClipboardContext, content: String) {
        ctx.set_contents(content).unwrap();
    }
}
