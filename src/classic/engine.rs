//! What the classic screens need from the program behind them, so DRILLPRO and ROUTEPRO share one front end.

use std::path::{Path, PathBuf};

use crate::app::{App, Mode};
use crate::config::{Config, MachineSettings};
use crate::job::sort_holes;

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

pub trait ClassicEngine {
    fn kind(&self) -> Kind;
    fn lang(&self) -> crate::i18n::Lang;
    fn set_lang(&mut self, lang: crate::i18n::Lang);
    fn settings(&self) -> Option<&MachineSettings>;
    fn settings_mut(&mut self) -> Option<&mut MachineSettings>;
    fn config(&self) -> Option<&Config>;
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
    fn start(&mut self);
    /// True once the job moved past the homing.
    fn started(&self) -> bool;
    fn done(&self) -> usize;
    /// Advances the job; true while it goes on.
    fn tick(&mut self, n: usize) -> bool;
    fn stop(&mut self);
}

impl ClassicEngine for App {
    fn kind(&self) -> Kind {
        Kind::Drill
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
        self.entries
            .iter()
            .map(|e| (e.name.clone(), e.dir))
            .collect()
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
        let more = App::tick(self, n);
        if !more {
            self.mode = Mode::Main;
        }
        more
    }

    fn stop(&mut self) {
        self.run = None;
        self.mode = Mode::Main;
    }
}
