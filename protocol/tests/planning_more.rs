//! The planning of moves in steps and millimetres, the rounding of the original and the values of the tuning, checked from the edges: what a plan adds up to, what a bad value of the
//! machine file does, and what a stream keeps of the steps it carries.

use eh3d_drillpro_protocol::motion::{
    drill_stroke, duration_s, feed_xy, rapid_xy, round_half_even, z_levels, z_move, Geometry,
    ZParams,
};
use eh3d_drillpro_protocol::tuning::Tuning;
use eh3d_drillpro_protocol::{
    bits, paced_stream, paced_stream_with, plan_line, Simulator, Step, Transport, BYTE_TIME_US,
};

/// Numbers that look random and are always the same.
struct Lcg(u64);
impl Lcg {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
    fn signed(&mut self, n: i32) -> i32 {
        self.below(2 * n as u64 + 1) as i32 - n
    }
}

fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

#[test]
fn a_planned_line_ends_exactly_where_it_was_asked_to_whatever_the_slope() {
    let mut r = Lcg(1);
    for _ in 0..300 {
        let (dx, dy) = (r.signed(300), r.signed(300));
        let plan = plan_line(dx, dy, 500.0, 4000.0, 40);
        assert_eq!(
            plan.len(),
            dx.unsigned_abs().max(dy.unsigned_abs()) as usize
        );
        let mut sim = Simulator::default();
        for s in &plan {
            sim.write_byte(s.byte).unwrap();
        }
        assert_eq!(
            (sim.x_steps, sim.y_steps, sim.z_steps),
            (i64::from(dx), i64::from(dy), 0),
            "{dx} {dy}"
        );
    }
}

#[test]
fn a_move_of_no_steps_is_an_empty_plan_and_its_duration_is_zero() {
    assert!(plan_line(0, 0, 500.0, 4000.0, 40).is_empty());
    assert_eq!(duration_s(&[]), 0.0);
    let g = Geometry::default();
    assert!(rapid_xy((3.0, 4.0), (3.0, 4.0), &g).is_empty());
    // a move that is less than half a step on both axes does not exist either
    assert!(rapid_xy((0.0, 0.0), (0.01, 0.01), &g).is_empty());
}

#[test]
fn the_speed_of_a_plan_stays_between_the_start_and_the_top_and_is_the_same_going_and_coming() {
    for (dx, dy, ramp) in [
        (500, 0, 40),
        (7, 3, 40),
        (100, -250, 80),
        (1, 0, 40),
        (2, 0, 40),
        (90, 90, 1000),
    ] {
        let plan = plan_line(dx, dy, 500.0, 4000.0, ramp);
        for s in &plan {
            assert!(
                s.delay_us >= 1e6 / 4000.0 - 1e-9 && s.delay_us <= 1e6 / 500.0 + 1e-9,
                "{dx} {dy}: {}",
                s.delay_us
            );
        }
        let back: Vec<f64> = plan.iter().rev().map(|s| s.delay_us).collect();
        let go: Vec<f64> = plan.iter().map(|s| s.delay_us).collect();
        for (a, b) in go.iter().zip(&back) {
            assert!(near(*a, *b, 1e-9), "the ramp is the same at both ends");
        }
    }
    // the first and the last step are at the start speed, the middle of a long move at the top speed
    let plan = plan_line(1000, 0, 500.0, 4000.0, 40);
    assert!(near(plan[0].delay_us, 2000.0, 1e-9) && near(plan[999].delay_us, 2000.0, 1e-9));
    assert!(near(plan[500].delay_us, 250.0, 1e-9));
    // with no ramp it is all at the top
    assert!(plan_line(50, 20, 500.0, 4000.0, 0)
        .iter()
        .all(|s| near(s.delay_us, 250.0, 1e-9)));
}

#[test]
fn a_longer_ramp_makes_the_same_move_take_longer() {
    let t = |ramp| duration_s(&plan_line(2000, 500, 500.0, 4000.0, ramp));
    assert!(
        t(0) < t(40) && t(40) < t(400) && t(400) < t(900),
        "{} {} {}",
        t(0),
        t(40),
        t(400)
    );
    // and a ramp longer than the move is no worse than the slowest one possible
    assert!(t(100_000) <= 2000.0 / 500.0 + 1e-9);
}

#[test]
fn the_steps_of_the_axis_that_does_not_lead_are_spread_evenly_and_ties_go_to_x() {
    let plan = plan_line(100, 25, 1000.0, 1000.0, 0);
    let y_steps = plan.iter().filter(|s| s.byte & 0x04 != 0).count();
    assert_eq!(y_steps, 25);
    // one in every four bytes, never two in a row
    let mut gap = 0;
    for s in &plan {
        if s.byte & 0x04 != 0 {
            assert!(gap <= 4, "{gap}");
            gap = 0;
        } else {
            gap += 1;
        }
    }
    // a diagonal steps both axes on every byte
    assert!(plan_line(10, 10, 1000.0, 1000.0, 0)
        .iter()
        .all(|s| s.byte & 0x05 == 0x05));
    // a pure move to the left is one byte, repeated, and carries the direction of the other axis
    assert!(plan_line(-5, 0, 1000.0, 1000.0, 0)
        .iter()
        .all(|s| s.byte == (bits::X_NEG | bits::Y_DIR)));
    assert!(plan_line(0, -5, 1000.0, 1000.0, 0)
        .iter()
        .all(|s| s.byte == (bits::Y_NEG | bits::X_DIR)));
}

#[test]
fn the_rounding_of_the_original_sends_a_half_to_the_even_number() {
    for (v, want) in [
        (0.5, 0.0),
        (1.5, 2.0),
        (2.5, 2.0),
        (3.5, 4.0),
        (1498.5, 1498.0),
        (1499.5, 1500.0),
        (-0.5, 0.0),
        (-1.5, -2.0),
        (-2.5, -2.0),
        (2.4, 2.0),
        (2.6, 3.0),
        (-2.6, -3.0),
        (0.0, 0.0),
        (7.0, 7.0),
    ] {
        assert_eq!(round_half_even(v), want, "{v}");
    }
}

#[test]
fn millimetres_become_steps_with_the_correction_of_each_axis() {
    let g = Geometry::default();
    assert_eq!(g.x_steps(0.0), 0);
    assert_eq!(g.x_steps(25.4), 1000);
    assert_eq!(g.x_steps(-25.4), -1000);
    assert_eq!(g.y_steps(-0.0127), 0, "half a step to the even one");
    let c = Geometry::with_correction(1.0, 1.1, 0.9);
    assert_eq!(c.y_steps(25.4), 1100);
    assert_eq!(c.x_steps(25.4), 1000);
    assert_eq!(c.z_steps(25.4), 900);
    // Z has no negative: above the switch is the switch
    assert_eq!(g.z_steps(-5.0), 0);
    assert_eq!(g.z_steps(f64::NAN), 0);
    // the way back from steps is the inverse, to within a step
    for mm in [0.0, 0.5, 3.17, 12.0, 37.3] {
        let back = c.z_mm(c.z_steps(mm));
        assert!(near(back, mm, 1.0 / (39.37 * 0.9)), "{mm} -> {back}");
    }
}

#[test]
fn the_speeds_of_the_machine_file_never_exceed_the_top_and_never_stop_the_motor() {
    let g = Geometry::default().with_speeds((600.0, 2000.0, 100.0), (300.0, 900.0, 50.0));
    assert!(near(g.max_hz, 600.0 / 60.0 * 39.3701, 0.01));
    assert_eq!(g.ref_hz, g.max_hz, "the reference speed is held to the top");
    assert!(near(g.start_hz, 100.0 / 60.0 * 39.3701, 0.01));
    assert_eq!(g.z_ref_hz, g.z_max_hz);
    assert!(g.z_start_hz < g.z_max_hz);
    // a feed of zero or less is still a step rate of 1 and never a division by zero
    assert_eq!(g.xy_hz(0.0), 1.0);
    assert_eq!(g.xy_hz(-50.0), 1.0);
    assert_eq!(g.z_hz(0.0), 1.0);
    assert!(g.xy_hz(1e9).is_finite());
}

#[test]
fn a_cap_of_the_motors_lowers_every_speed_and_never_raises_one() {
    let base = Geometry::default();
    let c = base.capped(Some(1000.0), Some(700.0));
    assert!(c.max_hz <= 1000.0 && c.start_hz <= c.max_hz && c.ref_hz <= c.max_hz);
    assert!(c.z_max_hz <= 700.0 && c.z_start_hz <= c.z_max_hz && c.z_ref_hz <= c.z_max_hz);
    let high = base.capped(Some(1e9), Some(1e9));
    assert_eq!(high, base, "a cap above the speeds changes nothing");
    let only_xy = base.capped(Some(800.0), None);
    assert_eq!(only_xy.z_max_hz, base.z_max_hz);
    assert_eq!(base.capped(None, None), base);
    // the cap holds for the plans made from it
    for s in rapid_xy((0.0, 0.0), (50.0, 30.0), &c) {
        assert!(s.delay_us >= 1e6 / 1000.0 - 1e-9);
    }
}

#[test]
fn a_move_at_a_feed_has_one_speed_and_the_top_speed_is_a_ceiling() {
    let g = Geometry::default();
    let slow = feed_xy((0.0, 0.0), (20.0, 10.0), 120.0, &g);
    assert!(!slow.is_empty());
    let d = 1e6 / g.xy_hz(120.0);
    assert!(
        slow.iter().all(|s| near(s.delay_us, d, 1e-6)),
        "a constant feed has no ramp"
    );
    let wild = feed_xy((0.0, 0.0), (20.0, 10.0), 1e7, &g);
    assert!(wild.iter().all(|s| near(s.delay_us, 1e6 / g.max_hz, 1e-6)));
    // the same distance twice as fast takes half the time
    let fast = feed_xy((0.0, 0.0), (20.0, 10.0), 240.0, &g);
    assert!(near(duration_s(&slow) / duration_s(&fast), 2.0, 1e-6));
}

#[test]
fn a_z_stroke_goes_down_and_up_the_same_number_of_steps_and_comes_back_faster() {
    let g = Geometry::default();
    let down = z_move(2.0, true, 120.0, &g);
    let up = z_move(2.0, false, 120.0, &g);
    assert_eq!(down.len(), g.z_steps(2.0) as usize);
    assert!(down.iter().all(|s| s.byte == bits::Z_POS));
    assert!(up.iter().all(|s| s.byte == bits::Z_NEG));
    let stroke = drill_stroke(2.0, 120.0, &g);
    assert_eq!(stroke.len(), 2 * down.len());
    assert!(stroke[..down.len()].iter().all(|s| s.byte == bits::Z_POS));
    assert!(stroke[down.len()..].iter().all(|s| s.byte == bits::Z_NEG));
    assert!(duration_s(&stroke[down.len()..]) < duration_s(&stroke[..down.len()]));
    assert!(z_move(-3.0, true, 120.0, &g).is_empty());
    assert!(z_move(0.0, false, 120.0, &g).is_empty());
}

#[test]
fn the_levels_of_z_are_cut_down_to_whole_steps_and_never_negative() {
    let g = Geometry::default();
    let p = ZParams {
        max_depth_mm: 37.3,
        base_mm: 8.0,
        board_mm: 1.6,
        distance_tool_board_mm: 5.0,
        extra_stroke_mm: 0.0,
    };
    let l = z_levels(&p, &g);
    assert_eq!(l.travel_steps, l.end_steps - l.stroke_steps);
    // a stroke longer than the end of the hole leaves no travel height instead of wrapping around
    let deep = ZParams {
        distance_tool_board_mm: 80.0,
        ..p
    };
    let l = z_levels(&deep, &g);
    assert!(l.stroke_steps > l.end_steps);
    assert_eq!(l.travel_steps, 0);
    // nothing negative gets through
    let odd = ZParams {
        max_depth_mm: -50.0,
        base_mm: 10.0,
        board_mm: -4.0,
        distance_tool_board_mm: -4.0,
        extra_stroke_mm: -1.0,
    };
    let l = z_levels(&odd, &g);
    assert_eq!((l.stroke_steps, l.end_steps, l.travel_steps), (0, 0, 0));
    // the correction of Z scales the levels
    let c = Geometry::with_correction(1.0, 1.0, 2.0);
    let a = z_levels(&p, &g);
    let b = z_levels(&p, &c);
    assert!(b.stroke_steps.abs_diff(2 * a.stroke_steps) <= 1);
    assert!(b.end_steps.abs_diff(2 * a.end_steps) <= 1);
}

#[test]
fn every_value_of_the_machine_file_is_set_by_its_name_and_an_unknown_name_is_left_alone() {
    let mut t = Tuning::default();
    assert_eq!(t.set("x_search_hz", "2400"), Ok(true));
    assert_eq!(t.x_search_hz, 2400.0);
    assert_eq!(t.set("search_margin_percent", "15.5"), Ok(true));
    assert_eq!(t.search_margin_percent, 15.5);
    assert_eq!(t.set("jump_ramp_parts", "3"), Ok(true));
    assert_eq!(t.jump_ramp_parts, 3);
    assert_eq!(t.set("spindle_start_wait_ms", "2500"), Ok(true));
    assert_eq!(t.spindle_start_wait_ms, 2500);
    assert_eq!(t.set("home_watch_chunk_bytes", "900"), Ok(true));
    assert_eq!(t.home_watch_chunk_bytes, 900);
    // a name that is not a value of the tuning is not an error: another part of the file may own it
    let before = t;
    assert_eq!(t.set("baud", "9600"), Ok(false));
    assert_eq!(t.set("", ""), Ok(false));
    assert_eq!(t.set("X_SEARCH_HZ", "100"), Ok(false), "names are exact");
    assert_eq!(t, before);
}

#[test]
fn a_value_outside_its_range_is_refused_with_its_name_and_the_old_value_stays() {
    let mut t = Tuning::default();
    let before = t;
    for (key, value, want) in [
        (
            "x_search_hz",
            "49.9",
            "x_search_hz must be between 50 and 3840",
        ),
        ("x_search_hz", "3840.1", "between 50 and 3840"),
        ("y_search_hz", "fast", "y_search_hz must be a number"),
        ("y_search_hz", "", "must be a number"),
        ("z_search_hz", "nan", "between"),
        ("z_search_hz", "inf", "between"),
        ("search_ramp_steps", "0", "between 1 and 5000"),
        ("search_ramp_steps", "5001", "between 1 and 5000"),
        ("search_margin_percent", "-1", "between 0 and 100"),
        ("jump_start_fraction", "0.19", "between 0.2 and 1"),
        ("jump_ramp_parts", "11", "between 1 and 10"),
        ("spindle_start_wait_ms", "20001", "between 0 and 20000"),
        ("unstick_mm", "51", "between 0 and 50"),
        ("dry_run_margin_mm", "-0.1", "between 0 and 20"),
        ("sanity_far_mm", "2001", "between 0 and 2000"),
        ("z_pacing", "fast", "z_pacing must be bytes or stream"),
        ("spindle", "both", "spindle must be central or node"),
        ("dry_run", "ignore", "dry_run must be reject or warn"),
        ("confirm_start", "yes", "confirm_start must be on or off"),
    ] {
        let e = t.set(key, value).unwrap_err();
        assert!(e.contains(want), "{key}={value}: {e}");
    }
    assert_eq!(t, before);
}

#[test]
fn the_edges_of_every_range_are_inside_and_a_number_is_cut_to_whole_steps() {
    let mut t = Tuning::default();
    for v in ["50", "3840"] {
        assert_eq!(t.set("z_start_hz", v), Ok(true));
    }
    for v in ["1", "5000"] {
        assert_eq!(t.set("z_ramp_steps", v), Ok(true));
    }
    assert_eq!(t.set("z_ramp_steps", "7.9"), Ok(true));
    assert_eq!(t.z_ramp_steps, 7, "a step is whole");
    assert_eq!(t.set("unstick_mm", "0"), Ok(true));
    assert_eq!(t.unstick_mm, 0.0);
    assert_eq!(t.set("home_watch_chunk_bytes", "0"), Ok(true));
    assert_eq!(t.set("home_watch_chunk_bytes", "4000"), Ok(true));
    assert!(t.set("home_watch_chunk_bytes", "4001").is_err());
}

#[test]
fn the_choices_are_read_in_any_case_and_each_one_sets_the_right_switch() {
    let mut t = Tuning::default();
    assert!(t.z_pacing_bytes && t.stream_keep_direction && !t.spindle_node && !t.vacuum_aux);
    assert_eq!(t.set("z_pacing", "STREAM"), Ok(true));
    assert!(!t.z_pacing_bytes);
    assert_eq!(t.set("z_pacing", "Bytes"), Ok(true));
    assert!(t.z_pacing_bytes);
    assert_eq!(t.set("stream_filler", "zero"), Ok(true));
    assert!(!t.stream_keep_direction);
    assert_eq!(t.set("spindle", "node"), Ok(true));
    assert!(t.spindle_node);
    assert_eq!(t.set("spindle", "central"), Ok(true));
    assert!(!t.spindle_node);
    assert_eq!(t.set("vacuum", "aux"), Ok(true));
    assert!(t.vacuum_aux);
    assert_eq!(t.set("dry_run", "warn"), Ok(true));
    assert!(!t.dry_run_reject);
    assert_eq!(t.set("confirm_start", "off"), Ok(true));
    assert!(!t.confirm_start);
    assert_eq!(t.set("unstick_always", "on"), Ok(true));
    assert!(t.unstick_always);
}

#[test]
fn what_the_default_says_is_accepted_by_the_file_that_would_write_it() {
    let d = Tuning::default();
    let mut t = Tuning::default();
    for (key, value) in [
        ("x_search_hz", d.x_search_hz),
        ("y_search_hz", d.y_search_hz),
        ("z_search_hz", d.z_search_hz),
        ("z_start_hz", d.z_start_hz),
        ("jump_peak_hz", d.jump_peak_hz),
        ("search_margin_percent", d.search_margin_percent),
        ("jump_start_fraction", d.jump_start_fraction),
        ("unstick_mm", d.unstick_mm),
        ("dry_run_margin_mm", d.dry_run_margin_mm),
        ("sanity_tiny_mm", d.sanity_tiny_mm),
        ("sanity_far_mm", d.sanity_far_mm),
        ("search_ramp_steps", f64::from(d.search_ramp_steps)),
        ("z_ramp_steps", f64::from(d.z_ramp_steps)),
        ("z_search_steps", f64::from(d.z_search_steps)),
        ("jump_backoff_steps", f64::from(d.jump_backoff_steps)),
        ("jump_approach_steps", f64::from(d.jump_approach_steps)),
        ("z_backoff_steps", f64::from(d.z_backoff_steps)),
        ("z_approach_steps", f64::from(d.z_approach_steps)),
        ("cancel_piece_steps", f64::from(d.cancel_piece_steps)),
        (
            "home_check_piece_steps",
            f64::from(d.home_check_piece_steps),
        ),
        ("travel_ramp_steps", f64::from(d.travel_ramp_steps)),
        (
            "search_hint_margin_steps",
            f64::from(d.search_hint_margin_steps),
        ),
        ("spindle_start_wait_ms", f64::from(d.spindle_start_wait_ms)),
        ("spindle_stop_wait_ms", f64::from(d.spindle_stop_wait_ms)),
        ("jump_ramp_parts", f64::from(d.jump_ramp_parts)),
    ] {
        assert_eq!(t.set(key, &value.to_string()), Ok(true), "{key} = {value}");
    }
    assert_eq!(t, d, "setting the default values changes nothing");
}

#[test]
fn the_wait_for_the_spindle_follows_its_speed_only_when_the_file_says_so() {
    let mut t = Tuning::default();
    assert_eq!(t.spindle_start_wait(24_000), t.spindle_start_wait_ms);
    assert_eq!(t.spindle_start_wait(0), t.spindle_start_wait_ms);
    t.spindle_start_ms_per_10000 = 100;
    assert_eq!(t.spindle_start_wait(24_000), 240);
    assert_eq!(t.spindle_start_wait(10_000), 100);
    assert_eq!(t.spindle_start_wait(0), 0);
    // a huge speed or a huge time per 10000 is held to a minute and does not overflow
    t.spindle_start_ms_per_10000 = u32::MAX;
    assert_eq!(t.spindle_start_wait(u32::MAX), 60_000);
}

fn steps_of(delays_us: &[f64]) -> Vec<Step> {
    delays_us
        .iter()
        .enumerate()
        .map(|(i, d)| Step {
            byte: if i % 2 == 0 { bits::X_POS } else { bits::Z_NEG },
            delay_us: *d,
        })
        .collect()
}

#[test]
fn a_paced_stream_carries_every_step_in_order_and_lasts_as_long_as_the_plan() {
    let mut r = Lcg(9);
    let delays: Vec<f64> = (0..200).map(|_| 300.0 + r.below(3000) as f64).collect();
    let steps = steps_of(&delays);
    let total_us: f64 = delays.iter().sum();
    for keep in [false, true] {
        let s = paced_stream_with(&steps, keep);
        let carried: Vec<u8> = s.iter().copied().filter(|b| b & 0x15 != 0).collect();
        let want: Vec<u8> = steps.iter().map(|s| s.byte).collect();
        assert_eq!(carried, want, "keep direction {keep}");
        assert!(
            (s.len() as f64 - total_us / BYTE_TIME_US).abs() <= 1.0,
            "{} / {}",
            s.len(),
            total_us / BYTE_TIME_US
        );
        assert_eq!(s[0], steps[0].byte, "the first step goes out at once");
    }
    assert_eq!(paced_stream(&steps), paced_stream_with(&steps, false));
    assert!(paced_stream(&[]).is_empty());
}

#[test]
fn the_filler_of_a_stream_is_zero_or_the_direction_of_the_last_step() {
    let steps = steps_of(&[2000.0, 2000.0]);
    let zero = paced_stream_with(&steps, false);
    let keep = paced_stream_with(&steps, true);
    assert_eq!(zero.len(), keep.len());
    // between the two steps: nothing, or what is left of X_POS when the step bit goes
    let gap_zero = &zero[1..zero.iter().rposition(|b| *b == bits::Z_NEG).unwrap()];
    assert!(gap_zero.iter().all(|b| *b == 0));
    let gap_keep = &keep[1..keep.iter().rposition(|b| *b == bits::Z_NEG).unwrap()];
    assert!(!gap_keep.is_empty() && gap_keep.iter().all(|b| *b == bits::X_POS & !0x15));
}

#[test]
fn two_steps_closer_than_a_byte_still_go_out_one_after_the_other() {
    let steps = steps_of(&[10.0, 10.0, 10.0, 10.0]);
    let s = paced_stream(&steps);
    assert_eq!(s.iter().filter(|b| **b != 0).count(), 4, "{s:?}");
    assert!(s.len() >= 4);
}
