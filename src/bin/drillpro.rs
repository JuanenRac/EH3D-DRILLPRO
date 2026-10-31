//! DRILLPRO front end (window).
//!
//! Usage: `drillpro [FILE.NCD] [--lang en|es|it|de|fr|pt|zh|ja] [--data DIR] [--connection FILE] [--modern] [--dump]`
//!
//! The classic screens of the MS-DOS program are the default; `--modern` starts the menu-driven interface.
//! Without `--data` the files are looked for in the current folder, the folder of the program and the user's
//! configuration folder.

#![cfg_attr(windows, windows_subsystem = "windows")]

use eh3d_drillpro::app::App;
use eh3d_drillpro::cli;
use eh3d_drillpro::gui;

const SETTINGS: &str = "eh3d-drillpro.lang";

fn main() -> eframe::Result {
    attach_console();
    let o = cli::parse(SETTINGS, "DRILLPRO.DCF");
    if o.machines {
        std::process::exit(cli::print_fleet(&o));
    }
    let capture = o.shot.clone().map(|file| gui::Capture {
        keys: gui::parse_keys(&o.keys),
        file,
        wait: o.wait,
        point: o.point,
    });
    if o.modern {
        let mut app = App::new(o.lang, &o.data);
        match eh3d_drillpro::protocol::connection::load(o.connection.as_deref()) {
            Ok(mut c) => {
                if let Some(m) = &o.machine {
                    c.server_machine = m.clone();
                }
                app.set_connection(c)
            }
            Err(e) => app.set_error(e),
        }
        if let Some(f) = &o.file {
            app.load_file(f);
        }
        if o.dump {
            let mut s = eh3d_drillpro::screen::Screen::default();
            app.render(&mut s);
            print!("{}", s.to_text());
            return Ok(());
        }
        return gui::run_capture("DRILLPRO", SETTINGS, app, capture);
    }
    let app = cli::start_classic(App::new(o.lang, &o.data), &o);
    if o.dump {
        print!("{}", cli::dump_text(&app));
        return Ok(());
    }
    gui::run_capture("DRILLPRO", SETTINGS, app, capture)
}

/// A program with a window has no console of its own on Windows (no black window); when it was started from a terminal, what it prints (the help, a mistake in the options) goes to that terminal.
#[cfg(windows)]
fn attach_console() {
    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    // SAFETY: a call of the system with a constant; if there is no parent console it does nothing.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_console() {}
