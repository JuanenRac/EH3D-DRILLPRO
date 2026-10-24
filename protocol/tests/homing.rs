use eh3d_drillpro_protocol::homing::*;
use eh3d_drillpro_protocol::motion::Geometry;
use eh3d_drillpro_protocol::safety::{Machine, MachineError, Verified};
use eh3d_drillpro_protocol::{bits, plan_line, Limits, Simulator};

fn machine(limits: Option<Limits>) -> Machine<Simulator> {
    let mut sim = Simulator::default();
    sim.limits = limits;
    let mut m = Machine::new(sim);
    m.connect().unwrap();
    m.arm(Verified::all()).unwrap();
    m
}

#[test]
fn z_homing_stops_on_the_limit_switch() {
    let mut m = machine(Some(Limits {
        x: 0,
        y: 0,
        z: -100,
    }));
    home_z(&mut m, &HomingParams::default(), &Geometry::default()).unwrap();
    let sim = m.transport();
    // Searched 100 steps up, backed off 45 down, then approached until the switch closed again.
    assert_eq!(sim.z_steps, -100);
    assert_eq!(sim.sent.iter().filter(|&&b| b == 0x30).count(), 100 + 45);
}

#[test]
fn xy_homing_moves_both_axes_until_the_switch() {
    let mut m = machine(Some(Limits {
        x: -300,
        y: 0,
        z: 0,
    }));
    home_xy(&mut m, &HomingParams::default()).unwrap();
    let sim = m.transport();
    assert_eq!(sim.x_steps, -300);
    assert!(sim.sent.contains(&0x05) && sim.sent.contains(&0x4F));
}

#[test]
fn missing_switch_is_reported_like_the_original_home_signal_error() {
    let mut m = machine(None);
    let p = HomingParams {
        z_search_steps: 50,
        ..HomingParams::default()
    };
    assert!(matches!(
        home_z(&mut m, &p, &Geometry::default()),
        Err(HomingError::NoLimitSignal)
    ));
    assert_eq!(m.transport().z_steps, -50);
}

#[test]
fn already_on_the_switch_does_not_move() {
    let mut m = machine(Some(Limits { x: 0, y: 0, z: 0 }));
    let moves = plan_line(10, 0, 500.0, 500.0, 0);
    let o = m.run_until_limit(&moves, 1).unwrap();
    assert!(o.limit_hit);
    assert_eq!(o.sent, 0);
    assert_eq!(m.transport().x_steps, 0);
}

#[test]
fn run_until_limit_is_refused_when_not_armed() {
    let mut m = Machine::new(Simulator::default());
    m.connect().unwrap();
    let moves = plan_line(10, 0, 500.0, 500.0, 0);
    assert!(matches!(
        m.run_until_limit(&moves, 1),
        Err(MachineError::NotArmed)
    ));
}

#[test]
fn limit_and_fault_lines_are_readable_without_arming() {
    let mut sim = Simulator::default();
    sim.ri = true;
    let mut m = Machine::new(sim);
    m.connect().unwrap();
    let i = m.read_inputs().unwrap();
    assert!(i.limit && !i.fault);
}

#[test]
fn blind_homing_runs_the_whole_sequence_without_reading_the_switch() {
    let mut m = machine(None);
    let p = HomingParams::blind();
    home_z(&mut m, &p, &Geometry::default()).unwrap();
    let sent = &m.transport().sent.clone();
    let up = sent.iter().filter(|&&b| b == bits::Z_NEG).count();
    let down = sent.iter().filter(|&&b| b == bits::Z_POS).count();
    assert_eq!(up as u32, p.z_search_steps + p.z_reapproach_steps);
    assert_eq!(down as u32, p.z_backoff_steps);
}
