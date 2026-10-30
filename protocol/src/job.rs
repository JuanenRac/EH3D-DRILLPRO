//! Complete drilling job, in the order the original program sends it.
//!
//! Reproduces, byte for byte, the traffic recorded from the original program (see
//! `tests/job_golden.rs`): drivers on, Z and XY homing, move to the first hole, tool change, spindle
//! start, the holes with their strokes at the travel height, spindle stop, and the return home.
//! Operator dialogs appear as [`Item::Pause`]; timing is only approximate.

use crate::latches::{spindle_level, Latches, Op};
use crate::motion::{z_levels, Geometry, ZParams};
use crate::{bits, plan_line, Step};

/// Machine data the job needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JobMachine {
    pub geometry: Geometry,
    /// Offset added to every hole position, in mm.
    pub offset_mm: (f64, f64),
    pub max_depth_mm: f64,
    pub base_mm: f64,
    pub board_mm: f64,
    pub distance_tool_board_mm: f64,
    /// Longest diagonal XY search of the homing, in steps (26613 and 24019 on the two machines
    /// recorded; the original derives it from the configuration, the rule is not known).
    pub xy_search_steps: u32,
}

/// The rack of a machine with the automatic tool changer: the place of each tool as (X, Y, Z) in mm. Place 0 is the test pin, 1 to
/// 14 are the tools (15 holds the drill break detector).
#[derive(Debug, Clone, PartialEq)]
pub struct Rack {
    pub places: Vec<(f64, f64, f64)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JobTool {
    pub number: u8,
    pub extra_stroke_mm: f64,
    pub feed_mm_min: u32,
    pub spindle_rpm: u32,
}

/// A hole in file coordinates (mm, before the offset).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JobHole {
    pub tool: u8,
    pub x_mm: f64,
    pub y_mm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pause {
    /// The operator must insert the tool and confirm.
    ToolChange { tool: u8 },
    /// The operator confirms the spindle start.
    SpindleStart,
}

/// The move of the "unstick" step: the two legs and how far each axis is from its switch afterwards.
#[derive(Debug, Clone, PartialEq)]
pub struct Unstick {
    pub x: Vec<Step>,
    pub y: Vec<Step>,
    /// Steps each axis must go back to reach its switch again (the distance moved plus the margin of the tuning).
    pub x_back: u32,
    pub y_back: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Steps(Vec<Step>),
    /// Steps towards the reference switches that stop as soon as the home line rises (when the machine reports it);
    /// otherwise they are sent in full and the control unit swallows the surplus at the switches.
    Search(Vec<Step>),
    /// A long search towards a switch that is skipped when the home line is already up (all three axes are on their switches, as
    /// when the machine is parked): the original goes straight to the calibration jump then. Without the home signal it is sent in full.
    Seek(Vec<Step>),
    /// Parked on the switches (the home line is up): X and Y leave them by a known distance, so the searches that follow can be short.
    /// Skipped when the line is down (positions unknown).
    Unstick(Unstick),
    Latch(Vec<Op>),
    /// The spindle of the node (a variable-frequency drive on the node's own outputs): speed in rpm (0 stops it) and how long to wait
    /// for it to reach that speed or to stop. Used instead of the control unit's spindle latches when the tuning says `spindle = node`.
    Spindle {
        rpm: u32,
        wait_ms: u32,
    },
    Pause(Pause),
}

/// One leg of the calibration jump: `count` steps of `byte`, smooth at both ends (the values are in [`crate::tuning::Tuning`]).
fn jump_leg(
    byte: u8,
    count: u32,
    reference: f64,
    max: f64,
    t: &crate::tuning::Tuning,
) -> Vec<Step> {
    let peak = t.jump_peak_hz.min(max);
    let start = (reference * t.jump_start_fraction).min(peak);
    ramped(
        byte,
        count,
        start,
        peak,
        (count / t.jump_ramp_parts.max(1)).max(1),
    )
}

/// Steps of constant rate, used by the homing searches.
fn run(byte: u8, count: u32, hz: f64) -> Vec<Step> {
    (0..count)
        .map(|_| Step {
            byte,
            delay_us: 1e6 / hz,
        })
        .collect()
}

/// `count` steps of one byte, accelerating from `start_hz` to `hz` over `ramp` steps at the start and slowing down the same way at the end.
fn ramped(byte: u8, count: u32, start_hz: f64, hz: f64, ramp: u32) -> Vec<Step> {
    (0..count)
        .map(|i| {
            let k = i.min(count - 1 - i).min(ramp) as f64;
            let r = if ramp == 0 { 1.0 } else { k / ramp as f64 };
            let f = start_hz.min(hz) + (hz - start_hz.min(hz)) * r;
            Step {
                byte,
                delay_us: 1e6 / f,
            }
        })
        .collect()
}

const XY_BACKOFF: u32 = 200;
const XY_REAPPROACH: u32 = 250;

struct Planner<'a> {
    m: &'a JobMachine,
    items: Vec<Item>,
    latches: Latches,
    /// Position in steps from the origin.
    pos: (i32, i32),
    /// Z depth in steps below home.
    z: u32,
    /// The tool changer of the ATC models: where the tools wait (see [`Rack`]).
    rack: Option<Rack>,
}

impl Planner<'_> {
    fn latch(&mut self, seq: crate::latches::Sequence) {
        self.latches = seq.next;
        self.items.push(Item::Latch(seq.ops));
    }

    /// Spindle on: the control unit's latches, or the node's spindle when the tuning says so.
    fn spindle_start(&mut self, rpm: u32) {
        let t = self.m.geometry.tuning;
        if t.spindle_node {
            self.items.push(Item::Spindle {
                rpm,
                wait_ms: t.spindle_start_wait_ms,
            });
        } else {
            let s = self
                .latches
                .spindle_on(spindle_level(rpm), t.spindle_start_wait_ms as u64 * 1000);
            self.latch(s);
            if t.vacuum_aux {
                let v = self.latches.vacuum_on(spindle_level(rpm));
                self.latch(v);
            }
        }
    }

    /// Spindle off, and the drivers on again.
    fn spindle_stop(&mut self) {
        let t = self.m.geometry.tuning;
        if t.spindle_node {
            self.items.push(Item::Spindle {
                rpm: 0,
                wait_ms: t.spindle_stop_wait_ms,
            });
        } else {
            let off = self
                .latches
                .spindle_off(t.spindle_stop_wait_ms as u64 * 1000);
            self.latch(off);
        }
        // the control unit's own stop already ends with the drivers on; the original sends them again only on the manual models
        if self.rack.is_none() {
            let on = self.latches.drivers_on();
            self.latch(on);
        }
    }

    /// The chuck exchange of the tool changer: to the place `from` of the rack, Z down, chuck open (auxiliary output), a slide to the
    /// place `to`, chuck closed, Z up. As the original does it (recorded on the automatic models).
    fn chuck_swap(&mut self, from: usize, to: usize) {
        let Some(rack) = self.rack.clone() else {
            return;
        };
        let g = self.m.geometry;
        let place = |i: usize| {
            let (x, y, z) = rack.places.get(i).copied().unwrap_or((0.0, 0.0, 0.0));
            ((g.x_steps(x), g.y_steps(y)), g.z_steps(z))
        };
        let ((from_xy, z_from), (to_xy, _)) = (place(from), place(to));
        self.goto_xy(from_xy);
        self.z_down(z_from, g.z_max_hz);
        let open = self.latches.aux(true);
        self.latch(open);
        self.goto_xy(to_xy);
        let close = self.latches.aux(false);
        self.latch(close);
        self.z_up(z_from, g.z_max_hz);
    }

    fn goto_xy(&mut self, target: (i32, i32)) {
        let g = &self.m.geometry;
        let steps = plan_line(
            target.0 - self.pos.0,
            target.1 - self.pos.1,
            g.start_hz,
            g.max_hz,
            g.ramp_steps,
        );
        if !steps.is_empty() {
            self.items.push(Item::Steps(steps));
        }
        self.pos = target;
    }

    fn z_down(&mut self, steps: u32, hz: f64) {
        if steps > 0 {
            self.items.push(Item::Steps(run(bits::Z_POS, steps, hz)));
            self.z += steps;
        }
    }

    fn z_up(&mut self, steps: u32, hz: f64) {
        let steps = steps.min(self.z);
        if steps > 0 {
            self.items.push(Item::Steps(run(bits::Z_NEG, steps, hz)));
            self.z -= steps;
        }
    }

    /// Homing of Z: up to the switch at the Z maximum speed (with a ramp), then the second cycle at the reference speed:
    /// down a little and up again, to remove the switch's play. The control unit stops the axis at the switch itself.
    fn home_z(&mut self) {
        let g = &self.m.geometry;
        let t = &g.tuning;
        // The whole travel of Z, with a margin: the control unit swallows the surplus once the axis is on its switch.
        let search = t.z_search_steps.max(g.z_steps(self.m.max_depth_mm));
        let start = g.z_start_hz.min(t.z_start_hz);
        let top = g.z_max_hz.min(t.z_search_hz).max(start);
        self.items.push(Item::Seek(ramped(
            bits::Z_NEG,
            search,
            start,
            top,
            t.z_ramp_steps,
        )));
        self.items
            .push(Item::Steps(run(bits::Z_POS, t.z_backoff_steps, g.z_ref_hz)));
        self.items.push(Item::Steps(run(
            bits::Z_NEG,
            t.z_approach_steps,
            g.z_ref_hz,
        )));
        self.z = 0;
    }

    /// The reference run of X and Y with its two cycles. The recorded original goes diagonally (`0x05` for the search); the
    /// real machine is homed axis by axis, as the original does on it: X to its switch, then Y, then the second cycle of each
    /// (back off, approach again, smoothly). Only the last search reads the home line (RI: all three axes home).
    fn xy_search(&mut self) {
        let g = self.m.geometry;
        let t = g.tuning;
        let (start, max, reference) = (g.start_hz, g.max_hz, g.ref_hz);
        if g.sequential_homing {
            let max_x = max.min(t.x_search_hz);
            let max = max.min(t.y_search_hz);
            let start = start.min(max);
            let (nx, ny) = (g.search_x_steps, g.search_y_steps);
            let ramp = t.search_ramp_steps;
            if t.unstick_mm > 0.0 {
                let (ux, uy) = (
                    g.x_steps(t.unstick_mm).unsigned_abs().max(1),
                    g.y_steps(t.unstick_mm).unsigned_abs().max(1),
                );
                self.items.push(Item::Unstick(Unstick {
                    x: jump_leg(bits::X_POS, ux, reference, max, &t),
                    y: jump_leg(bits::Y_POS, uy, reference, max, &t),
                    x_back: ux + t.search_hint_margin_steps,
                    y_back: uy + t.search_hint_margin_steps,
                }));
            }
            self.items
                .push(Item::Seek(ramped(bits::X_NEG, nx, start, max_x, ramp)));
            self.items
                .push(Item::Search(ramped(bits::Y_NEG, ny, start, max, ramp)));
            let (back, again) = (t.jump_backoff_steps, t.jump_approach_steps);
            self.items
                .push(Item::Steps(jump_leg(bits::X_POS, back, reference, max, &t)));
            self.items.push(Item::Steps(jump_leg(
                bits::X_NEG,
                again,
                reference,
                max,
                &t,
            )));
            self.items
                .push(Item::Steps(jump_leg(bits::Y_POS, back, reference, max, &t)));
            self.items.push(Item::Search(jump_leg(
                bits::Y_NEG,
                again,
                reference,
                max,
                &t,
            )));
            return;
        }
        let n = self.m.xy_search_steps as i32;
        self.items.push(Item::Search(plan_line(
            -n,
            -n,
            start,
            max,
            g.ramp_steps.max(150),
        )));
        let (b, r) = (XY_BACKOFF as i32, XY_REAPPROACH as i32);
        self.items
            .push(Item::Steps(plan_line(b, b, reference, reference, 0)));
        self.items
            .push(Item::Search(plan_line(-r, -r, reference, reference, 0)));
    }

    fn abs_steps(&self, hole: &JobHole) -> (i32, i32) {
        let g = &self.m.geometry;
        (
            g.x_steps(hole.x_mm + self.m.offset_mm.0),
            g.y_steps(hole.y_mm + self.m.offset_mm.1),
        )
    }
}

/// Builds a job whose moves are not holes (ROUTEPRO's paths): the start (drivers on and the reference runs), the spindle, the
/// operator's dialogs and the return are the same as in a drilling job.
pub struct JobBuilder<'a> {
    p: Planner<'a>,
}

impl<'a> JobBuilder<'a> {
    /// Drivers on (sent twice) and the reference runs of Z and of X and Y; the head is then at the origin, Z at the top.
    pub fn new(m: &'a JobMachine) -> Self {
        let mut p = Planner {
            m,
            items: Vec::new(),
            latches: Latches::default(),
            rack: None,
            pos: (0, 0),
            z: 0,
        };
        let on = p.latches.drivers_on();
        p.latch(on.clone());
        p.latch(on);
        p.home_z();
        p.xy_search();
        p.pos = (0, 0);
        Self { p }
    }

    pub fn pause(&mut self, pause: Pause) {
        self.p.items.push(Item::Pause(pause));
    }

    /// What has been planned so far, without the end of the job: for a session that homes the machine and then goes on by hand.
    pub fn into_items(self) -> Vec<Item> {
        self.p.items
    }

    /// Spindle on at `rpm` (it waits for the spindle to reach speed).
    pub fn spindle_on(&mut self, rpm: u32) {
        self.p.spindle_start(rpm);
    }

    /// Spindle off (brake, release) and drivers on again.
    pub fn spindle_off(&mut self) {
        self.p.spindle_stop();
    }

    /// Moves Z down from the top to `steps` below it (or up to it) at the Z maximum speed.
    pub fn z_to(&mut self, steps: u32) {
        let hz = self.p.m.geometry.z_max_hz;
        if steps > self.p.z {
            let d = steps - self.p.z;
            self.p.z_down(d, hz);
        } else {
            self.p.z_up(self.p.z - steps, hz);
        }
    }

    /// Steps that leave the head at `end` (in steps from the origin) and Z where it was.
    pub fn steps(&mut self, steps: Vec<Step>, end: (i32, i32)) {
        if !steps.is_empty() {
            self.p.items.push(Item::Steps(steps));
        }
        self.p.pos = end;
    }

    /// Moves the head to `target` (steps from the origin) with the XY ramp.
    pub fn goto_xy(&mut self, target: (i32, i32)) {
        self.p.goto_xy(target);
    }

    /// The end of the job: Z up to the top, the fine Z reference again, back to the origin, the XY reference and drivers off.
    pub fn finish(mut self) -> Vec<Item> {
        let all = self.p.z;
        let hz = self.p.m.geometry.z_max_hz;
        self.p.z_up(all, hz);
        let z_ref = self.p.m.geometry.z_ref_hz;
        self.p.items.push(Item::Steps(run(
            bits::Z_POS,
            self.p.m.geometry.tuning.z_backoff_steps,
            z_ref,
        )));
        self.p.items.push(Item::Steps(run(
            bits::Z_NEG,
            self.p.m.geometry.tuning.z_approach_steps,
            z_ref,
        )));
        self.p.goto_xy((0, 0));
        self.p.xy_search();
        let off = self.p.latches.drivers_off();
        self.p.latch(off);
        self.p.items
    }
}

/// Plans a drilling job. `holes` must already be in drilling order, grouped by tool in the order
/// the tools are used; `tools` gives the data of each tool number.
pub fn plan_drill_job(m: &JobMachine, tools: &[JobTool], holes: &[JobHole]) -> Vec<Item> {
    plan_job(m, tools, holes, None, &[])
}

/// The same job on a table that is not flat: `lifts_mm[i]` is how much higher the table is under hole `i` than at the place the Z was set (negative where it is
/// lower), and the drill goes that much less (or more) deep there, so every hole has the same depth into the board. Holes without a lift are drilled as they are.
pub fn plan_drill_job_levelled(
    m: &JobMachine,
    tools: &[JobTool],
    holes: &[JobHole],
    lifts_mm: &[f64],
    rack: Option<&Rack>,
) -> Vec<Item> {
    plan_job(m, tools, holes, rack.cloned(), lifts_mm)
}

/// The same job on a machine with the automatic tool changer: the tools wait in the rack and the head changes them itself (the
/// chuck opens with the auxiliary output). Tool number N of the job is taken from place N of the rack, in the order the tools are
/// used (place 1 for the first tool used, 2 for the second, ...); place 0 is the test pin, where the cycle starts and ends.
pub fn plan_drill_job_atc(
    m: &JobMachine,
    tools: &[JobTool],
    holes: &[JobHole],
    rack: &Rack,
) -> Vec<Item> {
    plan_job(m, tools, holes, Some(rack.clone()), &[])
}

fn plan_job(
    m: &JobMachine,
    tools: &[JobTool],
    holes: &[JobHole],
    rack: Option<Rack>,
    lifts_mm: &[f64],
) -> Vec<Item> {
    let mut p = Planner {
        m,
        items: Vec::new(),
        latches: Latches::default(),
        pos: (0, 0),
        z: 0,
        rack,
    };
    let g = m.geometry;

    // Start: drivers on (sent twice), then homing.
    let on = p.latches.drivers_on();
    p.latch(on.clone());
    p.latch(on);
    p.home_z();
    p.xy_search();
    p.pos = (0, 0);

    // Tool groups in order of first use.
    let mut order: Vec<u8> = Vec::new();
    for h in holes {
        if !order.contains(&h.tool) {
            order.push(h.tool);
        }
    }

    for (gi, &tool_no) in order.iter().enumerate() {
        let tool = tools
            .iter()
            .copied()
            .find(|t| t.number == tool_no)
            .unwrap_or(JobTool {
                number: tool_no,
                extra_stroke_mm: 0.0,
                feed_mm_min: 1000,
                spindle_rpm: 60_000,
            });
        let group: Vec<(&JobHole, f64)> = holes
            .iter()
            .enumerate()
            .filter(|(_, h)| h.tool == tool_no)
            .map(|(i, h)| (h, lifts_mm.get(i).copied().unwrap_or(0.0)))
            .collect();
        let first = p.abs_steps(group[0].0);
        let levels = z_levels(
            &ZParams {
                max_depth_mm: m.max_depth_mm,
                base_mm: m.base_mm,
                board_mm: m.board_mm,
                distance_tool_board_mm: m.distance_tool_board_mm,
                extra_stroke_mm: tool.extra_stroke_mm,
            },
            &g,
        );
        let feed_hz = g.z_hz(tool.feed_mm_min as f64);
        let fast_hz = g.z_max_hz;

        if p.rack.is_some() {
            // The tool changer: from the place of the tool that was in the chuck (place 0, the test pin, at the start) to the place of
            // this one, then to the first hole; the drivers go on again before the spindle starts.
            p.chuck_swap(gi, gi + 1);
            p.goto_xy(first);
            let on = p.latches.drivers_on();
            p.latch(on);
            if gi == 0 {
                p.items.push(Item::Pause(Pause::SpindleStart));
            }
            p.spindle_start(tool.spindle_rpm);
        } else if gi == 0 {
            // The first tool: go to the first hole, then confirm the tool, drivers on again.
            p.goto_xy(first);
            p.items
                .push(Item::Pause(Pause::ToolChange { tool: tool_no }));
            let on = p.latches.drivers_on();
            p.latch(on);
            p.items.push(Item::Pause(Pause::SpindleStart));
            p.spindle_start(tool.spindle_rpm);
        } else {
            // Later tools: the operator changes the tool where the head stopped, the spindle starts,
            // then the head goes to the first hole.
            p.items
                .push(Item::Pause(Pause::ToolChange { tool: tool_no }));
            p.spindle_start(tool.spindle_rpm);
            p.goto_xy(first);
        }

        for (hi, (h, lift)) in group.iter().enumerate() {
            if hi > 0 {
                let t = p.abs_steps(h);
                p.goto_xy(t);
            }
            // where the table is higher the hole is not as deep in steps from home, where it is lower it is deeper
            let shift = g.z_steps(lift.abs()) as i64 * if *lift >= 0.0 { -1 } else { 1 };
            let end = (levels.end_steps as i64 + shift).max(0) as u32;
            if end >= p.z {
                p.z_down(end - p.z, feed_hz);
            } else {
                p.z_up(p.z - end, fast_hz);
            }
            if hi + 1 == group.len() {
                let all = p.z;
                p.z_up(all, fast_hz);
            } else {
                p.z_up(levels.stroke_steps, fast_hz);
            }
        }

        // Spindle stop (brake, release run, release brake) and drivers on again.
        p.spindle_stop();
    }

    // The last tool goes back to its place and the chuck is left closed on the test pin's place.
    if p.rack.is_some() {
        p.chuck_swap(order.len(), 0);
        let on = p.latches.drivers_on();
        p.latch(on);
    }

    // Return: Z fine search again, back to the origin, XY search, drivers off.
    let z_ref = p.m.geometry.z_ref_hz;
    p.items.push(Item::Steps(run(
        bits::Z_POS,
        p.m.geometry.tuning.z_backoff_steps,
        z_ref,
    )));
    p.items.push(Item::Steps(run(
        bits::Z_NEG,
        p.m.geometry.tuning.z_approach_steps,
        z_ref,
    )));
    p.goto_xy((0, 0));
    p.xy_search();
    let off = p.latches.drivers_off();
    p.latch(off);
    p.items
}

/// Every byte the plan sends, in order (pauses and waits omitted).
pub fn flatten_bytes(items: &[Item]) -> Vec<u8> {
    let mut out = Vec::new();
    for it in items {
        match it {
            Item::Steps(s) | Item::Search(s) | Item::Seek(s) => {
                out.extend(s.iter().map(|x| x.byte))
            }
            Item::Latch(ops) => {
                out.extend(
                    ops.iter()
                        .filter_map(|o| if let Op::Send(b) = o { Some(*b) } else { None }),
                )
            }
            // pauses are not bytes, and the unstick only happens when the machine is parked (it depends on the home line)
            Item::Pause(_) | Item::Unstick(_) | Item::Spindle { .. } => {}
        }
    }
    out
}

/// Run-length view of a byte sequence, for comparing against recorded traffic.
pub fn run_lengths(bytes: &[u8]) -> Vec<(u8, usize)> {
    let mut v: Vec<(u8, usize)> = Vec::new();
    for &b in bytes {
        match v.last_mut() {
            Some((x, n)) if *x == b => *n += 1,
            _ => v.push((b, 1)),
        }
    }
    v
}
