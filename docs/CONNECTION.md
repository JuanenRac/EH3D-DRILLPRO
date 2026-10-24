# Connecting to the machine, and tuning it

DRILLPRO and ROUTEPRO read **one small file, the machine file**, to know how a job reaches the machine and how fast and how gently it may move. The two programs share it. Without it they run on the built-in simulator and move nothing.

## Machines

The programs are meant for the machines whose control unit sits on a serial port and was driven by the DOS software 4.6 of Bungard. Four machines were made in that period, two sizes in two versions:

| Model | Tool change | Table | Notes |
|---|---|---|---|
| CCD MTC | manual | 495 x 325 mm (555 mm of reach in offset mode) | the one it has been tried on |
| CCD/2 MTC | manual | 325 x 270 mm | smaller table; not tried |
| CCD ATC | automatic | as the CCD | not covered here |
| CCD/2 ATC | automatic | as the CCD/2 | not covered here |

The two **ATC** models (automatic tool change) are not covered by these two programs: they will get separate applications, built with what the reverse engineering of the original software allows. The **XL models** (and the machines made since) came with a modern control unit and modern software of their own: this project is **not** for them.

The size of the table and the other machine constants come from the machine's own configuration file (`DRILLPRO.DCF` and `ROUTEPRO.RCF`), as in the original software. **Only a CCD MTC has been tried**, and on it the spindle (the drill) and the vacuum cleaner output have not been tried yet.

## Where the file goes

| System | Path |
|---|---|
| Windows | `%APPDATA%\EH3D\machine.cfg` |
| Linux and macOS | `~/.config/eh3d/machine.cfg` |

Any other path can be given with `--connection FILE`. A missing default file means `demo`; a missing file that was asked for by name is an error. The file is plain text, one `key = value` per line; blank lines and lines starting with `#` or `;` are ignored. **An unknown key or a value out of range is an error** and is shown on the main screen: nothing is silently replaced by a default. Every key is optional. [machine.cfg.example](machine.cfg.example) lists all of them with their defaults and a comment each.

## How the job reaches the machine

`connection` takes one of five values:

| Value | Meaning |
|---|---|
| `demo` | no machine: the job runs on the built-in simulator (the default) |
| `rs232` | the computer's own serial port (`rs232_port`), one byte at a time with the computer's clock making the pauses, as the original program does |
| `usb` | a USB to RS-232 adapter (`usb_port`). The adapter delivers bytes in bursts, so a move is sent as one continuous stream whose byte positions carry the time and the adapter's UART spaces them exactly |
| `node` | the [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD) in the machine's box, over Ethernet or Wi-Fi (`node_address` and `node_token`: the token is the one set in the node's own panel, page Machine). The node also gives the spindle (`spindle = node`) and the machine's lamps |
| `server` | the [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER): the program does not drive the machine, it sends the job (`server_address` as `host:port`, `server_token` of this program and `server_machine`, the id of the machine on the server). The server plans it again with the constants and the tuning of that machine, runs it through the machine's node and tells the program how it goes. `Esc` stops the job on the server; closing the program does not |

With `server` the tuning values below belong to the machine on the server (they are set there, in the machine's `cfg`), not to this file. With any value but `demo` the main screen says, in red, which connection it is and that **Start moves the machine**. Keep the tool out, keep a hand on the machine's power switch and be ready to press `Esc`: it stops the job at once and switches the stepper drivers off.

## How the machine reports its limit switches

The control unit does not report each limit switch on a line of its own. It stops an axis at its switch by itself, and puts one signal on the **RI** line: **it is 1 only while X, Y and Z are all on their switches at the same time** (the three HOME lamps of the control unit), with DTR low, which is the state the machine is in while it works. The reference run is built around that:

1. **Z** goes up to its switch.
2. **X**, then **Y**, go to theirs. Only the last axis to arrive can be seen on RI: as soon as it rises the program stops sending the search and goes on.
3. A smooth **calibration jump** on X and on Y (back off a little and approach again, to remove the switch's play) ends the run, and RI is checked again.

If the home line is already up at the start the machine is parked: Z, X and Y are on their switches. The program then skips the long searches, moves X and Y away from their switches by `unstick_mm`, checks that RI drops, and searches for them again over that short, known distance, so it sees both arrive live. With `home_signal = off` none of this is used: every search is sent in full and nothing is checked.

## The values

Speeds are in steps per second: the control unit takes one byte per step, and the most the line allows at 38400 baud is 3840. Every value is tuned for a real machine, and can be changed one by one.

| Key | Default | What it does |
|---|---|---|
| `home_signal` | on | use the home line (RI) in the reference run; off: searches are sent in full |
| `xy_hz_limit` | none | the most steps per second the X and Y motors keep up with: no speed of the job goes above it |
| `z_hz_limit` | none | the same for Z |
| `x_search_hz` | 2200 | fastest rate of the X search |
| `y_search_hz` | 2200 | fastest rate of the Y search |
| `search_ramp_steps` | 800 | length of the ramp of those searches: a longer ramp is gentler and calmer to listen to |
| `search_margin_percent` | 12 | how much longer than the travel of each axis a search is |
| `z_search_hz`, `z_start_hz`, `z_ramp_steps` | 900, 450, 200 | fastest rate, starting rate and ramp of the Z search |
| `z_search_steps` | 2150 | steps of the Z search: more than the whole travel of Z (about 1900 steps for 54 mm) |
| `jump_peak_hz` | 1400 | rate the calibration jump rises to |
| `jump_start_fraction` | 0.5 | where the jump starts, as a fraction of the reference speed |
| `jump_ramp_parts` | 2 | 2: half accelerating and half slowing down, the smoothest; 3 and more: a stretch at constant rate in between |
| `jump_backoff_steps`, `jump_approach_steps` | 200, 250 | steps of each leg of the jump on X and Y |
| `z_backoff_steps`, `z_approach_steps` | 45, 54 | the same for Z |
| `unstick_mm` | 10 | when parked, how far X and Y leave their switches before the short search; 0 turns it off |
| `unstick_always` | off | do it even when the home line is down. The positions are then unknown: leave it off unless you know where the axes are |
| `search_hint_margin_steps` | 300 | steps added to the known distance of the short search (if RI still does not rise, the whole search is repeated) |
| `travel_ramp_steps` | 400 | ramp of the moves between holes and paths |
| `z_pacing` | bytes | `bytes`: moves of Z alone go one byte at a time even on a `usb` or `node` connection; `stream`: as a stream too |
| `stream_filler` | dir | the filler between two steps of a stream keeps the direction bits (`dir`) or is zero (`zero`) |
| `cancel_piece_steps` | 1500 | steps sent at a time before the cancel key is looked at again |
| `home_check_piece_steps` | 100 | steps between two readings of the home line on `rs232` |
| `home_watch_chunk_bytes` | 0 | on `usb` and `node`: 0 sends the last search as one unbroken stream and reads the home line once, at the end (every reading in the middle is a gap in the stream that made the Y motor lose steps on the CH340); a value of 50 to 4000 reads the line every that many bytes instead |
| `home_watch_seek` | off | also watch the home line during the first long searches (it costs small gaps in the stream) |
| `spindle_start_wait_ms`, `spindle_stop_wait_ms` | 1680, 1000 | how long the job waits for the spindle to reach speed and to stop |
| `spindle` | central | `central`: the spindle of the control unit (its latch bits, up to 60000 rpm); `node`: the spindle of the EH3D-NODE-BUNGARD (a variable-frequency drive on the node's own outputs, any speed up to the node's maximum). Only a `node` connection (or the server in front of one) has it |

## Trying a machine step by step

The `protocol/examples/` folder has small programs for trying a real machine one thing at a time, each with a hand on the power switch: reading the input lines, switching the drivers on and off, moving one axis a few millimetres, and the whole reference run with a log of every phase (`home_all_probe`). They take the port as an argument and need `--yes` before they move anything. `release_drivers` switches the stepper drivers off so the axes can be moved by hand.

## USB adapters: what was tried, and other chips

Everything in this project was tried with **one adapter: a cable with a CH340 chip** (WCH driver 3.5.2019, port COM8), on a CCD MTC.

| What was seen with the CH340 | What the software does about it |
|---|---|
| A newer WCH driver (4.0.2026.2) would not open the port even after replugging and rebooting; the 2019 driver works | nothing in software: if the port does not open, try the older driver |
| It delivers the bytes in bursts, one per 1 ms USB frame, so a plain "byte, wait, byte" loop gives uneven steps: rough noise and lost steps above about a thousand steps a second | `usb` sends a move as **one continuous stream** in which a pause is a byte with no axis bits, so the adapter's own UART keeps the time |
| A stream that runs dry (pieces with a pause between them, or status reads in between) stalls the motors | pieces are long (`cancel_piece_steps`), and the home line is read only once the last search has gone out (`home_watch_chunk_bytes = 0`); reading it in the middle of the stream made the Y motor lose steps |
| Reading the home line (RI) costs about 5 ms and only gets through about every 0.22 s while bytes are being written | only the last axis search is watched |
| Z with filler bytes between steps was weak and jerky | Z-only moves go one byte at a time (`z_pacing`) |

**Another cable with another chip (FTDI, CP210x, PL2303 ...) should work, but not necessarily the same way.** The protocol and the continuous-stream method do not depend on the chip, but the adapters differ in what the method rests on: how the driver buffers and flushes the bytes (the FTDI latency timer, for instance), how long a status read takes, how steadily the UART is fed and whether it keeps exactly 38400 baud. So the speeds and chunk sizes tuned on the CH340 (`x_search_hz`, `y_search_hz`, `home_watch_chunk_bytes`, `z_pacing` ...) may need adjusting; none of this has been measured on another chip. Try a new adapter as the CH340 was tried: with the probes in `protocol/examples/`, one axis at a time, from low speeds up, with a hand on the power switch, and put what works in the machine file. Reports with the chip, the driver and the values that worked are welcome.
