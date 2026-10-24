//! Measures how long the serial driver takes to answer the calls DOSBox-X makes every emulated
//! millisecond with a real port: reading the modem lines and asking how many bytes are waiting.
//! Read-only: it sends nothing and keeps DTR low.
//!
//! ```text
//! cargo run -p eh3d-drillpro-protocol --features serial --example latency_probe -- COM7
//! ```

use std::time::{Duration, Instant};

use eh3d_drillpro_protocol::Transport;

fn stats(name: &str, mut us: Vec<f64>) {
    us.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = us.iter().sum::<f64>() / us.len() as f64;
    println!(
        "{name:<34} mean {mean:8.1} us   median {:8.1}   p99 {:8.1}   max {:8.1}",
        us[us.len() / 2],
        us[us.len() * 99 / 100],
        us[us.len() - 1]
    );
}

fn main() {
    let Some(name) = std::env::args().nth(1) else {
        println!("Usage: latency_probe PORT");
        return;
    };
    let mut port = serialport::new(&name, 38_400)
        .timeout(Duration::from_millis(1))
        .open()
        .unwrap_or_else(|e| {
            eprintln!("cannot open {name}: {e}");
            std::process::exit(1);
        });
    port.write_data_terminal_ready(false).ok();
    port.write_request_to_send(false).ok();
    let n = 2000;
    let time = |f: &mut dyn FnMut()| -> Vec<f64> {
        (0..n)
            .map(|_| {
                let t = Instant::now();
                f();
                t.elapsed().as_secs_f64() * 1e6
            })
            .collect()
    };
    stats(
        "read modem line (DCD)",
        time(&mut || {
            let _ = port.read_carrier_detect();
        }),
    );
    stats(
        "bytes_to_read (ClearCommError)",
        time(&mut || {
            let _ = port.bytes_to_read();
        }),
    );
    let mut buf = [0u8; 1];
    stats(
        "read 1 byte (1 ms timeout)",
        time(&mut || {
            let _ = port.read(&mut buf);
        }),
    );
    drop(port);
    let mut transport = eh3d_drillpro_protocol::serial::SerialTransport::open(&name).expect("open");
    transport.set_dtr(false).ok();
    stats(
        "SerialTransport::status (all 4 lines)",
        time(&mut || {
            let _ = transport.status();
        }),
    );
    println!("({n} calls each; DOSBox-X makes about 1000 such calls per emulated second)");
}
