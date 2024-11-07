use crate::editor::terminal::{Position, Terminal};
use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::style::Color;

#[derive(Default)]
pub struct Theme {
    foreground: Option<Color>,
    background: Option<Color>,
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
        let mut user_choices: [String; 2] = Default::default();
        let render_option: [String; 2] = [
            "Select text color:".into(),
            "Select background color".into(),
        ];
        for (choice_index, render_screen) in render_option.iter().enumerate() {
            if cursor_position != 1 as usize {
                cursor_position = 1;
            }
            Terminal::clear_screen().expect("Error clearing screen");
            Terminal::render_line(0 as usize, &render_screen).expect("Error rendering line");
            for (line_index, color) in options.iter().enumerate() {
                Terminal::render_line(line_index.saturating_add(1), &color)
                    .expect("Error rendering theme screen");
            }
            Terminal::execute().expect("Error flushing queue");
            Self::move_cursor(cursor_position);
            loop {
                match read() {
                    Ok(event) => {
                        match event {
                            Event::Key(KeyEvent { code, .. }) => match code {
                                KeyCode::Up => {
                                    if cursor_position > 1 as usize {
                                        cursor_position = cursor_position.saturating_sub(1);
                                        Self::move_cursor(cursor_position);
                                    }
                                }

                                KeyCode::Down => {
                                    cursor_position = std::cmp::min(
                                        cursor_position.saturating_add(1),
                                        15 as usize,
                                    );
                                    Self::move_cursor(cursor_position);
                                }
                                KeyCode::Enter => {
                                    user_choices[choice_index] = options
                                        .get(cursor_position.saturating_sub(1))
                                        .expect("Out of bounds")
                                        .to_string();
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
        self.foreground = Some(Self::get_color(&user_choices[0]));
        self.background = Some(Self::get_color(&user_choices[1]));
        Terminal::set_foreground_color(self.foreground.expect("foreground_color is None"))
            .expect("Error setting foreground color");
        Terminal::set_background_color(self.background.expect("background color in None"))
            .expect("Error setting background color");
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
