//! The tools of a job in the places of the rack the person chose, which are not the places the order of the job would give them: the tool is taken from its
//! own place and the tool that goes back is put in its own. Checked on the planned steps, walking them as the control unit would.

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

/// The test pin (place 0) and six places, 20 mm apart along Y at X 540: the place `i` is at Y = 20 + 20 i mm.
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

/// Four tools, used in the order 1, 2, 3, 4.
fn holes() -> Vec<JobHole> {
    [
        (1, 50.0, 40.0),
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

/// The position in steps (X, Y) at every latch byte that opens (0xA0) or closes (0x80) the chuck at the bar, in the order they are sent.
fn chuck_events(bytes: &[u8], g: &Geometry) -> Vec<(u8, i32, i32)> {
    let (mut x, mut y) = (0i32, 0i32);
    let mut out = Vec::new();
    for &b in bytes {
        if b & 0x80 != 0 {
            if (b == 0xA0 || b == 0x80) && x > g.x_steps(500.0) {
                out.push((b, x, y));
            }
            continue;
        }
        let over = b & 0x40 != 0;
        for (step, dir, pos) in [(0x01u8, 0x02u8, &mut x), (0x04, 0x08, &mut y)] {
            if b & step == 0 {
                continue;
            }
            if b & dir != 0 {
                *pos += 1;
            } else if *pos > 0 || over {
                *pos -= 1;
            }
        }
    }
    out
}

/// The place of the rack each event happened at, and whether the chuck opened (the tool stays) or closed (the tool is taken).
fn places_of(rack: &Rack, events: &[(u8, i32, i32)], g: &Geometry) -> Vec<(char, usize)> {
    events
        .iter()
        .map(|&(byte, x, y)| {
            let (i, d) = rack
                .places
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let dx = (x - g.x_steps(p.0)) as f64 / g.x_steps(1.0) as f64;
                    let dy = (y - g.y_steps(p.1)) as f64 / g.y_steps(1.0) as f64;
                    (i, dx.hypot(dy))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            assert!(d < 0.2, "the chuck works {d} mm from the place {i}");
            (if byte == 0xA0 { 'o' } else { 'c' }, i)
        })
        .collect()
}

fn plan_with(rack: Rack) -> Vec<(char, usize)> {
    let g = machine().geometry;
    let tools = [tool(1), tool(2), tool(3), tool(4)];
    let items = plan_drill_job_atc(&machine(), &tools, &holes(), &rack);
    let bytes = flatten_bytes(&items);
    places_of(&rack, &chuck_events(&bytes, &g), &g)
}

#[test]
fn without_places_the_tools_are_taken_from_the_places_in_the_order_of_the_job() {
    let order = plan_with(rack());
    assert_eq!(
        order,
        vec![
            ('o', 0),
            ('c', 1),
            ('o', 1),
            ('c', 2),
            ('o', 2),
            ('c', 3),
            ('o', 3),
            ('c', 4),
            ('o', 4),
            ('c', 0),
        ]
    );
}

#[test]
fn a_tool_is_taken_from_the_place_it_was_given_and_put_back_in_that_place() {
    // tool 1 is in place 6, tool 2 in 3, tool 3 in 5 and tool 4 in 1: not in the order of the job
    let placed = rack().with_slots(vec![(1, 6), (2, 3), (3, 5), (4, 1)], None);
    let order = plan_with(placed);
    assert_eq!(
        order,
        vec![
            ('o', 0),
            ('c', 6),
            ('o', 6),
            ('c', 3),
            ('o', 3),
            ('c', 5),
            ('o', 5),
            ('c', 1),
            ('o', 1),
            ('c', 0),
        ]
    );
}

#[test]
fn the_places_of_a_job_in_a_load_follow_the_places_given_and_the_chuck_comes_from_the_place_of_the_tool_it_holds(
) {
    // the job before left its last tool (in place 5) in the chuck; this job's tools are in 2 and 4
    let r = rack()
        .in_load(3, true, false)
        .with_slots(vec![(7, 2), (8, 4)], Some(5));
    assert_eq!(r.start_place(), 5);
    assert_eq!(r.exchange_of(0, 7, None), (5, 2));
    assert_eq!(r.exchange_of(1, 8, Some(7)), (2, 4));
    assert_eq!(r.return_place(2, Some(8)), 4);
    // a tool without a place is taken from the place the order of the job gives it
    assert_eq!(r.exchange_of(1, 9, Some(7)), (2, 3 + 1 + 1));
    // with no places at all the answers are the ones of always
    let plain = rack().in_load(2, true, false);
    assert_eq!(plain.exchange_of(0, 1, None), plain.exchange(0));
    assert_eq!(plain.exchange_of(1, 2, Some(1)), plain.exchange(1));
    assert_eq!(plain.return_place(2, Some(2)), 4);
    assert_eq!(plain.return_place(0, None), 2);
    // a chuck that comes holding a tool without a place for it starts from the place `first`
    assert_eq!(
        rack()
            .in_load(2, true, false)
            .with_slots(vec![], None)
            .start_place(),
        2
    );
}

#[test]
fn a_job_that_hands_its_tool_on_leaves_it_in_the_chuck_and_the_next_puts_it_back_in_the_place_given(
) {
    let g = machine().geometry;
    let tools = [tool(1), tool(2)];
    let holes = [holes()[0], holes()[1]];
    let first = rack()
        .in_load(0, false, true)
        .with_slots(vec![(1, 4), (2, 2)], None);
    let items = plan_drill_job_atc(&machine(), &tools, &holes, &first);
    let order = places_of(&first, &chuck_events(&flatten_bytes(&items), &g), &g);
    // the pin goes, tool 1 comes from 4 and goes back to 4, tool 2 comes from 2 and stays in the chuck
    assert_eq!(order, vec![('o', 0), ('c', 4), ('o', 4), ('c', 2)]);
    // the next job holds tool 2 (place 2): the first thing it does is put it back in place 2 and take its own from place 6
    let next = rack()
        .in_load(2, true, false)
        .with_slots(vec![(9, 6)], Some(2));
    let more = [JobHole {
        tool: 9,
        x_mm: 40.0,
        y_mm: 40.0,
    }];
    let items = plan_drill_job_atc(&machine(), &[tool(9)], &more, &next);
    let order = places_of(&next, &chuck_events(&flatten_bytes(&items), &g), &g);
    assert_eq!(order, vec![('o', 2), ('c', 6), ('o', 6), ('c', 0)]);
}
