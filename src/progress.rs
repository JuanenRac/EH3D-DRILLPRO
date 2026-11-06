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

/// How many steps apart the points are that the track of the head keeps inside an item (a whole path of the router is one item: its corners must be in the track).
pub const TRACK_EVERY_STEPS: usize = 32;

/// One point of the track of the head inside an item: the seconds from the start of the item, and X, Y and Z in mm from the switches.
type TrackPoint = (f64, (f64, f64, f64));

/// The time of every item of a plan, added up, and the items that end a unit of work (a hole, a path).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Progress {
    /// `before[i]` is the seconds the items from the first to the `i`-th (not included) take; one more entry than items.
    before: Vec<f64>,
    marks: Vec<usize>,
    /// `ends[i]` is where the head is (X, Y, Z in mm from the switches) after the items before the `i`-th; one more entry than items, empty when the
    /// positions were not asked for (see [`Progress::with_track`]).
    ends: Vec<(f64, f64, f64)>,
    /// For each item that moves the axes, the points the head passes inside it (so that a path of the router, which is one item, is followed along its corners and
    /// not in a straight line from where it begins to where it ends); empty with `ends`.
    inner: Vec<Vec<TrackPoint>>,
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
            ends: Vec::new(),
            inner: Vec::new(),
        }
    }

    /// The same, following the steps of the items from the switches to know where the head is at any moment of the plan (what a view that draws the
    /// machine moving needs: the trips to the tool bar are in the plan as moves like any other).
    pub fn with_track(
        mut self,
        items: &[Item],
        geometry: &crate::protocol::motion::Geometry,
    ) -> Self {
        let per_mm = [
            geometry.steps_per_mm_xy * geometry.correction.0,
            geometry.steps_per_mm_xy * geometry.correction.1,
            geometry.steps_per_mm_z * geometry.correction.2,
        ];
        let mut at = [0i64; 3];
        let apply = |steps: &[crate::protocol::Step], at: &mut [i64; 3]| {
            for s in steps {
                let (dx, dy, dz) = eh3d_summary::dryrun::decode(s.byte);
                for (i, d) in [dx, dy, dz].into_iter().enumerate() {
                    // an axis cannot go behind its switch
                    at[i] = (at[i] + d).max(0);
                }
            }
        };
        let mm = |at: &[i64; 3]| {
            let f = |i: usize| {
                if per_mm[i] > 0.0 {
                    at[i] as f64 / per_mm[i]
                } else {
                    0.0
                }
            };
            (f(0), f(1), f(2))
        };
        let mut ends = Vec::with_capacity(items.len() + 1);
        let mut inner: Vec<Vec<TrackPoint>> = Vec::with_capacity(items.len());
        ends.push(mm(&at));
        for item in items {
            let mut way: Vec<TrackPoint> = Vec::new();
            match item {
                Item::Steps(s) | Item::Search(s) | Item::Seek(s) => {
                    way.push((0.0, mm(&at)));
                    let mut t = 0.0;
                    for (k, st) in s.iter().enumerate() {
                        apply(std::slice::from_ref(st), &mut at);
                        t += st.delay_us / 1e6;
                        if (k + 1).is_multiple_of(TRACK_EVERY_STEPS) || k + 1 == s.len() {
                            way.push((t, mm(&at)));
                        }
                    }
                }
                Item::Unstick(u) => {
                    apply(&u.x, &mut at);
                    apply(&u.y, &mut at);
                }
                _ => {}
            }
            ends.push(mm(&at));
            inner.push(way);
        }
        self.ends = ends;
        self.inner = inner;
        self
    }

    /// Where the head is (X, Y, Z in mm from the switches) when the run is about to start the `next`-th item and is `into_item` seconds into the one it is on
    /// (the head moves in a straight line along an item, as the steps of one item go the same way); `None` when the plan was not followed.
    pub fn head_mm(&self, next: usize, into_item: f64) -> Option<(f64, f64, f64)> {
        let items = self.ends.len().checked_sub(1)?;
        let at = next.min(items);
        let from = self.ends[at];
        if at >= items {
            return Some(from);
        }
        // inside an item that has its points: between the two the head is passing
        if let Some(way) = self.inner.get(at).filter(|w| w.len() > 1) {
            let t = into_item.max(0.0);
            let j = way.partition_point(|(tk, _)| *tk <= t);
            if j == 0 {
                return Some(way[0].1);
            }
            if j >= way.len() {
                return Some(way[way.len() - 1].1);
            }
            let ((t0, p0), (t1, p1)) = (way[j - 1], way[j]);
            let k = if t1 > t0 {
                ((t - t0) / (t1 - t0)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            return Some((
                p0.0 + (p1.0 - p0.0) * k,
                p0.1 + (p1.1 - p0.1) * k,
                p0.2 + (p1.2 - p0.2) * k,
            ));
        }
        let to = self.ends[at + 1];
        let dur = self.before.get(at + 1).map_or(0.0, |e| e - self.before[at]);
        let k = if dur > 0.0 {
            (into_item / dur).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Some((
            from.0 + (to.0 - from.0) * k,
            from.1 + (to.1 - from.1) * k,
            from.2 + (to.2 - from.2) * k,
        ))
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

    #[test]
    fn the_head_is_followed_along_the_steps_of_the_plan() {
        let g = crate::protocol::motion::Geometry::default();
        let per_mm = g.steps_per_mm_xy * g.correction.0;
        let n = (20.0 * per_mm).round() as usize;
        let items = vec![
            run_of(n, 500.0),
            Item::Steps(vec![
                Step {
                    byte: bits::Y_POS,
                    delay_us: 500.0
                };
                n
            ]),
            // going back past the switch is swallowed: the head cannot be behind it
            Item::Steps(vec![
                Step {
                    byte: bits::X_NEG,
                    delay_us: 500.0
                };
                n * 2
            ]),
        ];
        let p = Progress::new(&items, &[]);
        assert_eq!(p.head_mm(0, 0.0), None, "no track unless asked for");
        let p = p.with_track(&items, &g);
        let (x, y, _) = p.head_mm(1, 0.0).unwrap();
        assert!((x - 20.0).abs() < 0.1 && y.abs() < 0.01, "{x} {y}");
        // half way into the second item the head is half way up Y
        let half = n as f64 * 500e-6 / 2.0;
        let (x, y, _) = p.head_mm(1, half).unwrap();
        assert!((x - 20.0).abs() < 0.1 && (y - 10.0).abs() < 0.2, "{x} {y}");
        let (x, _, _) = p.head_mm(3, 0.0).unwrap();
        assert!(x.abs() < 0.01, "the head stops on its switch: {x}");
        // past the end it stays where the plan ends
        assert_eq!(p.head_mm(99, 0.0), p.head_mm(3, 0.0));
    }

    #[test]
    fn a_single_item_with_a_corner_is_followed_round_the_corner() {
        let g = crate::protocol::motion::Geometry::default();
        let per_mm = g.steps_per_mm_xy * g.correction.0;
        let n = (20.0 * per_mm).round() as usize;
        // one item (as a whole path of the router is): 20 mm along X and then 20 mm along Y
        let leg = |byte| {
            vec![
                Step {
                    byte,
                    delay_us: 500.0
                };
                n
            ]
        };
        let mut steps = leg(bits::X_POS);
        steps.extend(leg(bits::Y_POS));
        let items = vec![Item::Steps(steps)];
        let p = Progress::new(&items, &[0]).with_track(&items, &g);
        // at the end of the first leg the head is at the corner, not half way along the diagonal
        let (x, y, _) = p.head_mm(0, n as f64 * 500e-6).unwrap();
        assert!((x - 20.0).abs() < 1.0 && y.abs() < 1.0, "{x} {y}");
        // half way up the second leg
        let (x, y, _) = p.head_mm(0, n as f64 * 1.5 * 500e-6).unwrap();
        assert!((x - 20.0).abs() < 1.0 && (y - 10.0).abs() < 1.0, "{x} {y}");
        // the start and the end of the item
        let (x, y, _) = p.head_mm(0, 0.0).unwrap();
        assert!(x.abs() < 0.1 && y.abs() < 0.1);
        let (x, y, _) = p.head_mm(1, 0.0).unwrap();
        assert!((x - 20.0).abs() < 0.1 && (y - 20.0).abs() < 0.1);
    }
}
