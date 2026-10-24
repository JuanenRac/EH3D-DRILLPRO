//! Homing of the X or Y axis alone, the way the original does it on the real machine: the control unit
//! stops the axis at its limit switch and reports nothing, so the search is sent in full, then the
//! axis backs off and approaches again (the second approach is longer than the back-off, so it ends
//! on the switch).
//!
//! `--search` must be at least the distance from the carriage to the switch, but keep it modest
//! for a first try (default 1000 steps, about 25 mm; at most 30000). The drivers are switched off
//! when it ends.
//!
//! ```text
//! cargo run --release -p eh3d-drillpro-protocol --features serial --example home_y_probe -- COM8 --yes --search 1000
//! ```

use eh3d_drillpro_protocol::safety::{Machine, Verified};
use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::{plan_line, Step};

fn arg<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Start speed of the ramp: 1000 mm/min of the configuration, in steps per second.
const START_HZ: f64 = 656.0;

/// `count` steps of one axis (negative = towards the switch) with a ramp from the start speed of the configuration.
fn axis_move(x_axis: bool, count: i32, hz: f64) -> Vec<Step> {
    let (dx, dy) = if x_axis { (count, 0) } else { (0, count) };
    plan_line(dx, dy, START_HZ.min(hz), hz, 150)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(port) = args
        .iter()
        .find(|a| a.starts_with("COM") || a.starts_with("/dev"))
    else {
        println!("Usage: home_y_probe PORT --yes [--axis x|y] [--search N] [--hz H]");
        return;
    };
    if !args.iter().any(|a| a == "--yes") {
        println!("This moves the Y axis towards its limit switch on {port}. Add --yes.");
        return;
    }
    let axis: String = arg(&args, "--axis", "y".to_string());
    let x_axis = match axis.as_str() {
        "x" => true,
        "y" => false,
        _ => {
            println!("Axis must be x or y.");
            return;
        }
    };
    let search: u32 = arg::<u32>(&args, "--search", 1000).clamp(1, 30_000);
    let hz: f64 = arg::<f64>(&args, "--hz", 2400.0).clamp(50.0, 2700.0);
    let (backoff, reapproach) = (200u32, 250u32);

    let t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    let mut m = Machine::new(t);
    m.connect().expect("handshake");
    m.arm(Verified::all()).expect("arm");
    let on = m.latches().drivers_on();
    let result = (|| {
        m.apply(on)?;
        println!("drivers ON; searching the {axis} switch ({search} steps)");
        m.run_paced(&axis_move(x_axis, -(search as i32), hz))?;
        println!("backing off {backoff} steps");
        m.run_paced(&axis_move(x_axis, backoff as i32, hz))?;
        println!("approaching again {reapproach} steps");
        m.run_paced(&axis_move(x_axis, -(reapproach as i32), hz))
    })();
    let off = m.latches().drivers_off();
    let _ = m.apply(off);
    println!("drivers OFF; result: {result:?}");
}
