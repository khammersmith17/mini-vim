use super::clipboard_interface::ClipboardUtils;
use crate::editor::Terminal;
use crate::editor::{
    editorcommands::{
        ColonQueueActions, Direction, QueueInitCommand, VimColonQueue, VimModeCommands,
    },
    view::{Buffer, Coordinate, Mode, Position, ScreenOffset, Size},
};
use crossterm::event::{read, Event, KeyCode, KeyEvent};

pub struct VimMode<'a> {
    cursor_position: Position,
    screen_offset: ScreenOffset,
    size: Size,
    buffer: &'a mut Buffer,
}

impl VimMode<'_> {
    pub fn new<'a>(
        cursor_position: Position,
        screen_offset: ScreenOffset,
        size: Size,
        buffer: &'a mut Buffer,
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
    ) -> bool {
        self.status_line();
        Terminal::move_cursor_to(
            self.cursor_position
                .relative_view_position(&self.screen_offset),
        )
        .unwrap();
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
                        }
                        _ => continue,
                    },
                    VimModeCommands::StartOfNextWord => {
                        self.buffer.begining_of_next_word(&mut self.cursor_position)
                    }
                    VimModeCommands::EndOfCurrentWord => {
                        self.buffer.end_of_current_word(&mut self.cursor_position)
                    }
                    VimModeCommands::BeginingOfCurrentWord => self
                        .buffer
                        .begining_of_current_word(&mut self.cursor_position),
                    VimModeCommands::ComplexCommand(queue_command) => {
                        // if we get true back, staying in vim mode
                        // else user is exiting the session
                        if !self.determine_queue_command(queue_command) {
                            return false;
                        }
                    }
                    VimModeCommands::Resize(new_size) => self.resize(new_size),
                    VimModeCommands::Exit => {
                        // here user is staying in terminal session
                        // but exiting vim mode
                        self.hand_back_state(cursor_position, screen_offset, size);
                        return true;
                    }
                    VimModeCommands::Paste => self.add_to_clipboard(),
                    VimModeCommands::NoAction => continue, // skipping other
                },
                Err(_) => continue, //ignoring error
            }

            Terminal::hide_cursor().unwrap();
            Terminal::move_cursor_to(self.screen_offset.to_position()).unwrap();
            Terminal::clear_screen().unwrap();
            self.render();
            self.status_line();
            Terminal::move_cursor_to(
                self.cursor_position
                    .relative_view_position(&self.screen_offset),
            )
            .unwrap();
            Terminal::show_cursor().unwrap();
            Terminal::execute().unwrap();
        }
    }

    fn add_to_clipboard(&mut self) {
        let paste_text = ClipboardUtils::get_text_from_clipboard();
        if paste_text.is_ok() {
            self.buffer
                .add_text_from_clipboard(&paste_text.unwrap(), &mut self.cursor_position);
        }
    }

    #[inline]
    fn status_line(&self) {
        Terminal::render_status_line(
            Mode::VimMode,
            self.buffer.is_saved,
            &self.size,
            self.buffer.filename.as_deref(),
            Some((
                self.cursor_position.height.saturating_add(1),
                self.buffer.len(),
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
            self.move_and_resolve(dir);
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

    #[inline]
    fn determine_queue_command(&mut self, command: QueueInitCommand) -> bool {
        // propogate up the result of the typed command
        // otherwise we are staying in terminal session, thus true
        match command {
            QueueInitCommand::Colon => self.queue_colon(),
            QueueInitCommand::PageUp => {
                self.queue_page_up();
                // stay in vim mode
                return true;
            }
            QueueInitCommand::PageDown => {
                self.queue_page_down();
                // stay in vim mode
                return true;
            }
        }
    }

    fn queue_colon(&mut self) -> bool {
        // return true if we are staying in vim mode after executing command
        // return false if we are ending the terminal session from here
        // in the case the command executes, propogate up the state result
        let mut queue: String = String::new();
        self.command_status_line(&queue);

        loop {
            let Ok(read_event) = read() else { continue }; //skipping an error on read cursor action
            match VimColonQueue::try_from(read_event) {
                Ok(event) => match event {
                    VimColonQueue::New(c) => queue.push(c), //queue any of these commands
                    VimColonQueue::Backspace => {
                        if queue.is_empty() {
                            return true;
                        }
                        let _ = queue.pop();
                    }
                    VimColonQueue::Execute => {
                        let Ok(mapped) = Self::map_string_to_queue_vec(&queue) else {
                            self.command_status_line("Invalid command");
                            queue.clear();
                            continue;
                        };
                        // execute action
                        return self.eval_colon_queue(mapped);
                    }
                    VimColonQueue::Resize(size) => self.resize(size),
                    VimColonQueue::Other => continue,
                },
                Err(_) => continue,
            }
            self.command_status_line(&queue);
        }
    }

    fn command_status_line(&self, message: &str) {
        Terminal::render_line(self.size.height.saturating_sub(2), format!(":{message}")).unwrap();
        Terminal::execute().unwrap();
    }

    fn eval_colon_queue(&mut self, queue: Vec<ColonQueueActions>) -> bool {
        // return true if we are staying in vim mode after executing the command
        // false if we are ending our terminal session
        match queue.len() {
            1 => match queue[0] {
                ColonQueueActions::Write => {
                    // execute and stay in vim mode
                    self.buffer.save();
                }
                ColonQueueActions::Quit => {
                    // figure out how to propogate up quit
                    // exit session
                    return false;
                }
                ColonQueueActions::Override => self.command_status_line("Invalid command"),
            },
            2 => {
                match queue.as_slice() {
                    [ColonQueueActions::Write, ColonQueueActions::Quit] => {
                        self.buffer.save();
                        // figure out how to propogate up quit
                        // exit terminal session
                        return false;
                    }
                    [ColonQueueActions::Quit, ColonQueueActions::Override] => {
                        //figure out how to propogate up quit
                        //exit terminal session
                        return false;
                    }
                    _ => self.command_status_line("Invalid command"),
                }
            }
            _ => self.command_status_line("Invalid command"),
        }
        true
    }

    fn map_string_to_queue_vec(string_queue: &str) -> Result<Vec<ColonQueueActions>, String> {
        let mut res: Vec<ColonQueueActions> = Vec::new();
        for c in string_queue.chars() {
            let mapped_val = ColonQueueActions::try_from(c)?;
            res.push(mapped_val);
        }

        Ok(res)
    }

    fn queue_page_up(&mut self) {
        let event = Self::wait_for_successful_event();
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Char('g') => {
                    self.move_and_resolve(Direction::PageUp);
                }
                _ => {}
            }
        }
    }

    fn queue_page_down(&mut self) {
        let event = Self::wait_for_successful_event();
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Char('G') => {
                    self.move_and_resolve(Direction::PageDown);
                }
                _ => {}
            }
        }
    }

    #[inline]
    fn move_and_resolve(&mut self, dir: Direction) {
        dir.move_cursor(&mut self.cursor_position, &*self.buffer);
        self.resolve_displacement();
    }

    #[inline]
    fn wait_for_successful_event() -> Event {
        // we are waiting on a single event
        // so wait for an ok event
        loop {
            let Ok(read_event) = read() else { continue };
            return read_event;
        }
    }
}
