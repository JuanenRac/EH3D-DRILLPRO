//! Behaviour of the planning primitives at the exact edge of each rule: the bytes of a move, its steps counted against the simulator, the ramp, the rounding of the original,
//! the geometry and its speeds, the levels of Z and the paced stream. Seeded and without a clock.

use eh3d_drillpro_protocol::bits::{self, ZDir};
use eh3d_drillpro_protocol::motion::{
    drill_stroke, duration_s, feed_xy, rapid_xy, round_half_even, z_levels, z_move, Geometry,
    ZParams, STEPS_PER_MM, Z_DEPTH_MARGIN_MM,
};
use eh3d_drillpro_protocol::{
    paced_stream, paced_stream_with, plan_line, run_steps, ConnectError, ModemStatus, Simulator,
    Step, BYTE_TIME_US,
};

/// A small generator with a fixed seed (31 bits of state).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = (self.0 * 1_103_515_245 + 12_345) & 0x7FFF_FFFF;
        self.0 >> 8
    }

    fn range(&mut self, low: i32, high: i32) -> i32 {
        low + (self.next() % (high - low + 1) as u64) as i32
    }
}

fn count_bit(steps: &[Step], bit: u8) -> usize {
    steps.iter().filter(|s| s.byte & bit != 0).count()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + a.abs().max(b.abs()))
}

#[test]
fn each_line_of_the_modem_register_is_read_from_its_own_bit() {
    for (msr, cts, dsr, ri, dcd) in [
        (0x00, false, false, false, false),
        (0x10, true, false, false, false),
        (0x20, false, true, false, false),
        (0x40, false, false, true, false),
        (0x80, false, false, false, true),
        (0x0F, false, false, false, false),
        (0xFF, true, true, true, true),
        (0xF0, true, true, true, true),
    ] {
        let s = ModemStatus::from_msr(msr);
        assert_eq!(
            (s.cts, s.dsr, s.ri, s.dcd),
            (cts, dsr, ri, dcd),
            "{msr:#04x}"
        );
    }
    assert_eq!(ModemStatus::default(), ModemStatus::from_msr(0));
}

#[test]
fn a_byte_says_the_direction_of_z_only_when_it_steps_z_and_the_two_bytes_come_back() {
    for dir in ZDir::ALL {
        assert_eq!(ZDir::of_step_byte(dir.step_byte()), Some(dir));
    }
    assert_eq!(ZDir::Down.step_byte(), bits::Z_POS);
    assert_eq!(ZDir::Up.step_byte(), bits::Z_NEG);
    for byte in 0..=255u8 {
        let says = ZDir::of_step_byte(byte);
        assert_eq!(says.is_some(), byte & bits::Z_STEP != 0, "{byte:#04x}");
        if let Some(dir) = says {
            assert_eq!(
                dir == ZDir::Down,
                byte & bits::Z_DOWN_BIT != 0,
                "{byte:#04x}"
            );
        }
    }
}

#[test]
fn the_errors_of_the_connection_are_told_in_different_words_and_an_io_error_keeps_its_text() {
    let a = ConnectError::DcdStuckHigh.to_string();
    let b = ConnectError::NoAnswer.to_string();
    assert!(!a.is_empty() && !b.is_empty() && a != b);
    let io = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "the cable came out");
    let e = ConnectError::from(io);
    assert_eq!(e, ConnectError::Io("the cable came out".into()));
    assert!(e.to_string().contains("the cable came out"));
}

#[test]
fn a_move_of_no_steps_is_an_empty_plan() {
    assert!(plan_line(0, 0, 500.0, 4000.0, 40).is_empty());
    assert!(plan_line(0, 0, 500.0, 4000.0, 0).is_empty());
    assert_eq!(duration_s(&[]), 0.0);
}

#[test]
fn the_byte_of_a_pure_move_in_each_direction_is_the_one_of_the_original() {
    let byte = |dx, dy| plan_line(dx, dy, 500.0, 500.0, 0)[0].byte;
    assert_eq!(byte(3, 0), 0x4B);
    assert_eq!(byte(-3, 0), 0x09);
    assert_eq!(byte(0, 3), 0x4E);
    assert_eq!(byte(0, -3), 0x06);
    assert_eq!(byte(1, 0), bits::X_POS | bits::Y_DIR);
    assert_eq!(byte(0, 1), bits::Y_POS | bits::X_DIR);
}

#[test]
fn a_diagonal_of_equal_sides_steps_both_axes_in_every_byte_and_ties_go_to_x() {
    let steps = plan_line(5, 5, 500.0, 500.0, 0);
    assert_eq!(steps.len(), 5);
    assert!(steps.iter().all(|s| s.byte == 0x4F));
    let back = plan_line(-5, -5, 500.0, 500.0, 0);
    assert!(back.iter().all(|s| s.byte == (bits::X_NEG | bits::Y_NEG)));
}

#[test]
fn a_move_has_as_many_bytes_as_its_longest_side_and_each_axis_steps_exactly_its_distance() {
    let mut rng = Lcg(31);
    for _ in 0..300 {
        let (dx, dy) = (rng.range(-400, 400), rng.range(-400, 400));
        let steps = plan_line(dx, dy, 500.0, 4000.0, 40);
        assert_eq!(
            steps.len(),
            dx.unsigned_abs().max(dy.unsigned_abs()) as usize
        );
        assert_eq!(
            count_bit(&steps, 0x01),
            dx.unsigned_abs() as usize,
            "{dx},{dy}"
        );
        assert_eq!(
            count_bit(&steps, 0x04),
            dy.unsigned_abs() as usize,
            "{dx},{dy}"
        );
    }
}

#[test]
fn the_simulator_ends_where_the_plan_says_for_any_seeded_move() {
    let mut rng = Lcg(77);
    for _ in 0..100 {
        let (dx, dy) = (rng.range(-300, 300), rng.range(-300, 300));
        let mut sim = Simulator::default();
        run_steps(&mut sim, &plan_line(dx, dy, 500.0, 4000.0, 20)).unwrap();
        assert_eq!(
            (sim.x_steps, sim.y_steps, sim.z_steps),
            (i64::from(dx), i64::from(dy), 0)
        );
        let mut there_and_back = Simulator::default();
        run_steps(&mut there_and_back, &plan_line(dx, dy, 500.0, 4000.0, 20)).unwrap();
        run_steps(&mut there_and_back, &plan_line(-dx, -dy, 500.0, 4000.0, 20)).unwrap();
        assert_eq!((there_and_back.x_steps, there_and_back.y_steps), (0, 0));
    }
}

#[test]
fn without_a_ramp_every_step_has_the_delay_of_the_top_speed() {
    let steps = plan_line(50, 20, 100.0, 2000.0, 0);
    assert!(steps.iter().all(|s| s.delay_us == 1e6 / 2000.0));
}

#[test]
fn the_ramp_starts_and_ends_at_the_start_speed_and_is_the_same_both_ways() {
    let steps = plan_line(200, 0, 500.0, 4000.0, 40);
    assert_eq!(steps[0].delay_us, 1e6 / 500.0);
    assert_eq!(steps[199].delay_us, 1e6 / 500.0);
    assert_eq!(steps[100].delay_us, 1e6 / 4000.0);
    for (i, (a, b)) in steps.iter().zip(steps.iter().rev()).enumerate() {
        assert_eq!(a.delay_us, b.delay_us, "{i}");
    }
    assert!(steps
        .iter()
        .all(|s| s.delay_us >= 1e6 / 4000.0 && s.delay_us <= 1e6 / 500.0));
}

#[test]
fn a_move_shorter_than_its_ramp_never_reaches_the_top_speed_and_a_move_of_one_step_starts_slowly() {
    let short = plan_line(10, 0, 500.0, 4000.0, 40);
    assert!(short.iter().all(|s| s.delay_us > 1e6 / 4000.0));
    let one = plan_line(1, 0, 500.0, 4000.0, 40);
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].delay_us, 1e6 / 500.0);
}

#[test]
fn the_same_move_there_and_back_takes_the_same_time() {
    let mut rng = Lcg(5);
    for _ in 0..50 {
        let (dx, dy) = (rng.range(-300, 300), rng.range(-300, 300));
        let there = duration_s(&plan_line(dx, dy, 500.0, 4000.0, 40));
        let back = duration_s(&plan_line(-dx, -dy, 500.0, 4000.0, 40));
        assert!(close(there, back), "{dx},{dy}: {there} {back}");
    }
}

#[test]
fn the_rounding_of_the_original_sends_a_half_to_the_even_number() {
    for (value, expected) in [
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
    ] {
        assert_eq!(round_half_even(value), expected, "{value}");
    }
}

#[test]
fn the_rounding_is_never_more_than_half_a_step_away_and_an_integer_stays() {
    let mut rng = Lcg(9);
    for _ in 0..500 {
        let v = f64::from(rng.range(-100_000, 100_000)) / 8.0;
        let r = round_half_even(v);
        assert!((r - v).abs() <= 0.5 + 1e-9, "{v} -> {r}");
        assert_eq!(r.fract(), 0.0);
        let whole = f64::from(rng.range(-5000, 5000));
        assert_eq!(round_half_even(whole), whole);
    }
}

#[test]
fn the_steps_of_a_distance_follow_the_mil_and_the_correction_of_each_axis() {
    assert!(close(STEPS_PER_MM, 39.370_078_74));
    let g = Geometry::default();
    assert_eq!(g.x_steps(25.4), 1000);
    assert_eq!(g.y_steps(25.4), 1000);
    assert_eq!(g.x_steps(-25.4), -1000);
    assert_eq!(g.x_steps(0.0), 0);
    assert_eq!(g.x_steps(1.0), 39);
    let c = Geometry::with_correction(2.0, 0.5, 1.0);
    assert_eq!(c.x_steps(25.4), 2000);
    assert_eq!(c.y_steps(25.4), 500);
    assert_eq!(c.z_steps(25.4), 1000);
}

#[test]
fn z_never_goes_below_zero_steps_and_its_millimetres_come_back_within_one_step() {
    let g = Geometry::with_correction(1.0, 1.0, 1.25);
    assert_eq!(g.z_steps(-3.0), 0);
    assert_eq!(g.z_steps(0.0), 0);
    let mut rng = Lcg(21);
    for _ in 0..200 {
        let mm = f64::from(rng.range(0, 40_000)) / 1000.0;
        let back = g.z_mm(g.z_steps(mm));
        assert!(
            (back - mm).abs() <= 0.5 / (STEPS_PER_MM * 1.25) + 1e-9,
            "{mm} -> {back}"
        );
    }
}

#[test]
fn a_feed_of_nothing_or_less_still_makes_a_step_rate_of_one_hertz() {
    let g = Geometry::default();
    assert!(close(g.xy_hz(60.0), STEPS_PER_MM));
    assert_eq!(g.xy_hz(0.0), 1.0);
    assert_eq!(g.xy_hz(-500.0), 1.0);
    assert_eq!(g.z_hz(0.0), 1.0);
    assert!(close(
        Geometry::with_correction(1.0, 1.0, 2.0).z_hz(60.0),
        STEPS_PER_MM * 2.0
    ));
    assert!(close(g.z_max_feed(), g.z_max_hz * 60.0 / STEPS_PER_MM));
}

#[test]
fn whatever_the_speeds_of_the_configuration_the_start_and_the_reference_stay_under_the_top() {
    let mut rng = Lcg(404);
    for _ in 0..200 {
        let mut speed = || f64::from(rng.range(1, 6000));
        let xy = (speed(), speed(), speed());
        let z = (speed(), speed(), speed());
        let g = Geometry::default().with_speeds(xy, z);
        assert!(g.start_hz <= g.max_hz && g.ref_hz <= g.max_hz, "{xy:?}");
        assert!(
            g.z_start_hz <= g.z_max_hz && g.z_ref_hz <= g.z_max_hz,
            "{z:?}"
        );
        assert!(close(g.max_hz, Geometry::default().xy_hz(xy.0)));
    }
}

#[test]
fn the_cap_of_the_motors_lowers_every_speed_and_a_cap_above_them_changes_nothing() {
    let g = Geometry::default();
    assert_eq!(g.capped(None, None), g);
    assert_eq!(g.capped(Some(1.0e9), Some(1.0e9)), g);
    let low = g.capped(Some(800.0), Some(300.0));
    assert_eq!(low.max_hz, 800.0);
    assert_eq!(low.start_hz, 500.0);
    assert_eq!(low.ref_hz, 800.0);
    assert_eq!(low.z_max_hz, 300.0);
    assert_eq!(low.z_start_hz, 300.0);
    assert_eq!(low.z_ref_hz, 300.0);
    let xy_only = g.capped(Some(100.0), None);
    assert_eq!((xy_only.max_hz, xy_only.start_hz), (100.0, 100.0));
    assert_eq!(xy_only.z_max_hz, g.z_max_hz);
}

#[test]
fn a_rapid_move_between_two_points_has_the_steps_of_the_difference_of_their_rounded_steps() {
    let g = Geometry::default();
    assert!(rapid_xy((3.0, 4.0), (3.0, 4.0), &g).is_empty());
    let mut rng = Lcg(8);
    for _ in 0..100 {
        let a = (
            f64::from(rng.range(0, 3000)) / 10.0,
            f64::from(rng.range(0, 2000)) / 10.0,
        );
        let b = (
            f64::from(rng.range(0, 3000)) / 10.0,
            f64::from(rng.range(0, 2000)) / 10.0,
        );
        let steps = rapid_xy(a, b, &g);
        let dx = (g.x_steps(b.0) - g.x_steps(a.0)).unsigned_abs();
        let dy = (g.y_steps(b.1) - g.y_steps(a.1)).unsigned_abs();
        assert_eq!(steps.len(), dx.max(dy) as usize);
        assert_eq!(steps.len(), rapid_xy(b, a, &g).len());
    }
}

#[test]
fn a_move_at_a_feed_has_no_ramp_and_never_goes_faster_than_the_top_speed() {
    let g = Geometry::default();
    let slow = feed_xy((0.0, 0.0), (10.0, 0.0), 60.0, &g);
    assert!(slow.iter().all(|s| close(s.delay_us, 1e6 / STEPS_PER_MM)));
    let fast = feed_xy((0.0, 0.0), (10.0, 0.0), 1.0e6, &g);
    assert!(fast.iter().all(|s| close(s.delay_us, 1e6 / g.max_hz)));
    assert_eq!(slow.len(), fast.len());
    assert!(feed_xy((1.0, 1.0), (1.0, 1.0), 60.0, &g).is_empty());
}

#[test]
fn a_z_move_has_the_steps_of_its_depth_all_in_the_same_direction_at_the_same_pace() {
    let g = Geometry::default();
    let down = z_move(2.0, true, 120.0, &g);
    let up = z_move(2.0, false, 120.0, &g);
    assert_eq!(down.len(), g.z_steps(2.0) as usize);
    assert_eq!(up.len(), down.len());
    assert!(down.iter().all(|s| s.byte == bits::Z_POS));
    assert!(up.iter().all(|s| s.byte == bits::Z_NEG));
    assert!(down.iter().all(|s| s.delay_us == 1e6 / g.z_hz(120.0)));
    assert!(z_move(-1.0, true, 120.0, &g).is_empty());
    assert!(z_move(0.0, true, 120.0, &g).is_empty());
}

#[test]
fn a_drilling_stroke_goes_down_slowly_and_up_at_the_fastest_rate_and_ends_where_it_began() {
    let g = Geometry::default();
    let stroke = drill_stroke(1.6, 60.0, &g);
    let n = g.z_steps(1.6) as usize;
    assert_eq!(stroke.len(), 2 * n);
    assert!(stroke[..n].iter().all(|s| s.byte == bits::Z_POS));
    assert!(stroke[n..].iter().all(|s| s.byte == bits::Z_NEG));
    assert!(stroke[n..]
        .iter()
        .all(|s| close(s.delay_us, 1e6 / g.z_max_hz)));
    assert!(duration_s(&stroke[..n]) > duration_s(&stroke[n..]));
    let mut sim = Simulator::default();
    run_steps(&mut sim, &stroke).unwrap();
    assert_eq!(sim.z_steps, 0);
}

#[test]
fn the_time_of_a_plan_is_the_sum_of_its_pauses() {
    let steps = vec![
        Step {
            byte: bits::X_POS,
            delay_us: 250.0
        };
        8
    ];
    assert!(close(duration_s(&steps), 0.002));
    assert!(close(
        duration_s(&steps[..3]) + duration_s(&steps[3..]),
        duration_s(&steps)
    ));
}

fn params(max_depth: f64, base: f64, board: f64, distance: f64, extra: f64) -> ZParams {
    ZParams {
        max_depth_mm: max_depth,
        base_mm: base,
        board_mm: board,
        distance_tool_board_mm: distance,
        extra_stroke_mm: extra,
    }
}

#[test]
fn the_levels_of_z_are_cut_to_whole_steps_and_the_travel_is_what_is_left_of_the_depth() {
    let g = Geometry::default();
    let l = z_levels(&params(37.3, 0.0, 0.0, 0.0, 0.0), &g);
    assert_eq!(l.stroke_steps, 0);
    assert_eq!(l.end_steps, 1488);
    assert_eq!(l.travel_steps, 1488);
    assert!(close(Z_DEPTH_MARGIN_MM, 0.5));
    let deeper = z_levels(&params(37.3, 0.0, 1.0, 0.0, 0.0), &g);
    assert_eq!(deeper.stroke_steps, 39);
    assert_eq!(deeper.travel_steps, deeper.end_steps - deeper.stroke_steps);
}

#[test]
fn a_stroke_longer_than_the_depth_leaves_no_travel_and_nothing_is_negative() {
    let g = Geometry::default();
    let l = z_levels(&params(1.0, 0.0, 20.0, 5.0, 0.0), &g);
    assert!(l.stroke_steps > l.end_steps);
    assert_eq!(l.travel_steps, 0);
    let flat = z_levels(&params(10.0, 10.5, 0.0, 0.0, 0.0), &g);
    assert_eq!(flat.end_steps, 0);
    let negative = z_levels(&params(-5.0, 0.0, -5.0, -5.0, 0.0), &g);
    assert_eq!(
        (
            negative.stroke_steps,
            negative.end_steps,
            negative.travel_steps
        ),
        (0, 0, 0)
    );
}

#[test]
fn a_thicker_board_makes_a_longer_stroke_and_a_thicker_base_a_shallower_end() {
    let g = Geometry::default();
    let mut rng = Lcg(66);
    for _ in 0..100 {
        let board = f64::from(rng.range(0, 60)) / 10.0;
        let base = f64::from(rng.range(0, 200)) / 10.0;
        let a = z_levels(&params(37.3, base, board, 5.0, 0.0), &g);
        let thicker = z_levels(&params(37.3, base, board + 1.0, 5.0, 0.0), &g);
        let higher = z_levels(&params(37.3, base + 1.0, board, 5.0, 0.0), &g);
        assert!(thicker.stroke_steps >= a.stroke_steps);
        assert!(higher.end_steps <= a.end_steps);
        assert_eq!(
            a.end_steps - a.stroke_steps.min(a.end_steps),
            a.travel_steps
        );
    }
}

#[test]
fn a_stream_of_no_steps_is_empty_and_a_step_sits_at_the_slot_of_its_time() {
    assert!(paced_stream(&[]).is_empty());
    let slot = BYTE_TIME_US;
    let steps = [
        Step {
            byte: 0x43,
            delay_us: 3.0 * slot,
        },
        Step {
            byte: 0x4B,
            delay_us: 2.0 * slot,
        },
    ];
    assert_eq!(paced_stream(&steps), vec![0x43, 0x00, 0x00, 0x4B, 0x00]);
    assert_eq!(paced_stream(&steps).len(), 5);
}

#[test]
fn steps_faster_than_a_byte_are_never_dropped_or_overlapped() {
    let steps: Vec<Step> = (0..20)
        .map(|i| Step {
            byte: if i % 2 == 0 { 0x43 } else { 0x01 },
            delay_us: BYTE_TIME_US / 10.0,
        })
        .collect();
    let stream = paced_stream(&steps);
    assert!(stream.len() >= steps.len());
    let sent: Vec<u8> = stream.iter().copied().filter(|b| *b != 0).collect();
    let planned: Vec<u8> = steps.iter().map(|s| s.byte).collect();
    assert_eq!(sent, planned);
}

#[test]
fn the_filler_keeps_the_direction_bits_only_when_it_is_asked_to() {
    let slot = BYTE_TIME_US;
    let steps = [Step {
        byte: bits::Z_NEG,
        delay_us: 3.0 * slot,
    }];
    assert_eq!(paced_stream_with(&steps, false), vec![0x30, 0x00, 0x00]);
    assert_eq!(paced_stream_with(&steps, true), vec![0x30, 0x20, 0x20]);
    assert_eq!(paced_stream_with(&steps, false), paced_stream(&steps));
}

#[test]
fn the_length_of_a_stream_follows_the_total_time_of_a_seeded_plan() {
    let mut rng = Lcg(123);
    for _ in 0..40 {
        let (dx, dy) = (rng.range(1, 200), rng.range(-200, 200));
        let steps = plan_line(dx, dy, 500.0, 2000.0, 20);
        let stream = paced_stream(&steps);
        let expected = duration_s(&steps) * 1e6 / BYTE_TIME_US;
        assert!((stream.len() as f64 - expected).abs() <= 1.0, "{dx},{dy}");
        let sent = stream.iter().filter(|b| **b != 0).count();
        assert_eq!(sent, steps.len());
    }
}
