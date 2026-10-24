use eh3d_drillpro_protocol::*;

#[test]
fn handshake_succeeds_against_simulator() {
    let mut sim = Simulator::default();
    assert_eq!(connect(&mut sim), Ok(()));
    assert_eq!(sim.sent, vec![0x00, 0x10, 0x00, 0x10]);
}

#[test]
fn handshake_fails_when_nothing_answers() {
    struct Dead;
    impl Transport for Dead {
        fn set_dtr(&mut self, _: bool) -> std::io::Result<()> {
            Ok(())
        }
        fn write_byte(&mut self, _: u8) -> std::io::Result<()> {
            Ok(())
        }
        fn status(&mut self) -> std::io::Result<ModemStatus> {
            Ok(ModemStatus::default())
        }
        fn sleep_us(&mut self, _: u64) {}
    }
    assert_eq!(connect(&mut Dead), Err(ConnectError::NoAnswer));
}

#[test]
fn diagonal_line_reaches_the_target() {
    for &(dx, dy) in &[(100, 37), (-80, 80), (5, -300), (0, 0), (-1, 0), (250, 250)] {
        let steps = plan_line(dx, dy, 500.0, 4000.0, 20);
        let mut sim = Simulator::default();
        run_steps(&mut sim, &steps).unwrap();
        assert_eq!(
            (sim.x_steps, sim.y_steps),
            (dx as i64, dy as i64),
            "move {dx},{dy}"
        );
    }
}

#[test]
fn ramp_is_slowest_at_the_ends() {
    let steps = plan_line(200, 0, 500.0, 4000.0, 20);
    assert!(steps[0].delay_us > steps[100].delay_us);
    assert!(steps[199].delay_us > steps[100].delay_us);
}

#[test]
fn byte_time_matches_38400_baud() {
    assert!((BYTE_TIME_US - 260.4).abs() < 0.1);
}

#[test]
fn paced_stream_places_each_step_in_its_byte_slot() {
    use eh3d_drillpro_protocol::{paced_stream, Step, BYTE_TIME_US};
    let step = |byte, slots: f64| Step {
        byte,
        delay_us: slots * BYTE_TIME_US,
    };
    let v = paced_stream(&[step(0x43, 2.0), step(0x43, 1.0), step(0x43, 3.0)]);
    assert_eq!(v, vec![0x43, 0x00, 0x43, 0x43, 0x00, 0x00]);
}

#[test]
fn a_filler_that_keeps_the_direction_clears_only_the_step_bits() {
    use eh3d_drillpro_protocol::{bits, paced_stream_with, Step, BYTE_TIME_US};
    let step = |byte, slots: f64| Step {
        byte,
        delay_us: slots * BYTE_TIME_US,
    };
    // Z up (0x30) keeps 0x20 between steps; Z down (0x50) keeps 0x40; X negative has no direction bits left
    let up = paced_stream_with(&[step(bits::Z_NEG, 3.0), step(bits::Z_NEG, 3.0)], true);
    assert_eq!(up, vec![0x30, 0x20, 0x20, 0x30, 0x20, 0x20]);
    let down = paced_stream_with(&[step(bits::Z_POS, 2.0), step(bits::Z_POS, 2.0)], true);
    assert_eq!(down, vec![0x50, 0x40, 0x50, 0x40]);
    let x = paced_stream_with(&[step(bits::X_NEG, 2.0), step(bits::X_NEG, 2.0)], true);
    assert_eq!(x, vec![0x01, 0x00, 0x01, 0x00]);
    // the plain stream is unchanged: zero fillers
    let plain = paced_stream_with(&[step(bits::Z_NEG, 3.0)], false);
    assert_eq!(plain, vec![0x30, 0x00, 0x00]);
}
