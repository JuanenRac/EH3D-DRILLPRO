//! Safety gate between application code and the machine.
//!
//! The protocol was recovered from the original software and has not been confirmed on
//! hardware. [`Machine`] therefore refuses to send any motion byte until the caller states,
//! explicitly, that the bit map and the geometry have been verified.

use std::io;

use crate::latches::{run_ops, Latches, Op, Sequence};
use crate::{
    connect, run_paced_until_home, run_paced_with, run_steps, ConnectError, ModemStatus, Pacing,
    Step, Transport,
};

/// What the operator has verified on the real machine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Verified {
    /// The meaning of the output bits (axis, direction, spindle) was confirmed.
    pub bit_map: bool,
    /// Steps per mm and speed limits were measured.
    pub geometry: bool,
}

impl Verified {
    pub fn all() -> Self {
        Self {
            bit_map: true,
            geometry: true,
        }
    }
    pub fn is_complete(self) -> bool {
        self.bit_map && self.geometry
    }
}

#[derive(Debug)]
pub enum MachineError {
    NotConnected,
    /// Motion was requested before [`Machine::arm`] was called with a complete [`Verified`].
    NotArmed,
    Connect(ConnectError),
    Io(io::Error),
    /// A reference search ended without the home line (RI: all three axes on their switches) rising.
    HomeNotFound,
}

impl From<io::Error> for MachineError {
    fn from(e: io::Error) -> Self {
        MachineError::Io(e)
    }
}

impl std::fmt::Display for MachineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MachineError::NotConnected => write!(f, "not connected to the control unit"),
            MachineError::NotArmed => {
                write!(f, "motion refused: bit map and geometry are not verified")
            }
            MachineError::Connect(e) => write!(f, "connection failed: {e:?}"),
            MachineError::Io(e) => write!(f, "I/O error: {e}"),
            MachineError::HomeNotFound => {
                write!(f, "the reference search ended without the home signal")
            }
        }
    }
}

impl std::error::Error for MachineError {}

/// The machine inputs in the terms the original program uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Inputs {
    /// Home / limit switch (RI). The switches of all axes share this one line.
    pub limit: bool,
    /// Machine fault (DCD), for example spindle overload.
    pub fault: bool,
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunOutcome {
    /// Number of step bytes actually sent.
    pub sent: usize,
    pub limit_hit: bool,
    pub fault: bool,
}

pub struct Machine<T: Transport> {
    transport: T,
    connected: bool,
    armed: bool,
    latches: Latches,
    pacing: Pacing,
    home_signal: bool,
    tuning: crate::tuning::Tuning,
    search_hint: Option<(u32, u32)>,
}

impl<T: Transport> Machine<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            connected: false,
            armed: false,
            latches: Latches::default(),
            pacing: Pacing::default(),
            home_signal: false,
            tuning: crate::tuning::Tuning::default(),
            search_hint: None,
        }
    }

    /// How [`Machine::run`] times the steps (the default is one byte at a time, as the original does).
    pub fn set_pacing(&mut self, pacing: Pacing) {
        self.pacing = pacing;
    }

    /// The pacing to use for these steps. A move of Z alone always goes one byte at a time with the PC clock, whatever the
    /// connection: on the real machine the Z motor (which carries the spindle) lifts with full strength that way, and a paced
    /// stream with filler bytes between the steps leaves it weak and jerky (the filler seems to cut its holding current, which X and
    /// Y do not mind).
    fn pacing_for(&self, steps: &[Step]) -> Pacing {
        let z_only = !steps.is_empty()
            && steps
                .iter()
                .all(|s| s.byte & 0x10 != 0 && s.byte & 0x05 == 0);
        if z_only && self.tuning.z_pacing_bytes {
            Pacing::Bytes
        } else {
            self.pacing
        }
    }

    /// Says whether the control unit reports "all three axes on their switches" on the RI line (the real machine does,
    /// measured with DTR low). With it, [`crate::execute::run_items`] stops a reference search the moment RI rises and
    /// fails if it never does; without it the search is sent in full.
    pub fn set_home_signal(&mut self, on: bool) {
        self.home_signal = on;
    }

    /// The tuning values the machine's own rules use (the Z pacing, the sizes of the pieces of a move).
    pub fn set_tuning(&mut self, tuning: crate::tuning::Tuning) {
        self.tuning = tuning;
    }

    /// How far X and Y are from their switches (in steps), when that is known (after an unstick): the searches are then cut to it.
    pub fn set_search_hint(&mut self, hint: Option<(u32, u32)>) {
        self.search_hint = hint;
    }

    pub fn search_hint(&self) -> Option<(u32, u32)> {
        self.search_hint
    }

    pub fn tuning(&self) -> &crate::tuning::Tuning {
        &self.tuning
    }

    pub fn pacing(&self) -> Pacing {
        self.pacing
    }

    pub fn home_signal(&self) -> bool {
        self.home_signal
    }

    /// Runs the connection handshake (no motion). The link is opened for writing first (a node asks for it).
    pub fn connect(&mut self) -> Result<(), MachineError> {
        self.transport
            .arm_link()
            .map_err(|e| MachineError::Connect(ConnectError::Io(e.to_string())))?;
        connect(&mut self.transport).map_err(MachineError::Connect)?;
        self.connected = true;
        Ok(())
    }

    /// Allows motion. Fails unless every item of `verified` is set.
    pub fn arm(&mut self, verified: Verified) -> Result<(), MachineError> {
        if !self.connected {
            return Err(MachineError::NotConnected);
        }
        if !verified.is_complete() {
            return Err(MachineError::NotArmed);
        }
        self.armed = true;
        Ok(())
    }

    pub fn disarm(&mut self) {
        self.armed = false;
        let _ = self.transport.disarm_link();
    }

    /// Stops everything now: the queue held far from here is emptied and the stepper drivers are switched off.
    /// Works whether or not the machine is armed; the machine is disarmed afterwards.
    pub fn emergency_stop(&mut self) {
        self.armed = false;
        let _ = self.transport.abort_link();
        let _ = self.transport.set_spindle(0, false);
        // 0xC0 is the latch value that switches the drivers off; a transport that is not armed refuses it, which is fine
        // because its abort has already done the same.
        let _ = self.transport.write_byte(crate::bits::LATCH_B_IDLE);
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// Reads the machine inputs (limit switches and sensors). Allowed without arming.
    pub fn inputs(&mut self) -> Result<ModemStatus, MachineError> {
        if !self.connected {
            return Err(MachineError::NotConnected);
        }
        Ok(self.transport.status()?)
    }

    /// Reads the limit and fault lines. Allowed without arming.
    pub fn read_inputs(&mut self) -> Result<Inputs, MachineError> {
        let s = self.inputs()?;
        Ok(Inputs {
            limit: s.ri,
            fault: s.dcd,
        })
    }

    /// Sends planned steps, reading the inputs after every `check_every` steps and stopping at the
    /// first limit or fault signal. Refused unless armed. Inputs are also read once before moving,
    /// so a machine that is already at a limit or in fault does not move.
    pub fn run_until_limit(
        &mut self,
        steps: &[Step],
        check_every: usize,
    ) -> Result<RunOutcome, MachineError> {
        if !self.armed {
            return Err(MachineError::NotArmed);
        }
        let start = self.read_inputs()?;
        if start.limit || start.fault {
            return Ok(RunOutcome {
                sent: 0,
                limit_hit: start.limit,
                fault: start.fault,
            });
        }
        let every = check_every.max(1);
        for (i, s) in steps.iter().enumerate() {
            self.transport.write_byte(s.byte)?;
            self.transport.sleep_us(s.delay_us as u64);
            if (i + 1) % every == 0 {
                let st = self.transport.status()?;
                if st.ri || st.dcd {
                    return Ok(RunOutcome {
                        sent: i + 1,
                        limit_hit: st.ri,
                        fault: st.dcd,
                    });
                }
            }
        }
        Ok(RunOutcome {
            sent: steps.len(),
            limit_hit: false,
            fault: false,
        })
    }

    /// Sends planned steps. Refused unless armed.
    pub fn run(&mut self, steps: &[Step]) -> Result<(), MachineError> {
        if !self.armed {
            return Err(MachineError::NotArmed);
        }
        match self.pacing_for(steps) {
            Pacing::Bytes => Ok(run_steps(&mut self.transport, steps)?),
            Pacing::Stream => Ok(run_paced_with(&mut self.transport, steps, 0.0, false)?),
            Pacing::StreamKeepDir => Ok(run_paced_with(&mut self.transport, steps, 0.0, true)?),
        }
    }

    /// Sends planned steps paced by the serial line itself (see [`crate::paced_stream`]). Refused
    /// unless armed. Use this on a USB adapter, where single bytes written with pauses come out in bursts.
    pub fn run_paced(&mut self, steps: &[Step]) -> Result<(), MachineError> {
        if !self.armed {
            return Err(MachineError::NotArmed);
        }
        Ok(run_paced_with(
            &mut self.transport,
            steps,
            0.0,
            self.pacing == Pacing::StreamKeepDir,
        )?)
    }

    /// Like [`Machine::run`], but with a paced stream it returns while half of it is still going out (see
    /// [`crate::run_paced_ahead`]), so a caller that reads the inputs between pieces keeps the move continuous. The last piece of a
    /// move should be sent with [`Machine::run`], which waits for the end.
    pub fn run_ahead(&mut self, steps: &[Step]) -> Result<(), MachineError> {
        if !self.armed {
            return Err(MachineError::NotArmed);
        }
        match self.pacing_for(steps) {
            Pacing::Bytes => Ok(run_steps(&mut self.transport, steps)?),
            Pacing::Stream => Ok(run_paced_with(&mut self.transport, steps, 0.5, false)?),
            Pacing::StreamKeepDir => Ok(run_paced_with(&mut self.transport, steps, 0.5, true)?),
        }
    }

    /// Sends a search towards the switches and stops sending when the home line rises (RI: all three axes on their switches), which is
    /// seen within a chunk of the stream (`home_watch_chunk_bytes`) on a paced connection. Returns whether it rose. Refused unless
    /// armed. On a connection that sends one byte at a time the line is read between pieces of `home_check_piece_steps` steps instead.
    pub fn run_until_home(&mut self, steps: &[Step]) -> Result<bool, MachineError> {
        if !self.armed {
            return Err(MachineError::NotArmed);
        }
        let chunk = self.tuning.home_watch_chunk_bytes as usize;
        let pacing = self.pacing_for(steps);
        match pacing {
            Pacing::Bytes => {
                let piece = self.tuning.home_check_piece_steps.max(1) as usize;
                for part in steps.chunks(piece) {
                    if self.read_inputs()?.limit {
                        return Ok(true);
                    }
                    run_steps(&mut self.transport, part)?;
                }
                Ok(self.read_inputs()?.limit)
            }
            // `home_watch_chunk_bytes = 0`: the search goes out as one unbroken stream and the line is read once, at the end. Every
            // reading in the middle is a gap of a few milliseconds in the stream, which makes the motor lose steps.
            Pacing::Stream | Pacing::StreamKeepDir if chunk == 0 => {
                run_paced_with(
                    &mut self.transport,
                    steps,
                    0.0,
                    pacing == Pacing::StreamKeepDir,
                )?;
                Ok(self.read_inputs()?.limit)
            }
            Pacing::Stream => Ok(run_paced_until_home(
                &mut self.transport,
                steps,
                false,
                chunk,
            )?),
            Pacing::StreamKeepDir => Ok(run_paced_until_home(
                &mut self.transport,
                steps,
                true,
                chunk,
            )?),
        }
    }

    /// Sends a timed sequence of latch writes (spindle, drivers ...). Refused unless armed.
    pub fn run_ops(&mut self, ops: &[Op]) -> Result<(), MachineError> {
        if !self.armed {
            return Err(MachineError::NotArmed);
        }
        Ok(run_ops(&mut self.transport, ops)?)
    }

    /// Sets the spindle of the node (see [`Transport::set_spindle`]). Refused unless armed.
    pub fn set_spindle(&mut self, rpm: u32, reverse: bool) -> Result<(), MachineError> {
        if !self.armed {
            return Err(MachineError::NotArmed);
        }
        Ok(self.transport.set_spindle(rpm, reverse)?)
    }

    /// Sends a latch [`Sequence`] and, only if that succeeds, adopts its resulting latch state.
    pub fn apply(&mut self, seq: Sequence) -> Result<(), MachineError> {
        self.run_ops(&seq.ops)?;
        self.latches = seq.next;
        Ok(())
    }

    /// The latch state as last confirmed by a successful send.
    pub fn latches(&self) -> Latches {
        self.latches
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Waits `us` microseconds on the link's own clock (a simulated link does not really wait).
    pub fn sleep_us(&mut self, us: u64) {
        self.transport.sleep_us(us);
    }
}
