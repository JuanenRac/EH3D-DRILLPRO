//! The connection with the machine, chosen in the Parameter window: a drop-down with the five ways and, under it, what each one needs; what is chosen is
//! applied with OK and written into the machine file without touching the other lines of it.

use std::path::{Path, PathBuf};

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::engine::ClassicEngine;
use eh3d_drillpro::classic::ClassicApp;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::protocol::connection::{load, Kind};
use eh3d_drillpro::screen::Screen;
use eh3d_drillpro::textapp::TextApp;

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("eh3d-drillpro-conn-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["DRILLPRO.DCF", "DRILLPRO.DTD"] {
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("testdata")
                .join(f),
            dir.join(f),
        )
        .unwrap();
    }
    dir
}

fn text(a: &ClassicApp) -> String {
    let mut s = Screen::default();
    a.render(&mut s);
    s.to_text()
}

fn type_in(a: &mut ClassicApp, keys: &str) {
    for c in keys.chars() {
        a.handle(Input::Char(c));
    }
}

#[test]
fn the_connection_is_chosen_in_the_config_window_and_written_into_the_machine_file() {
    let dir = scratch("choose");
    let file = dir.join("machine.cfg");
    std::fs::write(
        &file,
        "# my machine\nconnection = demo\nx_search_hz = 2700\n",
    )
    .unwrap();
    let mut a = ClassicApp::new(Lang::En, &dir);
    a.set_machine_file(Some(file.clone()));
    // the Config window shows the way the job goes now
    a.handle(Input::Char('c'));
    let t = text(&a);
    assert!(
        t.contains("Connection") && t.contains("Demo (simulator)"),
        "{t}"
    );
    // the first place of the window is the connection: Enter opens its window
    a.handle(Input::Tab);
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(
        t.contains("Connection with the machine") && t.contains("The job runs on the simulator"),
        "{t}"
    );
    // the simulator needs nothing; the server asks for an address, a token and a machine
    for _ in 0..4 {
        a.handle(Input::Down);
    }
    let t = text(&a);
    assert!(
        t.contains("EH3D server (network)")
            && t.contains("Address")
            && t.contains("Token")
            && t.contains("Machine"),
        "{t}"
    );
    assert!(!t.contains("Port"), "{t}");
    // an address that is not host:port is refused with a sentence, and nothing is applied
    a.handle(Input::Tab);
    type_in(&mut a, "cm5");
    a.handle(Input::Tab);
    a.handle(Input::Tab);
    type_in(&mut a, "bench-1");
    a.handle(Input::Tab);
    a.handle(Input::Enter);
    assert!(
        text(&a).contains("The address must be host:port."),
        "{}",
        text(&a)
    );
    // the address is corrected (three places back from OK) and the window closes with OK
    for _ in 0..3 {
        a.handle(Input::BackTab);
    }
    type_in(&mut a, "192.168.0.180:18200");
    for _ in 0..3 {
        a.handle(Input::Tab);
    }
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(!t.contains("Connection with the machine"), "{t}");
    assert!(t.contains("EH3D server (network)"), "{t}");
    // nothing is written until the Config window itself is accepted; its OK button (the second to last place of the ring) writes the file
    assert!(!std::fs::read_to_string(&file).unwrap().contains("server"));
    for _ in 0..40 {
        a.handle(Input::BackTab);
        if text(&a).contains("Abort") && a.engine.connection().kind == Kind::Demo {
            // the focus walks the ring backwards: stop on the OK button
            break;
        }
    }
    a.click(44, 19);
    let saved = load(Some(&file)).unwrap();
    assert_eq!(
        (
            saved.kind,
            saved.server_address.as_str(),
            saved.server_machine.as_str()
        ),
        (Kind::Server, "192.168.0.180:18200", "bench-1")
    );
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.contains("# my machine") && text.contains("x_search_hz = 2700"),
        "{text}"
    );
    assert_eq!(a.engine.connection().kind, Kind::Server);
}

#[test]
fn the_machine_file_keeps_its_other_lines_when_a_connection_is_saved() {
    let dir = scratch("keep");
    let file = dir.join("machine.cfg");
    std::fs::write(
        &file,
        "# my machine\nconnection = demo\nx_search_hz = 2700\n",
    )
    .unwrap();
    let mut config = load(Some(&file)).unwrap();
    config.kind = Kind::Usb;
    config.usb_port = "COM8".into();
    eh3d_drillpro::protocol::connection::save_connection(Some(&file), &config).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.contains("# my machine")
            && text.contains("x_search_hz = 2700")
            && text.contains("usb_port = COM8"),
        "{text}"
    );
    let back = load(Some(&file)).unwrap();
    assert_eq!((back.kind, back.tuning == config.tuning), (Kind::Usb, true));
    let engine = ClassicApp::new(Lang::En, &dir);
    assert_eq!(engine.engine.connection().kind, Kind::Demo);
}
