use eh3d_drillpro::app::{App, Input};
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::screen::Screen;
use std::path::PathBuf;

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

fn render(app: &App) -> String {
    let mut s = Screen::default();
    app.render(&mut s);
    s.to_text()
}

fn loaded(lang: Lang) -> App {
    let mut app = App::new(lang, &data());
    app.load_file(&data().join("I8d.ncd"));
    app
}

#[test]
fn main_screen_shows_menu_and_status_in_english_by_default() {
    let app = App::new(Lang::default(), &data());
    let t = render(&app);
    assert!(t.lines().next().unwrap().contains("Load"), "{t}");
    assert!(t.contains("Quit"));
    assert!(t.contains("No file loaded"));
    assert!(t.contains("SIMULATION"));
}

#[test]
fn alt_hotkey_opens_vectors_with_hole_count() {
    let mut app = loaded(Lang::En);
    app.handle(Input::Alt('v'));
    let t = render(&app);
    assert!(t.contains("Vectors"));
    assert!(t.contains("Holes:"));
    assert!(t.contains("I8d.ncd"));
}

#[test]
fn every_language_renders_every_screen_without_panicking() {
    for lang in Lang::ALL {
        let mut app = loaded(lang);
        for hot in ['l', 'v', 't', 'c', 'o', 's', 'g', 'd', 'q'] {
            app.handle(Input::Alt(hot));
            let t = render(&app);
            assert!(t.lines().count() == 25);
            app.handle(Input::Esc);
        }
    }
}

#[test]
fn language_can_be_changed_from_the_menu() {
    let mut app = App::new(Lang::En, &data());
    app.handle(Input::Alt('g')); // Lan~g~uage
    app.handle(Input::Down); // Español
    app.handle(Input::Enter);
    assert_eq!(app.lang, Lang::Es);
    assert!(render(&app).lines().next().unwrap().contains("Cargar"));
}

#[test]
fn chinese_menu_contains_wide_characters() {
    let app = App::new(Lang::Zh, &data());
    let first = render(&app).lines().next().unwrap().to_string();
    assert!(first.contains("载入"), "{first}");
}

#[test]
fn start_runs_the_simulated_job_to_completion() {
    let mut app = loaded(Lang::En);
    app.handle(Input::Alt('s')); // ~S~tart
    let mut guard = 0;
    while app.tick(50) {
        guard += 1;
        assert!(guard < 1000);
    }
    let t = render(&app);
    assert!(t.contains("Done"), "{t}");
}

#[test]
fn quit_needs_confirmation() {
    let mut app = App::new(Lang::En, &data());
    app.handle(Input::Alt('q'));
    assert!(!app.quit);
    assert!(render(&app).contains("Do you really want to quit?"));
    app.handle(Input::Enter);
    assert!(app.quit);
}
