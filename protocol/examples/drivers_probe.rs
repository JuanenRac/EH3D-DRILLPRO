//! Switches the stepper drivers on (latch byte 0xD0) and listens to the machine inputs while they are
//! on, then switches them off (0xC0). It sends NO step bytes, so nothing moves, but the motors are
//! energised and hold their position while the probe runs.
//!
//! Sequence (as the original does at the start of a job): handshake (DTR low, DTR high, 0x00, 0x10,
//! DTR low), then 0xD0. Press the limit switches by hand while it listens.
//!
//! ```text
//! cargo run -p eh3d-drillpro-protocol --features serial --example drivers_probe -- COM7 --yes --seconds 120
//! ```

use std::time::{Duration, Instant};

use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::{connect, ModemStatus, Transport};

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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(port) = args
        .iter()
        .find(|a| !a.starts_with("--") && a.starts_with("COM") || a.starts_with("/dev"))
    else {
        println!("Usage: drivers_probe PORT --yes [--seconds N]");
        return;
    };
    if !args.iter().any(|a| a == "--yes") {
        println!("This energises the motors (0xD0) on {port}. Add --yes to continue.");
        return;
    }
    let seconds: u64 = args
        .iter()
        .position(|a| a == "--seconds")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let mut t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    t.set_dtr(false).expect("DTR low");
    println!("handshake: {:?}", connect(&mut t));
    println!("after handshake: {}", show(t.status().expect("status")));

    t.write_byte(0xD0).expect("write D0");
    t.sleep_us(10_000);
    println!("drivers ON (0xD0 sent); listening for {seconds} s, press the limit switches now");

    let guard = DriversOff(&mut t);
    let start = Instant::now();
    let mut last = guard.0.status().expect("status");
    println!("{:9.3}  start  {}", 0.0, show(last));
    while start.elapsed() < Duration::from_secs(seconds) {
        let now = guard.0.status().expect("status");
        if now != last {
            println!(
                "{:9.3}  change {}",
                start.elapsed().as_secs_f64(),
                show(now)
            );
            last = now;
        }
        std::thread::sleep(Duration::from_micros(500));
    }
}
