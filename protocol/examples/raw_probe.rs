//! Sends ONE byte value a number of times, one byte per step with the PC clock making the pauses, to find out what the bits of
//! a movement byte do on the real machine. The drivers are on while it runs and off when it ends.
//!
//! ```text
//! cargo run --release -p eh3d-drillpro-protocol --features serial --example raw_probe -- COM8 --yes --byte 0x30 --count 100 --hz 300
//! ```
//!
//! Keep it short (at most 400 steps, which is a few millimetres) and keep a hand near the machine's power switch. The byte must
//! be a movement byte (bit 7 clear); latch writes (0x80 and above) are refused here.

use eh3d_drillpro_protocol::safety::{Machine, Verified};
use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::{Pacing, Step};

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn parse_byte(s: &str) -> Option<u8> {
    match s.strip_prefix("0x") {
        Some(h) => u8::from_str_radix(h, 16).ok(),
        None => s.parse().ok(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(port) = args
        .iter()
        .find(|a| a.starts_with("COM") || a.starts_with("/dev"))
    else {
        println!(
            "Usage: raw_probe PORT --yes --byte 0xNN [--pattern 0xNN,0xNN] [--count N] [--hz H]"
        );
        return;
    };
    // --pattern 0x30,0x20 sends the bytes in turn (every one at the same rate); --byte sends one value.
    let pattern: Vec<u8> = match arg(&args, "--pattern") {
        Some(p) => p.split(',').filter_map(|b| parse_byte(b.trim())).collect(),
        None => arg(&args, "--byte")
            .and_then(|b| parse_byte(&b))
            .into_iter()
            .collect(),
    };
    if pattern.is_empty() {
        println!("--byte 0xNN or --pattern 0xNN,0xNN is required");
        return;
    }
    if let Some(bad) = pattern.iter().find(|&&b| b & 0x80 != 0) {
        println!(
            "0x{bad:02X} is a latch write; this probe only sends movement bytes (bit 7 clear)."
        );
        return;
    }
    if !args.iter().any(|a| a == "--yes") {
        println!("This sends {pattern:02X?} to the machine on {port}. Add --yes.");
        return;
    }
    let count: usize = arg(&args, "--count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(100)
        .clamp(1, 400);
    let hz: f64 = arg(&args, "--hz")
        .and_then(|c| c.parse::<f64>().ok())
        .unwrap_or(300.0)
        .clamp(50.0, 600.0);

    let t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    let mut m = Machine::new(t);
    m.set_pacing(Pacing::Bytes);
    m.connect().expect("handshake");
    m.arm(Verified::all()).expect("arm");
    let on = m.latches().drivers_on();
    let steps: Vec<Step> = (0..count)
        .map(|i| Step {
            byte: pattern[i % pattern.len()],
            delay_us: 1e6 / hz,
        })
        .collect();
    let result = (|| {
        m.apply(on)?;
        println!("drivers ON; sending {pattern:02X?} (bytes in turn) x {count} at {hz} Hz");
        m.run(&steps)
    })();
    let off = m.latches().drivers_off();
    let _ = m.apply(off);
    println!("drivers OFF; result: {result:?}");
}
