# eh3d-drillpro-protocol

Serial protocol library for the Bungard CCD control unit. No GUI and no file formats: it is meant
to be embedded in other applications (a combined drill-and-route program, a 2D/3D viewer, a CLI).

```
Transport   byte link: DTR, write byte, modem status lines   (Simulator, SerialTransport)
   |
motion      planning in millimetres: rapid, feed, Z, drill stroke, Bresenham + ramp
   |
positioner  logical position and backlash compensation (as in the original)
   |
safety      Machine: connect, read inputs, refuses motion until armed with Verified::all()
```

## What the machine speaks

- 38400 baud, 8N1, no flow control; one byte is 260 us on the wire.
- The control unit is a dumb stepper driver: the host sends **one byte per motor event** and
  decides the time between bytes. There are no text commands.
- Handshake: DTR low, DCD must read low; DTR high, send `0x00` then `0x10`, DCD must read high.
- Machine inputs (limit switches, sensors) arrive on the modem status lines, not as data.

## Status of the knowledge

Verified by running the ORIGINAL program in an emulator and recording its serial traffic (the
`golden` tests encode those recordings): the handshake, the step bytes and their combination
rules, the drivers latch, the spindle start/stop sequence and its timing, and the geometry
(one step per mil times the correction coefficients of the configuration file).

NOT verified on a real machine: that the bytes make the real hardware move as expected, the
homing behaviour with real limit switches, the aux output (vacuum), and the speed values.
For that reason `Machine::run` fails with `MachineError::NotArmed` unless the caller passes
`Verified { bit_map: true, geometry: true }` to `arm`, which should only happen after
step-by-step validation on the real machine.

Inputs: the single line RI is the home / limit switch signal (all axis switches share it) and DCD
is the machine fault line. Aux output: bit 5 of latch A (`latches::A_AUX`, 500 ms settling).

## Features

- `serial` - `SerialTransport` over the `serialport` crate (compiles; not tested on hardware).

## Example

```rust
use eh3d_drillpro_protocol::{motion::*, safety::*, Simulator};

let mut m = Machine::new(Simulator::default());
m.connect()?;
let plan = rapid_xy((0.0, 0.0), (10.0, 5.0), &Geometry::default());
assert!(m.run(&plan).is_err());          // not armed: refused
m.arm(Verified::all())?;                 // only after verifying the machine
m.run(&plan)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```
