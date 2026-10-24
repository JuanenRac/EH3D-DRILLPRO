//! Listens to everything the machine sends back (RX) while an axis moves into its limit switch,
//! and prints each byte that arrives with its time. TX is known: this program sends the steps.
//!
//! The axis moves `--steps` steps towards its switch (negative direction) in one paced stream and a
//! second thread reads the port. The control unit stops the axis at the switch; if it also sends
//! something back, it shows up here.
//!
//! ```text
//! cargo run --release -p eh3d-drillpro-protocol --features serial --example rx_sniff -- COM8 --yes --axis y --steps 2500
//! ```

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::{bits, connect, paced_stream, plan_line, Transport};

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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(port) = args
        .iter()
        .find(|a| a.starts_with("COM") || a.starts_with("/dev"))
    else {
        println!("Usage: rx_sniff PORT --yes [--axis x|y] [--steps N] [--hz H]");
        return;
    };
    if !args.iter().any(|a| a == "--yes") {
        println!("This moves an axis towards its limit switch on {port}. Add --yes.");
        return;
    }
    let axis: String = arg(&args, "--axis", "y".to_string());
    let (dx, dy) = match axis.as_str() {
        "x" => (-1, 0),
        "y" => (0, -1),
        _ => {
            println!("Axis must be x or y.");
            return;
        }
    };
    let steps: i32 = arg::<i32>(&args, "--steps", 2500).clamp(1, 30_000);
    let hz: f64 = arg::<f64>(&args, "--hz", 1500.0).clamp(50.0, 2700.0);

    let mut t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    t.set_dtr(false).expect("DTR low");
    println!("handshake: {:?}", connect(&mut t));
    let mut reader = t.rx_reader().expect("second handle");
    let stop = Arc::new(AtomicBool::new(false));
    let start = Instant::now();
    let flag = stop.clone();
    let listener = std::thread::spawn(move || {
        let mut total = 0usize;
        let mut buf = [0u8; 64];
        while !flag.load(Ordering::Relaxed) {
            match reader.read(&mut buf) {
                Ok(n) if n > 0 => {
                    total += n;
                    println!(
                        "{:8.3}  RX {n} byte(s): {:02x?}",
                        start.elapsed().as_secs_f64(),
                        &buf[..n]
                    );
                }
                _ => std::thread::sleep(Duration::from_millis(2)),
            }
        }
        total
    });

    t.write_byte(0xD0).expect("write D0");
    t.sleep_us(10_000);
    {
        let guard = DriversOff(&mut t);
        let plan = plan_line(dx * steps, dy * steps, 656.0_f64.min(hz), hz, 150);
        let stream = paced_stream(&plan);
        println!(
            "drivers ON; {axis} {steps} steps towards the switch, {} bytes",
            stream.len()
        );
        for chunk in stream.chunks(1900) {
            guard.0.write_all_bytes(chunk).expect("write");
        }
        std::thread::sleep(Duration::from_secs_f64(
            stream.len() as f64 * 10.0 / 38_400.0 + 0.5,
        ));
        let _ = bits::X_NEG;
    }
    stop.store(true, Ordering::Relaxed);
    let total = listener.join().unwrap_or(0);
    println!("bytes received in total: {total}");
}
