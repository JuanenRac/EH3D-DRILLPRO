//! The machine moved by hand (the Offset Teach window): it homes, follows the jogs and homes again, here on the built-in simulator.

use std::time::{Duration, Instant};

use eh3d_drillpro::protocol::connection::ConnectionConfig;
use eh3d_drillpro::runner::{TeachSession, TeachState};

fn wait_for(s: &TeachSession, want: TeachState) {
    let t0 = Instant::now();
    while s.state() != want {
        assert!(
            t0.elapsed() < Duration::from_secs(120),
            "stuck at {:?}, wanted {want:?}",
            s.state()
        );
        if let TeachState::Failed(e) = s.state() {
            panic!("the session failed: {e}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn the_session_homes_follows_the_jogs_and_homes_again() {
    let jm = eh3d_drillpro::job::job_machine(
        None,
        eh3d_drillpro::protocol::motion::Geometry::default(),
        &ConnectionConfig::default(),
        1.6,
    );
    // the simulator has no home line: the searches are sent in full
    let config = ConnectionConfig {
        home_signal: false,
        ..ConnectionConfig::default()
    };
    let s = TeachSession::start(config, jm);
    assert_eq!(s.state(), TeachState::Homing);
    wait_for(&s, TeachState::Ready);
    s.jog(2.54, -1.27, 0.0);
    s.jog(0.0, 0.0, 0.5);
    s.end();
    wait_for(&s, TeachState::Finished);
}
