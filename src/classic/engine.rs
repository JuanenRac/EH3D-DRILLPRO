//! What the classic screens need from the program behind them, so DRILLPRO and ROUTEPRO share one front end.

use std::path::{Path, PathBuf};

use crate::app::{App, Mode};
use crate::config::{self, Config, MachineSettings};
use crate::job::{geometry_from, sort_holes, Sorting};
pub use crate::job::{JobOptions, Panel};
use crate::tools::{self, Tool};

/// Which of the two programs the screens are for: they differ in the tool table and in one box of the Parameter dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Drill,
    Route,
}

/// One place of the tool table as the Tool Data dialog shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolRow {
    /// Name of the tool in the place (`--` when the place is free).
    pub name: Option<u8>,
    /// Texts of the editable columns: stroke, feed and spindle for drilling; depth, iterations, X feed, Z feed and spindle for routing.
    pub cells: Vec<String>,
    /// Number of the first vector that uses the tool (0 when none).
    pub first_vector: usize,
    /// The tool was found in the tool table file.
    pub in_table: bool,
}

/// What the job window shows: where the job is, and what the tool is doing (the values are the planned ones).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JobView {
    /// How far the job has gone, in percent.
    pub percent: u8,
    pub tool: Option<u8>,
    /// Stroke of the tool: board thickness plus the extra stroke of its table entry.
    pub stroke_mm: f64,
    pub x_mm: f64,
    pub y_mm: f64,
    pub z_feed: u32,
    pub rpm: u32,
    /// Vectors done, and drilled depth so far in mm.
    pub done: usize,
    pub used_mm: f64,
    /// Vectors left out because they fall outside the working area.
    pub skipped: usize,
    /// Length of the run the current tool makes from here on, in mm (what the tool change window tells).
    pub run_mm: f64,
}

/// What the job waits for the operator to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    /// Put this tool in and confirm.
    ToolChange(u8),
    /// Confirm that the spindle may start.
    SpindleStart,
}

/// What the About dialog says about the program (the texts around it belong to the front end).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct About {
    pub name: &'static str,
    pub version: &'static str,
    pub repository: &'static str,
}

pub trait ClassicEngine {
    fn kind(&self) -> Kind;
    fn about(&self) -> About;
    /// Chooses how the job reaches the machine (see the connection file).
    fn set_connection(&mut self, connection: crate::protocol::connection::ConnectionConfig);
    /// Shows a problem found at start-up on the main screen.
    fn set_error(&mut self, message: String);
    /// The Offset Teach window moves the head by these steps of 1 mil (0.001 inch) in X, Y and Z (Z positive = down). The
    /// simulation only follows the position on the screen; an engine on a real machine moves it.
    fn jog(&mut self, _dx: i32, _dy: i32, _dz: i32) {}
    /// The Offset Teach window opened: an engine on a real machine homes it and follows the jogs from now on.
    fn teach_begin(&mut self) {}
    /// The window closed: the machine goes home again and the drivers go off.
    fn teach_end(&mut self) {}
    /// How the machine is doing, when the window moves a real one.
    fn teach_state(&self) -> Option<crate::runner::TeachState> {
        None
    }
    fn lang(&self) -> crate::i18n::Lang;
    fn set_lang(&mut self, lang: crate::i18n::Lang);
    fn settings(&self) -> Option<&MachineSettings>;
    fn settings_mut(&mut self) -> Option<&mut MachineSettings>;
    fn config(&self) -> Option<&Config>;
    fn config_mut(&mut self) -> Option<&mut Config>;
    /// Tells the engine that the settings changed (the job is planned with them).
    fn settings_changed(&mut self);
    /// The folder of the configuration and the tool table.
    fn data_dir(&self) -> PathBuf;
    /// Writes the configuration file (the old one stays as `.BAK`); returns where it went.
    fn save_config(&mut self) -> Result<PathBuf, String>;
    /// Reads the configuration file again.
    fn reload_config(&mut self) -> Result<(), String>;
    /// The board thickness the job is planned with.
    fn set_board(&mut self, mm: f64);
    fn load(&mut self, path: &Path);
    fn file_loaded(&self) -> bool;
    /// Name of the loaded file in capitals, as the original shows it.
    fn file_name(&self) -> Option<String>;
    /// Number of vectors (holes, or paths) in the file.
    fn vectors(&self) -> usize;
    /// Bounding box of the vectors in mm: (min x, min y, max x, max y).
    fn bounds(&self) -> Option<(f64, f64, f64, f64)>;
    fn error(&self) -> Option<&str>;
    fn dir(&self) -> PathBuf;
    fn set_dir(&mut self, dir: PathBuf);
    /// File name pattern of the Select File dialog, for example `*.NCD`.
    fn pattern(&self) -> &'static str;
    /// The entries of the folder: `..` first, then folders, then files (name, is a folder).
    fn entries(&mut self) -> Vec<(String, bool)>;
    fn tool_rows(&self) -> Vec<ToolRow>;
    /// Takes the edited places into the tool table (the tools that the places name).
    fn apply_tool_rows(&mut self, rows: &[ToolRow]);
    fn save_tools(&mut self) -> Result<PathBuf, String>;
    fn reload_tools(&mut self) -> Result<(), String>;
    /// Loads the tool table from another file (the Load button of the Tool Data window).
    fn load_tools_from(&mut self, _path: &Path) -> Result<(), String> {
        Err("not supported".into())
    }
    /// Loads the machine configuration from another file (the Load button of the Parameter window).
    fn load_config_from(&mut self, _path: &Path) -> Result<(), String> {
        Err("not supported".into())
    }
    /// What the main form and the Panelization dialog add to the next job.
    fn set_job_options(&mut self, options: JobOptions);
    /// The job window's figures; `None` when no job is running.
    fn job_view(&self) -> Option<JobView>;
    /// The dialog the job is waiting at, if any.
    fn question(&self) -> Option<Question>;
    /// Answers it: Ok goes on, Abort stops the job.
    fn answer(&mut self, ok: bool);
    fn start(&mut self);
    /// True once the job moved past the homing.
    fn started(&self) -> bool;
    fn done(&self) -> usize;
    /// Advances the job; true while it goes on.
    fn tick(&mut self, n: usize) -> bool;
    /// Advances the simulated job by this many seconds of machine time (the moves take as long as the machine would take, with the feeds
    /// and the speeds of the tools); true while it goes on. An engine without a clock does one step.
    fn tick_for(&mut self, _seconds: f64) -> bool {
        self.tick(1)
    }
    fn stop(&mut self);
}

impl ClassicEngine for App {
    fn kind(&self) -> Kind {
        Kind::Drill
    }

    fn jog(&mut self, dx: i32, dy: i32, dz: i32) {
        const MIL_MM: f64 = 0.0254;
        if let Some(t) = &self.teach {
            t.jog(dx as f64 * MIL_MM, dy as f64 * MIL_MM, dz as f64 * MIL_MM);
        }
    }

    fn teach_begin(&mut self) {
        self.teach = self.connection.is_real().then(|| {
            crate::runner::TeachSession::start(self.connection.clone(), self.job_machine())
        });
    }

    fn teach_end(&mut self) {
        if let Some(t) = &self.teach {
            t.end();
        }
    }

    fn teach_state(&self) -> Option<crate::runner::TeachState> {
        self.teach.as_ref().map(|t| t.state())
    }

    fn about(&self) -> About {
        About {
            name: "EH3D-DRILLPRO",
            version: env!("CARGO_PKG_VERSION"),
            repository: env!("CARGO_PKG_REPOSITORY"),
        }
    }

    fn set_connection(&mut self, connection: crate::protocol::connection::ConnectionConfig) {
        App::set_connection(self, connection);
    }

    fn set_error(&mut self, message: String) {
        App::set_error(self, message);
    }

    fn lang(&self) -> crate::i18n::Lang {
        self.lang
    }

    fn set_lang(&mut self, lang: crate::i18n::Lang) {
        self.lang = lang;
    }

    fn settings(&self) -> Option<&MachineSettings> {
        self.settings.as_ref()
    }

    fn settings_mut(&mut self) -> Option<&mut MachineSettings> {
        self.settings.as_mut()
    }

    fn config(&self) -> Option<&Config> {
        self.config.as_ref()
    }

    fn config_mut(&mut self) -> Option<&mut Config> {
        self.config.as_mut()
    }

    fn settings_changed(&mut self) {
        self.geometry = geometry_from(self.settings.as_ref());
        if let Some(m) = &self.settings {
            self.sorting = match m.sorting {
                config::SortMode::None => Sorting::None,
                config::SortMode::XFirst => Sorting::XFirst,
                config::SortMode::YFirst => Sorting::YFirst,
                config::SortMode::Special => Sorting::Special,
            };
            self.board_mm = m.board_mm;
        }
    }

    fn data_dir(&self) -> PathBuf {
        self.data_dir.clone()
    }

    fn save_config(&mut self) -> Result<PathBuf, String> {
        let (Some(text), Some(settings), Some(cfg)) = (
            self.config_text.as_deref(),
            self.settings.as_ref(),
            self.config.as_ref(),
        ) else {
            return Err("no configuration file".into());
        };
        let out = config::rewrite(
            text,
            settings,
            &cfg.directory,
            &cfg.default_extension,
            false,
        );
        let path = self.data_dir.join("DRILLPRO.DCF");
        config::write_with_backup(&path, &out)?;
        self.config = config::parse(&out);
        self.config_text = Some(out);
        Ok(path)
    }

    fn reload_config(&mut self) -> Result<(), String> {
        let path = self.data_dir.join("DRILLPRO.DCF");
        self.load_config_from(&path)
    }

    fn load_config_from(&mut self, path: &std::path::Path) -> Result<(), String> {
        let text = config::bytes_to_text(&std::fs::read(path).map_err(|e| e.to_string())?);
        let cfg =
            config::parse(&text).ok_or("the configuration file is not in the expected layout")?;
        self.settings = cfg.settings();
        self.config = Some(cfg);
        self.config_text = Some(text);
        self.settings_changed();
        Ok(())
    }

    fn set_board(&mut self, mm: f64) {
        self.board_mm = mm;
    }

    fn load(&mut self, path: &Path) {
        self.load_file(path);
    }

    fn file_loaded(&self) -> bool {
        self.loaded.is_some()
    }

    fn file_name(&self) -> Option<String> {
        self.loaded
            .as_ref()
            .and_then(|l| l.path.file_name())
            .map(|n| n.to_string_lossy().to_ascii_uppercase())
    }

    fn vectors(&self) -> usize {
        self.loaded.as_ref().map_or(0, |l| l.file.holes.len())
    }

    fn bounds(&self) -> Option<(f64, f64, f64, f64)> {
        let l = self.loaded.as_ref()?;
        crate::job::bounds_mm(&sort_holes(&l.file, self.sorting))
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn dir(&self) -> PathBuf {
        self.dir.clone()
    }

    fn set_dir(&mut self, dir: PathBuf) {
        self.dir = dir;
    }

    fn pattern(&self) -> &'static str {
        "*.NCD"
    }

    fn entries(&mut self) -> Vec<(String, bool)> {
        self.refresh_entries();
        dos_order(
            self.entries
                .iter()
                .filter(|e| !e.name.starts_with('.') || e.name == "..")
                .map(|e| (e.name.clone(), e.dir))
                .collect(),
        )
    }

    fn tool_rows(&self) -> Vec<ToolRow> {
        let used: Vec<u8> = self
            .loaded
            .as_ref()
            .map(|l| l.file.tools.clone())
            .unwrap_or_default();
        (0..15)
            .map(|i| {
                let name = used.get(i).copied();
                let tool = name.and_then(|n| self.tools.iter().find(|t| t.number == n));
                let (stroke, feed, rpm) = tool.map_or((0.0, 50, 30000), |t| {
                    (t.extra_stroke_mm, t.feed_mm_min, t.spindle_rpm)
                });
                ToolRow {
                    name,
                    cells: vec![
                        format!("{stroke:.2}"),
                        format!("{feed:04}"),
                        format!("{rpm:05}"),
                    ],
                    first_vector: name
                        .and_then(|n| {
                            self.loaded
                                .as_ref()
                                .and_then(|l| l.file.holes.iter().position(|h| h.tool == n))
                        })
                        .map_or(0, |p| p + 1),
                    in_table: tool.is_some(),
                }
            })
            .collect()
    }

    fn apply_tool_rows(&mut self, rows: &[ToolRow]) {
        for row in rows {
            let (Some(n), [stroke, feed, rpm]) = (row.name, row.cells.as_slice()) else {
                continue;
            };
            let parsed = (
                stroke.trim().parse::<f64>(),
                feed.trim().parse::<u32>(),
                rpm.trim().parse::<u32>(),
            );
            let (Ok(extra_stroke_mm), Ok(feed_mm_min), Ok(spindle_rpm)) = parsed else {
                continue;
            };
            let tool = Tool {
                number: n,
                extra_stroke_mm,
                feed_mm_min,
                spindle_rpm,
            };
            match self.tools.iter_mut().find(|t| t.number == n) {
                Some(t) => *t = tool,
                None => self.tools.push(tool),
            }
        }
    }

    fn save_tools(&mut self) -> Result<PathBuf, String> {
        let original = self.tools_text.clone().unwrap_or_default();
        let out = tools::rewrite(&original, &self.tools);
        let path = self.data_dir.join("DRILLPRO.DTD");
        config::write_with_backup(&path, &out)?;
        self.tools_text = Some(out);
        self.tools_found = true;
        Ok(path)
    }

    fn reload_tools(&mut self) -> Result<(), String> {
        let path = self.data_dir.join("DRILLPRO.DTD");
        self.load_tools_from(&path)
    }

    fn load_tools_from(&mut self, path: &std::path::Path) -> Result<(), String> {
        let text = config::bytes_to_text(&std::fs::read(path).map_err(|e| e.to_string())?);
        self.tools = tools::parse(&text);
        self.tools_text = Some(text);
        self.tools_found = true;
        Ok(())
    }

    fn set_job_options(&mut self, options: JobOptions) {
        self.job_options = options;
    }

    fn job_view(&self) -> Option<JobView> {
        let run = self.run.as_ref()?;
        let offset = self.settings.as_ref().map_or((0.0, 0.0), |m| m.offset_mm);
        let at = run.done.min(run.plan.len().saturating_sub(1));
        let hole = run.plan.get(at)?;
        let tool = self.tools.iter().find(|t| t.number == hole.tool);
        let stroke_of = |n: u8| {
            self.board_mm
                + self
                    .tools
                    .iter()
                    .find(|t| t.number == n)
                    .map_or(0.0, |t| t.extra_stroke_mm)
        };
        let used: f64 = run
            .plan
            .iter()
            .take(run.done)
            .map(|h| stroke_of(h.tool))
            .sum();
        // the length of the run the tool makes from here on
        let mut run_mm = 0.0;
        let mut prev = (hole.x_mm, hole.y_mm);
        for h in run.plan[at..].iter().take_while(|h| h.tool == hole.tool) {
            run_mm += ((h.x_mm - prev.0).powi(2) + (h.y_mm - prev.1).powi(2)).sqrt();
            prev = (h.x_mm, h.y_mm);
        }
        Some(JobView {
            percent: (run.done * 100 / run.plan.len().max(1)).min(100) as u8,
            tool: Some(hole.tool),
            stroke_mm: stroke_of(hole.tool),
            x_mm: hole.x_mm + offset.0,
            y_mm: hole.y_mm + offset.1,
            z_feed: tool.map_or(50, |t| t.feed_mm_min),
            rpm: tool.map_or(30000, |t| t.spindle_rpm),
            done: run.done,
            used_mm: used,
            skipped: run.skipped,
            run_mm,
        })
    }

    fn question(&self) -> Option<Question> {
        use crate::protocol::job::Pause;
        let run = self.run.as_ref()?;
        let pause = match run.real.as_ref().map(|r| r.state()) {
            Some(crate::runner::RunState::WaitPause(p)) => Some(p),
            Some(_) => None,
            None => run.waiting,
        }?;
        Some(match pause {
            Pause::ToolChange { tool } => Question::ToolChange(tool),
            Pause::SpindleStart => Question::SpindleStart,
        })
    }

    fn answer(&mut self, ok: bool) {
        let Some(run) = self.run.as_mut() else { return };
        if let Some(real) = &run.real {
            if ok {
                real.confirm();
            } else {
                real.cancel();
            }
            return;
        }
        if ok {
            run.waiting = None;
        } else {
            self.stop();
        }
    }

    fn start(&mut self) {
        App::start(self);
    }

    fn started(&self) -> bool {
        self.run.as_ref().is_some_and(|r| r.next > 0)
    }

    fn done(&self) -> usize {
        self.run.as_ref().map_or(0, |r| r.done)
    }

    fn tick(&mut self, n: usize) -> bool {
        let was = self.mode == Mode::Running;
        let more = App::tick(self, n);
        if !more {
            if was {
                self.play_finish_sound();
            }
            self.mode = Mode::Main;
        }
        more
    }

    fn tick_for(&mut self, seconds: f64) -> bool {
        let was = self.mode == Mode::Running;
        let more = self.tick_seconds(seconds);
        if !more {
            if was {
                self.play_finish_sound();
            }
            self.mode = Mode::Main;
        }
        more
    }

    fn stop(&mut self) {
        self.run = None;
        self.mode = Mode::Main;
    }
}

/// The entries of a folder in the order the original lists them: the files, then the folders, then `..` last.
pub fn dos_order(entries: Vec<(String, bool)>) -> Vec<(String, bool)> {
    let key = |s: &str| s.to_lowercase();
    let mut files: Vec<(String, bool)> = entries.iter().filter(|(_, d)| !d).cloned().collect();
    let mut dirs: Vec<(String, bool)> = entries
        .iter()
        .filter(|(n, d)| *d && n != "..")
        .cloned()
        .collect();
    let parent: Vec<(String, bool)> = entries.iter().filter(|(n, _)| n == "..").cloned().collect();
    files.sort_by_key(|(n, _)| key(n));
    dirs.sort_by_key(|(n, _)| key(n));
    files.extend(dirs);
    files.extend(parent);
    files
}

/// A path made absolute (and without the `\\?\` prefix Windows adds), so that going up always has somewhere to go.
pub fn absolute(p: &Path) -> PathBuf {
    let a = std::fs::canonicalize(p)
        .or_else(|_| std::path::absolute(p))
        .unwrap_or_else(|_| p.to_path_buf());
    let text = a.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => a,
    }
}

/// The drives of the computer as `[-C-]` entries (Windows; none elsewhere), to move from one to another.
pub fn drive_entries() -> Vec<(String, bool)> {
    if !cfg!(windows) {
        return Vec::new();
    }
    ('C'..='Z')
        .filter(|c| Path::new(&format!("{c}:\\")).exists())
        .map(|c| (format!("[-{c}-]"), true))
        .collect()
}

/// The folder a `[-C-]` entry stands for.
pub fn drive_path(name: &str) -> Option<PathBuf> {
    let letter = name.strip_prefix("[-")?.strip_suffix("-]")?;
    (letter.len() == 1).then(|| PathBuf::from(format!("{letter}:\\")))
}
