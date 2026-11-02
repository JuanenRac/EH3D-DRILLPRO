# Hardware checklist - DRILLPRO on the machine

Run the automatic part first, then the steps by hand. Write the result next to each step and keep the reports.

## Automatic part

```
python tools/machine_selftest.py --dcf DRILLPRO.DCF --machine-cfg machine.cfg --spindle original
python tools/machine_selftest.py --measure
```

The first command only reads: it checks the machine file (speeds against the serial line, acceleration against the ramp model, spindle start time, tool speeds, depth, calibration factors). The second walks you through timing a move and the spindle start with a stopwatch, computes the values and writes a proposed file `DRILLPRO_proposal_<time>.DCF`. It never changes your DCF and never sends anything to the machine; copy the proposal over by hand after you tried it. Use `--spindle vfd` for the variable-frequency drive of the node.

## By hand

| Step | What to do | What must happen | Result |
|---|---|---|---|
| 1 | Reference run | All three axes find their switches; the program says homed | |
| 2 | Time a long X move at the fastest speed (`--measure`) | The tool gives an ACCELERATION; no step is lost (return to the same spot) | |
| 3 | Same for Z with the spindle off | As above, and the head does not hit the table | |
| 4 | Start the spindle at 12000 rpm and time it (`--measure`) | The tool gives a SPINDLE START TIME; the job waits at least that long | |
| 5 | Drill a test board | Holes land where the file says; depth goes through the board only | |
| 6 | Move or panelize the job, then run it | The board stays centred inside the clipping | |
| 7 | Press Stop during a job | The machine stops and the spindle goes off | |
| 8 | Tool changer (only if the machine has one) | Rack positions and changes work; skip on a manual machine | |
