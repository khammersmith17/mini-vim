#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::integer_division
)]
mod editor;
use editor::terminal::Terminal;
use editor::Editor;

fn main() {
    let loader = Editor::new();
    match loader {
        Ok(mut editor) => match editor.run() {
            Ok(_) => {}
            Err(e) => {
                let _ = Terminal::terminate();
                println!("MiniVim Error:\n{}", e);
            }
        },
        Err(e) => {
            let _ = Terminal::terminate();
            println!("MiniVim Error:\n{}", e);
        }
    }
}
