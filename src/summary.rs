//! The summary a person sees before a job starts on a machine that moves: what the job does, how long it takes, how far each axis goes
//! (from the dry run of the exact plan) and what looks wrong in the file (the sanity checks of the units and the scale).
//!
//! The summary itself lives in EH3D-COMMON (`eh3d-summary`), shared with the other programs; this file gives it the thresholds of the tuning
//! of the machine protocol and the dry run of a plan of that protocol.

use crate::config::MachineSettings;
use crate::dryrun;
use crate::i18n::Lang;
use crate::protocol::job::Item;
use crate::protocol::motion::Geometry;
use crate::protocol::tuning::Tuning;
pub use eh3d_summary::summary::{bounds_of, clock, course_warnings, Limits, Summary, Warning};

/// The thresholds of the checks, as the tuning of the machine sets them.
pub fn limits(tuning: &Tuning) -> Limits {
    Limits {
        tiny_mm: tuning.sanity_tiny_mm,
        far_mm: tuning.sanity_far_mm,
        margin_mm: tuning.dry_run_margin_mm,
        reject: tuning.dry_run_reject,
    }
}

/// The sanity checks of a loaded work: `bounds_mm` is (min x, min y, max x, max y) of the holes or paths, `count` how many there are.
pub fn check_work(
    bounds_mm: Option<[f64; 4]>,
    count: usize,
    settings: Option<&MachineSettings>,
    tuning: &Tuning,
) -> Vec<Warning> {
    eh3d_summary::summary::check_work(bounds_mm, count, settings, &limits(tuning))
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
    eh3d_summary::summary::load_warning(points_mm, count, settings, &limits(tuning), lang)
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
    eh3d_summary::summary::build(
        settings,
        &limits(tuning),
        points_mm,
        count,
        tools,
        |course, margin| dryrun::run(items, geometry, course, margin),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::bits;
    use crate::protocol::Step;

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

    #[test]
    fn the_limits_are_the_ones_of_the_tuning() {
        let t = Tuning {
            sanity_tiny_mm: 1.5,
            sanity_far_mm: 99.0,
            dry_run_margin_mm: 0.25,
            dry_run_reject: false,
            ..Tuning::default()
        };
        let l = limits(&t);
        assert_eq!(
            (l.tiny_mm, l.far_mm, l.margin_mm, l.reject),
            (1.5, 99.0, 0.25, false)
        );
    }

    #[test]
    fn the_thresholds_of_the_tuning_decide_the_warnings() {
        let t = Tuning {
            sanity_tiny_mm: 0.0,
            ..Tuning::default()
        };
        assert!(check_work(Some([1.0, 1.0, 2.0, 2.0]), 5, None, &t).is_empty());
        let w = check_work(Some([10.0, 10.0, 12.0, 11.0]), 20, None, &Tuning::default());
        assert!(matches!(w.as_slice(), [Warning::Tiny { .. }]), "{w:?}");
        assert!(load_warning(
            [(5.0, 5.0), (80.0, 60.0)].into_iter(),
            20,
            None,
            &Tuning::default(),
            Lang::En
        )
        .is_none());
    }

    #[test]
    fn a_plan_past_the_course_is_refused_in_the_language_of_the_program() {
        let m = settings();
        let g = Geometry::default();
        let n = ((m.clip_mm.0 + 30.0) * g.steps_per_mm_xy * g.correction.0) as usize;
        let items = vec![Item::Steps(vec![
            Step {
                byte: bits::X_POS,
                delay_us: 500.0
            };
            n
        ])];
        let t = Tuning::default();
        let s = build(
            &items,
            &g,
            Some(&m),
            &t,
            [(1.0, 1.0), (50.0, 50.0)].into_iter(),
            2,
            1,
        );
        assert!(!s.within_course());
        let why = s.refusal(&limits(&t), Lang::Es).expect("refused");
        assert!(why.contains('X'), "{why}");
        assert_ne!(Some(why), s.refusal(&limits(&t), Lang::En));
        let warn = Tuning {
            dry_run_reject: false,
            ..t
        };
        assert_eq!(s.refusal(&limits(&warn), Lang::En), None);
    }
}
