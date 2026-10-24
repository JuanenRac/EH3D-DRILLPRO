//! Runs a planned job ([`crate::job::Item`] list) on a [`Machine`], whatever the link behind it: a simulator, a serial port or the
//! node. The operator's dialogs and the cancel key come in through [`Hooks`].
//!
//! A cancel, an error or a refused dialog stops the machine at once ([`Machine::emergency_stop`]: the queue far from here is
//! emptied and the stepper drivers are switched off). Long moves are cut into pieces so a cancel is noticed within a fraction
//! of a second, not at the end of the move.

use crate::job::{Item, Pause};
use crate::safety::{Machine, MachineError};
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
                match m.run_until_home(first) {
                    Ok(true) => Ok(()),
                    // the distance was wrong (the axis was further than the position said): try again with the whole search
                    Ok(false) if short.is_some() => match m.run_until_home(steps) {
                        Ok(true) => Ok(()),
                        Ok(false) => Err(MachineError::HomeNotFound),
                        Err(e) => Err(e),
                    },
                    Ok(false) => Err(MachineError::HomeNotFound),
                    Err(e) => Err(e),
                }
            }
            Item::Search(steps) => m.run(steps),
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
                        if m.tuning().home_watch_seek {
                            m.run_until_home(steps).map(|_| ())
                        } else {
                            m.run(steps)
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            Item::Seek(steps) => m.run(steps),
            Item::Unstick(u) => {
                if !m.home_signal() {
                    Ok(())
                } else {
                    // The home line is up only when all three axes are on their switches, so X and Y are at their zero: moving away
                    // is safe, and the distance back is known.
                    match m.read_inputs() {
                        Ok(inputs) if inputs.limit || m.tuning().unstick_always => {
                            let parked = inputs.limit;
                            let moved = m.run(&u.x).and_then(|_| m.run(&u.y));
                            match moved.and_then(|_| m.read_inputs()) {
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

/// The number of steps to send of a long search of X or Y when the distance to the switch is known; `None` for any other move.
fn hinted(m: &Machine<impl Transport>, steps: &[Step]) -> Option<usize> {
    let (x, y) = m.search_hint()?;
    match steps.first()?.byte {
        crate::bits::X_NEG => Some((x as usize).min(steps.len())),
        crate::bits::Y_NEG => Some((y as usize).min(steps.len())),
        _ => None,
    }
}
