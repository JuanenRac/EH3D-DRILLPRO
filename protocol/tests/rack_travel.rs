//! The tool change of the machines with the changer when the chuck goes over the tools of the bar: the used tool is left in its own place,
//! the chuck goes up over the bar, to the place of the next tool, and takes it. Nothing here is a recording of a real machine: it is
//! checked on the planned steps, walking them as the control unit would.

use eh3d_drillpro_protocol::job::*;
use eh3d_drillpro_protocol::motion::Geometry;

fn machine() -> JobMachine {
    let mut geometry = Geometry::with_correction(0.9993, 0.9990, 0.8988);
    geometry.tuning.z_search_steps = 0;
    JobMachine {
        geometry,
        offset_mm: (0.0, 0.0),
        max_depth_mm: 37.5,
        base_mm: 6.0,
        board_mm: 1.5,
        distance_tool_board_mm: 3.0,
        xy_search_steps: 26613,
    }
}

const TRAVEL: RackTravel = RackTravel {
    tool_length_mm: 38.0,
    ring_to_tip_mm: 21.0,
    clearance_mm: 8.0,
    approach_mm: -8.0,
};

/// A bar of the first place (the test pin) and five tools, 20 mm apart along Y.
fn rack() -> Rack {
    Rack::new(
        (0..7)
            .map(|i| (540.0, 20.0 + 20.0 * i as f64, 37.4))
            .collect(),
    )
    .with_travel(TRAVEL)
}

fn tool(n: u8) -> JobTool {
    JobTool {
        number: n,
        extra_stroke_mm: 0.1,
        feed_mm_min: 100,
        spindle_rpm: 60_000,
    }
}

fn holes() -> Vec<JobHole> {
    // the tools in the order of the job: 1, 2, 3, 4
    [
        (1, 50.0, 40.0),
        (1, 70.0, 60.0),
        (2, 90.0, 40.0),
        (3, 20.0, 80.0),
        (4, 120.0, 30.0),
    ]
    .iter()
    .map(|&(t, x, y)| JobHole {
        tool: t,
        x_mm: x,
        y_mm: y,
    })
    .collect()
}

/// What the control unit does, step by step: the position in steps (X, Y, depth of Z) after every motor byte and the latch bytes where
/// they are sent.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Event {
    Moved { x: i32, y: i32, z: i32, xy: bool },
    Latch { byte: u8, x: i32, y: i32, z: i32 },
}

fn walk(bytes: &[u8]) -> Vec<Event> {
    let (mut x, mut y, mut z) = (0i32, 0i32, 0i32);
    let mut out = Vec::new();
    for &b in bytes {
        if b & 0x80 != 0 {
            out.push(Event::Latch { byte: b, x, y, z });
            continue;
        }
        let over = b & 0x40 != 0;
        let mut xy = false;
        for (step, dir, pos, up_is_plus, is_xy) in [
            (0x01u8, 0x02u8, &mut x, false, true),
            (0x04, 0x08, &mut y, false, true),
            (0x10, 0x20, &mut z, true, false),
        ] {
            if b & step == 0 {
                continue;
            }
            xy |= is_xy;
            let plus = (b & dir != 0) != up_is_plus;
            if plus {
                *pos += 1;
            } else if *pos > 0 || over {
                *pos -= 1;
            }
        }
        out.push(Event::Moved { x, y, z, xy });
    }
    out
}

fn plan() -> (Vec<Item>, Vec<Event>) {
    let tools = [tool(1), tool(2), tool(3), tool(4)];
    let items = plan_drill_job_atc(&machine(), &tools, &holes(), &rack());
    let events = walk(&flatten_bytes(&items));
    (items, events)
}

/// The place of the rack nearest to a point in steps, and how far it is in mm.
fn nearest(g: &Geometry, rack: &Rack, x: i32, y: i32) -> (usize, f64) {
    rack.places
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let dx = (x - g.x_steps(p.0)) as f64 / g.x_steps(1.0) as f64;
            let dy = (y - g.y_steps(p.1)) as f64 / g.y_steps(1.0) as f64;
            (i, dx.hypot(dy))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap()
}

#[test]
fn the_used_tool_is_left_in_its_place_before_the_next_one_is_taken_one_place_after_the_other() {
    let g = machine().geometry;
    let r = rack();
    let (_, events) = plan();
    // the chuck opens (0xA0) where it leaves the tool and closes (0x80) where it takes the next one
    let at: Vec<(u8, usize, f64)> = events
        .iter()
        .filter_map(|e| match *e {
            // (the latch byte 0x80 is also the spindle stopped, over the holes: the chuck is the ones at the bar)
            Event::Latch { byte, x, y, z }
                if (byte == 0xA0 || byte == 0x80) && x > g.x_steps(500.0) =>
            {
                // every opening and closing is made down in a place, at its depth
                assert_eq!(
                    z,
                    g.z_steps(37.4) as i32,
                    "the chuck works at the depth of the place"
                );
                let (i, d) = nearest(&g, &r, x, y);
                assert!(d < 0.2, "at the place {i}, {d} mm off");
                Some((byte, i, d))
            }
            _ => None,
        })
        .collect();
    let order: Vec<(u8, usize)> = at.iter().map(|&(b, i, _)| (b, i)).collect();
    // the test pin is in the chuck at the start; four tools follow; the last one goes back and the pin is taken again
    assert_eq!(
        order,
        vec![
            (0xA0, 0),
            (0x80, 1),
            (0xA0, 1),
            (0x80, 2),
            (0xA0, 2),
            (0x80, 3),
            (0xA0, 3),
            (0x80, 4),
            (0xA0, 4),
            (0x80, 0),
        ]
    );
}

#[test]
fn nothing_moves_across_the_bar_below_the_height_of_the_tools_except_into_and_out_of_a_place_from_its_free_side(
) {
    let g = machine().geometry;
    let r = rack();
    let lane = g.z_steps(r.lane_mm()) as i32;
    assert!(
        lane > 0 && lane < g.z_steps(37.4) as i32,
        "the lane is over the tools: {lane}"
    );
    let (_, events) = plan();
    let mut deep_moves = 0;
    for e in &events {
        let Event::Moved { x, y, z, xy } = *e else {
            continue;
        };
        // (over the board the holes are drilled with the head at the height it travels between them: only the bar is of interest)
        if !xy || z <= lane + 1 || x <= g.x_steps(500.0) {
            continue;
        }
        // below the height of the tools a move across is only the way in or out of one place: on its row and between the free side and the place
        deep_moves += 1;
        let (i, d) = nearest(&g, &r, x, y);
        let p = r.places[i];
        let (px, py) = (g.x_steps(p.0), g.y_steps(p.1));
        let (ax, _) = (g.x_steps(p.0 + TRAVEL.approach_mm), 0);
        assert!(
            (y - py).abs() <= 1 && x >= ax.min(px) - 1 && x <= ax.max(px) + 1,
            "a move across the bar at depth {z} (lane {lane}) at ({x}, {y}), {d:.1} mm from the place {i}"
        );
        // and no other place is anywhere near
        for (j, q) in r.places.iter().enumerate() {
            if j != i {
                let dx = (x - g.x_steps(q.0)) as f64 / g.x_steps(1.0) as f64;
                let dy = (y - g.y_steps(q.1)) as f64 / g.y_steps(1.0) as f64;
                assert!(
                    dx.hypot(dy) > 8.0,
                    "passes {} mm from the place {j}",
                    dx.hypot(dy)
                );
            }
        }
    }
    assert!(deep_moves > 0, "the way in to the places was walked");
}

#[test]
fn the_chuck_is_at_home_before_it_leaves_a_tool_and_after_it_takes_one_and_goes_down_only_at_the_free_side_of_the_place(
) {
    let g = machine().geometry;
    let (items, events) = plan();
    // before the first latch of the chuck and after each closing the chuck is up (the tool held goes to the holes at home height)
    let mut last_z = 0;
    let mut closings = 0;
    for (n, e) in events.iter().enumerate() {
        match *e {
            Event::Moved { z, .. } => last_z = z,
            Event::Latch { byte: 0x80, z, .. } if z > 0 => {
                closings += 1;
                // after the closing the chuck leaves the place and rises to home before anything else moves in XY far away
                let next_far = events[n..].iter().find_map(|e| match *e {
                    Event::Moved { x, z, xy: true, .. }
                        if (x - g.x_steps(532.0)).abs() > g.x_steps(20.0) =>
                    {
                        Some(z)
                    }
                    _ => None,
                });
                assert_eq!(next_far, Some(0), "it is at home when it leaves the bar");
            }
            _ => {}
        }
    }
    assert_eq!(closings, 5);
    assert_eq!(last_z, 0);
    assert!(!items.is_empty());
}

#[test]
fn the_exchange_recorded_on_the_original_machine_is_still_what_a_rack_without_the_travel_does() {
    let tools = [tool(1), tool(2)];
    let plain = Rack::new(
        (0..4)
            .map(|i| (540.0, 20.0 + 20.0 * i as f64, 37.4))
            .collect(),
    );
    let over = Rack::new(
        (0..4)
            .map(|i| (540.0, 20.0 + 20.0 * i as f64, 37.4))
            .collect(),
    )
    .with_travel(TRAVEL);
    let a = flatten_bytes(&plan_drill_job_atc(
        &machine(),
        &tools,
        &holes()[..3],
        &plain,
    ));
    let b = flatten_bytes(&plan_drill_job_atc(
        &machine(),
        &tools,
        &holes()[..3],
        &over,
    ));
    assert_ne!(a, b, "the chuck goes another way");
    // the same number of times the chuck opens and closes in both
    let count = |bytes: &[u8], v: u8| bytes.iter().filter(|&&x| x == v).count();
    assert_eq!(count(&a, 0xA0), count(&b, 0xA0));
}

#[test]
fn a_place_outside_the_reach_of_the_machine_is_found() {
    let r = rack();
    assert_eq!(r.first_outside(6, (700.0, 400.0, 38.0)), None);
    assert_eq!(r.first_outside(6, (700.0, 100.0, 38.0)), Some(5));
    assert_eq!(r.first_outside(3, (700.0, 100.0, 38.0)), None);
    assert_eq!(r.first_outside(6, (500.0, 400.0, 38.0)), Some(0));
    assert_eq!(r.first_outside(6, (700.0, 400.0, 30.0)), Some(0));
}

#[test]
fn the_lane_is_over_the_shanks_of_the_tools_by_the_clearance_and_never_above_home() {
    let r = rack();
    // 37.4 - (38 - 21) - 8
    assert!((r.lane_mm() - 12.4).abs() < 1e-9);
    let tall = Rack::new(vec![(0.0, 0.0, 10.0)]).with_travel(TRAVEL);
    assert_eq!(tall.lane_mm(), 0.0);
    assert_eq!(Rack::new(vec![(0.0, 0.0, 10.0)]).lane_mm(), 0.0);
}
