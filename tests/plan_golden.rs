//! The plan of a job made on paper, with no machine: stored copies of what the plan says for the sample files, so a change that moves a step, a time or a check shows up here.
//!
//! The copies are in `tests/golden/`. After a change that is meant (and read), write them again with `EH3D_UPDATE_GOLDEN=1 cargo test --test plan_golden`.

use std::path::{Path, PathBuf};
use std::process::Command;

use eh3d_drillpro::app::App;
use eh3d_drillpro::i18n::Lang;

fn testdata() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

fn golden(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

fn plan(file: &str, lang: Lang) -> String {
    let mut app = App::new(lang, &testdata());
    app.load_file(&testdata().join(file));
    let mut text = app
        .plan_report()
        .unwrap_or_else(|e| panic!("{file}: {e}"))
        .join("\n");
    text.push('\n');
    text
}

fn same_as_stored(file: &str, lang: Lang) {
    let text = plan(file, lang);
    let stored = golden(&format!(
        "{}.{}.plan",
        file.to_ascii_lowercase(),
        lang.code()
    ));
    if std::env::var_os("EH3D_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(stored.parent().unwrap()).unwrap();
        std::fs::write(&stored, &text).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&stored)
        .unwrap_or_else(|_| {
            panic!(
                "{} is missing: write it with EH3D_UPDATE_GOLDEN=1",
                stored.display()
            )
        })
        .replace("\r\n", "\n");
    assert_eq!(text, want, "the plan of {file} in {} changed", lang.code());
}

#[test]
fn the_plan_of_each_sample_file_is_the_stored_one() {
    for file in ["T1.NCD", "14TOOLS.NCD", "refholes.ncd"] {
        same_as_stored(file, Lang::En);
    }
}

#[test]
fn the_plan_is_said_in_the_language_asked_and_has_the_same_facts() {
    let en = plan("T1.NCD", Lang::En);
    let es = plan("T1.NCD", Lang::Es);
    same_as_stored("T1.NCD", Lang::Es);
    assert_ne!(en, es);
    // the first five lines are facts that do not depend on the language
    assert_eq!(
        en.lines().take(5).collect::<Vec<_>>(),
        es.lines().take(5).collect::<Vec<_>>()
    );
}

#[test]
fn the_same_files_give_the_same_plan_twice() {
    assert_eq!(plan("14TOOLS.NCD", Lang::En), plan("14TOOLS.NCD", Lang::En));
}

#[test]
fn a_file_that_is_not_there_has_no_plan_and_says_why() {
    let mut app = App::new(Lang::En, &testdata());
    app.load_file(&testdata().join("NOT-THERE.PLT"));
    let e = app.plan_report().unwrap_err();
    assert!(e.contains("NOT-THERE") || !e.is_empty(), "{e}");
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_drillpro-plan"))
        .args(args)
        .output()
        .unwrap();
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn the_command_prints_the_plan_and_refuses_what_it_does_not_know() {
    let data = testdata();
    let file = data.join("T1.NCD");
    let (code, out, err) = run(&[file.to_str().unwrap(), "--data", data.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, plan("T1.NCD", Lang::En));
    let (code, _, err) = run(&[file.to_str().unwrap(), "--bogus"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("--bogus"), "{err}");
    let (code, _, err) = run(&[file.to_str().unwrap(), "--lang", "xx"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("xx"), "{err}");
    let (code, _, _) = run(&[]);
    assert_eq!(code, Some(1));
    let (code, _, _) = run(&[
        data.join("NOT-THERE.PLT").to_str().unwrap(),
        "--data",
        data.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(1));
}
