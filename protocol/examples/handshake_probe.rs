//! Runs the connection handshake of the original program on a real serial port and reports the result.
//!
//! The handshake sends two bytes, 0x00 and 0x10 (they only write the idle latch values), after raising
//! DTR, and expects DCD to rise. It moves nothing. It does NOT arm the machine: no motion bytes are sent.
//! It needs `--yes` because it does transmit.
//!
//! ```text
//! cargo run -p eh3d-drillpro-protocol --features serial --example handshake_probe -- COM7 --yes
//! ```

use eh3d_drillpro_protocol::connect;
use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::Transport;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(port) = args.iter().find(|a| !a.starts_with("--")) else {
        println!(
            "Usage: handshake_probe PORT --yes   (ports: {:?})",
            SerialTransport::available()
        );
        return;
    };
    if !args.iter().any(|a| a == "--yes") {
        println!("This sends the bytes 0x00 and 0x10 to {port}. Add --yes to continue.");
        return;
    }
    let mut t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    t.set_dtr(false).expect("set DTR");
    println!("before: {:?}", t.status().expect("status"));
    let result = connect(&mut t);
    println!("handshake: {result:?}");
    println!("after:  {:?}", t.status().expect("status"));
}
