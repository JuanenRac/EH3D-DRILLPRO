//! Reference values captured from the ORIGINAL program running in an emulator, with the machine
//! configuration of the capture: correction coefficients X 0.9992, Y 0.9998, Z 0.8988.
//! Each assertion states what was seen on the serial line.

use eh3d_drillpro_protocol::latches::{Latches, Op};
use eh3d_drillpro_protocol::motion::{rapid_xy, Geometry};
use eh3d_drillpro_protocol::{plan_line, Step};

fn geometry() -> Geometry {
    Geometry::with_correction(0.9992, 0.9998, 0.8988)
}

fn runs(steps: &[Step]) -> Vec<(u8, usize)> {
    let mut v: Vec<(u8, usize)> = Vec::new();
    for s in steps {
        match v.last_mut() {
            Some((b, n)) if *b == s.byte => *n += 1,
            _ => v.push((s.byte, 1)),
        }
    }
    v
}

fn sent(ops: &[Op]) -> Vec<u8> {
    ops.iter()
        .filter_map(|o| if let Op::Send(b) = o { Some(*b) } else { None })
        .collect()
}

#[test]
fn first_move_to_the_first_hole_is_1098_diagonal_steps() {
    // Hole at 1000 mil + 2.5 mm offset = 27.9 mm on both axes: original sent 0x4F x 1098.
    let steps = rapid_xy((0.0, 0.0), (27.9, 27.9), &geometry());
    assert_eq!(runs(&steps), vec![(0x4F, 1098)]);
}

#[test]
fn pure_x_move_between_holes_is_0x4b() {
    // 1000 mil to the right: original sent 0x4B x 999 (Y direction bit set, Y not stepping).
    let steps = rapid_xy((27.9, 27.9), (53.3, 27.9), &geometry());
    assert_eq!(runs(&steps), vec![(0x4B, 999)]);
}

#[test]
fn pure_y_move_between_holes_is_0x4e() {
    // 1000 mil up: original sent 0x4E x 1000 (X direction bit set, X not stepping).
    let steps = rapid_xy((53.3, 27.9), (53.3, 53.3), &geometry());
    assert_eq!(runs(&steps), vec![(0x4E, 1000)]);
}

#[test]
fn negative_moves_use_the_same_rules() {
    // Seen in a routing job: pure -X is 0x09 and pure -Y is 0x06; the homing diagonal is 0x05.
    let g = Geometry::default();
    assert_eq!(
        runs(&plan_line(-100, 0, 500.0, 500.0, 0)),
        vec![(0x09, 100)]
    );
    assert_eq!(
        runs(&plan_line(0, -100, 500.0, 500.0, 0)),
        vec![(0x06, 100)]
    );
    assert_eq!(
        runs(&plan_line(-100, -100, 500.0, 500.0, 0)),
        vec![(0x05, 100)]
    );
    let _ = g;
}

#[test]
fn routing_a_300_by_200_mm_rectangle_matches_the_original() {
    // MEASURE.PLT: the original sent 0x4B x 11802 and 0x4E x 7873. Positions include the 2.5 mm
    // offset and the rounding happens on absolute positions, not on the distance.
    let g = geometry();
    let x = rapid_xy((2.5, 2.5), (302.5, 2.5), &g);
    let y = rapid_xy((302.5, 2.5), (302.5, 202.5), &g);
    assert_eq!(runs(&x), vec![(0x4B, 11802)]);
    assert_eq!(runs(&y), vec![(0x4E, 7873)]);
}

#[test]
fn z_homing_distance_is_the_maximum_depth_in_steps() {
    // 37.30 mm of maximum depth became 1320 steps of 0x30.
    assert_eq!(geometry().z_steps(37.30), 1320);
}

#[test]
fn spindle_start_and_stop_bytes_match_the_original() {
    // Job start sent d0 (drivers on); the spindle then sent f8, 90, 9f; the stop sent d2, 80, d0.
    let drivers_on = Latches::default().drivers_on().next;
    assert_eq!(drivers_on.b, 0xD0);
    let on = drivers_on.spindle_on(15, 1_680_000);
    assert_eq!(sent(&on.ops), vec![0xF8, 0x90, 0x9F]);
    let off = on.next.spindle_off(1_000_000);
    assert_eq!(sent(&off.ops), vec![0xD2, 0x80, 0xD0]);
}

#[test]
fn spindle_levels_match_the_captured_speeds() {
    use eh3d_drillpro_protocol::latches::spindle_level;
    // Captured: 60000 rpm -> 9F, 30000 -> 90, 36000 -> 93, 42000 -> 96, 54000 -> 9C.
    assert_eq!(spindle_level(60_000), 15);
    assert_eq!(spindle_level(30_000), 0);
    assert_eq!(spindle_level(36_000), 3);
    assert_eq!(spindle_level(42_000), 6);
    assert_eq!(spindle_level(54_000), 12);
    assert_eq!(spindle_level(10_000), 0);
    assert_eq!(spindle_level(90_000), 15);
    for (rpm, byte) in [
        (36_000, 0x93u8),
        (42_000, 0x96),
        (54_000, 0x9C),
        (60_000, 0x9F),
    ] {
        let on = Latches::default().spindle_on(spindle_level(rpm), 0);
        assert_eq!(*sent(&on.ops).last().unwrap(), byte);
    }
}

#[test]
fn z_levels_match_the_captured_configurations() {
    use eh3d_drillpro_protocol::motion::{z_levels, ZParams};
    let g = geometry();
    let base = ZParams {
        max_depth_mm: 37.30,
        base_mm: 12.0,
        board_mm: 1.0,
        distance_tool_board_mm: 16.0,
        extra_stroke_mm: 0.0,
    };
    let l = z_levels(&base, &g);
    assert_eq!(
        (l.stroke_steps, l.end_steps, l.travel_steps),
        (601, 912, 311)
    );
    let board = z_levels(
        &ZParams {
            board_mm: 2.5,
            ..base
        },
        &g,
    );
    assert_eq!((board.stroke_steps, board.end_steps), (654, 912));
    let sheet = z_levels(
        &ZParams {
            base_mm: 8.0,
            ..base
        },
        &g,
    );
    assert_eq!((sheet.stroke_steps, sheet.end_steps), (601, 1054));
    let extra = z_levels(
        &ZParams {
            extra_stroke_mm: 0.5,
            ..base
        },
        &g,
    );
    assert_eq!((extra.stroke_steps, extra.end_steps), (619, 930));
    let dist = z_levels(
        &ZParams {
            distance_tool_board_mm: 10.0,
            ..base
        },
        &g,
    );
    assert_eq!(dist.stroke_steps, 389);
    assert!((912..=913).contains(&dist.end_steps));
}

#[test]
fn minor_axis_steps_are_spread_like_the_original() {
    // Segments of a routing job captured from the original: the first bytes where the minor axis
    // does NOT step (1-based), for -639/+492 and -148/-99.
    let skips = |steps: &[Step], minor_bit: u8| -> Vec<usize> {
        steps
            .iter()
            .enumerate()
            .filter(|(_, s)| s.byte & minor_bit == 0)
            .map(|(i, _)| i + 1)
            .take(6)
            .collect()
    };
    let a = plan_line(-639, 492, 500.0, 500.0, 0);
    assert_eq!(skips(&a, 0x04), vec![3, 7, 11, 16, 20, 24]);
    let b = plan_line(-148, -99, 500.0, 500.0, 0);
    assert_eq!(skips(&b, 0x04), vec![2, 5, 8, 11, 14, 17]);
    let c = plan_line(787, 197, 500.0, 500.0, 0);
    assert_eq!(skips(&c, 0x04), vec![1, 3, 4, 5, 7, 8]);
}
