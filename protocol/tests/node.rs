//! The link to the EH3D-NODE-BUNGARD: the frames (with the vectors the node's firmware tests use too), the
//! decoder, and `NodeTransport` against a stand-in node on the local machine.
#![cfg(feature = "node")]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eh3d_drillpro_protocol::node::{crc16, encode, kind, Decoder, NodeTransport};
use eh3d_drillpro_protocol::Transport;

#[test]
fn crc_and_frame_match_the_firmware_vectors() {
    assert_eq!(crc16(b"123456789"), 0x29B1);
    let f = encode(kind::WRITE, &[0x43, 0x00, 0x43, 0xFF]).unwrap();
    assert_eq!(
        f,
        vec![0xB5, 0x05, 0x04, 0x00, 0x43, 0x00, 0x43, 0xFF, 0xE7, 0x3D]
    );
    assert_eq!(encode(kind::PING, &[]).unwrap().len(), 6);
    assert!(encode(kind::WRITE, &vec![0; 2049]).is_none());
    assert!(encode(kind::WRITE, &vec![0; 2048]).is_some());
}

#[test]
fn decoder_handles_split_garbage_and_damaged_frames() {
    let a = encode(kind::ARM, &[]).unwrap();
    let w = encode(kind::WRITE, &[1, 2, 3]).unwrap();
    // split across feeds
    let mut d = Decoder::default();
    d.feed(&w[..3]);
    assert!(d.next_frame().is_none());
    d.feed(&w[3..]);
    let f = d.next_frame().unwrap();
    assert_eq!((f.kind, f.payload), (kind::WRITE, vec![1, 2, 3]));
    assert!(d.next_frame().is_none());

    // garbage and a false start of frame (0xB5 0xFF 0xB5 0x02 would claim 693 bytes), then two real frames
    let mut stream = vec![0x00, 0x13, 0xB5, 0xFF];
    stream.extend(&a);
    stream.extend(&w);
    stream.extend(vec![0u8; 800]);
    stream.extend(encode(kind::PING, &[]).unwrap());
    let mut d = Decoder::default();
    d.feed(&stream);
    let kinds: Vec<u8> = std::iter::from_fn(|| d.next_frame())
        .map(|f| f.kind)
        .collect();
    assert_eq!(kinds, vec![kind::ARM, kind::WRITE, kind::PING]);

    // a damaged byte drops the frame and is counted
    let mut bad = encode(kind::WRITE, &[1, 2, 3]).unwrap();
    bad[5] ^= 1;
    let mut d = Decoder::default();
    d.feed(&bad);
    assert!(d.next_frame().is_none());
    assert!(d.damaged >= 1);
}

struct FakeNode {
    addr: SocketAddr,
    received: Arc<Mutex<Vec<u8>>>,
    seen: Arc<Mutex<Vec<u8>>>, // the kinds of frames received, in order
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for FakeNode {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// A stand-in for the node: logs in with `token`, reports `capacity`, takes Write bytes only while armed and
/// answers every Write with a Status that says everything has been sent.
fn fake_node(token: &'static str, capacity: u32) -> FakeNode {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let received = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let (rc, sn, st) = (received.clone(), seen.clone(), stop.clone());
    let handle = thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
        let mut d = Decoder::default();
        let (mut authed, mut armed) = (false, false);
        let status = |sent: usize, armed: bool| {
            let mut p = Vec::new();
            p.extend(0u32.to_le_bytes());
            p.extend((sent as u32).to_le_bytes());
            p.push(armed as u8);
            p.push(0x08); // DCD
            encode(kind::STATUS, &p).unwrap()
        };
        let mut buf = [0u8; 4096];
        while !st.load(Ordering::Relaxed) {
            match s.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => d.feed(&buf[..n]),
                Err(_) => {}
            }
            while let Some(f) = d.next_frame() {
                sn.lock().unwrap().push(f.kind);
                match f.kind {
                    kind::HELLO => {
                        if f.payload == token.as_bytes() {
                            authed = true;
                            let mut p = vec![0, 0, 1];
                            p.extend(38_400u32.to_le_bytes());
                            p.extend(capacity.to_le_bytes());
                            s.write_all(&encode(kind::INFO, &p).unwrap()).unwrap();
                            s.write_all(&status(0, armed)).unwrap();
                        } else {
                            s.write_all(&encode(kind::ERROR, &[2]).unwrap()).unwrap();
                        }
                    }
                    kind::SPINDLE if authed => {
                        let mut p = vec![0u8; 8];
                        p.push(armed as u8);
                        p.push(0);
                        p.extend(&f.payload[..2]);
                        p.push(*f.payload.get(2).unwrap_or(&0));
                        s.write_all(&encode(kind::STATUS, &p).unwrap()).unwrap();
                    }
                    kind::STOP_ON_HOME if authed => {
                        s.write_all(&encode(kind::HOME_SEEN, &[0b101]).unwrap())
                            .unwrap();
                    }
                    kind::ARM if authed => armed = true,
                    kind::DISARM if authed => armed = false,
                    kind::WRITE if authed => {
                        if armed {
                            rc.lock().unwrap().extend(&f.payload);
                            let total = rc.lock().unwrap().len();
                            s.write_all(&status(total, armed)).unwrap();
                        } else {
                            s.write_all(&encode(kind::ERROR, &[3]).unwrap()).unwrap();
                        }
                    }
                    _ => {}
                }
            }
        }
    });
    FakeNode {
        addr,
        received,
        seen,
        stop,
        handle: Some(handle),
    }
}

#[test]
fn login_with_the_right_token_gives_the_node_info() {
    let node = fake_node("secret", 4096);
    let t = NodeTransport::connect(node.addr, "secret").unwrap();
    assert_eq!(t.info.version, (0, 0, 1));
    assert_eq!(t.info.baud, 38_400);
    assert_eq!(t.info.queue_capacity, 4096);
}

#[test]
fn login_with_a_wrong_token_is_refused() {
    let node = fake_node("secret", 4096);
    let err = NodeTransport::connect(node.addr, "nope")
        .err()
        .expect("must fail");
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(err.to_string().contains("bad token"));
}

#[test]
fn bytes_are_refused_until_armed_and_then_arrive_in_order_across_chunks() {
    let node = fake_node("t", 4096);
    let mut t = NodeTransport::connect(node.addr, "t").unwrap();
    assert!(t.write_bytes(&[1, 2, 3]).is_err()); // not armed: nothing is sent
    thread::sleep(Duration::from_millis(100));
    assert!(node.received.lock().unwrap().is_empty());
    assert!(!node.seen.lock().unwrap().contains(&kind::WRITE));

    t.arm().unwrap();
    let data: Vec<u8> = (0..3000u32).map(|i| (i * 7 % 251) as u8).collect();
    t.write_bytes(&data).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while node.received.lock().unwrap().len() < data.len() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(*node.received.lock().unwrap(), data);
}

#[test]
fn the_input_lines_come_from_the_nodes_status() {
    let node = fake_node("t", 4096);
    let mut t = NodeTransport::connect(node.addr, "t").unwrap();
    thread::sleep(Duration::from_millis(50));
    let s = t.status().unwrap();
    assert!(s.dcd && !s.ri && !s.cts && !s.dsr);
}

#[test]
fn waiting_ends_early_once_the_node_has_put_everything_out() {
    let node = fake_node("t", 4096);
    let mut t = NodeTransport::connect(node.addr, "t").unwrap();
    t.arm().unwrap();
    t.write_bytes(&[0x43, 0, 0, 0]).unwrap();
    let start = Instant::now();
    t.sleep_us(5_000_000); // the stream would take 5 s; the node reports it all sent at once
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "waited {:?}",
        start.elapsed()
    );
}

#[test]
fn a_node_that_closes_the_link_is_reported() {
    let node = fake_node("t", 4096);
    let mut t = NodeTransport::connect(node.addr, "t").unwrap();
    t.arm().unwrap();
    drop(node); // the stand-in node goes away
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut failed = false;
    while Instant::now() < deadline {
        if t.status().is_err() {
            failed = true;
            break;
        }
    }
    assert!(failed);
}

#[test]
fn the_spindle_speed_and_direction_go_to_the_node_and_come_back_in_the_status() {
    let node = fake_node("t", 4096);
    let mut t = NodeTransport::connect(node.addr, "t").unwrap();
    t.set_spindle(18_000, true).unwrap();
    for _ in 0..50 {
        let _ = t.take_rx();
        if t.spindle() == (18_000, true) {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(t.spindle(), (18_000, true));
    assert!(node.seen.lock().unwrap().contains(&kind::SPINDLE));
}

#[test]
fn a_stop_on_home_request_is_answered_with_the_lamps_that_were_lit() {
    let node = fake_node("t", 4096);
    let mut t = NodeTransport::connect(node.addr, "t").unwrap();
    t.stop_on_home(0b011).unwrap();
    let mut seen = None;
    for _ in 0..50 {
        seen = t.take_home_seen();
        if seen.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(seen, Some(0b101));
    assert_eq!(t.lamps(), 0b101);
    assert_eq!(t.take_home_seen(), None); // only once
}

#[test]
fn a_queued_spindle_command_carries_the_flag_and_the_status_says_when_the_node_is_finishing() {
    let node = fake_node("t", 4096);
    let mut t = NodeTransport::connect(node.addr, "t").unwrap();
    t.spindle_at_end_of_queue(12_000, true).unwrap();
    for _ in 0..50 {
        let _ = t.take_rx();
        if t.spindle() == (12_000, true) {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    // the stand-in answers every Spindle frame with a Status that repeats its payload: the speed and the direction came back
    assert_eq!(t.spindle(), (12_000, true));
    assert!(!t.draining());
}
