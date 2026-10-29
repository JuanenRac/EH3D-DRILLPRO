//! The console front end: keys, the mouse, the picture, and the command line shared with the window.

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::ClassicApp;
use eh3d_drillpro::cli::{parse_from, start_classic, Options};
use eh3d_drillpro::console::{ansi_frame, cell_at, map_key, origin_for};
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::screen::{Screen, COLS, ROWS};
use eh3d_drillpro::textapp::TextApp;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

#[test]
fn keys_become_the_inputs_of_the_programs() {
    let none = KeyModifiers::NONE;
    assert_eq!(
        map_key(key(KeyCode::Char('a'), none)),
        Some(Input::Char('a'))
    );
    assert_eq!(
        map_key(key(KeyCode::Char('V'), KeyModifiers::SHIFT)),
        Some(Input::Char('V'))
    );
    assert_eq!(
        map_key(key(KeyCode::Char('V'), KeyModifiers::ALT)),
        Some(Input::Alt('v'))
    );
    assert_eq!(map_key(key(KeyCode::Enter, none)), Some(Input::Enter));
    assert_eq!(map_key(key(KeyCode::Esc, none)), Some(Input::Esc));
    assert_eq!(map_key(key(KeyCode::Tab, none)), Some(Input::Tab));
    assert_eq!(
        map_key(key(KeyCode::BackTab, KeyModifiers::SHIFT)),
        Some(Input::BackTab)
    );
    assert_eq!(
        map_key(key(KeyCode::Backspace, none)),
        Some(Input::Backspace)
    );
    assert_eq!(map_key(key(KeyCode::Delete, none)), Some(Input::Delete));
    assert_eq!(map_key(key(KeyCode::Left, none)), Some(Input::Left));
    assert_eq!(map_key(key(KeyCode::Home, none)), Some(Input::Home));
    assert_eq!(map_key(key(KeyCode::PageDown, none)), Some(Input::PageDown));
    assert_eq!(map_key(key(KeyCode::F(5), none)), None);
    // Windows also reports the release of a key: it must not count twice
    let mut release = key(KeyCode::Char('a'), none);
    release.kind = KeyEventKind::Release;
    assert_eq!(map_key(release), None);
}

#[test]
fn a_bigger_terminal_shows_the_screen_in_the_middle_and_clicks_follow() {
    assert_eq!(origin_for((80, 25)), (1, 1));
    assert_eq!(origin_for((120, 40)), (21, 8));
    let origin = origin_for((120, 40));
    // the first cell of the screen sits at terminal column 20, row 7 (counted from 0)
    assert_eq!(cell_at(20, 7, origin), Some((0, 0)));
    assert_eq!(cell_at(99, 31, origin), Some((79, 24)));
    // outside the screen: nothing
    assert_eq!(cell_at(19, 7, origin), None);
    assert_eq!(cell_at(100, 7, origin), None);
    assert_eq!(cell_at(20, 32, origin), None);
}

#[test]
fn the_picture_has_every_row_placed_and_the_colours_of_the_cells() {
    let mut app = ClassicApp::new(
        Lang::En,
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata"),
    );
    let mut s = Screen::default();
    app.render(&mut s);
    let frame = ansi_frame(&s, (1, 1));
    // one cursor move per row, and the box characters are in it
    assert!(frame.matches("\x1b[").count() > ROWS);
    for y in 1..=ROWS {
        assert!(frame.contains(&format!("\x1b[{y};1H")), "row {y}");
    }
    assert!(frame.contains('\u{2554}') && frame.contains('\u{2591}'));
    assert!(frame.ends_with("\x1b[0m"));
    // a wide character takes two cells but is drawn once: the frame has no more characters than cells
    let text: String = frame
        .split("\x1b[")
        .map(|p| {
            p.split_once('m')
                .or_else(|| p.split_once('H'))
                .map_or("", |(_, t)| t)
        })
        .collect();
    assert!(
        text.chars().count() <= COLS * ROWS,
        "{}",
        text.chars().count()
    );
    // the same picture is drawn when asked twice, and a click changes it
    assert_eq!(frame, ansi_frame(&s, (1, 1)));
    app.click(66, 19);
    let mut s2 = Screen::default();
    app.render(&mut s2);
    assert_ne!(frame, ansi_frame(&s2, (1, 1)));
}

#[test]
fn the_command_line_is_the_same_for_the_window_and_the_console() {
    let o: Options = parse_from(
        [
            "T1.NCD",
            "--lang",
            "de",
            "--data",
            "somewhere",
            "--modern",
            "--dump",
            "--wait",
            "3",
        ]
        .map(String::from),
        "eh3d-test.lang",
        "DRILLPRO.DCF",
    );
    assert_eq!(o.lang, Lang::De);
    assert_eq!(o.data, std::path::PathBuf::from("somewhere"));
    assert_eq!(o.file, Some(std::path::PathBuf::from("T1.NCD")));
    assert!(o.modern && o.dump);
    assert_eq!(o.wait, 3);
    let o = parse_from(
        ["--modern", "--classic"].map(String::from),
        "eh3d-test.lang",
        "DRILLPRO.DCF",
    );
    assert!(!o.modern, "the last word wins");
}

#[test]
fn start_classic_loads_the_file_the_command_line_names() {
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let o = parse_from(
        [
            data.join("T1.NCD").to_string_lossy().into_owned(),
            "--data".into(),
            data.to_string_lossy().into_owned(),
        ],
        "eh3d-test.lang",
        "DRILLPRO.DCF",
    );
    let app = start_classic(eh3d_drillpro::app::App::new(o.lang, &o.data), &o);
    let mut s = Screen::default();
    app.render(&mut s);
    assert!(s.to_text().contains("T1.NCD"));
}
