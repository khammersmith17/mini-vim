use crate::editor::Terminal;
use crate::editor::{
    editorcommands::{Direction, VimModeCommands},
    view::{Buffer, Coordinate, Mode, Position, ScreenOffset, Size},
};
use crossterm::event::read;

pub struct VimMode<'a> {
    cursor_position: Position,
    screen_offset: ScreenOffset,
    size: Size,
    buffer: &'a Buffer,
}

/*
enum QueueCommands {
    UpperG,
    LowerG,
    Colon,
    Write,
    Quit,
}
*/

impl VimMode<'_> {
    pub fn new<'a>(
        cursor_position: Position,
        screen_offset: ScreenOffset,
        size: Size,
        buffer: &'a Buffer,
    ) -> VimMode<'a> {
        VimMode {
            cursor_position,
            screen_offset,
            size,
            buffer,
        }
    }
    pub fn run(
        &mut self,
        cursor_position: &mut Position,
        screen_offset: &mut ScreenOffset,
        size: &mut Size,
    ) {
        self.render_status_line();
        Terminal::move_cursor_to(self.cursor_position.view_height(&self.screen_offset)).unwrap();
        Terminal::execute().unwrap();
        loop {
            let Ok(read_event) = read() else { continue }; //skipping an error on read cursor action

            match VimModeCommands::try_from(read_event) {
                Ok(event) => match event {
                    VimModeCommands::Move(dir) => match dir {
                        Direction::Right
                        | Direction::Left
                        | Direction::Up
                        | Direction::Down
                        | Direction::End
                        | Direction::Home => {
                            self.move_cursor(dir);
                            self.resolve_displacement();
                        }
                        _ => continue,
                    },
                    VimModeCommands::Resize(new_size) => self.resize(new_size),
                    VimModeCommands::Exit => {
                        self.hand_back_state(cursor_position, screen_offset, size);
                        return;
                    }
                    VimModeCommands::NoAction => continue, // skipping other
                },
                Err(_) => continue, //ignoring error
            }

            Terminal::hide_cursor().unwrap();
            Terminal::move_cursor_to(self.screen_offset.to_position()).unwrap();
            Terminal::clear_screen().unwrap();
            self.render();
            self.render_status_line();
            Terminal::move_cursor_to(self.cursor_position.view_height(&self.screen_offset))
                .unwrap();
            Terminal::show_cursor().unwrap();
            Terminal::execute().unwrap();
        }
    }

    #[inline]
    fn render_status_line(&self) {
        Terminal::render_status_line(
            Mode::VimMode,
            self.buffer.is_saved,
            &self.size,
            self.buffer.filename.as_deref(),
            Some((
                self.cursor_position.height.saturating_add(1),
                self.buffer.len().saturating_add(1),
            )),
        )
        .unwrap();
    }

    fn resize(&mut self, new_size: Size) {
        self.size = new_size;
    }

    fn hand_back_state(&self, pos: &mut Position, offset: &mut ScreenOffset, size: &mut Size) {
        *pos = self.cursor_position;
        *offset = self.screen_offset;
        if *size != self.size {
            *size = self.size;
        }
    }

    fn render(&self) {
        if (self.size.width == 0) | (self.size.height == 0) {
            return;
        }

        #[allow(clippy::integer_division)]
        for current_row in self.screen_offset.height
            ..self
                .screen_offset
                .height
                .saturating_add(self.size.height)
                .saturating_sub(1)
        {
            let relative_row = current_row.saturating_sub(self.screen_offset.height);

            if let Some(line) = self.buffer.text.get(current_row) {
                Terminal::render_line(
                    relative_row,
                    line.get_line_subset(
                        self.screen_offset.width
                            ..self.screen_offset.width.saturating_add(self.size.width),
                    ),
                )
                .unwrap();
            } else if self.buffer.is_empty() && (current_row == self.size.height / 3) {
                Terminal::render_line(
                    relative_row,
                    Terminal::get_welcome_message(&self.size, &self.screen_offset),
                )
                .unwrap();
            } else {
                Terminal::render_line(relative_row, "~").unwrap();
            }
        }
    }

    fn move_cursor(&mut self, dir: Direction) {
        if self.buffer.is_empty() {
            self.cursor_position.snap_left();
            self.cursor_position.page_up();
        } else {
            dir.move_cursor(&mut self.cursor_position, &self.buffer);
        }
    }

    fn resolve_displacement(&mut self) {
        let dis =
            self.cursor_position
                .max_displacement_from_view(&self.screen_offset, &self.size, 2);
        if dis == 1 {
            self.screen_offset
                .update_offset_single_move(&self.cursor_position, &self.size, 1);
        } else if dis > 1 {
            self.screen_offset.handle_offset_screen_snap(
                &self.cursor_position,
                &self.size,
                1,
                self.buffer.len(),
            );
        }
    }
}
