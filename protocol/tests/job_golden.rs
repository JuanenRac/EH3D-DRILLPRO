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
