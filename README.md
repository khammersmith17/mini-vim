## Mini-Vim
This is a terminal based text editor. Currently it supports any text editing (utf-8 characters work better at the moment than other characters), setting the theme, searching, resizing the terminal, and saving, among other things, such as copy and paste. There is also support for the most common vim key bindings.

The goal for this editor is to be more like Vim than nano, but more lightweight than vim itself. Vim has a relatively steep learning curve, this can serve as an entry to get comfortable with editing files in the terminal. I also hope you find it responsive. This is written in pure Rust and uses the crossterm crate for all event reading, as well as the famous rust-clipboard for reading and writing to the OS clipboard. At some point, it might be fun to hand roll this.

I developed this on MacOS. If the installer does not run on your target platform, or some features do not work, please leave an issue.

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
Alt-g = Snap cursor to first line\
Ctrl-g = Snap cursor to last line\
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
Use the arrow keys to move or use vim single cursor movements./
Ctrl-c = copy text\
Backspace = delete text\
Esc = revert to pre highlight screen state

## Vim Mode
Not all vim commands are support as of yet. Currently supported:/
h = left\
j = down\
k = up\
l = right\
o = new-line\
0 = snap left\
$ = snap right\
d = delete\
y = yank\
/ = search mode\
Esc | i = exit vim mode\
gg = page up\
GG = page down\
:w = write\
:wq = write and quit\
:q = quit\
:q! = quit without saving
:{line number} = jump to line

## Jump Cursor Mode
Type new line location when prompted. Press enter to jump to line
