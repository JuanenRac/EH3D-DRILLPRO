use eh3d_drillpro_protocol::nullmodem::*;

#[test]
fn data_and_line_frames_are_told_apart() {
    let mut d = Decoder::default();
    let f = d.feed(&[0x00, 0x30, 0xFF, 0x02, 0x4F, 0xFF, 0xFF, 0xFF, 0x03]);
    assert_eq!(
        f,
        vec![
            Frame::Data(0x00),
            Frame::Data(0x30),
            Frame::Lines {
                rts: false,
                dtr: true,
                brk: false
            },
            Frame::Data(0x4F),
            Frame::Data(0xFF),
            Frame::Lines {
                rts: true,
                dtr: true,
                brk: false
            },
        ]
    );
}

#[test]
fn a_frame_split_across_reads_is_decoded_once() {
    let mut d = Decoder::default();
    assert_eq!(d.feed(&[0x10, 0xFF]), vec![Frame::Data(0x10)]);
    assert_eq!(
        d.feed(&[0x00]),
        vec![Frame::Lines {
            rts: false,
            dtr: false,
            brk: false
        }]
    );
    assert_eq!(d.feed(&[0xFF]), vec![]);
    assert_eq!(d.feed(&[0xFF]), vec![Frame::Data(0xFF)]);
}

#[test]
fn encoding_doubles_ff_and_round_trips() {
    let raw = [0x01, 0xFF, 0x43, 0xFF, 0xFF];
    let wire = encode_data(&raw);
    assert_eq!(wire, vec![0x01, 0xFF, 0xFF, 0x43, 0xFF, 0xFF, 0xFF, 0xFF]);
    let back: Vec<u8> = Decoder::default()
        .feed(&wire)
        .into_iter()
        .map(|f| {
            if let Frame::Data(b) = f {
                b
            } else {
                panic!("unexpected line frame")
            }
        })
        .collect();
    assert_eq!(back, raw);
}

#[test]
fn machine_lines_become_cts_and_dsr() {
    assert_eq!(lines_from_machine(false, false), [0xFF, 0x00]);
    assert_eq!(lines_from_machine(true, false), [0xFF, 0x01]);
    assert_eq!(lines_from_machine(false, true), [0xFF, 0x02]);
    assert_eq!(lines_from_machine(true, true), [0xFF, 0x03]);
}
