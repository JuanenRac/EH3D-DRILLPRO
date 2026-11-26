//! A link that goes wrong on purpose, for tests and for the simulation mode.
//!
//! [`FaultyLink`] wraps any [`Transport`] (normally the [`crate::Simulator`]) and injects the failures a
//! real cable, adapter or control unit can have: a unit that never answers the handshake, a fault line stuck
//! high, a limit switch that never closes, a cable pulled in the middle of a run, a status read that fails and
//! bytes lost on the line. Everything is deterministic: the lost bytes come from a seeded generator, so the same
//! seed loses the same bytes. [`FaultyLink::clear`] takes every fault away (the cable is plugged in again), so
//! a test can also check that the program recovers.
//!
//! Nothing here has run against a real machine: it only shows how the program behaves when the link fails.

use std::io;

use crate::{ModemStatus, Transport};

/// One failure of the link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The control unit never answers the handshake (DCD stays low).
    SilentUnit,
    /// DCD is high whatever DTR does (nothing, or something else, is connected).
    StuckFault,
    /// No limit switch ever closes (RI stays low).
    DeadLimitSwitch,
    /// Every write fails once this many bytes have gone out (the cable is pulled).
    CablePulled {
        /// Bytes written before the cable comes out.
        after: usize,
    },
    /// Every read of the modem lines fails.
    StatusReadFails,
}

/// A transport that wraps another one and injects [`Fault`]s into it.
#[derive(Debug)]
pub struct FaultyLink<T: Transport> {
    inner: T,
    faults: Vec<Fault>,
    written: usize,
    loss_per_mille: u32,
    rng: u64,
    dropped: usize,
}

impl<T: Transport> FaultyLink<T> {
    /// A link with no fault: it behaves exactly like `inner`.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            faults: Vec::new(),
            written: 0,
            loss_per_mille: 0,
            rng: 0,
            dropped: 0,
        }
    }

    /// Adds a fault.
    pub fn with(mut self, fault: Fault) -> Self {
        self.faults.push(fault);
        self
    }

    /// Loses `per_mille` bytes in every thousand, chosen by a generator started from `seed` (the same seed loses the same bytes).
    pub fn with_loss(mut self, seed: u64, per_mille: u32) -> Self {
        self.loss_per_mille = per_mille.min(1000);
        self.rng = seed | 1;
        self
    }

    /// Takes every fault away (and the loss): the link works again.
    pub fn clear(&mut self) {
        self.faults.clear();
        self.loss_per_mille = 0;
    }

    /// How many bytes were lost on the line.
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    /// How many bytes the program tried to write (lost ones included).
    pub fn written(&self) -> usize {
        self.written
    }

    /// The wrapped transport.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// The wrapped transport, to change.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    fn has(&self, f: Fault) -> bool {
        self.faults.contains(&f)
    }

    fn cable_pulled(&self) -> bool {
        self.faults
            .iter()
            .any(|f| matches!(f, Fault::CablePulled { after } if self.written >= *after))
    }

    /// xorshift64*: small, fixed and the same on every machine.
    fn next(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

impl<T: Transport> Transport for FaultyLink<T> {
    fn set_dtr(&mut self, on: bool) -> io::Result<()> {
        if self.cable_pulled() {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "cable pulled"));
        }
        self.inner.set_dtr(on)
    }

    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        if self.cable_pulled() {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "cable pulled"));
        }
        self.written += 1;
        if self.loss_per_mille > 0 && (self.next() >> 33) % 1000 < u64::from(self.loss_per_mille) {
            self.dropped += 1;
            return Ok(());
        }
        self.inner.write_byte(byte)
    }

    fn status(&mut self) -> io::Result<ModemStatus> {
        if self.has(Fault::StatusReadFails) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "the modem lines cannot be read",
            ));
        }
        let mut s = self.inner.status()?;
        if self.has(Fault::SilentUnit) {
            s.dcd = false;
        }
        if self.has(Fault::StuckFault) {
            s.dcd = true;
        }
        if self.has(Fault::DeadLimitSwitch) {
            s.ri = false;
        }
        Ok(s)
    }

    fn sleep_us(&mut self, us: u64) {
        self.inner.sleep_us(us);
    }

    fn wait_us(&mut self, us: u64) {
        self.inner.wait_us(us);
    }
}
