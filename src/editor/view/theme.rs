use crate::editor::terminal::{Position, Terminal};
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::style::Color;

pub struct Theme {
    foreground: Color,
    background: Color,
    pub search_highlight: Color,
    pub search_text: Color,
    cursor_style: SetCursorStyle,
}

impl Default for Theme {
    fn default() -> Theme {
        Theme {
            foreground: Color::White,
            background: Color::Black,
            search_highlight: Color::Blue,
            search_text: Color::White,
            cursor_style: SetCursorStyle::DefaultUserShape,
        }
    }
}

impl Theme {
    pub fn set_theme(&mut self) {
        let mut cursor_position: usize = 1;
        let options: [String; 16] = [
            "DarkGrey".into(),
            "Red".into(),
            "Green".into(),
            "Yellow".into(),
            "Blue".into(),
            "Magenta".into(),
            "Cyan".into(),
            "White".into(),
            "Black".into(),
            "DarkRed".into(),
            "DarkGreen".into(),
            "DarkYellow".into(),
            "DarkBlue".into(),
            "DarkMagenta".into(),
            "DarkCyan".into(),
            "Grey".into(),
        ];

        let cursor_options: [String; 7] = [
            "DefaultUserShape".into(),
            "BlinkingBlock".into(),
            "SteadyBlock".into(),
            "BlinkingUnderScore".into(),
            "SteadyUnderScore".into(),
            "BlinkingBar".into(),
            "SteadyBar".into(),
        ];

        let mut user_choices: [String; 5] = Default::default();

        let render_option: [String; 5] = [
            "Select text color:".into(),
            "Select background color:".into(),
            "Select search highlight color:".into(),
            "Select search text color:".into(),
            "Select cursor style:".into(),
        ];

        Terminal::clear_screen().unwrap();
        for (line_index, color) in options.iter().enumerate() {
            Terminal::render_line(line_index.saturating_add(1), &color).unwrap();
        }
        for (choice_index, render_screen) in render_option.iter().enumerate() {
            if cursor_position != 1 as usize {
                cursor_position = 1;
            }
            if choice_index == 4 {
                Terminal::clear_screen().unwrap();
                for (line_index, option) in cursor_options.iter().enumerate() {
                    Terminal::render_line(line_index.saturating_add(1), &option).unwrap();
                }
            }
            Terminal::render_line(0 as usize, &render_screen).unwrap();
            Terminal::execute().unwrap();
            Self::move_cursor(cursor_position);
            loop {
                match read() {
                    Ok(event) => {
                        match event {
                            Event::Key(KeyEvent { code, .. }) => match code {
                                KeyCode::Up => {
                                    if cursor_position > 1_usize {
                                        cursor_position = cursor_position.saturating_sub(1);
                                        Self::move_cursor(cursor_position);
                                    }
                                }

                                KeyCode::Down => {
                                    if choice_index != 4 {
                                        cursor_position = std::cmp::min(
                                            cursor_position.saturating_add(1),
                                            16_usize,
                                        );
                                    } else {
                                        cursor_position = std::cmp::min(
                                            cursor_position.saturating_add(1),
                                            7_usize,
                                        );
                                    }
                                    Self::move_cursor(cursor_position);
                                }
                                KeyCode::Enter => {
                                    if choice_index != 4 {
                                        user_choices[choice_index] = options
                                            .get(cursor_position.saturating_sub(1))
                                            .expect("Out of bounds")
                                            .to_string();
                                    } else {
                                        user_choices[choice_index] = cursor_options
                                            .get(cursor_position.saturating_sub(1))
                                            .expect("Out of bounds")
                                            .to_string();
                                    }
                                    break;
                                }
                                KeyCode::Esc => {
                                    // do not change the state at all and return
                                    return;
                                }
                                _ => {
                                    //not addressing any other key presses
                                }
                            },
                            _ => {
                                //not addressing other events
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        self.foreground = Self::get_color(&user_choices[0]);
        self.background = Self::get_color(&user_choices[1]);
        self.search_highlight = Self::get_color(&user_choices[2]);
        self.search_text = Self::get_color(&user_choices[3]);
        self.cursor_style = Self::get_cursor_style(&user_choices[4]);

        Terminal::set_foreground_color(self.foreground.clone()).unwrap();
        Terminal::set_background_color(self.background.clone()).unwrap();
        Terminal::set_cursor_style(self.cursor_style.clone()).unwrap();

        Terminal::execute().expect("Error flushing terminal queue");
    }

    fn move_cursor(position: usize) {
        Terminal::hide_cursor().expect("Error hiding cursor");
        Terminal::move_cursor_to(Position {
            height: position,
            width: 0,
        })
        .expect("Error moving cursor");
        Terminal::show_cursor().expect("Error showing cursor");
        Terminal::execute().expect("Error flushing terminal queue");
    }

    fn get_cursor_style(style_str: &str) -> SetCursorStyle {
        match style_str {
            "DefaultUserShape" => SetCursorStyle::DefaultUserShape,
            "BlinkingBlock" => SetCursorStyle::BlinkingBlock,
            "SteadyBlock" => SetCursorStyle::SteadyBlock,
            "BlinkingUnderScore" => SetCursorStyle::BlinkingUnderScore,
            "SteadyUnderScore" => SetCursorStyle::SteadyUnderScore,
            "BlinkingBar" => SetCursorStyle::BlinkingBar,
            "SteadyBar" => SetCursorStyle::SteadyBar,
            _ => SetCursorStyle::DefaultUserShape,
        }
    }

    fn get_color(color_str: &str) -> Color {
        match color_str {
            "DarkGrey" => Color::DarkGrey,
            "Red" => Color::Red,
            "Green" => Color::Green,
            "Yellow" => Color::Yellow,
            "Blue" => Color::Blue,
            "Magenta" => Color::Magenta,
            "Cyan" => Color::Cyan,
            "White" => Color::White,
            "Black" => Color::Black,
            "DarkRed" => Color::DarkRed,
            "DarkGreen" => Color::DarkGreen,
            "DarkYellow" => Color::DarkYellow,
            "DarkBlue" => Color::DarkBlue,
            "DarkMagenta" => Color::DarkMagenta,
            "DarkCyan" => Color::DarkCyan,
            "Grey" => Color::Grey,
            _ => Color::White,
        }
    }
}
