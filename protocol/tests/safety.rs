use eh3d_drillpro_protocol::motion::*;
use eh3d_drillpro_protocol::safety::*;
use eh3d_drillpro_protocol::{bits, Simulator};

fn connected() -> Machine<Simulator> {
    let mut m = Machine::new(Simulator::default());
    m.connect().unwrap();
    m
}

#[test]
fn motion_is_refused_until_everything_is_verified() {
    let mut m = connected();
    let plan = rapid_xy((0.0, 0.0), (5.0, 5.0), &Geometry::default());
    assert!(matches!(m.run(&plan), Err(MachineError::NotArmed)));
    assert!(matches!(
        m.arm(Verified {
            bit_map: true,
            geometry: false
        }),
        Err(MachineError::NotArmed)
    ));
    assert!(matches!(
        m.arm(Verified::default()),
        Err(MachineError::NotArmed)
    ));
    assert!(
        m.transport().sent.iter().all(|&b| b == 0x00 || b == 0x10),
        "only the handshake may have been sent"
    );
    m.arm(Verified::all()).unwrap();
    m.run(&plan).unwrap();
    assert!(!m.transport().sent.is_empty());
}

#[test]
fn arming_needs_a_connection_and_disarm_stops_motion() {
    let mut m = Machine::new(Simulator::default());
    assert!(matches!(
        m.arm(Verified::all()),
        Err(MachineError::NotConnected)
    ));
    let mut m = connected();
    m.arm(Verified::all()).unwrap();
    m.disarm();
    assert!(matches!(
        m.run(&rapid_xy((0.0, 0.0), (1.0, 0.0), &Geometry::default())),
        Err(MachineError::NotArmed)
    ));
}

#[test]
fn inputs_can_be_read_without_arming() {
    let mut m = connected();
    assert!(m.inputs().is_ok());
}

#[test]
fn rapid_move_has_the_right_number_of_steps() {
    let g = Geometry::default();
    let steps = rapid_xy((0.0, 0.0), (10.0, 2.5), &g);
    assert_eq!(steps.len(), 394); // 10 mm * 39.3701 steps/mm on the major axis
}

#[test]
fn drill_stroke_goes_down_then_up() {
    let g = Geometry::default();
    let s = drill_stroke(2.0, 600.0, &g);
    let n = g.z_steps(2.0) as usize;
    assert_eq!(s.len(), 2 * n);
    assert!(s[..n].iter().all(|x| x.byte == bits::Z_NEG));
    assert!(s[n..].iter().all(|x| x.byte == bits::Z_POS));
    assert!(
        s[0].delay_us > s[n].delay_us,
        "the way up is faster than the feed down"
    );
}

#[test]
fn duration_follows_the_feed() {
    let g = Geometry::default();
    let slow = feed_xy((0.0, 0.0), (20.0, 0.0), 300.0, &g);
    let fast = feed_xy((0.0, 0.0), (20.0, 0.0), 1200.0, &g);
    assert!(duration_s(&slow) > 3.0 * duration_s(&fast));
}
