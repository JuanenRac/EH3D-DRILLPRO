//! Position tracking with backlash compensation, modelled on the original positioning layer.
//!
//! The original keeps, per axis, the current position in steps and the direction of the last move
//! (initially "negative", towards the home switch). When a move reverses the direction it adds the
//! axis backlash to the steps actually sent, but the logical position only advances by the requested
//! amount. Recovered from the original code; not verified on hardware.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Positioner {
    /// Logical position in steps (X, Y).
    pub pos: (i64, i64),
    /// Backlash in steps (X, Y), added on every direction reversal.
    pub backlash: (u32, u32),
    last_negative: (bool, bool),
}

impl Positioner {
    pub fn new(backlash: (u32, u32)) -> Self {
        Self {
            pos: (0, 0),
            backlash,
            last_negative: (true, true),
        }
    }

    fn axis(delta: i64, last_negative: &mut bool, backlash: u32) -> i64 {
        if delta > 0 && *last_negative {
            *last_negative = false;
            delta + backlash as i64
        } else if delta < 0 && !*last_negative {
            *last_negative = true;
            delta - backlash as i64
        } else {
            delta
        }
    }

    /// Steps to send to reach `target` (logical steps), including backlash on reversals.
    /// Call [`Positioner::commit`] with the same target once the move has been carried out.
    pub fn plan(&mut self, target: (i64, i64)) -> (i64, i64) {
        let dx = Self::axis(
            target.0 - self.pos.0,
            &mut self.last_negative.0,
            self.backlash.0,
        );
        let dy = Self::axis(
            target.1 - self.pos.1,
            &mut self.last_negative.1,
            self.backlash.1,
        );
        (dx, dy)
    }

    /// Records that the machine reached `target` (or stopped short by `remaining` steps, already
    /// expressed in logical steps).
    pub fn commit(&mut self, target: (i64, i64), remaining: (i64, i64)) {
        self.pos = (target.0 - remaining.0, target.1 - remaining.1);
    }

    /// Declares the current position as the origin (after homing).
    pub fn set_origin(&mut self) {
        self.pos = (0, 0);
        self.last_negative = (true, true);
    }
}
