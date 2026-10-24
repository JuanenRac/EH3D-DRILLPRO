//! Tests a way to pace the steps with the serial line itself instead of the PC clock. A USB adapter
//! delivers the bytes of each 1 ms USB frame in a burst, so single bytes written with pauses come out
//! unevenly and the motor sounds rough. Here the whole move is written in ONE buffer: one step byte
//! followed by `--every - 1` filler bytes (0x00, no axis bits), and the UART spaces them at the baud
//! rate (about 260 us per byte). The step rate is 38400 / 10 / `--every`.
//!
//! The first run must check that the filler byte really does nothing: with the default 100 steps
//! the axis must move 2.5 mm out and back, no more.
//!
//! ```text
//! cargo run --release -p eh3d-drillpro-protocol --features serial --example slot_probe -- COM8 --yes --axis y --steps 100 --every 2
//! ```

use std::time::Duration;

use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::{bits, connect, Transport};

/// Switches the drivers off when dropped, also if the program panics.
struct DriversOff<'a>(&'a mut SerialTransport);

impl Drop for DriversOff<'_> {
    fn drop(&mut self) {
        self.0.sleep_us(100_000);
        let _ = self.0.write_byte(0xC0);
        self.0.sleep_us(20_000);
        let _ = self.0.set_dtr(false);
        println!("drivers OFF (0xC0 sent)");
    }
}

fn arg<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn stream(byte: u8, steps: usize, every: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(steps * every);
    for _ in 0..steps {
        v.push(byte);
        v.extend(std::iter::repeat_n(0x00, every - 1));
    }
    v
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(port) = args
        .iter()
        .find(|a| a.starts_with("COM") || a.starts_with("/dev"))
    else {
        println!("Usage: slot_probe PORT --yes [--axis x|y] [--steps N] [--every K]");
        return;
    };
    if !args.iter().any(|a| a == "--yes") {
        println!("This moves an axis out and back on {port}. Add --yes.");
        return;
    }
    let axis: String = arg(&args, "--axis", "y".to_string());
    let (plus, minus) = match axis.as_str() {
        "x" => (bits::X_POS, bits::X_NEG),
        "y" => (bits::Y_POS, bits::Y_NEG),
        _ => {
            println!("Axis must be x or y.");
            return;
        }
    };
    let steps: usize = arg::<usize>(&args, "--steps", 100).clamp(1, 1500);
    let every: usize = arg::<usize>(&args, "--every", 2).clamp(1, 12);
    let rate = 3840.0 / every as f64;

    let mut t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    t.set_dtr(false).expect("DTR low");
    println!("handshake: {:?}", connect(&mut t));
    t.write_byte(0xD0).expect("write D0");
    t.sleep_us(10_000);
    let guard = DriversOff(&mut t);
    println!("drivers ON; {axis} {steps} steps out and back at {rate:.0} steps/s ({every} bytes per step)");
    for (name, byte) in [("out", plus), ("back", minus)] {
        let buf = stream(byte, steps, every);
        let seconds = buf.len() as f64 * 10.0 / 38_400.0;
        guard.0.write_all_bytes(&buf).expect("write");
        println!("{name}: {} bytes, {seconds:.2} s", buf.len());
        std::thread::sleep(Duration::from_secs_f64(seconds + 0.3));
    }
}
