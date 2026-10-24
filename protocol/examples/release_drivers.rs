//! Switches the stepper drivers off (the latch byte 0xC0 of the original) so the axes can be moved by hand. Use it when a test was
//! cut short (Ctrl+C, or the machine's power switch) and the motors are still holding. It moves nothing.
//!
//! ```text
//! cargo run --release -p eh3d-drillpro-protocol --features serial --example release_drivers -- COM8 --yes
//! ```

use eh3d_drillpro_protocol::safety::{Machine, Verified};
use eh3d_drillpro_protocol::serial::SerialTransport;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(port) = args
        .iter()
        .find(|a| a.starts_with("COM") || a.starts_with("/dev"))
    else {
        println!("Usage: release_drivers PORT --yes");
        return;
    };
    if !args.iter().any(|a| a == "--yes") {
        println!("This switches the stepper drivers off on {port}. Add --yes.");
        return;
    }
    let t = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    let mut m = Machine::new(t);
    m.connect().expect("handshake");
    m.arm(Verified::all()).expect("arm");
    // The drivers may have been left on by an earlier run: say "on" first so the next write is the change to "off".
    let on = m.latches().drivers_on();
    let _ = m.apply(on);
    let off = m.latches().drivers_off();
    match m.apply(off) {
        Ok(()) => println!("drivers OFF: the axes are free, you can move them by hand."),
        Err(e) => println!("could not switch the drivers off: {e}"),
    }
}
