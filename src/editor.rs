use crossterm::event::{
    read,
    Event::{self, Key},
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
mod terminal;
use std::cmp::{max, min};
use std::io::Error;
use terminal::{Position, Size, Terminal};
mod view;
use view::View;

#[derive(Default)]
pub struct Editor {
    should_quit: bool,
    cursor_position: Position,
    view: View,
}

impl Editor {
    pub fn run(&mut self) {
        Terminal::initialize().unwrap();
        self.view.buffer.text.push(String::from("Hello World"));
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
        Terminal::move_cursor_to(Position::default())?;
        if self.should_quit {
            Terminal::clear_screen()?;
            Terminal::print("Goodbye.\r\n")?;
        } else {
            self.view.render()?;
            Terminal::move_cursor_to(Position {
                x: self.cursor_position.x,
                y: self.cursor_position.y,
            })?;
        }
        Terminal::show_cursor()?;
        Terminal::execute()?;
        Ok(())
    }

    fn move_cursor(&mut self, key_code: KeyCode) -> Result<(), Error> {
        let Size { height, width } = Terminal::size()?;
        match key_code {
            KeyCode::Down => {
                self.cursor_position.y = min(
                    self.cursor_position.y.saturating_add(1),
                    width.saturating_sub(1),
                );
            }
            KeyCode::Up => {
                self.cursor_position.y = max(self.cursor_position.y.saturating_sub(1), 0);
            }
            KeyCode::Left => {
                self.cursor_position.x = max(self.cursor_position.x.saturating_sub(1), 0);
            }
            KeyCode::Right => {
                self.cursor_position.x = min(
                    self.cursor_position.x.saturating_add(1),
                    height.saturating_sub(1),
                );
            }
            KeyCode::PageDown => {
                self.cursor_position.y = height;
            }
            KeyCode::PageUp => {
                self.cursor_position.y = 0;
            }
            KeyCode::End => {
                self.cursor_position.x = 0;
            }
            KeyCode::Home => {
                self.cursor_position.x = width;
            }
            _ => {}
        }
        Ok(())
    }
}
