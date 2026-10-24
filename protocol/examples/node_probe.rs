//! Connects to an EH3D-NODE-BUNGARD, logs in and shows what the node reports. Nothing moves: the machine is not
//! armed. With `--jog` it also arms and moves the Y axis 100 steps out and back through the node (about 2.5 mm each
//! way), the same test `jog_probe` does through a serial port.
//!
//! ```text
//! cargo run -p eh3d-drillpro-protocol --features node --example node_probe -- 192.168.0.50:5800 --token SECRET
//! cargo run -p eh3d-drillpro-protocol --features node --example node_probe -- 192.168.0.50:5800 --token SECRET --jog --yes
//! ```

use std::time::{Duration, Instant};

use eh3d_drillpro_protocol::motion::STEPS_PER_MM;
use eh3d_drillpro_protocol::node::NodeTransport;
use eh3d_drillpro_protocol::safety::{Machine, Verified};
use eh3d_drillpro_protocol::{plan_line, Transport};

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(addr) = args
        .iter()
        .find(|a| a.contains(':') && !a.starts_with("--"))
    else {
        println!("Usage: node_probe HOST:PORT --token TOKEN [--jog --yes]");
        return;
    };
    let token = arg(&args, "--token").unwrap_or_default();
    let mut t = NodeTransport::connect(addr.as_str(), &token).unwrap_or_else(|e| {
        eprintln!("cannot use the node at {addr}: {e}");
        std::process::exit(1);
    });
    let v = t.info.version;
    println!(
        "node {}.{}.{}, {} baud, queue of {} bytes",
        v.0, v.1, v.2, t.info.baud, t.info.queue_capacity
    );
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        let s = t.status().expect("status");
        println!(
            "{:5.2}s  CTS={} DSR={} RI={} DCD={}  queue {:?}",
            start.elapsed().as_secs_f64(),
            s.cts as u8,
            s.dsr as u8,
            s.ri as u8,
            s.dcd as u8,
            t.node_queue()
        );
        std::thread::sleep(Duration::from_millis(500));
    }
    if !args.iter().any(|a| a == "--jog") {
        return;
    }
    if !args.iter().any(|a| a == "--yes") {
        println!("--jog moves the Y axis by 100 steps and back. Add --yes to continue.");
        return;
    }
    t.arm().expect("arm the node");
    let mut m = Machine::new(t);
    m.connect().expect("handshake");
    m.arm(Verified::all()).expect("arm");
    let on = m.latches().drivers_on();
    let hz = 656.0;
    let out = plan_line(0, 100, hz / 4.0, hz, 50);
    let back = plan_line(0, -100, hz / 4.0, hz, 50);
    let result = (|| {
        m.apply(on)?;
        println!("drivers ON; Y +100 steps ({:.1} mm)", 100.0 / STEPS_PER_MM);
        m.run_paced(&out)?;
        m.run_paced(&back)
    })();
    let off = m.latches().drivers_off();
    let _ = m.apply(off);
    println!("drivers OFF; result: {result:?}");
}
