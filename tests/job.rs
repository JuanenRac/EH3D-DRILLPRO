use eh3d_drillpro::job::*;
use eh3d_drillpro::ncd;

fn file() -> ncd::DrillFile {
    ncd::parse("%\nT01\nX1000Y1000\nX100Y100\nX900Y200\nT02\nX500Y500\nX0Y0\n").unwrap()
}

#[test]
fn tools_stay_grouped_in_order_of_first_use() {
    let h = sort_holes(&file(), Sorting::XFirst);
    let tools: Vec<u8> = h.iter().map(|h| h.tool).collect();
    assert_eq!(tools, vec![1, 1, 1, 2, 2]);
    assert_eq!(h[0].x_mil, 100);
}

#[test]
fn bounds() {
    let b = bounds_mm(&file().holes).unwrap();
    assert!((b.2 - 25.4).abs() < 1e-9 && b.0 == 0.0);
}

fn grid() -> ncd::DrillFile {
    // Scrambled 3 x 4 grid used on the original program.
    let pts = [
        (1000, 1500),
        (500, 500),
        (1500, 2000),
        (500, 2000),
        (1000, 500),
        (1500, 500),
        (500, 1000),
        (1000, 2000),
        (1500, 1000),
        (500, 1500),
        (1000, 1000),
        (1500, 1500),
    ];
    let mut text = String::from("%\r\nT01\r\n");
    for (x, y) in pts {
        text.push_str(&format!("X{x}Y{y}\r\n"));
    }
    ncd::parse(&text).unwrap()
}

fn xy(v: &[eh3d_drillpro::ncd::Hole]) -> Vec<(i32, i32)> {
    v.iter().map(|h| (h.x_mil, h.y_mil)).collect()
}

#[test]
fn special_sorting_is_a_meander_over_the_columns_like_the_original() {
    let h = sort_holes(&grid(), Sorting::Special);
    assert_eq!(
        xy(&h),
        vec![
            (500, 500),
            (500, 1000),
            (500, 1500),
            (500, 2000),
            (1000, 2000),
            (1000, 1500),
            (1000, 1000),
            (1000, 500),
            (1500, 500),
            (1500, 1000),
            (1500, 1500),
            (1500, 2000),
        ]
    );
}

#[test]
fn x_first_and_y_first_have_no_meander() {
    let x = sort_holes(&grid(), Sorting::XFirst);
    assert_eq!(
        xy(&x)[..8],
        [
            (500, 500),
            (500, 1000),
            (500, 1500),
            (500, 2000),
            (1000, 500),
            (1000, 1000),
            (1000, 1500),
            (1000, 2000)
        ]
    );
    let y = sort_holes(&grid(), Sorting::YFirst);
    assert_eq!(
        xy(&y)[..6],
        [
            (500, 500),
            (1000, 500),
            (1500, 500),
            (500, 1000),
            (1000, 1000),
            (1500, 1000)
        ]
    );
}

#[test]
fn none_keeps_the_file_order_inside_each_tool() {
    let h = sort_holes(&grid(), Sorting::None);
    assert_eq!(xy(&h)[..3], [(1000, 1500), (500, 500), (1500, 2000)]);
}

#[test]
fn the_tools_switched_off_leave_their_holes_out_of_the_job() {
    let all = select_holes(
        &file(),
        Sorting::XFirst,
        &JobOptions::default(),
        0.0254,
        (1.0, 1.0),
        (0.0, 0.0),
    );
    assert_eq!(all.holes.len(), 5);
    let o = JobOptions {
        skip_tools: vec![1],
        ..Default::default()
    };
    let some = select_holes(&file(), Sorting::XFirst, &o, 0.0254, (1.0, 1.0), (0.0, 0.0));
    assert_eq!(some.holes.len(), 2);
    assert!(some.holes.iter().all(|h| h.tool == 2));
}

fn shipped_machine() -> eh3d_drillpro::config::MachineSettings {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config/DRILLPRO.DCF");
    let text = eh3d_drillpro::config::bytes_to_text(&std::fs::read(path).unwrap());
    eh3d_drillpro::config::parse(&text)
        .unwrap()
        .settings()
        .unwrap()
}

#[test]
fn the_acceleration_of_the_machine_file_sets_the_length_of_the_ramps() {
    use eh3d_drillpro::protocol::connection::ConnectionConfig;
    let mut m = shipped_machine();
    assert_eq!((m.accel.0, m.accel.1), (10, 20), "the file as it ships");
    let plain = Geometry::default();
    let shipped = job_machine(Some(&m), plain, &ConnectionConfig::default(), 1.6);
    // the values that ship leave the ramps as they were
    assert_eq!(
        (
            shipped.geometry.ramp_steps,
            shipped.geometry.tuning.z_ramp_steps
        ),
        (plain.ramp_steps, plain.tuning.z_ramp_steps)
    );
    // "if the motors stall at the start of a move, try 5": the ramp is twice as long; a harder start is a shorter ramp
    m.accel = (5, 10);
    let gentle = job_machine(Some(&m), plain, &ConnectionConfig::default(), 1.6);
    assert_eq!(
        (
            gentle.geometry.ramp_steps,
            gentle.geometry.tuning.z_ramp_steps
        ),
        (80, 400)
    );
    m.accel = (20, 40);
    let hard = job_machine(Some(&m), plain, &ConnectionConfig::default(), 1.6);
    assert_eq!(
        (hard.geometry.ramp_steps, hard.geometry.tuning.z_ramp_steps),
        (20, 100)
    );
}

#[test]
fn the_spindle_start_time_of_the_machine_file_is_the_wait_for_each_ten_thousand_rpm() {
    use eh3d_drillpro::protocol::connection::ConnectionConfig;
    let mut m = shipped_machine();
    assert!((m.spindle_start_time - 0.5).abs() < 1e-9);
    let t = job_machine(
        Some(&m),
        Geometry::default(),
        &ConnectionConfig::default(),
        1.6,
    )
    .geometry
    .tuning;
    assert_eq!(
        (
            t.spindle_start_wait(10_000),
            t.spindle_start_wait(24_000),
            t.spindle_start_wait(60_000)
        ),
        (500, 1200, 3000)
    );
    // the measured 1680 ms of a machine at 60000 rpm is 0.28 s for each 10000
    m.spindle_start_time = 0.28;
    let t = job_machine(
        Some(&m),
        Geometry::default(),
        &ConnectionConfig::default(),
        1.6,
    )
    .geometry
    .tuning;
    assert_eq!(t.spindle_start_wait(60_000), 1680);
    // with no machine file the fixed wait stays
    let none = job_machine(None, Geometry::default(), &ConnectionConfig::default(), 1.6)
        .geometry
        .tuning;
    assert_eq!(none.spindle_start_wait(60_000), 1680);
}

fn changer_machine(places: usize) -> eh3d_drillpro::config::MachineSettings {
    use eh3d_drillpro::config::{rack_line, RackSetup};
    let mut m = shipped_machine();
    m.clip_mm = (300.0, 200.0);
    m.z_max_depth_mm = 40.0;
    m.auto_change = true;
    m.rack = RackSetup::default();
    m.atc = rack_line(
        (320.0, 40.0, 30.0),
        (320.0, 40.0 + 20.0 * (places as f64 - 1.0), 30.0),
        places,
    );
    m
}

#[test]
fn a_machine_changed_by_hand_or_with_no_places_has_no_rack() {
    let mut m = changer_machine(4);
    assert!(job_rack(Some(&m), 2).unwrap().is_some());
    m.auto_change = false;
    assert_eq!(job_rack(Some(&m), 2), Ok(None));
    let mut m = changer_machine(4);
    m.atc.clear();
    assert_eq!(job_rack(Some(&m), 2), Ok(None));
    assert_eq!(job_rack(None, 2), Ok(None));
}

#[test]
fn a_job_the_rack_cannot_hold_or_reach_is_refused_with_its_reason() {
    let m = changer_machine(3);
    assert_eq!(job_rack(Some(&m), 4), Err(RackRefusal::TooManyTools(3)));
    // a place with no position
    let mut gap = changer_machine(3);
    gap.atc.retain(|a| a.index != 2);
    assert_eq!(job_rack(Some(&gap), 3), Err(RackRefusal::Gap(2)));
    // a bar further than the machine reaches
    let mut far = changer_machine(3);
    far.rack.reach_x_mm = 5.0;
    assert!(matches!(
        job_rack(Some(&far), 3),
        Err(RackRefusal::Outside(_))
    ));
    // the words are in the language of the person
    let text = RackRefusal::Gap(2).message(eh3d_drillpro::i18n::Lang::En);
    assert!(text.ends_with(": 2"), "{text}");
}

#[test]
fn the_places_given_to_the_tools_are_checked_against_the_machine_file_and_go_into_the_rack() {
    let m = changer_machine(5);
    let given = JobOptions {
        rack_slots: vec![(1, 4), (2, 2)],
        ..JobOptions::default()
    };
    let rack = job_rack_for(Some(&m), 2, &given).unwrap().unwrap();
    assert_eq!(rack.slots, vec![(1, 4), (2, 2)]);
    assert_eq!(rack.exchange_of(0, 1, None), (0, 4));
    assert_eq!(rack.exchange_of(1, 2, Some(1)), (4, 2));
    assert_eq!(rack.return_place(2, Some(2)), 2);
    // the places need not be the first ones in a row: a gap in the places that are not used does not matter
    let mut gap = changer_machine(5);
    gap.atc.retain(|a| a.index != 1 && a.index != 3);
    assert!(job_rack_for(Some(&gap), 2, &given).is_ok());
    // a place the tool was given that the machine file does not define is refused, with its number
    let missing = JobOptions {
        rack_slots: vec![(1, 4), (2, 3)],
        ..JobOptions::default()
    };
    assert_eq!(
        job_rack_for(Some(&gap), 2, &missing),
        Err(RackRefusal::Gap(3))
    );
    // so is one past the end of the bar
    let past = JobOptions {
        rack_slots: vec![(1, 40)],
        ..JobOptions::default()
    };
    assert_eq!(job_rack_for(Some(&m), 1, &past), Err(RackRefusal::Gap(40)));
    // and the place of the tool the chuck comes holding is checked too
    let carried = JobOptions {
        rack_first: 2,
        rack_carry_in: true,
        rack_slots: vec![(5, 1)],
        rack_carry_slot: Some(40),
        ..JobOptions::default()
    };
    assert_eq!(
        job_rack_for(Some(&m), 1, &carried),
        Err(RackRefusal::Gap(40))
    );
    // a bar further than the machine reaches is refused whatever the places
    let mut far = changer_machine(5);
    far.rack.reach_x_mm = 5.0;
    assert!(matches!(
        job_rack_for(Some(&far), 2, &given),
        Err(RackRefusal::Outside(_))
    ));
    // a machine changed by hand has no rack, with or without places
    let mut hand = changer_machine(5);
    hand.auto_change = false;
    assert_eq!(job_rack_for(Some(&hand), 2, &given), Ok(None));
}

#[test]
fn without_places_the_rack_of_a_job_is_the_one_of_always() {
    let m = changer_machine(5);
    let plain = job_rack_for(Some(&m), 2, &JobOptions::default())
        .unwrap()
        .unwrap();
    assert!(plain.slots.is_empty() && plain.carry_slot.is_none());
    assert_eq!(plain.exchange_of(0, 1, None), plain.exchange(0));
    let chained = JobOptions {
        rack_first: 2,
        rack_carry_in: true,
        ..JobOptions::default()
    };
    let r = job_rack_for(Some(&m), 1, &chained).unwrap().unwrap();
    assert_eq!((r.first, r.carry_in), (2, true));
    assert_eq!(r.exchange_of(0, 9, None), (2, 3));
}
