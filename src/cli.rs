//! The command line shared by the programs and their front ends (window and console).

use std::path::PathBuf;

use crate::i18n::Lang;

/// File where the chosen language is remembered between runs.
pub fn settings_path(name: &str) -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(name)
}

/// What the command line asks for.
#[derive(Debug, Clone)]
pub struct Options {
    pub lang: Lang,
    /// The folder of the configuration and the tool table.
    pub data: PathBuf,
    pub file: Option<PathBuf>,
    /// The machine file to use instead of the default one.
    pub connection: Option<PathBuf>,
    /// The menu-driven interface instead of the classic screens.
    pub modern: bool,
    /// Print the screen as text and leave.
    pub dump: bool,
    /// Keys to play before a screenshot (`--keys alt+v,enter`), the file of the screenshot and the frames to wait.
    pub keys: String,
    pub shot: Option<PathBuf>,
    pub wait: u32,
}

/// Reads the arguments. `settings` names the file that remembers the language; `data_file` is the configuration file
/// that tells which folder holds the program's files when `--data` is not given.
pub fn parse(settings: &str, data_file: &str) -> Options {
    parse_from(std::env::args().skip(1), settings, data_file)
}

/// [`parse`] over any list of arguments.
pub fn parse_from(
    args: impl IntoIterator<Item = String>,
    settings: &str,
    data_file: &str,
) -> Options {
    let mut o = Options {
        lang: std::fs::read_to_string(settings_path(settings))
            .ok()
            .and_then(|s| Lang::from_code(&s))
            .unwrap_or_default(),
        data: crate::config::find_data_dir(data_file),
        file: None,
        connection: None,
        modern: false,
        dump: false,
        keys: String::new(),
        shot: None,
        wait: 8,
    };
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--lang" => {
                if let Some(l) = args.next().and_then(|c| Lang::from_code(&c)) {
                    o.lang = l;
                }
            }
            "--data" => {
                if let Some(d) = args.next() {
                    o.data = PathBuf::from(d);
                }
            }
            "--connection" => o.connection = args.next().map(PathBuf::from),
            "--dump" => o.dump = true,
            "--modern" => o.modern = true,
            "--classic" => o.modern = false,
            "--keys" => o.keys = args.next().unwrap_or_default(),
            "--shot" => o.shot = args.next().map(PathBuf::from),
            "--wait" => {
                if let Some(n) = args.next().and_then(|n| n.parse().ok()) {
                    o.wait = n;
                }
            }
            other => o.file = Some(PathBuf::from(other)),
        }
    }
    o
}

/// The classic screens over `engine`, with the machine link and the file the command line names.
pub fn start_classic<E: crate::classic::engine::ClassicEngine>(
    mut engine: E,
    o: &Options,
) -> crate::classic::ClassicApp<E> {
    match crate::protocol::connection::load(o.connection.as_deref()) {
        Ok(c) => engine.set_connection(c),
        Err(e) => engine.set_error(e),
    }
    let mut app = crate::classic::ClassicApp::with_engine(engine);
    if let Some(f) = &o.file {
        if f.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
            app.load_zip(f);
        } else {
            app.load_file(f);
        }
    }
    app
}

/// The screen of `app` as plain text (the `--dump` option).
pub fn dump_text(app: &impl crate::textapp::TextApp) -> String {
    let mut s = crate::screen::Screen::default();
    app.render(&mut s);
    s.to_text()
}
