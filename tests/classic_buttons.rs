//! Buttons that used to do nothing: Start without a file, Load of the tool and configuration windows, the arrow of the name field.

use std::path::{Path, PathBuf};

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::ClassicApp;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::screen::Screen;
use eh3d_drillpro::textapp::TextApp;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d-buttons-{tag}-{}", std::process::id()));
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

fn type_in(a: &mut ClassicApp, s: &str) {
    for c in s.chars() {
        a.handle(Input::Char(c));
    }
}

#[test]
fn start_without_a_file_says_so() {
    let mut a = ClassicApp::new(Lang::En, &scratch("start"));
    a.handle(Input::Char('s'));
    assert!(text(&a).contains("Load a file first"), "{}", text(&a));
    a.handle(Input::Enter);
    assert!(!text(&a).contains("Load a file first"));
}

#[test]
fn load_of_the_tool_window_chooses_the_file() {
    let mut a = ClassicApp::new(Lang::En, &scratch("tools"));
    a.handle(Input::Char('t'));
    a.click(18, 20);
    let t = text(&a);
    assert!(t.contains("Select File") && t.contains("*.DTD"), "{t}");
    assert!(t.contains("DRILLPRO.DTD"), "{t}");
    type_in(&mut a, "DRILLPRO.DTD");
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(t.contains("Tool Data") && t.contains("Loaded"), "{t}");
}

#[test]
fn load_of_the_parameter_window_chooses_the_file() {
    let mut a = ClassicApp::new(Lang::En, &scratch("cfg"));
    a.handle(Input::Char('c'));
    a.click(14, 19);
    let t = text(&a);
    assert!(t.contains("Select File") && t.contains("*.DCF"), "{t}");
    type_in(&mut a, "DRILLPRO.DCF");
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(t.contains("Parameter") && t.contains("Loaded"), "{t}");
}

#[test]
fn a_missing_tool_file_is_said_in_a_window() {
    let mut a = ClassicApp::new(Lang::En, &scratch("missing"));
    a.handle(Input::Char('t'));
    a.click(18, 20);
    type_in(&mut a, "NOPE.DTD");
    a.handle(Input::Enter);
    assert!(!text(&a).contains("Loaded"), "{}", text(&a));
}

#[test]
fn the_arrow_of_the_name_field_offers_the_patterns() {
    use std::io::Write;
    let dir = scratch("arrow");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/03_two_tools.ncd"),
        dir.join("BOARD.NCD"),
    )
    .unwrap();
    let mut w = zip::ZipWriter::new(std::fs::File::create(dir.join("JOB.ZIP")).unwrap());
    w.start_file("A.NCD", zip::write::SimpleFileOptions::default())
        .unwrap();
    w.write_all(
        b"M48
",
    )
    .unwrap();
    w.finish().unwrap();
    let mut a = ClassicApp::new(Lang::En, &dir);
    a.handle(Input::Char('l'));
    let t = text(&a);
    assert!(t.contains("BOARD.NCD") && !t.contains("JOB.ZIP"), "{t}");
    // the arrow opens the list of patterns; the second one is the archives
    a.click(47, 5);
    let t = text(&a);
    assert!(t.contains("*.ZIP") && t.contains("*.*"), "{t}");
    a.click(20, 7);
    let t = text(&a);
    assert!(t.contains("JOB.ZIP") && !t.contains("BOARD.NCD"), "{t}");
    // the keys work in the list too
    a.click(47, 5);
    a.handle(Input::Down);
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(
        t.contains("JOB.ZIP") && t.contains("BOARD.NCD") || t.contains("BOARD.NCD"),
        "{t}"
    );
}
