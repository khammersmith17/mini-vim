## Mini-Vim
This is a terminal based text editor. Currently it supports any text editing (utf-8 characters work better at the moment than other characters), setting the theme, searching, resizing the terminal, and saving.

I am currently working on adding syntax highlighting and search highlighting. Eventually, I would like to also build an installer for it. The terminal interaction is using the crossterm crate.

Until the installer is ready, you can fork the repo and try yourself.
To run a fresh working file simply execute cargo run\
To edit an existing file: cargo run <filename>

## Modes
There are essentially 3 modes: normal, search, save as, and set theme.\
If there are changes to the file state when trying to exit, a message will appear asking if you want to exit without saving (Ctrl-y = Exit without saving, Ctrl-n = Save file before exit).

## Normal Mode Commands
Like other terminal based text editors, there are a number of commands.\
Ctrl-q = Quit\
Ctrl-l = Snap cursor to end of line\
Ctrl-u = Snap cursor to first line\
Ctrl-d = Snap cursor to last line\
Ctrl-r = Snap cursor to first line\
Ctrl-w = Save\
Ctrl-h = Help\
Ctrl-f = Search\
Ctrl-t = Theme

## Search Mode
Type test to search. The cursor will move to the first match. The screen state will revert to pre search state when there are no matches.\
Ctrl-n = Move to next match.\
Esc = Revert screen state to pre search.\
Enter = assume current screen state in search

## Save as Mode
This mode will be engaged if the current working file has no filename associated.\
Enter the filename when prompted.\
Enter - Save filename.

## Theme Mode
The first screen will be to set the text color.\
The second screen will be to set the background color.\
Move the cursor up or down, and select enter when the cursor is on the color you want for the respective settings.

