use crossterm::cursor::SetCursorStyle;
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
pub mod terminal;
use std::env::args;
use std::io::{Error, ErrorKind};
use std::panic::{set_hook, take_hook};
use std::{thread, time::Duration};
use terminal::Terminal;
mod view;
use view::View;
pub mod editorcommands;
use editorcommands::EditorCommand;

#[derive(Default)]
pub struct Editor {
    view: View,
    // should_quit: bool,
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
            if view.load(filename).is_err() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("{filename} is a directory"),
                ));
            };
        }
        Ok(Self {
            // should_quit: false,
            view,
        })
    }

    pub fn run(&mut self) -> Result<(), Error> {
        // inital render
        let res = self.view.start();
        debug_assert!(res.is_ok());
        loop {
            /*
                        if self.should_quit {
                            break;
                        }
            */
            match read() {
                Ok(event) => {
                    let cont = self.evaluate_event(event)?;
                    if !cont {
                        break;
                    }
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
    fn evaluate_event(&mut self, event: Event) -> Result<bool, Error> {
        let should_process = match &event {
            Event::Key(KeyEvent { kind, .. }) => kind == &KeyEventKind::Press,
            Event::Resize(_, _) => true,
            _ => false,
        };

        if should_process {
            match EditorCommand::try_from(event) {
                Ok(command) => {
                    if matches!(command, EditorCommand::Quit) {
                        if !self.view.buffer.is_saved && !self.view.buffer.is_empty() {
                            let exit = Self::exit_without_saving()?;
                            if exit {
                                Terminal::clear_screen()?;
                                Terminal::render_line(0, "Exiting without saving...")?;
                                Terminal::execute()?;
                                thread::sleep(Duration::from_millis(300));
                            } else {
                                if self.view.buffer.filename.is_none() {
                                    self.view.get_file_name();
                                }
                                if self.view.buffer.filename.is_some() {
                                    self.view.buffer.save();
                                }
                            }
                        }
                        return Ok(false);
                    }
                    // process the event
                    // handle is any downtream commands quit the session
                    if let Ok(should_continue) = self.view.handle_event(command) {
                        return Ok(should_continue);
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
        Ok(true)
    }

    fn exit_without_saving() -> Result<bool, Error> {
        Terminal::clear_screen()?;
        Terminal::hide_cursor()?;
        Terminal::render_line(0, "Leave without saving:")?;
        Terminal::render_line(1, "Ctrl-y = exit | Ctrl-n = save")?;
        Terminal::execute()?;

        loop {
            match read() {
                Ok(event) => {
                    if let Event::Key(KeyEvent {
                        code, modifiers, ..
                    }) = event
                    {
                        match (code, modifiers) {
                            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                                return Ok(true);
                            }
                            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                                return Ok(false);
                            }
                            _ => {
                                // not reading other key presses
                            }
                        }
                    } else {
                        // doing nothing for other events
                    }
                }
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not handle event {err}");
                    }
                }
            }
        }
    }

    /*
    fn refresh_screen(&mut self) -> Result<(), Error> {
        Terminal::hide_cursor()?;
        Terminal::move_cursor_to(self.view.screen_offset.to_position())?;
        Terminal::clear_screen()?;
        /*
                if self.should_quit {
                    Terminal::print("Goodbye.\r\n")?;
                } else if self.view.needs_redraw {
                    let _ = self.view.render(true);
                }
        */
        let _ = self.view.render(true);
        Terminal::render_status_line(
            Mode::Insert,
            self.buffer.is_saved,
            &self.size,
            self.buffer.filename.as_deref(),
            Some((
                self.cursor_position.height.saturating_add(1),
                self.buffer.len(),
            )),
        )?;
        Terminal::render_status_line()
        Terminal::move_cursor_to(
            self.view
                .cursor_position
                .relative_view_position(&self.view.screen_offset),
        )?;
        Terminal::show_cursor()?;
        Terminal::execute()?;
        Ok(())
    }
    */
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = Terminal::set_cursor_style(SetCursorStyle::DefaultUserShape);
        let _ = Terminal::terminate();
        let _ = Terminal::print("Goodbye.\r\n");
    }
}
