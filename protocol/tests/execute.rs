use eh3d_drillpro_protocol::execute::{run_items, Hooks, Outcome};
use eh3d_drillpro_protocol::job::{
    flatten_bytes, plan_drill_job, JobHole, JobMachine, JobTool, Pause,
};
use eh3d_drillpro_protocol::motion::Geometry;
use eh3d_drillpro_protocol::safety::{Machine, Verified};
use eh3d_drillpro_protocol::{Pacing, Simulator};

struct Auto {
    pauses: Vec<Pause>,
    seen: Vec<usize>,
    cancel_at_item: Option<usize>,
    refuse_pause: bool,
}

impl Hooks for Auto {
    fn pause(&mut self, p: Pause) -> bool {
        self.pauses.push(p);
        !self.refuse_pause
    }
    fn progress(&mut self, next: usize, _total: usize) {
        self.seen.push(next);
    }
    fn cancelled(&mut self) -> bool {
        matches!(self.cancel_at_item, Some(n) if self.seen.last().copied().unwrap_or(0) >= n)
    }
}

fn auto() -> Auto {
    Auto {
        pauses: vec![],
        seen: vec![],
        cancel_at_item: None,
        refuse_pause: false,
    }
}

fn job() -> Vec<eh3d_drillpro_protocol::job::Item> {
    let m = JobMachine {
        geometry: Geometry::with_correction(0.9992, 0.9998, 0.8988),
        offset_mm: (2.5, 2.5),
        max_depth_mm: 37.3,
        base_mm: 12.0,
        board_mm: 1.6,
        distance_tool_board_mm: 16.0,
        xy_search_steps: 26_613,
    };
    let tools = [
        JobTool {
            number: 1,
            extra_stroke_mm: 0.5,
            feed_mm_min: 1000,
            spindle_rpm: 60_000,
        },
        JobTool {
            number: 2,
            extra_stroke_mm: 0.5,
            feed_mm_min: 1000,
            spindle_rpm: 60_000,
        },
    ];
    let holes = [
        JobHole {
            tool: 1,
            x_mm: 10.0,
            y_mm: 10.0,
        },
        JobHole {
            tool: 1,
            x_mm: 20.0,
            y_mm: 10.0,
        },
        JobHole {
            tool: 2,
            x_mm: 20.0,
            y_mm: 20.0,
        },
    ];
    plan_drill_job(&m, &tools, &holes)
}

fn armed_machine(pacing: Pacing) -> Machine<Simulator> {
    let mut m = Machine::new(Simulator::default());
    m.set_pacing(pacing);
    m.connect().unwrap();
    m.arm(Verified::all()).unwrap();
    m
}

#[test]
fn a_whole_job_reaches_the_machine_byte_for_byte() {
    let items = job();
    for pacing in [Pacing::Bytes, Pacing::Stream] {
        let mut m = armed_machine(pacing);
        let before = m.transport().sent.len(); // the handshake bytes
        let mut h = auto();
        assert_eq!(
            run_items(&mut m, &items, &mut h).unwrap(),
            Outcome::Completed
        );
        let sent: Vec<u8> = m.transport().sent[before..].to_vec();
        let wanted = flatten_bytes(&items);
        if pacing == Pacing::Bytes {
            assert_eq!(sent, wanted);
        } else {
            // paced: the stream carries 0x00 fillers between the step bytes; without them it is the same
            let steps_only: Vec<u8> = sent.iter().copied().filter(|&b| b != 0x00).collect();
            let wanted_nz: Vec<u8> = wanted.iter().copied().filter(|&b| b != 0x00).collect();
            assert_eq!(steps_only, wanted_nz);
        }
        assert_eq!(
            h.pauses,
            vec![
                Pause::ToolChange { tool: 1 },
                Pause::SpindleStart,
                Pause::ToolChange { tool: 2 }
            ]
        );
        assert_eq!(*h.seen.last().unwrap(), items.len());
    }
}

#[test]
fn cancelling_stops_the_machine_with_the_drivers_off() {
    let items = job();
    let mut m = armed_machine(Pacing::Bytes);
    let mut h = auto();
    h.cancel_at_item = Some(5);
    assert_eq!(
        run_items(&mut m, &items, &mut h).unwrap(),
        Outcome::Cancelled
    );
    assert!(!m.is_armed());
    assert_eq!(*m.transport().sent.last().unwrap(), 0xC0);
    assert!(h.seen.len() <= 7);
}

#[test]
fn a_refused_dialog_cancels_the_job() {
    let items = job();
    let mut m = armed_machine(Pacing::Bytes);
    let mut h = auto();
    h.refuse_pause = true;
    assert_eq!(
        run_items(&mut m, &items, &mut h).unwrap(),
        Outcome::Cancelled
    );
    assert_eq!(*m.transport().sent.last().unwrap(), 0xC0);
}

#[test]
fn an_unarmed_machine_moves_nothing_and_the_error_stops_it() {
    let items = job();
    let mut m = Machine::new(Simulator::default());
    m.connect().unwrap();
    let before = m.transport().sent.len();
    let mut h = auto();
    assert!(run_items(&mut m, &items, &mut h).is_err());
    // nothing but the safety byte went out
    assert!(m.transport().sent[before..].iter().all(|&b| b == 0xC0));
}

// ---- reference searches that stop on the home line ----

use eh3d_drillpro_protocol::job::Item;
use eh3d_drillpro_protocol::safety::MachineError;
use eh3d_drillpro_protocol::{plan_line, Limits};

fn machine_with(limits: Option<Limits>, home_signal: bool) -> Machine<Simulator> {
    let mut sim = Simulator::default();
    sim.limits = limits;
    let mut m = Machine::new(sim);
    m.set_home_signal(home_signal);
    m.connect().unwrap();
    m.arm(Verified::all()).unwrap();
    m
}

#[test]
fn a_search_stops_within_a_piece_of_the_switches() {
    let mut m = machine_with(
        Some(Limits {
            x: -300,
            y: -300,
            z: 0,
        }),
        true,
    );
    let items = vec![Item::Search(plan_line(
        -10_000, -10_000, 1000.0, 2000.0, 100,
    ))];
    let mut h = auto();
    assert_eq!(
        run_items(&mut m, &items, &mut h).unwrap(),
        Outcome::Completed
    );
    let x = m.transport().x_steps;
    assert!((-420..=-300).contains(&x), "stopped at {x}");
}

#[test]
fn a_search_that_never_sees_the_home_line_fails_and_stops_the_machine() {
    let mut m = machine_with(None, true);
    let items = vec![Item::Search(plan_line(-500, -500, 1000.0, 2000.0, 50))];
    let mut h = auto();
    let err = run_items(&mut m, &items, &mut h).unwrap_err();
    assert!(matches!(err, MachineError::HomeNotFound));
    assert_eq!(*m.transport().sent.last().unwrap(), 0xC0);
}

#[test]
fn without_the_home_signal_a_search_is_sent_in_full() {
    let mut m = machine_with(None, false);
    let items = vec![Item::Search(plan_line(-500, -500, 1000.0, 2000.0, 50))];
    let mut h = auto();
    assert_eq!(
        run_items(&mut m, &items, &mut h).unwrap(),
        Outcome::Completed
    );
    assert_eq!(m.transport().x_steps, -500);
}

#[test]
fn a_machine_already_home_does_not_move_in_the_search() {
    // parked on its switches: the home line is up before the first step
    let mut m = machine_with(Some(Limits { x: 0, y: 0, z: 0 }), true);
    let before = m.transport().sent.len();
    let items = vec![Item::Search(plan_line(-500, -500, 1000.0, 2000.0, 50))];
    let mut h = auto();
    assert_eq!(
        run_items(&mut m, &items, &mut h).unwrap(),
        Outcome::Completed
    );
    assert_eq!(m.transport().sent.len(), before);
}

// ---- the reference run of a real machine goes axis by axis ----

#[test]
fn a_sequential_reference_run_homes_x_then_y_and_only_the_last_search_reads_the_home_line() {
    let mut m = JobMachine {
        geometry: Geometry::with_correction(0.9992, 0.9998, 0.8988),
        offset_mm: (0.0, 0.0),
        max_depth_mm: 37.3,
        base_mm: 12.0,
        board_mm: 1.6,
        distance_tool_board_mm: 16.0,
        xy_search_steps: 26_613,
    };
    m.geometry.sequential_homing = true;
    let items = plan_drill_job(&m, &[], &[]);
    // no diagonal byte (0x05) anywhere in the reference run
    let bytes = flatten_bytes(&items);
    assert!(!bytes.contains(&0x05) && !bytes.contains(&0x4F));
    // X goes first: its long run of 0x01 comes before the long run of 0x04
    let first_x = bytes.iter().position(|&b| b == 0x01).unwrap();
    let first_y = bytes.iter().position(|&b| b == 0x04).unwrap();
    assert!(first_x < first_y);
    // the searches that read the home line: only the Y approach of each reference run
    let searches = items
        .iter()
        .filter(|i| matches!(i, Item::Search(_)))
        .count();
    assert_eq!(searches, 4); // the long Y search and the Y approach, at the start and at the return
                             // it runs on the machine
    let mut machine = armed_machine(Pacing::Bytes);
    let mut h = auto();
    assert_eq!(
        run_items(&mut machine, &items, &mut h).unwrap(),
        Outcome::Completed
    );
}

#[test]
fn moves_of_z_alone_go_one_byte_at_a_time_even_on_a_paced_connection() {
    let mut m = armed_machine(Pacing::Stream);
    let before = m.transport().sent.len();
    let z: Vec<eh3d_drillpro_protocol::Step> = (0..50)
        .map(|_| eh3d_drillpro_protocol::Step {
            byte: 0x30,
            delay_us: 2500.0,
        })
        .collect();
    m.run(&z).unwrap();
    let sent = &m.transport().sent[before..];
    assert_eq!(sent.len(), 50);
    assert!(sent.iter().all(|&b| b == 0x30)); // no filler bytes
                                              // X and Y still go as a stream: filler bytes between the steps
    let before = m.transport().sent.len();
    let x: Vec<eh3d_drillpro_protocol::Step> = (0..50)
        .map(|_| eh3d_drillpro_protocol::Step {
            byte: 0x01,
            delay_us: 2500.0,
        })
        .collect();
    m.run(&x).unwrap();
    let sent = &m.transport().sent[before..];
    assert!(sent.len() > 50 && sent.contains(&0x00));
    // a diagonal with Z in the same byte is not a Z-only move
    let both: Vec<eh3d_drillpro_protocol::Step> = (0..10)
        .map(|_| eh3d_drillpro_protocol::Step {
            byte: 0x11,
            delay_us: 2500.0,
        })
        .collect();
    let before = m.transport().sent.len();
    m.run(&both).unwrap();
    assert!(m.transport().sent[before..].len() > 10);
}

// ---- the long searches are skipped when the machine is already home ----

#[test]
fn a_seek_is_skipped_when_the_home_line_is_already_up_and_run_otherwise() {
    let steps = plan_line(-500, 0, 1000.0, 2000.0, 50);
    // parked: all three switches closed, so the home line is up
    let mut m = machine_with(Some(Limits { x: 0, y: 0, z: 0 }), true);
    let before = m.transport().sent.len();
    let mut h = auto();
    run_items(&mut m, &[Item::Seek(steps.clone())], &mut h).unwrap();
    assert_eq!(
        m.transport().sent.len(),
        before,
        "a parked machine must not move"
    );
    // away from home: it is sent
    let mut m = machine_with(
        Some(Limits {
            x: -100_000,
            y: 0,
            z: 0,
        }),
        true,
    );
    let before = m.transport().sent.len();
    run_items(&mut m, &[Item::Seek(steps.clone())], &mut auto()).unwrap();
    assert!(m.transport().sent.len() > before);
    // without the home signal it is always sent
    let mut m = machine_with(Some(Limits { x: 0, y: 0, z: 0 }), false);
    let before = m.transport().sent.len();
    run_items(&mut m, &[Item::Seek(steps)], &mut auto()).unwrap();
    assert!(m.transport().sent.len() > before);
}

// ---- the plan follows the tuning ----

#[test]
fn the_plan_of_a_real_machine_follows_the_tuning() {
    use eh3d_drillpro_protocol::tuning::Tuning;
    let plan = |t: Tuning| {
        let mut m = JobMachine {
            geometry: Geometry::with_correction(0.9992, 0.9998, 0.8988)
                .with_speeds((4000.0, 1000.0, 1000.0), (5000.0, 1000.0, 1000.0)),
            offset_mm: (0.0, 0.0),
            max_depth_mm: 37.3,
            base_mm: 12.0,
            board_mm: 1.6,
            distance_tool_board_mm: 16.0,
            xy_search_steps: 26_613,
        };
        m.geometry.sequential_homing = true;
        m.geometry.tuning = t;
        plan_drill_job(&m, &[], &[])
    };
    let base = plan(Tuning::default());
    let fast = plan(Tuning {
        jump_peak_hz: 2800.0,
        ..Tuning::default()
    });
    let long = plan(Tuning {
        z_backoff_steps: 80,
        z_approach_steps: 90,
        jump_backoff_steps: 300,
        ..Tuning::default()
    });
    let seconds = |items: &[Item], byte: u8| -> f64 {
        items
            .iter()
            .filter_map(|i| {
                if let Item::Steps(s) | Item::Search(s) = i {
                    Some(s)
                } else {
                    None
                }
            })
            .flat_map(|s| s.iter())
            .filter(|s| s.byte == byte)
            .map(|s| s.delay_us)
            .sum::<f64>()
            / 1e6
    };
    // a higher cruising rate of the jump makes the jump legs shorter in time (X back-off uses the byte 0x43 only in the jump)
    assert!(seconds(&fast, 0x43) < seconds(&base, 0x43));
    // the step counts of the jump and of Z come from the tuning
    let count =
        |items: &[Item], byte: u8| flatten_bytes(items).iter().filter(|&&b| b == byte).count();
    assert!(count(&long, 0x43) > count(&base, 0x43));
    assert!(count(&long, 0x50) > count(&base, 0x50));
}

#[test]
fn z_pacing_stream_in_the_tuning_sends_z_as_a_stream_too() {
    use eh3d_drillpro_protocol::tuning::Tuning;
    let mut m = armed_machine(Pacing::Stream);
    m.set_tuning(Tuning {
        z_pacing_bytes: false,
        ..Tuning::default()
    });
    let before = m.transport().sent.len();
    let z: Vec<eh3d_drillpro_protocol::Step> = (0..50)
        .map(|_| eh3d_drillpro_protocol::Step {
            byte: 0x30,
            delay_us: 2500.0,
        })
        .collect();
    m.run(&z).unwrap();
    assert!(m.transport().sent.len() - before > 50);
}

// ---- leaving the switches when parked ----

fn real_job() -> Vec<Item> {
    let mut m = JobMachine {
        geometry: Geometry::with_correction(0.9992, 0.9998, 0.8988)
            .with_speeds((4000.0, 1000.0, 1000.0), (5000.0, 1000.0, 1000.0)),
        offset_mm: (0.0, 0.0),
        max_depth_mm: 37.3,
        base_mm: 12.0,
        board_mm: 1.6,
        distance_tool_board_mm: 16.0,
        xy_search_steps: 26_613,
    };
    m.geometry.sequential_homing = true;
    plan_drill_job(&m, &[], &[])
}

#[test]
fn a_parked_machine_leaves_its_switches_and_searches_the_short_distance_back() {
    // parked: all three switches closed, so the home line is up from the start
    let mut m = machine_with(Some(Limits { x: 0, y: 0, z: 0 }), true);
    let before = m.transport().sent.len();
    let mut h = auto();
    assert_eq!(
        run_items(&mut m, &real_job(), &mut h).unwrap(),
        Outcome::Completed
    );
    let sent = &m.transport().sent[before..];
    // X went away first (0x43), then back; nothing like the 22000 + 17000 steps of the full searches
    assert!(
        sent.contains(&0x43) && sent.contains(&0x4C),
        "X and Y must have moved away"
    );
    assert!(
        sent.len() < 20_000,
        "{} bytes: the searches were not short",
        sent.len()
    );
    // and it ends home again
    let t = m.transport();
    assert!(t.x_steps <= 0 && t.y_steps <= 0);
}

#[test]
fn a_machine_whose_home_line_is_down_does_not_leave_anything_and_searches_in_full() {
    let mut m = machine_with(
        Some(Limits {
            x: -100,
            y: -100,
            z: -100,
        }),
        true,
    );
    let before = m.transport().sent.len();
    let mut h = auto();
    // the machine is away from home: the line is down and the positions are unknown
    assert_eq!(
        run_items(&mut m, &real_job(), &mut h).unwrap(),
        Outcome::Completed
    );
    let sent = &m.transport().sent[before..];
    // the first X movement is the search towards home (0x01), not a move away (0x43)
    let first_x = sent.iter().position(|&b| b == 0x01 || b == 0x43).unwrap();
    assert_eq!(sent[first_x], 0x01);
    assert!(
        sent.len() > 22_000,
        "{} bytes: the full X search was expected",
        sent.len()
    );
}

#[test]
fn unstick_off_in_the_tuning_means_no_move_away() {
    use eh3d_drillpro_protocol::tuning::Tuning;
    let mut job = real_job();
    job.retain(|i| !matches!(i, Item::Unstick(_)));
    assert!(!job.iter().any(|i| matches!(i, Item::Unstick(_))));
    // with unstick_mm = 0 the planner does not even put the step in the plan
    let mut m = JobMachine {
        geometry: Geometry::with_correction(1.0, 1.0, 1.0),
        offset_mm: (0.0, 0.0),
        max_depth_mm: 37.3,
        base_mm: 12.0,
        board_mm: 1.6,
        distance_tool_board_mm: 16.0,
        xy_search_steps: 26_613,
    };
    m.geometry.sequential_homing = true;
    m.geometry.tuning = Tuning {
        unstick_mm: 0.0,
        ..Tuning::default()
    };
    assert!(!plan_drill_job(&m, &[], &[])
        .iter()
        .any(|i| matches!(i, Item::Unstick(_))));
}
