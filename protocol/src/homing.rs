//! Homing sequences of the original program, driven by the limit switches.
//!
//! Observed on the serial line (original DRILLPRO and ROUTEPRO): Z goes up (`Z_NEG` bytes) until the
//! limit line rises (the whole travel is 1320 steps for a 37.30 mm maximum depth), backs off 45 steps
//! down, and approaches again 54 steps up. X and Y then move together diagonally towards the origin
//! (0x05 bytes, up to 26613 steps), back off 200 steps, and approach again 250 steps. The step
//! rates are not verified.

use crate::motion::Geometry;
use crate::safety::{Machine, MachineError, RunOutcome};
use crate::{bits, Step, Transport};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HomingParams {
    /// Longest Z search in steps (the original uses the maximum depth).
    pub z_search_steps: u32,
    pub z_backoff_steps: u32,
    pub z_reapproach_steps: u32,
    /// Longest diagonal XY search in steps.
    pub xy_search_steps: u32,
    pub xy_backoff_steps: u32,
    pub xy_reapproach_steps: u32,
    /// Step rate of the searches (Hz). Not verified.
    pub search_hz: f64,
    /// Inputs are read after this many steps.
    pub check_every: usize,
    /// The limit switches cannot be read: the machine itself stops an axis at its switch and the
    /// searches run their full length (see [`HomingParams::blind`]).
    pub blind: bool,
}

impl HomingParams {
    /// Parameters for the real machine. Its control unit lights a lamp and stops the axis in that
    /// direction when a limit switch is pressed, but it reports nothing on the serial input lines
    /// (RI and DCD only follow DTR), so the original sends the full search, backs off and
    /// approaches again without reading anything.
    pub fn blind() -> Self {
        Self {
            blind: true,
            ..Self::default()
        }
    }
}

impl Default for HomingParams {
    fn default() -> Self {
        Self {
            z_search_steps: 1320,
            z_backoff_steps: 45,
            z_reapproach_steps: 54,
            xy_search_steps: 26613,
            xy_backoff_steps: 200,
            xy_reapproach_steps: 250,
            search_hz: 1000.0,
            check_every: 1,
            blind: false,
        }
    }
}

#[derive(Debug)]
pub enum HomingError {
    /// The search ended without the limit line rising ("HOME signal error" in the original).
    NoLimitSignal,
    /// The fault line (DCD) rose during the move.
    Fault,
    Machine(MachineError),
}

impl From<MachineError> for HomingError {
    fn from(e: MachineError) -> Self {
        HomingError::Machine(e)
    }
}

fn constant_rate(byte: u8, count: u32, hz: f64) -> Vec<Step> {
    (0..count)
        .map(|_| Step {
            byte,
            delay_us: 1e6 / hz,
        })
        .collect()
}

/// Runs a search towards a switch. With readable switches it stops on the limit line and demands
/// it; on the real machine (blind) it sends every step.
fn seek<T: Transport>(
    m: &mut Machine<T>,
    steps: &[Step],
    p: &HomingParams,
) -> Result<(), HomingError> {
    if p.blind {
        m.run(steps)?;
        Ok(())
    } else {
        require_limit(m.run_until_limit(steps, p.check_every)?)
    }
}

fn require_limit(o: RunOutcome) -> Result<(), HomingError> {
    if o.fault {
        Err(HomingError::Fault)
    } else if !o.limit_hit {
        Err(HomingError::NoLimitSignal)
    } else {
        Ok(())
    }
}

/// Homes Z: search up to the switch, back off, approach again. Leaves the tool on the switch.
/// The geometry is accepted for symmetry with the other planners; the counts are in steps.
pub fn home_z<T: Transport>(
    m: &mut Machine<T>,
    p: &HomingParams,
    _g: &Geometry,
) -> Result<(), HomingError> {
    let up = constant_rate(bits::Z_NEG, p.z_search_steps, p.search_hz);
    seek(m, &up, p)?;
    m.run(&constant_rate(bits::Z_POS, p.z_backoff_steps, p.search_hz))?;
    let again = constant_rate(bits::Z_NEG, p.z_reapproach_steps, p.search_hz);
    seek(m, &again, p)
}

/// Homes X and Y together, diagonally towards the origin.
pub fn home_xy<T: Transport>(m: &mut Machine<T>, p: &HomingParams) -> Result<(), HomingError> {
    let toward = bits::X_NEG | bits::Y_NEG;
    let away = bits::X_POS | bits::Y_POS;
    let search = constant_rate(toward, p.xy_search_steps, p.search_hz);
    seek(m, &search, p)?;
    m.run(&constant_rate(away, p.xy_backoff_steps, p.search_hz))?;
    let again = constant_rate(toward, p.xy_reapproach_steps, p.search_hz);
    seek(m, &again, p)
}
