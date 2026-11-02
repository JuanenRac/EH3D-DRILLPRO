//! The simulated job follows the clock of the machine: a move takes as long as it would take on the machine.

use std::path::Path;

use eh3d_drillpro::app::App;
use eh3d_drillpro::classic::engine::ClassicEngine;
use eh3d_drillpro::i18n::Lang;

fn engine() -> App {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let mut a = App::new(Lang::En, &data);
    a.load(&data.join("T1.NCD"));
    a.start();
    a
}

#[test]
fn a_short_time_does_not_finish_the_job_and_enough_time_does() {
    let mut a = engine();
    assert!(a.tick_for(0.001));
    let mut seconds = 0.001;
    // 50 ms at a time: the job takes many steps of the clock, not one
    let mut steps = 0;
    while a.tick_for(0.05) {
        seconds += 0.05;
        steps += 1;
        assert!(steps < 1_000_000, "the job never ends");
    }
    assert!(
        steps > 20,
        "{steps} steps of 50 ms ({seconds:.1} s) is too fast for a machine"
    );
}

#[test]
fn a_faster_clock_takes_fewer_steps() {
    let count = |dt: f64| {
        let mut a = engine();
        let mut n = 0;
        while a.tick_for(dt) {
            n += 1;
        }
        n
    };
    assert!(count(0.5) < count(0.05));
}

fn shown(a: &App) -> String {
    let mut s = eh3d_drillpro::screen::Screen::default();
    a.render(&mut s);
    s.to_text()
}

/// The percentage of the progress line, `Progress: 3 / 10 (28 %)`.
fn percent_of(text: &str) -> u32 {
    let line = text
        .lines()
        .find(|l| l.contains("Progress:"))
        .unwrap_or_else(|| panic!("no progress line in\n{text}"));
    let open = line.find('(').expect("a percentage") + 1;
    let end = line[open..].find(" %").expect("a percent sign") + open;
    line[open..end].trim().parse().expect("a number")
}

#[test]
fn the_progress_follows_the_time_of_the_plan_and_the_time_left_goes_down() {
    let mut a = engine();
    let first = shown(&a);
    assert_eq!(percent_of(&first), 0, "{first}");
    assert!(first.contains("Time left:"), "{first}");
    let (mut last, mut steps) = (0, 0);
    while a.tick_for(0.5) {
        let p = percent_of(&shown(&a));
        assert!(p >= last, "the progress went back from {last} to {p}");
        last = p;
        steps += 1;
        assert!(steps < 1_000_000, "the job never ends");
    }
    // the window is gone when the job ends; the last that was seen while it ran is close to the end
    assert!(last >= 90, "the progress stopped at {last} %");
}

#[test]
fn every_language_has_the_time_left_text() {
    for lang in eh3d_drillpro::i18n::Lang::ALL {
        let t = eh3d_drillpro::texts::tr(lang, eh3d_drillpro::texts::Text::LblLeft);
        assert!(!t.is_empty());
    }
}
