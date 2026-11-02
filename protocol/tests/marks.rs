//! The plan says where each unit of work ends, so a run does not have to guess it from the shape of the steps.

use eh3d_drillpro_protocol::bits;
use eh3d_drillpro_protocol::job::*;
use eh3d_drillpro_protocol::motion::Geometry;
use eh3d_drillpro_protocol::Step;

fn machine() -> JobMachine {
    JobMachine {
        geometry: Geometry::default(),
        offset_mm: (2.5, 2.5),
        max_depth_mm: 37.3,
        base_mm: 12.0,
        board_mm: 1.0,
        distance_tool_board_mm: 16.0,
        xy_search_steps: 26613,
    }
}

fn tools() -> Vec<JobTool> {
    [1u8, 2]
        .iter()
        .map(|&number| JobTool {
            number,
            extra_stroke_mm: 0.0,
            feed_mm_min: 1000,
            spindle_rpm: 60_000,
        })
        .collect()
}

fn holes() -> Vec<JobHole> {
    [
        (1u8, 5.0, 5.0),
        (1, 10.0, 5.0),
        (2, 10.0, 10.0),
        (1, 5.0, 10.0),
    ]
    .iter()
    .map(|&(tool, x_mm, y_mm)| JobHole { tool, x_mm, y_mm })
    .collect()
}

#[test]
fn every_hole_has_a_mark_on_the_item_that_lifts_the_drill_out() {
    let (items, marks) = plan_drill_job_marked(&machine(), &tools(), &holes(), &[], None);
    assert_eq!(marks.len(), 4, "one mark for each hole");
    assert!(
        marks.windows(2).all(|w| w[0] < w[1]),
        "in the order of the plan"
    );
    for &m in &marks {
        match &items[m] {
            Item::Steps(s) => assert_eq!(s[0].byte, bits::Z_NEG, "the drill comes up"),
            other => panic!("the mark {m} is on {other:?}"),
        }
    }
}

#[test]
fn the_plan_is_the_same_with_or_without_the_marks() {
    let marked = plan_drill_job_marked(&machine(), &tools(), &holes(), &[], None).0;
    assert_eq!(marked, plan_drill_job(&machine(), &tools(), &holes()));
}

#[test]
fn a_levelled_job_has_the_same_marks_as_a_flat_one() {
    let lifts = [0.1, -0.1, 0.0, 0.05];
    let flat = plan_drill_job_marked(&machine(), &tools(), &holes(), &[], None).1;
    let (items, levelled) = plan_drill_job_marked(&machine(), &tools(), &holes(), &lifts, None);
    assert_eq!(levelled.len(), flat.len());
    assert!(levelled.iter().all(|&m| m < items.len()));
}

#[test]
fn the_builder_marks_the_item_pushed_last() {
    let m = machine();
    let mut b = JobBuilder::new(&m);
    let step = |byte| Step {
        byte,
        delay_us: 500.0,
    };
    b.steps(vec![step(bits::X_POS); 10], (10, 0));
    b.mark_done();
    b.steps(vec![step(bits::X_POS); 10], (20, 0));
    b.mark_done();
    let (items, marks) = b.finish_marked();
    assert_eq!(marks.len(), 2);
    assert_eq!(marks[1], marks[0] + 1);
    assert!(matches!(&items[marks[1]], Item::Steps(s) if s.len() == 10));
}

#[test]
fn finish_and_finish_marked_make_the_same_plan() {
    let m = machine();
    let build = || {
        let mut b = JobBuilder::new(&m);
        b.steps(
            vec![
                Step {
                    byte: bits::X_POS,
                    delay_us: 500.0
                };
                5
            ],
            (5, 0),
        );
        b.mark_done();
        b
    };
    assert_eq!(build().finish(), build().finish_marked().0);
}
