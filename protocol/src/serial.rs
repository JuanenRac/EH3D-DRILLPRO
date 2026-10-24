//! Real serial port transport (feature `serial`). Not validated against hardware yet, except for the
//! handshake and reading the input lines, which were tried on the real machine.

use std::io::{self, Write};
use std::time::Duration;

use serialport::SerialPort;

use crate::{ModemStatus, Transport, BAUD};

#[cfg(windows)]
type Port = serialport::COMPort;
#[cfg(not(windows))]
type Port = Box<dyn SerialPort>;

pub struct SerialTransport {
    port: Port,
}

impl SerialTransport {
    /// Opens `name` (for example `COM2` or `/dev/ttyUSB0`) as 38400 8N1 without flow control.
    pub fn open(name: &str) -> io::Result<Self> {
        let builder = serialport::new(name, BAUD)
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .flow_control(serialport::FlowControl::None)
            .timeout(Duration::from_millis(100));
        #[cfg(windows)]
        let port = builder.open_native().map_err(io::Error::other)?;
        #[cfg(not(windows))]
        let port = builder.open().map_err(io::Error::other)?;
        Ok(Self { port })
    }

    /// Writes a whole buffer in one call, so the UART (not the PC clock) spaces the bytes.
    pub fn write_all_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.port.write_all(bytes)
    }

    /// A second handle on the same port, for a thread that only reads what the machine sends.
    pub fn rx_reader(&self) -> io::Result<Box<dyn SerialPort>> {
        self.port.try_clone().map_err(io::Error::other)
    }

    /// Takes the bytes the machine has sent to the PC so far (empty if there are none).
    pub fn read_available(&mut self) -> Vec<u8> {
        use std::io::Read;
        let waiting = self.port.bytes_to_read().unwrap_or(0) as usize;
        let mut buf = vec![0u8; waiting];
        let n = if waiting == 0 {
            0
        } else {
            self.port.read(&mut buf).unwrap_or(0)
        };
        buf.truncate(n);
        buf
    }

    /// Lists the serial ports the system reports.
    pub fn available() -> Vec<String> {
        serialport::available_ports()
            .map(|v| v.into_iter().map(|p| p.port_name).collect())
            .unwrap_or_default()
    }
}

impl Transport for SerialTransport {
    fn set_dtr(&mut self, on: bool) -> io::Result<()> {
        self.port
            .write_data_terminal_ready(on)
            .map_err(io::Error::other)
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.port.write_all(bytes)
    }

    /// Writes the bytes in chunks while a second thread reads the home line (RI) from a second handle on the same port; once the line
    /// is up no more chunks go out. On a USB adapter the driver lets a reading through about every 0.2 s of writing, so the line is seen
    /// within a chunk or two; what was already written past the switches is swallowed by the control unit.
    fn write_bytes_until_home(&mut self, bytes: &[u8], chunk: usize) -> io::Result<bool> {
        use std::sync::atomic::{AtomicBool, Ordering};
        let mut watcher = self.port.try_clone().map_err(io::Error::other)?;
        let found = AtomicBool::new(false);
        let done = AtomicBool::new(false);
        let mut result = Ok(());
        std::thread::scope(|scope| {
            scope.spawn(|| {
                while !done.load(Ordering::Relaxed) {
                    if watcher.read_ring_indicator().unwrap_or(false) {
                        found.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            });
            for piece in bytes.chunks(chunk.max(1)) {
                if found.load(Ordering::Relaxed) {
                    break;
                }
                if let Err(e) = self.port.write_all(piece) {
                    result = Err(e);
                    break;
                }
            }
            done.store(true, Ordering::Relaxed);
        });
        result?;
        // the line may have risen during the last chunk
        Ok(found.load(Ordering::Relaxed) || self.status()?.ri)
    }

    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        self.port.write_all(&[byte])
    }

    /// Reads the four input lines. On Windows this is ONE call to the driver: reading each line
    /// separately costs four calls, and on a USB adapter such as the CH340 every call is a USB
    /// transfer of several milliseconds.
    #[cfg(windows)]
    fn status(&mut self) -> io::Result<ModemStatus> {
        use std::os::windows::io::AsRawHandle;

        extern "system" {
            fn GetCommModemStatus(handle: *mut std::ffi::c_void, status: *mut u32) -> i32;
        }
        let mut bits = 0u32;
        // SAFETY: the handle belongs to the open port owned by `self`, and `bits` is a valid u32.
        let ok = unsafe { GetCommModemStatus(self.port.as_raw_handle(), &mut bits) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        // The Windows bits (CTS 0x10, DSR 0x20, RING 0x40, RLSD 0x80) are the MSR layout.
        Ok(ModemStatus::from_msr(bits as u8))
    }

    #[cfg(not(windows))]
    fn status(&mut self) -> io::Result<ModemStatus> {
        let e = io::Error::other;
        Ok(ModemStatus {
            cts: self.port.read_clear_to_send().map_err(e)?,
            dsr: self.port.read_data_set_ready().map_err(e)?,
            ri: self.port.read_ring_indicator().map_err(e)?,
            dcd: self.port.read_carrier_detect().map_err(e)?,
        })
    }

    fn sleep_us(&mut self, us: u64) {
        // Long waits sleep; short ones spin so step timing stays tight.
        let end = std::time::Instant::now() + Duration::from_micros(us);
        if us > 2_000 {
            std::thread::sleep(Duration::from_micros(us - 1_500));
        }
        while std::time::Instant::now() < end {
            std::hint::spin_loop();
        }
    }
}
