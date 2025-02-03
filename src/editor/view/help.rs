use super::Size;
use crate::editor::editorcommands::HelpCommand;
use crate::editor::terminal::{Position, Terminal};
use crossterm::event::{read, Event, KeyEvent};
use crossterm::style::{Color, PrintStyledContent, StyledContent, Stylize};

// trying to get the help mapping items map at comptime
// since these are static
struct HelpItemMap {
    offset: usize,
    help_str: &'static str,
}

enum HelpKeys {
    Save,
    Quit,
    JumpTo,
    Search,
    SnapUp,
    SnapDown,
    Highlight,
    VimMode,
}

impl From<&'static str> for HelpKeys {
    fn from(val: &'static str) -> Self {
        match val {
            "Save" => Self::Save,
            "Quit" => Self::Quit,
            "JumpTo" => Self::JumpTo,
            "Search" => Self::Search,
            "SnapUp" => Self::SnapUp,
            "SnapDown" => Self::SnapDown,
            "Highlight" => Self::Highlight,
            "VimMode" => Self::VimMode,
            _ => panic!("Unsupported item"),
        }
    }
}

impl HelpKeys {
    const fn value(self) -> &'static HelpItemMap {
        match self {
            Self::Save => &HelpItemMap {
                offset: 2,
                help_str: "Ctrl-w = save       ",
            },
            Self::Quit => &HelpItemMap {
                offset: 3,
                help_str: "Ctrl-q = quit       ",
            },
            Self::JumpTo => &HelpItemMap {
                offset: 4,
                help_str: "Ctrl-j = jump-to    ",
            },
            Self::Search => &HelpItemMap {
                offset: 5,
                help_str: "Ctrl-f = search     ",
            },
            Self::SnapUp => &HelpItemMap {
                offset: 6,
                help_str: "Ctrl-u = snap-up    ",
            },
            Self::SnapDown => &HelpItemMap {
                offset: 7,
                help_str: "Ctrl-d = snap-down  ",
            },
            Self::Highlight => &HelpItemMap {
                offset: 8,
                help_str: "Ctrl-c = highlight  ",
            },
            Self::VimMode => &HelpItemMap {
                offset: 9,
                help_str: "Ctrl-n = vim mode   ",
            },
        }
    }
}

const HELP_ITEMS: [&str; 8] = [
    "Save",
    "Quit",
    "JumpTo",
    "Search",
    "SnapUp",
    "SnapDown",
    "Highlight",
    "VimMode",
];

pub struct Help;
impl Help {
    pub fn render_help(size: &mut Size, h_color: Color, t_color: Color) {
        //render the help commands
        //clear lines size - 1
        //up to size - n up to number of help commands
        //go back on esc
        //like nvim
        Terminal::hide_cursor().unwrap();
        Self::render(size, h_color, t_color);
        loop {
            let Ok(read_event) = read() else { continue };
            match HelpCommand::try_from(read_event) {
                Ok(command) => match command {
                    HelpCommand::Exit => break,
                    HelpCommand::NoAction => continue,
                    HelpCommand::Resize(new_size) => {
                        *size = new_size;

                        Self::render(size, h_color, t_color);
                    }
                },
                Err(_) => continue,
            }
        }
        Terminal::show_cursor().unwrap();
        Terminal::execute().unwrap();
    }

    fn render(size: &Size, h_color: Color, t_color: Color) {
        for item in &HELP_ITEMS {
            let help_map = HelpKeys::from(*item).value();
            let highlight_seg: StyledContent<String> =
                help_map.help_str.to_owned().with(t_color).on(h_color);
            Terminal::move_cursor_to(Position {
                height: size.height.saturating_sub(help_map.offset),
                width: 0,
            })
            .unwrap();

            Terminal::queue_command(PrintStyledContent(highlight_seg)).unwrap();
        }
        Terminal::execute().unwrap();
    }
}

const VIM_BINDINGS: [&str; 12] = [
    "Jump To Begining Of Next Word",
    "Jump To End Of Current Word",
    "Jump to Begining Of Current Word",
    "Page Up",
    "Page Down",
    "Page Left",
    "Page Right",
    "Right",
    "Left",
    "Up",
    "Down",
    "Exit",
];

enum VimKeyBindings {
    JumpToBeginingOfNextWord,
    JumpToEndOfCurrentWord,
    JumpToBeginingOfCurrentWord,
    PageUp,
    PageDown,
    PageRight,
    PageLeft,
    Right,
    Left,
    Up,
    Down,
    Exit,
}

impl From<&'static str> for VimKeyBindings {
    fn from(v: &'static str) -> Self {
        match v {
            "Jump To Begining Of Next Word" => Self::JumpToBeginingOfNextWord,
            "Jump To End Of Current Word" => Self::JumpToEndOfCurrentWord,
            "Jump to Begining Of Current Word" => Self::JumpToBeginingOfCurrentWord,
            "Page Up" => Self::PageUp,
            "Page Down" => Self::PageDown,
            "Right" => Self::Right,
            "Left" => Self::Left,
            "Up" => Self::Up,
            "Down" => Self::Down,
            "Page Left" => Self::PageLeft,
            "Page Right" => Self::PageRight,
            _ => Self::Exit,
        }
    }
}

struct VimItemHelpMap {
    offset: usize,
    help_str: &'static str,
}

impl VimKeyBindings {
    const fn value(self) -> &'static VimItemHelpMap {
        match self {
            VimKeyBindings::JumpToBeginingOfNextWord => &VimItemHelpMap {
                offset: 1,
                help_str: "w = Begining of next word     ",
            },
            VimKeyBindings::JumpToEndOfCurrentWord => &VimItemHelpMap {
                offset: 2,
                help_str: "e = End of current word       ",
            },
            VimKeyBindings::JumpToBeginingOfCurrentWord => &VimItemHelpMap {
                offset: 3,
                help_str: "b = Begining of current word  ",
            },
            VimKeyBindings::PageUp => &VimItemHelpMap {
                offset: 4,
                help_str: "gg = Page Up                  ",
            },
            VimKeyBindings::PageDown => &VimItemHelpMap {
                offset: 5,
                help_str: "GG = Page Down                ",
            },
            VimKeyBindings::Right => &VimItemHelpMap {
                offset: 6,
                help_str: "l = Right                     ",
            },
            VimKeyBindings::Left => &VimItemHelpMap {
                offset: 7,
                help_str: "h = Left                      ",
            },
            VimKeyBindings::Up => &VimItemHelpMap {
                offset: 8,
                help_str: "k = Up                        ",
            },
            VimKeyBindings::Down => &VimItemHelpMap {
                offset: 9,
                help_str: "j = Down                      ",
            },
            VimKeyBindings::Exit => &VimItemHelpMap {
                offset: 10,
                help_str: "Esc = Exit                    ",
            },
            VimKeyBindings::PageRight => &VimItemHelpMap {
                offset: 11,
                help_str: "$ = Page Right                ",
            },
            VimKeyBindings::PageLeft => &VimItemHelpMap {
                offset: 12,
                help_str: "0 = Page Left                 ",
            },
        }
    }
}

pub struct VimHelpScreen;
impl VimHelpScreen {
    pub fn render_help(size: &mut Size, h_color: Color, t_color: Color) {
        //render the help commands
        //clear lines size - 1
        //up to size - n up to number of help commands
        //go back on esc
        //like nvim
        Terminal::hide_cursor().unwrap();
        Self::render(size, h_color, t_color);
        loop {
            let Ok(event) = read() else { continue }; //clear the help screen on next key press
            match event {
                Event::Key(KeyEvent { .. }) => break,
                _ => continue,
            }
        }
    }

    fn render(size: &Size, h_color: Color, t_color: Color) {
        for item in &VIM_BINDINGS {
            let help_map = VimKeyBindings::from(*item).value();
            let highlight_seg: StyledContent<String> =
                help_map.help_str.to_owned().with(t_color).on(h_color);
            Terminal::move_cursor_to(Position {
                height: size.height.saturating_sub(help_map.offset),
                width: 0,
            })
            .unwrap();

            Terminal::queue_command(PrintStyledContent(highlight_seg)).unwrap();
        }
        Terminal::execute().unwrap();
    }
}
