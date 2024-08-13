use crossterm::event::{read, Event, Event::Key, KeyCode::Char, KeyEvent, KeyModifiers};
use crossterm::terminal::{size, enable_raw_mode, disable_raw_mode, Clear, ClearType};
use crossterm::execute;
use crossterm::cursor::MoveTo;
use std::io::stdout;


pub struct Editor{
    should_quit: bool,
}

impl Editor {
    pub fn new() -> Self {
        Editor{
            should_quit: false
        }
    }

    pub fn run(&mut self){
        Self::initialize().unwrap();
        Self::draw_rows().unwrap();
        let result = self.repl();
        Self::terminate().unwrap();
        result.unwrap();
    }

    fn initialize() -> Result<(), std::io::Error> {
        enable_raw_mode()?;
        Self::clear_screen()
    }

    fn terminate() -> Result<(), std::io::Error> {
        disable_raw_mode()
    }

    fn clear_screen() -> Result<(), std::io::Error> {
        let mut stdout = stdout();
        execute!(stdout, Clear(ClearType::All))
    }
    fn repl(&mut self) -> Result<(), std::io::Error> {
        loop {
            let event = read()?;
            self.evaluate_event(&event);
            self.refresh_screen()?;

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    fn evaluate_event(&mut self, event: &Event) {
        if let Key(KeyEvent {
            code, modifiers, ..
        }) = event {
            match code {
                Char('q') if *modifiers == KeyModifiers::CONTROL => {
                    self.should_quit = true;
                },
                _ => (),
            }
        }
    }

    fn refresh_screen(&self) -> Result<(), std::io::Error> {
        if self.should_quit {
            Self::clear_screen()?;
            println!("Goodbye.\r\n");
        }
        Ok(())
    }

    fn draw_rows() -> Result<(), std::io::Error> {
        let size = size()?;
        let mut stdout = stdout();
        let column: u16 = 0;
        for i in 1..size.0 {
            execute!(stdout, MoveTo(column, i))?;
            print!("~");
        }
        Ok(())
    }
}
