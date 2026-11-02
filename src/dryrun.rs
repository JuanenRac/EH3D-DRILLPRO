//! Dry run of a planned job: the exact steps of the plan are followed on paper, without a machine, to know how far each axis goes and
//! how long the movements take. A plan that leaves the course of the machine is found here, before a single motor turns.
//!
//! The measure itself lives in EH3D-COMMON (`eh3d-summary`), shared with the other programs; this file turns the items of a plan of the
//! machine protocol into what that measure follows.

use crate::protocol::job::Item;
use crate::protocol::latches::Op;
use crate::protocol::motion::Geometry;
pub use eh3d_summary::dryrun::{decode, Axis, Course, DryRun, Tracker, Violation};

/// Follows the items of a plan from the switches (every axis at 0) and measures them against `course`. `margin_mm` is how far past the
/// course an axis may go before it counts (the play of the machine); it comes from the configuration.
pub fn run(items: &[Item], geometry: &Geometry, course: &Course, margin_mm: f64) -> DryRun {
    let per_mm = [
        geometry.steps_per_mm_xy * geometry.correction.0,
        geometry.steps_per_mm_xy * geometry.correction.1,
        geometry.steps_per_mm_z * geometry.correction.2,
    ];
    let mut t = Tracker::new(per_mm, course, margin_mm);
    let steps =
        |s: &[crate::protocol::Step]| s.iter().map(|x| (x.byte, x.delay_us)).collect::<Vec<_>>();
    for item in items {
        match item {
            Item::Steps(s) | Item::Search(s) | Item::Seek(s) => t.steps(steps(s)),
            Item::Unstick(u) => {
                t.steps(steps(&u.x));
                t.steps(steps(&u.y));
            }
            Item::Latch(ops) => {
                for op in ops {
                    if let Op::WaitUs(us) = op {
                        t.wait_us(*us as f64);
                    }
                }
            }
            Item::Spindle { wait_ms, .. } => t.wait_us(f64::from(*wait_ms) * 1000.0),
            Item::Pause(_) => {}
        }
        t.end_item();
    }
    t.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{bits, Step};

    fn run_of(byte: u8, n: usize) -> Vec<Step> {
        vec![
            Step {
                byte,
                delay_us: 500.0
            };
            n
        ]
    }

    fn geometry() -> Geometry {
        Geometry::default()
    }

    #[test]
    fn a_move_inside_the_course_is_measured() {
        let g = geometry();
        let n = (10.0 * g.steps_per_mm_xy * g.correction.0) as usize;
        let items = vec![Item::Steps(run_of(bits::X_POS, n))];
        let course = Course {
            x_mm: 100.0,
            y_mm: 100.0,
            z_mm: 20.0,
        };
        let d = run(&items, &g, &course, 0.5);
        assert!(d.within_course());
        assert!((d.max_mm.0 - 10.0).abs() < 0.1, "{:?}", d.max_mm);
        assert!((d.end_mm.0 - 10.0).abs() < 0.1);
        assert_eq!(d.steps, n as u64);
        assert!((d.seconds - n as f64 * 500e-6).abs() < 1e-9);
    }

    #[test]
    fn a_move_past_the_far_end_is_a_violation_with_its_item() {
        let g = geometry();
        let n = (30.0 * g.steps_per_mm_xy * g.correction.0) as usize;
        let items = vec![
            Item::Steps(run_of(bits::Y_POS, 10)),
            Item::Steps(run_of(bits::X_POS, n)),
        ];
        let course = Course {
            x_mm: 20.0,
            y_mm: 100.0,
            z_mm: 20.0,
        };
        let d = run(&items, &g, &course, 0.5);
        assert_eq!(d.violations.len(), 1);
        let v = d.violations[0];
        assert_eq!((v.axis, v.item), (Axis::X, 1));
        assert!(
            v.excess_mm() > 9.0 && v.excess_mm() < 10.5,
            "{}",
            v.excess_mm()
        );
    }

    #[test]
    fn going_behind_the_switch_is_swallowed_not_a_violation() {
        let g = geometry();
        let items = vec![Item::Search(run_of(bits::X_NEG, 5000))];
        let course = Course {
            x_mm: 100.0,
            y_mm: 100.0,
            z_mm: 20.0,
        };
        let d = run(&items, &g, &course, 0.5);
        assert!(d.within_course(), "a search stops on its switch");
        assert_eq!(d.end_mm.0, 0.0);
        assert_eq!(d.swallowed_steps, 5000);
        // the same for steps that are not a search: the axis cannot go behind its switch, and coming back starts from the switch
        let items = [
            Item::Steps(run_of(bits::X_NEG, 100)),
            Item::Steps(run_of(bits::X_POS, 50)),
        ];
        let d = run(&items, &g, &course, 0.5);
        assert!(d.within_course());
        assert_eq!(d.swallowed_steps, 100);
        assert!((d.end_mm.0 - 50.0 / (g.steps_per_mm_xy * g.correction.0)).abs() < 1e-9);
    }

    #[test]
    fn z_down_is_measured_against_the_deepest_position() {
        let g = geometry();
        let n = (15.0 * g.steps_per_mm_z * g.correction.2) as usize;
        let course = Course {
            x_mm: 100.0,
            y_mm: 100.0,
            z_mm: 10.0,
        };
        let d = run(&[Item::Steps(run_of(bits::Z_POS, n))], &g, &course, 0.5);
        assert_eq!(d.violations.len(), 1);
        assert_eq!(d.violations[0].axis, Axis::Z);
    }

    #[test]
    fn the_margin_forgives_the_play() {
        let g = geometry();
        let n = (10.4 * g.steps_per_mm_xy * g.correction.0) as usize;
        let course = Course {
            x_mm: 10.0,
            y_mm: 10.0,
            z_mm: 10.0,
        };
        let items = [Item::Steps(run_of(bits::X_POS, n))];
        assert!(run(&items, &g, &course, 0.5).within_course());
        assert!(!run(&items, &g, &course, 0.1).within_course());
    }

    #[test]
    fn waits_and_the_spindle_add_time_but_no_steps() {
        let g = geometry();
        let items = vec![
            Item::Latch(vec![Op::WaitUs(250_000)]),
            Item::Spindle {
                rpm: 1000,
                wait_ms: 750,
            },
        ];
        let course = Course {
            x_mm: 1.0,
            y_mm: 1.0,
            z_mm: 1.0,
        };
        let d = run(&items, &g, &course, 0.5);
        assert_eq!(d.steps, 0);
        assert!((d.seconds - 1.0).abs() < 1e-9);
    }
}
