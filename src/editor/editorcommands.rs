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
    JumpWord(Direction),
    JumpLine,
    Highlight,
    Paste,
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
                (KeyCode::Char('j'), KeyModifiers::CONTROL) => Ok(Self::JumpLine),
                (KeyCode::Char('l'), KeyModifiers::CONTROL) => Ok(Self::Move(Direction::Home)),
                (KeyCode::Char('u'), KeyModifiers::CONTROL) => Ok(Self::Move(Direction::PageUp)),
                (KeyCode::Char('d'), KeyModifiers::CONTROL) => Ok(Self::Move(Direction::PageDown)),
                (KeyCode::Char('r'), KeyModifiers::CONTROL) => Ok(Self::Move(Direction::End)),
                (KeyCode::Char('w'), KeyModifiers::CONTROL) => Ok(Self::Save),
                (KeyCode::Char('h'), KeyModifiers::CONTROL) => Ok(Self::Help),
                (KeyCode::Char('f'), KeyModifiers::CONTROL) => Ok(Self::Search),
                (KeyCode::Char('t'), KeyModifiers::CONTROL) => Ok(Self::Theme),
                (KeyCode::Char('v'), KeyModifiers::CONTROL) => Ok(Self::Paste),
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => Ok(Self::Highlight),
                (KeyCode::Left, KeyModifiers::SHIFT) => Ok(Self::JumpWord(Direction::Left)),
                (KeyCode::Right, KeyModifiers::SHIFT) => Ok(Self::JumpWord(Direction::Right)),
                (KeyCode::Up, _) => Ok(Self::Move(Direction::Up)),
                (KeyCode::Down, _) => Ok(Self::Move(Direction::Down)),
                (KeyCode::Left, _) => Ok(Self::Move(Direction::Left)),
                (KeyCode::Right, _) => Ok(Self::Move(Direction::Right)),
                (KeyCode::Char(c), _) => Ok(Self::Insert(c)),
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

#[derive(Copy, Clone)]
pub enum SearchCommand {
    Insert(char),
    Next,
    Previous,
    BackSpace,
    RevertState,
    AssumeState,
    Resize(Size),
    NoAction,
}

impl TryFrom<Event> for SearchCommand {
    type Error = String;
    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => match (code, modifiers) {
                (KeyCode::Char('n'), KeyModifiers::CONTROL) => Ok(Self::Next),
                (KeyCode::Char('p'), KeyModifiers::CONTROL) => Ok(Self::Previous),
                (KeyCode::Char(c), _) => Ok(Self::Insert(c)),
                (KeyCode::Enter, _) => Ok(Self::AssumeState),
                (KeyCode::Esc, _) => Ok(Self::RevertState),
                (KeyCode::Backspace, _) => Ok(Self::BackSpace),
                _ => Ok(Self::NoAction),
            },
            Event::Resize(width_u16, height_u16) => Ok(Self::Resize(Size {
                height: height_u16 as usize,
                width: width_u16 as usize,
            })),
            _ => Err("Invalid key press read".into()),
        }
    }
}

pub enum HighlightCommand {
    RevertState,
    Copy,
    Resize(Size),
    Move(Direction),
    NoAction,
}

impl TryFrom<Event> for HighlightCommand {
    type Error = String;
    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => match (code, modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => Ok(Self::Copy),
                (KeyCode::Up, _) => Ok(Self::Move(Direction::Up)),
                (KeyCode::Down, _) => Ok(Self::Move(Direction::Down)),
                (KeyCode::Right, _) => Ok(Self::Move(Direction::Right)),
                (KeyCode::Left, _) => Ok(Self::Move(Direction::Left)),
                (KeyCode::Esc, _) => Ok(Self::RevertState),
                _ => Ok(Self::NoAction),
            },
            Event::Resize(width_u16, height_u16) => Ok(Self::Resize(Size {
                height: height_u16 as usize,
                width: width_u16 as usize,
            })),
            _ => Err("Invalid key press read".into()),
        }
    }
}

pub enum FileNameCommand {
    Insert(char),
    BackSpace,
    SaveFileName,
    NoAction,
}

impl TryFrom<Event> for FileNameCommand {
    type Error = String;
    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Char(c) => Ok(Self::Insert(c)),
                KeyCode::Backspace => Ok(Self::BackSpace),
                KeyCode::Enter => Ok(Self::SaveFileName),
                _ => Ok(Self::NoAction),
            },
            _ => Ok(Self::NoAction),
        }
    }
}
