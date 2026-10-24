//! First motion test with the Rust protocol library on the real machine: moves ONE axis a short
//! distance out and back, slowly, with the drivers on only for that time.
//!
//! Default: Y axis, 100 steps (about 2.5 mm) out and 100 back, 400 steps/s. X and Z need
//! `--axis x` / `--axis z`; Z is the dangerous one (tool and board), so start with Y.
//! Keep a hand on the machine's power switch. It needs `--yes` because it moves the machine.
//!
//! ```text
//! cargo run -p eh3d-drillpro-protocol --features serial --example jog_probe -- COM8 --yes
//! cargo run -p eh3d-drillpro-protocol --features serial --example jog_probe -- COM8 --yes --axis x --steps 200 --hz 600
//! ```

use eh3d_drillpro_protocol::motion::STEPS_PER_MM;
use eh3d_drillpro_protocol::safety::{Machine, Verified};
use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::{bits, plan_line, Step};

fn arg<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(port) = args
        .iter()
        .find(|a| a.starts_with("COM") || a.starts_with("/dev"))
    else {
        println!(
            "Usage: jog_probe PORT --yes [--axis x|y|z] [--steps N] [--hz H | --feed MM_MIN [--start-feed MM_MIN] [--corr C]]  (ports: {:?})",
            SerialTransport::available()
        );
        return;
    };
    let axis: String = arg(&args, "--axis", "y".to_string());
    let steps: i32 = arg::<i32>(&args, "--steps", 100).clamp(1, 400);
    // Speeds can be given as in the original configuration (mm/min) or directly as step rates.
    let corr: f64 = arg(&args, "--corr", 1.0);
    let to_hz = |mm_min: f64| mm_min / 60.0 * STEPS_PER_MM * corr;
    let hz: f64 = match args.iter().position(|a| a == "--feed") {
        Some(_) => to_hz(arg(&args, "--feed", 0.0)),
        None => arg::<f64>(&args, "--hz", 400.0),
    }
    .clamp(50.0, 3000.0);
    let start_hz: f64 = match args.iter().position(|a| a == "--start-feed") {
        Some(_) => to_hz(arg(&args, "--start-feed", 0.0)),
        None => hz / 4.0,
    }
    .clamp(50.0, hz);
    if !args.iter().any(|a| a == "--yes") {
        println!(
            "This moves axis {axis} by {steps} steps and back on {port}. Add --yes to continue."
        );
        return;
    }
    let z_axis = axis == "z";
    let (dx, dy) = match axis.as_str() {
        "x" => (steps, 0),
        "y" => (0, steps),
        "z" => (0, 0),
        _ => {
            println!("Axis must be x, y or z.");
            return;
        }
    };
    // Z: at most 200 steps (about 5 mm) at 800 steps/s or less, constant rate, down first and then
    // back up. Take the bit out of the spindle and keep the spindle off.
    let zsteps = steps.min(200) as usize;
    let zdelay = 1e6 / hz.min(800.0);
    let z_down: Vec<Step> = (0..zsteps)
        .map(|_| Step {
            byte: bits::Z_POS,
            delay_us: zdelay,
        })
        .collect();
    let z_up: Vec<Step> = (0..zsteps)
        .map(|_| Step {
            byte: bits::Z_NEG,
            delay_us: zdelay,
        })
        .collect();
    let t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    let mut m = Machine::new(t);
    m.connect().expect("handshake");
    println!("handshake OK; inputs: {:?}", m.read_inputs());
    // The operator takes responsibility for the unverified items by passing --yes.
    m.arm(Verified::all()).expect("arm");

    // The Z head rests at the bottom when the drivers are off (the weight of the spindle), so the
    // test goes up first (`--first down` reverses it) and comes back to where it started.
    let up_first = !args.windows(2).any(|w| w[0] == "--first" && w[1] == "down");
    let (z_go, z_back) = if up_first {
        (z_up, z_down)
    } else {
        (z_down, z_up)
    };
    let go: Vec<Step> = if z_axis {
        z_go
    } else {
        plan_line(dx, dy, start_hz, hz, (steps as u32 / 2).max(1))
    };
    let back: Vec<Step> = if z_axis {
        z_back
    } else {
        plan_line(-dx, -dy, start_hz, hz, (steps as u32 / 2).max(1))
    };
    let drivers_on = m.latches().drivers_on();
    let result = (|| {
        m.apply(drivers_on)?;
        println!("drivers ON; moving {axis} +{steps}");
        m.run(&go)?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        println!("moving {axis} -{steps}");
        m.run(&back)
    })();
    // Always switch the drivers off, even after an error.
    let off = m.latches().drivers_off();
    let _ = m.apply(off);
    println!(
        "drivers OFF; result: {result:?}; inputs: {:?}",
        m.read_inputs()
    );
}
