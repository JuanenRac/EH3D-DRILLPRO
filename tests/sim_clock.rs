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
