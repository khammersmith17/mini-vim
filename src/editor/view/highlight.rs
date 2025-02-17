use super::clipboard_interface::ClipboardUtils;
use crate::editor::editorcommands::HighlightCommand;
use crate::editor::{
    terminal::{Coordinate, Position, ScreenOffset, Size, Terminal},
    view::{Buffer, Mode},
};
use crossterm::event::{read, Event};
use crossterm::style::{Color, Print, PrintStyledContent, StyledContent, Stylize};
use std::error::Error;
use std::ops::{Range, RangeInclusive};

/// type to identify the direction the highlight goes in
/// whether the highlight is going forward or backward
#[derive(Copy, Clone, Default, PartialEq)]
pub enum Orientation {
    #[default]
    StartFirst,
    EndFirst,
}

/// seperate the logic for highlight a partial and full line
pub enum LineType {
    Middle,
    Leading,
    Trailing,
    All,
}

/// type to handle the highlighting and copy logic
pub struct Highlight<'a> {
    end: Position, // one copy owned here, the end of the highlight owned by highlight
    offset: ScreenOffset,
    or: Orientation,
    line_range: RangeInclusive<usize>,
    start: &'a mut Position, //one mutably borrowed, the view's position
    size: &'a mut Size,      //owned by view
    buffer: &'a mut Buffer,  //owned by view
}

impl Highlight<'_> {
    pub fn new<'a>(
        end: &'a mut Position,
        offset: ScreenOffset,
        size: &'a mut Size,
        buffer: &'a mut Buffer,
    ) -> Highlight<'a> {
        Highlight {
            offset,
            end: *end,
            or: Orientation::default(),
            line_range: 0..=0,
            start: end, // the immutable reference
            size,
            buffer,
        }
    }

    pub fn run<P>(&mut self, highlight: Color, text: Color, parser: P)
    where
        P: Fn(Event) -> Result<HighlightCommand, Box<dyn Error>>,
    {
        let res = self.initial_set_screen();
        debug_assert!(res.is_ok());
        loop {
            let Ok(read_event) = read() else { continue }; //skipping errors here
            match parser(read_event) {
                Ok(event) => match event {
                    HighlightCommand::Move(dir) => dir.move_cursor(&mut self.end, &*self.buffer),
                    HighlightCommand::Copy => {
                        break;
                    }
                    HighlightCommand::Resize(new_size) => *self.size = new_size,
                    HighlightCommand::RevertState => {
                        return;
                    }
                    HighlightCommand::Delete => {
                        if *self.start != self.end {
                            self.batch_delete();
                        }
                        return;
                    }
                    HighlightCommand::NoAction => continue,
                },
                Err(_) => continue,
            }
            let view_delta = self
                .end
                .max_displacement_from_view(&self.offset, &self.size, 2);
            self.update_offset();
            self.resolve_orientation();
            self.adjust_range();
            let res = Terminal::hide_cursor();
            debug_assert!(res.is_ok());
            if view_delta > 0 {
                // only doing a full render when the offset shifts
                let res = self.render();
                debug_assert!(res.is_ok());
            }

            let res = if self.start.height == self.end.height {
                self.render_single_line(highlight, text)
            } else {
                self.multi_line_render(highlight, text)
            };
            debug_assert!(res.is_ok());
            let res = self.status_line();
            debug_assert!(res.is_ok());

            let res = self.post_render();
            debug_assert!(res.is_ok());
        }

        let copy_string = self.generate_copy_str();

        if !copy_string.is_empty() {
            let res = ClipboardUtils::copy_text_to_clipboard(copy_string);
            debug_assert!(res.is_ok());
        }
    }

    fn initial_set_screen(&self) -> Result<(), Box<dyn Error>> {
        self.status_line()?; // to see status line before first event is read
        Terminal::move_cursor_to(self.end)?;
        Terminal::execute()?;
        Ok(())
    }

    #[inline(always)]
    fn post_render(&self) -> Result<(), Box<dyn Error>> {
        Terminal::move_cursor_to(self.end.relative_view_position(&self.offset))?;
        Terminal::show_cursor()?;
        Terminal::execute()?;
        Ok(())
    }

    #[inline]
    fn status_line(&self) -> Result<(), Box<dyn Error>> {
        Terminal::render_status_line(
            Mode::Highlight,
            self.buffer.is_saved,
            &self.size,
            self.buffer.filename.as_deref(),
            Some((self.end.height.saturating_add(1), self.buffer.len())),
        )?;
        Ok(())
    }

    fn render(&self) -> Result<(), Box<dyn Error>> {
        Terminal::clear_screen()?;
        #[allow(clippy::integer_division)]
        for current_row in self.offset.height
            ..self
                .offset
                .height
                .saturating_add(self.size.height)
                .saturating_sub(1)
        {
            let relative_row = current_row.saturating_sub(self.offset.height);

            if self.line_range.contains(&current_row) {
                // going to handle rendering these lines with the highlight range
                // want to skip this so we do not render twice
                continue;
            }

            if let Some(line) = self.buffer.text.get(current_row) {
                Terminal::render_line(
                    relative_row,
                    line.get_line_subset(
                        self.offset.width..self.offset.width.saturating_add(self.size.width),
                    ),
                )?;
            } else {
                Terminal::render_line(relative_row, "~")?;
            }
        }
        Ok(())
    }

    pub fn adjust_range(&mut self) {
        match self.or {
            Orientation::StartFirst => {
                self.line_range = self.start.height..=self.end.height;
            }
            Orientation::EndFirst => {
                self.line_range = self.end.height..=self.start.height;
            }
        }
    }

    pub fn resolve_orientation(&mut self) {
        if self.start.height == self.end.height {
            if self.start.width <= self.end.width {
                self.or = Orientation::StartFirst;
            } else {
                self.or = Orientation::EndFirst;
            }
            return;
        }
        if self.start.height < self.end.height {
            self.or = Orientation::StartFirst;
        } else {
            self.or = Orientation::EndFirst;
        }
    }
    pub fn generate_copy_str(&self) -> String {
        match self.or {
            Orientation::StartFirst => self.buffer.get_segment(&self.start, &self.end),
            Orientation::EndFirst => self.buffer.get_segment(&self.end, &self.start),
        }
    }

    pub fn update_offset(&mut self) {
        // adding a method to handle the offset when
        // the end goes off screen of the highlight
        // block goes off screen
        // taken from View::update_offset_single_move
        // with different parameters to update the highlight end
        if self.end.height
            >= (self.size.height.saturating_add(self.offset.height)).saturating_sub(1)
        {
            self.offset.height = std::cmp::min(
                self.offset.height.saturating_add(1),
                self.end
                    .height
                    .saturating_sub(self.size.height)
                    .saturating_add(2), // space for file info line
            );
        }
        // if height moves less than the offset -> decrement height
        if self.end.height <= self.offset.height {
            self.offset.height = self.end.height;
        }
        //if widith less than offset -> decerement width
        if self.end.width < self.offset.width {
            self.offset.width = self.end.width;
        }
        // if new position is greater than offset, offset gets current_width - screen width
        // this better handles snapping the cursor to the end of the line
        if self.end.width >= self.size.width.saturating_add(self.offset.width) {
            //self.screen_offset.width = self.screen_offset.width.saturating_sub(1);
            self.offset.width = self.offset.width.saturating_add(1);
        }
    }

    pub fn render_single_line(
        &self,
        highlight_color: Color,
        text_color: Color,
    ) -> Result<(), Box<dyn Error>> {
        let h_r = match self.or {
            Orientation::EndFirst => self.end.width..self.start.width,
            Orientation::StartFirst => self.start.width..self.end.width,
        };

        // cond for is the highlight ends at the end of the line
        let te = self.buffer.text[self.start.height]
            .raw_string
            .len()
            .saturating_sub(1)
            == h_r.end;
        // cond for if the highlight starts at pos 0
        let ts = h_r.start == 0;

        // determine how the single line needs to be highlighted
        let h_t = match (te, ts) {
            (true, true) => LineType::All,
            (true, false) => LineType::Trailing,
            (false, true) => LineType::Leading,
            (false, false) => LineType::Middle,
        };

        HighlightUtility::render_highlight_line(
            &*self.buffer.text[self.start.height].raw_string,
            self.start.height,
            h_r,
            &h_t,
            highlight_color,
            text_color,
        )?;
        Ok(())
    }

    pub fn multi_line_render(
        &self,
        highlight_color: Color,
        text_color: Color,
    ) -> Result<(), Box<dyn Error>> {
        let visible_height_range = RangeInclusive::new(
            self.offset.height,
            self.offset.height.saturating_add(self.size.height),
        );
        let visible_width_range = RangeInclusive::new(
            self.offset.width,
            self.offset.width.saturating_add(self.size.width),
        );

        for line_height in self.line_range.clone() {
            // if line height not on the current screen view
            if !visible_height_range.contains(&line_height) {
                continue;
            }

            let line_text = &self.buffer.text[line_height].raw_string;

            // if line width not on screen
            if line_text.len().saturating_sub(1) < *visible_width_range.start() {
                continue;
            }

            // get the visible portion of the line
            // if line is empty -> " " so we get some highlight on the line
            let visible_line: &str = if line_text.is_empty() {
                " "
            } else if line_text.len() > *visible_width_range.start() {
                &line_text[*visible_width_range.start()
                    ..=std::cmp::min(
                        *visible_width_range.end(),
                        line_text.len().saturating_sub(1),
                    )]
            } else {
                continue;
            };

            // when the line is the start
            // need to handle a partial line highlight
            if line_height == self.start.height {
                match self.or {
                    Orientation::StartFirst => HighlightUtility::render_highlight_line(
                        visible_line,
                        line_height.saturating_sub(self.offset.height),
                        self.start.width..visible_line.len(),
                        &LineType::Trailing,
                        highlight_color,
                        text_color,
                    )?,
                    Orientation::EndFirst => HighlightUtility::render_highlight_line(
                        visible_line,
                        line_height.saturating_sub(self.offset.height),
                        0..self.start.width.saturating_add(1),
                        &LineType::Leading,
                        highlight_color,
                        text_color,
                    )?,
                }
                continue;
            }

            // when the line is the end
            // need to handle a partial line highlight
            if line_height == self.end.height {
                match self.or {
                    Orientation::StartFirst => HighlightUtility::render_highlight_line(
                        visible_line,
                        line_height.saturating_sub(self.offset.height),
                        0..self.end.width,
                        &LineType::Leading,
                        highlight_color,
                        text_color,
                    )?,
                    Orientation::EndFirst => HighlightUtility::render_highlight_line(
                        visible_line,
                        line_height.saturating_sub(self.offset.height),
                        self.end.width..visible_line.len(),
                        &LineType::Trailing,
                        highlight_color,
                        text_color,
                    )?,
                };
                continue;
            }

            // if we get here, we are highlighting the whole line
            HighlightUtility::render_highlight_line(
                visible_line,
                line_height.saturating_sub(self.offset.height),
                0..visible_line.len(),
                &LineType::All,
                highlight_color,
                text_color,
            )?;
        }
        Ok(())
    }

    fn batch_delete(&mut self) {
        self.resolve_orientation();

        if self.start.diff_height(&self.end) == 0 {
            match self.or {
                Orientation::StartFirst => self.buffer.delete_segment(&self.start, &mut self.end),
                Orientation::EndFirst => self.buffer.delete_segment(&self.end, &mut self.start),
            }
        } else {
            if self.start.diff_height(&self.end) > 1 {
                let range_iter = match self.or {
                    Orientation::StartFirst => {
                        (self.start.height.saturating_add(1)..self.end.height).rev()
                    }
                    Orientation::EndFirst => {
                        (self.end.height.saturating_add(1)..self.start.height).rev()
                    }
                };

                for line in range_iter {
                    self.buffer.pop_line(line);
                }
            }

            //delete everything left of bottom position
            //delete everything right of top position
            //join the lines
            match self.or {
                Orientation::StartFirst => {
                    self.end.set_height(self.start.height.saturating_add(1));
                    self.buffer.delete_segment(
                        &Position {
                            width: 0,
                            height: self.end.height,
                        },
                        &mut self.end,
                    );
                    self.buffer.delete_segment(
                        &self.start,
                        &mut Position {
                            width: self.buffer.text[self.start.height].len().saturating_sub(1),
                            height: self.start.height,
                        },
                    );
                    self.buffer.join_line(self.end.height);
                }
                Orientation::EndFirst => {
                    self.start.set_height(self.end.height.saturating_add(1));
                    self.buffer.delete_segment(
                        &self.start,
                        &mut Position {
                            width: self.buffer.text[self.end.height].len().saturating_sub(1),
                            height: self.end.height,
                        },
                    );
                    self.buffer.delete_segment(
                        &Position {
                            width: 0,
                            height: self.start.height,
                        },
                        &mut self.start,
                    );
                    self.buffer.join_line(self.start.height);
                    self.start.set_position(self.end);
                }
            }
        }
    }
}

struct HighlightUtility;

impl HighlightUtility {
    pub fn render_highlight_line(
        line: &str,
        height: usize,
        h_range: Range<usize>,
        ctx: &LineType,
        h_color: Color,
        t_color: Color,
    ) -> Result<(), Box<dyn Error>> {
        Terminal::move_cursor_to(Position { height, width: 0 })?;
        Terminal::clear_line()?;

        let segment_to_highlight: String = line[h_range.clone()].to_owned();
        let highlight_seg: StyledContent<String> =
            segment_to_highlight.clone().with(t_color).on(h_color);

        // order in which elements are rendered
        // on the line based on line type
        match ctx {
            LineType::All => {
                Terminal::queue_command(PrintStyledContent(highlight_seg))?;
            }
            LineType::Leading => {
                Terminal::queue_command(PrintStyledContent(highlight_seg))?;
                Terminal::queue_command(Print(&line[(h_range.end)..]))?;
            }
            LineType::Trailing => {
                Terminal::queue_command(Print(&line[..h_range.start]))?;
                Terminal::queue_command(PrintStyledContent(highlight_seg))?;
            }
            LineType::Middle => {
                Terminal::queue_command(Print(&line[..h_range.start]))?;
                Terminal::queue_command(PrintStyledContent(highlight_seg))?;
                Terminal::queue_command(Print(&line[h_range.end..]))?;
            }
        }

        Ok(())
    }
}
