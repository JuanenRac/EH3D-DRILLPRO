//! The summary a person sees before a job starts on a machine that moves: what the job does, how long it takes, how far each axis goes
//! (from the dry run of the exact plan) and what looks wrong in the file (the sanity checks of the units and the scale).
//!
//! Shared by DRILLPRO, ROUTEPRO, the interface of both and the EH3D server, so every front end says the same in every language.

use crate::config::MachineSettings;
use crate::dryrun::{Axis, Course, DryRun};
use crate::i18n::Lang;
use crate::protocol::job::Item;
use crate::protocol::motion::Geometry;
use crate::protocol::tuning::Tuning;
use crate::texts::{tr, Text};

/// What looks wrong in the work that was loaded.
#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    /// The whole work is only this small: the unit of the file is probably not the one the machine file says.
    Tiny { width_mm: f64, height_mm: f64 },
    /// The work is bigger than the table.
    TooBig {
        width_mm: f64,
        height_mm: f64,
        table_x_mm: f64,
        table_y_mm: f64,
    },
    /// The work starts this far from the corner of the table.
    Far { from_mm: f64 },
    /// The machine file stretches the work (its scale is not 1).
    Scale { x: f64, y: f64 },
    /// The exact plan takes an axis past its course.
    OutOfCourse { axis: Axis, excess_mm: f64 },
}

/// Replaces the `{}` of a text, one by one, with the values.
fn fill(template: &str, values: &[String]) -> String {
    let mut out = String::new();
    let mut rest = template;
    let mut i = 0;
    while let Some(p) = rest.find("{}") {
        out.push_str(&rest[..p]);
        match values.get(i) {
            Some(v) => out.push_str(v),
            None => out.push_str("{}"),
        }
        i += 1;
        rest = &rest[p + 2..];
    }
    out.push_str(rest);
    out
}

fn mm(v: f64) -> String {
    format!("{v:.1}")
}

impl Warning {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            Warning::Tiny {
                width_mm,
                height_mm,
            } => fill(tr(lang, Text::WarnTiny), &[mm(*width_mm), mm(*height_mm)]),
            Warning::TooBig {
                width_mm,
                height_mm,
                table_x_mm,
                table_y_mm,
            } => fill(
                tr(lang, Text::WarnBig),
                &[
                    mm(*width_mm),
                    mm(*height_mm),
                    mm(*table_x_mm),
                    mm(*table_y_mm),
                ],
            ),
            Warning::Far { from_mm } => fill(tr(lang, Text::WarnFar), &[mm(*from_mm)]),
            Warning::Scale { x, y } => {
                fill(tr(lang, Text::WarnScale), &[format!("{x}"), format!("{y}")])
            }
            Warning::OutOfCourse { axis, excess_mm } => fill(
                tr(lang, Text::DryOut),
                &[axis.letter().to_string(), mm(*excess_mm)],
            ),
        }
    }
}

/// The sanity checks of a loaded work: `bounds_mm` is (min x, min y, max x, max y) of the holes or paths, `count` how many there are.
pub fn check_work(
    bounds_mm: Option<[f64; 4]>,
    count: usize,
    settings: Option<&MachineSettings>,
    tuning: &Tuning,
) -> Vec<Warning> {
    let mut out = Vec::new();
    if let Some(m) = settings {
        if (m.scale.0 - 1.0).abs() > 1e-9 || (m.scale.1 - 1.0).abs() > 1e-9 {
            out.push(Warning::Scale {
                x: m.scale.0,
                y: m.scale.1,
            });
        }
    }
    let Some([x0, y0, x1, y1]) = bounds_mm else {
        return out;
    };
    let (w, h) = (x1 - x0, y1 - y0);
    if count >= 2 && w < tuning.sanity_tiny_mm && h < tuning.sanity_tiny_mm {
        out.push(Warning::Tiny {
            width_mm: w,
            height_mm: h,
        });
    }
    if let Some(m) = settings {
        if w > m.clip_mm.0 || h > m.clip_mm.1 {
            out.push(Warning::TooBig {
                width_mm: w,
                height_mm: h,
                table_x_mm: m.clip_mm.0,
                table_y_mm: m.clip_mm.1,
            });
        }
    }
    let from = x0.max(y0);
    if from > tuning.sanity_far_mm {
        out.push(Warning::Far { from_mm: from });
    }
    out
}

/// The axes that the dry run takes past their course, as warnings (one per axis, the biggest excess).
pub fn course_warnings(dry: &DryRun) -> Vec<Warning> {
    let mut out: Vec<Warning> = Vec::new();
    for v in &dry.violations {
        match out
            .iter_mut()
            .find(|w| matches!(w, Warning::OutOfCourse { axis, .. } if *axis == v.axis))
        {
            Some(Warning::OutOfCourse { excess_mm, .. }) => {
                *excess_mm = excess_mm.max(v.excess_mm())
            }
            _ => out.push(Warning::OutOfCourse {
                axis: v.axis,
                excess_mm: v.excess_mm(),
            }),
        }
    }
    out
}

/// Everything the person is shown before the start.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    /// Holes (drilling) or paths (routing) the job does, and the tools it uses.
    pub work_count: usize,
    pub tools: usize,
    pub bounds_mm: Option<[f64; 4]>,
    pub dry: DryRun,
    pub course: Course,
    pub warnings: Vec<Warning>,
}

impl Summary {
    pub fn new(
        work_count: usize,
        tools: usize,
        bounds_mm: Option<[f64; 4]>,
        dry: DryRun,
        course: Course,
        mut warnings: Vec<Warning>,
    ) -> Summary {
        warnings.extend(course_warnings(&dry));
        Summary {
            work_count,
            tools,
            bounds_mm,
            dry,
            course,
            warnings,
        }
    }

    /// Whether the exact plan stays inside the course of the machine.
    pub fn within_course(&self) -> bool {
        self.dry.within_course()
    }

    /// The lines of the summary, ready to show: the facts, the reach of the axes, the verdict of the dry run and the warnings.
    pub fn lines(&self, lang: Lang) -> Vec<String> {
        let mut out = vec![tr(lang, Text::SumTitle).to_string()];
        out.push(format!("{}: {}", tr(lang, Text::LblHoles), self.work_count));
        out.push(format!("{}: {}", tr(lang, Text::LblToolsUsed), self.tools));
        if let Some([x0, y0, x1, y1]) = self.bounds_mm {
            out.push(format!(
                "{}: {} x {}",
                tr(lang, Text::LblSize),
                mm(x1 - x0),
                mm(y1 - y0)
            ));
        }
        out.push(format!(
            "{}: {}",
            tr(lang, Text::SumTime),
            clock(self.dry.seconds)
        ));
        let (a, b) = (self.dry.min_mm, self.dry.max_mm);
        out.push(format!(
            "{}: X {} .. {}   Y {} .. {}   Z {} .. {}",
            tr(lang, Text::SumReach),
            mm(a.0),
            mm(b.0),
            mm(a.1),
            mm(b.1),
            mm(a.2),
            mm(b.2)
        ));
        if self.within_course() {
            out.push(tr(lang, Text::DryOk).to_string());
        }
        for w in &self.warnings {
            out.push(w.message(lang));
        }
        out
    }
}

/// (min x, min y, max x, max y) of some points, in mm.
pub fn bounds_of(points_mm: impl Iterator<Item = (f64, f64)>) -> Option<[f64; 4]> {
    let mut bounds: Option<[f64; 4]> = None;
    for (x, y) in points_mm {
        bounds = Some(match bounds {
            None => [x, y, x, y],
            Some([x0, y0, x1, y1]) => [x0.min(x), y0.min(y), x1.max(x), y1.max(y)],
        });
    }
    bounds
}

/// What looks wrong in a work that has just been loaded, as one line in the language (nothing when it looks right): the sanity checks
/// shown on the main screen before any plan exists.
pub fn load_warning(
    points_mm: impl Iterator<Item = (f64, f64)>,
    count: usize,
    settings: Option<&MachineSettings>,
    tuning: &Tuning,
    lang: Lang,
) -> Option<String> {
    let found = check_work(bounds_of(points_mm), count, settings, tuning);
    (!found.is_empty()).then(|| {
        found
            .iter()
            .map(|w| w.message(lang))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

/// The summary of a planned job for a machine: the dry run of its exact `items` against the course of the machine file, and the sanity checks of
/// the work (`points_mm`: where the holes are, or the points of the paths, in mm from the zero of the file; `tools` how many tools it uses).
pub fn build(
    items: &[Item],
    geometry: &Geometry,
    settings: Option<&MachineSettings>,
    tuning: &Tuning,
    points_mm: impl Iterator<Item = (f64, f64)>,
    count: usize,
    tools: usize,
) -> Summary {
    let bounds = bounds_of(points_mm);
    let course = settings.map_or(
        Course {
            x_mm: f64::INFINITY,
            y_mm: f64::INFINITY,
            z_mm: f64::INFINITY,
        },
        Course::from_settings,
    );
    let dry = crate::dryrun::run(items, geometry, &course, tuning.dry_run_margin_mm);
    let warnings = check_work(bounds, count, settings, tuning);
    Summary::new(count, tools, bounds, dry, course, warnings)
}

impl Summary {
    /// The reason to refuse the job, when the exact plan goes past the course of an axis and the tuning says `dry_run = reject`.
    pub fn refusal(&self, tuning: &Tuning, lang: Lang) -> Option<String> {
        if !tuning.dry_run_reject {
            return None;
        }
        course_warnings(&self.dry).first().map(|w| w.message(lang))
    }
}

/// `1 h 02 min`, `3 min 05 s` or `42 s`.
pub fn clock(seconds: f64) -> String {
    let s = seconds.max(0.0).round() as u64;
    if s >= 3600 {
        format!("{} h {:02} min", s / 3600, s % 3600 / 60)
    } else if s >= 60 {
        format!("{} min {:02} s", s / 60, s % 60)
    } else {
        format!("{s} s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dryrun::Violation;

    fn settings() -> MachineSettings {
        // the file is written in a single-byte code page: each byte is one character
        let text: String = include_bytes!("../config/DRILLPRO.DCF")
            .iter()
            .map(|&c| c as char)
            .collect();
        crate::config::parse(&text)
            .and_then(|c| c.settings())
            .expect("the sample machine file reads")
    }

    fn dry(violations: Vec<Violation>) -> DryRun {
        DryRun {
            min_mm: (0.0, 0.0, 0.0),
            max_mm: (10.0, 20.0, 1.0),
            end_mm: (0.0, 0.0, 0.0),
            steps: 10,
            swallowed_steps: 0,
            seconds: 125.0,
            items: 3,
            violations,
        }
    }

    #[test]
    fn fill_replaces_in_order_and_keeps_missing() {
        assert_eq!(fill("a {} b {}", &["1".into(), "2".into()]), "a 1 b 2");
        assert_eq!(fill("a {} b {}", &["1".into()]), "a 1 b {}");
    }

    #[test]
    fn the_clock_reads_hours_minutes_and_seconds() {
        assert_eq!(clock(42.4), "42 s");
        assert_eq!(clock(185.0), "3 min 05 s");
        assert_eq!(clock(3720.0), "1 h 02 min");
    }

    #[test]
    fn a_small_work_looks_like_a_wrong_unit() {
        let t = Tuning::default();
        let w = check_work(Some([10.0, 10.0, 12.0, 11.0]), 20, None, &t);
        assert!(matches!(w.as_slice(), [Warning::Tiny { .. }]), "{w:?}");
        // one hole alone is not small, it is a point
        assert!(check_work(Some([10.0, 10.0, 10.0, 10.0]), 1, None, &t).is_empty());
        // a normal board is fine
        assert!(check_work(Some([5.0, 5.0, 80.0, 60.0]), 20, None, &t).is_empty());
    }

    #[test]
    fn a_work_far_from_the_corner_or_bigger_than_the_table_is_said() {
        let t = Tuning::default();
        let w = check_work(Some([400.0, 20.0, 450.0, 60.0]), 20, None, &t);
        assert!(matches!(w.as_slice(), [Warning::Far { .. }]), "{w:?}");
        let m = settings();
        let w = check_work(Some([0.0, 0.0, m.clip_mm.0 + 10.0, 10.0]), 20, Some(&m), &t);
        assert!(
            w.iter().any(|x| matches!(x, Warning::TooBig { .. })),
            "{w:?}"
        );
    }

    #[test]
    fn a_scale_that_is_not_one_is_said() {
        let mut m = settings();
        m.scale = (1.0, 1.0);
        let t = Tuning::default();
        assert!(check_work(None, 0, Some(&m), &t).is_empty());
        m.scale = (1.0, 0.5);
        let w = check_work(None, 0, Some(&m), &t);
        assert_eq!(w, vec![Warning::Scale { x: 1.0, y: 0.5 }]);
    }

    #[test]
    fn the_thresholds_come_from_the_tuning() {
        let t = Tuning {
            sanity_tiny_mm: 0.0,
            ..Tuning::default()
        };
        assert!(check_work(Some([1.0, 1.0, 2.0, 2.0]), 5, None, &t).is_empty());
        let t = Tuning {
            sanity_far_mm: 10.0,
            ..t
        };
        assert_eq!(
            check_work(Some([20.0, 1.0, 40.0, 5.0]), 5, None, &t).len(),
            1
        );
    }

    #[test]
    fn course_warnings_keep_the_biggest_excess_per_axis() {
        let v = |axis, at_mm, limit_mm| Violation {
            item: 0,
            axis,
            at_mm,
            limit_mm,
        };
        let d = dry(vec![
            v(Axis::X, 105.0, 100.0),
            v(Axis::X, 103.0, 100.0),
            v(Axis::Z, 12.0, 10.0),
        ]);
        let w = course_warnings(&d);
        assert_eq!(
            w,
            vec![
                Warning::OutOfCourse {
                    axis: Axis::X,
                    excess_mm: 5.0
                },
                Warning::OutOfCourse {
                    axis: Axis::Z,
                    excess_mm: 2.0
                },
            ]
        );
    }

    #[test]
    fn the_summary_lists_the_facts_and_the_verdict_in_a_language() {
        let course = Course {
            x_mm: 100.0,
            y_mm: 100.0,
            z_mm: 20.0,
        };
        let s = Summary::new(
            12,
            3,
            Some([1.0, 2.0, 11.0, 22.0]),
            dry(vec![]),
            course,
            vec![],
        );
        assert!(s.within_course());
        let en = s.lines(Lang::En);
        assert!(en.iter().any(|l| l.contains("2 min 05 s")), "{en:?}");
        assert!(en.iter().any(|l| l.contains("10.0 x 20.0")), "{en:?}");
        assert!(en.contains(&tr(Lang::En, Text::DryOk).to_string()));
        let es = s.lines(Lang::Es);
        assert_ne!(en, es);
        let bad = Summary::new(
            12,
            3,
            None,
            dry(vec![Violation {
                item: 1,
                axis: Axis::Y,
                at_mm: 104.0,
                limit_mm: 100.0,
            }]),
            course,
            vec![],
        );
        assert!(!bad.within_course());
        let l = bad.lines(Lang::En);
        assert!(!l.contains(&tr(Lang::En, Text::DryOk).to_string()));
        assert!(
            l.iter().any(|x| x.contains('Y') && x.contains("4.0")),
            "{l:?}"
        );
    }
}
