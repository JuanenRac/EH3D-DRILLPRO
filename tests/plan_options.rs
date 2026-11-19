//! The plan on paper with the options of the form: the order of the holes, the vectors, the tools switched off, the repeats left out, a line for each tool and the exit code that
//! says whether the plan stays inside the course of the machine.

use std::path::{Path, PathBuf};
use std::process::Command;

fn testdata() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// A job of three holes of the first tool, one of them drilled twice, and one hole of a second tool.
fn job(name: &str) -> String {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(
        &path,
        "%\nT01\nX1000Y1000\nX2000Y1000\nX1000Y1000\nT02\nX3000Y3000\nM30\n",
    )
    .unwrap();
    path.to_str().unwrap().to_string()
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let data = testdata();
    let o = Command::new(env!("CARGO_BIN_EXE_drillpro-plan"))
        .args(args)
        .args(["--data", data.to_str().unwrap()])
        .output()
        .unwrap();
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn has(out: &str, line: &str) -> bool {
    out.lines().any(|l| l == line)
}

#[test]
fn the_plan_without_options_is_the_one_of_the_file_and_has_no_extra_lines() {
    let file = job("plan_options_plain.ncd");
    let (code, out, err) = run(&[&file]);
    assert_eq!(code, Some(0), "{err}");
    assert!(has(&out, "holes: 4"), "{out}");
    assert!(has(&out, "tools: 2"), "{out}");
    assert!(!out.contains("repeats left out"), "{out}");
    assert!(!out.contains("of travel"), "{out}");
}

#[test]
fn the_repeats_are_left_out_only_when_asked_and_counted() {
    let file = job("plan_options_repeats.ncd");
    let (code, out, err) = run(&[&file, "--skip-duplicates"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(has(&out, "holes: 3"), "{out}");
    assert!(has(&out, "repeats left out: 1"), "{out}");
}

#[test]
fn a_line_for_each_tool_gives_its_holes_and_its_travel() {
    let file = job("plan_options_tools.ncd");
    let (code, out, err) = run(&[&file, "--per-tool", "--skip-duplicates"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(has(&out, "tool 1: 2 holes, 25.4 mm of travel"), "{out}");
    assert!(has(&out, "tool 2: 1 holes, 0.0 mm of travel"), "{out}");
}

#[test]
fn the_vectors_and_the_tools_switched_off_leave_holes_out_of_the_plan() {
    let file = job("plan_options_range.ncd");
    let (code, out, err) = run(&[&file, "--from", "2", "--to", "3"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(has(&out, "holes: 2"), "{out}");
    let (code, out, err) = run(&[&file, "--skip-tool", "2"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(has(&out, "holes: 3"), "{out}");
    assert!(has(&out, "tools: 1"), "{out}");
    // a tool that the job does not use switches nothing off, and the option may be given more than once
    let (code, out, err) = run(&[&file, "--skip-tool", "9", "--skip-tool", "1"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(has(&out, "holes: 1"), "{out}");
}

#[test]
fn the_order_of_the_holes_is_chosen_by_name_and_the_plan_stays_the_same_size() {
    let file = job("plan_options_sort.ncd");
    for name in ["none", "x", "y", "special", "Y-First"] {
        let (code, out, err) = run(&[&file, "--sort", name]);
        assert_eq!(code, Some(0), "{name}: {err}");
        assert!(has(&out, "holes: 4"), "{name}: {out}");
    }
    let (code, _, err) = run(&[&file, "--sort", "shortest"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("--sort") && err.contains("shortest"), "{err}");
}

#[test]
fn a_number_that_is_not_one_and_a_missing_value_are_refused_with_the_option() {
    let file = job("plan_options_bad.ncd");
    let f = file.as_str();
    let cases: [&[&str]; 4] = [
        &[f, "--from", "two"],
        &[f, "--to", "-1"],
        &[f, "--skip-tool", "300"],
        &[f, "--skip-tool"],
    ];
    for args in cases {
        let (code, _, err) = run(args);
        assert_eq!(code, Some(1), "{args:?}");
        assert!(
            err.contains(args[args.len() - 1]) || err.contains("--"),
            "{err}"
        );
    }
}

#[test]
fn check_ends_with_zero_when_the_plan_stays_inside_the_course() {
    let file = job("plan_options_check.ncd");
    let (code, out, err) = run(&[&file, "--check"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        out.contains("Dry run: the exact plan stays inside the course of every axis"),
        "{out}"
    );
    // the flag is not an unknown option, and the same plan without it ends the same way
    let (plain, _, _) = run(&[&file]);
    assert_eq!(plain, Some(0));
}
