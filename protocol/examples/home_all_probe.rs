//! Reference run of the three axes with a log of the input lines, to find out how the control unit reports "home".
//!
//! Start with every axis away from its switch. Each search goes out as ONE continuous stream (cutting it in pieces leaves the
//! line empty for a few milliseconds, and a stepper motor at speed then loses steps and stalls); the four input lines are
//! read when a stream has gone:
//! 1. Z up to its switch (`z_search_steps`, more than the travel: the control unit stops the axis there), then the second cycle at
//!    the reference speed (down 45 steps, up 54);
//! 2. X alone towards its switch, then Y alone (`--x-steps` and `--y-steps`; with `--diag` both together, diagonally);
//! 3. the inputs: RI should be up when all three axes are on their switches;
//! 4. the second cycle of X and then of Y: away 200 steps (RI should fall), back 250 (RI should rise again).
//!
//! Everything it does is written to `home_all_<time>.log` in the folder given with `--log-dir` (default `local-data/traces`): one
//! line per phase with the time, the phase, the steps sent and CTS, DSR, RI, DCD; lines that show a change of any input are
//! marked CHANGE. The drivers are switched off when it ends, also on an error.
//!
//! ```text
//! cargo run --release -p eh3d-drillpro-protocol --features serial --example home_all_probe -- COM8 --yes
//! cargo run --release -p eh3d-drillpro-protocol --features serial --example home_all_probe -- COM8 --yes --x-steps 22000 --y-steps 17000 --hz 2400
//! ```

use std::fs::{self, File};
use std::io::Write as _;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use eh3d_drillpro_protocol::motion::STEPS_PER_MM;
use eh3d_drillpro_protocol::safety::{Machine, Verified};
use eh3d_drillpro_protocol::serial::SerialTransport;
use eh3d_drillpro_protocol::{bits, plan_line, ModemStatus, Pacing, Step};

/// DCF values of the owner's machine, in steps per second: ramp start and reference speed 1000 mm/min on X and Y, and for Z
/// 1000 and 5000 mm/min with the Z correction 0.8988.
const XY_REF_HZ: f64 = 656.0;
const Z_REF_HZ: f64 = 590.0;

fn arg<T: std::str::FromStr>(args: &[String], name: &str) -> Option<T> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
}

/// `count` steps of one byte, accelerating from `start` to `hz` over `ramp` steps and slowing down the same way.
fn ramped(byte: u8, count: u32, start: f64, hz: f64, ramp: u32) -> Vec<Step> {
    (0..count)
        .map(|i| {
            let k = i.min(count - 1 - i).min(ramp) as f64 / ramp.max(1) as f64;
            Step {
                byte,
                delay_us: 1e6 / (start + (hz - start) * k),
            }
        })
        .collect()
}

/// One leg of the calibration jump: smooth at both ends, up to `peak` steps per second.
fn leg(byte: u8, count: u32, peak: f64, start_fraction: f64, parts: u32) -> Vec<Step> {
    ramped(
        byte,
        count,
        XY_REF_HZ * start_fraction,
        peak,
        (count / parts.max(1)).max(1),
    )
}

fn constant(byte: u8, count: u32, hz: f64) -> Vec<Step> {
    (0..count)
        .map(|_| Step {
            byte,
            delay_us: 1e6 / hz,
        })
        .collect()
}

struct Log {
    file: File,
    start: Instant,
    last: Option<ModemStatus>,
}

impl Log {
    fn line(&mut self, text: &str) {
        let t = self.start.elapsed().as_secs_f64();
        let _ = writeln!(self.file, "{t:9.3}  {text}");
        let _ = self.file.flush();
        println!("{t:9.3}  {text}");
    }

    fn lines(&mut self, phase: &str, steps: u32, s: ModemStatus) {
        let changed = self.last != Some(s);
        self.last = Some(s);
        let mark = if changed { "CHANGE" } else { "      " };
        self.line(&format!(
            "{mark} {phase:<12} steps {steps:6}  CTS={} DSR={} RI={} DCD={}",
            s.cts as u8, s.dsr as u8, s.ri as u8, s.dcd as u8
        ));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(port) = args
        .iter()
        .find(|a| a.starts_with("COM") || a.starts_with("/dev"))
    else {
        println!("Usage: home_all_probe PORT --yes [--x-steps N] [--y-steps N] [--hz H] [--y-hz H] [--z-hz H] [--ramp STEPS] [--jump-hz H] [--jump-start F] [--jump-parts N] [--config FILE] [--name LABEL] [--only x|y [--away]] [--z-only] [--no-drivers] [--filler dir|zero] [--z-pacing bytes|stream] [--diag] [--log-dir DIR]");
        return;
    };
    if !args.iter().any(|a| a == "--yes") {
        println!(
            "This moves all three axes towards their reference switches on {port}. Add --yes."
        );
        return;
    }
    // --config FILE takes the tuning from a machine file (the one the apps read); the flags below still override each value.
    let tuning = arg::<String>(&args, "--config")
        .map(|p| eh3d_drillpro_protocol::connection::load(Some(std::path::Path::new(&p))))
        .transpose()
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        })
        .map_or_else(eh3d_drillpro_protocol::tuning::Tuning::default, |c| {
            c.tuning
        });
    let dir: PathBuf = arg::<String>(&args, "--log-dir")
        .map_or_else(|| PathBuf::from("local-data/traces"), PathBuf::from);
    let x_steps: u32 = arg(&args, "--x-steps").unwrap_or(22_000).clamp(500, 40_000);
    let y_steps: u32 = arg(&args, "--y-steps").unwrap_or(17_000).clamp(500, 40_000);
    let hz: f64 = arg::<f64>(&args, "--hz")
        .unwrap_or(tuning.x_search_hz)
        .clamp(600.0, 2900.0);
    let y_hz: f64 = arg::<f64>(&args, "--y-hz")
        .unwrap_or(tuning.y_search_hz)
        .clamp(600.0, 2900.0);
    let z_hz: f64 = arg::<f64>(&args, "--z-hz")
        .unwrap_or(tuning.z_search_hz)
        .clamp(400.0, 2900.0);
    let diag = args.iter().any(|a| a == "--diag");
    let jump_hz: f64 = arg::<f64>(&args, "--jump-hz")
        .unwrap_or(tuning.jump_peak_hz)
        .clamp(700.0, 2900.0);
    let jump_start: f64 = arg::<f64>(&args, "--jump-start")
        .unwrap_or(tuning.jump_start_fraction)
        .clamp(0.2, 1.0);
    let jump_parts: u32 = arg(&args, "--jump-parts")
        .unwrap_or(tuning.jump_ramp_parts)
        .clamp(1, 10);
    let ramp: u32 = arg(&args, "--ramp")
        .unwrap_or(tuning.search_ramp_steps)
        .clamp(50, 3000);
    let no_drivers = args.iter().any(|a| a == "--no-drivers");
    let keep_dir =
        arg::<String>(&args, "--filler").map_or(tuning.stream_keep_direction, |f| f != "zero");
    let z_bytes =
        arg::<String>(&args, "--z-pacing").map_or(tuning.z_pacing_bytes, |f| f != "stream");
    fs::create_dir_all(&dir).expect("log folder");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    // --name LABEL puts a label in the file name (home_all_LABEL_<time>.log) to tell several tests apart.
    let label: String = arg::<String>(&args, "--name")
        .map(|n| {
            let clean: String = n
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            format!("{clean}_")
        })
        .unwrap_or_default();
    let path = dir.join(format!("home_all_{label}{stamp}.log"));
    let mut log = Log {
        file: File::create(&path).expect("log file"),
        start: Instant::now(),
        last: None,
    };
    log.line(&format!(
        "log {}  x {x_steps} y {y_steps} steps at {hz} Hz, z at {z_hz} Hz, filler {}{}",
        path.display(),
        if keep_dir {
            "keeps the direction"
        } else {
            "zero"
        },
        if diag { " (diagonal)" } else { "" }
    ));

    let transport = SerialTransport::open(port).unwrap_or_else(|e| {
        eprintln!("cannot open {port}: {e}");
        std::process::exit(1);
    });
    let mut m = Machine::new(transport);
    m.set_tuning(eh3d_drillpro_protocol::tuning::Tuning {
        z_pacing_bytes: z_bytes,
        ..tuning
    });
    m.set_pacing(if keep_dir {
        Pacing::StreamKeepDir
    } else {
        Pacing::Stream
    });
    m.connect().expect("handshake");
    m.arm(Verified::all()).expect("arm");
    let s = m.inputs().expect("inputs");
    log.lines("start", 0, s);

    let on = m.latches().drivers_on();
    let result = (|| -> Result<(), String> {
        macro_rules! phase {
            ($name:expr, $steps:expr, $plan:expr) => {{
                m.run(&$plan).map_err(|e| e.to_string())?;
                let s = m.inputs().map_err(|e| e.to_string())?;
                log.lines($name, $steps, s);
                s
            }};
        }
        // A long search is skipped when the home line is already up (all three axes on their switches, as when parked): the
        // original goes straight to the calibration jump then. Not with --z-only, which is for testing Z.
        let z_only = args.iter().any(|a| a == "--z-only");
        macro_rules! seek {
            ($name:expr, $steps:expr, $plan:expr, $watch:expr) => {{
                let now = m.inputs().map_err(|e| e.to_string())?;
                if now.ri && !z_only {
                    log.line(&format!("{} skipped: already on all three switches", $name));
                } else {
                    // the search goes out as one stream and stops being sent when RI rises (see Machine::run_until_home)
                    let began = log.start.elapsed().as_secs_f64();
                    let watch: bool = $watch;
                    let rose = if watch {
                        m.run_until_home(&$plan).map_err(|e| e.to_string())?
                    } else {
                        m.run(&$plan).map_err(|e| e.to_string())?;
                        false
                    };
                    let s = m.inputs().map_err(|e| e.to_string())?;
                    log.lines($name, $steps, s);
                    log.line(&format!(
                        "{} {} after {:.2} s",
                        $name,
                        if rose {
                            "ended early: RI rose"
                        } else if watch {
                            "sent in full: RI did not rise"
                        } else {
                            "sent in full (not watched)"
                        },
                        log.start.elapsed().as_secs_f64() - began
                    ));
                }
            }};
        }
        if no_drivers {
            log.line("drivers-on byte NOT sent (as the original's own Z test at start)");
        } else {
            m.apply(on).map_err(|e| e.to_string())?;
        }
        let s = m.inputs().map_err(|e| e.to_string())?;
        log.lines("drivers on", 0, s);

        // --only x|y: just that axis, in the same stream, with the speed and ramp of the search; with --away it moves away from its
        // switch (position it about 100 mm from home first: nothing can be reached), otherwise towards it. Then it stops.
        if let Some(axis) = arg::<String>(&args, "--only") {
            let away = args.iter().any(|a| a == "--away");
            let (byte, steps, rate, name) = match (axis.as_str(), away) {
                ("x", false) => (bits::X_NEG, x_steps, hz, "x only"),
                ("x", true) => (bits::X_POS, x_steps, hz, "x only, away"),
                ("y", false) => (bits::Y_NEG, y_steps, y_hz, "y only"),
                ("y", true) => (bits::Y_POS, y_steps, y_hz, "y only, away"),
                _ => return Err("--only takes x or y".into()),
            };
            log.line(&format!(
                "{name}: {steps} steps up to {rate} Hz, ramp {ramp} steps"
            ));
            phase!(name, steps, ramped(byte, steps, XY_REF_HZ, rate, ramp));
            return Ok(());
        }

        // 1. Z up to its switch, then the second cycle.
        seek!(
            "z search",
            tuning.z_search_steps,
            ramped(
                bits::Z_NEG,
                tuning.z_search_steps,
                450.0_f64.min(z_hz),
                z_hz,
                200,
            ),
            tuning.home_watch_seek
        );
        phase!("z away", 45, constant(bits::Z_POS, 45, Z_REF_HZ));
        phase!("z back", 54, constant(bits::Z_NEG, 54, Z_REF_HZ));

        if z_only {
            return Ok(());
        }

        // The trick: when the machine is parked (RI up: all three axes are on their switches) X and Y first move away from them, smoothly,
        // and RI must drop; the search back is then a short, known distance and both arrivals are seen live on the home line.
        let mut known: Option<(u32, u32)> = None;
        if !diag && !z_only && tuning.unstick_mm > 0.0 {
            let now = m.inputs().map_err(|e| e.to_string())?;
            if now.ri || tuning.unstick_always {
                let ux = (tuning.unstick_mm * STEPS_PER_MM * 0.9992).round() as u32;
                let uy = (tuning.unstick_mm * STEPS_PER_MM * 0.9998).round() as u32;
                log.line(&format!(
                    "unstick: X and Y move {} mm away from their switches (RI was {})",
                    tuning.unstick_mm, now.ri as u8
                ));
                phase!(
                    "x unstick",
                    ux,
                    leg(bits::X_POS, ux, jump_hz, jump_start, jump_parts)
                );
                phase!(
                    "y unstick",
                    uy,
                    leg(bits::Y_POS, uy, jump_hz, jump_start, jump_parts)
                );
                let after = m.inputs().map_err(|e| e.to_string())?;
                if now.ri && !after.ri {
                    let margin = tuning.search_hint_margin_steps;
                    known = Some((ux + margin, uy + margin));
                    log.line(&format!(
                        "RI dropped: the way back is known, {} steps for X and {} for Y",
                        ux + margin,
                        uy + margin
                    ));
                } else {
                    log.line("RI did not drop, or the positions were unknown: full searches");
                }
            }
        }
        let x_eff = known.map_or(x_steps, |k| k.0.min(x_steps));
        let y_eff = known.map_or(y_steps, |k| k.1.min(y_steps));

        // 2. X and Y.
        if diag {
            let n = x_steps.max(y_steps);
            phase!(
                "xy search",
                n,
                plan_line(-(n as i32), -(n as i32), XY_REF_HZ, hz, 400)
            );
        } else {
            seek!(
                "x search",
                x_eff,
                ramped(bits::X_NEG, x_eff, XY_REF_HZ, hz, ramp),
                tuning.home_watch_seek
            );
            seek!(
                "y search",
                y_eff,
                ramped(bits::Y_NEG, y_eff, XY_REF_HZ, y_hz, ramp),
                true
            );
            // The distance was wrong (an axis was further than the position said): repeat with the whole travel.
            if known.is_some() && !m.inputs().map_err(|e| e.to_string())?.ri {
                log.line("the short searches did not reach RI: repeating them in full");
                seek!(
                    "x search (full)",
                    x_steps,
                    ramped(bits::X_NEG, x_steps, XY_REF_HZ, hz, ramp),
                    tuning.home_watch_seek
                );
                seek!(
                    "y search (full)",
                    y_steps,
                    ramped(bits::Y_NEG, y_steps, XY_REF_HZ, y_hz, ramp),
                    true
                );
            }
        }
        // 3. All three should be on their switches now.
        let s = m.inputs().map_err(|e| e.to_string())?;
        log.lines("all home?", 0, s);
        log.line(if s.ri {
            "RI is up: all three axes are on their switches"
        } else {
            "RI is NOT up after the searches"
        });

        // 4. The second cycle, axis by axis.
        phase!(
            "x away",
            200,
            leg(bits::X_POS, 200, jump_hz, jump_start, jump_parts)
        );
        phase!(
            "x back",
            250,
            leg(bits::X_NEG, 250, jump_hz, jump_start, jump_parts)
        );
        phase!(
            "y away",
            200,
            leg(bits::Y_POS, 200, jump_hz, jump_start, jump_parts)
        );
        let s = phase!(
            "y back",
            250,
            leg(bits::Y_NEG, 250, jump_hz, jump_start, jump_parts)
        );
        log.line(if s.ri {
            "second cycle done: RI is up"
        } else {
            "second cycle done: RI is NOT up"
        });
        Ok(())
    })();

    match &result {
        Ok(()) => log.line("RESULT ok"),
        Err(e) => log.line(&format!("RESULT error: {e}")),
    }
    let off = m.latches().drivers_off();
    let _ = m.apply(off);
    if let Ok(s) = m.inputs() {
        log.lines("drivers off", 0, s);
    }
    println!("log written to {}", path.display());
}
