//! A panic in the thread of the work is an error that is shown, and the crash log is where the panic is told.

use eh3d_drillpro::crash::{log_path, text_of};
use eh3d_drillpro::runner::caught;

#[test]
fn a_panic_in_the_work_becomes_an_error_that_names_it() {
    let r: Result<(), String> = caught(|| panic!("the table of tools is damaged"));
    assert_eq!(
        r.unwrap_err(),
        "internal error: the table of tools is damaged"
    );
    let r: Result<u32, String> = caught(|| Ok(7));
    assert_eq!(r.unwrap(), 7);
    let r: Result<(), String> = caught(|| Err("a plain error".into()));
    assert_eq!(r.unwrap_err(), "a plain error");
}

#[test]
fn the_words_of_a_panic_are_read_from_a_text_or_a_string() {
    let a: Box<dyn std::any::Any + Send> = Box::new("one");
    let b: Box<dyn std::any::Any + Send> = Box::new(String::from("two"));
    let c: Box<dyn std::any::Any + Send> = Box::new(3_u8);
    assert_eq!(text_of(&*a), "one");
    assert_eq!(text_of(&*b), "two");
    assert_eq!(text_of(&*c), "unknown");
}

#[test]
fn the_crash_log_is_in_the_configuration_folder() {
    assert_eq!(log_path().file_name().unwrap(), "eh3d-crash.log");
}
