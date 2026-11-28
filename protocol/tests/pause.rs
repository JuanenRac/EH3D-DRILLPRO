//! The pause the operator asks for: only at the end of a hole, with the spindle left as it was or stopped and brought back exactly, and the link
//! kept alive while the program waits. All of it on the simulator: not tried on a machine.

use std::cell::Cell;
use std::io;
use std::rc::Rc;

use eh3d_drillpro_protocol::execute::{run_items_resting, Hooks, Outcome, Rest};
use eh3d_drillpro_protocol::job::{
    plan_drill_job_marked, Item, JobHole, JobMachine, JobTool, Pause,
};
use eh3d_drillpro_protocol::latches::{A_SPINDLE_RUN, B_DRIVERS};
use eh3d_drillpro_protocol::motion::Geometry;
use eh3d_drillpro_protocol::safety::{Machine, Verified};
use eh3d_drillpro_protocol::tuning::Tuning;
use eh3d_drillpro_protocol::{ModemStatus, Pacing, Simulator, Transport};

/// The simulator that also counts the times it is told that the program is still there.
struct Counting {
    sim: Simulator,
    alive: Rc<Cell<u32>>,
}

impl Transport for Counting {
    fn set_dtr(&mut self, on: bool) -> io::Result<()> {
        self.sim.set_dtr(on)
    }
    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        self.sim.write_byte(byte)
    }
    fn status(&mut self) -> io::Result<ModemStatus> {
        self.sim.status()
    }
    fn sleep_us(&mut self, us: u64) {
        self.sim.sleep_us(us)
    }
    fn keep_alive(&mut self) -> io::Result<()> {
        self.alive.set(self.alive.get() + 1);
        Ok(())
    }
}

fn plan() -> (Vec<Item>, Vec<usize>) {
    let m = JobMachine {
        geometry: Geometry::with_correction(0.9992, 0.9998, 0.8988),
        offset_mm: (2.5, 2.5),
        max_depth_mm: 37.3,
        base_mm: 12.0,
        board_mm: 1.6,
        distance_tool_board_mm: 16.0,
        xy_search_steps: 26_613,
    };
    let tools = [JobTool {
        number: 1,
        extra_stroke_mm: 0.5,
        feed_mm_min: 1000,
        spindle_rpm: 42_000,
    }];
    let holes = [10.0, 20.0, 30.0].map(|x_mm| JobHole {
        tool: 1,
        x_mm,
        y_mm: 10.0,
    });
    plan_drill_job_marked(&m, &tools, &holes, &[], None)
}

struct Operator {
    /// Items done so far, as `progress` told it.
    at: usize,
    /// The pause is asked once this item is reached.
    ask_from: Option<usize>,
    /// The item after which each pause held.
    held_after: Vec<(usize, Rest)>,
    refuse: Option<Rest>,
}

impl Operator {
    fn asking_from(item: usize) -> Self {
        Operator {
            at: 0,
            ask_from: Some(item),
            held_after: Vec::new(),
            refuse: None,
        }
    }
}

impl Hooks for Operator {
    fn pause(&mut self, _p: Pause) -> bool {
        true
    }
    fn progress(&mut self, next: usize, _total: usize) {
        self.at = next;
    }
    fn cancelled(&mut self) -> bool {
        false
    }
    fn pause_requested(&mut self) -> bool {
        match self.ask_from {
            Some(from) if self.at >= from => {
                self.ask_from = None;
                true
            }
            _ => false,
        }
    }
    fn rest(&mut self, rest: Rest, idle: &mut dyn FnMut()) -> bool {
        self.held_after.push((self.at, rest));
        for _ in 0..3 {
            idle();
        }
        self.refuse != Some(rest)
    }
}

fn run(stops_spindle: bool, operator: &mut Operator) -> (Outcome, Vec<u8>, u32) {
    let (items, marks) = plan();
    let alive = Rc::new(Cell::new(0));
    let mut m = Machine::new(Counting {
        sim: Simulator::default(),
        alive: alive.clone(),
    });
    m.set_pacing(Pacing::Bytes);
    m.set_tuning(Tuning {
        pause_stops_spindle: stops_spindle,
        ..Tuning::default()
    });
    m.connect().unwrap();
    m.arm(Verified::all()).unwrap();
    let before = m.transport().sim.sent.len();
    let outcome = run_items_resting(&mut m, &items, &marks, operator).unwrap();
    let sent = m.transport().sim.sent[before..].to_vec();
    (outcome, sent, alive.get())
}

/// The last byte written to each latch after the first `n` bytes.
fn latches(sent: &[u8]) -> (u8, u8) {
    let last = |mask: u8| sent.iter().rev().copied().find(|&b| b & 0xC0 == mask);
    (last(0x80).unwrap_or(0x80), last(0xC0).unwrap_or(0xC0))
}

#[test]
fn a_pause_asked_in_the_middle_waits_for_the_end_of_the_hole() {
    let (_, marks) = plan();
    // asked while the first hole is being drilled
    let mut operator = Operator::asking_from(marks[0]);
    let (outcome, sent, alive) = run(false, &mut operator);
    assert_eq!(outcome, Outcome::Completed);
    assert_eq!(
        operator.held_after,
        [(
            marks[0],
            Rest::Paused {
                spindle_stopped: false
            }
        )]
    );
    // the bytes are those of the job without a pause
    let (_, plain, _) = run(false, &mut Operator::asking_from(usize::MAX));
    assert_eq!(sent, plain);
    assert!(alive >= 3, "the link was kept alive while waiting: {alive}");
}

#[test]
fn a_pause_asked_before_the_first_hole_holds_after_it_and_not_before() {
    let (_, marks) = plan();
    let mut operator = Operator::asking_from(0);
    // the planner's first safe point is the first one the job passes
    let (outcome, _, _) = run(false, &mut operator);
    assert_eq!(outcome, Outcome::Completed);
    assert_eq!(operator.held_after.len(), 1);
    assert_eq!(operator.held_after[0].0, marks[0]);
}

#[test]
fn a_pause_that_stops_the_spindle_brings_it_back_exactly_and_asks_twice() {
    let (_, marks) = plan();
    let mut operator = Operator::asking_from(marks[1]);
    let (outcome, sent, _) = run(true, &mut operator);
    assert_eq!(outcome, Outcome::Completed);
    assert_eq!(
        operator.held_after,
        [
            (
                marks[1],
                Rest::Paused {
                    spindle_stopped: true
                }
            ),
            (marks[1], Rest::SpindleUp)
        ]
    );
    let (_, plain, _) = run(false, &mut Operator::asking_from(usize::MAX));
    // the job is the same up to the pause, then come the stop and the start, then the job goes on as it was
    let extra = sent.len() - plain.len();
    let at = plain
        .iter()
        .zip(&sent)
        .position(|(a, b)| a != b)
        .expect("the stop and the start are in it");
    assert_eq!(sent[at + extra..], plain[at..]);
    assert_eq!(latches(&sent[..at + extra]), latches(&plain[..at]));
    // in between the run bit was released and the drivers stayed on
    let between = &sent[at..at + extra];
    assert!(between
        .iter()
        .any(|&b| b & 0xC0 == 0x80 && b & A_SPINDLE_RUN == 0));
    assert!(between
        .iter()
        .filter(|&&b| b & 0xC0 == 0xC0)
        .all(|&b| b & B_DRIVERS != 0));
    assert!(
        between.iter().all(|&b| b & 0x80 != 0),
        "no motor moves during the pause"
    );
}

#[test]
fn stopping_while_paused_stops_the_machine() {
    let (_, marks) = plan();
    for (stops_spindle, refuse) in [
        (
            false,
            Rest::Paused {
                spindle_stopped: false,
            },
        ),
        (
            true,
            Rest::Paused {
                spindle_stopped: true,
            },
        ),
        (true, Rest::SpindleUp),
    ] {
        let mut operator = Operator::asking_from(marks[0]);
        operator.refuse = Some(refuse);
        let (outcome, sent, _) = run(stops_spindle, &mut operator);
        assert_eq!(outcome, Outcome::Cancelled, "{refuse:?}");
        let (a, b) = latches(&sent);
        assert_eq!(a & A_SPINDLE_RUN, 0, "{refuse:?}: the spindle is released");
        assert_eq!(b, 0xC0, "{refuse:?}: the drivers are off");
    }
}
