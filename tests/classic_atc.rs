//! The classic screens on a machine with the automatic tool changer (the ATC models): the AutoChg box and a job that changes tools.

use std::path::{Path, PathBuf};

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::engine::ClassicEngine;
use eh3d_drillpro::classic::ClassicApp;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::screen::Screen;
use eh3d_drillpro::textapp::TextApp;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d-atc-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    std::fs::copy(testdata.join("DRILLPRO_CNCA.DCF"), dir.join("DRILLPRO.DCF")).unwrap();
    std::fs::copy(testdata.join("DRILLPRO.DTD"), dir.join("DRILLPRO.DTD")).unwrap();
    std::fs::write(
        dir.join("THREE.NCD"),
        "%\nT01\nX500Y500\nX3000Y500\nT02\nX1000Y2500\nX2000Y1500\nT03\nX2500Y3000\nM30\n",
    )
    .unwrap();
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

#[test]
fn the_autochg_box_is_on_the_machines_with_a_changer_and_is_saved() {
    let dir = scratch("box");
    let mut a = ClassicApp::new(Lang::En, &dir);
    press(&mut a, "c");
    assert!(text(&a).contains("[X] AutoChg"), "{}", text(&a));
    // the hot letter U switches it off; Ok writes it into the configuration file
    press(&mut a, "u");
    assert!(text(&a).contains("[ ] AutoChg"), "{}", text(&a));
    press(&mut a, "o");
    a.handle(Input::Enter);
    // the setting is in the program's settings; the file is written when the program saves it
    assert!(!a.engine.settings().unwrap().auto_change);
}

#[test]
fn the_manual_machines_have_no_autochg_box() {
    let dir = std::env::temp_dir().join(format!("eh3d-atc-manual-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    for f in ["DRILLPRO.DCF", "DRILLPRO.DTD"] {
        std::fs::copy(testdata.join(f), dir.join(f)).unwrap();
    }
    let mut a = ClassicApp::new(Lang::En, &dir);
    press(&mut a, "c");
    assert!(!text(&a).contains("AutoChg"));
}

#[test]
fn a_job_with_three_tools_runs_to_the_end_changing_them_by_itself() {
    let dir = scratch("job");
    let mut a = ClassicApp::new(Lang::En, &dir);
    a.load_file(&dir.join("THREE.NCD"));
    a.handle(Input::Char('s'));
    let mut questions = Vec::new();
    for _ in 0..20_000 {
        if let Some(q) = a.engine.question() {
            questions.push(format!("{q:?}"));
            a.handle(Input::Enter);
        }
        if !a.tick() {
            break;
        }
    }
    assert_eq!(a.engine.done(), 5, "every hole is drilled");
    // the machine changes the tools itself: the only question is whether the spindle may start, once
    assert!(
        questions.iter().all(|q| q.contains("SpindleStart")),
        "{questions:?}"
    );
    assert_eq!(questions.len(), 1, "{questions:?}");
}
