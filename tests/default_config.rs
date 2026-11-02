//! A folder with no machine file: the program starts from the file that comes with it, so the settings can be changed and saved.
use eh3d_drillpro::app::App;
use eh3d_drillpro::classic::engine::ClassicEngine;
use eh3d_drillpro::i18n::Lang;

fn empty_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d-drillpro-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_folder_without_a_machine_file_has_the_settings_of_the_one_that_comes_with_the_program() {
    let dir = empty_dir("nodcf");
    let mut app = App::new(Lang::En, &dir);
    let settings = app
        .settings_mut()
        .expect("the settings exist without a file");
    let before = settings.rack;
    settings.rack.clearance_mm += 2.0;
    assert_eq!(
        app.settings().unwrap().rack.clearance_mm,
        before.clearance_mm + 2.0
    );
}

#[test]
fn the_settings_changed_in_such_a_folder_are_written_and_read_back() {
    let dir = empty_dir("savedcf");
    let mut app = App::new(Lang::En, &dir);
    app.settings_mut().unwrap().rack.clearance_mm = 9.0;
    app.settings_changed();
    app.save_config().expect("the machine file is written");
    assert!(dir.join("DRILLPRO.DCF").exists());
    let again = App::new(Lang::En, &dir);
    assert_eq!(again.settings().unwrap().rack.clearance_mm, 9.0);
}
