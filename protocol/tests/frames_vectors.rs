//! The frames of the link to the node against the shared test vectors (`tests/testdata/frames.json`), the same file that the firmware of the node tests its C++ code with: what this side
//! writes is what the node writes, and the decoder gives back the type and the payload, counts the damage and resynchronises on the next good frame.
#![cfg(feature = "node")]

use eh3d_drillpro_protocol::node::{crc16, encode, Decoder, SYNC};

const FILE: &str = include_str!("testdata/frames.json");

fn hex(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2), "odd hex text {text}");
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

/// The text value of `"key": "value"` inside one object of the file.
fn text_of(object: &str, key: &str) -> String {
    let marker = format!("\"{key}\": \"");
    let start = object
        .find(&marker)
        .unwrap_or_else(|| panic!("{key} in {object}"))
        + marker.len();
    object[start..start + object[start..].find('"').unwrap()].to_string()
}

/// The whole number value of `"key": 12` inside one object of the file.
fn number_of(object: &str, key: &str) -> u64 {
    let marker = format!("\"{key}\": ");
    let start = object
        .find(&marker)
        .unwrap_or_else(|| panic!("{key} in {object}"))
        + marker.len();
    let digits: String = object[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().unwrap()
}

/// The objects of one array of the file (`"vectors"` or `"damaged"`), each as its own text: the objects are flat, so the closing brace ends them.
fn objects(section: &str) -> Vec<String> {
    let from = FILE.find(&format!("\"{section}\": [")).expect(section);
    let body = &FILE[from..];
    let end = body.find("\n ]").expect("end of the array");
    body[..end]
        .split('{')
        .skip(1)
        .map(|o| o.split('}').next().unwrap().to_string())
        .collect()
}

#[test]
fn the_sync_byte_and_the_crc_are_the_ones_of_the_file() {
    assert_eq!(number_of(FILE, "sync"), u64::from(SYNC));
    assert_eq!(crc16(b"123456789"), 0x29B1);
}

#[test]
fn every_vector_is_written_and_read_back_the_same() {
    let vectors = objects("vectors");
    assert!(vectors.len() >= 10, "{}", vectors.len());
    for v in &vectors {
        let name = text_of(v, "name");
        let kind = number_of(v, "type") as u8;
        let payload = hex(&text_of(v, "payload"));
        let want = hex(&text_of(v, "frame"));
        assert_eq!(encode(kind, &payload).unwrap(), want, "{name}");
        let mut d = Decoder::default();
        d.feed(&want);
        let frame = d.next_frame().unwrap_or_else(|| panic!("{name}: no frame"));
        assert_eq!((frame.kind, frame.payload), (kind, payload), "{name}");
        assert!(d.next_frame().is_none() && d.damaged == 0, "{name}");
        // fed one byte at a time the result is the same, and two in a row come out as two
        let mut slow = Decoder::default();
        let mut got = Vec::new();
        for b in want.iter().chain(want.iter()) {
            slow.feed(&[*b]);
            while let Some(f) = slow.next_frame() {
                got.push(f);
            }
        }
        assert_eq!(got.len(), 2, "{name}");
    }
}

#[test]
fn damaged_streams_give_the_good_frames_and_count_the_damage() {
    let damaged = objects("damaged");
    assert!(damaged.len() >= 4, "{}", damaged.len());
    for v in &damaged {
        let name = text_of(v, "name");
        let mut d = Decoder::default();
        d.feed(&hex(&text_of(v, "bytes")));
        let mut frames = 0;
        while d.next_frame().is_some() {
            frames += 1;
        }
        assert_eq!(frames as u64, number_of(v, "frames"), "{name}");
        assert!(
            u64::from(d.damaged) >= number_of(v, "damaged_at_least"),
            "{name}"
        );
    }
}
