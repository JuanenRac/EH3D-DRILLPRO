//! The Offset button of the classic screens: the question, the Offset Teach window and the Teach-In mode.

use std::path::{Path, PathBuf};

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::engine::ClassicEngine;
use eh3d_drillpro::classic::ClassicApp;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::screen::Screen;
use eh3d_drillpro::textapp::TextApp;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d-teach-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    for f in ["DRILLPRO.DCF", "DRILLPRO.DTD"] {
        std::fs::copy(testdata.join(f), dir.join(f)).unwrap();
    }
    dir
}

fn text(a: &ClassicApp) -> String {
    let mut s = Screen::default();
    a.render(&mut s);
    s.to_text()
}

fn press(a: &mut ClassicApp, keys: &str) {
    for c in keys.chars() {
        a.handle(Input::Char(c));
    }
}

fn app(tag: &str) -> ClassicApp {
    ClassicApp::new(Lang::En, &scratch(tag))
}

#[test]
fn ok_makes_the_position_reached_the_offset_of_the_form() {
    let mut a = app("ok");
    // No keeps the old offset (2.5 mm, 98 mil); 2 x 1000 mil forward and 1000 to the left; the same figures as the original
    press(&mut a, "on");
    assert!(text(&a).contains("X: 002.49"), "{}", text(&a));
    press(&mut a, "dde");
    press(&mut a, "o");
    let t = text(&a);
    assert!(t.contains("+053.29") && t.contains("+027.89"), "{t}");
}

#[test]
fn yes_deletes_the_old_offset_and_starts_at_home() {
    let mut a = app("yes");
    press(&mut a, "oy");
    let t = text(&a);
    assert!(t.contains("X: 000.00") && t.contains("Y: 000.00"), "{t}");
    assert!(t.contains("(Z:  37.30)"), "{t}");
    press(&mut a, "d");
    assert!(text(&a).contains("X: 025.40"), "{}", text(&a));
}

#[test]
fn the_steps_and_the_directions() {
    let mut a = app("steps");
    press(&mut a, "oy");
    // 10 mil, 100 mil, 1 mil in X; then back with S; the arrows and the page keys do the same as the letters
    press(&mut a, "2d3d1d");
    assert!(text(&a).contains("X: 002.82"), "{}", text(&a)); // 111 mil
    press(&mut a, "4s");
    a.handle(Input::Right);
    a.handle(Input::Up);
    assert!(text(&a).contains("Y: 025.40"), "{}", text(&a));
    // Z goes down with C and up with R, never above the top
    press(&mut a, "4c");
    assert!(text(&a).contains("(Z:  11.90)"), "{}", text(&a));
    press(&mut a, "rr");
    assert!(text(&a).contains("(Z:  37.30)"), "{}", text(&a));
}

#[test]
fn abort_leaves_the_offset_as_it_was() {
    let mut a = app("abort");
    press(&mut a, "ony");
    let _ = a;
    let mut a = app("abort2");
    press(&mut a, "ondd");
    press(&mut a, "a");
    let t = text(&a);
    assert!(t.contains("+002.50"), "{t}");
    assert!(!t.contains("Offset Teach"), "{t}");
}

#[test]
fn teach_in_records_positions_and_writes_a_file_the_program_reads() {
    let dir = scratch("teachin");
    let mut a = ClassicApp::new(Lang::En, &dir);
    press(&mut a, "oy");
    press(&mut a, "t"); // Teach: here is the origin
    let t = text(&a);
    assert!(t.contains("N:  0000") && t.contains("X: 000.00"), "{t}");
    // Abort is not available while teaching
    press(&mut a, "a");
    assert!(text(&a).contains("Offset Teach"));
    // two holes: 1000 mil to the right and 500 up
    press(&mut a, "d+");
    press(&mut a, "3e+");
    let t = text(&a);
    assert!(t.contains("N:  0002") || t.contains("N:   0002"), "{t}");
    assert!(t.contains("00100"), "{t}"); // Y of the last hole: 100 mil with the step 100
                                         // - Pos takes the last one back, Mod puts the last one where the head is
    press(&mut a, "-");
    assert!(text(&a).contains("0001"));
    press(&mut a, "e");
    press(&mut a, "m");
    // Ok asks for the file
    press(&mut a, "o");
    let t = text(&a);
    assert!(t.contains("Write Teach-In") && t.contains("Info:"), "{t}");
    press(&mut a, "y");
    let written = std::fs::read_to_string(a.engine.dir().join("TEACH.NCD")).expect("the file");
    assert!(written.contains("BUNGARD CCD TEACH-IN FILE"), "{written}");
    assert!(
        written.contains("T01") && written.contains("M30"),
        "{written}"
    );
    assert!(text(&a).contains("TEACH.NCD"));
}

#[test]
fn the_mouse_reaches_the_buttons_and_the_steps() {
    let mut a = app("mouse");
    press(&mut a, "oy");
    // radio button "100" (row 18), then D (6) forw. (button at column 38, row 10)
    a.click(14, 18);
    a.click(40, 10);
    assert!(text(&a).contains("X: 002.54"), "{}", text(&a));
    // Teach (column 60, row 15) and Ok (column 60, row 19) asks for the file; No leaves
    a.click(60, 15);
    assert!(text(&a).contains("N:"), "{}", text(&a));
    a.click(60, 19);
    assert!(text(&a).contains("Write Teach-In"), "{}", text(&a));
    a.click(46, 14);
    assert!(!text(&a).contains("Offset Teach"), "{}", text(&a));
}
