//! Checks whether the input lines can be read WHILE a long write is going out on the same port (the USB adapter and its driver
//! decide). It sends only 0x00 bytes (no axis bits: nothing moves) and reads RI from a second handle in another thread.
//!
//! If the readings keep coming during the write, a move can be sent in one continuous stream and cut the moment the home line rises,
//! like the original does on a real serial port. If they only come after the write, they cannot.
//!
//! ```text
//! cargo run --release -p eh3d-drillpro-protocol --features serial --example concurrency_probe -- COM8 --yes
//! ```

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use eh3d_drillpro_protocol::connect;
use eh3d_drillpro_protocol::serial::SerialTransport;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(port) = args
        .iter()
        .find(|a| a.starts_with("COM") || a.starts_with("/dev"))
    else {
        println!("Usage: concurrency_probe PORT --yes");
        return;
    };
    if !args.iter().any(|a| a == "--yes") {
        println!("This sends 3000 bytes of 0x00 to {port}. Add --yes.");
        return;
    }
    let mut t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    println!("handshake: {:?}", connect(&mut t));
    let mut watcher = t.rx_reader().expect("second handle");
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    let start = Instant::now();
    let handle = std::thread::spawn(move || {
        let mut readings: Vec<f64> = Vec::new();
        while !flag.load(Ordering::Relaxed) {
            let _ = watcher.read_ring_indicator();
            readings.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        readings
    });
    // 3000 bytes at 38400 baud is about 0.78 s on the line.
    let data = vec![0x00u8; 3000];
    let t0 = start.elapsed().as_secs_f64() * 1000.0;
    t.write_all_bytes(&data).expect("write");
    let t1 = start.elapsed().as_secs_f64() * 1000.0;
    done.store(true, Ordering::Relaxed);
    let readings = handle.join().unwrap_or_default();
    let during: Vec<f64> = readings
        .iter()
        .copied()
        .filter(|&r| r >= t0 && r <= t1)
        .collect();
    let mut gaps: Vec<f64> = during.windows(2).map(|w| w[1] - w[0]).collect();
    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("write took {:.0} ms (the line needs about 780 ms)", t1 - t0);
    println!(
        "readings of RI during the write: {} (max gap {:.1} ms, median {:.1} ms)",
        during.len(),
        gaps.last().copied().unwrap_or(0.0),
        gaps.get(gaps.len() / 2).copied().unwrap_or(0.0)
    );
    println!(
        "{}",
        if during.len() > 20 {
            "YES: the lines can be read while a write is going out."
        } else {
            "NO: the readings wait for the write to end."
        }
    );
    let _ = std::io::stdout().flush();
}
