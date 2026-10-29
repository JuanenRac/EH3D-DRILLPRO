//! Application state and text-mode rendering, independent of any windowing toolkit.

use std::path::{Path, PathBuf};

use crate::config::{self, Config};
use crate::i18n::{str_width, Lang};
use crate::job::{bounds_mm, sort_holes, travel_mm, Geometry, Sorting};
use crate::ncd::{self, DrillFile};
use crate::protocol::connection::{ConnectionConfig, Kind as ConnectionKind};
use crate::protocol::job::{plan_drill_job, Item, JobHole, JobMachine, JobTool, Pause};
use crate::protocol::latches::run_ops;
use crate::protocol::{bits, connect, run_steps, Simulator};
use crate::runner::{RealRun, RunState};
use crate::screen::*;
use crate::textapp::TextApp;
use crate::texts::{tr, Text};
use crate::tools::{self, Tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Char(char),
    Alt(char),
    Enter,
    Esc,
    Up,
    Down,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Mode {
    Main,
    FileDialog,
    Vectors,
    Tools,
    Config,
    Sort,
    Language,
    Running,
    ConfirmQuit,
}

pub(crate) struct Loaded {
    pub(crate) path: PathBuf,
    pub(crate) file: DrillFile,
}

pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) dir: bool,
}

pub(crate) struct Run {
    pub(crate) items: Vec<Item>,
    pub(crate) next: usize,
    pub(crate) total_holes: usize,
    /// Holes whose stroke has been completed.
    pub(crate) done: usize,
    pub(crate) pending_descent: bool,
    pub(crate) sim: Simulator,
    pub(crate) connected: bool,
    /// The job on a real machine (a connection other than `demo`); the simulator above is then unused.
    pub(crate) real: Option<RealRun>,
    /// Index of the item that finishes each hole, to count the holes done from the real progress.
    pub(crate) hole_ends: Vec<usize>,
}

pub struct App {
    pub lang: Lang,
    pub(crate) mode: Mode,
    pub(crate) dir: PathBuf,
    pub(crate) entries: Vec<Entry>,
    pub(crate) sel: usize,
    pub(crate) loaded: Option<Loaded>,
    pub(crate) error: Option<String>,
    pub(crate) tools: Vec<Tool>,
    pub(crate) tools_found: bool,
    pub(crate) config: Option<Config>,
    pub(crate) settings: Option<crate::config::MachineSettings>,
    pub(crate) sorting: Sorting,
    pub(crate) geometry: Geometry,
    pub(crate) run: Option<Run>,
    pub(crate) board_mm: f64,
    pub(crate) connection: ConnectionConfig,
    /// The folder of `DRILLPRO.DTD` and `DRILLPRO.DCF`, and the text of both files as they were read (to write them back).
    pub(crate) data_dir: PathBuf,
    pub(crate) config_text: Option<String>,
    pub(crate) tools_text: Option<String>,
    pub quit: bool,
}

const MENU_BG: u8 = LIGHT_GRAY;
const WIN_BG: u8 = CYAN;

impl App {
    /// `data_dir` is where `DRILLPRO.DTD` and `DRILLPRO.DCF` are looked up.
    pub fn new(lang: Lang, data_dir: &Path) -> Self {
        let read = |name: &str| {
            std::fs::read(data_dir.join(name))
                .ok()
                .map(|b| b.iter().map(|&c| c as char).collect::<String>())
        };
        let tool_text = read("DRILLPRO.DTD");
        let config_text = read("DRILLPRO.DCF");
        let config = config_text.as_deref().and_then(config::parse);
        // The machine calibration (correction coefficients) comes from the configuration file.
        let settings = config.as_ref().and_then(|c| c.settings());
        let geometry = crate::job::geometry_from(settings.as_ref());
        // The sorting mode and the board thickness of the job also come from the configuration.
        let sorting = settings
            .as_ref()
            .map_or(Sorting::Special, |m| match m.sorting {
                config::SortMode::None => Sorting::None,
                config::SortMode::XFirst => Sorting::XFirst,
                config::SortMode::YFirst => Sorting::YFirst,
                config::SortMode::Special => Sorting::Special,
            });
        let board_mm = settings.as_ref().map_or(1.0, |m| m.board_mm);
        let mut app = App {
            lang,
            mode: Mode::Main,
            dir: data_dir.to_path_buf(),
            entries: Vec::new(),
            sel: 0,
            loaded: None,
            error: None,
            tools_found: tool_text.is_some(),
            tools: tool_text.as_deref().map(tools::parse).unwrap_or_default(),
            data_dir: data_dir.to_path_buf(),
            config_text,
            tools_text: tool_text,
            settings,
            config: config.clone(),
            sorting,
            geometry,
            run: None,
            board_mm,
            connection: ConnectionConfig::default(),
            quit: false,
        };
        app.dir = crate::config::work_dir(data_dir, app.config.as_ref());
        app.refresh_entries();
        app
    }

    pub fn load_file(&mut self, path: &Path) {
        let text: String = match std::fs::read(path) {
            Ok(b) => b.iter().map(|&c| c as char).collect(),
            Err(e) => {
                self.error = Some(format!("{}: {e}", tr(self.lang, Text::ErrRead)));
                return;
            }
        };
        match ncd::parse(&text) {
            Ok(file) => {
                self.error = None;
                self.loaded = Some(Loaded {
                    path: path.to_path_buf(),
                    file,
                });
            }
            Err(e) => self.error = Some(format!("{}: {e}", tr(self.lang, Text::ErrRead))),
        }
    }

    pub(crate) fn refresh_entries(&mut self) {
        let mut v = vec![Entry {
            name: "..".into(),
            dir: true,
        }];
        if let Ok(rd) = std::fs::read_dir(&self.dir) {
            let mut items: Vec<Entry> = rd
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let dir = e.path().is_dir();
                    let ok = dir || name.to_ascii_lowercase().ends_with(".ncd");
                    ok.then_some(Entry { name, dir })
                })
                .collect();
            items.sort_by(|a, b| {
                b.dir
                    .cmp(&a.dir)
                    .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            v.extend(items);
        }
        self.entries = v;
        self.sel = 0;
    }

    fn menu_items(&self) -> [(Text, Mode); 8] {
        [
            (Text::MenuLoad, Mode::FileDialog),
            (Text::MenuVectors, Mode::Vectors),
            (Text::MenuTools, Mode::Tools),
            (Text::MenuConfig, Mode::Config),
            (Text::MenuSort, Mode::Sort),
            (Text::MenuStart, Mode::Running),
            (Text::MenuLanguage, Mode::Language),
            (Text::MenuQuit, Mode::ConfirmQuit),
        ]
    }

    fn hotkey(&self, t: Text) -> char {
        tr(self.lang, t)
            .split('~')
            .nth(1)
            .and_then(|s| s.chars().next())
            .map_or('\0', |c| c.to_ascii_lowercase())
    }

    /// Menu bar positions: `(x, width, text)` for each item.
    pub fn menu_layout(&self) -> Vec<(usize, usize, Text)> {
        let mut x = 1;
        self.menu_items()
            .iter()
            .map(|(t, _)| {
                let w = str_width(tr(self.lang, *t)) + 2;
                let r = (x, w, *t);
                x += w;
                r
            })
            .collect()
    }

    pub fn click(&mut self, col: usize, row: usize) {
        if row != 0 {
            return;
        }
        let hit = self
            .menu_layout()
            .into_iter()
            .find(|&(x, w, _)| col >= x && col < x + w)
            .map(|(_, _, t)| t);
        if let Some(t) = hit {
            self.open(t);
        }
    }

    fn open(&mut self, t: Text) {
        let target = self
            .menu_items()
            .into_iter()
            .find(|(k, _)| *k == t)
            .map(|(_, m)| m);
        match target {
            Some(Mode::FileDialog) => {
                self.refresh_entries();
                self.mode = Mode::FileDialog;
            }
            Some(Mode::Running) => self.start(),
            Some(Mode::Sort) => {
                self.sel = self.sorting_index();
                self.mode = Mode::Sort;
            }
            Some(Mode::Language) => {
                self.sel = self.lang as usize;
                self.mode = Mode::Language;
            }
            Some(m) => self.mode = m,
            None => {}
        }
    }

    /// Chooses how the job reaches the machine (see the connection file). `demo` runs on the simulator.
    pub fn set_connection(&mut self, connection: ConnectionConfig) {
        self.connection = connection;
    }

    /// Shows a problem found at start-up (for example a mistake in the connection file) on the main screen.
    pub fn set_error(&mut self, message: String) {
        self.error = Some(message);
    }

    /// Items that finish a hole: a long Z descent followed by a long ascent.
    fn hole_end_indexes(items: &[Item]) -> Vec<usize> {
        crate::job::hole_end_indexes(items)
    }

    fn sorting_index(&self) -> usize {
        [
            Sorting::None,
            Sorting::XFirst,
            Sorting::YFirst,
            Sorting::Special,
        ]
        .iter()
        .position(|&s| s == self.sorting)
        .unwrap_or(0)
    }

    pub(crate) fn start(&mut self) {
        let Some(l) = &self.loaded else {
            self.mode = Mode::FileDialog;
            self.refresh_entries();
            return;
        };
        let holes = sort_holes(&l.file, self.sorting);
        let unit_mm = self
            .settings
            .as_ref()
            .map_or(crate::ncd::MIL_MM, |m| m.unit.mm());
        let scale = self.settings.as_ref().map_or((1.0, 1.0), |m| m.scale);
        let job_holes: Vec<JobHole> = holes
            .iter()
            .map(|h| JobHole {
                tool: h.tool,
                x_mm: h.x_mil as f64 * unit_mm * scale.0,
                y_mm: h.y_mil as f64 * unit_mm * scale.1,
            })
            .collect();
        let tools: Vec<JobTool> = self
            .tools
            .iter()
            .map(|t| JobTool {
                number: t.number,
                extra_stroke_mm: t.extra_stroke_mm,
                feed_mm_min: t.feed_mm_min,
                spindle_rpm: t.spindle_rpm,
            })
            .collect();
        let items = plan_drill_job(&self.job_machine(), &tools, &job_holes);
        let hole_ends = Self::hole_end_indexes(&items);
        let mut sim = Simulator::default();
        let (connected, real) = if self.connection.kind == ConnectionKind::Server {
            // the job goes to the EH3D server, which plans it again with the machine's constants and runs it on the machine's node
            let name = l
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let request = serde_json::json!({
                "kind": "drill",
                "name": name,
                "board_mm": self.board_mm,
                "tools": tools.iter().map(|t| serde_json::json!({
                    "number": t.number,
                    "extra_stroke_mm": t.extra_stroke_mm,
                    "feed_mm_min": t.feed_mm_min,
                    "spindle_rpm": t.spindle_rpm,
                })).collect::<Vec<_>>(),
                "holes": job_holes.iter().map(|h| serde_json::json!({
                    "tool": h.tool, "x_mm": h.x_mm, "y_mm": h.y_mm,
                })).collect::<Vec<_>>(),
            });
            (
                true,
                Some(RealRun::start_on_server(
                    self.connection.clone(),
                    request,
                    hole_ends.clone(),
                    items.len(),
                )),
            )
        } else if self.connection.is_real() {
            (
                true,
                Some(RealRun::start(self.connection.clone(), items.clone())),
            )
        } else {
            (connect(&mut sim).is_ok(), None)
        };
        self.run = Some(Run {
            items,
            next: 0,
            total_holes: holes.len(),
            done: 0,
            pending_descent: false,
            sim,
            connected,
            real,
            hole_ends,
        });
        self.mode = Mode::Running;
    }

    /// Machine data for the job: from the configuration file when it was read, defaults otherwise.
    fn job_machine(&self) -> JobMachine {
        crate::job::job_machine(
            self.settings.as_ref(),
            self.geometry,
            &self.connection,
            self.board_mm,
        )
    }

    /// Advances the simulated job by up to `n` holes. Returns true while work remains.
    pub fn tick(&mut self, n: usize) -> bool {
        if self.mode != Mode::Running {
            return false;
        }
        let Some(run) = self.run.as_mut() else {
            return false;
        };
        if let Some(real) = &run.real {
            let (next, _) = real.progress();
            run.next = next;
            run.done = run.hole_ends.iter().filter(|&&e| e < next).count();
            return !real.finished();
        }
        for _ in 0..n {
            let Some(item) = run.items.get(run.next) else {
                return false;
            };
            match item {
                Item::Steps(steps) | Item::Search(steps) | Item::Seek(steps) => {
                    let first = steps.first().map(|x| x.byte);
                    let _ = run_steps(&mut run.sim, steps);
                    // A long Z descent followed by a long ascent is one finished hole.
                    if first == Some(bits::Z_POS) && steps.len() >= 100 {
                        run.pending_descent = true;
                    } else if first == Some(bits::Z_NEG)
                        && steps.len() >= 100
                        && run.pending_descent
                    {
                        run.done += 1;
                        run.pending_descent = false;
                    }
                }
                Item::Latch(ops) => {
                    let _ = run_ops(&mut run.sim, ops);
                }
                // The operator dialogs are confirmed automatically in the simulation; there is nothing to unstick, and the simulator has no spindle.
                Item::Pause(_) | Item::Unstick(_) | Item::Spindle { .. } => {}
            }
            run.next += 1;
        }
        run.next < run.items.len()
    }

    pub fn handle(&mut self, input: Input) {
        if let Input::Alt(c) = input {
            let c = c.to_ascii_lowercase();
            let hit = self
                .menu_items()
                .into_iter()
                .find(|(t, _)| self.hotkey(*t) == c)
                .map(|(t, _)| t);
            if let Some(t) = hit {
                self.open(t);
            }
            return;
        }
        match self.mode.clone() {
            Mode::Main => {}
            Mode::FileDialog => self.key_file_dialog(input),
            Mode::Sort => self.key_list(input, 4, |a, i| {
                a.sorting = [
                    Sorting::None,
                    Sorting::XFirst,
                    Sorting::YFirst,
                    Sorting::Special,
                ][i];
            }),
            Mode::Language => self.key_list(input, Lang::ALL.len(), |a, i| a.lang = Lang::ALL[i]),
            Mode::ConfirmQuit => match input {
                Input::Enter | Input::Char('y' | 'Y' | 's' | 'S' | 'j' | 'J' | 'o' | 'O') => {
                    self.quit = true
                }
                Input::Esc | Input::Char(_) => self.mode = Mode::Main,
                _ => {}
            },
            Mode::Running => {
                let live = self.run.as_ref().and_then(|r| r.real.as_ref());
                match (live, input) {
                    // A job on the real machine: Enter answers a dialog, Esc stops it, and once it has ended Esc leaves.
                    (Some(real), Input::Enter) => real.confirm(),
                    (Some(real), Input::Esc) if !real.finished() => real.cancel(),
                    (_, Input::Esc) => {
                        self.mode = Mode::Main;
                        self.run = None;
                    }
                    _ => {}
                }
            }
            Mode::Vectors | Mode::Tools | Mode::Config => {
                if matches!(input, Input::Esc | Input::Enter) {
                    self.mode = Mode::Main;
                }
            }
        }
    }

    fn key_list(&mut self, input: Input, len: usize, apply: impl Fn(&mut App, usize)) {
        match input {
            Input::Up => self.sel = self.sel.saturating_sub(1),
            Input::Down => self.sel = (self.sel + 1).min(len - 1),
            Input::Enter => {
                let i = self.sel;
                apply(self, i);
                self.mode = Mode::Main;
            }
            Input::Esc => self.mode = Mode::Main,
            _ => {}
        }
    }

    fn key_file_dialog(&mut self, input: Input) {
        match input {
            Input::Up => self.sel = self.sel.saturating_sub(1),
            Input::Down => self.sel = (self.sel + 1).min(self.entries.len().saturating_sub(1)),
            Input::PageUp => self.sel = self.sel.saturating_sub(10),
            Input::PageDown => self.sel = (self.sel + 10).min(self.entries.len().saturating_sub(1)),
            Input::Esc => self.mode = Mode::Main,
            Input::Enter => {
                let Some(e) = self.entries.get(self.sel) else {
                    return;
                };
                if e.dir {
                    if e.name == ".." {
                        if let Some(p) = self.dir.parent() {
                            self.dir = p.to_path_buf();
                        }
                    } else {
                        self.dir = self.dir.join(&e.name);
                    }
                    self.refresh_entries();
                } else {
                    let path = self.dir.join(&e.name);
                    self.load_file(&path);
                    self.mode = if self.error.is_none() {
                        Mode::Vectors
                    } else {
                        Mode::Main
                    };
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------ rendering

    pub fn render(&self, s: &mut Screen) {
        *s = Screen::default();
        s.fill(0, 0, COLS, ROWS, ' ', BLUE, BLUE);
        self.render_menu(s);
        self.render_info(s);
        match self.mode {
            Mode::Main => {}
            Mode::FileDialog => self.render_files(s),
            Mode::Vectors => self.render_vectors(s),
            Mode::Tools => self.render_tools(s),
            Mode::Config => self.render_config(s),
            Mode::Sort => self.render_list(s, Text::TitleSort, &self.sort_labels()),
            Mode::Language => {
                let names: Vec<String> = Lang::ALL
                    .iter()
                    .map(|l| l.native_name().to_string())
                    .collect();
                self.render_list(s, Text::TitleLanguage, &names)
            }
            Mode::Running => self.render_run(s),
            Mode::ConfirmQuit => self.render_quit(s),
        }
        s.fill(0, ROWS - 1, COLS, 1, ' ', BLACK, MENU_BG);
        s.text(1, ROWS - 1, tr(self.lang, Text::StatusMain), BLACK, MENU_BG);
    }

    fn render_menu(&self, s: &mut Screen) {
        s.fill(0, 0, COLS, 1, ' ', BLACK, MENU_BG);
        for (x, _, t) in self.menu_layout() {
            s.text(x, 0, " ", BLACK, MENU_BG);
            s.hot_text(x + 1, 0, tr(self.lang, t), BLACK, RED, MENU_BG);
        }
    }

    fn render_info(&self, s: &mut Screen) {
        s.text(2, 2, "DRILLPRO", YELLOW, BLUE);
        s.text(2, 3, "Bungard CCD", LIGHT_CYAN, BLUE);
        let msg = match (&self.error, &self.loaded) {
            (Some(e), _) => e.clone(),
            (None, Some(l)) => format!("{}: {}", tr(self.lang, Text::LblFile), l.path.display()),
            (None, None) => tr(self.lang, Text::NoFile).to_string(),
        };
        s.text(2, 5, &msg, WHITE, BLUE);
        if self.connection.is_real() {
            // A connection other than demo: the job moves the real machine. Say so, in red, with where it goes.
            let line = tr(self.lang, Text::RealMachine).replace("{}", &self.connection.describe());
            s.text(2, 7, &line, WHITE, RED);
            s.text(2, 8, tr(self.lang, Text::RealStop), LIGHT_RED, BLUE);
        } else {
            s.text(2, 7, tr(self.lang, Text::SimOnly), YELLOW, BLUE);
            s.text(2, 8, tr(self.lang, Text::Uncalibrated), LIGHT_RED, BLUE);
        }
    }

    fn render_files(&self, s: &mut Screen) {
        let (x, y, w, h) = (14, 3, 52, 19);
        s.window(
            x,
            y,
            w,
            h,
            tr(self.lang, Text::TitleSelectFile),
            BLACK,
            WIN_BG,
        );
        let dir = self.dir.display().to_string();
        let shown: String = dir
            .chars()
            .rev()
            .take(w - 6)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        s.text(x + 2, y + 1, &shown, BLUE, WIN_BG);
        let rows = h - 4;
        let top = self.sel.saturating_sub(rows - 1);
        for (i, e) in self.entries.iter().enumerate().skip(top).take(rows) {
            let label = if e.dir {
                format!("[{}]", e.name)
            } else {
                e.name.clone()
            };
            let (fg, bg) = if i == self.sel {
                (WHITE, BLACK)
            } else {
                (BLACK, WIN_BG)
            };
            s.fill(x + 2, y + 2 + i - top, w - 4, 1, ' ', fg, bg);
            s.text(x + 2, y + 2 + i - top, &label, fg, bg);
        }
    }

    fn render_vectors(&self, s: &mut Screen) {
        let (x, y, w, h) = (4, 2, 72, 22);
        s.window(x, y, w, h, tr(self.lang, Text::TitleVectors), BLACK, WIN_BG);
        let Some(l) = &self.loaded else {
            s.text(x + 2, y + 2, tr(self.lang, Text::NoFile), BLACK, WIN_BG);
            return;
        };
        let holes = sort_holes(&l.file, self.sorting);
        let b = bounds_mm(&holes);
        let (bw, bh) = b.map_or((0.0, 0.0), |b| (b.2 - b.0, b.3 - b.1));
        let name = l
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let info = [
            (Text::LblFile, name),
            (Text::LblHoles, holes.len().to_string()),
            (Text::LblToolsUsed, l.file.tools.len().to_string()),
            (Text::LblSize, format!("{bw:.1} x {bh:.1}")),
            (Text::LblTravel, format!("{:.0}", travel_mm(&holes))),
            (
                Text::LblSorting,
                self.sort_labels()[self.sorting_index()].clone(),
            ),
        ];
        for (i, (k, v)) in info.iter().enumerate() {
            s.text(
                x + 2 + (i % 2) * 34,
                y + 1 + i / 2,
                &format!("{}: {v}", tr(self.lang, *k)),
                BLACK,
                WIN_BG,
            );
        }
        // Plot area: cells are about twice as tall as wide.
        let (px, py, pw, ph) = (x + 2, y + 5, w - 4, h - 7);
        s.fill(px, py, pw, ph, ' ', WHITE, BLACK);
        if let Some(b) = b {
            let sx = if bw > 0.0 { (pw - 1) as f64 / bw } else { 1.0 };
            let sy = if bh > 0.0 { (ph - 1) as f64 / bh } else { 1.0 };
            let scale = sx.min(sy * 2.0);
            for h in &holes {
                let cx = ((h.x_mm() - b.0) * scale).round() as usize;
                let cy = ((h.y_mm() - b.1) * scale / 2.0).round() as usize;
                if cx < pw && cy < ph {
                    let idx = l.file.tools.iter().position(|&t| t == h.tool).unwrap_or(0);
                    let glyph = char::from_digit((idx % 35 + 1) as u32, 36)
                        .unwrap_or('#')
                        .to_ascii_uppercase();
                    s.put(
                        px + cx,
                        py + ph - 1 - cy,
                        glyph,
                        [LIGHT_GREEN, LIGHT_CYAN, YELLOW, LIGHT_RED, WHITE][idx % 5],
                        BLACK,
                    );
                }
            }
        }
    }

    fn render_tools(&self, s: &mut Screen) {
        let (x, y, w, h) = (8, 3, 64, 19);
        s.window(x, y, w, h, tr(self.lang, Text::TitleTools), BLACK, WIN_BG);
        if !self.tools_found {
            s.text(x + 2, y + 2, tr(self.lang, Text::NoToolFile), RED, WIN_BG);
            return;
        }
        let cols = [
            (x + 2, Text::ColTool),
            (x + 12, Text::ColStroke),
            (x + 28, Text::ColFeed),
            (x + 46, Text::ColSpindle),
        ];
        for (cx, t) in cols {
            s.text(cx, y + 1, tr(self.lang, t), BLUE, WIN_BG);
        }
        for (i, t) in self.tools.iter().take(h - 4).enumerate() {
            let r = y + 2 + i;
            s.text(x + 2, r, &format!("{:02}", t.number), BLACK, WIN_BG);
            s.text(
                x + 12,
                r,
                &format!("{:.1}", t.extra_stroke_mm),
                BLACK,
                WIN_BG,
            );
            s.text(x + 28, r, &t.feed_mm_min.to_string(), BLACK, WIN_BG);
            s.text(x + 46, r, &t.spindle_rpm.to_string(), BLACK, WIN_BG);
        }
    }

    fn render_config(&self, s: &mut Screen) {
        let (x, y, w, h) = (10, 5, 60, 12);
        s.window(x, y, w, h, tr(self.lang, Text::TitleConfig), BLACK, WIN_BG);
        s.text(
            x + 2,
            y + 9,
            &format!(
                "{}: {}",
                tr(self.lang, Text::LblLink),
                self.connection.describe()
            ),
            BLACK,
            WIN_BG,
        );
        let Some(c) = &self.config else {
            s.text(x + 2, y + 2, tr(self.lang, Text::NoConfigFile), RED, WIN_BG);
            return;
        };
        let rows = [
            ("Serial", c.serial_number.as_str()),
            ("Version", c.version.as_str()),
            (tr(self.lang, Text::Directory), c.directory.as_str()),
            ("Ext.", c.default_extension.as_str()),
        ];
        for (i, (k, v)) in rows.iter().enumerate() {
            s.text(x + 2, y + 2 + i, &format!("{k}: {v}"), BLACK, WIN_BG);
        }
        let n = c.values.len();
        if n >= 2 {
            let port = ["COM1", "COM2", "COM3", "COM4", "DEMO"]
                .get(c.values[n - 2].parse::<usize>().unwrap_or(9))
                .copied()
                .unwrap_or("?");
            let baud = ["9600", "19200", "38400"]
                .get(c.values[n - 1].parse::<usize>().unwrap_or(9))
                .copied()
                .unwrap_or("?");
            s.text(
                x + 2,
                y + 7,
                &format!("{}: {port}  {baud} 8N1", tr(self.lang, Text::LblSerial)),
                BLACK,
                WIN_BG,
            );
        }
    }

    fn sort_labels(&self) -> Vec<String> {
        [Text::SortNone, Text::SortX, Text::SortY, Text::SortSpecial]
            .iter()
            .map(|t| tr(self.lang, *t).to_string())
            .collect()
    }

    fn render_list(&self, s: &mut Screen, title: Text, items: &[String]) {
        let w = items
            .iter()
            .map(|i| str_width(i))
            .max()
            .unwrap_or(10)
            .max(str_width(tr(self.lang, title)))
            + 6;
        let h = items.len() + 4;
        let x = (COLS - w) / 2;
        let y = (ROWS - h) / 2;
        s.window(x, y, w, h, tr(self.lang, title), BLACK, WIN_BG);
        for (i, it) in items.iter().enumerate() {
            let (fg, bg) = if i == self.sel {
                (WHITE, BLACK)
            } else {
                (BLACK, WIN_BG)
            };
            s.fill(x + 2, y + 2 + i, w - 4, 1, ' ', fg, bg);
            s.text(x + 3, y + 2 + i, it, fg, bg);
        }
    }

    fn render_run(&self, s: &mut Screen) {
        let (x, y, w, h) = (12, 6, 56, 11);
        s.window(
            x,
            y,
            w,
            h,
            tr(self.lang, Text::TitleDrilling),
            BLACK,
            WIN_BG,
        );
        let Some(r) = &self.run else { return };
        let total = r.total_holes;
        let real = r.real.as_ref().map(|x| x.state());
        let status = match &real {
            Some(RunState::Connecting) => tr(self.lang, Text::Connecting).to_string(),
            Some(RunState::Running) => tr(self.lang, Text::Drilling).to_string(),
            Some(RunState::WaitPause(Pause::ToolChange { tool })) => {
                tr(self.lang, Text::WaitToolChange).replace("{}", &tool.to_string())
            }
            Some(RunState::WaitPause(Pause::SpindleStart)) => {
                tr(self.lang, Text::WaitSpindle).to_string()
            }
            Some(RunState::Done) => tr(self.lang, Text::Done).to_string(),
            Some(RunState::Cancelled) => tr(self.lang, Text::Stopped).to_string(),
            Some(RunState::Failed(_)) => tr(self.lang, Text::LinkFailed).to_string(),
            None if !r.connected => tr(self.lang, Text::Connecting).to_string(),
            None if r.next < r.items.len() => tr(self.lang, Text::Drilling).to_string(),
            None => tr(self.lang, Text::Done).to_string(),
        };
        let attention = matches!(
            real,
            Some(RunState::WaitPause(_)) | Some(RunState::Failed(_))
        );
        s.text(
            x + 2,
            y + 2,
            &status,
            if attention { RED } else { BLACK },
            WIN_BG,
        );
        if let Some(RunState::Failed(why)) = &real {
            let max = w - 4;
            let shown: String = why.chars().take(max).collect();
            s.text(x + 2, y + 3, &shown, RED, WIN_BG);
        }
        s.text(
            x + 2,
            y + 4,
            &format!(
                "{}: {} / {}",
                tr(self.lang, Text::LblProgress),
                r.done,
                total
            ),
            BLACK,
            WIN_BG,
        );
        let bar = w - 6;
        let filled = (bar * r.done).checked_div(total).unwrap_or(0);
        s.fill(x + 3, y + 5, bar, 1, '░', DARK_GRAY, WIN_BG);
        s.fill(x + 3, y + 5, filled, 1, '█', BLUE, WIN_BG);
        if real.is_some() {
            s.text(
                x + 2,
                y + 7,
                &format!(
                    "{}: {} / {}",
                    tr(self.lang, Text::LblItems),
                    r.next,
                    r.items.len()
                ),
                BLACK,
                WIN_BG,
            );
            s.text(
                x + 2,
                y + 8,
                &format!(
                    "{}: {}",
                    tr(self.lang, Text::LblLink),
                    self.connection.describe()
                ),
                BLACK,
                WIN_BG,
            );
            let done = real.as_ref().is_some_and(RunState::finished);
            if !done {
                s.text(
                    x + 2,
                    y + 9,
                    tr(self.lang, Text::KeyStop),
                    DARK_GRAY,
                    WIN_BG,
                );
            }
        } else {
            s.text(
                x + 2,
                y + 7,
                &format!(
                    "{}: {:.1}",
                    tr(self.lang, Text::LblTime),
                    r.sim.elapsed_us as f64 / 1e6
                ),
                BLACK,
                WIN_BG,
            );
            s.text(
                x + 2,
                y + 8,
                &format!("{} B", r.sim.sent.len()),
                BLACK,
                WIN_BG,
            );
        }
    }

    fn render_quit(&self, s: &mut Screen) {
        let q = tr(self.lang, Text::QuitAsk);
        let w = str_width(q) + 8;
        let (x, y, h) = ((COLS - w) / 2, 10, 6);
        s.window(x, y, w, h, tr(self.lang, Text::TitleQuit), WHITE, RED);
        s.text_centered(y + 2, x, w, q, WHITE, RED);
        let buttons = format!(
            "[ {} ]   [ {} ]",
            tr(self.lang, Text::BtnYes),
            tr(self.lang, Text::BtnNo)
        );
        s.text_centered(y + 4, x, w, &buttons, YELLOW, RED);
    }
}

impl TextApp for App {
    fn handle(&mut self, input: Input) {
        App::handle(self, input)
    }
    fn click(&mut self, col: usize, row: usize) {
        App::click(self, col, row)
    }
    fn tick(&mut self) -> bool {
        App::tick(self, 1)
    }
    fn render(&self, screen: &mut Screen) {
        App::render(self, screen)
    }
    fn lang(&self) -> Lang {
        self.lang
    }
    fn wants_quit(&self) -> bool {
        self.quit
    }
}
