//! Machine protocol of the Bungard CCD control unit, as recovered from the original program.
//!
//! This crate has no GUI and no file-format dependencies, so a larger application can use it
//! directly. Layers: [`Transport`] (byte link) -> [`motion`] (planning in mm) -> [`safety::Machine`]
//! (refuses motion until the operator has verified the machine).
//!
//! The control unit is a dumb stepper driver: the PC sends one byte per motor event
//! over a 38400 8N1 UART and reads the machine's inputs from the modem status lines.
//!
//! EVERYTHING in [`bits`] IS A HYPOTHESIS taken from constants in the original code and
//! has NOT been verified against a real machine. Use [`Calibration`] mode first.

use std::io;

pub mod connection;
pub mod execute;
pub mod homing;
pub mod job;
pub mod latches;
pub mod motion;
#[cfg(feature = "node")]
pub mod node;
pub mod nullmodem;
pub mod positioner;
pub mod safety;
#[cfg(feature = "serial")]
pub mod serial;
pub mod tuning;

/// Serial parameters used by the original program (divisor 3 on a 115200 base).
pub const BAUD: u32 = 38_400;

/// Time one byte occupies on the wire at 8N1: 10 bits / 38400 baud.
pub const BYTE_TIME_US: f64 = 10.0 / BAUD as f64 * 1e6;

pub mod bits {
    //! Output bytes of the control unit.
    //!
    //! VERIFIED against the original program running in an emulator (every byte below was seen on
    //! the serial line): the step bytes, the idle and drivers values and the spindle sequence.
    //! Not verified on a real machine.

    /// Idle value of the first output latch (bit 7 set, bit 6 clear).
    pub const LATCH_A_IDLE: u8 = 0x80;
    /// Idle value of the second output latch (bits 7 and 6 set).
    pub const LATCH_B_IDLE: u8 = 0xC0;
    /// Handshake bytes sent while connecting.
    pub const HANDSHAKE: [u8; 2] = [0x00, 0x10];

    /// Step byte of an axis moving in a direction (step bit, direction bit and the `0x40` strobe).
    pub const X_NEG: u8 = 0x01;
    pub const X_POS: u8 = 0x43;
    pub const Y_NEG: u8 = 0x04;
    pub const Y_POS: u8 = 0x4C;
    pub const Z_NEG: u8 = 0x30;
    pub const Z_POS: u8 = 0x50;
    /// Direction bits of X and Y. The axis that does not step in a byte still carries its
    /// direction bit, set when its delta is positive or zero.
    pub const X_DIR: u8 = 0x02;
    pub const Y_DIR: u8 = 0x08;
}

/// State of the machine's inputs, read from the modem status register.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ModemStatus {
    pub cts: bool,
    pub dsr: bool,
    pub ri: bool,
    pub dcd: bool,
}

impl ModemStatus {
    pub fn from_msr(msr: u8) -> Self {
        Self {
            cts: msr & 0x10 != 0,
            dsr: msr & 0x20 != 0,
            ri: msr & 0x40 != 0,
            dcd: msr & 0x80 != 0,
        }
    }
}

/// Byte-level link to the control unit. Implemented by the simulator and, later, a real serial port.
pub trait Transport {
    fn set_dtr(&mut self, on: bool) -> io::Result<()>;
    fn write_byte(&mut self, byte: u8) -> io::Result<()>;
    fn status(&mut self) -> io::Result<ModemStatus>;
    fn sleep_us(&mut self, us: u64);
    /// Writes several bytes back to back. A transport with a real UART behind it should send them in
    /// one call so that the UART, not the PC clock, spaces them.
    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        for &b in bytes {
            self.write_byte(b)?;
        }
        Ok(())
    }
    /// Writes the bytes and, while they go out, watches the home line (RI, "all three axes on their switches"); when it rises no more
    /// of them are sent. Returns whether it rose. `chunk` is how many bytes are written between two looks at the watcher. The default
    /// writes everything and looks at the line afterwards; a transport that can read the line while it writes does better.
    fn write_bytes_until_home(&mut self, bytes: &[u8], chunk: usize) -> io::Result<bool> {
        let _ = chunk;
        self.write_bytes(bytes)?;
        Ok(self.status()?.ri)
    }
    /// Opens the link for writing. Only a node in front of the machine needs it (it refuses bytes until then);
    /// every other transport is always open.
    fn arm_link(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn disarm_link(&mut self) -> io::Result<()> {
        Ok(())
    }
    /// Stops at once what is queued far from here (the node's queue) and switches the drivers off there.
    fn abort_link(&mut self) -> io::Result<()> {
        Ok(())
    }
    /// Sets the spindle of the node (speed in rpm, 0 stops it; `reverse` for the other direction). Only a node has one.
    fn set_spindle(&mut self, rpm: u32, reverse: bool) -> io::Result<()> {
        let _ = (rpm, reverse);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "this connection has no spindle of its own",
        ))
    }
}

impl<T: Transport + ?Sized> Transport for Box<T> {
    fn set_dtr(&mut self, on: bool) -> io::Result<()> {
        (**self).set_dtr(on)
    }
    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        (**self).write_byte(byte)
    }
    fn status(&mut self) -> io::Result<ModemStatus> {
        (**self).status()
    }
    fn sleep_us(&mut self, us: u64) {
        (**self).sleep_us(us)
    }
    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        (**self).write_bytes(bytes)
    }
    fn write_bytes_until_home(&mut self, bytes: &[u8], chunk: usize) -> io::Result<bool> {
        (**self).write_bytes_until_home(bytes, chunk)
    }
    fn arm_link(&mut self) -> io::Result<()> {
        (**self).arm_link()
    }
    fn disarm_link(&mut self) -> io::Result<()> {
        (**self).disarm_link()
    }
    fn set_spindle(&mut self, rpm: u32, reverse: bool) -> io::Result<()> {
        (**self).set_spindle(rpm, reverse)
    }
    fn abort_link(&mut self) -> io::Result<()> {
        (**self).abort_link()
    }
}

/// How the steps of a move are timed on the way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pacing {
    /// One byte at a time, the PC clock making the pauses: what the original program does, fine on a real serial port.
    #[default]
    Bytes,
    /// The whole move as one stream whose byte positions carry the time (see [`paced_stream`]): for a USB adapter, whose
    /// bytes come out in bursts, and for the node.
    Stream,
    /// Like `Stream`, with fillers that keep the direction bits of the last step (see [`paced_stream_with`]).
    StreamKeepDir,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConnectError {
    /// DCD was already high with DTR low: nothing (or something else) is connected.
    DcdStuckHigh,
    /// DCD did not rise after the handshake bytes.
    NoAnswer,
    Io(String),
}

impl From<io::Error> for ConnectError {
    fn from(e: io::Error) -> Self {
        ConnectError::Io(e.to_string())
    }
}

/// Handshake of the original program: DTR low -> DCD must be low; DTR high, send 0x00 and 0x10 -> DCD must be high.
pub fn connect<T: Transport>(t: &mut T) -> Result<(), ConnectError> {
    t.set_dtr(false)?;
    t.sleep_us(200_000);
    if t.status()?.dcd {
        return Err(ConnectError::DcdStuckHigh);
    }
    t.set_dtr(true)?;
    t.sleep_us(100_000);
    t.write_byte(bits::HANDSHAKE[0])?;
    t.sleep_us(20_000);
    t.write_byte(bits::HANDSHAKE[1])?;
    if !t.status()?.dcd {
        return Err(ConnectError::NoAnswer);
    }
    t.set_dtr(false)?;
    // The original sends the two bytes once more with DTR low (its recorded traffic is `00 10 00 10`): the control unit is then out
    // of its error state, and this second pair seems to finish setting it up.
    t.sleep_us(20_000);
    t.write_byte(bits::HANDSHAKE[0])?;
    t.sleep_us(20_000);
    t.write_byte(bits::HANDSHAKE[1])?;
    t.sleep_us(20_000);
    Ok(())
}

/// One planned motor event: the byte to send and the pause that follows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    pub byte: u8,
    pub delay_us: f64,
}

/// Plans a straight XY move of `dx`,`dy` steps with Bresenham and a trapezoid speed profile.
///
/// `start_hz`/`max_hz` are step rates of the major axis and `ramp_steps` is the length of
/// the acceleration ramp. Returns the byte sequence with per-step delays.
pub fn plan_line(dx: i32, dy: i32, start_hz: f64, max_hz: f64, ramp_steps: u32) -> Vec<Step> {
    let (ax, ay) = (dx.unsigned_abs(), dy.unsigned_abs());
    let major = ax.max(ay);
    if major == 0 {
        return Vec::new();
    }
    // Byte of each axis when it steps, and the direction bit it contributes when it does not.
    let xb = if dx >= 0 { bits::X_POS } else { bits::X_NEG };
    let yb = if dy >= 0 { bits::Y_POS } else { bits::Y_NEG };
    let (x_dir, y_dir) = (
        if dx >= 0 { bits::X_DIR } else { 0 },
        if dy >= 0 { bits::Y_DIR } else { 0 },
    );
    // The major axis steps on every byte; the minor axis only OR-s its step byte on its own steps.
    // Ties go to X. Verified against the original (e.g. a pure +X move is 0x4B, a pure -X move 0x09).
    let (major_byte, minor_byte, minor) = if ax >= ay {
        (xb | y_dir, yb, ay)
    } else {
        (yb | x_dir, xb, ax)
    };

    let mut out = Vec::with_capacity(major as usize);
    let mut err = 0i64;
    for i in 0..major {
        // Distance to the nearest end decides the ramp position.
        let k = i.min(major - 1 - i).min(ramp_steps) as f64;
        let hz = if ramp_steps == 0 {
            max_hz
        } else {
            start_hz + (max_hz - start_hz) * k / ramp_steps as f64
        };
        err += minor as i64;
        let byte = if err * 2 >= major as i64 {
            err -= major as i64;
            major_byte | minor_byte
        } else {
            major_byte
        };
        out.push(Step {
            byte,
            delay_us: 1e6 / hz,
        });
    }
    out
}

/// Positions (in steps, counted from the start) at which the limit switch of each axis closes.
/// The three switches share the single RI line, which is high only while all three are closed, like on the real unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

/// Simulated control unit: records every byte and answers the handshake.
#[derive(Debug, Default)]
pub struct Simulator {
    /// Forces the home / limit line (RI) high.
    pub ri: bool,
    /// When set, RI is high while ALL three axes are at or below their limit positions (the real unit reports "all home").
    pub limits: Option<Limits>,
    pub sent: Vec<u8>,
    pub dtr: bool,
    pub elapsed_us: u64,
    answered_zero: bool,
    answered_ten: bool,
    pub x_steps: i64,
    pub y_steps: i64,
    pub z_steps: i64,
}

impl Transport for Simulator {
    fn set_dtr(&mut self, on: bool) -> io::Result<()> {
        self.dtr = on;
        Ok(())
    }
    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        self.sent.push(byte);
        match byte {
            0x00 => self.answered_zero = true,
            0x10 => self.answered_ten = self.answered_zero,
            b if b & 0x80 == 0 => {
                // Movement byte: hypothesis, see `bits`.
                if b & 0x01 != 0 {
                    self.x_steps += if b & 0x02 != 0 { 1 } else { -1 };
                }
                if b & 0x04 != 0 {
                    self.y_steps += if b & 0x08 != 0 { 1 } else { -1 };
                }
                if b & 0x10 != 0 {
                    self.z_steps += if b & 0x40 != 0 { 1 } else { -1 };
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn status(&mut self) -> io::Result<ModemStatus> {
        let at_limit = self
            .limits
            .is_some_and(|l| self.x_steps <= l.x && self.y_steps <= l.y && self.z_steps <= l.z);
        Ok(ModemStatus {
            dcd: self.dtr && self.answered_ten,
            ri: self.ri || at_limit,
            ..Default::default()
        })
    }
    fn sleep_us(&mut self, us: u64) {
        self.elapsed_us += us;
    }
}

/// Turns planned steps into a byte stream whose position carries the timing: every byte takes
/// [`BYTE_TIME_US`] on the line, so a step is placed at the byte slot nearest to its time and the slots
/// in between are filled with 0x00 (no axis bits, no motion). Sent in one piece, the UART spaces the
/// steps exactly, without depending on the PC clock or on how a USB adapter groups its bytes.
pub fn paced_stream(steps: &[Step]) -> Vec<u8> {
    paced_stream_with(steps, false)
}

/// The step bits of a movement byte (X, Y and Z); what is left of such a byte is its direction information.
const STEP_BITS: u8 = 0x01 | 0x04 | 0x10;

/// Like [`paced_stream`]. With `keep_direction` the filler between two steps is the last step byte with its step bits cleared
/// (for example `0x20` while Z goes up) instead of `0x00`, so the direction lines of the control unit stay where the step put
/// them, as they do in the original's runs of identical bytes.
pub fn paced_stream_with(steps: &[Step], keep_direction: bool) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut filler = 0x00u8;
    let mut t = 0.0f64;
    for s in steps {
        let slot = ((t / BYTE_TIME_US).round() as usize).max(out.len());
        out.resize(slot, filler);
        out.push(s.byte);
        if keep_direction {
            filler = s.byte & !STEP_BITS;
        }
        t += s.delay_us;
    }
    let end = (t / BYTE_TIME_US).round() as usize;
    if end > out.len() {
        out.resize(end, filler);
    }
    out
}

/// Sends planned steps as a [`paced_stream`] while the home line is watched; returns whether it rose (and the rest was not sent).
pub fn run_paced_until_home<T: Transport>(
    t: &mut T,
    steps: &[Step],
    keep_direction: bool,
    chunk: usize,
) -> io::Result<bool> {
    let stream = paced_stream_with(steps, keep_direction);
    t.write_bytes_until_home(&stream, chunk)
}

/// Sends planned steps as a [`paced_stream`], in chunks of about half a second, and waits until it has gone out.
pub fn run_paced<T: Transport>(t: &mut T, steps: &[Step]) -> io::Result<()> {
    run_paced_ahead(t, steps, 0.0)
}

/// Like [`run_paced`], but returns while a fraction `ahead` (0 to 1) of the stream is still going out, so the caller can read the
/// inputs and send the next piece without the line ever running dry: a stop-and-go move sounds and feels rough, and half as fast.
/// A piece sent this way is followed by another call, or by a plain [`run_paced`] that waits for the end.
pub fn run_paced_ahead<T: Transport>(t: &mut T, steps: &[Step], ahead: f64) -> io::Result<()> {
    run_paced_with(t, steps, ahead, false)
}

/// [`run_paced_ahead`] with the choice of the filler (see [`paced_stream_with`]).
pub fn run_paced_with<T: Transport>(
    t: &mut T,
    steps: &[Step],
    ahead: f64,
    keep_direction: bool,
) -> io::Result<()> {
    let stream = paced_stream_with(steps, keep_direction);
    // A serial adapter's write may not return until the bytes have gone out: what it took is part of the wait.
    let started = std::time::Instant::now();
    for chunk in stream.chunks(1900) {
        t.write_bytes(chunk)?;
    }
    let wait = stream.len() as f64 * BYTE_TIME_US * (1.0 - ahead.clamp(0.0, 0.95));
    let spent = started.elapsed().as_micros() as f64;
    t.sleep_us((wait - spent).max(0.0) as u64);
    Ok(())
}

/// Sends a planned move through the transport, honouring the delays.
pub fn run_steps<T: Transport>(t: &mut T, steps: &[Step]) -> io::Result<()> {
    for s in steps {
        t.write_byte(s.byte)?;
        t.sleep_us(s.delay_us as u64);
    }
    Ok(())
}
