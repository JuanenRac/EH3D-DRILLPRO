//! The whole drilling job of the original program, recorded in an emulator for a three-hole file
//! (1000/1000, 2000/1000, 2000/2000 mil, one tool at 60000 rpm) on the manual machine
//! (corrections X 0.9992, Y 0.9998, Z 0.8988; offset 2.5 mm; maximum depth 37.30; base 12;
//! board 1.00; distance tool-board 16.00).

use eh3d_drillpro_protocol::job::*;
use eh3d_drillpro_protocol::motion::Geometry;

fn machine() -> JobMachine {
    // The original searches Z for its maximum depth only; the machine file's longer search (`z_search_steps`) is left out here.
    let mut geometry = Geometry::with_correction(0.9992, 0.9998, 0.8988);
    geometry.tuning.z_search_steps = 0;
    JobMachine {
        geometry,
        offset_mm: (2.5, 2.5),
        max_depth_mm: 37.30,
        base_mm: 12.0,
        board_mm: 1.0,
        distance_tool_board_mm: 16.0,
        xy_search_steps: 26613,
    }
}

fn mil(v: f64) -> f64 {
    v * 0.0254
}

#[test]
fn three_holes_one_tool_match_the_recorded_job_exactly() {
    let tools = [JobTool {
        number: 1,
        extra_stroke_mm: 0.0,
        feed_mm_min: 1000,
        spindle_rpm: 60_000,
    }];
    let holes = [
        JobHole {
            tool: 1,
            x_mm: mil(1000.0),
            y_mm: mil(1000.0),
        },
        JobHole {
            tool: 1,
            x_mm: mil(2000.0),
            y_mm: mil(1000.0),
        },
        JobHole {
            tool: 1,
            x_mm: mil(2000.0),
            y_mm: mil(2000.0),
        },
    ];
    let plan = plan_drill_job(&machine(), &tools, &holes);
    let got = run_lengths(&flatten_bytes(&plan));
    // Recorded from the original (run-length: byte, count).
    let expected: Vec<(u8, usize)> = vec![
        (0xD0, 2),
        (0x30, 1320),
        (0x50, 45),
        (0x30, 54),
        (0x05, 26613),
        (0x4F, 200),
        (0x05, 250),
        (0x4F, 1098),
        (0xD0, 1),
        (0xF8, 1),
        (0x90, 1),
        (0x9F, 1),
        (0x50, 912),
        (0x30, 601),
        (0x4B, 999),
        (0x50, 601),
        (0x30, 601),
        (0x4E, 1000),
        (0x50, 601),
        (0x30, 912),
        (0xD2, 1),
        (0x80, 1),
        (0xD0, 2),
        (0x50, 45),
        (0x30, 54),
        // return to the origin (2097/2098 steps, Y major: the X step is skipped once, in the middle)
        // followed by the diagonal search
        (0x05, 1049),
        (0x04, 1),
        (0x05, 27661),
        (0x4F, 200),
        (0x05, 250),
        (0xC0, 1),
    ];
    assert_eq!(got, expected);
}

#[test]
fn the_plan_contains_the_operator_pauses_in_order() {
    let tools = [JobTool {
        number: 1,
        extra_stroke_mm: 0.0,
        feed_mm_min: 1000,
        spindle_rpm: 60_000,
    }];
    let holes = [JobHole {
        tool: 1,
        x_mm: 10.0,
        y_mm: 10.0,
    }];
    let plan = plan_drill_job(&machine(), &tools, &holes);
    let pauses: Vec<Pause> = plan
        .iter()
        .filter_map(|i| {
            if let Item::Pause(p) = i {
                Some(*p)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        pauses,
        vec![Pause::ToolChange { tool: 1 }, Pause::SpindleStart]
    );
}

#[test]
fn three_tools_use_the_recorded_spindle_levels_and_stop_between_tools() {
    // Recorded with spindle speeds 36000, 42000 and 54000: levels 3, 6 and 12 (bytes 93, 96, 9C).
    let tools = [
        JobTool {
            number: 1,
            extra_stroke_mm: 0.0,
            feed_mm_min: 1000,
            spindle_rpm: 36_000,
        },
        JobTool {
            number: 2,
            extra_stroke_mm: 0.0,
            feed_mm_min: 1000,
            spindle_rpm: 42_000,
        },
        JobTool {
            number: 3,
            extra_stroke_mm: 0.0,
            feed_mm_min: 1000,
            spindle_rpm: 54_000,
        },
    ];
    let holes = [
        JobHole {
            tool: 1,
            x_mm: mil(500.0),
            y_mm: mil(500.0),
        },
        JobHole {
            tool: 1,
            x_mm: mil(3000.0),
            y_mm: mil(500.0),
        },
        JobHole {
            tool: 2,
            x_mm: mil(1000.0),
            y_mm: mil(2500.0),
        },
        JobHole {
            tool: 3,
            x_mm: mil(2500.0),
            y_mm: mil(3000.0),
        },
    ];
    let bytes = flatten_bytes(&plan_drill_job(&machine(), &tools, &holes));
    let levels: Vec<u8> = bytes
        .windows(2)
        .filter(|w| w[0] == 0x90 && w[1] & 0xF0 == 0x90 && w[1] != 0x90)
        .map(|w| w[1])
        .collect();
    assert_eq!(levels, vec![0x93, 0x96, 0x9C]);
    let stops = bytes.iter().filter(|&&b| b == 0xD2).count();
    assert_eq!(stops, 3, "the spindle is stopped after every tool");
}

#[test]
fn with_the_nodes_spindle_the_job_asks_the_node_instead_of_writing_the_latches() {
    let mut m = machine();
    m.geometry.tuning.spindle_node = true;
    let tools = [JobTool {
        number: 1,
        extra_stroke_mm: 0.0,
        feed_mm_min: 1000,
        spindle_rpm: 24_000,
    }];
    let holes = [JobHole {
        tool: 1,
        x_mm: mil(1000.0),
        y_mm: mil(1000.0),
    }];
    let items = plan_drill_job(&m, &tools, &holes);
    let spindle: Vec<(u32, u32)> = items
        .iter()
        .filter_map(|i| match i {
            Item::Spindle { rpm, wait_ms } => Some((*rpm, *wait_ms)),
            _ => None,
        })
        .collect();
    assert_eq!(spindle, vec![(24_000, 1680), (0, 1000)]);
    // the control unit's spindle bits (run in latch A, enable in latch B) are not written
    let bytes = flatten_bytes(&items);
    assert!(!bytes.iter().any(|&b| b == 0x90 || b == 0xA8 || b == 0xE8));
}

#[test]
fn the_vacuum_output_goes_on_with_the_spindle_and_off_with_it_only_when_asked() {
    let tools = [JobTool {
        number: 1,
        extra_stroke_mm: 0.0,
        feed_mm_min: 1000,
        spindle_rpm: 60_000,
    }];
    let holes = [JobHole {
        tool: 1,
        x_mm: mil(1000.0),
        y_mm: mil(1000.0),
    }];
    // as the original: the auxiliary bit of latch A (0x20) is never written
    let plain = flatten_bytes(&plan_drill_job(&machine(), &tools, &holes));
    assert!(!plain.iter().any(|&b| (0xA0..=0xBF).contains(&b)));
    // asked for: it goes out with the spindle at its speed (0x9F | 0x20) and the latch goes back to 0x80 with the stop
    let mut m = machine();
    m.geometry.tuning.vacuum_aux = true;
    let bytes = flatten_bytes(&plan_drill_job(&m, &tools, &holes));
    assert_eq!(bytes.iter().filter(|&&b| b == 0xBF).count(), 1);
    let on = bytes.iter().position(|&b| b == 0xBF).unwrap();
    assert_eq!(bytes[on - 1], 0x9F, "right after the speed level");
    let stop = bytes.iter().position(|&b| b == 0xD2).unwrap();
    assert!(on < stop);
    assert_eq!(
        bytes[stop + 1],
        0x80,
        "the stop clears run and auxiliary together"
    );
    assert!(!bytes[stop..].iter().any(|&b| (0xA0..=0xBF).contains(&b)));
}

// ---------------------------------------------------------------------------------------------- the tool changer

/// Where the control unit's latch bytes go out and where the head is then (steps), following the bytes as the control unit does:
/// a step towards a switch is swallowed at the switch unless the limit override bit is set.
fn latch_events(bytes: &[u8]) -> Vec<(u8, i32, i32, i32)> {
    let (mut x, mut y, mut z) = (0i32, 0i32, 0i32);
    let mut out = Vec::new();
    for &b in bytes {
        if b & 0x80 != 0 {
            out.push((b, x, y, z));
            continue;
        }
        let over = b & 0x40 != 0;
        for (step, dir, pos, up_is_plus) in [
            (0x01u8, 0x02u8, &mut x, false),
            (0x04, 0x08, &mut y, false),
            (0x10, 0x20, &mut z, true),
        ] {
            if b & step == 0 {
                continue;
            }
            let plus = (b & dir != 0) != up_is_plus;
            if plus {
                *pos += 1;
            } else if *pos > 0 || over {
                *pos -= 1;
            }
        }
    }
    out
}

fn atc_machine() -> JobMachine {
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

/// The rack of the recorded machine (the positions of its configuration file; the first four places).
fn atc_rack() -> Rack {
    Rack {
        places: vec![
            (540.0, 20.0, 37.4),
            (530.0, 40.0, 37.4),
            (540.0, 60.0, 37.4),
            (530.0, 80.0, 37.4),
        ],
    }
}

#[test]
fn three_tools_on_the_tool_changer_match_the_recorded_job_event_by_event() {
    let tool = |n, rpm| JobTool {
        number: n,
        extra_stroke_mm: 0.1,
        feed_mm_min: 100,
        spindle_rpm: rpm,
    };
    let tools = [tool(1, 60_000), tool(2, 60_000), tool(3, 30_000)];
    let h = |tool, x: f64, y: f64| JobHole {
        tool,
        x_mm: mil(x),
        y_mm: mil(y),
    };
    // the holes in the order the original drilled them (its sorting is the program's, not the protocol's)
    let holes = [
        h(1, 500.0, 500.0),
        h(1, 3000.0, 500.0),
        h(1, 3000.0, 2500.0),
        h(2, 500.0, 2000.0),
        h(2, 1000.0, 2500.0),
        h(2, 2000.0, 1500.0),
        h(3, 1500.0, 500.0),
        h(3, 2500.0, 3000.0),
    ];
    let plan = plan_drill_job_atc(&atc_machine(), &tools, &holes, &atc_rack());
    let got = latch_events(&flatten_bytes(&plan));
    // Recorded from the original: (latch byte, X, Y, Z in steps). A0 opens the chuck, 80 closes it.
    let expected: Vec<(u8, i32, i32, i32)> = vec![
        (0xD0, 0, 0, 0),
        (0xD0, 0, 0, 0),
        // tool 1: to place 0, Z down, open, slide to place 1, close, up and to the first hole
        (0xA0, 21245, 787, 1323),
        (0x80, 20852, 1573, 1323),
        (0xD0, 500, 500, 0),
        (0xF8, 500, 500, 0),
        (0x90, 500, 500, 0),
        (0x9F, 500, 500, 0),
        (0xD2, 2998, 2498, 0),
        (0x80, 2998, 2498, 0),
        (0xD0, 2998, 2498, 0),
        // tool 2: back to place 1, open, slide to place 2, close
        (0xA0, 20852, 1573, 1323),
        (0x80, 21245, 2360, 1323),
        (0xD0, 500, 1998, 0),
        (0xF8, 500, 1998, 0),
        (0x90, 500, 1998, 0),
        (0x9F, 500, 1998, 0),
        (0xD2, 1999, 1498, 0),
        (0x80, 1999, 1498, 0),
        (0xD0, 1999, 1498, 0),
        // tool 3
        (0xA0, 21245, 2360, 1323),
        (0x80, 20852, 3146, 1323),
        (0xD0, 1499, 500, 0),
        (0xF8, 1499, 500, 0),
        (0x90, 1499, 500, 0),
        (0x90, 1499, 500, 0),
        (0xD2, 2498, 2997, 0),
        (0x80, 2498, 2997, 0),
        (0xD0, 2498, 2997, 0),
        // the last tool goes back to its place and the chuck closes on the test pin's place
        (0xA0, 20852, 3146, 1323),
        (0x80, 21245, 787, 1323),
        (0xD0, 21245, 787, 0),
        (0xC0, 0, 0, 0),
    ];
    assert_eq!(got, expected);
}

#[test]
fn the_manual_plan_never_opens_the_chuck() {
    let tools = [JobTool {
        number: 1,
        extra_stroke_mm: 0.0,
        feed_mm_min: 1000,
        spindle_rpm: 60_000,
    }];
    let holes = [JobHole {
        tool: 1,
        x_mm: 10.0,
        y_mm: 10.0,
    }];
    let bytes = flatten_bytes(&plan_drill_job(&machine(), &tools, &holes));
    assert!(!bytes.iter().any(|&b| (0xA0..=0xBF).contains(&b)));
}

#[test]
fn a_table_that_is_not_flat_changes_the_depth_of_each_hole_and_nothing_else() {
    use eh3d_drillpro_protocol::job::plan_drill_job_levelled;
    let tools = [JobTool {
        number: 1,
        extra_stroke_mm: 0.0,
        feed_mm_min: 1000,
        spindle_rpm: 60_000,
    }];
    let hole = |x: f64| JobHole {
        tool: 1,
        x_mm: x,
        y_mm: 10.0,
    };
    let holes = [hole(10.0), hole(60.0), hole(110.0)];
    let down = |lifts: &[f64]| {
        let plan = plan_drill_job_levelled(&machine(), &tools, &holes, lifts, None);
        flatten_bytes(&plan)
            .iter()
            .filter(|&&b| b == eh3d_drillpro_protocol::bits::Z_POS)
            .count()
    };
    let flat = down(&[]);
    // no lift at all, or a flat table, is the same job
    assert_eq!(down(&[0.0, 0.0, 0.0]), flat);
    assert_eq!(flat, down(&[]));
    let plain = flatten_bytes(&plan_drill_job(&machine(), &tools, &holes));
    assert_eq!(
        plain,
        flatten_bytes(&plan_drill_job_levelled(
            &machine(),
            &tools,
            &holes,
            &[],
            None
        ))
    );
    // where the table is 0.3 mm higher the drill goes less deep, where it is lower it goes deeper
    let g = machine().geometry;
    let steps = g.z_steps(0.3) as usize;
    // the depth is the same into the board: a hole on a higher table is shallower by what the table is higher, and the next one makes it up again
    assert_eq!(down(&[0.0, 0.3, 0.0]), flat);
    assert_eq!(down(&[0.3, 0.3, 0.3]) + steps, flat);
    assert_eq!(down(&[-0.3, -0.3, -0.3]), flat + steps);
}
