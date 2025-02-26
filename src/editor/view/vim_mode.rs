use super::clipboard_interface::ClipboardUtils;
use crate::editor::Terminal;
use crate::editor::{
    editorcommands::{
        parse_highlight_vim_mode, ColonQueueActions, Direction, QueueInitCommand, VimColonQueue,
        VimModeCommands,
    },
    view::{
        help::VimHelpScreen, highlight::Highlight, Buffer, Coordinate, Mode, Position,
        ScreenOffset, Size,
    },
};
use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::style::Color;
use std::error::Error;

enum ContinueState {
    ExitSession,
    ContinueVim,
    ContinueVimPersistError,
    InvalidCommand,
    JumpCursor(usize),
}

pub struct VimMode<'a> {
    cursor_position: Position,
    screen_offset: ScreenOffset,
    size: Size,
    buffer: &'a mut Buffer,
}

impl VimMode<'_> {
    pub fn new(
        cursor_position: Position,
        screen_offset: ScreenOffset,
        size: Size,
        buffer: &'_ mut Buffer, // mutable reference to buffer
    ) -> VimMode<'_> {
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
        h_color: Color,
        t_color: Color,
    ) -> bool {
        let res = self.start();
        debug_assert!(res.is_ok());
        loop {
            let mut needs_render = false;
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
                            if self.move_cursor(dir) > 0 {
                                needs_render = true;
                            }
                        }
                        _ => continue,
                    },
                    VimModeCommands::JumpUp => {
                        if self.jump_up() > 0 {
                            needs_render = true;
                        }
                    }
                    VimModeCommands::JumpDown => {
                        if self.jump_down() > 0 {
                            needs_render = true;
                        }
                    }
                    VimModeCommands::NewLine => {
                        // new line
                        self.buffer.add_new_line(&mut self.cursor_position);
                        needs_render = true;
                    }
                    VimModeCommands::StartOfNextWord => {
                        self.buffer.begining_of_next_word(&mut self.cursor_position);
                    }
                    VimModeCommands::EndOfCurrentWord => {
                        self.buffer.end_of_current_word(&mut self.cursor_position);
                    }
                    VimModeCommands::BeginingOfCurrentWord => self
                        .buffer
                        .begining_of_current_word(&mut self.cursor_position),
                    VimModeCommands::ComplexCommand(queue_command) => {
                        // if we get true back, staying in vim mode
                        // else user is exiting the session
                        match self.determine_queue_command(&queue_command) {
                            ContinueState::ContinueVimPersistError => continue,
                            ContinueState::ContinueVim => {
                                needs_render = true;
                            } // no action
                            ContinueState::InvalidCommand => {
                                // if the command is invalid, render the help
                                VimHelpScreen::render_help(&mut self.size, h_color, t_color);
                            }
                            ContinueState::JumpCursor(line) => {
                                if self.jump_cursor_to(line) > 0 {
                                    needs_render = true;
                                }
                            }
                            ContinueState::ExitSession => return false,
                        }
                    }
                    VimModeCommands::Highlight => {
                        let mut highlight = Highlight::new(
                            &mut self.cursor_position,
                            self.screen_offset,
                            &mut self.size,
                            self.buffer,
                        );
                        highlight.run(h_color, t_color, parse_highlight_vim_mode);
                        if self.resolve_displacement() > 0 {
                            needs_render = true;
                        } // making sure the offset is correct on a delete
                    }
                    VimModeCommands::Resize(new_size) => {
                        self.resize(new_size);
                        needs_render = true;
                    }
                    VimModeCommands::Exit => {
                        // here user is staying in terminal session
                        // but exiting vim mode
                        self.hand_back_state(cursor_position, screen_offset, size);
                        return true;
                    }
                    VimModeCommands::Paste => {
                        self.add_from_clipboard();
                        needs_render = true;
                    }
                    VimModeCommands::NoAction => {
                        VimHelpScreen::render_help(&mut self.size, h_color, t_color);
                        needs_render = true;
                    } // skipping other
                },
                Err(_) => continue, //ignoring error
            }
            if needs_render {
                let res = self.render_proc();
                debug_assert!(res.is_ok());
            }

            let res = self.cursor_and_status();
            debug_assert!(res.is_ok());
        }
    }

    fn jump_cursor_to(&mut self, line: usize) -> usize {
        self.cursor_position.height = std::cmp::min(line, self.buffer.len().saturating_sub(1));
        self.resolve_displacement()
    }

    fn jump_down(&mut self) -> usize {
        self.cursor_position.height = std::cmp::min(
            self.cursor_position.height.saturating_add(10),
            self.buffer.len().saturating_sub(1),
        );
        self.resolve_displacement()
    }

    fn jump_up(&mut self) -> usize {
        self.cursor_position.height = self.cursor_position.height.saturating_sub(10);
        self.resolve_displacement()
    }

    #[inline]
    fn cursor_and_status(&self) -> Result<(), Box<dyn Error>> {
        self.status_line()?;
        Terminal::move_cursor_to(
            self.cursor_position
                .relative_view_position(&self.screen_offset),
        )?;

        Terminal::show_cursor()?;
        Terminal::execute()?;
        Ok(())
    }

    fn start(&self) -> Result<(), Box<dyn Error>> {
        self.status_line()?;
        Terminal::move_cursor_to(
            self.cursor_position
                .relative_view_position(&self.screen_offset),
        )?;
        Terminal::execute()?;
        Ok(())
    }

    #[inline]
    fn render_proc(&self) -> Result<(), Box<dyn Error>> {
        Terminal::hide_cursor()?;
        Terminal::move_cursor_to(self.screen_offset.to_position())?;
        Terminal::clear_screen()?;
        self.render()?;

        Ok(())
    }

    fn add_from_clipboard(&mut self) {
        if let Ok(paste_text) = ClipboardUtils::get_text_from_clipboard() {
            self.buffer
                .add_text_from_clipboard(&paste_text, &mut self.cursor_position);
        }
    }

    #[inline]
    fn status_line(&self) -> Result<(), Box<dyn Error>> {
        Terminal::render_status_line(
            &Mode::Vim,
            self.buffer.is_saved,
            &self.size,
            self.buffer.filename.as_deref(),
            Some((
                self.cursor_position.height.saturating_add(1),
                self.buffer.len(),
            )),
        )?;
        Ok(())
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

    fn render(&self) -> Result<(), Box<dyn Error>> {
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
                )?;
            } else if self.buffer.is_empty() && (current_row == self.size.height / 3) {
                Terminal::render_line(
                    relative_row,
                    Terminal::get_welcome_message(&self.size, &self.screen_offset),
                )?;
            } else {
                Terminal::render_line(relative_row, "~")?;
            }
        }
        Ok(())
    }

    // handing back view delta
    #[inline]
    fn move_cursor(&mut self, dir: Direction) -> usize {
        if self.buffer.is_empty() {
            self.cursor_position.snap_left();
            self.cursor_position.page_up();
            0
        } else {
            self.move_and_resolve(dir)
        }
    }

    fn resolve_displacement(&mut self) -> usize {
        let dis =
            self.cursor_position
                .max_displacement_from_view(&self.screen_offset, &self.size, 2);
        match dis {
            0 => {}
            1 => {
                self.screen_offset
                    .update_offset_single_move(&self.cursor_position, &self.size, 1);
            }
            _ => {
                self.screen_offset.handle_offset_screen_snap(
                    &self.cursor_position,
                    &self.size,
                    1,
                    self.buffer.len(),
                );
            }
        }
        dis
    }

    #[inline]
    fn determine_queue_command(&mut self, command: &QueueInitCommand) -> ContinueState {
        // propogate up the result of the typed command
        // otherwise we are staying in terminal session, thus true
        match command {
            QueueInitCommand::Colon => self.queue_colon(),
            QueueInitCommand::PageUp => {
                let valid = self.queue_page_up();
                // stay in vim mode
                if valid {
                    ContinueState::ContinueVim
                } else {
                    ContinueState::InvalidCommand
                }
            }
            QueueInitCommand::PageDown => {
                let valid = self.queue_page_down();
                // stay in vim mode
                if valid {
                    ContinueState::ContinueVim
                } else {
                    ContinueState::InvalidCommand
                }
            }
            QueueInitCommand::Delete => {
                // delete the block associated with the next key press
                if self.queue_delete() {
                    ContinueState::ContinueVim
                } else {
                    ContinueState::InvalidCommand
                }
            }
            QueueInitCommand::Yank => {
                // copy the block associated with the next key press
                if self.queue_yank() {
                    ContinueState::ContinueVim
                } else {
                    ContinueState::InvalidCommand
                }
            }
        }
    }

    fn queue_colon(&mut self) -> ContinueState {
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
                            return ContinueState::ContinueVim;
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
                        return self.eval_colon_queue(&mapped);
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
        let render =
            Terminal::render_line(self.size.height.saturating_sub(2), format!(":{message}"));
        let flush = Terminal::execute();
        debug_assert!(render.is_ok() & flush.is_ok());
    }

    fn eval_colon_queue(&mut self, queue: &[ColonQueueActions]) -> ContinueState {
        // return true if we are staying in vim mode after executing the command
        // false if we are ending our terminal session
        match queue.len() {
            1 => match queue[0] {
                ColonQueueActions::Write => {
                    // execute and stay in vim mode
                    self.buffer.save();
                }
                ColonQueueActions::Quit => {
                    // exit session
                    if !self.buffer.is_saved {
                        self.command_status_line("not saved: ! to override, w: to save");
                        return ContinueState::ContinueVimPersistError;
                    }
                    return ContinueState::ExitSession;
                }
                ColonQueueActions::Override => {
                    self.command_status_line("Invalid command");
                    return ContinueState::ContinueVimPersistError;
                }
                ColonQueueActions::Jump(line) => {
                    // jump to the line
                    // continue state is continue vim
                    // figure out where to do the rendering if the cursor moves off screen
                    return ContinueState::JumpCursor(line);
                }
            },
            2 => {
                match queue {
                    [ColonQueueActions::Write, ColonQueueActions::Quit] => {
                        self.buffer.save();
                        // exit terminal session
                        return ContinueState::ExitSession;
                    }
                    [ColonQueueActions::Quit, ColonQueueActions::Override] => {
                        //exit terminal session
                        return ContinueState::ExitSession;
                    }
                    _ => self.command_status_line("Invalid command!"),
                }
            }
            _ => self.command_status_line("Invalid command!"),
        }
        ContinueState::ContinueVim
    }

    fn map_string_to_queue_vec(string_queue: &str) -> Result<Vec<ColonQueueActions>, String> {
        if let Ok(line) = string_queue.parse::<usize>() {
            return Ok(vec![ColonQueueActions::Jump(line)]);
        }
        let mut res: Vec<ColonQueueActions> = Vec::new();
        for c in string_queue.chars() {
            let mapped_val = ColonQueueActions::try_from(c)?;
            res.push(mapped_val);
        }

        Ok(res)
    }

    fn queue_page_up(&mut self) -> bool {
        // bool propogates up an invalid complex command
        let event = Self::wait_for_successful_event();
        if let Event::Key(KeyEvent { code, .. }) = event {
            if matches!(code, KeyCode::Char('g')) {
                //only handling if gg, otherwise skip
                self.move_and_resolve(Direction::PageUp);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn queue_page_down(&mut self) -> bool {
        // bool propogates up an invalid complex command
        let event = Self::wait_for_successful_event();
        if let Event::Key(KeyEvent { code, .. }) = event {
            if matches!(code, KeyCode::Char('G')) {
                // only handling if GG otherwise skip
                self.move_and_resolve(Direction::PageDown);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    #[inline]
    fn move_and_resolve(&mut self, dir: Direction) -> usize {
        dir.move_cursor(&mut self.cursor_position, self.buffer);
        self.resolve_displacement()
    }

    fn wait_for_successful_event() -> Event {
        // we are waiting on a single event
        // so wait for an ok event
        loop {
            let Ok(read_event) = read() else { continue };
            return read_event;
        }
    }

    fn queue_delete(&mut self) -> bool {
        let event = Self::wait_for_successful_event();
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Char('w') => {
                    let mut right = self.cursor_position;
                    self.buffer.begining_of_next_word(&mut right);
                    self.buffer
                        .delete_segment(&self.cursor_position, &mut right);
                }
                KeyCode::Char('b') => {
                    let mut left = self.cursor_position;
                    self.buffer.begining_of_current_word(&mut left);
                    self.buffer.delete_segment(&left, &mut self.cursor_position);
                }
                KeyCode::Char('d') => {
                    self.buffer.pop_line(self.cursor_position.height);
                }
                KeyCode::Char('e') => {
                    let mut right = self.cursor_position;
                    self.buffer.end_of_current_word(&mut right);
                    self.buffer
                        .delete_segment(&self.cursor_position, &mut right);
                }
                _ => return false,
            }
        } else {
            return false;
        };
        true
    }

    fn queue_yank(&mut self) -> bool {
        let event = Self::wait_for_successful_event();
        if let Event::Key(KeyEvent { code, .. }) = event {
            let copy_string = match code {
                KeyCode::Char('w') => {
                    let mut right = self.cursor_position;
                    self.buffer.begining_of_next_word(&mut right);
                    self.buffer.get_segment(&self.cursor_position, &right)
                }
                KeyCode::Char('b') => {
                    let mut left = self.cursor_position;
                    self.buffer.begining_of_current_word(&mut left);

                    self.buffer.get_segment(&left, &self.cursor_position)
                }
                KeyCode::Char('y') => self.buffer.text[self.cursor_position.height]
                    .raw_string
                    .clone(),
                KeyCode::Char('e') => {
                    let mut right = self.cursor_position;
                    self.buffer.end_of_current_word(&mut right);
                    self.buffer.get_segment(&self.cursor_position, &right)
                }
                _ => return false,
            };
            if !copy_string.is_empty() {
                let res = ClipboardUtils::copy_text_to_clipboard(copy_string);
                debug_assert!(res.is_ok());
            }
        } else {
            return false;
        };
        true
    }
}
