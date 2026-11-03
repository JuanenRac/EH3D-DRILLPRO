//! More cases of the rack of the changer and of the plan of a job as bytes: the gaps, the places that are out of reach, the lane over the tools, and the run lengths of what is sent.

use eh3d_drillpro_protocol::job::*;
use eh3d_drillpro_protocol::Step;

const TRAVEL: RackTravel = RackTravel {
    tool_length_mm: 38.0,
    ring_to_tip_mm: 21.0,
    clearance_mm: 8.0,
    approach_mm: -8.0,
};

fn five() -> Rack {
    Rack::new(vec![
        (10.0, 10.0, 30.0),
        (10.0, 30.0, 30.0),
        (10.0, 50.0, 30.0),
        (10.0, 70.0, 30.0),
        (10.0, 90.0, 30.0),
    ])
}

#[test]
fn a_new_rack_has_every_place_defined_and_no_travel_and_no_gap() {
    let r = five();
    assert_eq!(r.defined, vec![true; 5]);
    assert_eq!(r.travel, None);
    assert_eq!(r.first_gap(4), None);
    assert_eq!(r.lane_mm(), 0.0);
}

#[test]
fn the_first_gap_is_the_first_place_with_no_position_and_a_place_that_does_not_exist_is_a_gap() {
    let mut r = five();
    r.defined[2] = false;
    r.defined[4] = false;
    assert_eq!(r.first_gap(4), Some(2));
    assert_eq!(r.first_gap(1), None);
    assert_eq!(r.first_gap(2), Some(2));
    // asking for more places than the rack has: the first one past its end is a gap
    assert_eq!(five().first_gap(7), Some(5));
    // an empty rack has a gap at place 0
    assert_eq!(Rack::new(vec![]).first_gap(0), Some(0));
}

#[test]
fn a_place_with_a_short_list_of_defined_flags_counts_as_a_gap() {
    let mut r = five();
    r.defined.truncate(3);
    assert_eq!(r.first_gap(4), Some(3));
}

#[test]
fn a_place_is_outside_when_any_of_its_three_numbers_is_negative_or_past_the_reach() {
    let reach = (100.0, 100.0, 40.0);
    assert_eq!(five().first_outside(4, reach), None);
    let mut r = five();
    r.places[1].0 = -0.1;
    assert_eq!(r.first_outside(4, reach), Some(1));
    let mut r = five();
    r.places[3].1 = 100.1;
    assert_eq!(r.first_outside(4, reach), Some(3));
    let mut r = five();
    r.places[2].2 = 40.5;
    assert_eq!(r.first_outside(4, reach), Some(2));
    // the edges are inside
    let mut r = five();
    r.places[0] = (0.0, 0.0, 0.0);
    r.places[1] = (100.0, 100.0, 40.0);
    assert_eq!(r.first_outside(4, reach), None);
}

#[test]
fn only_the_places_up_to_the_last_one_asked_for_are_looked_at() {
    let mut r = five();
    r.places[4].0 = 1000.0;
    assert_eq!(r.first_outside(3, (100.0, 100.0, 40.0)), None);
    assert_eq!(r.first_outside(4, (100.0, 100.0, 40.0)), Some(4));
    // asking past the end of a rack does not go past it
    assert_eq!(five().first_outside(99, (100.0, 100.0, 40.0)), None);
    assert_eq!(Rack::new(vec![]).first_outside(3, (1.0, 1.0, 1.0)), None);
}

#[test]
fn the_lane_is_the_shallowest_place_less_the_shank_over_the_ring_and_the_clearance() {
    let r = five().with_travel(TRAVEL);
    // 30 - (38 - 21) - 8
    assert!((r.lane_mm() - 5.0).abs() < 1e-9, "{}", r.lane_mm());
    let mut shallow = five().with_travel(TRAVEL);
    shallow.places[2].2 = 20.0;
    assert!((shallow.lane_mm() - 0.0).abs() < 1e-9 || shallow.lane_mm() >= 0.0);
    assert!((shallow.lane_mm() - (20.0 - 17.0 - 8.0f64).max(0.0)).abs() < 1e-9);
}

#[test]
fn the_lane_never_goes_above_home_and_a_gap_does_not_count_for_the_top() {
    let mut r = five().with_travel(TRAVEL);
    r.places[0].2 = 2.0;
    assert_eq!(r.lane_mm(), 0.0);
    // the shallowest place has no position: the lane follows the others
    r.defined[0] = false;
    assert!((r.lane_mm() - 5.0).abs() < 1e-9);
    // no place has a position at all
    let mut none = five().with_travel(TRAVEL);
    none.defined = vec![false; 5];
    assert_eq!(none.lane_mm(), 0.0);
}

#[test]
fn a_tool_shorter_than_the_distance_from_the_ring_to_its_tip_leaves_no_shank_to_clear() {
    let travel = RackTravel {
        tool_length_mm: 10.0,
        ring_to_tip_mm: 21.0,
        ..TRAVEL
    };
    let r = five().with_travel(travel);
    assert!((r.lane_mm() - (30.0 - 8.0)).abs() < 1e-9);
}

#[test]
fn the_run_lengths_of_a_sequence_of_bytes_count_the_equal_ones_in_a_row() {
    assert_eq!(run_lengths(&[]), vec![]);
    assert_eq!(run_lengths(&[7]), vec![(7, 1)]);
    assert_eq!(
        run_lengths(&[1, 1, 1, 2, 2, 1]),
        vec![(1, 3), (2, 2), (1, 1)]
    );
    let all_the_same = vec![9u8; 1000];
    assert_eq!(run_lengths(&all_the_same), vec![(9, 1000)]);
    // the total is always the number of bytes
    let mixed: Vec<u8> = (0..200u32).map(|i| (i / 3 % 4) as u8).collect();
    assert_eq!(
        run_lengths(&mixed).iter().map(|r| r.1).sum::<usize>(),
        mixed.len()
    );
}

#[test]
fn the_bytes_of_a_plan_are_the_steps_in_order_and_pauses_and_spindle_orders_are_not_bytes() {
    let step = |byte: u8| Step {
        byte,
        delay_us: 1.0,
    };
    let items = vec![
        Item::Steps(vec![step(1), step(2)]),
        Item::Spindle {
            rpm: 24000,
            wait_ms: 100,
        },
        Item::Search(vec![step(3)]),
        Item::Seek(vec![step(4), step(5)]),
        Item::Steps(vec![]),
    ];
    assert_eq!(flatten_bytes(&items), vec![1, 2, 3, 4, 5]);
    assert!(flatten_bytes(&[]).is_empty());
}
