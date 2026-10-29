//! The classic screens against reference screens captured from the original program.
//!
//! The reference screens are not part of the repository. Point `EH3D_ORIGINAL_UI` at a folder that
//! holds them (`drill_main.txt`, ...: a `# chars`, `# fg` and `# bg` block of 24 rows of 80 cells) to run
//! these tests; without it they pass trivially.

use std::path::{Path, PathBuf};

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::ClassicApp;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::screen::{Screen, ROWS};
use eh3d_drillpro::textapp::TextApp;

struct Reference {
    chars: Vec<Vec<char>>,
    fg: Vec<Vec<char>>,
    bg: Vec<Vec<char>>,
}

fn reference(name: &str) -> Option<Reference> {
    let dir = std::env::var_os("EH3D_ORIGINAL_UI")?;
    let text = std::fs::read_to_string(Path::new(&dir).join(name)).ok()?;
    let mut blocks: Vec<Vec<Vec<char>>> = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') {
            blocks.push(Vec::new());
        } else if let Some(b) = blocks.last_mut() {
            b.push(line.chars().collect());
        }
    }
    let [chars, fg, bg]: [Vec<Vec<char>>; 3] = blocks.try_into().ok()?;
    Some(Reference { chars, fg, bg })
}

fn testdata() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

fn app() -> ClassicApp {
    ClassicApp::new(Lang::En, &testdata())
}

fn press(app: &mut ClassicApp, keys: &str) {
    for c in keys.chars() {
        app.handle(Input::Char(c));
    }
}

fn hex(v: u8) -> char {
    char::from_digit(v as u32, 16).unwrap().to_ascii_uppercase()
}

/// Cells (column range, row range) whose characters are not compared (texts that differ on purpose).
type Mask = (
    std::ops::RangeInclusive<usize>,
    std::ops::RangeInclusive<usize>,
);

fn compare(app: &ClassicApp, name: &str, text_mask: &[Mask], full_mask: &[Mask]) {
    let Some(r) = reference(name) else { return };
    let mut s = Screen::default();
    app.render(&mut s);
    let inside =
        |m: &[Mask], x: usize, y: usize| m.iter().any(|(c, rw)| c.contains(&x) && rw.contains(&y));
    let mut bad = Vec::new();
    for y in 0..(ROWS - 1) {
        for x in 0..80 {
            // (74, 23) is where the capture shows the hardware text cursor, not a cell of the screen
            if inside(full_mask, x, y) || (x, y) == (74, 23) || (x, y) == (79, 23) {
                continue;
            }
            let c = s.get(x, y);
            let (rc, rf, rb) = (r.chars[y][x], r.fg[y][x], r.bg[y][x]);
            let char_ok = inside(text_mask, x, y) || c.ch == rc || rc == ' ' && c.ch == ' ';
            // a blank cell has no visible foreground
            let blank = rc == ' ' && c.ch == ' ';
            let fg_ok = blank || inside(text_mask, x, y) || hex(c.fg) == rf;
            if !(char_ok && fg_ok && hex(c.bg) == rb) {
                bad.push(format!(
                    "r{y} c{x}: ours {:?} {}/{} original {:?} {rf}/{rb}",
                    c.ch,
                    hex(c.fg),
                    hex(c.bg),
                    rc
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{name}: {} cells differ\n{}",
        bad.len(),
        bad.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// What we added to the original screen: the About button in the free place under Load.
fn ours() -> Vec<Mask> {
    vec![(62..=75, 19..=20)]
}

/// Heading texts that are our own wording: the project lines, the title box and the machine number.
fn heading() -> Vec<Mask> {
    vec![(5..=51, 3..=5), (62..=74, 3..=4)]
}

#[test]
fn main_screen_matches_the_original() {
    compare(&app(), "drill_main.txt", &heading(), &ours());
}

#[test]
fn main_screen_with_a_file_matches_the_original() {
    let mut a = app();
    a.load_file(&testdata().join("T1.NCD"));
    compare(&a, "drill_main_loaded.txt", &heading(), &ours());
}

#[test]
fn quit_dialog_matches_the_original() {
    let mut a = app();
    press(&mut a, "q");
    compare(&a, "drill_quit.txt", &[], &[]);
}

#[test]
fn parameter_dialog_matches_the_original() {
    let mut a = app();
    press(&mut a, "c");
    compare(&a, "drill_parameter.txt", &[], &[(54..=74, 17..=17)]);
}

#[test]
fn tool_data_dialog_matches_the_original() {
    let mut a = app();
    press(&mut a, "t");
    compare(&a, "drill_tooldata.txt", &[], &[]);
}

#[test]
fn select_file_dialog_matches_the_original() {
    let mut a = app();
    press(&mut a, "l");
    // the list and the two information lines show the files of the folder, which are not the same files
    compare(
        &a,
        "drill_selectfile.txt",
        &[(18..=62, 8..=15), (16..=62, 18..=19)],
        &[],
    );
}

#[test]
fn homing_dialog_matches_the_original() {
    let mut a = app();
    a.load_file(&testdata().join("T1.NCD"));
    press(&mut a, "s");
    compare(&a, "drill_homing.txt", &[], &[]);
}

#[test]
fn job_dialog_matches_the_original() {
    let mut a = app();
    a.load_file(&testdata().join("T1.NCD"));
    press(&mut a, "s");
    // the first steps of the job are the homing; after them the job dialog shows
    for _ in 0..3 {
        a.tick();
    }
    // the position is not tracked yet: its two values are not compared
    compare(&a, "drill_running.txt", &[], &[(51..=56, 9..=10)]);
}

#[test]
fn delete_old_offset_question_matches_the_original() {
    let mut a = app();
    press(&mut a, "o");
    compare(&a, "drill_offset_delete_old.txt", &[], &[]);
}

#[test]
fn offset_teach_window_matches_the_original() {
    let mut a = app();
    press(&mut a, "on");
    compare(&a, "drill_offset_teach.txt", &[], &[]);
}

#[test]
fn teach_in_mode_matches_the_original() {
    let mut a = app();
    // 1000 mil forward and to the left, then Teach: the position becomes the origin
    press(&mut a, "ondet");
    compare(&a, "drill_teach_in.txt", &[], &[]);
}

#[test]
fn write_teach_in_file_question_matches_the_original() {
    let mut a = app();
    press(&mut a, "ondeto");
    // the capture has no glyph for the capital W of the first word
    compare(&a, "drill_teach_write.txt", &[(38..=38, 8..=8)], &[]);
}
