//! Moving through the folders of the computer in the Select File dialog, and what the form adds to a job.

use std::path::{Path, PathBuf};

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::engine::{absolute, dos_order, drive_entries, drive_path};
use eh3d_drillpro::classic::ClassicApp;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::job::{select_holes, JobOptions, Panel, Sorting};
use eh3d_drillpro::ncd::{DrillFile, Hole};
use eh3d_drillpro::screen::Screen;
use eh3d_drillpro::textapp::TextApp;

fn text(a: &ClassicApp) -> String {
    let mut s = Screen::default();
    a.render(&mut s);
    s.to_text()
}

fn click_on(a: &mut ClassicApp, needle: &str) {
    let t = text(a);
    for (y, line) in t.lines().enumerate() {
        if let Some(x) = line.find(needle) {
            a.click(line[..x].chars().count(), y);
            return;
        }
    }
    panic!("{needle:?} is not on the screen:\n{t}");
}

fn press(a: &mut ClassicApp, keys: &str) {
    for c in keys.chars() {
        a.handle(Input::Char(c));
    }
}

/// A scratch folder with files and sub folders.
fn tree(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d-nav-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("jobs").join("deeper")).unwrap();
    std::fs::create_dir_all(dir.join("archive")).unwrap();
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    for f in ["DRILLPRO.DCF", "DRILLPRO.DTD", "T1.NCD"] {
        std::fs::copy(testdata.join(f), dir.join(f)).unwrap();
    }
    std::fs::copy(testdata.join("T1.NCD"), dir.join("jobs").join("INSIDE.NCD")).unwrap();
    std::fs::copy(
        testdata.join("T1.NCD"),
        dir.join("jobs").join("deeper").join("DEEP.NCD"),
    )
    .unwrap();
    std::fs::write(dir.join(".hidden.ncd"), "%\nT01\nX1Y1\nM30\n").unwrap();
    dir
}

#[test]
fn the_list_is_files_then_folders_then_the_parent_like_the_original() {
    let order = dos_order(vec![
        ("zeta".into(), true),
        ("..".into(), true),
        ("b.ncd".into(), false),
        ("Alpha".into(), true),
        ("A.NCD".into(), false),
    ]);
    let names: Vec<&str> = order.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["A.NCD", "b.ncd", "Alpha", "zeta", ".."]);
}

#[test]
fn a_folder_is_entered_and_left_with_the_mouse_and_hidden_files_stay_hidden() {
    let dir = tree("enter");
    let mut a = ClassicApp::new(Lang::En, &dir);
    press(&mut a, "l");
    let t = text(&a);
    assert!(
        t.contains("jobs\\") && t.contains("archive\\") && t.contains("..\\"),
        "{t}"
    );
    assert!(!t.contains(".hidden"), "{t}");
    // two clicks go into the folder
    click_on(&mut a, "jobs\\");
    click_on(&mut a, "jobs\\");
    let t = text(&a);
    assert!(t.contains("INSIDE.NCD") && t.contains("deeper\\"), "{t}");
    assert!(t.contains("jobs"), "the path line names the folder");
    // and back up with ..\
    click_on(&mut a, "..\\");
    click_on(&mut a, "..\\");
    assert!(text(&a).contains("archive\\"), "{}", text(&a));
}

#[test]
fn a_relative_folder_can_still_go_up() {
    // the files are looked for in `.` or `testdata`: going up must work from a relative folder too
    let mut a = ClassicApp::new(Lang::En, Path::new("testdata"));
    press(&mut a, "l");
    click_on(&mut a, "..\\");
    click_on(&mut a, "..\\");
    let t = text(&a);
    assert!(t.contains("testdata\\") && t.contains("src\\"), "{t}");
}

#[test]
fn a_written_path_goes_to_its_folder_and_takes_the_rest_as_the_pattern() {
    let dir = tree("typed");
    let mut a = ClassicApp::new(Lang::En, &dir);
    press(&mut a, "l");
    let target = dir.join("jobs");
    let typed = format!("{}\\*.ncd", target.display());
    for c in typed.chars() {
        a.handle(Input::Char(c));
    }
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(t.contains("INSIDE.NCD"), "{t}");
    assert!(!t.contains("T1.NCD"), "{t}");
    // a file written with its path is opened
    let mut b = ClassicApp::new(Lang::En, &dir);
    press(&mut b, "l");
    let file = dir.join("jobs").join("deeper").join("DEEP.NCD");
    for c in file.display().to_string().chars() {
        b.handle(Input::Char(c));
    }
    b.handle(Input::Enter);
    assert!(text(&b).contains("Tool Data"), "{}", text(&b));
    // a folder that does not exist is said so
    let mut c = ClassicApp::new(Lang::En, &dir);
    press(&mut c, "l");
    for ch in format!("{}\\nowhere\\x.ncd", dir.display()).chars() {
        c.handle(Input::Char(ch));
    }
    c.handle(Input::Enter);
    assert!(text(&c).contains("Cannot open that folder"), "{}", text(&c));
}

#[test]
fn the_drives_of_a_windows_computer_are_in_the_list() {
    let drives = drive_entries();
    if cfg!(windows) {
        assert!(drives.iter().any(|(n, d)| n == "[-C-]" && *d), "{drives:?}");
        assert_eq!(drive_path("[-C-]"), Some(PathBuf::from("C:\\")));
        let dir = tree("drives");
        let mut a = ClassicApp::new(Lang::En, &dir);
        press(&mut a, "l");
        assert!(text(&a).contains("[-C-]"), "{}", text(&a));
        click_on(&mut a, "[-C-]");
        click_on(&mut a, "[-C-]");
        assert!(text(&a).contains("C:\\"), "{}", text(&a));
    } else {
        assert!(drives.is_empty());
    }
    assert_eq!(drive_path("jobs"), None);
    assert!(absolute(Path::new(".")).is_absolute());
}

#[test]
fn a_long_name_ends_with_a_tilde() {
    let dir = tree("long");
    std::fs::write(dir.join("A_VERY_LONG_FILE_NAME.NCD"), "%\nT01\nX1Y1\nM30\n").unwrap();
    let mut a = ClassicApp::new(Lang::En, &dir);
    press(&mut a, "l");
    assert!(text(&a).contains("A_VERY_LONG_F~"), "{}", text(&a));
}

fn file_of(holes: &[(u8, i32, i32)]) -> DrillFile {
    let mut f = DrillFile::default();
    for &(tool, x, y) in holes {
        if !f.tools.contains(&tool) {
            f.tools.push(tool);
        }
        f.holes.push(Hole {
            tool,
            x_mil: x,
            y_mil: y,
        });
    }
    f
}

#[test]
fn the_vectors_asked_for_are_the_only_ones_drilled() {
    let f = file_of(&[
        (1, 1000, 1000),
        (1, 2000, 1000),
        (1, 3000, 1000),
        (1, 4000, 1000),
    ]);
    let all = select_holes(
        &f,
        Sorting::None,
        &JobOptions::default(),
        0.0254,
        (1.0, 1.0),
        (0.0, 0.0),
    );
    assert_eq!(all.holes.len(), 4);
    let some = select_holes(
        &f,
        Sorting::None,
        &JobOptions {
            from: 2,
            to: 3,
            ..Default::default()
        },
        0.0254,
        (1.0, 1.0),
        (0.0, 0.0),
    );
    assert_eq!(some.holes.len(), 2);
    assert!((some.holes[0].x_mm - 50.8).abs() < 1e-9);
}

#[test]
fn holes_outside_the_working_area_are_left_out_and_counted() {
    let f = file_of(&[(1, 1000, 1000), (1, 30000, 1000), (1, 1000, 30000)]);
    let o = JobOptions {
        clip_mm: Some((555.0, 405.0)),
        ..Default::default()
    };
    let s = select_holes(&f, Sorting::None, &o, 0.0254, (1.0, 1.0), (2.5, 2.5));
    // 30000 mil = 762 mm: past 555 in X and past 405 in Y
    assert_eq!(s.holes.len(), 1);
    assert_eq!(s.skipped, 2);
    // the offset counts: a hole at 553 mm in X moves outside with an offset of 3 mm
    let g = file_of(&[(1, 21772, 1000)]);
    let inside = select_holes(&g, Sorting::None, &o, 0.0254, (1.0, 1.0), (0.0, 0.0));
    let outside = select_holes(&g, Sorting::None, &o, 0.0254, (1.0, 1.0), (3.0, 0.0));
    assert_eq!((inside.holes.len(), outside.holes.len()), (1, 0));
}

#[test]
fn a_panel_repeats_every_tool_over_all_the_copies() {
    let f = file_of(&[(1, 1000, 1000), (2, 2000, 2000)]);
    let o = JobOptions {
        panel: Some(Panel {
            columns: 3,
            rows: 2,
            pitch_mm: (50.0, 40.0),
            tabs: None,
        }),
        ..Default::default()
    };
    let s = select_holes(&f, Sorting::None, &o, 0.0254, (1.0, 1.0), (0.0, 0.0));
    assert_eq!(s.holes.len(), 12);
    // tool 1 in all six copies, then tool 2: the tool change is done once
    assert!(s.holes[..6].iter().all(|h| h.tool == 1));
    assert!(s.holes[6..].iter().all(|h| h.tool == 2));
    let xs: Vec<f64> = s.holes[..6]
        .iter()
        .map(|h| (h.x_mm * 10.0).round() / 10.0)
        .collect();
    assert_eq!(xs, [25.4, 75.4, 125.4, 25.4, 75.4, 125.4]);
    assert!((s.holes[3].y_mm - (25.4 + 40.0)).abs() < 1e-9);
}

#[test]
fn the_form_and_the_panel_reach_the_job_and_the_window_shows_it() {
    let dir = tree("job");
    let mut a = ClassicApp::new(Lang::En, &dir);
    a.load_file(&dir.join("T1.NCD"));
    // Panelization: the copies that fit, accepted with Ok
    press(&mut a, "p");
    a.handle(Input::Enter);
    // the three holes become the copies of the panel
    press(&mut a, "s");
    for _ in 0..40 {
        a.tick();
    }
    let t = text(&a);
    assert!(
        t.contains("Drilling")
            || t.contains("Start Pos.")
            || t.contains("Attention!")
            || t.contains("Tool Change")
            || t.contains("Homing"),
        "{t}"
    );
    let view = {
        use eh3d_drillpro::classic::engine::ClassicEngine;
        a.engine.job_view().expect("a job is running")
    };
    assert!(view.percent <= 100);
    assert_eq!(view.tool, Some(1));
}

#[test]
fn the_wheel_scrolls_the_file_list() {
    let dir = tree("wheel");
    for i in 0..30 {
        std::fs::write(dir.join(format!("F{i:02}.NCD")), "%\nT01\nX1Y1\nM30\n").unwrap();
    }
    let mut a = ClassicApp::new(Lang::En, &dir);
    press(&mut a, "l");
    a.scroll(20);
    // the second page: the first files are gone, later ones show
    let t = text(&a);
    assert!(t.contains("F20.NCD") || t.contains("F21.NCD"), "{t}");
    assert!(!t.contains("F00.NCD"), "{t}");
    a.scroll(-100);
    assert!(text(&a).contains("F00.NCD"));
}

#[test]
fn a_file_that_cannot_be_read_is_said_so_in_a_window() {
    let dir = tree("bad");
    std::fs::write(
        dir.join("BAD.NCD"),
        "X1000Y1000\nthis is not a drill file\n",
    )
    .unwrap();
    let mut a = ClassicApp::new(Lang::En, &dir);
    press(&mut a, "l");
    press(&mut a, "bad.ncd");
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(t.contains("Error reading"), "{t}");
    // any key closes the window and the form is back with no file
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(t.contains("--------.---"), "{t}");
}
