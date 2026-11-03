//! The answers of the server as they come on the wire, the names that go into a request and the command line, from their edges: cut answers, chunks that lie, names that try to split a
//! request, and options that are written wrong.

use eh3d_drillpro::cli::{parse_from, parse_prefs, render_prefs, Prefs};
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::server_link::{check_request_part, error_text, parse_response, path_segment};
use serde_json::{json, Value};
use std::path::PathBuf;

fn http(status_line: &str, headers: &str, body: &[u8]) -> Vec<u8> {
    let mut v = format!("{status_line}\r\n{headers}\r\n").into_bytes();
    v.extend_from_slice(body);
    v
}

#[test]
fn a_plain_answer_has_its_status_and_its_json() {
    let (status, body) = parse_response(&http(
        "HTTP/1.1 200 OK",
        "Content-Type: application/json\r\n",
        br#"{"ok":true,"n":[1,2]}"#,
    ))
    .unwrap();
    assert_eq!(status, 200);
    assert_eq!(body, json!({"ok": true, "n": [1, 2]}));
    let (status, body) = parse_response(&http("HTTP/1.1 204 No Content", "", b"")).unwrap();
    assert_eq!((status, body), (204, Value::Null));
    let (status, _) = parse_response(&http(
        "HTTP/1.0 503 Service Unavailable",
        "",
        b"<html>gateway</html>",
    ))
    .unwrap();
    assert_eq!(status, 503);
}

#[test]
fn a_body_that_is_not_json_is_nothing_and_not_an_error() {
    for body in [&b"not json"[..], b"", b"{\"cut\":", b"\xFF\xFE\x00"] {
        let (status, value) = parse_response(&http("HTTP/1.1 200 OK", "", body)).unwrap();
        assert_eq!((status, value), (200, Value::Null), "{body:?}");
    }
}

#[test]
fn an_answer_that_is_cut_or_has_no_status_is_refused_with_a_reason() {
    let cases: &[(&[u8], &str)] = &[
        (b"", "incomplete"),
        (b"HTTP/1.1 200 OK\r\nContent-Length: 2", "incomplete"),
        (b"HTTP/1.1 200 OK\r\n\r", "incomplete"),
        (b"\r\n\r\n{}", "no status"),
        (b"HTTP/1.1\r\n\r\n{}", "no status"),
        (b"HTTP/1.1 OK 200\r\n\r\n{}", "no status"),
        (b"HTTP/1.1 99999 Huge\r\n\r\n{}", "no status"),
        (b"HTTP/1.1 -5 Neg\r\n\r\n{}", "no status"),
    ];
    for (raw, want) in cases {
        let e = parse_response(raw).unwrap_err();
        assert!(e.contains(want), "{:?}: {e}", String::from_utf8_lossy(raw));
    }
}

#[test]
fn a_body_sent_in_chunks_is_put_together_in_any_case_of_the_header() {
    let chunks = b"5\r\n{\"a\":\r\n4\r\n[1]}\r\n0\r\n\r\n";
    for header in [
        "Transfer-Encoding: chunked\r\n",
        "transfer-encoding: Chunked\r\n",
        "TRANSFER-ENCODING: gzip, chunked\r\n",
    ] {
        let (status, body) = parse_response(&http("HTTP/1.1 200 OK", header, chunks)).unwrap();
        assert_eq!((status, body), (200, json!({"a": [1]})), "{header}");
    }
    // a chunk may be bigger than nine bytes and its size is in hexadecimal, in either case
    let big = format!("{{\"k\":\"{}\"}}", "x".repeat(2));
    let raw = format!("{:X}\r\n{big}\r\n0\r\n\r\n", big.len());
    assert_eq!(
        parse_response(&http(
            "HTTP/1.1 200 OK",
            "Transfer-Encoding: chunked\r\n",
            raw.as_bytes()
        ))
        .unwrap()
        .1["k"]
            .as_str()
            .unwrap()
            .len(),
        2
    );
    let raw_lower = format!("{:x}\r\n{big}\r\n0\r\n\r\n", big.len());
    assert!(parse_response(&http(
        "HTTP/1.1 200 OK",
        "Transfer-Encoding: chunked\r\n",
        raw_lower.as_bytes()
    ))
    .is_ok());
    // the last chunk with nothing after it is enough
    assert_eq!(
        parse_response(&http(
            "HTTP/1.1 200 OK",
            "Transfer-Encoding: chunked\r\n",
            b"2\r\n{}\r\n0\r\n"
        ))
        .unwrap()
        .1,
        json!({})
    );
}

#[test]
fn a_chunked_body_that_lies_is_refused_and_never_stops_the_program() {
    let header = "Transfer-Encoding: chunked\r\n";
    let cases: &[(&[u8], &str)] = &[
        (b"zz\r\n{}\r\n0\r\n\r\n", "bad size"),
        (b"5\r\n{}\r\n0\r\n\r\n", "bad size"),
        (b"2\r\n{}", "cut"),
        (b"2\r\n{}\r\n", "cut"),
        (b"", "cut"),
        (b"FFFFFFFFFFFFFFFF\r\nabc\r\n", "cut"),
        (b"FFFFFFFFFFFFFFFE\r\nabc\r\n", "cut"),
        (b"7FFFFFFFFFFFFFFF\r\nabc\r\n", "cut"),
        (b"FFFFFFFFFFFFFFFFFFFF\r\nabc", "bad size"),
        (b"-2\r\n{}\r\n0\r\n\r\n", "bad size"),
    ];
    for (body, want) in cases {
        let e = parse_response(&http("HTTP/1.1 200 OK", header, body)).unwrap_err();
        assert!(e.contains(want), "{:?}: {e}", String::from_utf8_lossy(body));
    }
}

#[test]
fn the_error_of_the_server_is_one_line_with_the_code_and_the_detail_when_there_is_one() {
    assert_eq!(
        error_text(
            409,
            &json!({"error": "busy", "detail": "the machine works"})
        ),
        "busy: the machine works"
    );
    assert_eq!(
        error_text(409, &json!({"error": "busy", "detail": ""})),
        "busy"
    );
    assert_eq!(error_text(409, &json!({"error": "busy"})), "busy");
    assert_eq!(
        error_text(500, &json!({"detail": "no code"})),
        "the server answered 500"
    );
    assert_eq!(error_text(502, &Value::Null), "the server answered 502");
    assert_eq!(
        error_text(400, &json!({"error": 5, "detail": "x"})),
        "the server answered 400"
    );
    assert_eq!(
        error_text(400, &json!(["error"])),
        "the server answered 400"
    );
}

#[test]
fn a_name_is_one_segment_of_a_path_whatever_is_in_it() {
    assert_eq!(path_segment(""), "");
    assert_eq!(path_segment("bench-1_a.b~c"), "bench-1_a.b~c");
    assert_eq!(path_segment("../../etc/passwd"), "..%2F..%2Fetc%2Fpasswd");
    assert_eq!(path_segment("a%b"), "a%25b");
    assert_eq!(path_segment("日本"), "%E6%97%A5%E6%9C%AC");
    assert_eq!(path_segment("\0\u{7f}"), "%00%7F");
    for name in [
        "a b", "a/b", "a?b", "a#b", "a\r\nb", "a:b", "ñ", "a\\b", "a;b",
    ] {
        let s = path_segment(name);
        assert!(
            s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~%".contains(&b)),
            "{name}: {s}"
        );
        assert!(
            check_request_part("the path", &format!("/api/v1/{s}")).is_ok(),
            "{name}"
        );
    }
}

#[test]
fn a_part_of_a_request_refuses_everything_that_could_end_its_line() {
    for ok in ["", "/", "/api/v1/fleet?x=1&y=2", "Bearer-abc.DEF-123_~+/="] {
        assert!(check_request_part("p", ok).is_ok(), "{ok:?}");
    }
    for bad in [
        " ", "a b", "a\tb", "a\r", "a\n", "a\0", "a\x7f", "ñ", "日本", "a\u{85}b", "\u{a0}",
    ] {
        let e = check_request_part("the token", bad).unwrap_err();
        assert!(e.starts_with("the token has"), "{bad:?}: {e}");
    }
}

#[test]
fn the_remembered_language_and_window_read_back_for_every_language() {
    for lang in Lang::ALL {
        for fullscreen in [false, true] {
            assert_eq!(
                parse_prefs(&render_prefs(lang, fullscreen)),
                Prefs {
                    lang: Some(lang),
                    fullscreen
                },
                "{lang:?} {fullscreen}"
            );
        }
    }
}

#[test]
fn a_settings_file_written_by_hand_or_by_an_older_version_is_read_with_care() {
    assert_eq!(
        parse_prefs(""),
        Prefs {
            lang: None,
            fullscreen: false
        }
    );
    assert_eq!(
        parse_prefs("de"),
        Prefs {
            lang: Some(Lang::De),
            fullscreen: false
        }
    );
    assert_eq!(
        parse_prefs("de\r\nfullscreen = true\r\n"),
        Prefs {
            lang: Some(Lang::De),
            fullscreen: true
        }
    );
    assert_eq!(parse_prefs("xx\nfullscreen=1").lang, None);
    assert!(parse_prefs("xx\nfullscreen=1").fullscreen);
    for off in [
        "fullscreen=0",
        "fullscreen=yes",
        "fullscreen=",
        "fullscreen",
        "Fullscreen=1",
        "x=fullscreen=1",
    ] {
        assert!(!parse_prefs(&format!("en\n{off}")).fullscreen, "{off}");
    }
    // a line with nothing but the key on the first line is the language, not the window
    assert!(!parse_prefs("fullscreen=1").fullscreen);
    assert!(parse_prefs("en\n\n\nfullscreen=1\n").fullscreen);
}

fn args(list: &[&str]) -> eh3d_drillpro::cli::Options {
    parse_from(
        list.iter().map(|s| s.to_string()),
        "eh3d-test-that-does-not-exist.cfg",
        "eh3d-test-that-does-not-exist.cfg",
    )
}

#[test]
fn the_command_line_reads_its_options_and_keeps_the_defaults_for_what_it_cannot_read() {
    let o = args(&[]);
    assert_eq!(
        (o.wait, o.modern, o.dump, o.machines),
        (8, false, false, false)
    );
    assert!(
        o.file.is_none()
            && o.connection.is_none()
            && o.shot.is_none()
            && o.point.is_none()
            && o.machine.is_none()
    );
    let o = args(&[
        "--lang",
        "de",
        "--data",
        "/d",
        "--connection",
        "m.cfg",
        "--dump",
        "--machines",
        "--machine",
        "bench",
        "--modern",
        "--keys",
        "alt+v,enter",
        "--shot",
        "s.png",
        "--wait",
        "3",
        "job.ncd",
    ]);
    assert_eq!(o.lang, Lang::De);
    assert_eq!(o.data, PathBuf::from("/d"));
    assert_eq!(o.connection, Some(PathBuf::from("m.cfg")));
    assert!(o.dump && o.machines && o.modern);
    assert_eq!(
        (
            o.machine.as_deref(),
            o.keys.as_str(),
            o.shot,
            o.wait,
            o.file
        ),
        (
            Some("bench"),
            "alt+v,enter",
            Some(PathBuf::from("s.png")),
            3,
            Some(PathBuf::from("job.ncd"))
        )
    );
    let o = args(&[
        "--lang",
        "xx",
        "--wait",
        "many",
        "--wait",
        "-1",
        "--modern",
        "--classic",
    ]);
    assert_eq!((o.lang, o.wait, o.modern), (Lang::En, 8, false));
}

#[test]
fn an_option_without_its_value_at_the_end_does_not_break_the_reading() {
    for last in [
        "--lang",
        "--data",
        "--connection",
        "--machine",
        "--keys",
        "--shot",
        "--point",
        "--wait",
    ] {
        let o = args(&["file.ncd", last]);
        assert_eq!(o.file, Some(PathBuf::from("file.ncd")), "{last}");
    }
}

#[test]
fn the_pointer_of_the_pictures_is_a_column_a_row_and_optionally_down() {
    assert_eq!(args(&["--point", "3,4"]).point, Some((3, 4, false)));
    assert_eq!(args(&["--point", " 3 , 4 ,down"]).point, Some((3, 4, true)));
    assert_eq!(args(&["--point", "3,4,up"]).point, Some((3, 4, false)));
    assert_eq!(args(&["--point", "0,0,down"]).point, Some((0, 0, true)));
    for bad in ["", "3", "a,b", "3,", ",4", "-1,2", "3;4"] {
        assert_eq!(args(&["--point", bad]).point, None, "{bad:?}");
    }
}

#[test]
fn the_last_file_on_the_command_line_is_the_one_that_is_opened() {
    assert_eq!(args(&["a.ncd", "b.ncd"]).file, Some(PathBuf::from("b.ncd")));
}
