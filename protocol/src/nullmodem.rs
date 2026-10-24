//! The wire format of DOSBox-X's `nullmodem` serial device (non-transparent mode), as used by the
//! serial bridge that connects the emulated original program to a real machine.
//!
//! Data bytes travel as they are, with `0xFF` doubled. A modem-line change is `0xFF` followed by a
//! status byte. From the emulated PC: bit 0 = RTS, bit 1 = DTR, bit 2 = break. To the emulated PC:
//! bit 0 = CTS, bit 1 = DSR (DOSBox-X cannot deliver RI or DCD this way; the bridge maps the machine's
//! RI and DCD onto CTS and DSR and the emulated program is patched to read those).

/// One decoded item from the emulated PC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    Data(u8),
    Lines { rts: bool, dtr: bool, brk: bool },
}

/// Incremental decoder (a frame may be split across reads).
#[derive(Debug, Default)]
pub struct Decoder {
    escape: bool,
}

impl Decoder {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Frame> {
        let mut out = Vec::new();
        for &b in bytes {
            if self.escape {
                self.escape = false;
                if b == 0xFF {
                    out.push(Frame::Data(0xFF));
                } else {
                    out.push(Frame::Lines {
                        rts: b & 1 != 0,
                        dtr: b & 2 != 0,
                        brk: b & 4 != 0,
                    });
                }
            } else if b == 0xFF {
                self.escape = true;
            } else {
                out.push(Frame::Data(b));
            }
        }
        out
    }
}

/// Encodes data bytes for the emulated PC (or from it): `0xFF` is doubled.
pub fn encode_data(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for &b in bytes {
        out.push(b);
        if b == 0xFF {
            out.push(0xFF);
        }
    }
    out
}

/// The line-status frame sent to the emulated PC: CTS in bit 0, DSR in bit 1.
pub fn encode_lines_to_pc(cts: bool, dsr: bool) -> [u8; 2] {
    [0xFF, (cts as u8) | ((dsr as u8) << 1)]
}

/// The machine side of the line mapping: the machine's RI is shown to the program as CTS and its
/// DCD as DSR.
pub fn lines_from_machine(ri: bool, dcd: bool) -> [u8; 2] {
    encode_lines_to_pc(ri, dcd)
}
