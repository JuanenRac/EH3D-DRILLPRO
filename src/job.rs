//! Drilling job: ordering of holes and the move/drill sequence sent to the machine.

use crate::config::MachineSettings;
use crate::ncd::{DrillFile, Hole, MIL_MM};
use crate::protocol::connection::ConnectionConfig;
use crate::protocol::job::JobMachine;
use crate::protocol::Step;
use crate::tools::Tool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sorting {
    None,
    XFirst,
    YFirst,
    /// Columns by X with the Y direction alternating (meander), within each tool.
    Special,
}

pub use crate::protocol::motion::Geometry;

/// Orders holes by tool (in order of first use), then by the chosen strategy within each tool.
pub fn sort_holes(file: &DrillFile, how: Sorting) -> Vec<Hole> {
    let mut out = Vec::with_capacity(file.holes.len());
    for &t in &file.tools {
        let mut group: Vec<Hole> = file.holes.iter().filter(|h| h.tool == t).cloned().collect();
        match how {
            Sorting::None => {}
            Sorting::XFirst => group.sort_by_key(|h| (h.x_mil, h.y_mil)),
            Sorting::YFirst => group.sort_by_key(|h| (h.y_mil, h.x_mil)),
            Sorting::Special => group = meander(group),
        }
        out.extend(group);
    }
    out
}

/// The original program's "special" sorting: columns by X, and inside each column the holes are
/// visited alternately upwards and downwards in Y (a meander), starting upwards.
/// Verified on a grid of scrambled holes; whether holes whose X differs by a few mils count as one
/// column is not known (here only an equal X does).
fn meander(mut group: Vec<Hole>) -> Vec<Hole> {
    group.sort_by_key(|h| (h.x_mil, h.y_mil));
    let mut out = Vec::with_capacity(group.len());
    let mut i = 0;
    let mut upwards = true;
    while i < group.len() {
        let x = group[i].x_mil;
        let mut j = i;
        while j < group.len() && group[j].x_mil == x {
            j += 1;
        }
        let column = &group[i..j];
        if upwards {
            out.extend_from_slice(column);
        } else {
            out.extend(column.iter().rev().cloned());
        }
        upwards = !upwards;
        i = j;
    }
    out
}

/// Total rapid travel in mm for a visiting order starting at the origin.
pub fn travel_mm(holes: &[Hole]) -> f64 {
    let (mut px, mut py, mut total) = (0.0f64, 0.0f64, 0.0f64);
    for h in holes {
        let (x, y) = (h.x_mil as f64 * MIL_MM, h.y_mil as f64 * MIL_MM);
        total += ((x - px).powi(2) + (y - py).powi(2)).sqrt();
        px = x;
        py = y;
    }
    total
}

/// Bounding box in mm: (min_x, min_y, max_x, max_y).
pub fn bounds_mm(holes: &[Hole]) -> Option<(f64, f64, f64, f64)> {
    let first = holes.first()?;
    let mut b = (first.x_mm(), first.y_mm(), first.x_mm(), first.y_mm());
    for h in holes {
        b.0 = b.0.min(h.x_mm());
        b.1 = b.1.min(h.y_mm());
        b.2 = b.2.max(h.x_mm());
        b.3 = b.3.max(h.y_mm());
    }
    Some(b)
}

/// Steps for the XY move from `from` to `to` (both in mil).
pub fn move_steps(from: (i32, i32), to: (i32, i32), g: &Geometry) -> Vec<Step> {
    let mm = |p: (i32, i32)| (p.0 as f64 * MIL_MM, p.1 as f64 * MIL_MM);
    crate::protocol::motion::rapid_xy(mm(from), mm(to), g)
}

/// Z stroke steps for a tool: down then up (`board_mm` + the tool extra stroke, in mm).
pub fn drill_steps(tool: &Tool, board_mm: f64, g: &Geometry) -> Vec<Step> {
    crate::protocol::motion::drill_stroke(
        board_mm + tool.extra_stroke_mm,
        tool.feed_mm_min as f64,
        g,
    )
}

/// The geometry of a machine from its configuration file (the correction coefficients and the speeds); neutral values without one.
pub fn geometry_from(settings: Option<&MachineSettings>) -> Geometry {
    settings
        .map(|m| {
            Geometry::with_correction(m.correction.0, m.correction.1, m.correction.2).with_speeds(
                (
                    m.speed_xy.0 as f64,
                    m.speed_xy.1 as f64,
                    m.speed_xy.2 as f64,
                ),
                (m.speed_z.0 as f64, m.speed_z.1 as f64, m.speed_z.2 as f64),
            )
        })
        .unwrap_or_default()
}

/// The machine data a job needs, from the configuration file of the machine and the connection chosen for it. On a real machine the
/// speeds are capped at what the motors keep up with, the values found by trying the machine (`connection.tuning`) are used and the
/// reference run goes axis by axis, each search a little longer than that axis's travel. `default_board_mm` is the thickness of the
/// board when there is no configuration file. Shared by DRILLPRO, ROUTEPRO and the EH3D server.
pub fn job_machine(
    settings: Option<&MachineSettings>,
    geometry: Geometry,
    connection: &ConnectionConfig,
    default_board_mm: f64,
) -> JobMachine {
    let mut geometry = geometry;
    if connection.is_real() {
        geometry = geometry.capped(connection.xy_hz_limit, connection.z_hz_limit);
        geometry.tuning = connection.tuning;
        geometry.ramp_steps = connection.tuning.travel_ramp_steps;
        geometry.sequential_homing = true;
        if let Some(m) = settings {
            let margin = 1.0 + connection.tuning.search_margin_percent / 100.0;
            geometry.search_x_steps = geometry.x_steps(m.clip_mm.0 * margin).unsigned_abs();
            geometry.search_y_steps = geometry.y_steps(m.clip_mm.1 * margin).unsigned_abs();
        }
    }
    match settings {
        Some(m) => JobMachine {
            geometry,
            offset_mm: m.offset_mm,
            max_depth_mm: m.z_max_depth_mm,
            base_mm: m.base_mm,
            board_mm: m.board_mm,
            distance_tool_board_mm: m.distance_tool_board_mm,
            xy_search_steps: 26_613,
        },
        None => JobMachine {
            geometry,
            offset_mm: (0.0, 0.0),
            max_depth_mm: 37.3,
            base_mm: 12.0,
            board_mm: default_board_mm,
            distance_tool_board_mm: 16.0,
            xy_search_steps: 26_613,
        },
    }
}

/// The items that finish a hole: a long Z descent followed by a long ascent. Tells how many holes are done from the item the job has reached.
pub fn hole_end_indexes(items: &[crate::protocol::job::Item]) -> Vec<usize> {
    use crate::protocol::{bits, job::Item};
    let mut ends = Vec::new();
    let mut pending = false;
    for (i, item) in items.iter().enumerate() {
        if let Item::Steps(steps) = item {
            let first = steps.first().map(|x| x.byte);
            if first == Some(bits::Z_POS) && steps.len() >= 100 {
                pending = true;
            } else if first == Some(bits::Z_NEG) && steps.len() >= 100 && pending {
                ends.push(i);
                pending = false;
            }
        }
    }
    ends
}
