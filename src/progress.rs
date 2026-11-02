//! How far a planned job has gone, weighted by the time each item takes on the machine, and how long is left. Shared by DRILLPRO and ROUTEPRO.
//!
//! A count of holes (or paths) says little: the reference runs, the long moves and the tool changes take much more than one more hole. The
//! progress here is the time of the items already done over the time of the whole plan, and the time left is what the rest of the plan takes on the
//! machine. The dialogs of the operator are not counted (nobody knows how long a person takes).

use crate::protocol::job::Item;
use crate::protocol::latches::{run_ops, Op};
use crate::protocol::{run_steps, Simulator};

/// How long an item of the plan takes on the machine, in seconds.
pub fn item_seconds(item: &Item) -> f64 {
    match item {
        Item::Steps(s) | Item::Search(s) | Item::Seek(s) => crate::protocol::motion::duration_s(s),
        Item::Latch(ops) => ops
            .iter()
            .map(|op| match op {
                Op::WaitUs(us) => *us as f64 / 1e6,
                Op::Send(_) => 0.0,
            })
            .sum(),
        Item::Spindle { wait_ms, .. } => f64::from(*wait_ms) / 1000.0,
        // the dialogs wait for a person; the unstick only happens when the machine is parked (it depends on the home line)
        Item::Pause(_) | Item::Unstick(_) => 0.0,
    }
}

/// Sends the bytes of one item to the simulator. The dialogs, the unstick and the spindle of the node have nothing to send there.
pub fn play(sim: &mut Simulator, item: &Item) {
    match item {
        Item::Steps(steps) | Item::Search(steps) | Item::Seek(steps) => {
            let _ = run_steps(sim, steps);
        }
        Item::Latch(ops) => {
            let _ = run_ops(sim, ops);
        }
        Item::Pause(_) | Item::Unstick(_) | Item::Spindle { .. } => {}
    }
}

/// The time of every item of a plan, added up, and the items that end a unit of work (a hole, a path).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Progress {
    /// `before[i]` is the seconds the items from the first to the `i`-th (not included) take; one more entry than items.
    before: Vec<f64>,
    marks: Vec<usize>,
}

impl Progress {
    /// `marks` are the indexes of the items that end each unit of work, as the planner gives them.
    pub fn new(items: &[Item], marks: &[usize]) -> Self {
        let mut before = Vec::with_capacity(items.len() + 1);
        let mut t = 0.0;
        before.push(t);
        for item in items {
            t += item_seconds(item);
            before.push(t);
        }
        Progress {
            before,
            marks: marks.to_vec(),
        }
    }

    /// The time the whole plan takes, in seconds.
    pub fn total_seconds(&self) -> f64 {
        self.before.last().copied().unwrap_or(0.0)
    }

    /// The time of the items before the `next`-th, plus `into_item` seconds of that one (the item that is running now).
    pub fn done_seconds(&self, next: usize, into_item: f64) -> f64 {
        let at = next.min(self.before.len().saturating_sub(1));
        let item = self
            .before
            .get(at + 1)
            .map_or(0.0, |end| end - self.before[at]);
        (self.before.get(at).copied().unwrap_or(0.0) + into_item.clamp(0.0, item))
            .min(self.total_seconds())
    }

    /// The time left, in seconds.
    pub fn remaining_seconds(&self, next: usize, into_item: f64) -> f64 {
        (self.total_seconds() - self.done_seconds(next, into_item)).max(0.0)
    }

    /// How much of the plan is done, from 0 to 1, by time. A plan that takes no time is done when all its items are.
    pub fn fraction(&self, next: usize, into_item: f64) -> f64 {
        let total = self.total_seconds();
        if total <= 0.0 {
            let items = self.before.len().saturating_sub(1);
            return if next >= items { 1.0 } else { 0.0 };
        }
        (self.done_seconds(next, into_item) / total).clamp(0.0, 1.0)
    }

    /// How many units of work are done when the run is about to start the `next`-th item.
    pub fn units_done(&self, next: usize) -> usize {
        self.marks.iter().filter(|&&e| e < next).count()
    }

    /// How many units of work the plan has.
    pub fn units_total(&self) -> usize {
        self.marks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{bits, Step};

    fn run_of(n: usize, delay_us: f64) -> Item {
        Item::Steps(vec![
            Step {
                byte: bits::X_POS,
                delay_us
            };
            n
        ])
    }

    #[test]
    fn a_long_item_weighs_more_than_a_short_one() {
        // one second and nine seconds: the first is a tenth of the plan, not half of it
        let items = vec![run_of(1000, 1000.0), run_of(9000, 1000.0)];
        let p = Progress::new(&items, &[0, 1]);
        assert!((p.total_seconds() - 10.0).abs() < 1e-6);
        assert!((p.fraction(1, 0.0) - 0.1).abs() < 1e-6);
        assert!((p.remaining_seconds(1, 0.0) - 9.0).abs() < 1e-6);
        assert_eq!(
            p.units_done(1),
            1,
            "the count of units is a different thing"
        );
    }

    #[test]
    fn the_running_item_counts_for_the_time_it_has_already_taken() {
        let items = vec![run_of(1000, 1000.0), run_of(9000, 1000.0)];
        let p = Progress::new(&items, &[0, 1]);
        assert!((p.fraction(1, 4.5) - 0.55).abs() < 1e-6);
        assert!((p.remaining_seconds(1, 4.5) - 4.5).abs() < 1e-6);
        // more time than the item takes does not go past its end
        assert!((p.fraction(0, 99.0) - 0.1).abs() < 1e-6);
    }

    #[test]
    fn the_start_is_zero_and_the_end_is_one() {
        let items = vec![run_of(10, 1000.0), run_of(10, 1000.0)];
        let p = Progress::new(&items, &[1]);
        assert_eq!(p.fraction(0, 0.0), 0.0);
        assert_eq!(p.fraction(2, 0.0), 1.0);
        assert_eq!(p.fraction(99, 0.0), 1.0, "a run past the end is done");
        assert_eq!(p.remaining_seconds(2, 0.0), 0.0);
    }

    #[test]
    fn the_dialogs_of_the_operator_take_no_time() {
        use crate::protocol::job::Pause;
        let items = vec![Item::Pause(Pause::SpindleStart), run_of(1000, 1000.0)];
        let p = Progress::new(&items, &[1]);
        assert!((p.total_seconds() - 1.0).abs() < 1e-6);
        assert_eq!(p.fraction(1, 0.0), 0.0);
    }

    #[test]
    fn a_plan_with_no_time_is_done_when_its_items_are() {
        let items = vec![Item::Latch(vec![Op::Send(0x10)])];
        let p = Progress::new(&items, &[]);
        assert_eq!(p.fraction(0, 0.0), 0.0);
        assert_eq!(p.fraction(1, 0.0), 1.0);
        assert_eq!(Progress::default().fraction(0, 0.0), 1.0);
    }

    #[test]
    fn the_latch_waits_and_the_spindle_waits_count() {
        let items = vec![
            Item::Latch(vec![Op::Send(0x10), Op::WaitUs(2_000_000)]),
            Item::Spindle {
                rpm: 30000,
                wait_ms: 3000,
            },
        ];
        let p = Progress::new(&items, &[]);
        assert!((p.total_seconds() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn the_units_are_counted_from_the_marks() {
        let items = vec![run_of(1, 1.0), run_of(1, 1.0), run_of(1, 1.0)];
        let p = Progress::new(&items, &[0, 2]);
        assert_eq!(p.units_total(), 2);
        assert_eq!(p.units_done(0), 0);
        assert_eq!(
            p.units_done(1),
            1,
            "the item 0 is done when the run is at 1"
        );
        assert_eq!(p.units_done(2), 1);
        assert_eq!(p.units_done(3), 2);
    }

    #[test]
    fn play_sends_the_bytes_of_the_steps_to_the_simulator() {
        let mut sim = Simulator::default();
        play(&mut sim, &run_of(5, 1000.0));
        assert_eq!(sim.sent.len(), 5);
        play(
            &mut sim,
            &Item::Pause(crate::protocol::job::Pause::SpindleStart),
        );
        assert_eq!(sim.sent.len(), 5);
    }
}
