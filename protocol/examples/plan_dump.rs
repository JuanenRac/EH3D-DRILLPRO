//! Prints the run-length of the planned job for the three-tool test file, to compare with a recording.
use eh3d_drillpro_protocol::job::*;
use eh3d_drillpro_protocol::motion::Geometry;

fn main() {
    let m = JobMachine {
        geometry: Geometry::with_correction(0.9992, 0.9998, 0.8988),
        offset_mm: (2.5, 2.5),
        max_depth_mm: 37.30,
        base_mm: 12.0,
        board_mm: 1.0,
        distance_tool_board_mm: 16.0,
        xy_search_steps: 26613,
    };
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
    let mil = |v: f64| v * 0.0254;
    let pts = [
        (1, 500, 500),
        (1, 3000, 500),
        (1, 3000, 2500),
        (2, 1000, 2500),
        (2, 2000, 1500),
        (2, 500, 2000),
        (3, 2500, 3000),
        (3, 1500, 500),
    ];
    let holes: Vec<JobHole> = pts
        .iter()
        .map(|&(t, x, y)| JobHole {
            tool: t,
            x_mm: mil(x as f64),
            y_mm: mil(y as f64),
        })
        .collect();
    let plan = plan_drill_job(&m, &tools, &holes);
    let runs = run_lengths(&flatten_bytes(&plan));
    println!(
        "{}",
        runs.iter()
            .map(|(b, n)| format!("{b:02x}x{n}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
}
