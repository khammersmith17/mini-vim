## Mini-Vim
This is a terminal based text editor. Currently it supports any text editing (utf-8 characters work better at the moment than other characters), setting the theme, searching, resizing the terminal, and saving, among other things.

I am currently working on adding syntax highlighting and search highlighting. Eventually, I would like to also build an installer for it. The terminal interaction is using the crossterm crate.

Until the installer is ready, you can fork the repo and try yourself.
To run a fresh working file simply execute cargo run
To edit an existing file: cargo run {filename}

## Modes
There are essentially 7 modes: normal, search, save as, vim motions, highlight text, jump to line, and set theme.\
If there are changes to the file state when trying to exit, a message will appear asking if you want to exit without saving (Ctrl-y = Exit without saving, Ctrl-n = Save file before exit).

## Normal Mode Commands
Like other terminal based text editors, there are a number of commands.\
Ctrl-q = Quit\
Ctrl-l = Snap cursor to end of line\
Ctrl-u = Snap cursor to first line\
Ctrl-d = Snap cursor to last line\
Ctrl-r = Snap cursor to first line\
Ctrl-w = Save\
Ctrl-c = Help\
Ctrl-f = Search\
Ctrl-t = Theme\
Ctrl-v = paste text\
Ctrl-j = Jump Cursor Mode\
Ctrl-n = Vim mode

## Search Mode
Type text to search. The cursor will move to the first match. All search hits will be highlighted. The screen state will revert to pre search state when there are no matches.\
Ctrl-n = Move to next match.\
Ctrl-p = Move to previous match\
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

## Highlight Mode
Move the cursor to highlight text with the arrows.\
Ctrl-c = copy text\
Backspace = delete text\
Esc = revert to pre highlight screen state

## Vim Mode
Right now the only support keys are h, j, k, l, 0, $. gg and GG are being added. Vim motions will also be available in highlight mode as well soon.\
h = left\
j = down\
k = up\
l = right\
0 = snap left\
$ = snap right\
Esc = exit vim mode\
gg = page up\
GG = page down\
:w = write\
:wq = write and quit\
:q = quit\
:q! = quit without saving

## Jump Cursor Mode
Type new line location when prompted. Press enter to jump to line
