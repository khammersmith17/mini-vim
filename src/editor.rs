use crossterm::event::{
    read,
    Event::{self, Key},
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
mod terminal;
use std::cmp::{max, min};
use std::io::Error;
use terminal::{Position, Size, Terminal};

const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
const PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Editor {
    should_quit: bool,
    position: Position,
}

impl Editor {
    pub const fn new() -> Self {
        #[allow(clippy::as_conversions)]
        let x_start: usize = 0;
        let y_start: usize = 0;
        Editor {
            should_quit: false,
            position: Position {
                x: x_start,
                y: y_start,
            },
        }
    }

    pub fn run(&mut self) {
        Terminal::initialize().unwrap();
        let result = self.repl();
        Terminal::terminate().unwrap();
        result.unwrap();
    }

    fn repl(&mut self) -> Result<(), Error> {
        loop {
            self.refresh_screen()?;
            if self.should_quit {
                break;
            }
            let event = read()?;
            self.evaluate_event(&event)?;
        }
        Ok(())
    }

    fn evaluate_event(&mut self, event: &Event) -> Result<(), Error> {
        if let Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            match code {
                KeyCode::Char('q') if *modifiers == KeyModifiers::CONTROL => {
                    self.should_quit = true;
                }
                KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::End
                | KeyCode::Home => {
                    self.move_cursor(*code)?;
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn refresh_screen(&self) -> Result<(), Error> {
        Terminal::hide_cursor()?;
        if self.should_quit {
            Terminal::clear_screen()?;
            Terminal::print("Goodbye.\r\n")?;
        } else {
            Self::draw_rows()?;
            #[allow(clippy::as_conversions)]
            Terminal::move_cursor_to(self.position)?;
        }
        Terminal::show_cursor()?;
        Terminal::execute()?;
        Ok(())
    }

    fn draw_rows() -> Result<(), Error> {
        let Size { height, .. } = Terminal::size()?;
        for row in 0..height {
            Terminal::clear_line()?;
            #[allow(clippy::integer_division)]
            if row == height / 3 {
                Self::draw_welcome_message()?;
            } else {
                Self::draw_empty_row()?;
            }
            if row.saturating_add(1) < height {
                Terminal::print("\r\n")?;
            }
        }
        Ok(())
    }

    fn draw_empty_row() -> Result<(), Error> {
        Terminal::print("~")?;
        Ok(())
    }

    fn draw_welcome_message() -> Result<(), Error> {
        let mut welcome_message = format!("{PROGRAM_NAME} editor -- version {PROGRAM_VERSION}");
        let width = Terminal::size()?.width;
        let len = welcome_message.len();
        #[allow(clippy::integer_division)]
        let padding = (width.saturating_sub(len)) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));
        welcome_message = format!("~{spaces}{welcome_message}");
        welcome_message.truncate(width);
        Terminal::print(welcome_message)?;
        Ok(())
    }

    fn move_cursor(&mut self, code: KeyCode) -> Result<(), Error> {
        match code {
            KeyCode::Down => {
                self.position.y = min(self.position.y.saturating_add(1), Terminal::size()?.width);
            }
            KeyCode::Up => {
                self.position.y = max(self.position.y.saturating_sub(1), 0);
            }
            KeyCode::Left => {
                self.position.x = max(self.position.x.saturating_sub(1), 0);
            }
            KeyCode::Right => {
                self.position.x = min(self.position.x.saturating_add(1), Terminal::size()?.height);
            }
            KeyCode::PageDown => {
                self.position.y = Terminal::size()?.height;
            }
            KeyCode::PageUp => {
                self.position.y = 0;
            }
            KeyCode::End => {
                self.position.x = 0;
            }
            KeyCode::Home => {
                self.position.x = Terminal::size()?.width;
            }
            _ => {}
        }
        Ok(())
    }
}
