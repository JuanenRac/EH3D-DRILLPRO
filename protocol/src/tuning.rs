//! The values that make the real machine run well and that were found by trying it: search speeds, ramps, the calibration jump.
//! They are the defaults of today and can be changed one by one in the machine file shared by DRILLPRO and ROUTEPRO (see
//! [`crate::connection`]), so they can be tuned later without touching the code.
//!
//! Speeds are in steps per second (the control unit takes one byte per step; the most the line allows at 38400 baud is 3840).

/// Tuning of the reference run and of the moves of Z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Fastest rate of the X search and of the Y search towards their switches. On the owner's machine X stalls at 2700 and goes well at
    /// 2400, Y stalls at 2700, and 2200 is the value chosen for both.
    pub x_search_hz: f64,
    pub y_search_hz: f64,
    /// Length of the ramp (in steps) of those searches. A longer ramp is gentler and calmer to listen to.
    pub search_ramp_steps: u32,
    /// How much longer than the travel of each axis a search is, in percent (the control unit swallows the surplus).
    pub search_margin_percent: f64,
    /// Z search: its fastest rate, the rate it starts at, and the length of its ramp.
    pub z_search_hz: f64,
    pub z_start_hz: f64,
    pub z_ramp_steps: u32,
    pub z_search_steps: u32,
    /// Calibration jump of X and Y (back off, approach the switch again): the rate it rises to, where it starts as a fraction of the
    /// reference speed, how many parts the move is cut into for the ramps (2: half accelerating and half slowing down, the
    /// smoothest; 3: a third accelerating, a third cruising, a third slowing down) and the number of steps of each leg.
    pub jump_peak_hz: f64,
    pub jump_start_fraction: f64,
    pub jump_ramp_parts: u32,
    pub jump_backoff_steps: u32,
    pub jump_approach_steps: u32,
    /// The same for Z: steps down and steps up again.
    pub z_backoff_steps: u32,
    pub z_approach_steps: u32,
    /// Moves of Z alone go one byte at a time with the PC clock (`true`) even on a connection that sends moves as a continuous
    /// stream (`false` = as a stream too). On the owner's machine the Z motor is weak and jerky with the stream's filler bytes.
    pub z_pacing_bytes: bool,
    /// The filler between two steps of a stream keeps the direction bits of the last step (`true`) or is zero (`false`).
    pub stream_keep_direction: bool,
    /// Steps sent at a time before the cancel key is looked at again, and steps between two readings of the home line in a search
    /// on a connection where reading it is cheap (the computer's own serial port).
    pub cancel_piece_steps: u32,
    pub home_check_piece_steps: u32,
    /// How long the job waits for the spindle to reach speed after starting it, and to stop after braking it.
    pub spindle_start_wait_ms: u32,
    pub spindle_stop_wait_ms: u32,
    pub spindle_node: bool,
    /// The auxiliary output (latch A bit 5) is switched on with the spindle and off with it, for a vacuum cleaner wired there. The
    /// original on the manual machine never does it (its vacuum relay, if any, follows the control unit's spindle bits), so the default is off.
    pub vacuum_aux: bool,
    /// A search on a paced connection goes out in chunks of this many bytes (260 microseconds each) while another thread watches the
    /// home line; when it rises no more chunks are sent. A chunk of 900 bytes is about 0.23 s, which is also how often the driver of a
    /// USB adapter lets a reading through while it writes.
    pub home_watch_chunk_bytes: u32,
    /// Also watch the home line during the first long searches (Z, then X), which end in the blind. Each reading of the line costs a
    /// small gap in the stream of steps and a motor at speed can lose steps in it; the line cannot rise before the last axis has
    /// arrived anyway, so this is off and only the last search is watched.
    pub home_watch_seek: bool,
    /// When the home line is up at the start (all three axes are on their switches: the machine is parked), X and Y first move this
    /// many millimetres away from their switches, smoothly, and then search for them again: the return is a short, known distance and
    /// the arrival of both is seen live on the home line. 0 turns it off.
    pub unstick_mm: f64,
    /// Do it even when the home line is down. The positions are then unknown, so there is no short search and an axis at the far end
    /// of its travel would be driven against its end stop: leave it off unless you know where the axes are.
    pub unstick_always: bool,
    /// Steps added to the known distance of the short search that follows, in case the axis is a little further than expected.
    pub search_hint_margin_steps: u32,
    /// Length of the ramp (in steps) of the moves between holes and paths (the original's acceleration coefficient). The 40 steps of
    /// a bare default are far too brusque for the owner's motors at speed.
    pub travel_ramp_steps: u32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            x_search_hz: 2200.0,
            y_search_hz: 2200.0,
            search_ramp_steps: 800,
            search_margin_percent: 12.0,
            z_search_hz: 900.0,
            z_start_hz: 450.0,
            z_ramp_steps: 200,
            z_search_steps: 2150,
            jump_peak_hz: 1400.0,
            jump_start_fraction: 0.5,
            jump_ramp_parts: 2,
            jump_backoff_steps: 200,
            jump_approach_steps: 250,
            z_backoff_steps: 45,
            z_approach_steps: 54,
            z_pacing_bytes: true,
            stream_keep_direction: true,
            cancel_piece_steps: 1500,
            home_check_piece_steps: 100,
            spindle_start_wait_ms: 1680,
            spindle_stop_wait_ms: 1000,
            spindle_node: false,
            vacuum_aux: false,
            home_watch_chunk_bytes: 0,
            home_watch_seek: false,
            unstick_mm: 10.0,
            unstick_always: false,
            search_hint_margin_steps: 300,
            travel_ramp_steps: 400,
        }
    }
}

fn number(key: &str, value: &str, lo: f64, hi: f64) -> Result<f64, String> {
    let v: f64 = value
        .parse()
        .map_err(|_| format!("{key} must be a number"))?;
    if (lo..=hi).contains(&v) {
        Ok(v)
    } else {
        Err(format!("{key} must be between {lo} and {hi}"))
    }
}

/// A word out of two: `Ok(true)` for the first, `Ok(false)` for the second.
fn choice(key: &str, value: &str, first: &str, second: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        v if v == first => Ok(true),
        v if v == second => Ok(false),
        _ => Err(format!("{key} must be {first} or {second}")),
    }
}

impl Tuning {
    /// Sets one value by its name in the machine file. `Ok(false)` when the name is not a tuning value.
    pub fn set(&mut self, key: &str, value: &str) -> Result<bool, String> {
        let hz = |v: &str| number(key, v, 50.0, 3840.0);
        let steps = |v: &str| number(key, v, 1.0, 5000.0).map(|n| n as u32);
        let ms = |v: &str| number(key, v, 0.0, 20000.0).map(|n| n as u32);
        match key {
            "x_search_hz" => self.x_search_hz = hz(value)?,
            "y_search_hz" => self.y_search_hz = hz(value)?,
            "search_ramp_steps" => self.search_ramp_steps = steps(value)?,
            "search_margin_percent" => self.search_margin_percent = number(key, value, 0.0, 100.0)?,
            "z_search_hz" => self.z_search_hz = hz(value)?,
            "z_start_hz" => self.z_start_hz = hz(value)?,
            "z_ramp_steps" => self.z_ramp_steps = steps(value)?,
            "z_search_steps" => self.z_search_steps = steps(value)?,
            "jump_peak_hz" => self.jump_peak_hz = hz(value)?,
            "jump_start_fraction" => self.jump_start_fraction = number(key, value, 0.2, 1.0)?,
            "jump_ramp_parts" => self.jump_ramp_parts = number(key, value, 1.0, 10.0)? as u32,
            "jump_backoff_steps" => self.jump_backoff_steps = steps(value)?,
            "jump_approach_steps" => self.jump_approach_steps = steps(value)?,
            "z_backoff_steps" => self.z_backoff_steps = steps(value)?,
            "z_approach_steps" => self.z_approach_steps = steps(value)?,
            "z_pacing" => self.z_pacing_bytes = choice(key, value, "bytes", "stream")?,
            "stream_filler" => self.stream_keep_direction = choice(key, value, "dir", "zero")?,
            "cancel_piece_steps" => self.cancel_piece_steps = steps(value)?,
            "home_check_piece_steps" => self.home_check_piece_steps = steps(value)?,
            "spindle_start_wait_ms" => self.spindle_start_wait_ms = ms(value)?,
            "spindle_stop_wait_ms" => self.spindle_stop_wait_ms = ms(value)?,
            "spindle" => self.spindle_node = !choice(key, value, "central", "node")?,
            "vacuum" => self.vacuum_aux = !choice(key, value, "central", "aux")?,
            "unstick_mm" => self.unstick_mm = number(key, value, 0.0, 50.0)?,
            "unstick_always" => self.unstick_always = choice(key, value, "on", "off")?,
            "travel_ramp_steps" => self.travel_ramp_steps = steps(value)?,
            "search_hint_margin_steps" => self.search_hint_margin_steps = steps(value)?,
            "home_watch_seek" => self.home_watch_seek = choice(key, value, "on", "off")?,
            "home_watch_chunk_bytes" => {
                self.home_watch_chunk_bytes = number(key, value, 0.0, 4000.0)? as u32
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
