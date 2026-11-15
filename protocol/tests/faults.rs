//! Failures of the link on the simulator: the program says what went wrong, stops, and works again once the link is back.
//! Every test is deterministic (fixed seeds, no clock, no port).

use eh3d_drillpro_protocol::faulty::{Fault, FaultyLink};
use eh3d_drillpro_protocol::homing::{home_z, HomingError, HomingParams};
use eh3d_drillpro_protocol::motion::{rapid_xy, Geometry};
use eh3d_drillpro_protocol::safety::{Machine, MachineError, Verified};
use eh3d_drillpro_protocol::{ConnectError, Limits, Simulator, Transport};

#[allow(clippy::field_reassign_with_default)]
fn simulator_with(limits: Limits) -> Simulator {
    let mut sim = Simulator::default();
    sim.limits = Some(limits);
    sim
}

fn armed(link: FaultyLink<Simulator>) -> Machine<FaultyLink<Simulator>> {
    let mut m = Machine::new(link);
    m.connect().unwrap();
    m.arm(Verified::all()).unwrap();
    m
}

#[test]
fn a_silent_unit_is_reported_and_the_connection_works_once_it_answers() {
    let link = FaultyLink::new(Simulator::default()).with(Fault::SilentUnit);
    let mut m = Machine::new(link);
    assert!(matches!(
        m.connect(),
        Err(MachineError::Connect(ConnectError::NoAnswer))
    ));
    // not connected, so nothing can be armed or moved
    assert!(matches!(
        m.arm(Verified::all()),
        Err(MachineError::NotConnected)
    ));
    let mut link = m.into_transport();
    link.clear();
    let mut again = Machine::new(link);
    again.connect().unwrap();
    again.arm(Verified::all()).unwrap();
}

#[test]
fn a_fault_line_stuck_high_is_told_before_anything_is_sent() {
    let link = FaultyLink::new(Simulator::default()).with(Fault::StuckFault);
    let mut m = Machine::new(link);
    assert!(matches!(
        m.connect(),
        Err(MachineError::Connect(ConnectError::DcdStuckHigh))
    ));
    assert_eq!(m.transport().written(), 0, "no byte goes out");
}

#[test]
fn a_cable_pulled_in_the_middle_of_a_run_stops_it_and_a_second_run_works() {
    // the handshake writes four bytes; the cable comes out ten bytes later
    let link = FaultyLink::new(Simulator::default()).with(Fault::CablePulled { after: 14 });
    let mut m = armed(link);
    let plan = rapid_xy((0.0, 0.0), (5.0, 5.0), &Geometry::default());
    assert!(plan.len() > 10);
    match m.run(&plan) {
        Err(MachineError::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::BrokenPipe),
        other => panic!("expected an I/O error, got {other:?}"),
    }
    let sent_before = m.transport().inner().sent.len();
    assert_eq!(
        sent_before, 14,
        "exactly the bytes before the cable came out"
    );
    // the machine stops and the operator plugs the cable again
    m.emergency_stop();
    let mut link = m.into_transport();
    link.clear();
    let mut again = armed(link);
    again.run(&plan).unwrap();
    assert!(again.transport().inner().sent.len() > sent_before + plan.len());
}

#[test]
fn a_dead_limit_switch_ends_the_reference_run_with_no_limit_signal() {
    let limits = Limits {
        x: 0,
        y: 0,
        z: -100,
    };
    let sim = simulator_with(limits);
    let link = FaultyLink::new(sim).with(Fault::DeadLimitSwitch);
    let mut m = armed(link);
    let r = home_z(&mut m, &HomingParams::default(), &Geometry::default());
    assert!(matches!(r, Err(HomingError::NoLimitSignal)), "{r:?}");
    // the whole search was sent: the program did not stop early on a signal it never saw
    assert!(m.transport().inner().z_steps <= -1320);

    // with the switch working the same run ends on the switch
    let sim = simulator_with(limits);
    let mut ok = armed(FaultyLink::new(sim));
    home_z(&mut ok, &HomingParams::default(), &Geometry::default()).unwrap();
}

#[test]
fn a_status_that_cannot_be_read_is_an_error_not_a_guess() {
    let mut link = FaultyLink::new(Simulator::default()).with(Fault::StatusReadFails);
    assert!(link.status().is_err());
    link.clear();
    assert!(link.status().is_ok());
    // the machine tells the same through its own read of the lines
    let mut m = Machine::new(FaultyLink::new(Simulator::default()));
    m.connect().unwrap();
    let mut link = m.into_transport();
    link = link.with(Fault::StatusReadFails);
    let mut m = Machine::new(link);
    m.connect().unwrap_err();
}

#[test]
fn lost_bytes_are_the_same_for_the_same_seed_and_differ_for_another() {
    let run = |seed: u64| {
        let mut link = FaultyLink::new(Simulator::default()).with_loss(seed, 300);
        for i in 0..1000u32 {
            link.write_byte((i % 100) as u8).unwrap();
        }
        (link.dropped(), link.inner().sent.clone())
    };
    let (a, sent_a) = run(7);
    let (b, sent_b) = run(7);
    let (c, sent_c) = run(8);
    assert_eq!(
        (a, &sent_a),
        (b, &sent_b),
        "the same seed loses the same bytes"
    );
    assert!(a > 200 && a < 400, "about 300 of 1000 are lost, got {a}");
    assert_eq!(
        sent_a.len() + a,
        1000,
        "every byte is either sent or counted as lost"
    );
    assert!(sent_a != sent_c || a != c, "another seed loses other bytes");
}

#[test]
fn a_link_without_faults_behaves_like_the_simulator() {
    let plan = rapid_xy((0.0, 0.0), (3.0, 2.0), &Geometry::default());
    let mut plain = Machine::new(Simulator::default());
    plain.connect().unwrap();
    plain.arm(Verified::all()).unwrap();
    plain.run(&plan).unwrap();
    let mut wrapped = armed(FaultyLink::new(Simulator::default()));
    wrapped.run(&plan).unwrap();
    assert_eq!(plain.transport().sent, wrapped.transport().inner().sent);
}
