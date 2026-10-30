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
        } else if matches!(item, Item::Latch(_)) {
            // a descent that ends in a latch is the tool changer's (Z down to open the chuck), not a hole
            pending = false;
        }
    }
    ends
}

/// Copies of the board in a panel: `columns` by `rows`, each `pitch_mm` from the next (the board less the spacing, as the
/// original's count of copies works out).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Panel {
    pub columns: u32,
    pub rows: u32,
    pub pitch_mm: (f64, f64),
    /// Tabs that hold the boards while the outside of the panel is routed; none routes the outside through.
    pub tabs: Option<Tabs>,
}

/// Where the tabs of a side go: spread evenly along it, near its two ends, or together in its middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabPlacement {
    Even,
    Ends,
    Middle,
}

/// The tabs (notches) that hold the boards of a panel while the outside is routed: on every straight side of a closed path the router lifts for `width_mm`
/// at each tab, so the board stays joined to the panel by a bridge there and is broken out by hand afterwards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tabs {
    /// How many tabs each side has.
    pub per_side: u32,
    /// How wide each tab is, along the side.
    pub width_mm: f64,
    pub placement: TabPlacement,
}

impl Tabs {
    /// The places of the tabs of a side, as fractions of its length (from 0 to 1).
    fn fractions(&self, length_mm: f64) -> Vec<f64> {
        let n = self.per_side as usize;
        if n == 0 || length_mm <= 0.0 {
            return Vec::new();
        }
        let w = self.width_mm / length_mm;
        match self.placement {
            TabPlacement::Even => (0..n).map(|k| (k + 1) as f64 / (n + 1) as f64).collect(),
            TabPlacement::Ends => match n {
                1 => vec![0.15],
                _ => (0..n)
                    .map(|k| 0.15 + 0.70 * k as f64 / (n - 1) as f64)
                    .collect(),
            },
            TabPlacement::Middle => (0..n)
                .map(|k| 0.5 + (k as f64 - (n as f64 - 1.0) / 2.0) * 2.0 * w)
                .collect(),
        }
    }
}

/// Cuts the tabs into a closed path: the pieces of the path between the tabs, in the order of the path. A path that is open, or has fewer than four points,
/// or a side too short to hold a tab with some bridge on both sides of it, is left as it is on that side. The piece that ends where the path began is joined to
/// the first one, so the router does not lift there for nothing.
pub fn cut_tabs(points: &[(f64, f64)], tabs: &Tabs) -> Vec<Vec<(f64, f64)>> {
    let closed = points.len() >= 4
        && (points[0].0 - points[points.len() - 1].0).abs() < 1e-9
        && (points[0].1 - points[points.len() - 1].1).abs() < 1e-9;
    if !closed || tabs.per_side == 0 || tabs.width_mm <= 0.0 {
        return vec![points.to_vec()];
    }
    let mut pieces: Vec<Vec<(f64, f64)>> = vec![vec![points[0]]];
    let mut cut_any = false;
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        let at = |d: f64| (a.0 + (b.0 - a.0) * d / len, a.1 + (b.1 - a.1) * d / len);
        // the gaps of this side, in mm from its start, inside the side with half a tab of bridge at each end
        let mut gaps: Vec<(f64, f64)> = Vec::new();
        if len > 1e-9 {
            for c in tabs.fractions(len) {
                let (g0, g1) = (c * len - tabs.width_mm / 2.0, c * len + tabs.width_mm / 2.0);
                let margin = tabs.width_mm / 2.0;
                if g0 >= margin
                    && g1 <= len - margin
                    && gaps.last().is_none_or(|l| g0 >= l.1 + 1e-9)
                {
                    gaps.push((g0, g1));
                }
            }
        }
        for (g0, g1) in gaps {
            cut_any = true;
            pieces.last_mut().expect("a piece").push(at(g0));
            pieces.push(vec![at(g1)]);
        }
        pieces.last_mut().expect("a piece").push(b);
    }
    if !cut_any {
        return vec![points.to_vec()];
    }
    // the last piece ends where the path began: it goes on into the first one
    if pieces.len() > 1 {
        let last = pieces.pop().expect("a piece");
        let first = pieces.remove(0);
        let mut joined = last;
        joined.extend(first.into_iter().skip(1));
        pieces.insert(0, joined);
        // the joined piece starts in the middle of the path: it goes first in the order of the pieces only for the router's sake, so put it where it was (last)
        let joined = pieces.remove(0);
        pieces.push(joined);
    }
    pieces
}

/// What the main form and the Panelization dialog add to a job.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JobOptions {
    /// First and last vector to do, counted from 1 in the order of the file; 0 for no limit.
    pub from: usize,
    pub to: usize,
    /// The working area in mm (the clip): vectors that end outside it are left out.
    pub clip_mm: Option<(f64, f64)>,
    pub panel: Option<Panel>,
    /// The job stops at the operator's dialogs (tool change, spindle start) even in the simulation, as the original does.
    pub ask_operator: bool,
}

/// The holes of a job: the vectors the form asks for, ordered, copied over the panel and clipped to the working area.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Selection {
    pub holes: Vec<crate::protocol::job::JobHole>,
    /// Holes left out because they fall outside the working area.
    pub skipped: usize,
}

/// Picks and orders the holes of `file` for a job. Coordinates are scaled, and the offset decides what falls outside the
/// working area; the holes returned are in file coordinates (the job planner adds the offset itself).
pub fn select_holes(
    file: &DrillFile,
    sorting: Sorting,
    opts: &JobOptions,
    unit_mm: f64,
    scale: (f64, f64),
    offset_mm: (f64, f64),
) -> Selection {
    use crate::protocol::job::JobHole;
    // the vectors asked for, in the order of the file
    let first = opts.from.max(1);
    let last = if opts.to == 0 { usize::MAX } else { opts.to };
    let kept: Vec<Hole> = file
        .holes
        .iter()
        .enumerate()
        .filter(|(i, _)| (first..=last).contains(&(i + 1)))
        .map(|(_, h)| h.clone())
        .collect();
    let mut tools: Vec<u8> = Vec::new();
    for h in &kept {
        if !tools.contains(&h.tool) {
            tools.push(h.tool);
        }
    }
    let sub = DrillFile {
        holes: kept,
        tools,
        diameters: Vec::new(),
        metric: file.metric,
    };
    let ordered = sort_holes(&sub, sorting);
    let base: Vec<JobHole> = ordered
        .iter()
        .map(|h| JobHole {
            tool: h.tool,
            x_mm: h.x_mil as f64 * unit_mm * scale.0,
            y_mm: h.y_mil as f64 * unit_mm * scale.1,
        })
        .collect();
    let copies: Vec<(f64, f64)> = match opts.panel {
        Some(p) if p.columns * p.rows > 1 => (0..p.rows)
            .flat_map(|iy| {
                (0..p.columns).map(move |ix| (ix as f64 * p.pitch_mm.0, iy as f64 * p.pitch_mm.1))
            })
            .collect(),
        _ => vec![(0.0, 0.0)],
    };
    let mut out = Vec::with_capacity(base.len() * copies.len());
    let mut skipped = 0;
    // one tool after the other (the tool changes are the operator's), every copy of its holes in turn
    let mut i = 0;
    while i < base.len() {
        let mut j = i;
        while j < base.len() && base[j].tool == base[i].tool {
            j += 1;
        }
        for &(dx, dy) in &copies {
            for h in &base[i..j] {
                let moved = JobHole {
                    tool: h.tool,
                    x_mm: h.x_mm + dx,
                    y_mm: h.y_mm + dy,
                };
                let inside = opts.clip_mm.is_none_or(|(cx, cy)| {
                    let (x, y) = (moved.x_mm + offset_mm.0, moved.y_mm + offset_mm.1);
                    x >= 0.0 && y >= 0.0 && x <= cx && y <= cy
                });
                if inside {
                    out.push(moved);
                } else {
                    skipped += 1;
                }
            }
        }
        i = j;
    }
    Selection {
        holes: out,
        skipped,
    }
}

#[cfg(test)]
mod tab_tests {
    use super::*;

    fn rectangle(w: f64, h: f64) -> Vec<(f64, f64)> {
        vec![(0.0, 0.0), (w, 0.0), (w, h), (0.0, h), (0.0, 0.0)]
    }

    fn length(p: &[(f64, f64)]) -> f64 {
        p.windows(2)
            .map(|s| (s[1].0 - s[0].0).hypot(s[1].1 - s[0].1))
            .sum()
    }

    #[test]
    fn two_tabs_on_each_side_of_a_rectangle_leave_eight_pieces_and_take_their_width_away() {
        let tabs = Tabs {
            per_side: 2,
            width_mm: 4.0,
            placement: TabPlacement::Even,
        };
        let pieces = cut_tabs(&rectangle(100.0, 50.0), &tabs);
        assert_eq!(pieces.len(), 8);
        let total: f64 = pieces.iter().map(|p| length(p)).sum();
        assert!((total - (300.0 - 8.0 * 4.0)).abs() < 1e-6, "{total}");
        // no point of any piece is inside a gap of the bottom side (the first tab is at a third of 100 mm)
        for p in &pieces {
            for &(x, y) in p {
                if y == 0.0 {
                    let (g0, g1) = (100.0 / 3.0 - 2.0, 100.0 / 3.0 + 2.0);
                    assert!(x <= g0 + 1e-6 || x >= g1 - 1e-6, "{x}");
                }
            }
        }
        // the pieces are in the order of the path, and the one that crosses the start is whole
        let around = pieces
            .iter()
            .find(|p| p.contains(&(0.0, 0.0)))
            .expect("the start");
        assert!(around.len() >= 3);
    }

    #[test]
    fn the_tabs_are_where_the_placement_says() {
        let r = rectangle(120.0, 80.0);
        for (placement, per_side) in [
            (TabPlacement::Even, 3),
            (TabPlacement::Ends, 2),
            (TabPlacement::Middle, 1),
            (TabPlacement::Middle, 2),
        ] {
            let tabs = Tabs {
                per_side,
                width_mm: 3.0,
                placement,
            };
            let pieces = cut_tabs(&r, &tabs);
            assert_eq!(
                pieces.len(),
                4 * per_side as usize,
                "{placement:?} {per_side}"
            );
        }
        // near the ends: the first tab of the bottom side is at 15 % of 120 mm = 18 mm
        let tabs = Tabs {
            per_side: 1,
            width_mm: 4.0,
            placement: TabPlacement::Ends,
        };
        let pieces = cut_tabs(&r, &tabs);
        assert!(pieces
            .iter()
            .any(|p| p.iter().any(|q| (q.0 - 16.0).abs() < 1e-6 && q.1 == 0.0)));
        assert!(pieces
            .iter()
            .any(|p| p.iter().any(|q| (q.0 - 20.0).abs() < 1e-6 && q.1 == 0.0)));
    }

    #[test]
    fn a_side_too_short_for_a_tab_and_a_path_that_is_not_closed_are_left_alone() {
        let tabs = Tabs {
            per_side: 1,
            width_mm: 10.0,
            placement: TabPlacement::Even,
        };
        // 12 mm sides cannot hold a 10 mm tab with half a tab of bridge at each end
        let small = rectangle(12.0, 12.0);
        assert_eq!(cut_tabs(&small, &tabs), vec![small.clone()]);
        // a side of 40 mm can, the short ones cannot
        let long = rectangle(40.0, 12.0);
        assert_eq!(cut_tabs(&long, &tabs).len(), 2);
        let open = vec![(0.0, 0.0), (50.0, 0.0), (50.0, 50.0)];
        assert_eq!(cut_tabs(&open, &tabs), vec![open.clone()]);
        let none = Tabs {
            per_side: 0,
            ..tabs
        };
        assert_eq!(cut_tabs(&long, &none), vec![long.clone()]);
    }
}
