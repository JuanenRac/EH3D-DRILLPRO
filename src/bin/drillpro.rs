//! DRILLPRO front end.
//!
//! Usage: `drillpro [FILE.NCD] [--lang en|es|it|de|fr|pt|zh|ja] [--data DIR] [--connection FILE] [--dump]`

use std::path::PathBuf;

use eh3d_drillpro::app::App;
use eh3d_drillpro::gui;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::screen::Screen;

const SETTINGS: &str = "eh3d-drillpro.lang";

fn main() -> eframe::Result {
    let mut lang = std::fs::read_to_string(gui::settings_path(SETTINGS))
        .ok()
        .and_then(|s| Lang::from_code(&s))
        .unwrap_or_default();
    let (mut data, mut file, mut dump) = (PathBuf::from("."), None, false);
    let mut connection: Option<PathBuf> = None;
    let (mut keys, mut shot, mut wait) = (String::new(), None::<PathBuf>, 8u32);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--lang" => {
                lang = args
                    .next()
                    .and_then(|c| Lang::from_code(&c))
                    .unwrap_or(lang)
            }
            "--data" => data = args.next().map(PathBuf::from).unwrap_or(data),
            "--connection" => connection = args.next().map(PathBuf::from),
            "--dump" => dump = true,
            "--keys" => keys = args.next().unwrap_or_default(),
            "--shot" => shot = args.next().map(PathBuf::from),
            "--wait" => wait = args.next().and_then(|n| n.parse().ok()).unwrap_or(wait),
            other => file = Some(PathBuf::from(other)),
        }
    }
    let mut app = App::new(lang, &data);
    // How the job reaches the machine: the connection file shared by DRILLPRO and ROUTEPRO (demo when there is none).
    match eh3d_drillpro::protocol::connection::load(connection.as_deref()) {
        Ok(c) => app.set_connection(c),
        Err(e) => app.set_error(e),
    }
    if let Some(f) = &file {
        app.load_file(f);
    }
    if dump {
        let mut s = Screen::default();
        app.render(&mut s);
        print!("{}", s.to_text());
        return Ok(());
    }
    let capture = shot.map(|file| gui::Capture {
        keys: gui::parse_keys(&keys),
        file,
        wait,
    });
    gui::run_capture("DRILLPRO", SETTINGS, app, capture)
}
