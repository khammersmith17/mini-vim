use crate::editor::terminal::{Position, Terminal};
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::style::Color;

pub struct Theme {
    foreground: Color,
    background: Color,
    pub highlight: Color,
    pub text: Color,
    cursor_style: SetCursorStyle,
}

impl Default for Theme {
    fn default() -> Theme {
        Theme {
            foreground: Color::White,
            background: Color::Black,
            highlight: Color::Blue,
            text: Color::White,
            cursor_style: SetCursorStyle::DefaultUserShape,
        }
    }
}

const OPTIONS: [&str; 16] = [
    "DarkGrey",
    "Red",
    "Green",
    "Yellow",
    "Blue",
    "Magenta",
    "Cyan",
    "White",
    "Black",
    "DarkRed",
    "DarkGreen",
    "DarkYellow",
    "DarkBlue",
    "DarkMagenta",
    "DarkCyan",
    "Grey",
];

const CURSOR_OPTIONS: [&str; 7] = [
    "DefaultUserShape",
    "BlinkingBlock",
    "SteadyBlock",
    "BlinkingUnderScore",
    "SteadyUnderScore",
    "BlinkingBar",
    "SteadyBar",
];

const RENDER_OPTION: [&str; 5] = [
    "Select text color:",
    "Select background color:",
    "Select search highlight color:",
    "Select search text color:",
    "Select cursor style:",
];

impl Theme {
    pub fn set_theme(&mut self) {
        let mut cursor_position: usize = 1;
        let mut user_choices: [String; 5] = Default::default();
        Terminal::clear_screen().unwrap();
        for (line_index, color) in OPTIONS.iter().enumerate() {
            Terminal::render_line(line_index.saturating_add(1), color).unwrap();
        }
        for (choice_index, render_screen) in RENDER_OPTION.iter().enumerate() {
            if cursor_position != 1_usize {
                cursor_position = 1;
            }
            if choice_index == 4 {
                Terminal::clear_screen().unwrap();
                for (line_index, option) in CURSOR_OPTIONS.iter().enumerate() {
                    Terminal::render_line(line_index.saturating_add(1), option).unwrap();
                }
            }
            Terminal::render_line(0_usize, render_screen).unwrap();
            Terminal::execute().unwrap();
            Self::move_cursor(cursor_position);
            loop {
                if let Ok(event) = read() {
                    if let Event::Key(KeyEvent { code, .. }) = event {
                        match code {
                            KeyCode::Up => {
                                if cursor_position > 1_usize {
                                    cursor_position = cursor_position.saturating_sub(1);
                                    Self::move_cursor(cursor_position);
                                }
                            }

                            KeyCode::Down => {
                                if choice_index == 4 {
                                    cursor_position =
                                        std::cmp::min(cursor_position.saturating_add(1), 7_usize);
                                } else {
                                    cursor_position =
                                        std::cmp::min(cursor_position.saturating_add(1), 16_usize);
                                }
                                Self::move_cursor(cursor_position);
                            }
                            KeyCode::Enter => {
                                if choice_index != 4 {
                                    user_choices[choice_index] =
                                        (*OPTIONS[cursor_position.saturating_sub(1)]).to_string();
                                } else {
                                    user_choices[choice_index] = (*CURSOR_OPTIONS
                                        [cursor_position.saturating_sub(1)])
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
                        }
                    }
                }
            }
        }
        self.foreground = Self::get_color(&user_choices[0]);
        self.background = Self::get_color(&user_choices[1]);
        self.highlight = Self::get_color(&user_choices[2]);
        self.text = Self::get_color(&user_choices[3]);
        self.cursor_style = Self::get_cursor_style(&user_choices[4]);
        Terminal::set_foreground_color(self.foreground).unwrap();
        Terminal::set_background_color(self.background).unwrap();
        Terminal::set_cursor_style(self.cursor_style).unwrap();
        Terminal::execute().unwrap();
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
