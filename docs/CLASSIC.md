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

## Keys

Letters (the highlighted one of each label) press a button or focus a field; `Enter` presses the highlighted
button or accepts a field; `Esc` leaves a field, or asks to quit; `Tab`, up and down move between fields.

## Not done yet, or inferred

* The inputs of the fields are checked against ranges taken from the comments of the configuration file (for example
  scale 0 to 10); the original's own limits have not been read from the program.
* The copies of the panelization are counted (the formula is inferred from one figure of the original) but the job does
  not repeat the board yet. The Offset button only reports that it is not available.
* The job window shows the counters and the hole count; the live position, tool and speed are not tracked yet.
* The Parameter dialog shows the values; editing them in the dialog and saving the file is not done yet.
