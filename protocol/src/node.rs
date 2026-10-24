//! Transport through the EH3D-NODE-BUNGARD: the program talks TCP to a small node in the machine's box,
//! and the node sends the bytes out of its own UART, so the timing of the steps does not depend on this
//! computer or on a USB adapter (feature `node`).
//!
//! The link is framed: `0xB5, type, length (u16 LE), payload, CRC-16/CCITT-FALSE (u16 LE)`. The node
//! wants a token first and the machine to be armed before it takes any byte for the control unit; it stops
//! (empties its queue and switches the stepper drivers off) when an armed session goes quiet or the
//! connection drops, so [`NodeTransport`] pings while it waits.
//!
//! Bytes for the control unit are best sent with [`crate::run_paced`] (or `Machine::run_paced`): the node
//! queues the whole stream and its UART spaces the bytes at 38400 baud exactly.

use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use crate::{ModemStatus, Transport};

pub const SYNC: u8 = 0xB5;
pub const MAX_PAYLOAD: usize = 2048;

/// Frame types of the link (see the node's PROTOCOL document).
pub mod kind {
    pub const HELLO: u8 = 0x01;
    pub const ARM: u8 = 0x02;
    pub const DISARM: u8 = 0x03;
    pub const DTR: u8 = 0x04;
    pub const WRITE: u8 = 0x05;
    pub const ABORT: u8 = 0x06;
    pub const PING: u8 = 0x07;
    pub const SPINDLE: u8 = 0x08;
    pub const STOP_ON_HOME: u8 = 0x09;
    pub const INFO: u8 = 0x81;
    pub const STATUS: u8 = 0x82;
    pub const RX: u8 = 0x83;
    pub const ERROR: u8 = 0x84;
    pub const HOME_SEEN: u8 = 0x85;
}

pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Builds a frame; `None` if the payload is too long.
pub fn encode(kind: u8, payload: &[u8]) -> Option<Vec<u8>> {
    if payload.len() > MAX_PAYLOAD {
        return None;
    }
    let mut out = Vec::with_capacity(payload.len() + 6);
    out.push(SYNC);
    out.push(kind);
    out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    out.extend_from_slice(payload);
    let crc = crc16(&out[1..]);
    out.extend_from_slice(&crc.to_le_bytes());
    Some(out)
}

/// A decoded frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub kind: u8,
    pub payload: Vec<u8>,
}

/// Stream decoder. After a damaged frame it looks again from the byte after the false start, so a false
/// start never swallows the good frames behind it.
#[derive(Debug, Default)]
pub struct Decoder {
    buf: Vec<u8>,
    /// Frames thrown away because of a bad length or a bad CRC.
    pub damaged: u32,
}

impl Decoder {
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// The next complete, valid frame in the buffer, if any.
    pub fn next_frame(&mut self) -> Option<Frame> {
        loop {
            let start = self.buf.iter().position(|&b| b == SYNC)?;
            self.buf.drain(..start);
            if self.buf.len() < 4 {
                return None;
            }
            let len = u16::from_le_bytes([self.buf[2], self.buf[3]]) as usize;
            if len > MAX_PAYLOAD {
                self.damaged += 1;
                self.buf.drain(..1);
                continue;
            }
            let total = 4 + len + 2;
            if self.buf.len() < total {
                return None;
            }
            let crc = u16::from_le_bytes([self.buf[4 + len], self.buf[5 + len]]);
            if crc != crc16(&self.buf[1..4 + len]) {
                self.damaged += 1;
                self.buf.drain(..1);
                continue;
            }
            let frame = Frame {
                kind: self.buf[1],
                payload: self.buf[4..4 + len].to_vec(),
            };
            self.buf.drain(..total);
            return Some(frame);
        }
    }
}

/// What the node says about itself when the session opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeInfo {
    pub version: (u8, u8, u8),
    pub baud: u32,
    pub queue_capacity: u32,
}

fn error_text(code: u8) -> &'static str {
    match code {
        1 => "not logged in",
        2 => "bad token",
        3 => "the machine is not armed",
        4 => "queue full",
        5 => "bad frame",
        6 => "the node has no token and refuses control",
        7 => "the session went quiet and the node stopped",
        8 => "this node has no spindle output",
        9 => "the spindle must stop before it can turn the other way",
        10 => "the spindle was stopped too recently to turn the other way",
        11 => "the node is finishing the job it holds and takes nothing new",
        _ => "unknown error",
    }
}

pub struct NodeTransport {
    stream: TcpStream,
    decoder: Decoder,
    pub info: NodeInfo,
    armed: bool,
    lines: ModemStatus,
    spindle_rpm: u16,
    spindle_reverse: bool,
    lamps: u8,
    draining: bool,
    home_seen: Option<u8>,
    queue_fill: u32,
    node_sent: u32,
    written: u64,
    node_sent_base: Option<u32>,
    rx: Vec<u8>,
    error: Option<u8>,
    last_tx: Instant,
}

impl NodeTransport {
    /// Opens the link and logs in. The machine stays disarmed until [`NodeTransport::arm`].
    pub fn connect(addr: impl ToSocketAddrs, token: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_millis(20)))?;
        let mut t = NodeTransport {
            stream,
            decoder: Decoder::default(),
            info: NodeInfo {
                version: (0, 0, 0),
                baud: 0,
                queue_capacity: 0,
            },
            armed: false,
            lines: ModemStatus {
                cts: false,
                dsr: false,
                ri: false,
                dcd: false,
            },
            spindle_rpm: 0,
            spindle_reverse: false,
            lamps: 0,
            draining: false,
            home_seen: None,
            queue_fill: 0,
            node_sent: 0,
            written: 0,
            node_sent_base: None,
            rx: Vec::new(),
            error: None,
            last_tx: Instant::now(),
        };
        t.send(kind::HELLO, token.as_bytes())?;
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut got_info = false;
        while !got_info && Instant::now() < deadline {
            t.pump()?;
            if let Some(code) = t.error.take() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("the node refused the login: {}", error_text(code)),
                ));
            }
            got_info = t.info.baud != 0;
        }
        if !got_info {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "the node did not answer the login",
            ));
        }
        Ok(t)
    }

    /// Allows the node to take bytes for the control unit. Call it only after the checks that
    /// `Machine::arm` asks for.
    pub fn arm(&mut self) -> io::Result<()> {
        self.send(kind::ARM, &[])?;
        self.armed = true;
        Ok(())
    }

    pub fn disarm(&mut self) -> io::Result<()> {
        self.armed = false;
        self.send(kind::DISARM, &[])
    }

    /// Empties the node's queue, switches the drivers off and disarms, at once.
    pub fn abort(&mut self) -> io::Result<()> {
        self.armed = false;
        self.send(kind::ABORT, &[])
    }

    /// Sets the spindle of the node: speed in rpm (0 stops it) and direction. A speed above 0 needs the node armed, the
    /// direction cannot change while it turns, and the node never goes above the speed it was built for.
    pub fn set_spindle(&mut self, rpm: u16, reverse: bool) -> io::Result<()> {
        let r = rpm.to_le_bytes();
        self.send(kind::SPINDLE, &[r[0], r[1], u8::from(reverse)])
    }

    /// Asks the node to drop the rest of the queue the moment the HOME lamp of one of these axes lights (bit 0 X, 1 Y, 2 Z; 0
    /// cancels). The control unit has already stopped the axis: this only saves the wait. Needs the lamps wired to the node.
    pub fn stop_on_home(&mut self, axes: u8) -> io::Result<()> {
        self.home_seen = None;
        self.send(kind::STOP_ON_HOME, &[axes & 7])
    }

    /// The lamps the node last reported: HOME X 1, HOME Y 2, HOME Z 4, ENABLE 8, TOOL 16.
    pub fn lamps(&self) -> u8 {
        self.lamps
    }

    /// The lamps the node saw when a `stop_on_home` request emptied its queue, once.
    pub fn take_home_seen(&mut self) -> Option<u8> {
        let _ = self.pump();
        self.home_seen.take()
    }

    /// A spindle command that waits in the node's queue and is applied when every byte sent before it has gone out (so a job can start and
    /// stop the spindle in the right places and still run to its end if this program goes away). The same rules as `set_spindle`, judged on
    /// the commands queued so far: no change of direction while the queued spindle turns, and the pauses of the stream between a stop and a
    /// start the other way must last as long as the node's settling time.
    pub fn spindle_at_end_of_queue(&mut self, rpm: u16, reverse: bool) -> io::Result<()> {
        let r = rpm.to_le_bytes();
        self.send(kind::SPINDLE, &[r[0], r[1], u8::from(reverse), 1])
    }

    /// Whether the node is finishing the job it holds (this program went away earlier) and so takes nothing new.
    pub fn draining(&self) -> bool {
        self.draining
    }

    /// The speed in rpm and the direction (true: reverse) the node last reported.
    pub fn spindle(&self) -> (u16, bool) {
        (self.spindle_rpm, self.spindle_reverse)
    }

    /// Bytes the control unit has sent back, taken from what has arrived.
    pub fn take_rx(&mut self) -> Vec<u8> {
        let _ = self.pump();
        std::mem::take(&mut self.rx)
    }

    /// Queue fill and bytes sent as last reported by the node.
    pub fn node_queue(&self) -> (u32, u32) {
        (self.queue_fill, self.node_sent)
    }

    fn send(&mut self, kind: u8, payload: &[u8]) -> io::Result<()> {
        let frame = encode(kind, payload)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "payload too long"))?;
        self.stream.write_all(&frame)?;
        self.last_tx = Instant::now();
        Ok(())
    }

    fn fail_if_error(&mut self) -> io::Result<()> {
        match self.error.take() {
            Some(code) => Err(io::Error::other(format!(
                "the node says: {}",
                error_text(code)
            ))),
            None => Ok(()),
        }
    }

    /// Reads whatever has arrived (waits at most the read timeout) and applies the frames.
    fn pump(&mut self) -> io::Result<()> {
        let mut buf = [0u8; 2048];
        match self.stream.read(&mut buf) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "the node closed the link",
                ))
            }
            Ok(n) => self.decoder.feed(&buf[..n]),
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(e),
        }
        while let Some(f) = self.decoder.next_frame() {
            match f.kind {
                kind::INFO if f.payload.len() >= 11 => {
                    let p = &f.payload;
                    self.info = NodeInfo {
                        version: (p[0], p[1], p[2]),
                        baud: u32::from_le_bytes([p[3], p[4], p[5], p[6]]),
                        queue_capacity: u32::from_le_bytes([p[7], p[8], p[9], p[10]]),
                    };
                }
                kind::STATUS if f.payload.len() >= 10 => {
                    let p = &f.payload;
                    self.queue_fill = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
                    self.node_sent = u32::from_le_bytes([p[4], p[5], p[6], p[7]]);
                    self.node_sent_base.get_or_insert(self.node_sent);
                    self.armed = p[8] != 0;
                    let l = p[9];
                    if p.len() >= 13 {
                        self.spindle_rpm = u16::from_le_bytes([p[10], p[11]]);
                        self.spindle_reverse = p[12] != 0;
                    }
                    if p.len() >= 14 {
                        self.lamps = p[13];
                    }
                    if p.len() >= 15 {
                        self.draining = p[14] & 1 != 0;
                    }
                    self.lines = ModemStatus {
                        cts: l & 1 != 0,
                        dsr: l & 2 != 0,
                        ri: l & 4 != 0,
                        dcd: l & 8 != 0,
                    };
                }
                kind::HOME_SEEN if !f.payload.is_empty() => {
                    self.lamps = f.payload[0];
                    self.home_seen = Some(f.payload[0]);
                }
                kind::RX => self.rx.extend_from_slice(&f.payload),
                kind::ERROR if !f.payload.is_empty() => self.error = Some(f.payload[0]),
                _ => {}
            }
        }
        Ok(())
    }

    /// Keeps an armed session alive while the program has nothing to send.
    fn keep_alive(&mut self) -> io::Result<()> {
        if self.armed && self.last_tx.elapsed() >= Duration::from_millis(500) {
            self.send(kind::PING, &[])?;
        }
        Ok(())
    }

    /// True when the node has put out everything written so far.
    fn drained(&self) -> bool {
        match self.node_sent_base {
            Some(base) => u64::from(self.node_sent.wrapping_sub(base)) >= self.written,
            None => false,
        }
    }
}

impl Transport for NodeTransport {
    fn set_dtr(&mut self, on: bool) -> io::Result<()> {
        self.send(kind::DTR, &[on as u8])
    }

    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        self.write_bytes(&[byte])
    }

    fn set_spindle(&mut self, rpm: u32, reverse: bool) -> io::Result<()> {
        NodeTransport::set_spindle(self, rpm.min(u32::from(u16::MAX)) as u16, reverse)
    }

    /// Queues the bytes on the node, waiting for room in its queue when it is full.
    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        if !self.armed {
            return Err(io::Error::other("the node is not armed: call arm() first"));
        }
        for chunk in bytes.chunks(1024) {
            let deadline = Instant::now() + Duration::from_secs(10);
            // wait until the whole chunk fits in what the node has room for
            while u64::from(self.queue_fill) + chunk.len() as u64
                > u64::from(self.info.queue_capacity)
            {
                self.pump()?;
                self.fail_if_error()?;
                self.keep_alive()?;
                if Instant::now() > deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "the node's queue did not drain",
                    ));
                }
            }
            self.send(kind::WRITE, chunk)?;
            self.queue_fill += chunk.len() as u32;
            self.written += chunk.len() as u64;
        }
        self.fail_if_error()
    }

    fn arm_link(&mut self) -> io::Result<()> {
        self.arm()
    }

    fn disarm_link(&mut self) -> io::Result<()> {
        self.disarm()
    }

    fn abort_link(&mut self) -> io::Result<()> {
        self.abort()
    }

    fn status(&mut self) -> io::Result<ModemStatus> {
        self.pump()?;
        self.fail_if_error()?;
        Ok(self.lines)
    }

    /// Waits `us` microseconds, or less when the node has already put out everything written (the
    /// stream is its own clock). Pings meanwhile so the node does not stop an armed session.
    fn sleep_us(&mut self, us: u64) {
        let start = Instant::now();
        let total = Duration::from_micros(us);
        while start.elapsed() < total {
            let _ = self.pump();
            let _ = self.keep_alive();
            if self.written > 0 && self.drained() && start.elapsed() >= Duration::from_millis(60) {
                break;
            }
            if total - start.elapsed() < Duration::from_millis(2) {
                break;
            }
        }
    }
}
