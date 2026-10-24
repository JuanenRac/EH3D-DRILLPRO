//! How DRILLPRO and ROUTEPRO reach the machine, chosen in one small file the two programs share.
//!
//! ```text
//! # EH3D machine connection
//! connection = usb          # demo | rs232 | usb | node
//! rs232_port = COM1         # the computer's own serial port
//! usb_port = COM8           # a USB to RS-232 adapter
//! node_address = 192.168.0.50:5800
//! node_token = the-access-token-set-in-the-node
//! # Tuning of the reference run (all optional; see docs/machine.cfg.example for every value and its default):
//! x_search_hz = 2700
//! xy_hz_limit = 1800      # optional: the most steps per second the X and Y motors keep up with
//! z_hz_limit = 400        # optional: the same for Z (its motor carries the spindle)
//! home_signal = on          # on | off: stop the reference searches on the control unit's "all home" line (RI)
//! ```
//!
//! - `demo`: no machine; the job runs on the built-in simulator (the default when there is no file).
//! - `rs232`: the computer's own serial port, one byte at a time with the PC clock making the pauses, as the original program does.
//! - `usb`: a USB serial adapter. Its bytes come out in bursts of one USB frame, so a move is sent as one stream whose
//!   byte positions carry the time and the adapter's UART spaces them.
//! - `node`: the EH3D-NODE-BUNGARD in the machine's box, over Ethernet or Wi-Fi; the same stream, spaced by the node's UART.
//!
//! The file lives in the user's settings folder (`%APPDATA%\EH3D\machine.cfg`, or `~/.config/eh3d/machine.cfg`); a program
//! can be pointed at another with `--connection FILE`. The ports and the node are separate keys so the file can describe all
//! three and only `connection` needs changing to switch.

use std::io;
use std::path::{Path, PathBuf};

use crate::{Pacing, Simulator, Transport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kind {
    #[default]
    Demo,
    Rs232,
    Usb,
    Node,
    /// The EH3D server: the job is sent to it and it runs it on a machine through the machine's node.
    Server,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Demo => "demo",
            Kind::Rs232 => "rs232",
            Kind::Usb => "usb",
            Kind::Node => "node",
            Kind::Server => "server",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionConfig {
    pub kind: Kind,
    pub rs232_port: String,
    pub usb_port: String,
    pub node_address: String,
    pub node_token: String,
    /// The EH3D server (`connection = server`): its address, the token of this program and the id of the machine the job is for.
    pub server_address: String,
    pub server_token: String,
    pub server_machine: String,
    /// The control unit reports "all three axes home" on RI: reference searches stop on it (see `Machine::set_home_signal`).
    pub home_signal: bool,
    /// Highest step rates (per second) the real machine's X/Y and Z motors keep up with; the speeds of the job never go above them.
    /// `None` leaves the speeds of the machine's configuration file as they are.
    pub xy_hz_limit: Option<f64>,
    pub z_hz_limit: Option<f64>,
    /// The values found by trying the real machine (search speeds, ramps, the calibration jump); see [`crate::tuning`].
    pub tuning: crate::tuning::Tuning,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        let (rs232, usb) = if cfg!(windows) {
            ("COM1", "COM3")
        } else {
            ("/dev/ttyS0", "/dev/ttyUSB0")
        };
        Self {
            kind: Kind::Demo,
            rs232_port: rs232.into(),
            usb_port: usb.into(),
            node_address: String::new(),
            node_token: String::new(),
            server_address: String::new(),
            server_token: String::new(),
            server_machine: String::new(),
            home_signal: true,
            xy_hz_limit: None,
            z_hz_limit: None,
            tuning: crate::tuning::Tuning::default(),
        }
    }
}

/// Reads the file's text. Blank lines and lines starting with `#` or `;` are ignored; an unknown key or value is an
/// error, so a typo is never taken for a setting.
pub fn parse(text: &str) -> Result<ConnectionConfig, String> {
    let mut c = ConnectionConfig::default();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {}: expected key = value", n + 1));
        };
        let (key, value) = (key.trim().to_ascii_lowercase(), value.trim().to_string());
        match key.as_str() {
            "connection" => {
                c.kind = match value.to_ascii_lowercase().as_str() {
                    "demo" => Kind::Demo,
                    "rs232" => Kind::Rs232,
                    "usb" => Kind::Usb,
                    "node" => Kind::Node,
                    "server" => Kind::Server,
                    other => {
                        return Err(format!(
                    "line {}: connection must be demo, rs232, usb, node or server, not '{other}'",
                    n + 1
                ))
                    }
                }
            }
            "rs232_port" => c.rs232_port = value,
            "usb_port" => c.usb_port = value,
            "node_address" => c.node_address = value,
            "node_token" => c.node_token = value,
            "server_address" => c.server_address = value,
            "server_token" => c.server_token = value,
            "server_machine" => c.server_machine = value,
            "xy_hz_limit" | "z_hz_limit" => {
                let v: f64 = value.parse().map_err(|_| {
                    format!("line {}: {key} must be a number of steps per second", n + 1)
                })?;
                if !(50.0..=3840.0).contains(&v) {
                    return Err(format!("line {}: {key} must be between 50 and 3840 (the most the baud rate allows)", n + 1));
                }
                if key == "xy_hz_limit" {
                    c.xy_hz_limit = Some(v);
                } else {
                    c.z_hz_limit = Some(v);
                }
            }
            "home_signal" => {
                c.home_signal = match value.to_ascii_lowercase().as_str() {
                    "on" | "yes" | "true" | "1" => true,
                    "off" | "no" | "false" | "0" => false,
                    other => {
                        return Err(format!(
                            "line {}: home_signal must be on or off, not '{other}'",
                            n + 1
                        ))
                    }
                }
            }
            other => match c.tuning.set(other, &value) {
                Ok(true) => {}
                Ok(false) => return Err(format!("line {}: unknown setting '{other}'", n + 1)),
                Err(e) => return Err(format!("line {}: {e}", n + 1)),
            },
        }
    }
    Ok(c)
}

/// Where the shared file is looked for when no path is given.
pub fn default_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    let folder = if cfg!(windows) { "EH3D" } else { "eh3d" };
    base.join(folder).join("machine.cfg")
}

/// Loads the file at `path`, or the default one. A missing default file means `demo`; a missing file that was asked
/// for by name is an error.
pub fn load(path: Option<&Path>) -> Result<ConnectionConfig, String> {
    let (file, named) = match path {
        Some(p) => (p.to_path_buf(), true),
        None => (default_path(), false),
    };
    match std::fs::read_to_string(&file) {
        Ok(text) => parse(&text).map_err(|e| format!("{}: {e}", file.display())),
        Err(e) if e.kind() == io::ErrorKind::NotFound && !named => Ok(ConnectionConfig::default()),
        Err(e) => Err(format!("{}: {e}", file.display())),
    }
}

impl ConnectionConfig {
    /// Short text for the screen: the kind and where it goes.
    pub fn describe(&self) -> String {
        match self.kind {
            Kind::Demo => "demo".into(),
            Kind::Rs232 => format!("rs232 {}", self.rs232_port),
            Kind::Usb => format!("usb {}", self.usb_port),
            Kind::Node => format!("node {}", self.node_address),
            Kind::Server => format!(
                "server {} machine {}",
                self.server_address, self.server_machine
            ),
        }
    }

    /// How the steps are timed with this kind of link.
    pub fn pacing(&self) -> Pacing {
        match self.kind {
            Kind::Demo | Kind::Rs232 => Pacing::Bytes,
            Kind::Usb | Kind::Node | Kind::Server if self.tuning.stream_keep_direction => {
                Pacing::StreamKeepDir
            }
            Kind::Usb | Kind::Node | Kind::Server => Pacing::Stream,
        }
    }

    /// True when the job moves a real machine.
    pub fn is_real(&self) -> bool {
        self.kind != Kind::Demo
    }

    /// Opens the link. Nothing moves yet: the handshake and the arming are the caller's (`Machine`).
    pub fn open(&self) -> io::Result<Box<dyn Transport + Send>> {
        match self.kind {
            Kind::Demo => Ok(Box::new(Simulator::default())),
            Kind::Rs232 => open_serial(&self.rs232_port),
            Kind::Usb => open_serial(&self.usb_port),
            Kind::Node => open_node(&self.node_address, &self.node_token),
            Kind::Server => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "a job on the server runs through the server, not through a link of this program",
            )),
        }
    }
}

#[cfg(feature = "serial")]
fn open_serial(port: &str) -> io::Result<Box<dyn Transport + Send>> {
    Ok(Box::new(crate::serial::SerialTransport::open(port)?))
}

#[cfg(not(feature = "serial"))]
fn open_serial(_port: &str) -> io::Result<Box<dyn Transport + Send>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "this build has no serial port support",
    ))
}

#[cfg(feature = "node")]
fn open_node(address: &str, token: &str) -> io::Result<Box<dyn Transport + Send>> {
    if address.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "node_address is not set in the connection file",
        ));
    }
    Ok(Box::new(crate::node::NodeTransport::connect(
        address, token,
    )?))
}

#[cfg(not(feature = "node"))]
fn open_node(_address: &str, _token: &str) -> io::Result<Box<dyn Transport + Send>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "this build has no node support",
    ))
}
