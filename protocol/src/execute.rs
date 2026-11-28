//! Runs a planned job ([`crate::job::Item`] list) on a [`Machine`], whatever the link behind it: a simulator, a serial port or the
//! node. The operator's dialogs and the cancel key come in through [`Hooks`].
//!
//! A cancel, an error or a refused dialog stops the machine at once ([`Machine::emergency_stop`]: the queue far from here is
//! emptied and the stepper drivers are switched off). Long moves, the searches towards the switches included, are cut into
//! pieces so a cancel is noticed within a fraction of a second, not at the end of the move.

use crate::job::{Item, Pause};
use crate::safety::{Machine, MachineError};
use crate::Pacing;
use crate::Step;
use crate::Transport;

/// What the caller gets to see and decide while the job runs.
pub trait Hooks {
    /// A dialog for the operator (tool change, spindle start). Return false to cancel the job.
    fn pause(&mut self, pause: Pause) -> bool;
    /// Called before each item with its index.
    fn progress(&mut self, next: usize, total: usize);
    /// Polled between pieces of a move; true cancels the job.
    fn cancelled(&mut self) -> bool;
    /// [`Hooks::pause`] for a front end that waits: it calls `idle` every few tens of milliseconds so the link stays alive while the operator takes their time.
    fn pause_waiting(&mut self, pause: Pause, idle: &mut dyn FnMut()) -> bool {
        let _ = idle;
        self.pause(pause)
    }
    /// The operator asked to pause. Only looked at where the tool is clear of the work (see [`run_items_resting`]).
    fn pause_requested(&mut self) -> bool {
        false
    }
    /// Waits for the operator at a step of a pause, calling `idle` as in [`Hooks::pause_waiting`]. Return false to cancel the job.
    fn rest(&mut self, rest: Rest, idle: &mut dyn FnMut()) -> bool {
        let _ = (rest, idle);
        true
    }
}

/// The two places a pause waits for the operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rest {
    /// The head is clear of the work and does not move. `spindle_stopped` says whether the pause also stopped the spindle.
    Paused { spindle_stopped: bool },
    /// The spindle was started again and had its time to reach speed; the head moves on once this is confirmed.
    SpindleUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Completed,
    Cancelled,
}

pub fn run_items<T: Transport>(
    m: &mut Machine<T>,
    items: &[Item],
    hooks: &mut dyn Hooks,
) -> Result<Outcome, MachineError> {
    run_items_resting(m, items, &[], hooks)
}

/// [`run_items`] that can be paused by the operator. `safe` are the indexes of the items after which the tool is clear of the work (the item that ends
/// each hole or path, as the planners mark them): a pause asked for at any other time waits for the next of them.
pub fn run_items_resting<T: Transport>(
    m: &mut Machine<T>,
    items: &[Item],
    safe: &[usize],
    hooks: &mut dyn Hooks,
) -> Result<Outcome, MachineError> {
    for (i, item) in items.iter().enumerate() {
        if hooks.cancelled() {
            m.emergency_stop();
            return Ok(Outcome::Cancelled);
        }
        hooks.progress(i, items.len());
        let result = match item {
            Item::Steps(steps) => {
                let mut r = Ok(());
                let pieces = steps.chunks(m.tuning().cancel_piece_steps as usize).count();
                for (k, piece) in steps
                    .chunks(m.tuning().cancel_piece_steps as usize)
                    .enumerate()
                {
                    if hooks.cancelled() {
                        m.emergency_stop();
                        return Ok(Outcome::Cancelled);
                    }
                    r = if k + 1 < pieces {
                        m.run_ahead(piece)
                    } else {
                        m.run(piece)
                    };
                    if r.is_err() {
                        break;
                    }
                }
                r
            }
            // A search towards the switches goes out as one continuous stream and stops being sent the moment the home line rises
            // (see `Machine::run_until_home`). The line is "all three axes on their switches", so it only rises for the last axis to
            // arrive. A search that never sees it is an error (`Search`) or just the end of a blind run (`Seek`).
            Item::Search(steps) if m.home_signal() => {
                if hooks.cancelled() {
                    m.emergency_stop();
                    return Ok(Outcome::Cancelled);
                }
                let short = hinted(m, steps).filter(|&n| n < steps.len());
                let first = short.map_or(steps.as_slice(), |n| &steps[..n]);
                match search_in_pieces(m, first, hooks) {
                    Ok(Homing::Cancelled) => {
                        m.emergency_stop();
                        return Ok(Outcome::Cancelled);
                    }
                    Ok(Homing::Reached) => Ok(()),
                    // the distance was wrong (the axis was further than the position said): try again with the whole search
                    Ok(Homing::NotReached) if short.is_some() => {
                        match search_in_pieces(m, steps, hooks) {
                            Ok(Homing::Reached) => Ok(()),
                            Ok(Homing::NotReached) => Err(MachineError::HomeNotFound),
                            Ok(Homing::Cancelled) => {
                                m.emergency_stop();
                                return Ok(Outcome::Cancelled);
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Ok(Homing::NotReached) => Err(MachineError::HomeNotFound),
                    Err(e) => Err(e),
                }
            }
            Item::Search(steps) => match run_in_pieces(m, steps, hooks) {
                Ok(true) => Ok(()),
                Ok(false) => {
                    m.emergency_stop();
                    return Ok(Outcome::Cancelled);
                }
                Err(e) => Err(e),
            },
            Item::Seek(steps) if m.home_signal() => {
                if hooks.cancelled() {
                    m.emergency_stop();
                    return Ok(Outcome::Cancelled);
                }
                // already on all three switches: nothing to search
                match m.read_inputs() {
                    Ok(inputs) if inputs.limit => Ok(()),
                    Ok(_) => {
                        let steps = hinted(m, steps).map_or(steps.as_slice(), |n| &steps[..n]);
                        let sent = if m.tuning().home_watch_seek {
                            search_in_pieces(m, steps, hooks).map(|h| h != Homing::Cancelled)
                        } else {
                            run_in_pieces(m, steps, hooks)
                        };
                        match sent {
                            Ok(true) => Ok(()),
                            Ok(false) => {
                                m.emergency_stop();
                                return Ok(Outcome::Cancelled);
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            Item::Seek(steps) => match run_in_pieces(m, steps, hooks) {
                Ok(true) => Ok(()),
                Ok(false) => {
                    m.emergency_stop();
                    return Ok(Outcome::Cancelled);
                }
                Err(e) => Err(e),
            },
            Item::Unstick(u) => {
                if !m.home_signal() {
                    Ok(())
                } else {
                    // The home line is up only when all three axes are on their switches, so X and Y are at their zero: moving away
                    // is safe, and the distance back is known.
                    match m.read_inputs() {
                        Ok(inputs) if inputs.limit || m.tuning().unstick_always => {
                            let parked = inputs.limit;
                            let moved = match run_in_pieces(m, &u.x, hooks) {
                                Ok(true) => run_in_pieces(m, &u.y, hooks),
                                other => other,
                            };
                            let seen = match moved {
                                Ok(false) => {
                                    m.emergency_stop();
                                    return Ok(Outcome::Cancelled);
                                }
                                Ok(true) => m.read_inputs(),
                                Err(e) => Err(e),
                            };
                            match seen {
                                Ok(after) => {
                                    // the line must drop: both axes really left their switches
                                    m.set_search_hint(if parked && !after.limit {
                                        Some((u.x_back, u.y_back))
                                    } else {
                                        None
                                    });
                                    Ok(())
                                }
                                Err(e) => Err(e),
                            }
                        }
                        Ok(_) => {
                            m.set_search_hint(None);
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
            }
            Item::Latch(ops) => m.run_ops(ops),
            Item::Spindle { rpm, wait_ms } => {
                // the speed first, then the wait for the spindle to get there (or to stop), in pieces so a cancel is noticed
                let started = m.set_spindle(*rpm, false);
                if started.is_ok() {
                    let mut left = u64::from(*wait_ms) * 1000;
                    while left > 0 {
                        if hooks.cancelled() {
                            m.emergency_stop();
                            return Ok(Outcome::Cancelled);
                        }
                        let piece = left.min(50_000);
                        m.sleep_us(piece);
                        left -= piece;
                    }
                }
                started
            }
            Item::Pause(p) => {
                let go_on = hooks.pause_waiting(*p, &mut || {
                    // a link that fails here says so at the next order
                    let _ = m.keep_alive();
                });
                if !go_on {
                    m.emergency_stop();
                    return Ok(Outcome::Cancelled);
                }
                Ok(())
            }
        };
        if let Err(e) = result {
            m.emergency_stop();
            return Err(e);
        }
        if i + 1 < items.len() && safe.contains(&i) && hooks.pause_requested() {
            match rest_here(m, &items[..=i], hooks) {
                Ok(true) => {}
                Ok(false) => {
                    m.emergency_stop();
                    return Ok(Outcome::Cancelled);
                }
                Err(e) => {
                    m.emergency_stop();
                    return Err(e);
                }
            }
        }
    }
    hooks.progress(items.len(), items.len());
    Ok(Outcome::Completed)
}

/// The spindle as the items already run have left it, with what it takes to stop it and to bring it back exactly as it was.
enum Spinning {
    /// The spindle of the node: its speed and the time it is given to reach it.
    Node { rpm: u32, wait_ms: u32 },
    /// The spindle of the control unit: the last byte written to each output latch.
    Latches { a: u8, b: u8 },
}

fn spinning(done: &[Item]) -> Option<Spinning> {
    use crate::latches::{Op, A_IDLE, A_SPINDLE_RUN, B_IDLE};
    let (mut a, mut b, mut node) = (A_IDLE, B_IDLE, None);
    for item in done {
        match item {
            Item::Latch(ops) => {
                for op in ops {
                    if let Op::Send(byte) = *op {
                        match byte & 0xC0 {
                            0x80 => a = byte,
                            0xC0 => b = byte,
                            _ => {}
                        }
                    }
                }
            }
            Item::Spindle { rpm, wait_ms } => node = (*rpm > 0).then_some((*rpm, *wait_ms)),
            _ => {}
        }
    }
    match node {
        Some((rpm, wait_ms)) => Some(Spinning::Node { rpm, wait_ms }),
        None => (a & A_SPINDLE_RUN != 0).then_some(Spinning::Latches { a, b }),
    }
}

/// The pause itself. `Ok(false)` means the operator cancelled: the caller stops the machine.
fn rest_here<T: Transport>(
    m: &mut Machine<T>,
    done: &[Item],
    hooks: &mut dyn Hooks,
) -> Result<bool, MachineError> {
    let spindle = if m.tuning().pause_stops_spindle {
        spinning(done)
    } else {
        None
    };
    if let Some(s) = &spindle {
        stop_spindle(m, s)?;
    }
    let wait = |m: &mut Machine<T>, hooks: &mut dyn Hooks, rest: Rest| {
        hooks.rest(rest, &mut || {
            let _ = m.keep_alive();
        })
    };
    let spindle_stopped = spindle.is_some();
    if !wait(m, hooks, Rest::Paused { spindle_stopped }) {
        return Ok(false);
    }
    // the link may have been lost while waiting: say so now and not half way into a move
    m.keep_alive()?;
    let Some(s) = spindle else {
        return Ok(true);
    };
    if !start_spindle(m, &s, hooks)? {
        return Ok(false);
    }
    Ok(wait(m, hooks, Rest::SpindleUp))
}

/// Stops the spindle the way the plan does: brake, release the run bit, release the brake (or speed 0 on the node).
fn stop_spindle<T: Transport>(m: &mut Machine<T>, s: &Spinning) -> Result<(), MachineError> {
    use crate::latches::{
        Op, A_AUX, A_LEVEL_MASK, A_SPINDLE_RUN, B_SPINDLE_BRAKE, B_SPINDLE_ENABLE,
    };
    let wait_ms = m.tuning().spindle_stop_wait_ms;
    match *s {
        Spinning::Node { .. } => {
            m.set_spindle(0, false)?;
            // the wait is not cut short: the next thing the operator may do is reach for the tool
            for _ in 0..wait_ms.div_ceil(50) {
                m.sleep_us(50_000);
            }
            Ok(())
        }
        Spinning::Latches { a, b } => {
            let braked = (b | B_SPINDLE_BRAKE) & !B_SPINDLE_ENABLE;
            m.run_ops(&[
                Op::Send(braked),
                Op::WaitUs(u64::from(wait_ms) * 1000),
                Op::Send(a & !(A_SPINDLE_RUN | A_AUX | A_LEVEL_MASK)),
                Op::WaitUs(10_000),
                Op::Send(braked & !B_SPINDLE_BRAKE),
                Op::WaitUs(10_000),
            ])
        }
    }
}

/// Starts the spindle again and leaves both latches as they were before the pause. `Ok(false)` when cancelled during the wait for its speed.
fn start_spindle<T: Transport>(
    m: &mut Machine<T>,
    s: &Spinning,
    hooks: &mut dyn Hooks,
) -> Result<bool, MachineError> {
    use crate::latches::{Op, A_AUX, A_LEVEL_MASK, SPINDLE_MIN_RPM, SPINDLE_STEP_RPM};
    match *s {
        Spinning::Node { rpm, wait_ms } => {
            m.set_spindle(rpm, false)?;
            Ok(wait_cancellable(m, wait_ms, hooks))
        }
        Spinning::Latches { a, b } => {
            let run = a & !(A_LEVEL_MASK | A_AUX);
            m.run_ops(&[Op::Send(b), Op::WaitUs(100_000), Op::Send(run)])?;
            let rpm = SPINDLE_MIN_RPM + u32::from(a & A_LEVEL_MASK) * SPINDLE_STEP_RPM;
            if !wait_cancellable(m, m.tuning().spindle_start_wait(rpm), hooks) {
                return Ok(false);
            }
            // the speed level and, when it was on, the auxiliary output with its settling time
            m.run_ops(&[Op::Send(a)])?;
            if a & A_AUX != 0 {
                m.run_ops(&[Op::WaitUs(500_000)])?;
            }
            Ok(true)
        }
    }
}

/// Waits in pieces; false when the job was cancelled meanwhile.
fn wait_cancellable<T: Transport>(m: &mut Machine<T>, wait_ms: u32, hooks: &mut dyn Hooks) -> bool {
    let mut left = u64::from(wait_ms) * 1000;
    while left > 0 {
        if hooks.cancelled() {
            return false;
        }
        let piece = left.min(50_000);
        m.sleep_us(piece);
        left -= piece;
    }
    !hooks.cancelled()
}

/// How a search towards the switches ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Homing {
    Reached,
    NotReached,
    Cancelled,
}

/// Sends `steps` in pieces of `cancel_piece_steps` and asks the cancel hook before each, so a stop is noticed within a piece and not at the end of a move that can last
/// several seconds. All but the last piece go out ahead (the line does not run dry between them). `Ok(false)` means cancelled: the caller stops the machine.
fn run_in_pieces<T: Transport>(
    m: &mut Machine<T>,
    steps: &[Step],
    hooks: &mut dyn Hooks,
) -> Result<bool, MachineError> {
    let size = (m.tuning().cancel_piece_steps as usize).max(1);
    let pieces = steps.chunks(size).count();
    for (k, piece) in steps.chunks(size).enumerate() {
        if hooks.cancelled() {
            return Ok(false);
        }
        if k + 1 < pieces {
            m.run_ahead(piece)?;
        } else {
            m.run(piece)?;
        }
    }
    Ok(true)
}

/// A search towards the switches with the cancel hook asked before each piece. With `home_watch_chunk_bytes = 0` (the default) the search is one unbroken stream and the home
/// line is read once, after the last piece, as it always was: the pieces before it go out ahead. With a watch chunk (or one byte at a time) every piece is watched on its own.
fn search_in_pieces<T: Transport>(
    m: &mut Machine<T>,
    steps: &[Step],
    hooks: &mut dyn Hooks,
) -> Result<Homing, MachineError> {
    let found = |reached: bool| {
        if reached {
            Homing::Reached
        } else {
            Homing::NotReached
        }
    };
    if steps.is_empty() {
        return Ok(found(m.run_until_home(steps)?));
    }
    let size = (m.tuning().cancel_piece_steps as usize).max(1);
    // One byte at a time the line is read between small pieces anyway, and a watch chunk asks for it: every piece is watched on its own.
    let watch = m.tuning().home_watch_chunk_bytes != 0 || m.pacing() == Pacing::Bytes;
    let pieces = steps.chunks(size).count();
    for (k, piece) in steps.chunks(size).enumerate() {
        if hooks.cancelled() {
            return Ok(Homing::Cancelled);
        }
        if watch {
            if m.run_until_home(piece)? {
                return Ok(Homing::Reached);
            }
        } else if k + 1 < pieces {
            m.run_ahead(piece)?;
        } else {
            return Ok(found(m.run_until_home(piece)?));
        }
    }
    Ok(Homing::NotReached)
}

/// The number of steps to send of a long search of X or Y when the distance to the switch is known; `None` for any other move.
fn hinted(m: &Machine<impl Transport>, steps: &[Step]) -> Option<usize> {
    let (x, y) = m.search_hint()?;
    match steps.first()?.byte {
        crate::bits::X_NEG => Some((x as usize).min(steps.len())),
        crate::bits::Y_NEG => Some((y as usize).min(steps.len())),
        _ => None,
    }
}
