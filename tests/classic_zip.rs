//! Jobs packed in ZIP archives: the Select File dialog goes inside them and the command line takes them.

use std::io::Write;
use std::path::{Path, PathBuf};

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::ClassicApp;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::screen::Screen;
use eh3d_drillpro::textapp::TextApp;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d-zip-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    for f in ["DRILLPRO.DCF", "DRILLPRO.DTD"] {
        std::fs::copy(testdata.join(f), dir.join(f)).unwrap();
    }
    let ncd =
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/03_two_tools.ncd"))
            .unwrap();
    let mut w = zip::ZipWriter::new(std::fs::File::create(dir.join("JOB.ZIP")).unwrap());
    let o = zip::write::SimpleFileOptions::default();
    w.start_file("notes.txt", o).unwrap();
    w.write_all(b"hello").unwrap();
    w.start_file("sub/PACKED.NCD", o).unwrap();
    w.write_all(&ncd).unwrap();
    w.finish().unwrap();
    dir
}

fn text(a: &ClassicApp) -> String {
    let mut s = Screen::default();
    a.render(&mut s);
    s.to_text()
}

#[test]
fn the_command_line_loads_the_first_drill_file_of_an_archive() {
    let dir = scratch("cli");
    let mut a = ClassicApp::new(Lang::En, &dir);
    a.load_zip(&dir.join("JOB.ZIP"));
    assert!(text(&a).contains("PACKED"), "{}", text(&a));
}

#[test]
fn the_dialog_goes_inside_an_archive_and_loads_a_file() {
    let dir = scratch("dlg");
    let mut a = ClassicApp::new(Lang::En, &dir);
    a.handle(Input::Char('l'));
    for c in "JOB.ZIP".chars() {
        a.handle(Input::Char(c));
    }
    a.handle(Input::Enter);
    let inside = text(&a);
    assert!(
        inside.contains("PACKED.NCD") && !inside.contains("notes"),
        "{inside}"
    );
    a.handle(Input::Enter);
    // loading a job opens the tool table of the job, as with a plain file
    assert!(text(&a).contains("Tool Data"), "{}", text(&a));
    a.handle(Input::Enter);
    assert!(text(&a).contains("PACKED"), "{}", text(&a));
}
