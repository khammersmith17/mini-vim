use super::Size;
use crate::editor::editorcommands::HelpCommand;
use crate::editor::terminal::Terminal;
use crossterm::event::read;

// trying to get the help mapping items map at comptime
// since these are static
struct HelpItemMap {
    relative_index: i8,
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
                relative_index: 2,
                help_str: "Ctrl-w = save",
            },
            Self::Quit => &HelpItemMap {
                relative_index: 3,
                help_str: "Ctrl-q = quit",
            },
            Self::JumpTo => &HelpItemMap {
                relative_index: 4,
                help_str: "Ctrl-j = jump-to",
            },
            Self::Search => &HelpItemMap {
                relative_index: 5,
                help_str: "Ctrl-f = search",
            },
            Self::SnapUp => &HelpItemMap {
                relative_index: 6,
                help_str: "Ctrl-u = snap-up",
            },
            Self::SnapDown => &HelpItemMap {
                relative_index: 7,
                help_str: "Ctrl-d = snap-down",
            },
            Self::Highlight => &HelpItemMap {
                relative_index: 8,
                help_str: "Ctrl-c = highlight",
            },
            Self::VimMode => &HelpItemMap {
                relative_index: 9,
                help_str: "Ctrl-n = vim mode",
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
    pub fn render_help(size: &mut Size) {
        //render the help commands
        //clear lines size - 1
        //up to size - n up to number of help commands
        //go back on esc
        //like nvim
        Terminal::hide_cursor().unwrap();
        Self::render(size);
        loop {
            let Ok(read_event) = read() else { continue };
            match HelpCommand::try_from(read_event) {
                Ok(command) => match command {
                    HelpCommand::Exit => break,
                    HelpCommand::NoAction => continue,
                    HelpCommand::Resize(new_size) => {
                        *size = new_size;
                        Self::render(size);
                    }
                },
                Err(_) => continue,
            }
        }
        Terminal::show_cursor().unwrap();
        Terminal::execute().unwrap();
    }

    fn render(size: &Size) {
        for item in &HELP_ITEMS {
            let help_map = HelpKeys::from(*item).value();
            #[allow(clippy::as_conversions)]
            Terminal::render_line(
                size.height.saturating_sub(help_map.relative_index as usize),
                help_map.help_str,
            )
            .unwrap();
            Terminal::execute().unwrap();
        }
    }
}
