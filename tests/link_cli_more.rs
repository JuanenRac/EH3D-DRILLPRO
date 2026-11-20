//! The answers of the server as the program tells them, the names that go into a request and the command line, from their edges: names that try to split a request, and options that are
//! written wrong.

use eh3d_drillpro::cli::{parse_from, parse_prefs, render_prefs, Prefs};
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::server_link::{check_request_part, error_text, path_segment};
use serde_json::{json, Value};
use std::path::PathBuf;

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

#[test]
fn the_server_status_is_an_option_that_is_off_unless_it_is_given() {
    assert!(!args(&[]).server_status);
    assert!(args(&["--server-status"]).server_status);
    // it does not take the next word as a value, so a file after it is still the file
    let o = args(&["--server-status", "job.ncd"]);
    assert!(o.server_status);
    assert_eq!(o.file, Some(PathBuf::from("job.ncd")));
}
