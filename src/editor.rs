use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
mod terminal;
use std::env::args;
use std::io::Error;
use std::panic::{set_hook, take_hook};
use terminal::{Position, Terminal};
mod view;
use view::View;
mod editorcommands;
use editorcommands::EditorCommand;

#[derive(Default)]
pub struct Editor {
    should_quit: bool,
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
        let should_process = match &event {
            Event::Key(KeyEvent { kind, .. }) => kind == &KeyEventKind::Press,
            Event::Resize(_, _) => true,
            _ => false,
        };

        if should_process {
            match EditorCommand::try_from(event) {
                Ok(command) => {
                    if matches!(command, EditorCommand::Quit) {
                        if !self.view.buffer.is_saved {
                            if !self.view.buffer.is_empty() {
                                let exit = self.exit_without_saving();
                                match exit {
                                    false => {
                                        if self.view.buffer.filename.is_none() {
                                            self.view.get_file_name();
                                        }
                                        self.view.buffer.save();
                                    }
                                    true => {
                                        Terminal::clear_screen().expect("Error clearing screen");
                                        self.view.render_line(0, "Exiting without saving...");
                                    }
                                }
                            }
                        }
                        self.should_quit = true;
                    } else {
                        self.view.handle_event(command)
                    }
                }
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not handle command {err:?}")
                    }
                }
            }
        } else {
            #[cfg(debug_assertions)]
            {
                panic!("Unsupported event")
            }
        }
    }

    fn exit_without_saving(&self) -> bool {
        let mut result: bool = false;
        Terminal::clear_screen().expect("Error clearing screen");
        Terminal::hide_cursor().expect("Error hiding cursor");
        self.view
            .render_line(0, "Leave without saving: Ctrl-y = exit | Ctrl-n = save");
        Terminal::execute().expect("Error flushing std buffer");
        loop {
            match read() {
                Ok(event) => match event {
                    Event::Key(KeyEvent {
                        code, modifiers, ..
                    }) => match (code, modifiers) {
                        (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                            result = true;
                            break;
                        }
                        (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                            break;
                        }
                        _ => {
                            // not reading other key presses
                        }
                    },
                    _ => {
                        // doing nothing for other events
                    }
                },
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not handle event {err}");
                    }
                }
            }
        }
        result
    }

    fn refresh_screen(&mut self) -> Result<(), Error> {
        Terminal::hide_cursor()?;
        Terminal::move_cursor_to(self.view.screen_offset)?;
        Terminal::clear_screen()?;
        if self.should_quit {
            Terminal::print("Goodbye.\r\n")?;
        } else if self.view.needs_redraw {
            self.view.render();
        }
        Terminal::move_cursor_to(Position {
            width: self
                .view
                .cursor_position
                .width
                .saturating_sub(self.view.screen_offset.width),
            height: self
                .view
                .cursor_position
                .height
                .saturating_sub(self.view.screen_offset.height),
        })?;
        Terminal::show_cursor()?;
        Terminal::execute()?;
        Ok(())
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
