use crate::editor::{
    terminal::{Position, Size, Terminal},
    view::Buffer,
};
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
// TODO:
// add the end position of the highlight as a member
// refactor the changes to update the member
pub struct Highlight {
    pub render: bool,
    pub line_range: RangeInclusive<usize>,
    pub or: HighlightOrientation,
    pub end: Position,
}

impl Default for Highlight {
    fn default() -> Highlight {
        Highlight {
            render: false,
            line_range: 0..=0,
            or: HighlightOrientation::default(),
            end: Position::default(),
        }
    }
}

impl Highlight {
    pub fn clean_up(&mut self) {
        self.render = false;
        self.line_range = 0..=0;
        self.or = HighlightOrientation::default();
        self.end = Position::default();
    }

    pub fn adjust_range(&mut self, pos1: &Position) {
        match self.or {
            HighlightOrientation::StartFirst => {
                self.line_range = pos1.height..=self.end.height;
            }
            HighlightOrientation::EndFirst => {
                self.line_range = self.end.height..=pos1.height;
            }
        }
    }

    pub fn resolve_orientation(&mut self, pos1: &Position) {
        if pos1.height == self.end.height {
            if pos1.width <= self.end.width {
                self.or = HighlightOrientation::StartFirst;
            } else {
                self.or = HighlightOrientation::EndFirst;
            }
            return;
        }
        if pos1.height < self.end.height {
            self.or = HighlightOrientation::StartFirst;
        } else {
            self.or = HighlightOrientation::EndFirst;
        }
    }
    pub fn generate_copy_str(&self, buffer: &Buffer, start: &Position) -> String {
        let mut copy_string = String::new();
        if start.height == self.end.height {
            let line_len = buffer.text[start.height].raw_string.len() - 1;
            let line_string = &buffer.text[start.height].raw_string;
            let slice: String = if self.end.width == line_len {
                line_string[start.width..].to_string()
            } else {
                line_string[start.width..self.end.width].to_string()
            };
            copy_string.push_str(&slice);
        } else {
            copy_string.push_str(&buffer.text[start.height].raw_string[start.width..]);
            copy_string.push_str("\n");
            for h in start.height + 1..self.end.height {
                copy_string.push_str(&buffer.text[h].raw_string);
                copy_string.push_str("\n");
            }
            copy_string.push_str(&buffer.text[self.end.height].raw_string[..self.end.width + 1]);
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

    pub fn update_offset(&self, offset: &mut Position, size: &Size) {
        // adding a method to handle the offset when
        // the end goes off screen of the highlight
        // block goes off screen
        // taken from View::update_offset_single_move
        // with different parameters to update the highlight end
        if self.end.height >= (size.height + offset.height).saturating_sub(1) {
            offset.height = std::cmp::min(
                offset.height.saturating_add(1),
                self.end
                    .height
                    .saturating_sub(size.height)
                    .saturating_add(2), // space for file info line
            );
        }
        // if height moves less than the offset -> decrement height
        if self.end.height <= offset.height {
            offset.height = self.end.height;
        }
        //if widith less than offset -> decerement width
        if self.end.width < offset.width {
            offset.width = self.end.width;
        }
        // if new position is greater than offset, offset gets current_width - screen width
        // this better handles snapping the cursor to the end of the line
        if self.end.width >= size.width + offset.width {
            //self.screen_offset.width = self.screen_offset.width.saturating_sub(1);
            offset.width = offset.width.saturating_add(1);
        }
    }

    pub fn render_single_line(
        &self,
        start_pos: &Position,
        buffer: &Buffer,
        highlight_color: Color,
        text_color: Color,
    ) {
        let h_r = match self.or {
            HighlightOrientation::EndFirst => self.end.width..=start_pos.width,
            HighlightOrientation::StartFirst => start_pos.width..=self.end.width,
        };

        // cond for is the highlight ends at the end of the line
        let te = buffer.text[start_pos.height].raw_string.len() - 1 == *h_r.end();
        // cond for if the highlight starts at pos 0
        let ts = *h_r.start() == 0;

        // determine how the single line needs to be highlighted
        let h_t = match (te, ts) {
            (true, true) => HighlightLineType::All,
            (true, false) => HighlightLineType::Trailing,
            (false, true) => HighlightLineType::Leading,
            (false, false) => HighlightLineType::Middle,
        };

        Self::render_highlight_line(
            &buffer.text[start_pos.height].raw_string,
            start_pos.height,
            h_r,
            h_t,
            highlight_color,
            text_color,
        );
    }

    pub fn multi_line_render(
        &self,
        start_pos: &Position,
        screen_offset: &Position,
        size: &Size,
        buffer: &Buffer,
        highlight_color: Color,
        text_color: Color,
    ) {
        let visible_height_range =
            RangeInclusive::new(screen_offset.height, screen_offset.height + size.height);
        let visible_width_range =
            RangeInclusive::new(screen_offset.width, screen_offset.width + size.width);

        for line_height in self.line_range.clone() {
            // if line height not on the current screen view
            if !visible_height_range.contains(&line_height) {
                continue;
            }

            let line_text = &buffer.text[line_height].raw_string;
            // if line width not on screen
            if line_text.len().saturating_sub(1) < *visible_width_range.start() {
                continue;
            }

            // get the visible portion of the line
            let visible_line = match line_text.get(
                *visible_width_range.start()
                    ..=std::cmp::min(
                        *visible_width_range.end(),
                        line_text.len().saturating_sub(1),
                    ),
            ) {
                Some(text) => text,
                None => continue,
            };

            // when the line is the start
            // need to handle a partial line highlight
            if line_height == start_pos.height {
                match self.or {
                    HighlightOrientation::StartFirst => Self::render_highlight_line(
                        visible_line,
                        line_height.saturating_sub(screen_offset.height),
                        start_pos.width..=visible_line.len() - 1,
                        HighlightLineType::Trailing,
                        highlight_color,
                        text_color,
                    ),
                    HighlightOrientation::EndFirst => Self::render_highlight_line(
                        visible_line,
                        line_height.saturating_sub(screen_offset.height),
                        0..=start_pos.width,
                        HighlightLineType::Leading,
                        highlight_color,
                        text_color,
                    ),
                }
                continue;
            }

            // when the line is the end
            // need to handle a partial line highlight
            if line_height == self.end.height {
                match self.or {
                    HighlightOrientation::StartFirst => Highlight::render_highlight_line(
                        visible_line,
                        line_height.saturating_sub(screen_offset.height),
                        0..=self.end.width,
                        HighlightLineType::Leading,
                        highlight_color,
                        text_color,
                    ),
                    HighlightOrientation::EndFirst => Highlight::render_highlight_line(
                        visible_line,
                        line_height.saturating_sub(screen_offset.height),
                        self.end.width..=visible_line.len() - 1,
                        HighlightLineType::Trailing,
                        highlight_color,
                        text_color,
                    ),
                }
                continue;
            }

            // if we get here, we are highlighting the whole line
            Highlight::render_highlight_line(
                visible_line,
                line_height.saturating_sub(screen_offset.height),
                0..=visible_line.len() - 1,
                HighlightLineType::All,
                highlight_color,
                text_color,
            )
        }
    }
}
