use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
mod terminal;
use std::cmp::{max, min};
use std::env::args;
use std::io::Error;
use std::panic::{set_hook, take_hook};
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
    pub fn new() -> Result<Self, Error> {
        let current_hook = take_hook();
        set_hook(Box::new(move |panic_info| {
            let _ = Terminal::terminate();
            current_hook(panic_info);
        }));
        Terminal::initialize()?;
        let args: Vec<String> = args().collect();
        let mut view = View::default();
        if let Some(filename) = args.get(1) {
            view.load(filename);
        }
        Ok(Self {
            should_quit: false,
            cursor_position: Position::default(),
            view,
        })
    }

    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            self.refresh_screen()?;
            if self.should_quit {
                break;
            }
            match read() {
                Ok(event) => {
                    self.evaluate_event(event);
                }
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not read event: {err}");
                    }
                }
            }
        }
        Ok(())
    }
    #[allow(clippy::needless_pass_by_value)]
    fn evaluate_event(&mut self, event: Event) {
        match event {
            Event::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press,
                ..
            }) => match (code, modifiers) {
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                (
                    KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::PageUp
                    | KeyCode::PageDown
                    | KeyCode::End
                    | KeyCode::Home,
                    _,
                ) => {
                    self.move_cursor(code);
                }
                (KeyCode::Char(_), _) => {
                    self.view.needs_redraw = true;
                }
                _ => {}
            },
            Event::Resize(width_16, height_16) => {
                #[allow(clippy::as_conversions)]
                let height = height_16 as usize;
                #[allow(clippy::as_conversions)]
                let width = width_16 as usize;
                self.view.resize(Size { height, width });
                self.view.needs_redraw = true;
            }
            _ => {}
        }
    }

    fn refresh_screen(&mut self) -> Result<(), Error> {
        Terminal::hide_cursor()?;
        Terminal::move_cursor_to(Position::default())?;
        if self.should_quit {
            Terminal::clear_screen()?;
            Terminal::print("Goodbye.\r\n")?;
        } else if self.view.needs_redraw {
            self.view.render();
        }
        Terminal::move_cursor_to(Position {
            x: self.cursor_position.x,
            y: self.cursor_position.y,
        })?;
        Terminal::show_cursor()?;
        Terminal::execute()?;
        Ok(())
    }

    fn move_cursor(&mut self, key_code: KeyCode) {
        let Size { height, width } = Terminal::size().unwrap_or_default();
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
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = Terminal::terminate();
        if self.should_quit {
            let _ = Terminal::print("Goodbye.\r\n");
        }
    }
}
