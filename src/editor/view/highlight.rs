use crate::editor::{view::Buffer, Position, Terminal};
use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{Color, PrintStyledContent, StyledContent, Stylize};
use std::ops::Range;

pub struct Highlight;
impl Highlight {
    pub fn highlight_text(
        ctx: &mut ClipboardContext,
        buffer: &Buffer,
        start: &Position,
        h_color: Color,
        t_color: Color,
    ) {
        let mut render_start = start.clone();
        let mut render_end = start.clone();
        let mut end = start.clone();
        let max_height = buffer.len() - 1;
        let mut max_width = buffer.text[start.height].grapheme_len();

        loop {
            match read() {
                Ok(event) => match event {
                    Event::Key(KeyEvent {
                        code, modifiers, ..
                    }) => match (code, modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            break;
                        }
                        (KeyCode::Right, _) => {
                            if end.width == max_width {
                                end.height = std::cmp::min(end.height + 1, max_height);
                                max_width = buffer.text[end.height].grapheme_len();
                                end.width = 0;
                            } else {
                                end.width += 1;
                            }
                        }
                        (KeyCode::Left, _) => {
                            if end.width == 0 {
                                end.height = end.height.saturating_sub(1);
                                max_width = buffer.text[end.height].grapheme_len();
                                end.width = max_width;
                            }
                        }
                        (KeyCode::Down, _) => {
                            end.height = std::cmp::min(max_height, end.height + 1);
                            max_width = buffer.text[end.height].grapheme_len();
                            end.width = std::cmp::min(end.width, max_width);
                        }
                        (KeyCode::Up, _) => {
                            end.height = end.height.saturating_sub(1);
                            max_width = buffer.text[end.height].grapheme_len();
                            end.width = std::cmp::min(end.width, max_width);
                        }
                        (KeyCode::Esc, _) => return,
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
            Terminal::hide_cursor().unwrap();
            match end.height == start.height {
                true => {
                    // we only need to render one line here
                    let ind_range = if end.width > start.width {
                        start.width..end.width
                    } else {
                        end.width..start.width
                    };
                    Self::render_highlight_line(
                        &buffer.text[start.height].raw_string,
                        start.height,
                        ind_range,
                        h_color.clone(),
                        t_color.clone(),
                    );
                }
                false => {
                    if end.height > start.height {
                        render_start = start.clone();
                        render_end = end.clone();
                    } else {
                        render_start = end.clone();
                        render_end = start.clone();
                    };
                    let height_range = render_start.height + 1..render_end.height;

                    Self::render_highlight_line(
                        &buffer.text[render_start.height].raw_string,
                        render_start.height,
                        render_start.width
                            ..buffer.text[render_start.height]
                                .grapheme_len()
                                .saturating_sub(1),
                        h_color.clone(),
                        t_color.clone(),
                    );

                    for h in height_range {
                        Self::render_highlight_line(
                            &buffer.text[h].raw_string,
                            h,
                            0..buffer.text[h].grapheme_len().saturating_sub(1),
                            h_color.clone(),
                            t_color.clone(),
                        );
                    }
                    Self::render_highlight_line(
                        &buffer.text[render_end.height].raw_string,
                        render_end.height,
                        0..render_end.width,
                        h_color.clone(),
                        t_color.clone(),
                    );
                }
            }
            Terminal::move_cursor_to(end).unwrap();
            Terminal::show_cursor().unwrap();
            Terminal::execute().unwrap();
        }

        let copy_string = Self::generate_copy_str(buffer, render_start, render_end);
        if copy_string.is_empty() {
            return;
        }
        Self::copy_text_to_clipboard(ctx, copy_string);
    }

    fn generate_copy_str(buffer: &Buffer, start: Position, end: Position) -> String {
        //check to see is string in copy buffer > 1 line
        //if not take start..end
        //if yes
        //take start line start..
        //take full lines for all start < line < end
        //take ..end for end line
        //\n for every line switch
        //(for new see how this copies)
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

    fn render_highlight_line(
        line: &str,
        height: usize,
        ind_range: Range<usize>,
        h_color: Color,
        t_color: Color,
    ) {
        let line_len = line.len() - 1;
        Terminal::move_cursor_to(Position { height, width: 0 }).unwrap();

        Terminal::clear_line().unwrap();
        if ind_range.start != 0 {
            Terminal::print(&line[..ind_range.start]).unwrap();
        }

        let seg_to_color: String = if ind_range.end == line_len {
            line[ind_range.start..].to_string()
        } else {
            line[ind_range.clone()].to_string()
        };

        let highlight_seg: StyledContent<String> = seg_to_color.clone().with(t_color).on(h_color);
        Terminal::queue_command(PrintStyledContent(highlight_seg)).unwrap();

        if ind_range.end != line_len {
            Terminal::print(&line[ind_range.end..]).unwrap();
        }
    }
    fn copy_text_to_clipboard(ctx: &mut ClipboardContext, content: String) {
        ctx.set_contents(content).unwrap();
    }
}
