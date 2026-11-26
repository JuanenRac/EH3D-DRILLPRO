//! The `.eh3d` project: the drill layers are read from the job, each plan is kept in the branch of this program only, a job that changes makes the plans out of date, and the screens
//! open the first drill layer and keep its plan. Every file is made here from invented data.

use std::path::{Path, PathBuf};
use std::process::Command;

use eh3d_drillpro::app::App;
use eh3d_drillpro::classic::ClassicApp;
use eh3d_drillpro::eh3dfile::{self, ProjectJob, KIT};
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::screen::Screen;
use eh3d_drillpro::textapp::TextApp;
use eh3d_project::manufacturing::{self as suite, SOURCE_BRANCH};
use eh3d_project::{BranchUpdate, NewProject, OpenOptions, Project, SaveOptions, Writer};

const BIN: &str = env!("CARGO_BIN_EXE_drillpro-plan");
const HOLES: &str = "M48\nINCH\nT1C0.0315\n%\nT1\nX1000Y1000\nX2000Y1000\nM30\n";
const OTHER: &str = "M48\nINCH\nT1C0.0315\n%\nT1\nX3000Y3000\nM30\n";

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

fn write_branch(path: &Path, branch: &str, files: &[(&str, &[u8])]) {
    let mut update = BranchUpdate::new(branch, "1.0", "0.1.0");
    for (name, bytes) in files {
        update.entries.insert((*name).to_string(), bytes.to_vec());
    }
    Project::open(path, &OpenOptions::default())
        .unwrap()
        .save_branch(&Writer::for_app(branch), update, &SaveOptions::default())
        .unwrap();
}

fn write_job(path: &Path, plated: &str) {
    let record = br#"{"format_version":"1.0.0","layers":[
        {"path":"layers/plated.drl","kind":"drill","name":"Plated holes"},
        {"path":"layers/loose.drl","kind":"drill","name":"Loose holes"}]}"#;
    write_branch(
        path,
        SOURCE_BRANCH,
        &[
            ("project.json", record),
            ("layers/plated.drl", plated.as_bytes()),
            ("layers/loose.drl", HOLES.as_bytes()),
        ],
    );
}

fn project(dir: &Path) -> PathBuf {
    let path = dir.join("board.eh3d");
    let new = NewProject {
        id: "test-id".into(),
        name: "Invented board".into(),
        units: "millimetres".into(),
        created: None,
    };
    Project::create(
        &path,
        new,
        &OpenOptions::default(),
        &SaveOptions::default().atomic,
    )
    .unwrap();
    write_branch(&path, "pcb", &[("board.json", b"{}")]);
    write_job(&path, HOLES);
    path
}

fn hash(path: &Path, branch: &str) -> String {
    Project::open(path, &OpenOptions::default())
        .unwrap()
        .branch(branch)
        .unwrap()
        .hash
        .clone()
}

fn run(path: &Path, args: &[&str]) -> std::process::Output {
    Command::new(BIN)
        .arg("project")
        .arg(args[0])
        .arg(path)
        .args(&args[1..])
        .arg("--data")
        .arg(data())
        .output()
        .expect("the program runs")
}

fn text(app: &impl TextApp) -> String {
    let mut screen = Screen::default();
    app.render(&mut screen);
    screen.to_text()
}

#[test]
fn plan_keeps_a_plan_for_each_drill_layer_in_its_own_branch_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let (pcb, job) = (hash(&path, "pcb"), hash(&path, SOURCE_BRANCH));
    let out = run(&path, &["plan"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(hash(&path, "pcb"), pcb);
    assert_eq!(hash(&path, SOURCE_BRANCH), job);
    let state = suite::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["drill-loose.txt", "drill-plated.txt"]);
    assert_eq!(state.inputs, ["layers/loose.drl", "layers/plated.drl"]);
    assert!(!state.out_of_date);
    let plan = suite::read_result(&KIT, &path, "drill-plated.txt").unwrap();
    let plan = String::from_utf8(plan).unwrap();
    assert!(plan.contains("file: plated.drl\nholes: 2\n"), "{plan}");
    assert_eq!(run(&path, &["info"]).status.code(), Some(0));
}

#[test]
fn a_job_that_changes_makes_the_plans_out_of_date_until_they_are_made_again() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    assert!(run(&path, &["plan"]).status.success());
    write_job(&path, OTHER);
    assert_eq!(run(&path, &["info"]).status.code(), Some(2));
    assert!(run(&path, &["plan", "--layer", "plated.drl"])
        .status
        .success());
    let state = suite::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["drill-plated.txt"]);
    assert!(!state.out_of_date);
}

#[test]
fn a_refused_plan_leaves_the_project_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let before = std::fs::read(&path).unwrap();
    for args in [
        &["plan", "--layer", "missing.drl"][..],
        &["plan", "--nothing-like-this"],
        &["info", "--layer", "plated.drl"],
        &["drill"],
    ] {
        assert_ne!(run(&path, args).status.code(), Some(0), "{args:?}");
        assert_eq!(std::fs::read(&path).unwrap(), before, "{args:?}");
    }
    // a layer that cannot be read stops every plan
    write_job(&path, "T1\nnot a hole\n");
    let before = std::fs::read(&path).unwrap();
    assert_ne!(run(&path, &["plan"]).status.code(), Some(0));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(eh3dfile::drill_layers(Path::new("missing.eh3d")).is_err());
}

#[test]
fn the_screens_open_the_first_drill_layer_and_keep_its_plan() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let mut modern = App::new(Lang::En, &data());
    modern.load_file(&path);
    let shown = text(&modern);
    assert!(shown.contains("plated.drl"), "{shown}");
    assert!(shown.contains("layer 1 of 2"), "{shown}");
    modern.save_project();
    assert!(text(&modern).contains("drill-plated.txt saved"));
    assert_eq!(
        suite::state(&KIT, &path).unwrap().results,
        ["drill-plated.txt"]
    );

    write_job(&path, OTHER);
    let mut classic = ClassicApp::new(Lang::En, &data());
    classic.load_file(&path);
    let shown = text(&classic);
    assert!(shown.contains("OUT OF DATE"), "{shown}");
    classic.save_project();
    assert!(!suite::state(&KIT, &path).unwrap().out_of_date);

    // a drill file of its own takes the place of the project: Ctrl+S then writes nothing
    let before = std::fs::read(&path).unwrap();
    let file = dir.path().join("own.ncd");
    std::fs::write(&file, HOLES).unwrap();
    classic.load_file(&file);
    classic.save_project();
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn what_a_window_says_of_a_project_is_in_the_language_of_the_screen() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let job = ProjectJob::open(&KIT, &path, "drill").unwrap();
    assert_eq!((job.layers, job.out_of_date), (2, false));
    assert!(job.notice("es").contains("capa 1 de 2"));
    assert!(ProjectJob::saved_notice("a.txt", "de").contains("a.txt"));
}
