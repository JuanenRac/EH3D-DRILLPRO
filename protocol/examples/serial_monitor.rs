//! Read-only monitor of the control unit's input lines on a real serial port.
//!
//! Prints every change of CTS, DSR, RI and DCD with a timestamp. It sends NO data. DTR and RTS are
//! kept low (the original raises DTR only at its handshake); use `--dtr` to raise DTR.
//!
//! Without arguments it lists the serial ports. Typical use, with the machine connected and the
//! emergency stop within reach:
//!
//! ```text
//! cargo run -p eh3d-drillpro-protocol --features serial --example serial_monitor -- COM7 --seconds 120
//! ```
//!
//! Press each limit switch by hand and watch which line changes: the original treats RI as the
//! home / limit switch and DCD as the machine fault.

use std::time::{Duration, Instant};

use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::{ModemStatus, Transport};

fn show(s: ModemStatus) -> String {
    format!(
        "CTS={} DSR={} RI={} DCD={}",
        s.cts as u8, s.dsr as u8, s.ri as u8, s.dcd as u8
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(port) = args.iter().find(|a| {
        !a.starts_with("--")
            && args.iter().position(|x| x == *a)
                != args.iter().position(|x| x == "--seconds").map(|i| i + 1)
    }) else {
        println!("Serial ports found:");
        for p in SerialTransport::available() {
            println!("  {p}");
        }
        println!("Usage: serial_monitor PORT [--seconds N] [--dtr]");
        return;
    };
    let seconds: u64 = args
        .iter()
        .position(|a| a == "--seconds")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let raise_dtr = args.iter().any(|a| a == "--dtr");

    let mut t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    t.set_dtr(raise_dtr).expect("set DTR");
    println!(
        "Monitoring {port} for {seconds} s (DTR {}). Nothing is sent.",
        if raise_dtr { "high" } else { "low" }
    );

    let start = Instant::now();
    let mut last = t.status().expect("read status");
    println!("{:9.3}  start  {}", 0.0, show(last));
    while start.elapsed() < Duration::from_secs(seconds) {
        let now = t.status().expect("read status");
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
    println!("done");
}
