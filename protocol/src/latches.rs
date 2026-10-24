//! Output latches and the byte sequences that drive them (spindle, stepper drivers, shutdown).
//!
//! Recovered from the original program; the meaning of every bit here is an UNVERIFIED hypothesis.
//! Bytes with bit 7 set write a latch (bit 6 chooses which one); bytes with bit 7 clear are
//! motor step events (see [`crate::bits`]).
//!
//! Observed in the original code:
//! - latch A (`0x80` idle): bit 4 spindle run, bit 5 auxiliary output (500 ms settling), bits 0-3
//!   look like a spindle speed level.
//! - latch B (`0xC0` idle): bit 4 stepper drivers (enabled for any motion, released afterwards),
//!   bits 1, 3 and 5 take part in the spindle start/stop sequence.
//! - input lines: DCD = fault (spindle overload / unspecific error), RI = home or limit signal.
//! - closing the port sends `0x00`, `0x10`, `0xC0` with 10 ms between them.

use std::io;

use crate::Transport;

/// One element of a timed sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Send(u8),
    WaitUs(u64),
}

/// Spindle speed of level 0 and the speed step per level: the control unit has 16 levels,
/// 30000 rpm (level 0) to 60000 rpm (level 15). Verified at 30000, 36000, 42000, 54000 and
/// 60000 rpm; the rounding of speeds that are not a multiple of 2000 is not known.
pub const SPINDLE_MIN_RPM: u32 = 30_000;
pub const SPINDLE_STEP_RPM: u32 = 2_000;

/// Level (0 to 15) of a spindle speed in rpm.
pub fn spindle_level(rpm: u32) -> u8 {
    (rpm.saturating_sub(SPINDLE_MIN_RPM) / SPINDLE_STEP_RPM).min(15) as u8
}

pub const A_IDLE: u8 = 0x80;
pub const B_IDLE: u8 = 0xC0;
pub const A_SPINDLE_RUN: u8 = 0x10;
pub const A_AUX: u8 = 0x20;
pub const A_LEVEL_MASK: u8 = 0x0F;
pub const B_DRIVERS: u8 = 0x10;
pub const B_SPINDLE_BRAKE: u8 = 0x02;
pub const B_SPINDLE_ENABLE: u8 = 0x28;

/// Closing sequence of the original program.
pub const SHUTDOWN: [Op; 6] = [
    Op::Send(0x00),
    Op::WaitUs(10_000),
    Op::Send(0x10),
    Op::WaitUs(10_000),
    Op::Send(0xC0),
    Op::WaitUs(10_000),
];

/// Shadow copies of the two latches, so a bit can change without disturbing the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Latches {
    pub a: u8,
    pub b: u8,
}

impl Default for Latches {
    fn default() -> Self {
        Self {
            a: A_IDLE,
            b: B_IDLE,
        }
    }
}

/// A timed sequence plus the latch state that results once it has been sent.
/// Building a sequence never changes anything; the state is committed only after a successful send
/// (see `safety::Machine::apply`), so a refused or failed send cannot desynchronise the shadow copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequence {
    pub ops: Vec<Op>,
    pub next: Latches,
}

impl Latches {
    /// Stepper drivers on: set the bit, send, wait 10 ms.
    pub fn drivers_on(&self) -> Sequence {
        let next = Latches {
            b: self.b | B_DRIVERS,
            ..*self
        };
        Sequence {
            ops: vec![Op::Send(next.b), Op::WaitUs(10_000)],
            next,
        }
    }

    /// Stepper drivers off: clear the bit, wait 100 ms, send, wait 10 ms.
    pub fn drivers_off(&self) -> Sequence {
        let next = Latches {
            b: self.b & !B_DRIVERS,
            ..*self
        };
        Sequence {
            ops: vec![Op::WaitUs(100_000), Op::Send(next.b), Op::WaitUs(10_000)],
            next,
        }
    }

    /// Auxiliary output with the 500 ms settling time the original used.
    pub fn aux(&self, on: bool) -> Sequence {
        let a = if on { self.a | A_AUX } else { self.a & !A_AUX };
        let next = Latches { a, ..*self };
        Sequence {
            ops: vec![Op::Send(a), Op::WaitUs(500_000)],
            next,
        }
    }

    /// Spindle start: enable bits in latch B, run bit in latch A, then the speed level (0-15).
    /// `spin_up_us` is the time the original waited for the spindle to reach speed.
    pub fn spindle_on(&self, level: u8, spin_up_us: u64) -> Sequence {
        let b = self.b | B_SPINDLE_ENABLE;
        let a = self.a | A_SPINDLE_RUN;
        let ops = vec![
            Op::Send(b),
            Op::WaitUs(100_000),
            Op::Send(a),
            Op::WaitUs(spin_up_us),
            Op::Send(a | (level & A_LEVEL_MASK)),
        ];
        Sequence {
            ops,
            next: Latches { a, b },
        }
    }

    /// Spindle stop: brake on, drop the enables, release run, then release the brake.
    pub fn spindle_off(&self, spin_down_us: u64) -> Sequence {
        let braked = (self.b | B_SPINDLE_BRAKE) & !B_SPINDLE_ENABLE;
        let a = self.a & !A_SPINDLE_RUN;
        let b = braked & !B_SPINDLE_BRAKE;
        let ops = vec![
            Op::Send(braked),
            Op::WaitUs(spin_down_us),
            Op::Send(a),
            Op::WaitUs(10_000),
            Op::Send(b),
            Op::WaitUs(10_000),
        ];
        Sequence {
            ops,
            next: Latches { a, b },
        }
    }
}

pub fn run_ops<T: Transport>(t: &mut T, ops: &[Op]) -> io::Result<()> {
    for op in ops {
        match *op {
            Op::Send(b) => t.write_byte(b)?,
            Op::WaitUs(us) => t.sleep_us(us),
        }
    }
    Ok(())
}
