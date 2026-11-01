//! A job is not started on made-up data: a tool that the table does not give, a tool table line that cannot be read, and a file with slots.

use std::path::{Path, PathBuf};

use eh3d_drillpro::app::App;
use eh3d_drillpro::classic::engine::ClassicEngine;
use eh3d_drillpro::i18n::Lang;

fn scratch(tag: &str, table: &str, file: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d-safety-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    std::fs::copy(testdata.join("DRILLPRO.DCF"), dir.join("DRILLPRO.DCF")).unwrap();
    std::fs::write(
        dir.join("DRILLPRO.DTD"),
        format!("{}{table}", ";\r\n".repeat(19)),
    )
    .unwrap();
    std::fs::write(dir.join("JOB.NCD"), file).unwrap();
    dir
}

const THREE: &str = "%\nT01\nX500Y500\nT02\nX1000Y2500\nT03\nX2500Y3000\nM30\n";

#[test]
fn a_tool_whose_line_cannot_be_read_runs_with_the_cautious_values_and_the_person_is_told_which() {
    let dir = scratch(
        "tools",
        "01 0.5 1200 24000
02 0,5 1200 24000
03 0.5 abc 24000
",
        THREE,
    );
    let mut app = App::new(Lang::En, &dir);
    app.load_file(&dir.join("JOB.NCD"));
    ClassicEngine::start(&mut app);
    // the job is not refused
    assert!(ClassicEngine::error(&app).is_none());
    let warning = ClassicEngine::warning(&app)
        .expect("the person is told")
        .to_string();
    // the decimal comma of tool 2 is read, so only tool 3 is named, with the line that could not be read
    assert!(
        warning.contains("line 22") && warning.contains("03 0.5 abc 24000"),
        "{warning}"
    );
    assert!(!warning.contains("02 0,5"), "{warning}");
}

#[test]
fn a_tool_with_no_line_at_all_is_named_too_and_a_complete_table_says_nothing() {
    let dir = scratch(
        "missing",
        "01 0.5 1200 24000
03 0.5 900 24000
",
        THREE,
    );
    let mut app = App::new(Lang::En, &dir);
    app.load_file(&dir.join("JOB.NCD"));
    ClassicEngine::start(&mut app);
    let warning = ClassicEngine::warning(&app)
        .expect("tool 2 has no line")
        .to_string();
    assert!(
        warning.ends_with(" 2") && !warning.contains("line"),
        "{warning}"
    );
    let dir = scratch(
        "whole",
        "01 0.5 1200 24000
02 0.5 1200 24000
03 0.5 900 24000
",
        THREE,
    );
    let mut app = App::new(Lang::En, &dir);
    app.load_file(&dir.join("JOB.NCD"));
    ClassicEngine::start(&mut app);
    assert!(ClassicEngine::warning(&app).is_none());
}

#[test]
fn a_complete_table_lets_the_job_start() {
    let dir = scratch(
        "complete",
        "01 0.5 1200 24000\r\n02 0,5 1200 24000\r\n03 0.5 900 24000\r\n",
        THREE,
    );
    let mut app = App::new(Lang::En, &dir);
    app.load_file(&dir.join("JOB.NCD"));
    ClassicEngine::start(&mut app);
    assert!(
        ClassicEngine::error(&app).is_none(),
        "{:?}",
        ClassicEngine::error(&app)
    );
}

#[test]
fn a_file_with_slots_is_not_taken_without_a_word() {
    let dir = scratch(
        "slots",
        "01 0.5 1200 24000\r\n",
        "M48\nINCH\nT01C0.0315\n%\nT01\nX500Y500\nG00X1000Y1000\nM15\nG01X2000Y1000\nM16\nM30\n",
    );
    let mut app = App::new(Lang::En, &dir);
    app.load_file(&dir.join("JOB.NCD"));
    let error = ClassicEngine::error(&app)
        .expect("the file is refused")
        .to_string();
    assert!(
        error.contains("slots") && error.contains("G00X1000Y1000"),
        "{error}"
    );
}
