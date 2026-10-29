//! A job through the classic front end: the operator's dialogs, the window and the stop.

use std::path::{Path, PathBuf};

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::engine::{ClassicEngine, Question};
use eh3d_drillpro::classic::ClassicApp;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::screen::Screen;
use eh3d_drillpro::textapp::TextApp;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d-job-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    for f in ["DRILLPRO.DCF", "DRILLPRO.DTD"] {
        std::fs::copy(testdata.join(f), dir.join(f)).unwrap();
    }
    std::fs::write(
        dir.join("TWO.NCD"),
        "%\nT04\nX1000Y1000\nX2000Y1000\nT06\nX1000Y2000\nX2000Y2000\nM30\n",
    )
    .unwrap();
    dir
}

fn text(a: &ClassicApp) -> String {
    let mut s = Screen::default();
    a.render(&mut s);
    s.to_text()
}

/// Ticks until the job asks something, or `max` ticks went by.
fn until_question(a: &mut ClassicApp, max: usize) -> Option<Question> {
    for _ in 0..max {
        if let Some(q) = a.engine.question() {
            return Some(q);
        }
        a.tick();
    }
    a.engine.question()
}

/// Where the Ok and Abort buttons of a question are (column, row).
fn buttons(q: Question) -> ((usize, usize), (usize, usize)) {
    if matches!(q, Question::ToolChange(_)) {
        ((28, 17), (44, 17))
    } else {
        ((31, 14), (44, 14))
    }
}

#[test]
fn the_job_stops_at_the_spindle_and_at_each_tool_change_until_the_operator_answers() {
    let dir = scratch("t1");
    let mut a = ClassicApp::new(Lang::En, &dir);
    a.load_file(&dir.join("TWO.NCD"));
    a.handle(Input::Char('s'));
    let q = until_question(&mut a, 400).expect("a question");
    let t = text(&a);
    match q {
        Question::ToolChange(n) => {
            assert!(
                t.contains("Tool Change") && t.contains(&format!("Insert tool: {n:02}")),
                "{t}"
            );
            assert!(
                t.contains("Run length")
                    && t.contains("Spindle")
                    && t.contains("Z Speed")
                    && t.contains("Stroke"),
                "{t}"
            );
        }
        Question::SpindleStart => {
            assert!(
                t.contains("Attention!") && t.contains("start the spindle"),
                "{t}"
            );
        }
    }
    // while it waits, nothing advances
    let done = a.engine.done();
    for _ in 0..20 {
        a.tick();
    }
    assert_eq!(a.engine.done(), done);
    assert_eq!(a.engine.question(), Some(q));
    // Enter goes on; the job reaches the next questions, and finally ends
    let mut seen = vec![q];
    a.handle(Input::Enter);
    for _ in 0..12 {
        match until_question(&mut a, 600) {
            Some(q) => {
                seen.push(q);
                a.handle(Input::Enter);
            }
            None => break,
        }
    }
    assert!(seen.contains(&Question::SpindleStart), "{seen:?}");
    assert!(
        seen.contains(&Question::ToolChange(6)),
        "the second tool is asked for: {seen:?}"
    );
    for _ in 0..3000 {
        if !a.tick() && a.engine.question().is_none() {
            break;
        }
    }
    assert!(!text(&a).contains("Attention!"));
}

#[test]
fn abort_at_a_question_stops_the_job_and_leaves_the_window() {
    let dir = scratch("t2");
    let mut a = ClassicApp::new(Lang::En, &dir);
    a.load_file(&dir.join("TWO.NCD"));
    a.handle(Input::Char('s'));
    assert!(until_question(&mut a, 400).is_some());
    a.handle(Input::Esc);
    assert!(a.engine.question().is_none());
    let t = text(&a);
    assert!(!t.contains("Press ESC to stop"), "{t}");
    // the form is back
    assert!(t.contains("Scale"), "{t}");
}

#[test]
fn the_question_buttons_answer_to_the_mouse() {
    let dir = scratch("t3");
    let mut a = ClassicApp::new(Lang::En, &dir);
    a.load_file(&dir.join("TWO.NCD"));
    a.handle(Input::Char('s'));
    let q = until_question(&mut a, 400).unwrap();
    let (ok, _) = buttons(q);
    a.click(ok.0, ok.1);
    assert_ne!(a.engine.question(), Some(q), "Ok answered the question");
    if let Some(q2) = until_question(&mut a, 600) {
        let (_, abort) = buttons(q2);
        a.click(abort.0, abort.1);
        assert!(a.engine.question().is_none(), "Abort stopped the job");
    }
}

#[test]
fn the_window_shows_the_planned_figures_while_drilling() {
    let dir = scratch("t4");
    let mut a = ClassicApp::new(Lang::En, &dir);
    a.load_file(&dir.join("TWO.NCD"));
    a.handle(Input::Char('s'));
    for _ in 0..6000 {
        if a.engine.question().is_some() {
            a.handle(Input::Enter);
        }
        a.tick();
        if a.engine.done() >= 2 {
            break;
        }
    }
    let v = a.engine.job_view().expect("running");
    assert!(v.done >= 2);
    assert!(v.percent > 0 && v.percent <= 100);
    assert!(v.used_mm > 0.0);
    assert!(v.rpm > 0);
    let t = text(&a);
    assert!(t.contains("Drilling") && t.contains("TWO.NCD"), "{t}");
}
