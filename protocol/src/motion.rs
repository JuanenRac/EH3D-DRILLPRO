//! Motion planning primitives in plain millimetres, independent of any file format.

use crate::{bits, plan_line, Step};

/// Nominal resolution of the machine: one step per mil (0.001 inch = 0.0254 mm), i.e. 39.3701 steps/mm.
/// Verified against the original program, which sends `round(mm * 39.3701 * correction)` steps.
pub const STEPS_PER_MM: f64 = 1000.0 / 25.4;

/// Machine geometry. The per-axis correction coefficients come from the machine configuration file
/// (`Correction coeff` X, Y, Z); the defaults are neutral (1.0). Speed values are placeholders until
/// they are measured on the machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub steps_per_mm_xy: f64,
    pub steps_per_mm_z: f64,
    /// Correction coefficients multiplying the nominal steps per mm of each axis.
    pub correction: (f64, f64, f64),
    /// Step rate (Hz) at the start and end of a move.
    pub start_hz: f64,
    /// Highest step rate (Hz).
    pub max_hz: f64,
    /// Length of the acceleration ramp, in steps.
    pub ramp_steps: u32,
    /// Highest Z step rate (Hz); the machine configuration gives Z its own maximum speed.
    pub z_max_hz: f64,
    /// Step rate of the second cycle of a reference run, where the axis backs off and approaches the switch again to
    /// remove its play (the REF. speed of the configuration), for XY and for Z.
    pub ref_hz: f64,
    pub z_ref_hz: f64,
    /// Start speed of the Z ramp.
    pub z_start_hz: f64,
    /// The reference run of X and Y goes axis by axis (X to its switch, then Y), as the original does on the real machine,
    /// instead of diagonally (what the recorded traffic of the emulated original shows). Real machines stall on the diagonal.
    pub sequential_homing: bool,
    /// Steps of the X and of the Y search when the run is sequential: a little more than the travel of each axis.
    pub search_x_steps: u32,
    pub search_y_steps: u32,
    /// The values found by trying the real machine (search speeds, ramps, the calibration jump).
    pub tuning: crate::tuning::Tuning,
}

impl Default for Geometry {
    fn default() -> Self {
        Self {
            steps_per_mm_xy: STEPS_PER_MM,
            steps_per_mm_z: STEPS_PER_MM,
            correction: (1.0, 1.0, 1.0),
            start_hz: 500.0,
            max_hz: 4000.0,
            ramp_steps: 40,
            z_max_hz: 4000.0,
            ref_hz: 1000.0,
            z_ref_hz: 1000.0,
            z_start_hz: 500.0,
            sequential_homing: false,
            search_x_steps: 22_000,
            search_y_steps: 16_500,
            tuning: crate::tuning::Tuning::default(),
        }
    }
}

impl Geometry {
    /// Geometry with the correction coefficients of the machine configuration file.
    pub fn with_correction(x: f64, y: f64, z: f64) -> Self {
        Self {
            correction: (x, y, z),
            ..Self::default()
        }
    }

    /// Takes the speeds of the machine configuration, all in mm/min: for XY the normal, reference and ramp-start speeds, and
    /// the same three for Z. The job screen of the original lets the operator change them.
    pub fn with_speeds(mut self, xy: (f64, f64, f64), z: (f64, f64, f64)) -> Self {
        self.max_hz = self.xy_hz(xy.0);
        self.ref_hz = self.xy_hz(xy.1).min(self.max_hz);
        self.start_hz = self.xy_hz(xy.2).min(self.max_hz);
        self.z_max_hz = self.z_hz(z.0);
        self.z_ref_hz = self.z_hz(z.1).min(self.z_max_hz);
        self.z_start_hz = self.z_hz(z.2).min(self.z_max_hz);
        self
    }

    /// Limits the step rates the machine's motors keep up with (in steps per second). Every speed of the job stays at or below
    /// them, whatever the configuration file says.
    pub fn capped(mut self, xy_hz: Option<f64>, z_hz: Option<f64>) -> Self {
        if let Some(c) = xy_hz {
            self.max_hz = self.max_hz.min(c);
            self.start_hz = self.start_hz.min(self.max_hz);
            self.ref_hz = self.ref_hz.min(self.max_hz);
        }
        if let Some(c) = z_hz {
            self.z_max_hz = self.z_max_hz.min(c);
            self.z_start_hz = self.z_start_hz.min(self.z_max_hz);
            self.z_ref_hz = self.z_ref_hz.min(self.z_max_hz);
        }
        self
    }

    pub fn x_steps(&self, mm: f64) -> i32 {
        (mm * self.steps_per_mm_xy * self.correction.0).round() as i32
    }

    pub fn y_steps(&self, mm: f64) -> i32 {
        (mm * self.steps_per_mm_xy * self.correction.1).round() as i32
    }

    /// X steps without correction; kept for callers that treat X and Y alike.
    pub fn xy_steps(&self, mm: f64) -> i32 {
        (mm * self.steps_per_mm_xy).round() as i32
    }

    pub fn z_steps(&self, mm: f64) -> u32 {
        (mm.max(0.0) * self.steps_per_mm_z * self.correction.2).round() as u32
    }

    /// Z distance in mm of a number of steps (inverse of [`Geometry::z_steps`]).
    pub fn z_mm(&self, steps: u32) -> f64 {
        steps as f64 / (self.steps_per_mm_z * self.correction.2)
    }

    /// Fastest Z feed in mm/min allowed by `max_hz`.
    pub fn z_max_feed(&self) -> f64 {
        self.z_max_hz * 60.0 / (self.steps_per_mm_z * self.correction.2)
    }

    /// Step rate for a feed in mm/min on the XY axes.
    pub fn xy_hz(&self, feed_mm_min: f64) -> f64 {
        (feed_mm_min / 60.0 * self.steps_per_mm_xy).max(1.0)
    }

    pub fn z_hz(&self, feed_mm_min: f64) -> f64 {
        (feed_mm_min / 60.0 * self.steps_per_mm_z * self.correction.2).max(1.0)
    }
}

/// Rapid XY move between two points given in mm.
pub fn rapid_xy(from_mm: (f64, f64), to_mm: (f64, f64), g: &Geometry) -> Vec<Step> {
    let dx = g.x_steps(to_mm.0) - g.x_steps(from_mm.0);
    let dy = g.y_steps(to_mm.1) - g.y_steps(from_mm.1);
    plan_line(dx, dy, g.start_hz, g.max_hz, g.ramp_steps)
}

/// XY move at a constant feed (mm/min), without acceleration ramp.
pub fn feed_xy(
    from_mm: (f64, f64),
    to_mm: (f64, f64),
    feed_mm_min: f64,
    g: &Geometry,
) -> Vec<Step> {
    let dx = g.x_steps(to_mm.0) - g.x_steps(from_mm.0);
    let dy = g.y_steps(to_mm.1) - g.y_steps(from_mm.1);
    let hz = g.xy_hz(feed_mm_min).min(g.max_hz);
    plan_line(dx, dy, g.start_hz.min(hz), hz, 0)
}

/// Z movement of `depth_mm` downwards (`down = true`) or upwards at `feed_mm_min`.
pub fn z_move(depth_mm: f64, down: bool, feed_mm_min: f64, g: &Geometry) -> Vec<Step> {
    let byte = if down { bits::Z_NEG } else { bits::Z_POS };
    let hz = g.z_hz(feed_mm_min);
    (0..g.z_steps(depth_mm))
        .map(|_| Step {
            byte,
            delay_us: 1e6 / hz,
        })
        .collect()
}

/// One drilling stroke: down at `feed_mm_min`, then back up at the fastest Z rate.
pub fn drill_stroke(depth_mm: f64, feed_mm_min: f64, g: &Geometry) -> Vec<Step> {
    let mut v = z_move(depth_mm, true, feed_mm_min, g);
    v.extend(z_move(depth_mm, false, g.z_max_feed(), g));
    v
}

/// Total time of a plan in seconds (sum of the programmed delays).
pub fn duration_s(steps: &[Step]) -> f64 {
    steps.iter().map(|s| s.delay_us).sum::<f64>() / 1e6
}

/// Distances of one drilling cycle, in mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZParams {
    /// Maximum Z depth of the machine (configuration `Max. Z depth`).
    pub max_depth_mm: f64,
    /// Thickness of the base sheet under the board.
    pub base_mm: f64,
    pub board_mm: f64,
    /// Distance between the tool and the board when the head is at the travel height.
    pub distance_tool_board_mm: f64,
    /// Extra stroke of the tool (tool table).
    pub extra_stroke_mm: f64,
}

/// Z positions of a drilling cycle in steps below the home switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZLevels {
    /// Length of one drilling stroke.
    pub stroke_steps: u32,
    /// Depth at which a hole ends.
    pub end_steps: u32,
    /// Height the head keeps while travelling between holes (`end - stroke`).
    pub travel_steps: u32,
}

/// The margin the original adds to the maximum depth before subtracting the base sheet.
/// Inferred from captures (37.30 + 0.5 mm gives the observed depths), not read from the code.
pub const Z_DEPTH_MARGIN_MM: f64 = 0.5;

/// Z positions of a drilling cycle. Strokes and depths are truncated (not rounded) to whole steps.
/// Reproduces the original within one step in all six captured configurations: baseline 912 / 601,
/// board 2.50 (654), base 8 (1054), extra stroke 0.5 (930 / 619) and distance 10 (389; the original
/// ended 1 step deeper, 913).
pub fn z_levels(p: &ZParams, g: &Geometry) -> ZLevels {
    let s = g.steps_per_mm_z * g.correction.2;
    let trunc = |mm: f64| (mm.max(0.0) * s).floor() as u32;
    let stroke = trunc(p.distance_tool_board_mm + p.board_mm + p.extra_stroke_mm);
    let end = trunc(p.max_depth_mm + Z_DEPTH_MARGIN_MM - p.base_mm + p.extra_stroke_mm);
    ZLevels {
        stroke_steps: stroke,
        end_steps: end,
        travel_steps: end.saturating_sub(stroke),
    }
}
