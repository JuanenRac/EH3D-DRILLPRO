//! The errors of the link are told in words, not by the name of the error type.

use eh3d_drillpro_protocol::ConnectError;

#[test]
fn the_errors_of_the_link_are_told_in_words() {
    for e in [
        ConnectError::DcdStuckHigh,
        ConnectError::NoAnswer,
        ConnectError::Io("port busy".into()),
    ] {
        let t = e.to_string();
        assert!(t.len() > 20, "{t}");
        assert!(
            !t.contains("DcdStuckHigh") && !t.contains("NoAnswer"),
            "{t}"
        );
    }
    assert!(ConnectError::Io("port busy".into())
        .to_string()
        .contains("port busy"));
}

#[test]
fn a_failed_connection_reads_as_a_sentence() {
    use eh3d_drillpro_protocol::homing::HomingError;
    use eh3d_drillpro_protocol::safety::MachineError;
    let t = MachineError::Connect(ConnectError::NoAnswer).to_string();
    assert!(t.starts_with("connection failed: "), "{t}");
    assert!(!t.contains("NoAnswer"), "{t}");
    let h = HomingError::Machine(MachineError::NotArmed).to_string();
    assert!(h.contains("not verified"), "{h}");
    assert!(HomingError::NoLimitSignal.to_string().contains("limit"));
    assert!(HomingError::Fault.to_string().contains("fault"));
}
