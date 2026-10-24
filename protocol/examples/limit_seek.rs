//! Moves ONE axis towards its limit switch in short chunks and reports where the input lines change.
//! It stops at the first change of RI / DCD, or after `--steps` steps (default 400, about 10 mm;
//! at most 1400), so a switch that is not detected cannot be driven into the end stop.
//!
//! Directions are the ones of the original homing: Z goes up (0x30), X and Y go negative. Start with Z
//! with the tool removed and the spindle off. The drivers are switched off when it ends.
//!
//! ```text
//! cargo run --release -p eh3d-drillpro-protocol --features serial --example limit_seek -- COM8 --yes --axis z --steps 400
//! ```

use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::{bits, connect, ModemStatus, Transport};

/// Switches the drivers off when dropped, also if the program panics.
struct DriversOff<'a>(&'a mut SerialTransport);

impl Drop for DriversOff<'_> {
    fn drop(&mut self) {
        self.0.sleep_us(100_000);
        let _ = self.0.write_byte(0xC0);
        self.0.sleep_us(20_000);
        println!("drivers OFF (0xC0 sent)");
    }
}

fn show(s: ModemStatus) -> String {
    format!(
        "CTS={} DSR={} RI={} DCD={}",
        s.cts as u8, s.dsr as u8, s.ri as u8, s.dcd as u8
    )
}

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
        println!("Usage: limit_seek PORT --yes [--axis x|y|z] [--steps N] [--hz H]");
        return;
    };
    if !args.iter().any(|a| a == "--yes") {
        println!("This moves an axis towards its limit switch on {port}. Add --yes.");
        return;
    }
    let axis: String = arg(&args, "--axis", "z".to_string());
    let byte = match axis.as_str() {
        "x" => bits::X_NEG,
        "y" => bits::Y_NEG,
        "z" => bits::Z_NEG,
        _ => {
            println!("Axis must be x, y or z.");
            return;
        }
    };
    let total: u32 = arg::<u32>(&args, "--steps", 400).clamp(1, 1400);
    let hz: f64 = arg::<f64>(&args, "--hz", 600.0).clamp(50.0, 800.0);
    let chunk = 10u32;

    let mut t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    t.set_dtr(false).expect("DTR low");
    println!("handshake: {:?}", connect(&mut t));
    t.write_byte(0xD0).expect("write D0");
    t.sleep_us(10_000);
    let guard = DriversOff(&mut t);
    let mut last = guard.0.status().expect("status");
    println!("drivers ON; axis {axis}, up to {total} steps at {hz} steps/s");
    println!("{:6}  start  {}", 0, show(last));

    let delay = (1e6 / hz) as u64;
    let mut sent = 0u32;
    while sent < total {
        for _ in 0..chunk.min(total - sent) {
            guard.0.write_byte(byte).expect("write");
            guard.0.sleep_us(delay);
            sent += 1;
        }
        let now = guard.0.status().expect("status");
        if now != last {
            println!("{sent:6}  change {}", show(now));
            last = now;
            if now.ri || now.dcd {
                println!("a limit/fault line went active after {sent} steps: stopping");
                break;
            }
        }
    }
    println!("steps sent: {sent}; final: {}", show(last));
}
