# The machine protocol: what is confirmed and what is still a hypothesis

This page says how the control unit of the Bungard CCD machines is driven by the PC. The same protocol is used by EH3D-DRILLPRO (drilling) and EH3D-ROUTEPRO (routing); the code is in the `protocol/` crate of EH3D-DRILLPRO. For the machine file and every tuning value see [CONNECTION.md](CONNECTION.md).

**Status: partly confirmed on one machine.** It was first worked out by studying how the original program talks to its control unit, and then tried step by step on a CCD MTC with its original control unit, over a USB serial adapter. What was checked is marked **confirmed** below; everything else is still a hypothesis. Never connect a machine without the tool out of the spindle and a hand on the power switch. A whole drilling or routing job has **not** run on the machine yet.

The control unit is the one of the serial-port machines driven by the DOS software 4.6 (CCD MTC and CCD/2 MTC, and the two ATC versions); only a CCD MTC was available. The two ATC models will get separate applications; the XL models and the machines made since have a modern control unit and software and are not covered.

## The link

- A plain serial port: 38400 baud, 8 data bits, no parity, 1 stop bit, no flow control. One byte takes about 260 microseconds on the wire, so the line carries at most 3840 steps a second. **Confirmed.**
- The control unit is a dumb stepper driver. There are no text commands: the PC sends **one byte for every motor event** and decides the time between bytes. Nothing comes back on the data line. **Confirmed.**
- The original gets the timing of the steps from a hardware timer. A modern computer cannot, and a USB serial adapter makes it worse: it delivers the bytes in bursts, one USB frame at a time, and the motors then sound rough and lose steps above about a thousand steps a second. What works: one continuous stream in which a byte with no axis bits is a pause, so every byte occupies one slot of 260 microseconds and the UART of the adapter keeps the time. **Confirmed on X and Y** at 2200 steps a second and more. Moves of **Z alone** are the exception: they are sent one byte at a time, because with filler bytes between the steps the Z motor was weak and jerky. **Confirmed.**
- Cutting a fast move into pieces with a pause between them makes the motors stall: the stream must not run dry. **Confirmed.**

## Handshake

1. DTR low; after about 200 ms the DCD line must read low.
2. DTR high; after about 100 ms send `0x00`, wait about 20 ms, send `0x10`; DCD must now read high.
3. DTR and RTS are then left low, and the two bytes `0x00` and `0x10` are sent **once more**, with DTR low. **Confirmed**: the original sends `00 10 00 10`.

Closing the link sends `0xC0` (stepper drivers off) after the last move.

## Inputs

The control unit answers on the modem status lines, not as data. How its RS-232 connector is wired explains what they carry: DTR goes to the interface chip's error and tool inputs through two diodes, and DCD and RI are driven back by the control unit.

- **RI: 1 only while X, Y and Z are all on their limit switches at the same time** (the three HOME lamps of the control unit), measured with DTR low, which is the state the machine is in while it works. One switch pressed alone changes nothing on any line. **Confirmed.**
- **DCD:** the error state of the control unit.
- With DTR high both lines read high whatever the switches do: they follow DTR.
- A limit switch is not reported on its own line. The control unit stops an axis at its switch by itself and keeps accepting the bytes: the PC cannot tell how far the axis got, only that all three have arrived. **Confirmed.**

## The reference run

What the original does, and what this project does on the real machine:

1. **Z** goes up (`0x30`) until its switch; the control unit stops it there.
2. **X**, then **Y**, go towards their switches (negative steps, `0x01` and `0x04`). Only the last axis to arrive is seen on RI; the search stops being sent as soon as it rises.
3. A **calibration jump** on X and on Y: back off a little and approach the switch again at a low speed, to remove the play of the switch. RI must fall when an axis leaves its switch and rise when it comes back.

The recorded traffic of the original run in an emulator shows a diagonal search (`0x05` repeated) because the emulated machine never raised RI; on the real machine the original homes axis by axis, and a diagonal at speed makes X stall.

When RI is already up at the start the machine is parked and the searches can be skipped; moving X and Y away by a few millimetres first lets the program see both arrive live and makes the search a short, known distance.

## What the node adds

With a `node` connection ([EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD)) the bytes go to the node over a TCP link and the node's UART spaces them, so the timing no longer depends on the computer or a USB adapter. The node also gives the program three things the control unit's RS-232 port does not:

- **The machine's lamps** (HOME X, Y and Z, ENABLE, TOOL), read from the control unit's own LEDs. They tell which axis is on its switch, live, so the program does not have to read the home line (RI) in the middle of the stream.
- **`StopOnHome`**: a request that makes the node drop the rest of a search the moment an axis is home. The control unit has already stopped the axis by itself; this only saves the wait.
- **The spindle of the node**: with `spindle = node` in the machine file the job uses `Item::Spindle { rpm, wait_ms }` instead of the control unit's spindle latch bits; the node turns the speed into a 0-10 V signal and the run/direction lines of a variable-frequency drive. The spindle is stopped on any stop of the job.

The node's own rules (token, arming, a stop when the link goes quiet) are in its [PROTOCOL.md](https://github.com/JuanenRac/EH3D-NODE-BUNGARD/blob/main/docs/PROTOCOL.md). None of this has run on a board yet.

## Output bytes

Bytes with bit 7 set write one of two latches (the base values are `0x80` and `0xC0`); bytes with bit 7 clear are motor steps.

| Byte | Meaning |
|---|---|
| `0x01` / `0x43` | X axis negative (towards its switch) / positive step. **Confirmed.** |
| `0x04` / `0x4C` | Y axis negative (towards its switch) / positive step. **Confirmed.** |
| `0x30` / `0x50` | Z axis up (towards its switch) / down. **Confirmed.** |
| `OR` of two | a diagonal step: the bytes of both axes combined (it stalls X at speed, see above) |
| latch `0xC0` bit 4 (`0x10`) | motor drivers on (needed before any move: without it nothing moves), off with `0xC0`. **Confirmed.** |
| latch `0x80` bit 4 (`0x10`) | spindle running (not tried on the machine yet) |
| latch `0x80` bit 5 (`0x20`) | an auxiliary output, with a 500 ms wait (not tried). The original on the manual machine never switches it; on the automatic-tool-change models it opens (`0xA0`) and closes (`0x80`) the chuck. With `vacuum = aux` in the machine file this program switches it on with the spindle and off with it |
| latch `0x80` low bits | spindle speed level (not tried) |

The spindle start sequence is: write the second latch with its spindle bits set, wait about 100 ms, write the first latch with bit 4, wait for the spindle to come up, then write the speed level. Stopping reverses it.

## Motion

- The PC plans the moves in steps: a line is drawn with a Bresenham-style stepper and an acceleration ramp. A bare ramp of 40 steps is far too brusque for the motors at speed: a ramp of several hundred steps is needed.
- On the tried machine the X and Y motors keep up with about 2200 to 2400 steps a second (X stalled at 2700, Y stalled at 2700) and Z, one byte at a time, lifts with force up to about 1300. The reference speeds of the original's configuration (4000 mm/min, which is about 2600 steps a second) are therefore too high for this machine: they are tuning values of the machine file.
- Steps per millimetre: the original uses one step per mil (0.0254 mm), multiplied by correction coefficients of the machine configuration. A move of 400 steps on X and Y covered the expected 10 mm to the eye; Z has not been measured.
- The work area comes from the machine configuration: 495 x 325 mm or 555 x 405 mm on the tried machine, 325 x 270 mm on the CCD/2.

## The tool changer (ATC models)

On the models with the automatic tool changer (`Tool change 1=Auto` in the configuration file, with the rack positions after it) the
auxiliary output, latch `0x80` bit 5, opens the chuck: `0xA0` opens it, `0x80` closes it, with 500 ms after each change. The rack has
16 places: 0 is the test pin, 1 to 14 hold the tools and 15 the drill break detector; each has its X, Y and Z in mm. A tool change is
(recorded on the original, event by event):

1. go to the place of the tool in the chuck (place 0 at the start), Z down to the place's Z;
2. `0xA0` (open);
3. slide to the place of the next tool;
4. `0x80` (close) and Z up.

The job begins with that change towards place 1, drills, and for each following tool changes again; after the last tool it changes
towards place 0. Between the steps the bytes are those of the manual models (drivers on, spindle `0xF8` `0x90` `0x9F`, stop `0xD2` `0x80`
`0xD0`). This has been compared with a recording only; it has not run on a real ATC machine.

## Safety in this project

The `protocol` crate has a safety gate: a `Machine` refuses to move anything until it has been armed. A real connection in the machine file is the operator's statement that the machine is ready. `Esc` cancels a job within a fraction of a second and switches the stepper drivers off, also after an error or when the link drops. The limit switches are watched by the control unit itself; this software has no emergency stop of its own.

## What is still open

- A whole drilling job and a whole routing job on the machine.
- The spindle (drill) bits and the vacuum cleaner output (the auxiliary output), on the MTC as on the other models; the node's spindle with the real variable-frequency drive; the calibration of Z.
- The CCD/2 MTC, not tried. The two ATC models get separate applications.
- Measured speeds above what the machine file limits to.

If you have a serial capture of the original software talking to a control unit, please share it.
