//! Finds out how the limit switches show up on the input lines. With the drivers on it sends single
//! step bytes, one at a time, in pairs that cancel out (X-, X+, Y-, Y+, Z up, Z down), reads the four
//! input lines after each byte and prints every change together with the byte that was just sent.
//! Hold ONE limit switch pressed by hand while it runs, then repeat for the next switch.
//!
//! Net motion is zero and each excursion is a single step (0.025 mm). The drivers are switched off
//! when it ends, also on error.
//!
//! ```text
//! cargo run --release -p eh3d-drillpro-protocol --features serial --example limit_probe -- COM8 --yes --seconds 30
//! ```

use std::time::{Duration, Instant};

use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::{bits, connect, ModemStatus, Transport};

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
        .find(|a| a.starts_with("COM") || a.starts_with("/dev"))
    else {
        println!("Usage: limit_probe PORT --yes [--seconds N] [--dtr-high] [--no-steps]");
        return;
    };
    if !args.iter().any(|a| a == "--yes") {
        println!("This energises the motors and sends single steps on {port}. Add --yes.");
        return;
    }
    let seconds: u64 = args
        .iter()
        .position(|a| a == "--seconds")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let mut t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    t.set_dtr(false).expect("DTR low");
    println!("handshake: {:?}", connect(&mut t));
    t.write_byte(0xD0).expect("write D0");
    t.sleep_us(10_000);
    let dtr_high = args.iter().any(|a| a == "--dtr-high");
    let no_steps = args.iter().any(|a| a == "--no-steps");
    if dtr_high {
        t.set_dtr(true).expect("DTR high");
        t.sleep_us(50_000);
    }
    println!(
        "drivers ON, DTR {}; hold ONE limit switch pressed now ({seconds} s)",
        if dtr_high { "HIGH" } else { "low" }
    );

    let guard = DriversOff(&mut t);
    let sequence = [
        ("X-", bits::X_NEG),
        ("X+", bits::X_POS),
        ("Y-", bits::Y_NEG),
        ("Y+", bits::Y_POS),
        ("Zup", bits::Z_NEG),
        ("Zdn", bits::Z_POS),
    ];
    let start = Instant::now();
    let mut last = guard.0.status().expect("status");
    println!("{:8.3}  start        {}", 0.0, show(last));
    let mut seen_ri = [0u32; 6];
    let mut rounds = 0u32;
    if no_steps {
        // Listen only: no step bytes at all.
        while start.elapsed() < Duration::from_secs(seconds) {
            let rx = guard.0.read_available();
            if !rx.is_empty() {
                println!(
                    "{:8.3}  RX bytes     {:02x?}",
                    start.elapsed().as_secs_f64(),
                    rx
                );
            }
            let now = guard.0.status().expect("status");
            if now != last {
                println!(
                    "{:8.3}  change       {}",
                    start.elapsed().as_secs_f64(),
                    show(now)
                );
                last = now;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        return;
    }
    while start.elapsed() < Duration::from_secs(seconds) {
        for (i, (name, byte)) in sequence.iter().enumerate() {
            guard.0.write_byte(*byte).expect("write");
            guard.0.sleep_us(3_000);
            let now = guard.0.status().expect("status");
            if now.ri {
                seen_ri[i] += 1;
            }
            if now != last {
                println!(
                    "{:8.3}  after {name:<4}  {}",
                    start.elapsed().as_secs_f64(),
                    show(now)
                );
                last = now;
            }
            guard.0.sleep_us(30_000);
        }
        rounds += 1;
    }
    println!("rounds: {rounds}; times RI was 1 after each byte:");
    for (i, (name, _)) in sequence.iter().enumerate() {
        println!("  {name:<4} {}", seen_ri[i]);
    }
}
