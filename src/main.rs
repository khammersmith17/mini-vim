#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::print_stdout,
    clippy::as_conversions
)]
mod editor;
use editor::terminal::Terminal;
use editor::Editor;

fn main() {
    let loader = Editor::new();
    match loader {
        Ok(mut editor) => match editor.run() {
            Ok(()) => {}
            Err(e) => {
                let _ = Terminal::terminate();
                let _ = Terminal::print(format!("MiniVim Error:\n{e}"));
            }
        },
        Err(e) => {
            let _ = Terminal::terminate();
            let _ = Terminal::print(format!("MiniVim Error:\n{e}"));
        }
    }
}
