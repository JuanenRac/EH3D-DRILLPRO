use eh3d_drillpro_protocol::latches::*;
use eh3d_drillpro_protocol::safety::{Machine, MachineError, Verified};
use eh3d_drillpro_protocol::Simulator;

fn sent(seq: &Sequence) -> Vec<u8> {
    seq.ops
        .iter()
        .filter_map(|o| if let Op::Send(b) = o { Some(*b) } else { None })
        .collect()
}

#[test]
fn idle_values_match_the_original() {
    let l = Latches::default();
    assert_eq!((l.a, l.b), (0x80, 0xC0));
}

#[test]
fn drivers_toggle_only_their_bit() {
    let l = Latches::default();
    let on = l.drivers_on();
    assert_eq!(sent(&on), vec![0xD0]);
    assert_eq!(sent(&on.next.drivers_off()), vec![0xC0]);
}

#[test]
fn building_a_sequence_does_not_change_the_state() {
    let l = Latches::default();
    let _ = l.spindle_on(9, 0);
    assert_eq!(l, Latches::default());
}

#[test]
fn spindle_start_sequence_orders_latch_b_before_a() {
    let seq = Latches::default().spindle_on(9, 300_000);
    assert_eq!(sent(&seq), vec![0xC0 | 0x28, 0x90, 0x90 | 9]);
    assert!(matches!(seq.ops[1], Op::WaitUs(100_000)));
}

#[test]
fn spindle_stop_releases_run_and_brake() {
    let started = Latches::default().spindle_on(5, 0).next;
    let stopped = started.spindle_off(200_000);
    assert_eq!(stopped.next.a & A_SPINDLE_RUN, 0);
    assert_eq!(stopped.next.b & (B_SPINDLE_BRAKE | B_SPINDLE_ENABLE), 0);
    assert_eq!(sent(&stopped).len(), 3);
}

#[test]
fn speed_level_is_limited_to_four_bits() {
    let seq = Latches::default().spindle_on(0xFF, 0);
    assert_eq!(sent(&seq).last().copied().unwrap() & 0xF0, 0x90);
}

#[test]
fn shutdown_is_the_original_sequence() {
    let b: Vec<u8> = SHUTDOWN
        .iter()
        .filter_map(|o| if let Op::Send(b) = o { Some(*b) } else { None })
        .collect();
    assert_eq!(b, vec![0x00, 0x10, 0xC0]);
}

#[test]
fn a_refused_sequence_leaves_the_shadow_state_untouched() {
    let mut m = Machine::new(Simulator::default());
    m.connect().unwrap();
    let before = m.latches();
    assert!(matches!(
        m.apply(m.latches().spindle_on(3, 0)),
        Err(MachineError::NotArmed)
    ));
    assert_eq!(m.latches(), before);
    m.arm(Verified::all()).unwrap();
    m.apply(m.latches().drivers_on()).unwrap();
    assert_eq!(m.latches().b, 0xD0);
    assert!(m.transport().sent.contains(&0xD0));
}
