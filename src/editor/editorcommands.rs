use super::terminal::Size;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::convert::TryFrom;

#[derive(Copy, Clone)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    End,
    Home,
}

#[derive(Copy, Clone)]
pub enum EditorCommand {
    Move(Direction),
    Insert(char),
    Resize(Size),
    Jump,
    Tab,
    NewLine,
    Save,
    Theme,
    Delete,
    Search,
    Help,
    None,
    Quit,
}

impl TryFrom<Event> for EditorCommand {
    type Error = String;
    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => match (code, modifiers) {
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => Ok(Self::Quit),
                (KeyCode::Char('j'), KeyModifiers::CONTROL) => Ok(Self::Jump),
                (KeyCode::Char('l'), KeyModifiers::CONTROL) => Ok(Self::Move(Direction::Home)),
                (KeyCode::Char('u'), KeyModifiers::CONTROL) => Ok(Self::Move(Direction::PageUp)),
                (KeyCode::Char('d'), KeyModifiers::CONTROL) => Ok(Self::Move(Direction::PageDown)),
                (KeyCode::Char('r'), KeyModifiers::CONTROL) => Ok(Self::Move(Direction::End)),
                (KeyCode::Char('w'), KeyModifiers::CONTROL) => Ok(Self::Save),
                (KeyCode::Char('h'), KeyModifiers::CONTROL) => Ok(Self::Help),
                (KeyCode::Char('f'), KeyModifiers::CONTROL) => Ok(Self::Search),
                (KeyCode::Char('t'), KeyModifiers::CONTROL) => Ok(Self::Theme),
                (KeyCode::Up, _) => Ok(Self::Move(Direction::Up)),
                (KeyCode::Down, _) => Ok(Self::Move(Direction::Down)),
                (KeyCode::Left, _) => Ok(Self::Move(Direction::Left)),
                (KeyCode::Right, _) => Ok(Self::Move(Direction::Right)),
                (KeyCode::Char(char), _) => Ok(Self::Insert(char)),
                (KeyCode::Backspace, _) => Ok(Self::Delete),
                (KeyCode::Enter, _) => Ok(Self::NewLine),
                (KeyCode::Tab, _) => Ok(Self::Tab),
                _ => Ok(Self::None),
            },
            Event::Resize(width_16, height_u16) => {
                #[allow(clippy::as_conversions)]
                let height = height_u16 as usize;
                #[allow(clippy::as_conversions)]
                let width = width_16 as usize;
                Ok(Self::Resize(Size { height, width }))
            }
            _ => Err(format!("Event not supported {event:?}")),
        }
    }
}
