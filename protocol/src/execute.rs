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
                if !hooks.pause(*p) {
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
    }
    hooks.progress(items.len(), items.len());
    Ok(Outcome::Completed)
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
