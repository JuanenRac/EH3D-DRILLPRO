# The classic screens

DRILLPRO and ROUTEPRO start with the screens of the MS-DOS programs they are open implementations of. The same
front end (`eh3d_drillpro::classic`) serves both programs: `ClassicApp<E>` draws and handles the keys, and an
engine (`ClassicEngine`) supplies the file, the tool table, the settings and the job. `--modern` starts the
menu-driven interface instead.

## What is reproduced

| Screen | Drilling | Routing |
|---|---|---|
| Main form: scale, offset, clip, vectors, board, base, counters, seven buttons | yes | yes |
| Parameter (configuration) | with the sorting | with the chord angle |
| Tool Data | stroke, feed, spindle | depth, iterations, X and Z feed, spindle |
| Select File | yes | yes |
| Panelization | yes | yes |
| Quit, homing window | yes | yes |
| Job window | yes | not yet |

The layout, characters and colours of each screen follow the original program cell by cell (80 columns, 24 visible
rows). Texts are ours, in eight languages; the heading lines and the machine number are our own wording.

## How it is checked

* `tests/classic.rs`: the hot letters of every screen are unique in every language, the labels fit the room the layout
  gives them, and the keyboard flow (load a file, edit a field with its range check, quit) works.
* `tests/classic_golden.rs`: every screen against a reference screen captured from the original. The reference
  screens are not in the repository: set `EH3D_ORIGINAL_UI` to a folder that holds them (`drill_main.txt`, ...: a
  `# chars`, `# fg` and `# bg` block of 24 rows of 80 cells). Without it these tests pass without comparing.

## Keys and mouse

* Letters (the highlighted one of each label) press a button or focus a field; `Alt` plus the letter works while a field is
  being typed in. `Tab` and `Shift+Tab` walk the fields and buttons, `Enter` accepts a field and presses a button, `Esc`
  cancels a typed value, closes a dialog, or asks to quit from the main screen.
* In a field: typing (the first key replaces the whole value), `Backspace`, `Delete`, `Home`, `End`, `Left`, `Right`.
  Values outside their range are refused with the limits shown.
* Radio buttons and the language: `Up`, `Down`, `Left`, `Right` or `Space` change the selection.
* Select File: type a name or a pattern (`*.NCD`) and press `Enter`; `Up`, `Down`, `Left`, `Right`, `PageUp`, `PageDown`
  move in the list; `Tab` switches between the name and the list.
* Mouse: every field, button, radio button and list entry; a double click on a file opens it.

## The window

The window can be maximized or resized. The 80x25 screen is drawn as large as the window allows (from 640x400 up to
1080p and beyond) with its proportions, centred; clicks are mapped through the scaling.

## The console version

`drillpro-console` draws the same screens in a terminal (80x25 at least; a larger terminal shows them in the middle), with the keyboard
and the mouse, and needs no window system. `Ctrl+C` leaves at once. It reads the same options as the window.

## The files

| Dialog | Reads | Writes |
|---|---|---|
| Select File | the `.NCD` (drilling) or `.PLT` (HP-GL) files of the folder the configuration names | |
| Parameter | the configuration file (`DRILLPRO.DCF` / `ROUTEPRO.RCF`) | Save: the same file, the old one kept as `.BAK`; every comment and reserved line is kept |
| Tool Data | the tool table (`DRILLPRO.DTD` / `ROUTEPRO.RTD`) | Save: the same file, with a `.BAK` copy |
| Leaving | | offers to save the form values (scale, offset, clip, board, base) into the configuration |

Without `--data` the files are looked for in the current folder, the folder of the program and the user's
configuration folder (`%APPDATA%\EH3D`, `~/.config/eh3d`).

## Folders and the job

* The Select File list is the files, then the folders, then `..\` last, as in the original. It works from the absolute
  folder, shows the drives as `[-C-]` entries on Windows, hides dotted files, and ends a name longer than 14 cells with `~`.
* A path written in the name field goes to its folder: `D:\jobs\*.NCD`, `..\other`, `C:\`. What follows the last
  separator is the pattern, or the file to open. The mouse wheel scrolls the list.
* The form drives the job: `from` and `to` choose the vectors in the order of the file, the clip leaves out what ends
  outside the working area (the offset counts), and the Panelization dialog's Ok makes the copies (a tool at a time).
  The copies are `board - spacing` apart: that is what the original's count of copies works out to on both axes of the one
  figure that was checked.
* The job window shows the planned position, tool, stroke, speeds, count and drilled depth.
* The job asks the operator, as the original does: Attention! before the spindle starts, and Tool Change (the tool to put in,
  the length of the run, the spindle, the Z speed and the stroke) for every tool. `Enter` (or the letter of Ok) goes on; `Esc`
  (or the letter of Abort) stops the job. The words of the Tool Change window are the original's; its layout is ours, as it
  could not be captured from the original.
* `Esc` while the job runs opens INTERRUPTED / What next?: Cont. (`Enter`, `C`) goes on, Abort (`Esc`, `A`) stops the job. The job
  waits while the window is open. During the homing `Esc` stops at once.
* **AutoChg** (Parameter window, only on the models with a tool changer): the box (`U`, or a click) says whether the tools are changed by
  the machine. The language field of ours moves one row down there.
* **Offset** (the button): "Delete old offset?" (Yes: start at home; No: start at the offset of the form), then the Offset Teach
  window. Move the head with `E` `S` `D` `X` (Y+, X-, X+, Y-) or the arrow keys, `R` `C` or `PageUp` `PageDown` for Z; `1`-`4` pick the
  step (1, 10, 100, 1000 mil); every button answers to the mouse. Ok copies the position into the Offset of the main screen.
  Teach starts the Teach-In mode (the position becomes the origin): `+` records, `-` takes the last back, `M` puts the last one
  where the head is; Ok asks "Write Teach-In file?" and writes `TEACH.NCD` in the folder of the files.

## The machines

Bungard CCD and CCD/2 with manual tool change (MTC): supported (tried on a CCD MTC only). CCD and CCD/2 with automatic
tool changer (ATC): not covered yet. The About button says so, in every language, and that the project is independent of
Bungard Elektronik.

## Not done yet, or inferred

* The limits of the fields come from the comments of the configuration file (for example scale 0 to 10) and from what the
  machine accepts; the original's own limits have not been read from the program.
* The count of copies of the panelization is inferred from one figure of the original. The Offset button only reports that it
  is not available.
* The job window shows planned values, not the machine's own readings.
