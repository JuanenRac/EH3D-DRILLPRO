//! The record of a crash. A window program on Windows has no console to show a panic, so where it happened is also appended to a file next to the settings.

use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// The file of the crash records: `eh3d-crash.log` in the configuration folder of the user (`%APPDATA%\EH3D`, `~/.config/eh3d`).
pub fn log_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(|a| PathBuf::from(a).join("EH3D"))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("eh3d")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("eh3d-crash.log")
}

/// The words of a panic, whatever it was raised with.
pub fn text_of(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown".to_string()
    }
}

/// From now on a panic is also appended to the crash log (seconds since 1970, the program, its version, the words and the place) before the usual message is given.
pub fn install(program: &'static str, version: &'static str) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let place = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = log_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(
                file,
                "[{secs}] {program} {version}: {} at {place}",
                text_of(info.payload())
            );
        }
        previous(info);
    }));
}
