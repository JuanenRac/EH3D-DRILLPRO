//! Editing, mouse and the real files in the classic front end.

use std::path::{Path, PathBuf};

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::edit::{glob_matches, Kind, LineEdit};
use eh3d_drillpro::classic::engine::ClassicEngine;
use eh3d_drillpro::classic::ClassicApp;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::screen::Screen;
use eh3d_drillpro::textapp::TextApp;

fn testdata() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// A scratch copy of the test data, so the tests can write files.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d-drillpro-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["DRILLPRO.DCF", "DRILLPRO.DTD", "T1.NCD", "100THOU.NCD"] {
        std::fs::copy(testdata().join(f), dir.join(f)).unwrap();
    }
    dir
}

fn app_in(dir: &Path) -> ClassicApp {
    ClassicApp::new(Lang::En, dir)
}

fn press(a: &mut ClassicApp, keys: &str) {
    for c in keys.chars() {
        a.handle(Input::Char(c));
    }
}

fn text(a: &ClassicApp) -> String {
    let mut s = Screen::default();
    a.render(&mut s);
    s.to_text()
}

/// Clicks the middle of the first cell where `needle` is written.
fn click_on(a: &mut ClassicApp, needle: &str) {
    let t = text(a);
    for (y, line) in t.lines().enumerate() {
        if let Some(x) = line.find(needle) {
            let col = line[..x].chars().count();
            a.click(col, y);
            return;
        }
    }
    panic!("{needle:?} is not on the screen:\n{t}");
}

#[test]
fn a_line_edit_takes_typing_backspace_and_the_cursor_keys() {
    let mut e = LineEdit::new("1.0000", Kind::Number, 6);
    // the first key replaces the selected text
    assert!(e.key(Input::Char('2')));
    assert_eq!(e.text, "2");
    for c in "5.5".chars() {
        e.key(Input::Char(c));
    }
    assert_eq!(e.text, "25.5");
    e.key(Input::Backspace);
    assert_eq!(e.text, "25.");
    e.key(Input::Left);
    e.key(Input::Left);
    e.key(Input::Char('7'));
    assert_eq!(e.text, "275.");
    e.key(Input::Home);
    e.key(Input::Delete);
    assert_eq!(e.text, "75.");
    // letters are not numbers
    assert!(!e.key(Input::Char('x')));
    // the length is limited
    for _ in 0..10 {
        e.key(Input::Char('1'));
    }
    assert_eq!(e.text.chars().count(), 6);
}

#[test]
fn file_name_patterns_have_stars_and_question_marks_and_no_case() {
    assert!(glob_matches("*.NCD", "t1.ncd"));
    assert!(glob_matches("T?.ncd", "T1.NCD"));
    assert!(!glob_matches("*.NCD", "T1.PLT"));
    assert!(glob_matches("*", "anything"));
}

#[test]
fn a_field_is_edited_with_backspace_and_the_mouse() {
    let dir = scratch("form");
    let mut a = app_in(&dir);
    // click the offset X field, retype it with a correction
    a.click(24, 10);
    press(&mut a, "3.25");
    a.handle(Input::Backspace);
    press(&mut a, "7");
    a.handle(Input::Enter);
    assert!(text(&a).contains("+003.27"), "{}", text(&a));
    // clicking another field while one is being edited takes the first one
    a.click(14, 10);
    press(&mut a, "1.5");
    a.click(35, 10);
    let t = text(&a);
    assert!(t.contains("1.5000"), "{t}");
}

#[test]
fn tab_walks_the_fields_and_enter_leaves_one() {
    let dir = scratch("tab");
    let mut a = app_in(&dir);
    a.handle(Input::Tab);
    press(&mut a, "2");
    a.handle(Input::Tab);
    press(&mut a, "3");
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(t.contains("2.0000") && t.contains("3.0000"), "{t}");
}

#[test]
fn the_buttons_answer_to_the_mouse() {
    let dir = scratch("mouse");
    let mut a = app_in(&dir);
    click_on(&mut a, "Tools");
    assert!(text(&a).contains("Tool Data"));
    // Abort closes it
    click_on(&mut a, "Abort");
    assert!(!text(&a).contains("Tool Data"));
    click_on(&mut a, "About");
    let t = text(&a);
    assert!(
        t.contains("EH3D-DRILLPRO") && t.contains("GPL-3.0-or-later"),
        "{t}"
    );
    assert!(t.contains("Bungard"), "{t}");
    assert!(
        t.contains("CCD/2") && t.contains("MTC") && t.contains("ATC"),
        "{t}"
    );
    click_on(&mut a, "Ok");
    assert!(text(&a).contains("DrillPro"));
}

#[test]
fn the_about_dialog_names_the_machines_in_every_language() {
    for lang in Lang::ALL {
        let mut a = ClassicApp::new(lang, &testdata());
        // the hot letter of About differs by language: use the mouse
        a.click(66, 19);
        let t = text(&a);
        assert!(t.contains("EH3D-DRILLPRO"), "{lang:?}:\n{t}");
        assert!(t.contains("CCD/2"), "{lang:?}");
        assert!(t.contains("MTC") && t.contains("ATC"), "{lang:?}");
    }
}

#[test]
fn the_file_list_is_filtered_scrolled_and_opened_with_the_mouse() {
    let dir = scratch("files");
    let mut a = app_in(&dir);
    press(&mut a, "l");
    let t = text(&a);
    assert!(t.contains("T1.NCD") && t.contains("100THOU.NCD"), "{t}");
    // a pattern in the name field filters the list
    press(&mut a, "t*.ncd");
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(t.contains("T1.NCD") && !t.contains("100THOU.NCD"), "{t}");
    // two clicks on an entry open it, and the tool data follow
    click_on(&mut a, "T1.NCD");
    click_on(&mut a, "T1.NCD");
    assert!(text(&a).contains("Tool Data"), "{}", text(&a));
    click_on(&mut a, "Ok");
    let t = text(&a);
    assert!(t.contains("T1.NCD") && t.contains("00003"), "{t}");
}

#[test]
fn an_unknown_file_name_says_so() {
    let dir = scratch("nofile");
    let mut a = app_in(&dir);
    press(&mut a, "l");
    press(&mut a, "nothere.ncd");
    a.handle(Input::Enter);
    assert!(text(&a).contains("File not found"), "{}", text(&a));
}

#[test]
fn the_language_is_changed_from_the_config_dialog() {
    let dir = scratch("lang");
    let mut a = app_in(&dir);
    press(&mut a, "c");
    assert!(text(&a).contains("English"));
    // one click on the field takes the next language, and the screen follows at once
    a.click(67, 17);
    assert_eq!(a.engine.lang, Lang::Es);
    assert!(text(&a).contains("Español"), "{}", text(&a));
    // the arrows at the ends of the field go both ways
    a.click(73, 17);
    assert_eq!(a.engine.lang, Lang::It);
    a.click(63, 17);
    a.click(63, 17);
    assert_eq!(a.engine.lang, Lang::En);
    // and the keyboard: the hot letter of the language, then the arrow keys
    press(&mut a, "u");
    a.handle(Input::Right);
    assert_eq!(a.engine.lang, Lang::Es);
    a.handle(Input::Left);
    assert_eq!(a.engine.lang, Lang::En);
    // every language can be reached in turn
    for _ in 0..Lang::ALL.len() {
        a.click(67, 17);
    }
    assert_eq!(a.engine.lang, Lang::En);
}

#[test]
fn parameters_are_edited_saved_to_the_real_file_and_read_back() {
    let dir = scratch("params");
    let mut a = app_in(&dir);
    press(&mut a, "c");
    // the XY maximum speed: click it, type, accept; the sorting radio by mouse
    a.click(23, 11);
    press(&mut a, "3500");
    a.handle(Input::Enter);
    click_on(&mut a, "X first");
    // out of range is refused with the limits
    a.click(23, 12);
    press(&mut a, "99999");
    a.handle(Input::Enter);
    assert!(text(&a).contains("Input error!"), "{}", text(&a));
    a.handle(Input::Esc);
    click_on(&mut a, "Save");
    assert!(text(&a).contains("Saved"), "{}", text(&a));
    // the file has the new values and a backup of the old one
    let written: String = std::fs::read(dir.join("DRILLPRO.DCF"))
        .unwrap()
        .iter()
        .map(|&c| c as char)
        .collect();
    let cfg = eh3d_drillpro::config::parse(&written).unwrap();
    let s = cfg.settings().unwrap();
    assert_eq!(s.speed_xy.0, 3500);
    assert_eq!(s.sorting, eh3d_drillpro::config::SortMode::XFirst);
    assert!(dir.join("DRILLPRO.DCF.BAK").exists());
    assert!(
        written.contains("* Scale X (0.0000 ... 10.000)"),
        "comments are kept"
    );
    // a new program reads the same values
    let b = app_in(&dir);
    assert_eq!(b.engine.settings().unwrap().speed_xy.0, 3500);
}

#[test]
fn the_main_form_is_saved_when_leaving() {
    let dir = scratch("leave");
    let mut a = app_in(&dir);
    a.click(14, 10);
    press(&mut a, "1.5");
    a.handle(Input::Enter);
    // Enter went on to the next field: Esc leaves it
    a.handle(Input::Esc);
    // Quit asks, then offers to save the changed settings
    press(&mut a, "q");
    press(&mut a, "y");
    assert!(!a.wants_quit());
    assert!(text(&a).contains("Save settings"), "{}", text(&a));
    press(&mut a, "y");
    assert!(a.wants_quit());
    let written: String = std::fs::read(dir.join("DRILLPRO.DCF"))
        .unwrap()
        .iter()
        .map(|&c| c as char)
        .collect();
    assert!(written.contains("1.5000"), "{written}");
}

#[test]
fn tool_data_is_edited_and_saved_to_the_tool_table() {
    let dir = scratch("tools");
    let mut a = app_in(&dir);
    a.load_file(&dir.join("T1.NCD"));
    press(&mut a, "t");
    // the feed of the first place: click it, retype
    a.click(33, 4);
    press(&mut a, "1234");
    a.handle(Input::Enter);
    click_on(&mut a, "Save");
    assert!(text(&a).contains("Saved"), "{}", text(&a));
    let table: String = std::fs::read(dir.join("DRILLPRO.DTD"))
        .unwrap()
        .iter()
        .map(|&c| c as char)
        .collect();
    let tools = eh3d_drillpro::tools::parse(&table);
    let t1 = tools
        .iter()
        .find(|t| t.number == 1)
        .expect("tool 01 was added");
    assert_eq!(t1.feed_mm_min, 1234);
    // the other tools of the table are still there
    assert!(tools.iter().any(|t| t.number == 4));
}

#[test]
fn the_dialogs_leave_with_escape_and_the_keyboard_alone() {
    let dir = scratch("keys");
    let mut a = app_in(&dir);
    // Panelization needs a file
    a.load_file(&dir.join("T1.NCD"));
    for keys in ["c", "t", "l", "p"] {
        press(&mut a, keys);
        a.handle(Input::Esc);
        assert!(
            text(&a).contains("DrillPro"),
            "{keys}: back on the main screen"
        );
    }
}
