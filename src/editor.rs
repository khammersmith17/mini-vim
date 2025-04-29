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

// approach to multi buffer editing
// we need a structure to store the state for each different open buffer
// we will need some sort of context switiching logic
// then something to store state
/*


enum BufferState {
    Normal,
    Vim
}

pub struct Editor {
    size: Size, //size will always be shared across all buffers
    theme: Theme, //theme will always be shared across all buffer
    cursor_position: Vec<Position>,
    screen_offset: Vec<ScreenOffset>,
    buffer: Vec<Buffer>,
    state: Vec<BufferState>
}

maybe should be this
struct BufferSession {
    screen_position: Position,
    screen_offset: ScreenOffset,
    buffer: Buffer,
    state: BufferState
}

default
    - init screen position and offset at origin
    - load in buffer if file exists
    - start buffer state in normal mode

switch between vim mode and normal mode
    - exit the current state
    - then move into the other BufferState

switch between buffers
    - preserve state of current buffer
    - if the new buffer is already open, just switch
    - otherwise, open a new buffer and add to our Editor session

with this there are 2 types of context switches
    - switching to another buffer
    - switching the state of the buffer

struct Editor {
    size: Size,
    theme: Theme,
    open_buffers: Vec<BufferSession>

}

make sure there is only 1 reference to the size and terminal theme at any given time
we need to limit this because we need a mutable reference, thus only 1 should be live

other things to consider
- this should be the new structure
- vim mode should be moved out to be at the same level as view
    - then we switch between modes by passing the state from the Editor
    - creating and destroy a new view/vim instance as needed
    - then there needs to some indicator of exit state
- on session exit, need to check all open buffers for saved status
*/

#[derive(Default)]
pub struct Editor {
    view: View,
}

impl Editor {
    pub fn new() -> Result<Self, Error> {
        let current_hook = take_hook();
        // here we need to set the panic hook to ensure that when panic occurs
        // the terminal itself moves back into the normal state
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
        Ok(Self { view })
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
                Err(_err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not read event: {_err}");
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
                Err(_err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not handle command {_err:?}")
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
                Err(_err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not handle event {_err}");
                    }
                }
            }
        }
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = Terminal::set_cursor_style(SetCursorStyle::DefaultUserShape);
        let _ = Terminal::terminate();
        let _ = Terminal::print("Goodbye.\r\n");
    }
}
