//! The record of the jobs run directly on a machine from this computer (`eh3d-jobs.log` in the data folder): one line when a job starts and one when it ends, so after a job
//! that failed it is known which file it was, on which connection and at which item it stopped. A job on EH3D-SERVER is recorded by the server and not here.
//!
//! The lines are plain `key=value` text. The file is closed at `job_log_max_kb` and kept once as `eh3d-jobs.log.1`; `job_log = off` in the machine file writes nothing.
//! A mistake while writing (a full disk, a folder that cannot be written) is swallowed: the record never stops a job. No token or password is ever written.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::protocol::tuning::Tuning;

/// The name of the record; the older one is `eh3d-jobs.log.1`.
pub const FILE_NAME: &str = "eh3d-jobs.log";

/// What is known of a job before it starts.
#[derive(Debug, Clone)]
pub struct Record {
    path: PathBuf,
    max_bytes: u64,
    job: String,
    /// The item that ends each hole or path, in the order they are done.
    marks: Vec<usize>,
    unit: &'static str,
}

/// The facts of a job the front end gives to [`Record::new`].
#[derive(Debug, Clone)]
pub struct Facts<'a> {
    /// The file of the job; only its name, size and a hash of its content are written.
    pub file: &'a Path,
    pub offset_mm: (f64, f64),
    pub board_mm: f64,
    /// `holes` or `paths`.
    pub unit: &'static str,
    /// The item that ends each hole or path.
    pub marks: &'a [usize],
    /// Holes or paths left out of the job.
    pub left_out: usize,
    pub items: usize,
    pub estimate_s: f64,
}

impl Record {
    /// The record of a job, or `None` when the machine file turns it off.
    pub fn new(folder: &Path, tuning: &Tuning, facts: Facts<'_>) -> Option<Record> {
        if !tuning.job_log {
            return None;
        }
        let name = facts
            .file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let content = match std::fs::read(facts.file) {
            Ok(bytes) => format!("bytes={} hash={:016x}", bytes.len(), fnv1a(&bytes)),
            Err(_) => "bytes=? hash=?".to_string(),
        };
        let job = format!(
            "file={} {content} offset={:.3},{:.3} thickness={:.3} {}={} left_out={} items={} estimate_s={:.0}",
            quoted(&name),
            facts.offset_mm.0,
            facts.offset_mm.1,
            facts.board_mm,
            facts.unit,
            facts.marks.len(),
            facts.left_out,
            facts.items,
            facts.estimate_s,
        );
        Some(Record {
            path: folder.join(FILE_NAME),
            max_bytes: u64::from(tuning.job_log_max_kb) * 1024,
            job,
            marks: facts.marks.to_vec(),
            unit: facts.unit,
        })
    }

    /// Writes the line of the start and gives what writes the line of the end.
    pub fn begin(self, connection: &str) -> Open {
        let now = SystemTime::now();
        let run = now
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let program = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        self.write(&format!(
            "{} start run={run:x} program={} engine={} connection={} {}",
            utc(now),
            quoted(&program),
            env!("CARGO_PKG_VERSION"),
            quoted(connection),
            self.job
        ));
        Open {
            record: self,
            run,
            started: Instant::now(),
        }
    }

    fn write(&self, line: &str) {
        let full = std::fs::metadata(&self.path).is_ok_and(|m| m.len() >= self.max_bytes);
        if full {
            let mut old = self.path.clone().into_os_string();
            old.push(".1");
            let _ = std::fs::remove_file(&old);
            let _ = std::fs::rename(&self.path, &old);
        }
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            // one write, so two programs that share the folder do not cut each other's lines
            let _ = f.write_all(format!("{line}\n").as_bytes());
        }
    }
}

/// A job whose start is written and whose end is not.
#[derive(Debug)]
pub struct Open {
    record: Record,
    run: u128,
    started: Instant,
}

impl Open {
    /// `result` is `done`, `cancelled` or `failed`; `item` of `items` is where the job had got to.
    pub fn end(self, result: &str, item: usize, items: usize, reason: Option<&str>) {
        let done = self.record.marks.iter().filter(|&&end| end < item).count();
        let mut line = format!(
            "{} end run={:x} result={result} item={item}/{items} {}={done}/{} seconds={:.0}",
            utc(SystemTime::now()),
            self.run,
            self.record.unit,
            self.record.marks.len(),
            self.started.elapsed().as_secs_f64(),
        );
        if let Some(why) = reason {
            line.push_str(" reason=");
            line.push_str(&quoted(why));
        }
        self.record.write(&line);
    }
}

/// A value in double quotes, on one line whatever it holds.
fn quoted(value: &str) -> String {
    let clean: String = value
        .chars()
        .map(|c| match c {
            '"' => '\'',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    format!("\"{clean}\"")
}

/// FNV-1a of 64 bits: enough to tell whether two files are the same one, with no dependency. It is not a signature.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// `2026-01-31T23:59:59Z`.
fn utc(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, rest) = ((seconds / 86_400) as i64, seconds % 86_400);
    // the civil date of a number of days since 1970-01-01 (the calendar repeats every 400 years of 146097 days)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn folder(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("eh3d-runlog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn facts<'a>(file: &'a Path, marks: &'a [usize]) -> Facts<'a> {
        Facts {
            file,
            offset_mm: (1.5, 0.0),
            board_mm: 1.6,
            unit: "holes",
            marks,
            left_out: 2,
            items: 10,
            estimate_s: 61.4,
        }
    }

    #[test]
    fn a_job_leaves_a_start_line_and_an_end_line() {
        let d = folder("lines");
        let file = d.join("board.drl");
        std::fs::write(&file, b"abc").unwrap();
        let record = Record::new(&d, &Tuning::default(), facts(&file, &[2, 5, 8])).unwrap();
        record
            .begin("usb \"COM3\"\n")
            .end("failed", 6, 10, Some("running the job:\nno answer"));
        let text = std::fs::read_to_string(d.join(FILE_NAME)).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "{text}");
        assert!(lines[0].contains(" start run="), "{text}");
        assert!(lines[0].contains("connection=\"usb 'COM3' \""), "{text}");
        assert!(
            lines[0].contains("file=\"board.drl\" bytes=3 hash=e71fa2190541574b"),
            "{text}"
        );
        assert!(
            lines[0].contains(
                "offset=1.500,0.000 thickness=1.600 holes=3 left_out=2 items=10 estimate_s=61"
            ),
            "{text}"
        );
        assert!(
            lines[1].contains("result=failed item=6/10 holes=2/3"),
            "{text}"
        );
        assert!(
            lines[1].ends_with("reason=\"running the job: no answer\""),
            "{text}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn off_writes_nothing_and_a_full_file_is_kept_once() {
        let d = folder("full");
        let file = d.join("missing.drl");
        let off = Tuning {
            job_log: false,
            ..Tuning::default()
        };
        assert!(Record::new(&d, &off, facts(&file, &[])).is_none());
        let small = Tuning {
            job_log_max_kb: 16,
            ..Tuning::default()
        };
        std::fs::write(d.join(FILE_NAME), vec![b'x'; 16 * 1024]).unwrap();
        let record = Record::new(&d, &small, facts(&file, &[])).unwrap();
        record.begin("demo").end("done", 10, 10, None);
        let old = std::fs::metadata(d.join("eh3d-jobs.log.1")).unwrap();
        assert_eq!(old.len(), 16 * 1024);
        let text = std::fs::read_to_string(d.join(FILE_NAME)).unwrap();
        assert!(text.contains("bytes=? hash=?"), "{text}");
        assert!(text.contains("result=done item=10/10"), "{text}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_folder_that_cannot_be_written_is_not_an_error() {
        let d = folder("nowrite");
        let blocker = d.join("not-a-folder");
        std::fs::write(&blocker, b"").unwrap();
        let record = Record::new(&blocker, &Tuning::default(), facts(&blocker, &[])).unwrap();
        record.begin("demo").end("done", 0, 0, None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn dates_are_written_in_utc() {
        let at = |s| utc(UNIX_EPOCH + Duration::from_secs(s));
        assert_eq!(at(0), "1970-01-01T00:00:00Z");
        assert_eq!(at(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(at(1_893_455_999), "2029-12-31T23:59:59Z");
    }
}
