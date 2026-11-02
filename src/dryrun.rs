//! Dry run of a planned job: the exact steps of the plan are followed on paper, without a machine, to know how far each axis goes and
//! how long the movements take. A plan that leaves the course of the machine is found here, before a single motor turns.
//!
//! Shared by DRILLPRO, ROUTEPRO, the interface of both and the EH3D server. The steps are decoded the way the control unit reads
//! them (the step bit and the direction bit of each axis), so what is measured is what the machine would be asked to do.

use crate::config::MachineSettings;
use crate::protocol::job::Item;
use crate::protocol::latches::Op;
use crate::protocol::motion::Geometry;

/// The axes of the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn letter(self) -> char {
        match self {
            Axis::X => 'X',
            Axis::Y => 'Y',
            Axis::Z => 'Z',
        }
    }
}

/// How far each axis may go from its switch, in mm (Z downwards, from the top).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Course {
    pub x_mm: f64,
    pub y_mm: f64,
    pub z_mm: f64,
}

impl Course {
    /// The course the configuration file of the machine gives: the clip of X and Y and the deepest Z.
    pub fn from_settings(m: &MachineSettings) -> Course {
        Course {
            x_mm: m.clip_mm.0,
            y_mm: m.clip_mm.1,
            z_mm: m.z_max_depth_mm,
        }
    }
}

/// An axis that goes past its course.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Violation {
    /// The item of the plan where it happens (the first one that does).
    pub item: usize,
    pub axis: Axis,
    /// Where the axis gets to, in mm from its switch.
    pub at_mm: f64,
    /// The end of the course it passes, in mm (0 for the switch side).
    pub limit_mm: f64,
}

impl Violation {
    /// How far past the limit, in mm.
    pub fn excess_mm(&self) -> f64 {
        (self.at_mm - self.limit_mm).abs()
    }
}

/// What the plan does, measured.
#[derive(Debug, Clone, PartialEq)]
pub struct DryRun {
    /// Lowest and highest position of each axis (X, Y, Z) reached by the movements, in mm from the switches (nothing goes behind a switch, so the
    /// lowest is 0).
    pub min_mm: (f64, f64, f64),
    pub max_mm: (f64, f64, f64),
    /// Where the plan ends, in mm.
    pub end_mm: (f64, f64, f64),
    /// Steps sent (one per motor event) and the seconds the movements and the waits take (the dialogs of the operator not counted).
    pub steps: u64,
    pub seconds: f64,
    /// Items of the plan that were followed.
    pub items: usize,
    /// Steps the switches swallowed: an axis cannot go behind its switch, the control unit takes the surplus (the approach of the calibration jump asks
    /// for a few steps more than it backed off on purpose).
    pub swallowed_steps: u64,
    /// The places where an axis goes past the far end of its course (at most one per axis).
    pub violations: Vec<Violation>,
}

impl DryRun {
    pub fn within_course(&self) -> bool {
        self.violations.is_empty()
    }
}

/// The movement a step byte asks for, in steps of X, Y and Z (positive: away from the switch, down for Z).
pub fn decode(byte: u8) -> (i64, i64, i64) {
    if byte & 0x80 != 0 {
        return (0, 0, 0);
    }
    let axis = |step: u8, dir: u8| -> i64 {
        if byte & step == 0 {
            0
        } else if byte & dir != 0 {
            1
        } else {
            -1
        }
    };
    (axis(0x01, 0x02), axis(0x04, 0x08), axis(0x10, 0x40))
}

struct Walker {
    at: [i64; 3],
    max: [i64; 3],
    steps: u64,
    swallowed: u64,
    us: f64,
}

impl Walker {
    fn run(&mut self, steps: &[crate::protocol::Step]) {
        for s in steps {
            let (dx, dy, dz) = decode(s.byte);
            for (i, d) in [dx, dy, dz].into_iter().enumerate() {
                if d != 0 {
                    self.at[i] += d;
                    if self.at[i] < 0 {
                        // the axis is on its switch: what it asked for beyond is swallowed by the control unit
                        self.at[i] = 0;
                        self.swallowed += 1;
                    }
                    self.max[i] = self.max[i].max(self.at[i]);
                }
            }
            self.steps += 1;
            self.us += s.delay_us;
        }
    }
}

/// Follows the items of a plan from the switches (every axis at 0) and measures them against `course`. `margin_mm` is how far past the
/// course an axis may go before it counts (the play of the machine); it comes from the configuration.
pub fn run(items: &[Item], geometry: &Geometry, course: &Course, margin_mm: f64) -> DryRun {
    let mut w = Walker {
        at: [0; 3],
        max: [0; 3],
        steps: 0,
        swallowed: 0,
        us: 0.0,
    };
    let per_mm = [
        geometry.steps_per_mm_xy * geometry.correction.0,
        geometry.steps_per_mm_xy * geometry.correction.1,
        geometry.steps_per_mm_z * geometry.correction.2,
    ];
    let limit = [course.x_mm, course.y_mm, course.z_mm];
    let axes = [Axis::X, Axis::Y, Axis::Z];
    let mut violations: Vec<Violation> = Vec::new();
    let note = |w: &Walker, item: usize, violations: &mut Vec<Violation>| {
        for i in 0..3 {
            if per_mm[i] <= 0.0 {
                continue;
            }
            let high = w.max[i] as f64 / per_mm[i];
            if high > limit[i] + margin_mm && !violations.iter().any(|v| v.axis == axes[i]) {
                violations.push(Violation {
                    item,
                    axis: axes[i],
                    at_mm: high,
                    limit_mm: limit[i],
                });
            }
        }
    };
    for (n, item) in items.iter().enumerate() {
        match item {
            Item::Steps(s) | Item::Search(s) | Item::Seek(s) => w.run(s),
            Item::Unstick(u) => {
                w.run(&u.x);
                w.run(&u.y);
            }
            Item::Latch(ops) => {
                for op in ops {
                    if let Op::WaitUs(t) = op {
                        w.us += *t as f64;
                    }
                }
            }
            Item::Spindle { wait_ms, .. } => w.us += f64::from(*wait_ms) * 1000.0,
            Item::Pause(_) => {}
        }
        note(&w, n, &mut violations);
    }
    let mm = |v: [i64; 3]| -> (f64, f64, f64) {
        let f = |i: usize| {
            if per_mm[i] > 0.0 {
                v[i] as f64 / per_mm[i]
            } else {
                0.0
            }
        };
        (f(0), f(1), f(2))
    };
    DryRun {
        min_mm: (0.0, 0.0, 0.0),
        max_mm: mm(w.max),
        end_mm: mm(w.at),
        steps: w.steps,
        swallowed_steps: w.swallowed,
        seconds: w.us / 1.0e6,
        items: items.len(),
        violations,
    }
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
    fn step_bytes_are_read_with_their_direction() {
        assert_eq!(decode(bits::X_POS), (1, 0, 0));
        assert_eq!(decode(bits::X_NEG), (-1, 0, 0));
        assert_eq!(decode(bits::Y_POS), (0, 1, 0));
        assert_eq!(decode(bits::Y_NEG), (0, -1, 0));
        assert_eq!(decode(bits::Z_POS), (0, 0, 1));
        assert_eq!(decode(bits::Z_NEG), (0, 0, -1));
        assert_eq!(
            decode(0x4B),
            (1, 0, 0),
            "a plain +X move with the Y direction bit"
        );
        assert_eq!(decode(0x80), (0, 0, 0), "a latch byte moves nothing");
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
