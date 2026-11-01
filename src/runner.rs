//! Runs a planned job on the machine chosen in the connection file, on its own thread, so the screen keeps
//! drawing and the Escape key keeps working while the machine moves. Shared by DRILLPRO and ROUTEPRO.
//!
//! The thread opens the link, does the handshake, arms the machine (choosing a real connection in the connection
//! file is the operator's statement that the machine is ready to move), runs the items and reports its state. A dialog
//! for the operator (tool change, spindle start) waits here until [`RealRun::confirm`]; [`RealRun::cancel`] stops the
//! machine with the drivers off.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::protocol::connection::ConnectionConfig;
use crate::protocol::execute::{run_items, Hooks, Outcome};
use crate::protocol::job::{Item, JobBuilder, JobMachine, Pause};
use crate::protocol::motion::{rapid_xy, z_move};
use crate::protocol::safety::{Machine, Verified};
use crate::server_link::path_segment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunState {
    /// Opening the link and shaking hands with the control unit.
    Connecting,
    Running,
    /// A dialog for the operator; [`RealRun::confirm`] continues.
    WaitPause(Pause),
    Done,
    /// Stopped by the operator.
    Cancelled,
    /// The link or the machine failed; the machine was stopped. The text says why (in English).
    Failed(String),
}

impl RunState {
    pub fn finished(&self) -> bool {
        matches!(
            self,
            RunState::Done | RunState::Cancelled | RunState::Failed(_)
        )
    }
}

/// What the server says of a job beyond its state: the excesses of the spindle it cut (or would have refused) and the node of the drill head, when the machine has one.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServerNotes {
    /// One short text per excess, for example `T1 60000>24000 rpm`.
    pub spindle_warnings: Vec<String>,
    /// The turns of the spindle and the temperature the node of the head reports.
    pub head_rpm: Option<f64>,
    pub head_temp_c: Option<f64>,
}

impl ServerNotes {
    /// The excesses of the spindle in one line, or `None` when there are none.
    pub fn spindle_line(&self) -> Option<String> {
        (!self.spindle_warnings.is_empty()).then(|| self.spindle_warnings.join("; "))
    }

    /// What the head reports, in one line (`18000 rpm  35.2 °C`), or `None` while it has said nothing.
    pub fn head_line(&self) -> Option<String> {
        match (self.head_rpm, self.head_temp_c) {
            (None, None) => None,
            (rpm, temp) => Some(
                [
                    rpm.map(|r| format!("{r:.0} rpm")),
                    temp.map(|t| format!("{t:.1} \u{b0}C")),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("  "),
            ),
        }
    }
}

/// The excesses of the spindle a job's view lists (`spindle_warnings`), each as a short text.
pub fn spindle_notes(view: &serde_json::Value) -> Vec<String> {
    view["spindle_warnings"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|w| {
                    let (tool, value, limit) = (
                        w["tool"].as_u64().unwrap_or(0),
                        w["value"].as_u64().unwrap_or(0),
                        w["limit"].as_u64().unwrap_or(0),
                    );
                    let (sign, unit) = match w["code"].as_str().unwrap_or("") {
                        "rpm_above_max" => (">", "rpm"),
                        "rpm_below_min" => ("<", "rpm"),
                        _ => (">", "mm/min"),
                    };
                    format!("T{tool} {value}{sign}{limit} {unit}")
                })
                .collect()
        })
        .unwrap_or_default()
}

struct Shared {
    notes: Mutex<ServerNotes>,
    state: Mutex<RunState>,
    next: AtomicUsize,
    total: AtomicUsize,
    cancel: AtomicBool,
    confirmed: AtomicBool,
    /// A job on the server goes on when this program closes; only the following of it ends.
    stop_following: AtomicBool,
}

impl Shared {
    fn set(&self, s: RunState) {
        if let Ok(mut g) = self.state.lock() {
            *g = s;
        }
    }
}

struct WorkerHooks(Arc<Shared>);

impl Hooks for WorkerHooks {
    fn pause(&mut self, pause: Pause) -> bool {
        self.0.confirmed.store(false, Ordering::SeqCst);
        self.0.set(RunState::WaitPause(pause));
        loop {
            if self.0.cancel.load(Ordering::SeqCst) {
                return false;
            }
            if self.0.confirmed.swap(false, Ordering::SeqCst) {
                self.0.set(RunState::Running);
                return true;
            }
            thread::sleep(Duration::from_millis(30));
        }
    }

    fn progress(&mut self, next: usize, total: usize) {
        self.0.next.store(next, Ordering::SeqCst);
        self.0.total.store(total, Ordering::SeqCst);
    }

    fn cancelled(&mut self) -> bool {
        self.0.cancel.load(Ordering::SeqCst)
    }
}

pub struct RealRun {
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
    /// True for a job on the server, which does not stop when this program does.
    on_server: bool,
}

impl RealRun {
    /// Starts the job on the machine of `config`. Returns at once; follow it with [`RealRun::state`].
    pub fn start(config: ConnectionConfig, items: Vec<Item>) -> Self {
        let shared = Arc::new(Shared {
            notes: Mutex::new(ServerNotes::default()),
            state: Mutex::new(RunState::Connecting),
            next: AtomicUsize::new(0),
            total: AtomicUsize::new(items.len()),
            cancel: AtomicBool::new(false),
            confirmed: AtomicBool::new(false),
            stop_following: AtomicBool::new(false),
        });
        let worker = shared.clone();
        let handle = thread::spawn(move || {
            let result = (|| -> Result<Outcome, String> {
                let link = config
                    .open()
                    .map_err(|e| format!("{}: {e}", config.describe()))?;
                let mut machine = Machine::new(link);
                machine.set_pacing(config.pacing());
                machine.set_tuning(config.tuning);
                machine.set_home_signal(config.home_signal);
                machine.connect().map_err(|e| format!("{e:?}"))?;
                machine.arm(Verified::all()).map_err(|e| format!("{e:?}"))?;
                worker.set(RunState::Running);
                run_items(&mut machine, &items, &mut WorkerHooks(worker.clone()))
                    .map_err(|e| format!("{e:?}"))
            })();
            worker.set(match result {
                Ok(Outcome::Completed) => RunState::Done,
                Ok(Outcome::Cancelled) => RunState::Cancelled,
                Err(e) => RunState::Failed(e),
            });
        });
        RealRun {
            shared,
            handle: Some(handle),
            on_server: false,
        }
    }

    /// Sends the job to the EH3D server (`connection = server`), has it started on the machine of the connection and follows it. `request` is
    /// the job as the server's interface takes it; `hole_ends` and `items_total` are the program's own plan of the same job, used to tell where
    /// the run has got to in the program's terms. The dialogs of the job come here as [`RunState::WaitPause`], [`RealRun::confirm`] answers
    /// them and [`RealRun::cancel`] stops the job on the server. Closing the program does not stop the job.
    pub fn start_on_server(
        config: ConnectionConfig,
        request: serde_json::Value,
        hole_ends: Vec<usize>,
        items_total: usize,
    ) -> Self {
        let shared = Arc::new(Shared {
            notes: Mutex::new(ServerNotes::default()),
            state: Mutex::new(RunState::Connecting),
            next: AtomicUsize::new(0),
            total: AtomicUsize::new(items_total),
            cancel: AtomicBool::new(false),
            confirmed: AtomicBool::new(false),
            stop_following: AtomicBool::new(false),
        });
        let worker = shared.clone();
        let handle = thread::spawn(move || {
            let result = follow_on_server(&config, &request, &hole_ends, items_total, &worker);
            if let Err(why) = result {
                worker.set(RunState::Failed(why));
            }
        });
        RealRun {
            shared,
            handle: Some(handle),
            on_server: true,
        }
    }

    pub fn state(&self) -> RunState {
        self.shared
            .state
            .lock()
            .map(|g| g.clone())
            .unwrap_or(RunState::Failed("internal error".into()))
    }

    /// What the server said of the job besides its state (empty for a job that runs on a machine of this computer).
    pub fn server_notes(&self) -> ServerNotes {
        self.shared
            .notes
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Index of the item being run and the number of items.
    pub fn progress(&self) -> (usize, usize) {
        (
            self.shared.next.load(Ordering::SeqCst),
            self.shared.total.load(Ordering::SeqCst),
        )
    }

    /// The operator answered the dialog.
    pub fn confirm(&self) {
        self.shared.confirmed.store(true, Ordering::SeqCst);
    }

    /// Stops the machine (drivers off). The state becomes `Cancelled` once it has stopped.
    pub fn cancel(&self) {
        self.shared.cancel.store(true, Ordering::SeqCst);
    }

    pub fn finished(&self) -> bool {
        self.state().finished()
    }
}

impl Drop for RealRun {
    fn drop(&mut self) {
        if self.on_server {
            self.shared.stop_following.store(true, Ordering::SeqCst);
        } else {
            self.shared.cancel.store(true, Ordering::SeqCst);
        }
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// The program's step in its own plan that matches what the server reports: for a drilling job the end of the last hole done, for any job
/// the same fraction of the items.
fn local_step(job: &serde_json::Value, hole_ends: &[usize], items_total: usize) -> usize {
    let holes_done = job["holes_done"].as_u64().unwrap_or(0) as usize;
    if !hole_ends.is_empty() {
        return match holes_done {
            0 => 0,
            n => hole_ends.get(n - 1).map_or(items_total, |&e| e + 1),
        };
    }
    let (done, total) = (
        job["items_done"].as_u64().unwrap_or(0) as usize,
        job["items_total"].as_u64().unwrap_or(0) as usize,
    );
    (done * items_total)
        .checked_div(total)
        .map_or(0, |step| step.min(items_total))
}

fn state_of(job: &serde_json::Value) -> RunState {
    match job["state"].as_str().unwrap_or("") {
        "running" => RunState::Running,
        "waiting_operator" => RunState::WaitPause(match job["pause"]["kind"].as_str() {
            Some("spindle_start") => Pause::SpindleStart,
            _ => Pause::ToolChange {
                tool: job["pause"]["tool"].as_u64().unwrap_or(0) as u8,
            },
        }),
        "done" => RunState::Done,
        "cancelled" => RunState::Cancelled,
        "failed" => RunState::Failed(
            job["message"]
                .as_str()
                .filter(|m| !m.is_empty())
                .unwrap_or("the job failed on the server")
                .to_string(),
        ),
        _ => RunState::Connecting,
    }
}

/// The head is asked every this many turns of the loop that follows a job (a turn is a quarter of a second).
const HEAD_EVERY: u32 = 8;

fn follow_on_server(
    config: &ConnectionConfig,
    request: &serde_json::Value,
    hole_ends: &[usize],
    items_total: usize,
    shared: &Shared,
) -> Result<(), String> {
    use crate::server_link::{error_text, Client};
    let client = Client::new(config);
    if client.address.is_empty() || client.machine.is_empty() {
        return Err("server_address and server_machine are not set in the connection file".into());
    }
    let (status, job) = client.call(
        "POST",
        &format!("/api/v1/machines/{}/jobs", path_segment(&client.machine)),
        Some(request),
    )?;
    if status != 200 {
        return Err(error_text(status, &job));
    }
    let id = job["id"]
        .as_str()
        .ok_or("the server did not give the job an id")?
        .to_string();
    if let Ok(mut n) = shared.notes.lock() {
        n.spindle_warnings = spindle_notes(&job["view"]);
    }
    let (status, answer) = client.call(
        "POST",
        &format!("/api/v1/jobs/{}/start", path_segment(&id)),
        None,
    )?;
    if status != 200 {
        // a job the server kept but could not start is not left lying ready on it
        let _ = client.call(
            "POST",
            &format!("/api/v1/jobs/{}/cancel", path_segment(&id)),
            None,
        );
        return Err(error_text(status, &answer));
    }
    let mut cancelled = false;
    let mut lost = 0u32;
    // the node of the drill head, when the machine has one, is asked from time to time: a machine without one (404) or a node that does not answer is let go
    let (mut turn, mut head_failures) = (0u32, 0u32);
    loop {
        if shared.stop_following.load(Ordering::SeqCst) {
            return Ok(());
        }
        if head_failures < 2 && turn % HEAD_EVERY == 0 {
            match client.call(
                "GET",
                &format!(
                    "/api/v1/machines/{}/head/status",
                    path_segment(&client.machine)
                ),
                None,
            ) {
                Ok((200, head)) => {
                    head_failures = 0;
                    if let Ok(mut n) = shared.notes.lock() {
                        n.head_rpm = head["rpm"].as_f64();
                        n.head_temp_c = head["temp_c"].as_f64();
                    }
                }
                Ok((404, _)) => head_failures = 2,
                _ => head_failures += 1,
            }
        }
        turn = turn.wrapping_add(1);
        if shared.cancel.load(Ordering::SeqCst) && !cancelled {
            // the stop is asked again at every turn until the server has taken it (or says the job is already over)
            match client.call(
                "POST",
                &format!("/api/v1/jobs/{}/cancel", path_segment(&id)),
                None,
            ) {
                Ok((200..=299, _)) | Ok((404 | 409, _)) => cancelled = true,
                _ => {}
            }
        }
        if shared.confirmed.swap(false, Ordering::SeqCst) {
            let _ = client.call(
                "POST",
                &format!("/api/v1/jobs/{}/confirm", path_segment(&id)),
                None,
            );
        }
        match client.call("GET", &format!("/api/v1/jobs/{}", path_segment(&id)), None) {
            Ok((200, job)) => {
                lost = 0;
                shared
                    .next
                    .store(local_step(&job, hole_ends, items_total), Ordering::SeqCst);
                let state = state_of(&job);
                let finished = state.finished();
                if finished && state == RunState::Done {
                    shared.next.store(items_total, Ordering::SeqCst);
                }
                shared.set(state);
                if finished {
                    return Ok(());
                }
            }
            Ok((status, body)) => return Err(error_text(status, &body)),
            Err(why) => {
                lost += 1;
                if lost >= 40 {
                    return Err(format!(
                        "lost the server ({why}); the job may still be running on it"
                    ));
                }
            }
        }
        thread::sleep(Duration::from_millis(250));
    }
}

// ---------------------------------------------------------------------------------------------------------------- by hand

/// Where a hand session (the Offset Teach window) is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeachState {
    /// Opening the link and homing the machine.
    Homing,
    /// Homed: the head follows the jogs.
    Ready,
    /// The head went home again and the drivers are off.
    Finished,
    Failed(String),
}

enum TeachCmd {
    /// A move relative to where the head is, in mm (X, Y, and Z downwards).
    Jog(f64, f64, f64),
    /// Z up, home again and drivers off.
    End,
    /// Stop at once.
    Cancel,
}

struct NoDialogs(Arc<AtomicBool>);

impl Hooks for NoDialogs {
    fn pause(&mut self, _pause: Pause) -> bool {
        true
    }
    fn progress(&mut self, _next: usize, _total: usize) {}
    fn cancelled(&mut self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// The machine moved by hand: it homes when the session starts, follows every [`TeachSession::jog`] and homes again at the end.
/// Used by the Offset Teach window of DRILLPRO and ROUTEPRO.
pub struct TeachSession {
    state: Arc<Mutex<TeachState>>,
    cancel: Arc<AtomicBool>,
    tx: std::sync::mpsc::Sender<TeachCmd>,
    handle: Option<JoinHandle<()>>,
}

impl TeachSession {
    pub fn start(config: ConnectionConfig, jm: JobMachine) -> Self {
        let state = Arc::new(Mutex::new(TeachState::Homing));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel::<TeachCmd>();
        let (st, cn) = (state.clone(), cancel.clone());
        let handle = thread::spawn(move || {
            let set = |s: TeachState| {
                if let Ok(mut g) = st.lock() {
                    *g = s;
                }
            };
            let result = (|| -> Result<(), String> {
                if config.kind == crate::protocol::connection::Kind::Server {
                    return Err(
                        "the Offset window does not move a machine through the server".into(),
                    );
                }
                let link = config
                    .open()
                    .map_err(|e| format!("{}: {e}", config.describe()))?;
                let mut machine = Machine::new(link);
                machine.set_pacing(config.pacing());
                machine.set_tuning(config.tuning);
                machine.set_home_signal(config.home_signal);
                machine.connect().map_err(|e| format!("{e:?}"))?;
                machine.arm(Verified::all()).map_err(|e| format!("{e:?}"))?;
                let start = JobBuilder::new(&jm).into_items();
                let mut hooks = NoDialogs(cn.clone());
                match run_items(&mut machine, &start, &mut hooks).map_err(|e| format!("{e:?}"))? {
                    Outcome::Completed => {}
                    Outcome::Cancelled => return Ok(()),
                }
                set(TeachState::Ready);
                let g = jm.geometry;
                let (mut pos, mut z) = ((0.0f64, 0.0f64), 0.0f64);
                loop {
                    match rx.recv() {
                        Ok(TeachCmd::Jog(dx, dy, dz)) => {
                            let to = (pos.0 + dx, pos.1 + dy);
                            let mut steps = Vec::new();
                            if dx != 0.0 || dy != 0.0 {
                                steps.extend(rapid_xy(pos, to, &g));
                            }
                            if dz != 0.0 {
                                let new_z = (z + dz).max(0.0);
                                let d = new_z - z;
                                if d != 0.0 {
                                    steps.extend(z_move(d.abs(), d > 0.0, g.z_max_feed(), &g));
                                }
                                z = new_z;
                            }
                            machine.run(&steps).map_err(|e| format!("{e:?}"))?;
                            pos = to;
                        }
                        Ok(TeachCmd::End) | Err(_) => break,
                        Ok(TeachCmd::Cancel) => {
                            machine.emergency_stop();
                            return Ok(());
                        }
                    }
                }
                if z > 0.0 {
                    machine
                        .run(&z_move(z, false, g.z_max_feed(), &g))
                        .map_err(|e| format!("{e:?}"))?;
                }
                let end = JobBuilder::new(&jm).finish();
                run_items(&mut machine, &end, &mut hooks).map_err(|e| format!("{e:?}"))?;
                Ok(())
            })();
            set(match result {
                Ok(()) => TeachState::Finished,
                Err(e) => TeachState::Failed(e),
            });
        });
        TeachSession {
            state,
            cancel,
            tx,
            handle: Some(handle),
        }
    }

    pub fn state(&self) -> TeachState {
        self.state
            .lock()
            .map(|g| g.clone())
            .unwrap_or(TeachState::Failed("internal error".into()))
    }

    /// Moves the head by this much (mm; Z positive is down). Moves queue up and run in order.
    pub fn jog(&self, dx_mm: f64, dy_mm: f64, dz_mm: f64) {
        let _ = self.tx.send(TeachCmd::Jog(dx_mm, dy_mm, dz_mm));
    }

    /// Z up, home again and drivers off; returns at once.
    pub fn end(&self) {
        let _ = self.tx.send(TeachCmd::End);
    }
}

impl Drop for TeachSession {
    fn drop(&mut self) {
        // a session that is dropped without the end stops the machine; one that ended is left to finish its way home
        self.cancel.store(true, Ordering::SeqCst);
        let _ = self.tx.send(TeachCmd::Cancel);
        if let Some(h) = self.handle.take() {
            if matches!(self.state(), TeachState::Finished | TeachState::Failed(_)) {
                let _ = h.join();
            }
        }
    }
}

#[cfg(test)]
mod server_notes_tests {
    use super::*;

    #[test]
    fn the_excesses_of_the_spindle_are_told_in_short_texts() {
        let view = serde_json::json!({"spindle_warnings": [
            {"code": "rpm_above_max", "tool": 1, "value": 60000, "limit": 24000},
            {"code": "rpm_below_min", "tool": 2, "value": 3000, "limit": 6000},
            {"code": "feed_xy_above_max", "tool": 1, "value": 2500, "limit": 1500},
        ]});
        assert_eq!(
            spindle_notes(&view),
            [
                "T1 60000>24000 rpm",
                "T2 3000<6000 rpm",
                "T1 2500>1500 mm/min"
            ]
        );
        assert!(spindle_notes(&serde_json::json!({})).is_empty());
        assert!(spindle_notes(&serde_json::json!({"spindle_warnings": []})).is_empty());
    }

    #[test]
    fn the_notes_become_lines_only_when_there_is_something_to_say() {
        let mut n = ServerNotes::default();
        assert_eq!((n.spindle_line(), n.head_line()), (None, None));
        n.spindle_warnings = vec!["T1 60000>24000 rpm".into(), "T2 3000<6000 rpm".into()];
        assert_eq!(
            n.spindle_line().as_deref(),
            Some("T1 60000>24000 rpm; T2 3000<6000 rpm")
        );
        n.head_rpm = Some(18000.4);
        assert_eq!(n.head_line().as_deref(), Some("18000 rpm"));
        n.head_temp_c = Some(35.25);
        assert_eq!(n.head_line().as_deref(), Some("18000 rpm  35.2 \u{b0}C"));
        n.head_rpm = None;
        assert_eq!(n.head_line().as_deref(), Some("35.2 \u{b0}C"));
    }
}
