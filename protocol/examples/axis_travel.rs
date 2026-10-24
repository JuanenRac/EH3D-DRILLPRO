//! Travel test of the X or Y axis, starting ON its limit switch: moves `--mm` away from it and comes
//! back to it, with 30 mm of extra search at the end that the control unit swallows at the switch,
//! and then the bounce of the original (back off 200 steps, approach 250).
//! The drivers are switched off when it ends.
//!
//! ```text
//! cargo run --release -p eh3d-drillpro-protocol --features serial --example axis_travel -- COM8 --yes --axis x --mm 400
//! ```

use eh3d_drillpro_protocol::motion::STEPS_PER_MM;
use eh3d_drillpro_protocol::safety::{Machine, Verified};
use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::{plan_line, Step};

/// Start speed of the ramp: 1000 mm/min of the configuration, in steps per second.
const START_HZ: f64 = 656.0;

fn arg<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

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
        println!("Usage: axis_travel PORT --yes [--axis x|y] [--mm D] [--hz H] [--corr C]");
        return;
    };
    if !args.iter().any(|a| a == "--yes") {
        println!("This moves an axis away from its switch and back on {port}. Add --yes.");
        return;
    }
    let axis: String = arg(&args, "--axis", "x".to_string());
    let x_axis = match axis.as_str() {
        "x" => true,
        "y" => false,
        _ => {
            println!("Axis must be x or y.");
            return;
        }
    };
    let mm: f64 = arg::<f64>(&args, "--mm", 100.0).clamp(1.0, 450.0);
    let hz: f64 = arg::<f64>(&args, "--hz", 2000.0).clamp(50.0, 2700.0);
    let default_corr = if x_axis { 0.9992 } else { 0.9998 };
    let corr: f64 = arg(&args, "--corr", default_corr);
    let steps = (mm * STEPS_PER_MM * corr).round() as i32;
    let extra = (30.0 * STEPS_PER_MM * corr).round() as i32;

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
        println!("drivers ON; {axis} away from the switch: {mm} mm ({steps} steps)");
        m.run_paced(&axis_move(x_axis, steps, hz))?;
        println!("back to the switch: {} steps", steps + extra);
        m.run_paced(&axis_move(x_axis, -(steps + extra), hz))?;
        // The bounce of the original: back off a little and approach the switch again.
        println!("bounce: back off 200 steps, approach 250");
        m.run_paced(&axis_move(x_axis, 200, hz))?;
        m.run_paced(&axis_move(x_axis, -250, hz))
    })();
    let off = m.latches().drivers_off();
    let _ = m.apply(off);
    println!("drivers OFF; result: {result:?}");
}
