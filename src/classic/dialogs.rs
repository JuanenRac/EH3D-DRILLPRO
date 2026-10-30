//! The dialogs of the classic front end: Parameter (with the language), Tool Data, Select File, Panelization,
//! Quit, the save question, About and the job windows. Each one works with the keyboard and with the mouse.

use super::*;
use crate::config::{CoordUnit, SortMode};
use edit::glob_matches;
use engine::ToolRow;
use std::path::PathBuf;

pub(crate) enum Dialog {
    None,
    Files(FileDlg),
    Tools(ToolDlg),
    Parameter(ParamDlg),
    Panel(PanelDlg),
    Quit {
        yes: bool,
    },
    SaveAsk {
        yes: bool,
    },
    /// "Delete old offset?" before the Offset Teach window.
    OffsetAsk {
        yes: bool,
    },
    Teach(Box<super::teach::TeachDlg>),
    /// "Write Teach-In file?" with the positions taught.
    TeachWrite {
        yes: bool,
        teach: Box<super::teach::TeachDlg>,
    },
    About,
    Message(String),
    Running,
}

/// What a dialog does after a key or a click.
pub(crate) enum Flow {
    Stay,
    Close,
    To(Box<Dialog>),
}

impl Flow {
    pub(crate) fn to(d: Dialog) -> Flow {
        Flow::To(Box::new(d))
    }
}

// ---------------------------------------------------------------------------------------------- state

/// What the Select File window is open for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Purpose {
    #[default]
    Job,
    Tools,
    Config,
}

pub(crate) struct FileDlg {
    name: LineEdit,
    /// The pattern the list is filtered with (`*.NCD`).
    pattern: String,
    entries: Vec<(String, bool)>,
    sel: usize,
    /// The arrows work on the list (true) or the keys go to the name field (false).
    list: bool,
    info: Option<String>,
    /// The archive the dialog is inside of, when it is.
    zip: Option<ZipView>,
    /// The drop-down list of patterns is open, with this one chosen.
    drop: Option<usize>,
}

/// A ZIP archive the Select File dialog is inside of: where it is and the full names of the files it lists (`entries[i + 1]` is `full[i]`;
/// the first entry takes back out).
pub(crate) struct ZipView {
    path: PathBuf,
    full: Vec<String>,
}

impl FileDlg {
    /// The folders and the files that match the pattern (inside an archive, every file of the kind of the program).
    fn visible(&self) -> Vec<&(String, bool)> {
        if self.zip.is_some() {
            return self.entries.iter().collect();
        }
        self.entries
            .iter()
            .filter(|(n, dir)| {
                // a job archive is listed like a folder, but only when the pattern asks for it (*.ZIP or *.*)
                let zip = *dir && n.to_ascii_lowercase().ends_with(".zip");
                if zip {
                    glob_matches(&self.pattern, n)
                } else {
                    *dir || glob_matches(&self.pattern, n)
                }
            })
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ctl {
    Text(usize),
    Unit,
    Sort,
    Lang,
    /// The "AutoChg" check box of the models with a tool changer.
    Auto,
    /// The connection with the machine: Enter or a click opens its window.
    Conn,
    Btn(Dlg),
}

pub(crate) struct ParamDlg {
    values: Vec<String>,
    unit: usize,
    sorting: usize,
    lang: usize,
    /// The state of the AutoChg check box; `None` on the models without a tool changer (no box).
    auto: Option<bool>,
    ring: Vec<Ctl>,
    focus: Option<usize>,
    edit: Option<LineEdit>,
    error: Option<String>,
    status: Option<String>,
    /// The connection with the machine, being chosen in the window that opens over this one.
    conn: super::connect::ConnState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolFocus {
    None,
    Cell(usize, usize),
    Btn(Dlg),
}

pub(crate) struct ToolDlg {
    rows: Vec<ToolRow>,
    focus: ToolFocus,
    edit: Option<LineEdit>,
    error: Option<String>,
    status: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PzCtl {
    Value(usize),
    Btn(Dlg),
}

pub(crate) struct PanelDlg {
    /// Board X and Y, spacing X and Y, number X and Y.
    values: [String; 6],
    ring: Vec<PzCtl>,
    focus: Option<usize>,
    edit: Option<LineEdit>,
    error: Option<String>,
}

/// Which keys a Parameter field takes, how long it is, and the range it accepts.
fn param_spec(i: usize) -> (EditKind, usize, f64, f64) {
    match i {
        0 => (EditKind::Text, 17, 0.0, 0.0),
        1 => (EditKind::Text, 12, 0.0, 0.0),
        2..=7 => (EditKind::Number, 5, 1.0, 20000.0),
        8 | 9 => (EditKind::Number, 2, 1.0, 99.0),
        10..=12 => (EditKind::Number, 6, 0.5, 1.5),
        13 | 14 => (EditKind::Number, 5, 0.0, 99.99),
        15 => (EditKind::Number, 3, 0.0, 9.9),
        _ => (EditKind::Number, 3, -60.0, 60.0),
    }
}

fn param_format(i: usize, v: f64) -> String {
    match i {
        2..=9 => format!("{v:.0}"),
        10..=12 => format!("{v:.4}"),
        13 | 14 => format!("{v:05.2}"),
        15 => format!("{v}"),
        _ => format!("{:+03}", v as i32),
    }
}

/// Max chars, range and display of a column of the tool table: (max, min, max value, decimals, width).
fn tool_spec(route: bool, col: usize) -> (usize, f64, f64, usize, usize) {
    if route {
        match col {
            0 => (5, 0.0, 99.99, 2, 5),
            1 => (2, 1.0, 99.0, 0, 2),
            2 | 3 => (4, 1.0, 9999.0, 0, 4),
            _ => (5, 1000.0, 99999.0, 0, 5),
        }
    } else {
        match col {
            0 => (4, 0.0, 9.99, 2, 4),
            1 => (4, 1.0, 9999.0, 0, 4),
            _ => (5, 1000.0, 99999.0, 0, 5),
        }
    }
}

fn pad_number(v: f64, decimals: usize, width: usize) -> String {
    if decimals == 0 {
        format!("{:0width$}", v.round() as i64, width = width)
    } else {
        format!("{v:0w$.d$}", w = width, d = decimals)
    }
}

impl<E: ClassicEngine> ClassicApp<E> {
    // ------------------------------------------------------------------------------------------ dispatch

    pub(crate) fn handle_input(&mut self, input: Input) {
        match self.dialog {
            Dialog::None => self.key_main(input),
            Dialog::Running => self.running_key(input),
            Dialog::Message(_) => self.dialog = Dialog::None,
            _ => {
                let mut d = std::mem::replace(&mut self.dialog, Dialog::None);
                let flow = self.dialog_key(&mut d, input);
                self.finish(d, flow);
            }
        }
    }

    pub(crate) fn handle_click(&mut self, act: Act, double: bool) {
        match self.dialog {
            Dialog::None => self.click_main(act),
            Dialog::Running if self.interrupted => match act {
                Act::Dlg(Dlg::Ok) => self.interrupted = false,
                Act::Dlg(Dlg::Abort) => self.abort_job(),
                _ => {}
            },
            Dialog::Running => {
                // the operator's question has its two buttons
                match act {
                    Act::Dlg(Dlg::Ok) if self.engine.question().is_some() => {
                        self.engine.answer(true)
                    }
                    Act::Dlg(Dlg::Abort) if self.engine.question().is_some() => {
                        self.engine.answer(false);
                        self.dialog = Dialog::None;
                    }
                    _ => {}
                }
            }
            Dialog::Message(_) => {}
            _ => {
                let mut d = std::mem::replace(&mut self.dialog, Dialog::None);
                let flow = self.dialog_click(&mut d, act, double);
                self.finish(d, flow);
            }
        }
    }

    /// Keys while a job runs: Esc stops it; at a question of the operator Enter (or the letter of Ok) goes on and Esc
    /// (or the letter of Abort) stops the job.
    fn running_key(&mut self, input: Input) {
        if self.interrupted {
            let cont = hot_letter(self.t(CText::BCont));
            let abort = hot_letter(self.t(CText::BAbort));
            match input {
                Input::Enter => self.interrupted = false,
                Input::Esc => self.abort_job(),
                Input::Char(c) | Input::Alt(c) => {
                    let c = c.to_ascii_lowercase();
                    if Some(c) == cont {
                        self.interrupted = false;
                    } else if Some(c) == abort {
                        self.abort_job();
                    }
                }
                _ => {}
            }
            return;
        }
        if self.engine.question().is_none() {
            if input == Input::Esc {
                if self.engine.started() {
                    // drilling: the original asks what next before it stops
                    self.interrupted = true;
                } else {
                    self.abort_job();
                }
            }
            return;
        }
        let hot_ok = hot_letter(self.t(CText::BOk));
        let hot_abort = hot_letter(self.t(CText::BAbort));
        let abort = match input {
            Input::Enter => {
                self.engine.answer(true);
                return;
            }
            Input::Esc => true,
            Input::Char(c) | Input::Alt(c) => {
                let c = c.to_ascii_lowercase();
                if Some(c) == hot_ok {
                    self.engine.answer(true);
                    return;
                }
                Some(c) == hot_abort
            }
            _ => false,
        };
        if abort {
            self.engine.answer(false);
            self.dialog = Dialog::None;
        }
    }

    fn abort_job(&mut self) {
        self.interrupted = false;
        self.engine.stop();
        self.dialog = Dialog::None;
    }

    /// The window after Esc during the job: INTERRUPTED, what next, Cont. and Abort.
    fn render_interrupted(&self, s: &mut Screen) {
        window(s, 19, 6, 41, 11, "", false);
        let title = self.t(CText::QInterrupted);
        label(
            s,
            19 + (41usize.saturating_sub(str_width(title))) / 2,
            8,
            title,
        );
        let ask = self.t(CText::QWhatNext);
        label(
            s,
            19 + (41usize.saturating_sub(str_width(ask))) / 2,
            10,
            ask,
        );
        button(s, 24, 13, 12, self.t(CText::BCont), true, true);
        button(s, 43, 13, 12, self.t(CText::BAbort), false, true);
        self.hit(24, 13, 13, 2, Act::Dlg(Dlg::Ok));
        self.hit(43, 13, 13, 2, Act::Dlg(Dlg::Abort));
    }

    /// The window the job asks the operator with: the sign, the words and Ok and Abort.
    fn render_question(&self, s: &mut Screen, q: engine::Question) {
        if let engine::Question::ToolChange(n) = q {
            return self.render_tool_change(s, n);
        }
        window(s, 19, 6, 41, 11, "", false);
        s.frame(21, 8, 12, 3, LIGHT_GREEN, GREY);
        s.text(23, 9, self.t(CText::QAttention), INK, GREY);
        for xx in 23..35 {
            s.put(xx, 11, ' ', BLACK, BLACK);
        }
        for yy in [9, 10] {
            s.put(33, yy, ' ', BLACK, BLACK);
            s.put(34, yy, ' ', BLACK, BLACK);
        }
        let ask = match q {
            engine::Question::SpindleStart => self.t(CText::QSpindleAsk).to_string(),
            engine::Question::ToolChange(n) => {
                self.t(CText::QToolAsk).replace("{}", &format!("{n:02}"))
            }
        };
        let (l1, l2) = split_two(&ask, 18);
        s.text(38, 9, &l1, INK, GREY);
        s.text(38, 10, &l2, INK, GREY);
        button(s, 29, 14, 10, self.t(CText::BOk), true, true);
        button(s, 42, 14, 10, self.t(CText::BAbort), false, true);
        self.hit(29, 14, 11, 2, Act::Dlg(Dlg::Ok));
        self.hit(42, 14, 11, 2, Act::Dlg(Dlg::Abort));
    }

    /// The tool change window: which tool goes in and what the run with it will be (the words are the original's).
    fn render_tool_change(&self, s: &mut Screen, tool: u8) {
        let (x, y) = (18usize, 3usize);
        window(s, x, y, 44, 18, "", false);
        let title = self.t(CText::TcTitle);
        label(
            s,
            x + (44usize.saturating_sub(str_width(title))) / 2,
            y + 1,
            title,
        );
        label(
            s,
            x + 4,
            y + 4,
            &format!("{} {:02}", self.t(CText::TcInsert), tool),
        );
        let view = self.engine.job_view().unwrap_or_default();
        let rows = [
            (y + 7, CText::TcRun, format!("{:.1}", view.run_mm)),
            (y + 8, CText::TcSpindle, format!("{:05}", view.rpm)),
            (y + 10, CText::TcZSpeed, format!("{:04}", view.z_feed)),
            (y + 11, CText::TcStroke, format!("{:05.2}", view.stroke_mm)),
        ];
        for (row, k, value) in rows {
            label(s, x + 4, row, self.t(k));
            label(s, x + 30, row, &value);
        }
        button(s, x + 9, y + 14, 10, self.t(CText::BOk), true, true);
        button(s, x + 25, y + 14, 10, self.t(CText::BAbort), false, true);
        self.hit(x + 9, y + 14, 11, 2, Act::Dlg(Dlg::Ok));
        self.hit(x + 25, y + 14, 11, 2, Act::Dlg(Dlg::Abort));
    }

    /// The wheel moves the selection of the file list.
    pub(crate) fn scroll_list(&mut self, lines: isize) {
        if let Dialog::Files(d) = &mut self.dialog {
            let n = d.visible().len();
            let next = (d.sel as isize + lines).clamp(0, n.saturating_sub(1) as isize);
            d.sel = next as usize;
            d.list = true;
        }
    }

    fn finish(&mut self, d: Dialog, flow: Flow) {
        let by_hand = |d: &Dialog| matches!(d, Dialog::Teach(_) | Dialog::TeachWrite { .. });
        let was_by_hand = by_hand(&d);
        match flow {
            Flow::Stay => self.dialog = d,
            Flow::Close => {}
            Flow::To(n) => self.dialog = *n,
        }
        // leaving the Offset Teach window sends a real machine home again
        if was_by_hand && !by_hand(&self.dialog) {
            self.engine.teach_end();
        }
    }

    fn dialog_key(&mut self, d: &mut Dialog, input: Input) -> Flow {
        match d {
            Dialog::Files(f) => self.files_key(f, input),
            Dialog::Tools(t) => self.tools_key(t, input),
            Dialog::Parameter(p) => self.param_key(p, input),
            Dialog::Panel(p) => self.panel_key(p, input),
            Dialog::Quit { yes } => self.yes_no_key(yes, input, false),
            Dialog::SaveAsk { yes } => self.yes_no_key(yes, input, true),
            Dialog::OffsetAsk { yes } => self.offset_ask_key(yes, input),
            Dialog::Teach(t) => self.teach_key(t, input),
            Dialog::TeachWrite { yes, teach } => self.teach_write_key(yes, teach, input),
            Dialog::About => match input {
                Input::Enter | Input::Esc | Input::Char(_) | Input::Alt(_) => Flow::Close,
                _ => Flow::Stay,
            },
            _ => Flow::Stay,
        }
    }

    fn dialog_click(&mut self, d: &mut Dialog, act: Act, double: bool) -> Flow {
        match d {
            Dialog::Files(f) => self.files_click(f, act, double),
            Dialog::Tools(t) => self.tools_click(t, act),
            Dialog::Parameter(p) => self.param_click(p, act),
            Dialog::Panel(p) => self.panel_click(p, act),
            Dialog::Quit { yes } => self.yes_no_click(yes, act, false),
            Dialog::SaveAsk { yes } => self.yes_no_click(yes, act, true),
            Dialog::OffsetAsk { .. } => self.offset_ask_click(act),
            Dialog::Teach(t) => self.teach_click(t, act),
            Dialog::TeachWrite { teach, .. } => self.teach_write_click(teach, act),
            Dialog::About => match act {
                Act::Dlg(_) => Flow::Close,
                _ => Flow::Stay,
            },
            _ => Flow::Stay,
        }
    }

    pub(crate) fn render_dialog(&self, s: &mut Screen) {
        match &self.dialog {
            Dialog::None => {}
            Dialog::Files(d) => self.render_files(s, d),
            Dialog::Tools(d) => self.render_tools(s, d),
            Dialog::Parameter(d) => self.render_parameter(s, d),
            Dialog::Panel(d) => self.render_panel(s, d),
            Dialog::Quit { yes } => self.render_yes_no(s, *yes, self.t(CText::QAsk), true),
            Dialog::SaveAsk { yes } => self.render_yes_no(s, *yes, self.t(CText::BackupAsk), false),
            Dialog::OffsetAsk { yes } => self.render_sign(
                s,
                *yes,
                self.t(CText::QOffsetDelete),
                self.t(CText::QAttentionPlain),
                9,
            ),
            Dialog::Teach(t) => self.render_teach(s, t),
            Dialog::TeachWrite { yes, .. } => {
                self.render_sign(s, *yes, self.t(CText::QTeachWrite), self.t(CText::QInfo), 8)
            }
            Dialog::About => self.render_about(s),
            Dialog::Message(m) => {
                let w = (str_width(m) + 6).min(COLS);
                let x = (COLS - w) / 2;
                window(s, x, 10, w, 5, "", false);
                s.text_centered(12, x, w, m, INK, GREY);
            }
            Dialog::Running => self.render_running(s),
        }
        // a hint on the grey strip while something is being typed
        if self.dialog_editing() {
            let hint = self.t(CText::HintEdit);
            s.text(1, ROWS - 2, hint, BLACK, GREY);
        }
    }

    fn dialog_editing(&self) -> bool {
        match &self.dialog {
            Dialog::Parameter(d) => d.edit.is_some() || (d.conn.open && d.conn.edit.is_some()),
            Dialog::Tools(d) => d.edit.is_some(),
            Dialog::Panel(d) => d.edit.is_some(),
            Dialog::Files(d) => !d.list && !d.name.fresh,
            _ => false,
        }
    }

    // ------------------------------------------------------------------------------------------ Quit and save

    fn yes_no_key(&mut self, yes: &mut bool, input: Input, save: bool) -> Flow {
        let hot_yes = hot_letter(self.t(CText::BYes));
        let hot_no = hot_letter(self.t(CText::BNo));
        match input {
            Input::Enter => {
                if *yes {
                    self.confirm(save)
                } else if save {
                    self.quit = true;
                    Flow::Close
                } else {
                    Flow::Close
                }
            }
            Input::Esc => Flow::Close,
            Input::Left | Input::Right | Input::Up | Input::Down | Input::Tab | Input::BackTab => {
                *yes = !*yes;
                Flow::Stay
            }
            Input::Char(c) | Input::Alt(c) => {
                let c = c.to_ascii_lowercase();
                if Some(c) == hot_yes {
                    self.confirm(save)
                } else if Some(c) == hot_no {
                    if save {
                        self.quit = true;
                    }
                    Flow::Close
                } else {
                    Flow::Stay
                }
            }
            _ => Flow::Stay,
        }
    }

    fn yes_no_click(&mut self, yes: &mut bool, act: Act, save: bool) -> Flow {
        match act {
            Act::Dlg(Dlg::Yes) => {
                *yes = true;
                self.confirm(save)
            }
            Act::Dlg(Dlg::No) => {
                if save {
                    self.quit = true;
                }
                Flow::Close
            }
            _ => Flow::Stay,
        }
    }

    /// "Yes": for the quit question, leave (after asking about the changed settings); for the save question, save and leave.
    fn confirm(&mut self, save: bool) -> Flow {
        if save {
            self.apply_form();
            self.engine.settings_changed();
            if let Err(e) = self.engine.save_config() {
                return Flow::to(Dialog::Message(format!("{}: {e}", self.t(CText::ErrWrite))));
            }
            self.dirty = false;
            self.quit = true;
            return Flow::Close;
        }
        match self.leave() {
            Dialog::None => Flow::Close,
            other => Flow::to(other),
        }
    }

    fn render_yes_no(&self, s: &mut Screen, yes: bool, ask: &str, stop_sign: bool) {
        let sign = if stop_sign {
            CText::QStop
        } else {
            CText::QSave
        };
        self.render_sign(s, yes, ask, self.t(sign), 9);
    }

    /// A question with its sign (STOP, Attention, Info) and Yes and No.
    pub(crate) fn render_sign(&self, s: &mut Screen, yes: bool, ask: &str, sign: &str, row: usize) {
        window(s, 19, 6, 41, 11, "", false);
        // the sign: a double frame with its own shadow
        s.frame(21, 8, 12, 3, LIGHT_GREEN, GREY);
        s.text(23, 9, sign, INK, GREY);
        for xx in 23..35 {
            s.put(xx, 11, ' ', BLACK, BLACK);
        }
        for yy in [9, 10] {
            s.put(33, yy, ' ', BLACK, BLACK);
            s.put(34, yy, ' ', BLACK, BLACK);
        }
        let (l1, l2) = match ask.split_once('|') {
            Some((a, b)) => (a.to_string(), b.to_string()),
            None => split_two(ask, 18),
        };
        s.text(38, row, &l1, INK, GREY);
        s.text(38, row + 1, &l2, INK, GREY);
        button(s, 29, 14, 10, self.t(CText::BYes), yes, true);
        button(s, 42, 14, 10, self.t(CText::BNo), !yes, true);
        self.hit(29, 14, 11, 2, Act::Dlg(Dlg::Yes));
        self.hit(42, 14, 11, 2, Act::Dlg(Dlg::No));
    }

    // ------------------------------------------------------------------------------------------ About

    fn render_about(&self, s: &mut Screen) {
        let about = self.engine.about();
        window(s, 7, 2, 66, 19, "", false);
        s.text_centered(3, 7, 66, about.name, WHITE, GREY);
        let desc = match self.engine.kind() {
            Kind::Drill => CText::AboutDescDrill,
            Kind::Route => CText::AboutDescRoute,
        };
        s.text_centered(4, 7, 66, self.t(desc), INK, GREY);
        let rows = [
            (6, CText::AboutVersion, about.version.to_string()),
            (7, CText::AboutLicense, "GPL-3.0-or-later".to_string()),
            (
                8,
                CText::AboutAuthor,
                "Electro Hobby 3D (JuanenRac)".to_string(),
            ),
            (
                9,
                CText::AboutRepo,
                about.repository.replace("https://", ""),
            ),
        ];
        for (y, k, v) in rows {
            label(s, 11, y, self.t(k));
            label(s, 22, y, &v);
        }
        label(s, 11, 11, self.t(CText::AboutMachines));
        label(s, 13, 12, self.t(CText::AboutMtc));
        label(s, 13, 13, self.t(CText::AboutAtc));
        label(s, 13, 14, self.t(CText::AboutTested));
        label(s, 11, 16, self.t(CText::AboutDisclaimer));
        button(s, 31, 18, 9, self.t(CText::BOk), true, true);
        self.hit(31, 18, 10, 2, Act::Dlg(Dlg::Ok));
    }

    // ------------------------------------------------------------------------------------------ Select File

    pub(crate) fn open_files(&mut self) -> Dialog {
        // a folder written as `.` has no parent to go up to: always work with the absolute one
        let dir = engine::absolute(&self.engine.dir());
        self.engine.set_dir(dir);
        let mut entries = match self.file_purpose {
            Purpose::Job => self.engine.entries(),
            p => {
                // the tool and configuration files of the program: the folders and the files with its extension
                let ext = self
                    .purpose_pattern(p)
                    .trim_start_matches('*')
                    .to_ascii_lowercase();
                let mut v = vec![("..".to_string(), true)];
                if let Ok(rd) = std::fs::read_dir(self.engine.dir()) {
                    for e in rd.flatten() {
                        let name = e.file_name().to_string_lossy().into_owned();
                        let dir = e.path().is_dir();
                        if !name.starts_with('.')
                            && (dir || name.to_ascii_lowercase().ends_with(&ext))
                        {
                            v.push((name, dir));
                        }
                    }
                }
                engine::dos_order(v)
            }
        };
        // the job archives of the folder are listed like folders: Enter goes inside
        if self.file_purpose == Purpose::Job {
            if let Ok(rd) = std::fs::read_dir(self.engine.dir()) {
                let mut zips: Vec<String> = rd
                    .flatten()
                    .filter(|e| e.path().is_file())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .filter(|n| n.to_ascii_lowercase().ends_with(".zip"))
                    .collect();
                zips.sort_unstable_by_key(|n| n.to_ascii_lowercase());
                entries.extend(zips.into_iter().map(|n| (n, true)));
            }
        }
        entries.extend(engine::drive_entries());
        let pattern = match self.file_purpose {
            Purpose::Job => self.engine.pattern().to_string(),
            p => self.purpose_pattern(p).to_string(),
        };
        Dialog::Files(FileDlg {
            name: LineEdit::new(&pattern, EditKind::Text, 260),
            pattern,
            entries,
            sel: 0,
            list: false,
            info: None,
            zip: None,
            drop: None,
        })
    }

    /// Goes into `target` (a folder); when it cannot be read the dialog stays where it was and says so.
    fn files_enter(&mut self, d: &mut FileDlg, target: PathBuf, keep_pattern: bool) -> Flow {
        let before = self.engine.dir();
        if std::fs::read_dir(&target).is_err() {
            d.info = Some(self.t(CText::FNoFolder).to_string());
            return Flow::Stay;
        }
        self.engine.set_dir(target);
        let Dialog::Files(mut fresh) = self.open_files() else {
            self.engine.set_dir(before);
            return Flow::Stay;
        };
        if keep_pattern {
            fresh.pattern = d.pattern.clone();
            fresh.name = LineEdit::new(&d.pattern, EditKind::Text, 260);
        }
        Flow::to(Dialog::Files(fresh))
    }

    fn files_open_selected(&mut self, d: &mut FileDlg) -> Flow {
        let Some((name, is_dir)) = d.visible().get(d.sel).map(|e| (*e).clone()) else {
            return Flow::Stay;
        };
        let dir = self.engine.dir();
        // inside an archive: the first entry takes back out, the others are extracted and loaded
        if let Some(z) = &d.zip {
            if d.sel == 0 {
                return self.files_enter(d, dir, true);
            }
            let (path, full) = (z.path.clone(), z.full.get(d.sel - 1).cloned());
            let Some(full) = full else {
                return Flow::Stay;
            };
            return match extract_to_temp(&path, &full) {
                Ok(p) => self.files_load(p),
                Err(e) => {
                    d.info = Some(e.chars().take(44).collect());
                    Flow::Stay
                }
            };
        }
        if is_dir && name.to_ascii_lowercase().ends_with(".zip") && dir.join(&name).is_file() {
            return self.files_enter_zip(d, dir.join(&name));
        }
        if is_dir {
            if let Some(drive) = engine::drive_path(&name) {
                return self.files_enter(d, drive, true);
            }
            let next = if name == ".." {
                match dir.parent() {
                    Some(p) => p.to_path_buf(),
                    None => return Flow::Stay,
                }
            } else {
                dir.join(name)
            };
            return self.files_enter(d, next, true);
        }
        self.files_load(dir.join(name))
    }

    /// Goes inside a job archive: lists the files of the kind of this program (drilling or routing).
    fn files_enter_zip(&mut self, d: &mut FileDlg, path: PathBuf) -> Flow {
        let kind = match self.engine.kind() {
            Kind::Drill => eh3d_archive::Kind::Drill,
            Kind::Route => eh3d_archive::Kind::Route,
        };
        let archive = match eh3d_archive::Archive::open(&path) {
            Ok(a) => a,
            Err(e) => {
                d.info = Some(e.chars().take(44).collect());
                return Flow::Stay;
            }
        };
        let full: Vec<String> = archive
            .entries
            .iter()
            .filter(|e| e.kind == kind)
            .map(|e| e.name.clone())
            .collect();
        if full.is_empty() {
            d.info = Some(self.t(CText::FNotFound).to_string());
            return Flow::Stay;
        }
        let mut entries = vec![("..".to_string(), true)];
        entries.extend(
            full.iter()
                .map(|n| (n.rsplit('/').next().unwrap_or(n).to_string(), false)),
        );
        Flow::to(Dialog::Files(FileDlg {
            name: LineEdit::new(&d.pattern, EditKind::Text, 260),
            pattern: d.pattern.clone(),
            entries,
            sel: 1,
            list: true,
            info: None,
            zip: Some(ZipView { path, full }),
            drop: None,
        }))
    }

    /// Loads the first file of the kind of this program from a job archive (`drillpro job.zip` on the command line).
    pub fn load_zip(&mut self, path: &std::path::Path) {
        let kind = match self.engine.kind() {
            Kind::Drill => eh3d_archive::Kind::Drill,
            Kind::Route => eh3d_archive::Kind::Route,
        };
        let first = eh3d_archive::Archive::open(path).and_then(|a| {
            a.first(kind)
                .map(|e| e.name.clone())
                .ok_or_else(|| format!("{}: no file of this kind in the archive", path.display()))
        });
        match first.and_then(|name| extract_to_temp(path, &name)) {
            Ok(p) => self.load_file(&p),
            Err(e) => self.engine.set_error(e),
        }
    }

    /// Like [`Self::files_enter`], with the pattern the dialog holds now.
    fn files_enter_pattern(&mut self, d: &mut FileDlg, target: PathBuf) -> Flow {
        let pattern = d.pattern.clone();
        match self.files_enter(d, target, false) {
            Flow::To(n) => match *n {
                Dialog::Files(mut f) => {
                    f.name = LineEdit::new(&pattern, EditKind::Text, 260);
                    f.pattern = pattern;
                    Flow::to(Dialog::Files(f))
                }
                other => Flow::to(other),
            },
            other => other,
        }
    }

    /// The pattern of the tool files or of the configuration files of this program.
    fn purpose_pattern(&self, p: Purpose) -> &'static str {
        match (self.engine.kind(), p) {
            (Kind::Drill, Purpose::Tools) => "*.DTD",
            (Kind::Drill, _) => "*.DCF",
            (Kind::Route, Purpose::Tools) => "*.RTD",
            (Kind::Route, _) => "*.RCF",
        }
    }

    /// The Load button of the Tool Data and Parameter windows: choose the file to load, from the folder of the machine files.
    fn open_files_for(&mut self, p: Purpose) -> Dialog {
        self.file_purpose = p;
        self.engine.set_dir(self.engine.data_dir());
        self.open_files()
    }

    fn files_load(&mut self, path: PathBuf) -> Flow {
        let purpose = std::mem::take(&mut self.file_purpose);
        let say = |e: String| Flow::to(Dialog::Message(e.chars().take(70).collect()));
        match purpose {
            Purpose::Tools => {
                return match self.engine.load_tools_from(&path) {
                    Ok(()) => {
                        let mut n = self.open_tools();
                        if let Dialog::Tools(t) = &mut n {
                            t.status = Some(format!(
                                "{} {}",
                                self.t(CText::Loaded),
                                file_label_of(&path)
                            ));
                        }
                        Flow::to(n)
                    }
                    Err(e) => say(format!("{}: {e}", self.t(CText::ErrRead))),
                };
            }
            Purpose::Config => {
                return match self.engine.load_config_from(&path) {
                    Ok(()) => {
                        self.form = Self::form_of(&self.engine);
                        self.sync_vectors();
                        self.area = (self.form.clip.0, self.form.clip.1);
                        self.dirty = false;
                        let mut n = self.open_parameter();
                        if let Dialog::Parameter(p) = &mut n {
                            p.status = Some(format!(
                                "{} {}",
                                self.t(CText::Loaded),
                                file_label_of(&path)
                            ));
                        }
                        Flow::to(n)
                    }
                    Err(e) => say(format!("{}: {e}", self.t(CText::ErrRead))),
                };
            }
            Purpose::Job => {}
        }
        self.load_file(&path);
        // a file that cannot be read is said so in a window, as the original does; otherwise its tool data follow
        if !self.engine.file_loaded() {
            if let Some(e) = self.engine.error() {
                let msg: String = e.chars().take(70).collect();
                return Flow::to(Dialog::Message(msg));
            }
        }
        // the original shows its tool data right after loading
        Flow::to(self.open_tools())
    }

    /// Enter in the name field: a pattern filters the list, a name opens the file.
    fn files_accept_name(&mut self, d: &mut FileDlg) -> Flow {
        let typed = d.name.text.trim().to_string();
        // the name of a job archive goes inside it
        if typed.to_ascii_lowercase().ends_with(".zip") {
            let dir = self.engine.dir();
            let candidate = [dir.join(&typed), PathBuf::from(&typed)]
                .into_iter()
                .find(|p| p.is_file());
            if let Some(p) = candidate {
                return self.files_enter_zip(d, p);
            }
        }
        if typed.is_empty() {
            d.pattern = self.engine.pattern().to_string();
            d.name = LineEdit::new(&d.pattern, EditKind::Text, 260);
            return Flow::Stay;
        }
        // a path (`D:\jobs\*.NCD`, `..\other`, `C:\`): go to its folder and take the rest as the pattern or the file
        if typed.contains('\\') || typed.contains('/') || typed.contains(':') {
            let norm = typed.replace('\\', "/");
            let (folder, rest) = match norm.rfind('/') {
                Some(i) => (norm[..=i].to_string(), norm[i + 1..].to_string()),
                None => (norm.clone(), String::new()),
            };
            let folder = if folder.ends_with(':') {
                format!("{folder}/")
            } else {
                folder
            };
            let absolute_path = folder.chars().nth(1) == Some(':') || folder.starts_with('/');
            let base = if absolute_path {
                PathBuf::from(&folder)
            } else {
                self.engine.dir().join(&folder)
            };
            if base.is_dir() {
                if !rest.is_empty() && !rest.contains('*') && !rest.contains('?') {
                    let file = base.join(&rest);
                    if file.is_file() {
                        return self.files_load(file);
                    }
                }
                d.pattern = if rest.is_empty() {
                    self.engine.pattern().to_string()
                } else {
                    rest
                };
                return self.files_enter_pattern(d, base);
            }
            d.info = Some(self.t(CText::FNoFolder).to_string());
            return Flow::Stay;
        }
        if typed.contains('*') || typed.contains('?') {
            d.pattern = typed;
            d.sel = 0;
            d.info = None;
            return Flow::Stay;
        }
        // DOS names have no case: match the typed name against the folder whatever its capitals
        let dir = self.engine.dir();
        let found = d
            .entries
            .iter()
            .find(|(n, is_dir)| !is_dir && n.eq_ignore_ascii_case(&typed))
            .map(|(n, _)| dir.join(n));
        match found {
            Some(p) => self.files_load(p),
            None => {
                d.info = Some(self.t(CText::FNotFound).to_string());
                Flow::Stay
            }
        }
    }

    /// The patterns the drop-down list of the name field offers.
    fn pattern_choices(&self) -> Vec<&'static str> {
        match self.file_purpose {
            Purpose::Job => {
                let mut v = vec![self.engine.pattern()];
                v.extend(self.engine.extra_patterns());
                v.extend(["*.ZIP", "*.*"]);
                v
            }
            p => vec![self.purpose_pattern(p), "*.*"],
        }
    }

    fn choose_pattern(&mut self, d: &mut FileDlg, i: usize) {
        if let Some(p) = self.pattern_choices().get(i) {
            d.pattern = (*p).to_string();
            d.name = LineEdit::new(p, EditKind::Text, 260);
            d.sel = 0;
            d.list = false;
        }
        d.drop = None;
    }

    fn files_key(&mut self, d: &mut FileDlg, input: Input) -> Flow {
        if let Some(cur) = d.drop {
            let n = self.pattern_choices().len();
            match input {
                Input::Esc => d.drop = None,
                Input::Up => d.drop = Some(cur.saturating_sub(1)),
                Input::Down => d.drop = Some((cur + 1).min(n - 1)),
                Input::Enter => self.choose_pattern(d, cur),
                _ => {}
            }
            return Flow::Stay;
        }
        let n = d.visible().len();
        match input {
            Input::Esc => Flow::Close,
            Input::Down => {
                d.list = true;
                d.sel = (d.sel + 1).min(n.saturating_sub(1));
                Flow::Stay
            }
            Input::Up => {
                d.list = true;
                d.sel = d.sel.saturating_sub(1);
                Flow::Stay
            }
            Input::PageDown => {
                d.list = true;
                d.sel = (d.sel + 16).min(n.saturating_sub(1));
                Flow::Stay
            }
            Input::PageUp => {
                d.list = true;
                d.sel = d.sel.saturating_sub(16);
                Flow::Stay
            }
            Input::Right if d.list => {
                d.sel = (d.sel + 8).min(n.saturating_sub(1));
                Flow::Stay
            }
            Input::Left if d.list => {
                d.sel = d.sel.saturating_sub(8);
                Flow::Stay
            }
            Input::Tab | Input::BackTab => {
                d.list = !d.list;
                Flow::Stay
            }
            Input::Enter => {
                if d.list {
                    self.files_open_selected(d)
                } else {
                    self.files_accept_name(d)
                }
            }
            other => {
                // typing goes to the name field
                d.list = false;
                d.info = None;
                if let Input::Alt(_) = other {
                    return Flow::Stay;
                }
                d.name.key(other);
                Flow::Stay
            }
        }
    }

    fn files_click(&mut self, d: &mut FileDlg, act: Act, double: bool) -> Flow {
        match act {
            Act::Name => {
                d.list = false;
                Flow::Stay
            }
            Act::Drop => {
                d.info = None;
                d.drop = match d.drop {
                    Some(_) => None,
                    None => Some(
                        self.pattern_choices()
                            .iter()
                            .position(|p| p.eq_ignore_ascii_case(&d.pattern))
                            .unwrap_or(0),
                    ),
                };
                Flow::Stay
            }
            Act::Pat(i) => {
                self.choose_pattern(d, i);
                Flow::Stay
            }
            Act::Item(i) => {
                d.list = true;
                let was = d.sel;
                d.sel = i.min(d.visible().len().saturating_sub(1));
                if double && was == d.sel {
                    self.files_open_selected(d)
                } else {
                    Flow::Stay
                }
            }
            Act::Dlg(Dlg::Ok) => {
                if d.list {
                    self.files_open_selected(d)
                } else {
                    self.files_accept_name(d)
                }
            }
            Act::Dlg(Dlg::Cancel) => Flow::Close,
            _ => Flow::Stay,
        }
    }

    fn render_files(&self, s: &mut Screen, d: &FileDlg) {
        window(s, 15, 2, 49, 19, self.t(CText::FTitle), true);
        caption(s, 18, 4, self.t(CText::FName), !d.list);
        // name field: 22 cells wide + the drop-down button
        s.fill(18, 5, 28, 1, ' ', BLUE, BLUE);
        // a long text shows its end, with the cursor in view
        let chars: Vec<char> = d.name.text.chars().collect();
        let start = if chars.len() <= 26 {
            0
        } else {
            (d.name.cursor + 1).saturating_sub(26).min(chars.len() - 26)
        };
        let shown: String = chars.iter().skip(start).take(26).collect();
        if d.list {
            s.text(19, 5, &shown, WHITE, BLUE);
        } else if d.name.fresh {
            s.text(19, 5, &shown, WHITE, GREEN);
        } else {
            s.text(19, 5, &shown, WHITE, BLUE);
            let at = 19 + d.name.cursor.saturating_sub(start).min(25);
            let ch = chars.get(d.name.cursor).copied().unwrap_or(' ');
            if super::ui::cursor_on() {
                s.put(at, 5, ch, BLUE, WHITE);
            }
        }
        s.put(46, 5, '\u{258c}', GREY, GREEN);
        s.put(47, 5, '\u{2193}', BLACK, GREEN);
        s.put(48, 5, '\u{258c}', GREEN, GREY);
        self.hit(18, 5, 31, 1, Act::Name);
        self.hit(46, 5, 3, 1, Act::Drop);
        button(s, 51, 5, 9, self.t(CText::BOkUpper), false, true);
        s.put(54, 5, 'O', LIGHT_CYAN, GREEN);
        self.hit(51, 5, 10, 2, Act::Dlg(Dlg::Ok));
        caption(s, 18, 7, self.t(CText::FFiles), false);
        // two-column list on cyan
        s.fill(18, 8, 15, 8, ' ', CYAN, CYAN);
        s.fill(33, 8, 1, 8, '\u{2502}', BLUE, CYAN);
        s.fill(34, 8, 15, 8, ' ', CYAN, CYAN);
        let visible = d.visible();
        let page = (d.sel / 16) * 16;
        for (k, (name, dir)) in visible.iter().skip(page).take(16).enumerate() {
            let i = page + k;
            let (col, row) = (k / 8, k % 8);
            let text = if *dir && !name.starts_with("[-") {
                format!("{name}\\")
            } else {
                name.clone()
            };
            // the column holds 14 cells: a longer name ends with ~
            let text = if text.chars().count() > 14 {
                format!("{}~", text.chars().take(13).collect::<String>())
            } else {
                text
            };
            let x = 19 + col * 16;
            let (fg, bg) = if i == d.sel && d.list {
                (WHITE, BLUE)
            } else if i == d.sel {
                (YELLOW, CYAN)
            } else {
                (BLACK, CYAN)
            };
            if bg != CYAN {
                s.fill(x - 1, 8 + row, 15, 1, ' ', fg, bg);
            }
            s.text(x, 8 + row, &text, fg, bg);
            self.hit(x - 1, 8 + row, 15, 1, Act::Item(i));
        }
        // scroll bar: the thumb follows the page
        s.fill(18, 16, 31, 1, '\u{2592}', CYAN, BLUE);
        s.put(18, 16, '\u{25c4}', CYAN, BLUE);
        let pages = visible.len().div_ceil(16).max(1);
        let thumb = 19 + (page / 16) * 28 / pages.max(1);
        s.put(thumb.min(47), 16, '\u{25a0}', CYAN, BLUE);
        s.put(48, 16, '\u{25ba}', CYAN, BLUE);
        button(s, 51, 8, 9, self.t(CText::BCancel), false, true);
        self.hit(51, 8, 10, 2, Act::Dlg(Dlg::Cancel));
        // path and file line
        s.fill(16, 18, 47, 2, ' ', BLUE, BLUE);
        let path = match &d.zip {
            Some(z) => format!("{}\\", z.path.display()),
            None => format!("{}\\{}", self.engine.dir().display(), d.pattern),
        };
        s.text(17, 18, &shorten_path(&path, 44), CYAN, BLUE);
        if let Some(info) = &d.info {
            s.text(17, 19, info, YELLOW, BLUE);
        } else if let Some((name, dir)) = visible.get(d.sel) {
            let size = if *dir {
                String::new()
            } else {
                std::fs::metadata(self.engine.dir().join(name))
                    .map(|m| format!("{}", m.len()))
                    .unwrap_or_default()
            };
            s.text(17, 19, &format!("{name}  {size}"), CYAN, BLUE);
        }
        // the drop-down list of patterns, over the file list
        if let Some(cur) = d.drop {
            let choices = self.pattern_choices();
            s.fill(18, 6, 31, choices.len(), ' ', BLACK, GREY);
            for (i, p) in choices.iter().enumerate() {
                let (fg, bg) = if i == cur {
                    (WHITE, BLUE)
                } else {
                    (BLACK, GREY)
                };
                s.fill(18, 6 + i, 31, 1, ' ', fg, bg);
                s.text(19, 6 + i, p, fg, bg);
                self.hit(18, 6 + i, 31, 1, Act::Pat(i));
            }
        }
    }

    // ------------------------------------------------------------------------------------------ Tool Data

    pub(crate) fn open_tools(&mut self) -> Dialog {
        Dialog::Tools(ToolDlg {
            rows: self.engine.tool_rows(),
            focus: ToolFocus::None,
            edit: None,
            error: None,
            status: None,
        })
    }

    fn tool_cells(&self, d: &ToolDlg) -> Vec<(usize, usize)> {
        let cols = self.tool_columns();
        let mut v = Vec::new();
        for (r, row) in d.rows.iter().enumerate() {
            if row.name.is_some() {
                for c in 0..cols {
                    v.push((r, c));
                }
            }
        }
        v
    }

    fn tool_columns(&self) -> usize {
        if self.engine.kind() == Kind::Route {
            5
        } else {
            3
        }
    }

    /// Takes the typed text into the cell; false (with the error shown) when it is out of range.
    fn tool_commit(&mut self, d: &mut ToolDlg) -> bool {
        let (ToolFocus::Cell(r, c), Some(e)) = (d.focus, d.edit.take()) else {
            return true;
        };
        let (_, lo, hi, dec, width) = tool_spec(self.engine.kind() == Kind::Route, c);
        match e.text.trim().replace(',', ".").parse::<f64>() {
            Ok(v) if v >= lo && v <= hi => {
                d.rows[r].cells[c] = pad_number(v, dec, width);
                d.error = None;
                true
            }
            _ => {
                d.error = Some(format!(
                    "{} {}{hi} {}{lo}",
                    self.t(CText::InputError),
                    self.t(CText::Max),
                    self.t(CText::Min)
                ));
                d.edit = Some(e);
                false
            }
        }
    }

    fn tool_focus(&mut self, d: &mut ToolDlg, to: ToolFocus) {
        if !self.tool_commit(d) {
            return;
        }
        d.focus = to;
        d.status = None;
        if let ToolFocus::Cell(r, c) = to {
            let (max, ..) = tool_spec(self.engine.kind() == Kind::Route, c);
            d.edit = Some(LineEdit::new(&d.rows[r].cells[c], EditKind::Number, max));
        } else {
            d.edit = None;
        }
    }

    /// The control after (`delta` 1) or before (-1) the focused one: the cells row by row, then the four buttons.
    fn tool_step(&mut self, d: &mut ToolDlg, delta: isize) {
        let mut ring: Vec<ToolFocus> = self
            .tool_cells(d)
            .into_iter()
            .map(|(r, c)| ToolFocus::Cell(r, c))
            .collect();
        ring.extend([Dlg::Load, Dlg::Save, Dlg::Ok, Dlg::Abort].map(ToolFocus::Btn));
        let at = ring.iter().position(|f| *f == d.focus);
        let next = match at {
            Some(i) => (i as isize + delta).rem_euclid(ring.len() as isize) as usize,
            None => {
                if delta > 0 {
                    0
                } else {
                    ring.len() - 1
                }
            }
        };
        self.tool_focus(d, ring[next]);
    }

    fn tool_button(&mut self, d: &mut ToolDlg, b: Dlg) -> Flow {
        match b {
            Dlg::Abort => Flow::Close,
            Dlg::Ok => {
                if !self.tool_commit(d) {
                    return Flow::Stay;
                }
                self.engine.apply_tool_rows(&d.rows);
                Flow::Close
            }
            Dlg::Save => {
                if !self.tool_commit(d) {
                    return Flow::Stay;
                }
                self.engine.apply_tool_rows(&d.rows);
                match self.engine.save_tools() {
                    Ok(p) => {
                        d.status = Some(format!("{} {}", self.t(CText::Saved), file_label_of(&p)));
                        d.error = None;
                    }
                    Err(e) => d.error = Some(format!("{}: {e}", self.t(CText::ErrWrite))),
                }
                Flow::Stay
            }
            Dlg::Load => Flow::to(self.open_files_for(Purpose::Tools)),
            _ => Flow::Stay,
        }
    }

    fn tools_key(&mut self, d: &mut ToolDlg, input: Input) -> Flow {
        if let Some(e) = d.edit.as_mut() {
            if e.key(input) {
                return Flow::Stay;
            }
        }
        let editing = d.edit.is_some();
        match input {
            Input::Esc => {
                if editing {
                    d.edit = None;
                    d.focus = ToolFocus::None;
                    d.error = None;
                    Flow::Stay
                } else {
                    Flow::Close
                }
            }
            Input::Tab | Input::Right => {
                self.tool_step(d, 1);
                Flow::Stay
            }
            Input::BackTab | Input::Left => {
                self.tool_step(d, -1);
                Flow::Stay
            }
            Input::Down => {
                match d.focus {
                    ToolFocus::Cell(r, c) => {
                        // the same column of the next place that holds a tool
                        let next = (r + 1..d.rows.len()).find(|&i| d.rows[i].name.is_some());
                        match next {
                            Some(i) => self.tool_focus(d, ToolFocus::Cell(i, c)),
                            None => self.tool_focus(d, ToolFocus::Btn(Dlg::Ok)),
                        }
                    }
                    _ => self.tool_step(d, 1),
                }
                Flow::Stay
            }
            Input::Up => {
                match d.focus {
                    ToolFocus::Cell(r, c) => {
                        if let Some(i) = (0..r).rev().find(|&i| d.rows[i].name.is_some()) {
                            self.tool_focus(d, ToolFocus::Cell(i, c));
                        }
                    }
                    _ => self.tool_step(d, -1),
                }
                Flow::Stay
            }
            Input::Enter => match d.focus {
                ToolFocus::Btn(b) => self.tool_button(d, b),
                ToolFocus::Cell(r, c) => {
                    let next = (r + 1..d.rows.len()).find(|&i| d.rows[i].name.is_some());
                    match next {
                        Some(i) => self.tool_focus(d, ToolFocus::Cell(i, c)),
                        None => self.tool_focus(d, ToolFocus::Btn(Dlg::Ok)),
                    }
                    Flow::Stay
                }
                ToolFocus::None => self.tool_button(d, Dlg::Ok),
            },
            Input::Char(' ') if matches!(d.focus, ToolFocus::Btn(_)) => {
                let ToolFocus::Btn(b) = d.focus else {
                    return Flow::Stay;
                };
                self.tool_button(d, b)
            }
            Input::Alt(c) => self.tool_hot(d, c),
            Input::Char(c) if !editing => self.tool_hot(d, c),
            _ => Flow::Stay,
        }
    }

    fn tool_hot(&mut self, d: &mut ToolDlg, c: char) -> Flow {
        let c = c.to_ascii_lowercase();
        let b = [
            (CText::BLoad, Dlg::Load),
            (CText::BOk, Dlg::Ok),
            (CText::BAbort, Dlg::Abort),
        ]
        .into_iter()
        .find(|(k, _)| hot_letter(self.t(*k)) == Some(c))
        .map(|(_, b)| b);
        match b {
            Some(b) => self.tool_button(d, b),
            None => Flow::Stay,
        }
    }

    fn tools_click(&mut self, d: &mut ToolDlg, act: Act) -> Flow {
        match act {
            Act::Cell(r, c) => {
                if d.focus != ToolFocus::Cell(r, c) {
                    self.tool_focus(d, ToolFocus::Cell(r, c));
                }
                Flow::Stay
            }
            Act::Dlg(b) => self.tool_button(d, b),
            _ => Flow::Stay,
        }
    }

    fn render_tools(&self, s: &mut Screen, d: &ToolDlg) {
        let route = self.engine.kind() == Kind::Route;
        let (x, w) = if route { (4, 72) } else { (11, 57) };
        window(s, x, 0, w, 23, self.t(CText::TTitle), true);
        // the editable cells: (x of the field, text width)
        let cols: &[(usize, usize)] = if route {
            &[(16, 5), (25, 2), (31, 4), (39, 4), (47, 5)]
        } else {
            &[(23, 4), (31, 4), (39, 5)]
        };
        // headings: (x, text, x of the unit line, unit)
        let heads: Vec<Heading> = if route {
            vec![
                (7, CText::TPl, None),
                (11, CText::TName, None),
                (17, CText::TDepth, Some((18, "mm"))),
                (25, CText::TIter, Some((26, "/n"))),
                (31, CText::TXFeed, Some((31, "mm/min"))),
                (39, CText::TZFeed, Some((39, "mm/min"))),
                (47, CText::TSpindle, Some((48, "1/min"))),
                (56, CText::TStart, Some((56, "Vect."))),
                (63, CText::TUsed, Some((64, "mm"))),
                (69, CText::TTab, None),
            ]
        } else {
            vec![
                (14, CText::TPl, None),
                (18, CText::TName, None),
                (23, CText::TStroke, Some((24, "+ mm"))),
                (32, CText::TFeed, Some((31, "mm/min"))),
                (39, CText::TSpindle, Some((40, "1/min"))),
                (48, CText::TStart, Some((48, "Vect."))),
                (56, CText::TUsed, Some((57, "mm"))),
                (62, CText::TTab, None),
            ]
        };
        for (hx, k, unit) in heads {
            label(s, hx, 2, self.t(k));
            if let Some((ux, u)) = unit {
                label(s, ux, 3, u);
            }
        }
        // the places hold the tools the loaded file uses, with their data looked up in the tool table
        let (num_x, name_x, start_x, used_x, tab_x) = if route {
            (7, 12, 56, 63, 69)
        } else {
            (14, 19, 48, 55, 62)
        };
        for (i, row) in d.rows.iter().enumerate() {
            let y = 4 + i;
            label(s, num_x, y, &format!("{:02}", i + 1));
            label(
                s,
                name_x,
                y,
                &row.name.map_or("--".to_string(), |n| format!("{n:02}")),
            );
            for (c, (fx, fw)) in cols.iter().enumerate() {
                let text = row.cells.get(c).map(String::as_str).unwrap_or("");
                match (&d.edit, d.focus == ToolFocus::Cell(i, c)) {
                    (Some(e), true) => edit_field(s, *fx, y, *fw, e),
                    _ => field(s, *fx, y, *fw, text, false),
                }
                if row.name.is_some() {
                    self.hit(*fx, y, fw + 2, 1, Act::Cell(i, c));
                }
            }
            if row.name.is_some() {
                label(s, start_x, y, &format!("{:>5}", row.first_vector));
                label(s, used_x, y, &format!("{:>5}", 0));
                if row.in_table {
                    label(s, tab_x, y, "\u{221a}");
                }
            } else {
                label(s, start_x, y, ".....");
                label(s, used_x, y, ".....");
            }
        }
        let bx = if route {
            [16usize, 29, 42, 55]
        } else {
            [15, 28, 41, 54]
        };
        let btns = [
            (Dlg::Load, CText::BLoad),
            (Dlg::Save, CText::BSave),
            (Dlg::Ok, CText::BOk),
            (Dlg::Abort, CText::BAbort),
        ];
        for (k, (b, text)) in btns.into_iter().enumerate() {
            let focused = match d.focus {
                ToolFocus::Btn(f) => f == b,
                ToolFocus::None => b == Dlg::Ok,
                ToolFocus::Cell(..) => false,
            };
            button(s, bx[k], 20, 9, self.t(text), focused, true);
            self.hit(bx[k], 20, 10, 2, Act::Dlg(b));
        }
        if let Some(e) = &d.error {
            s.text(x + 2, 18 + 1, e, RED, GREY);
        } else if let Some(st) = &d.status {
            s.text(x + 2, 18 + 1, st, GREEN, GREY);
        }
    }

    // ------------------------------------------------------------------------------------------ Parameter

    pub(crate) fn open_parameter(&mut self) -> Dialog {
        let m = self.engine.settings().cloned();
        let c = self.engine.config().cloned();
        let v = |x: Option<String>| x.unwrap_or_default();
        let route = self.engine.kind() == Kind::Route;
        let mut values = vec![
            v(c.as_ref().map(|c| c.directory.clone())),
            v(c.as_ref().map(|c| c.default_extension.clone())),
        ];
        for i in 2..=16 {
            values.push(v(m.as_ref().map(|m| match i {
                2 => param_format(i, f64::from(m.speed_xy.0)),
                3 => param_format(i, f64::from(m.speed_xy.1)),
                4 => param_format(i, f64::from(m.speed_xy.2)),
                5 => param_format(i, f64::from(m.speed_z.0)),
                6 => param_format(i, f64::from(m.speed_z.1)),
                7 => param_format(i, f64::from(m.speed_z.2)),
                8 => param_format(i, f64::from(m.accel.0)),
                9 => param_format(i, f64::from(m.accel.1)),
                10 => param_format(i, m.correction.0),
                11 => param_format(i, m.correction.1),
                12 => param_format(i, m.correction.2),
                13 => param_format(i, m.z_max_depth_mm),
                14 => param_format(i, m.distance_tool_board_mm),
                15 => param_format(i, m.spindle_start_time),
                _ => param_format(i, f64::from(m.chord_angle)),
            })));
        }
        let unit = m.as_ref().map_or(1, |m| m.unit.index() as usize);
        let sorting = m.as_ref().map_or(3, |m| m.sorting.index() as usize);
        let lang = Lang::ALL
            .iter()
            .position(|&l| l == self.engine.lang())
            .unwrap_or(0);
        let mut ring = vec![Ctl::Conn, Ctl::Text(0), Ctl::Text(1), Ctl::Unit];
        ring.push(if route { Ctl::Text(16) } else { Ctl::Sort });
        ring.extend((2..=15).map(Ctl::Text));
        let auto = m
            .as_ref()
            .filter(|m| !m.atc.is_empty() && !route)
            .map(|m| m.auto_change);
        if auto.is_some() {
            ring.push(Ctl::Auto);
        }
        ring.push(Ctl::Lang);
        ring.extend([Dlg::Load, Dlg::Save, Dlg::Ok, Dlg::Abort].map(Ctl::Btn));
        Dialog::Parameter(ParamDlg {
            values,
            unit,
            sorting,
            lang,
            auto,
            ring,
            focus: None,
            edit: None,
            error: None,
            status: None,
            conn: super::connect::ConnState::from_config(&self.engine.connection()),
        })
    }

    /// Takes the typed text of the focused field into its value; false (with the error shown) when it is not valid.
    fn param_commit(&mut self, d: &mut ParamDlg) -> bool {
        let (Some(i), Some(e)) = (d.focus, d.edit.take()) else {
            return true;
        };
        let Ctl::Text(k) = d.ring[i] else {
            return true;
        };
        if k <= 1 {
            let t = e.text.trim().to_string();
            if t.is_empty() {
                d.error = Some(self.t(CText::InputError).to_string());
                d.edit = Some(e);
                return false;
            }
            d.values[k] = t;
            d.error = None;
            return true;
        }
        let (_, _, lo, hi) = param_spec(k);
        match e.text.trim().replace(',', ".").parse::<f64>() {
            Ok(v) if v >= lo && v <= hi && !(k == 16 && v == 0.0) => {
                d.values[k] = param_format(k, v);
                d.error = None;
                true
            }
            _ => {
                d.error = Some(format!(
                    "{} {}{hi} {}{lo}",
                    self.t(CText::InputError),
                    self.t(CText::Max),
                    self.t(CText::Min)
                ));
                d.edit = Some(e);
                false
            }
        }
    }

    fn param_focus(&mut self, d: &mut ParamDlg, idx: usize) {
        if !self.param_commit(d) {
            return;
        }
        d.focus = Some(idx);
        d.status = None;
        d.edit = match d.ring[idx] {
            Ctl::Text(k) => {
                let (kind, max, ..) = param_spec(k);
                Some(LineEdit::new(&d.values[k], kind, max))
            }
            _ => None,
        };
    }

    fn param_step(&mut self, d: &mut ParamDlg, delta: isize) {
        let n = d.ring.len() as isize;
        let next = match d.focus {
            Some(i) => (i as isize + delta).rem_euclid(n) as usize,
            None => {
                if delta > 0 {
                    0
                } else {
                    d.ring.len() - 1
                }
            }
        };
        self.param_focus(d, next);
    }

    /// Changes the selection of a radio group or of the language by `delta`.
    fn param_cycle(&mut self, d: &mut ParamDlg, ctl: Ctl, delta: isize) {
        let step = |v: usize, n: usize| (v as isize + delta).rem_euclid(n as isize) as usize;
        match ctl {
            Ctl::Unit => d.unit = step(d.unit, 4),
            Ctl::Sort => d.sorting = step(d.sorting, 4),
            Ctl::Auto => d.auto = d.auto.map(|a| !a),
            Ctl::Lang => {
                d.lang = step(d.lang, Lang::ALL.len());
                // the language changes at once, so that the choice can be seen
                self.engine.set_lang(Lang::ALL[d.lang]);
            }
            _ => {}
        }
    }

    /// Takes the dialog's values into the settings the program works with.
    fn param_apply(&mut self, d: &ParamDlg) -> Result<(), String> {
        let route = self.engine.kind() == Kind::Route;
        let num = |i: usize| -> f64 { d.values[i].trim().parse().unwrap_or(0.0) };
        let Some(m) = self.engine.settings_mut() else {
            return Err("no configuration file".into());
        };
        m.speed_xy = (num(2) as u32, num(3) as u32, num(4) as u32);
        m.speed_z = (num(5) as u32, num(6) as u32, num(7) as u32);
        m.accel = (num(8) as u32, num(9) as u32);
        m.correction = (num(10), num(11), num(12));
        m.z_max_depth_mm = num(13);
        m.distance_tool_board_mm = num(14);
        m.spindle_start_time = num(15);
        m.unit = CoordUnit::from_index(d.unit as u32).unwrap_or(CoordUnit::Mil);
        if let Some(a) = d.auto {
            m.auto_change = a;
        }
        if route {
            m.chord_angle = num(16) as i32;
        } else {
            m.sorting = [
                SortMode::None,
                SortMode::XFirst,
                SortMode::YFirst,
                SortMode::Special,
            ][d.sorting.min(3)];
        }
        if let Some(c) = self.engine.config_mut() {
            c.directory = d.values[0].clone();
            c.default_extension = d.values[1].clone();
        }
        self.engine.settings_changed();
        self.dirty = true;
        // the connection chosen in its window is applied to the program and written into the machine file
        if d.conn.dirty {
            let config = d
                .conn
                .config(&self.engine.connection())
                .map_err(|why| self.t(why).to_string())?;
            self.engine.set_connection(config.clone());
            crate::protocol::connection::save_connection(self.machine_file.as_deref(), &config)
                .map_err(|_| self.t(CText::ConnNoSave).to_string())?;
        }
        Ok(())
    }

    fn param_button(&mut self, d: &mut ParamDlg, b: Dlg) -> Flow {
        match b {
            Dlg::Abort => Flow::Close,
            Dlg::Ok => {
                if !self.param_commit(d) {
                    return Flow::Stay;
                }
                match self.param_apply(d) {
                    Ok(()) => Flow::Close,
                    Err(e) => {
                        d.error = Some(e);
                        Flow::Stay
                    }
                }
            }
            Dlg::Save => {
                if !self.param_commit(d) {
                    return Flow::Stay;
                }
                let r = self.param_apply(d).and_then(|()| self.engine.save_config());
                match r {
                    Ok(p) => {
                        self.dirty = false;
                        d.status = Some(format!("{} {}", self.t(CText::Saved), file_label_of(&p)));
                        d.error = None;
                    }
                    Err(e) => d.error = Some(format!("{}: {e}", self.t(CText::ErrWrite))),
                }
                Flow::Stay
            }
            Dlg::Load => Flow::to(self.open_files_for(Purpose::Config)),
            _ => Flow::Stay,
        }
    }

    /// The control a hot letter leads to.
    fn param_hot(&self, c: char) -> Option<Ctl> {
        let c = c.to_ascii_lowercase();
        let route = self.engine.kind() == Kind::Route;
        let list = [
            (CText::PFile, Ctl::Text(0)),
            (CText::PCoordinates, Ctl::Unit),
            (
                if route {
                    CText::PChord
                } else {
                    CText::PSorting
                },
                if route { Ctl::Text(16) } else { Ctl::Sort },
            ),
            (CText::PSpeed, Ctl::Text(2)),
            (CText::PRamp, Ctl::Text(8)),
            (CText::PStroke, Ctl::Text(13)),
            (CText::PCorrect, Ctl::Text(10)),
            (CText::PSpindle, Ctl::Text(15)),
            (CText::PLanguage, Ctl::Lang),
            (CText::BLoad, Ctl::Btn(Dlg::Load)),
            (CText::BOk, Ctl::Btn(Dlg::Ok)),
            (CText::BAbort, Ctl::Btn(Dlg::Abort)),
        ];
        list.into_iter()
            .find(|(k, _)| hot_letter(self.t(*k)) == Some(c))
            .map(|(_, ctl)| ctl)
    }

    fn param_go(&mut self, d: &mut ParamDlg, ctl: Ctl) -> Flow {
        if let Ctl::Btn(b) = ctl {
            return self.param_button(d, b);
        }
        if let Some(i) = d.ring.iter().position(|c| *c == ctl) {
            self.param_focus(d, i);
        }
        Flow::Stay
    }

    fn param_key(&mut self, d: &mut ParamDlg, input: Input) -> Flow {
        if d.conn.open {
            return self.conn_key(d, input);
        }
        if let Some(e) = d.edit.as_mut() {
            if e.key(input) {
                return Flow::Stay;
            }
        }
        let cur = d.focus.map(|i| d.ring[i]);
        let on_text = matches!(cur, Some(Ctl::Text(_)));
        match input {
            Input::Esc => {
                if d.edit.is_some() {
                    d.edit = None;
                    d.focus = None;
                    d.error = None;
                    Flow::Stay
                } else {
                    Flow::Close
                }
            }
            Input::Tab => {
                self.param_step(d, 1);
                Flow::Stay
            }
            Input::BackTab => {
                self.param_step(d, -1);
                Flow::Stay
            }
            Input::Enter => match cur {
                Some(Ctl::Btn(b)) => self.param_button(d, b),
                Some(Ctl::Conn) => {
                    self.conn_open(d);
                    Flow::Stay
                }
                Some(_) => {
                    self.param_step(d, 1);
                    Flow::Stay
                }
                None => Flow::Close,
            },
            Input::Down | Input::Right => {
                match cur {
                    Some(c @ (Ctl::Unit | Ctl::Sort | Ctl::Lang)) => self.param_cycle(d, c, 1),
                    _ => self.param_step(d, 1),
                }
                Flow::Stay
            }
            Input::Up | Input::Left => {
                match cur {
                    Some(c @ (Ctl::Unit | Ctl::Sort | Ctl::Lang)) => self.param_cycle(d, c, -1),
                    _ => self.param_step(d, -1),
                }
                Flow::Stay
            }
            Input::Char(' ') if !on_text => match cur {
                Some(Ctl::Btn(b)) => self.param_button(d, b),
                Some(c) => {
                    self.param_cycle(d, c, 1);
                    Flow::Stay
                }
                None => Flow::Stay,
            },
            // the hot letter of the AutoChg check box (where the model has one)
            Input::Char(c) | Input::Alt(c)
                if d.auto.is_some()
                    && c.eq_ignore_ascii_case(&'u')
                    && (!on_text || matches!(input, Input::Alt(_))) =>
            {
                self.param_cycle(d, Ctl::Auto, 1);
                Flow::Stay
            }
            Input::Alt(c) => match self.param_hot(c) {
                Some(ctl) => self.param_go(d, ctl),
                None => Flow::Stay,
            },
            Input::Char(c) if !on_text => match self.param_hot(c) {
                Some(ctl) => self.param_go(d, ctl),
                None => Flow::Stay,
            },
            _ => Flow::Stay,
        }
    }

    fn param_click(&mut self, d: &mut ParamDlg, act: Act) -> Flow {
        if d.conn.open {
            return self.conn_click(d, act);
        }
        match act {
            Act::Ctl(i) => {
                if d.focus != Some(i) {
                    self.param_focus(d, i);
                }
                // the connection opens its window
                if d.ring[i] == Ctl::Conn {
                    self.conn_open(d);
                }
                // the language is chosen by clicking its field: each click takes the next one
                if matches!(d.ring[i], Ctl::Lang | Ctl::Auto) {
                    self.param_cycle(d, d.ring[i], 1);
                }
                Flow::Stay
            }
            Act::LangStep(delta) => {
                if let Some(i) = d.ring.iter().position(|c| *c == Ctl::Lang) {
                    self.param_focus(d, i);
                }
                self.param_cycle(d, Ctl::Lang, delta);
                Flow::Stay
            }
            Act::Unit(k) => {
                if let Some(i) = d.ring.iter().position(|c| *c == Ctl::Unit) {
                    self.param_focus(d, i);
                }
                d.unit = k.min(3);
                Flow::Stay
            }
            Act::Sort(k) => {
                if let Some(i) = d.ring.iter().position(|c| *c == Ctl::Sort) {
                    self.param_focus(d, i);
                }
                d.sorting = k.min(3);
                Flow::Stay
            }
            Act::Dlg(b) => self.param_button(d, b),
            _ => Flow::Stay,
        }
    }

    fn render_parameter(&self, s: &mut Screen, d: &ParamDlg) {
        window(s, 5, 1, 70, 21, "", false);
        let title = self.t(CText::PTitle);
        label(
            s,
            5 + (70usize.saturating_sub(str_width(title))) / 2 - 1,
            2,
            title,
        );
        let route = self.engine.kind() == Kind::Route;
        let ring_at = |c: Ctl| d.ring.iter().position(|x| *x == c).unwrap_or(0);
        let focused = |c: Ctl| d.focus.map(|i| d.ring[i]) == Some(c);
        // the connection with the machine: the way chosen, and the window with what it needs opens from it
        caption(
            s,
            10,
            3,
            self.t(CText::ConnLabel),
            focused(Ctl::Conn) || d.conn.open,
        );
        field(
            s,
            22,
            3,
            38,
            self.t(super::connect::NAMES[d.conn.kind]),
            focused(Ctl::Conn),
        );
        s.put(60, 3, '\u{25bc}', BLACK, GREEN);
        self.hit(10, 3, 52, 1, Act::Ctl(ring_at(Ctl::Conn)));
        caption(s, 10, 4, self.t(CText::PFile), focused(Ctl::Text(0)));
        caption(s, 35, 4, self.t(CText::PCoordinates), focused(Ctl::Unit));
        caption(
            s,
            54,
            4,
            self.t(if route {
                CText::PChord
            } else {
                CText::PSorting
            }),
            focused(Ctl::Sort) || focused(Ctl::Text(16)),
        );
        // a text field: the edited one is green with its cursor
        let text_field = |s: &mut Screen, k: usize, x: usize, y: usize, w: usize| {
            match (&d.edit, focused(Ctl::Text(k))) {
                (Some(e), true) => edit_field(s, x, y, w, e),
                _ => field(s, x, y, w, &d.values[k], false),
            }
            self.hit(x, y, w + 2, 1, Act::Ctl(ring_at(Ctl::Text(k))));
        };
        label(s, 11, 5, self.t(CText::PDirectory));
        text_field(s, 0, 11, 6, 17);
        label(s, 11, 7, self.t(CText::PPattern));
        text_field(s, 1, 11, 8, 12);
        let units: Vec<String> = [
            CText::PUnit110,
            CText::PUnit1,
            CText::PUnit0025,
            CText::PUnit001,
        ]
        .iter()
        .map(|k| self.t(*k).to_string())
        .collect();
        radio_group(s, 36, 5, 14, &units, d.unit, focused(Ctl::Unit));
        for i in 0..4 {
            self.hit(36, 5 + i, 14, 1, Act::Unit(i));
        }
        if route {
            text_field(s, 16, 55, 5, 3);
        } else {
            let sorts: Vec<String> = [
                CText::PSortNone,
                CText::PSortX,
                CText::PSortY,
                CText::PSortSpecial,
            ]
            .iter()
            .map(|k| self.t(*k).to_string())
            .collect();
            radio_group(s, 55, 5, 14, &sorts, d.sorting, focused(Ctl::Sort));
            for i in 0..4 {
                self.hit(55, 5 + i, 14, 1, Act::Sort(i));
            }
        }
        caption(s, 10, 10, self.t(CText::PSpeed), focused(Ctl::Text(2)));
        caption(s, 35, 10, self.t(CText::PRamp), focused(Ctl::Text(8)));
        caption(s, 54, 10, self.t(CText::PStroke), focused(Ctl::Text(13)));
        let rows = [
            (11, 11, CText::PXyMax, 2usize),
            (12, 11, CText::PXyRef, 3),
            (13, 11, CText::PXyRamp, 4),
            (15, 11, CText::PZMax, 5),
            (16, 11, CText::PZRef, 6),
            (17, 11, CText::PZRamp, 7),
        ];
        for (y, x, k, i) in rows {
            label(s, x, y, self.t(k));
            text_field(s, i, 21, y, 4);
        }
        label(s, 36, 11, "XY");
        text_field(s, 8, 40, 11, 2);
        label(s, 36, 12, "Z");
        text_field(s, 9, 40, 12, 2);
        label(s, 55, 11, self.t(CText::PLimit));
        text_field(s, 13, 62, 11, 5);
        label(s, 55, 12, self.t(CText::PFree));
        text_field(s, 14, 62, 12, 5);
        caption(s, 35, 14, self.t(CText::PCorrect), focused(Ctl::Text(10)));
        caption(s, 54, 14, self.t(CText::PSpindle), focused(Ctl::Text(15)));
        for (y, name, i) in [(15, "X", 10usize), (16, "Y", 11), (17, "Z", 12)] {
            label(s, 36, y, name);
            text_field(s, i, 40, y, 6);
        }
        label(s, 55, 15, self.t(CText::PDelay));
        text_field(s, 15, 62, 15, 3);
        label(s, 68, 15, self.t(CText::PSecond));
        // the AutoChg check box of the models with a tool changer, under the spindle
        let mut lang_row = 17;
        if let Some(on) = d.auto {
            let ink = if focused(Ctl::Auto) { WHITE } else { BLACK };
            s.fill(54, 17, 14, 1, ' ', ink, CYAN);
            let mark = if on { 'X' } else { ' ' };
            s.hot_text(55, 17, &format!("[{mark}] A~u~toChg"), ink, HOT, CYAN);
            self.hit(54, 17, 14, 1, Act::Ctl(ring_at(Ctl::Auto)));
            // the language moves down one row
            lang_row = 18;
        }
        // the language, an option of ours in the free place under the spindle
        caption(
            s,
            54,
            lang_row,
            self.t(CText::PLanguage),
            focused(Ctl::Lang),
        );
        let lang_name = Lang::ALL[d.lang].native_name();
        let lang_focus = focused(Ctl::Lang);
        field(s, 63, lang_row, 9, lang_name, lang_focus);
        s.put(63, lang_row, '\u{25c4}', LIGHT_CYAN, BLUE);
        s.put(73, lang_row, '\u{25ba}', LIGHT_CYAN, BLUE);
        self.hit(54, lang_row, 19, 1, Act::Ctl(ring_at(Ctl::Lang)));
        self.hit(63, lang_row, 1, 1, Act::LangStep(-1));
        self.hit(73, lang_row, 1, 1, Act::LangStep(1));
        // buttons
        let btns = [
            (11usize, Dlg::Load, CText::BLoad),
            (26, Dlg::Save, CText::BSave),
            (43, Dlg::Ok, CText::BOk),
            (59, Dlg::Abort, CText::BAbort),
        ];
        for (x, b, text) in btns {
            let on = match d.focus {
                Some(i) => d.ring[i] == Ctl::Btn(b),
                None => b == Dlg::Abort,
            };
            button(s, x, 19, 9, self.t(text), on, true);
            self.hit(x, 19, 10, 2, Act::Dlg(b));
        }
        if let Some(e) = &d.error {
            s.text(11, 18, e, RED, GREY);
        } else if let Some(st) = &d.status {
            s.text(11, 18, st, GREEN, GREY);
        }
        if d.conn.open {
            self.render_conn(s, d);
        }
    }

    // ------------------------------------------------------------------------------------------ Connection

    /// Opens the window of the connection over the Parameter window, starting from the connection the program has now.
    fn conn_open(&mut self, d: &mut ParamDlg) {
        d.conn.commit();
        let dirty = d.conn.dirty;
        d.conn = super::connect::ConnState::from_config(&self.engine.connection());
        if dirty {
            // what was chosen before in this window stays: only the program's own values are read again when nothing was chosen
            d.conn.dirty = true;
        }
        d.conn.open = true;
        d.error = None;
    }

    fn conn_close(&mut self, d: &mut ParamDlg) {
        d.conn.commit();
        d.conn.open = false;
        d.conn.list = false;
        d.conn.error = None;
    }

    /// OK of the connection window: what was typed is checked and kept for the Parameter window to apply.
    fn conn_ok(&mut self, d: &mut ParamDlg) {
        d.conn.commit();
        match d.conn.config(&self.engine.connection()) {
            Ok(_) => {
                d.conn.dirty = true;
                self.conn_close(d);
            }
            Err(why) => d.conn.error = Some(why),
        }
    }

    fn conn_key(&mut self, d: &mut ParamDlg, input: Input) -> Flow {
        use super::connect::Item;
        if let Some(e) = d.conn.edit.as_mut() {
            if e.key(input) {
                return Flow::Stay;
            }
        }
        let cur = d.conn.item();
        match input {
            Input::Esc => {
                if d.conn.list {
                    d.conn.list = false;
                } else {
                    // leaving with Esc drops what was typed in this window
                    d.conn.dirty = false;
                    d.conn.edit = None;
                    self.conn_close(d);
                }
            }
            Input::Tab => d.conn.step(1),
            Input::BackTab => d.conn.step(-1),
            Input::Down | Input::Right if cur == Item::Kind => {
                d.conn
                    .set_kind((d.conn.kind + 1) % super::connect::KINDS.len());
            }
            Input::Up | Input::Left if cur == Item::Kind => {
                let n = super::connect::KINDS.len();
                d.conn.set_kind((d.conn.kind + n - 1) % n);
            }
            Input::Down => d.conn.step(1),
            Input::Up => d.conn.step(-1),
            Input::Enter | Input::Char(' ') if cur == Item::Kind => d.conn.list = !d.conn.list,
            Input::Enter => match cur {
                Item::Ok => self.conn_ok(d),
                Item::Cancel => {
                    d.conn.dirty = false;
                    self.conn_close(d);
                }
                _ => d.conn.step(1),
            },
            _ => {}
        }
        Flow::Stay
    }

    fn conn_click(&mut self, d: &mut ParamDlg, act: Act) -> Flow {
        use super::connect::Item;
        match act {
            Act::Conn(i) => {
                d.conn.go(i);
                match d.conn.item() {
                    Item::Kind => d.conn.list = !d.conn.list,
                    Item::Ok => self.conn_ok(d),
                    Item::Cancel => {
                        d.conn.dirty = false;
                        self.conn_close(d);
                    }
                    Item::Field(_) => {}
                }
            }
            Act::ConnKind(k) => {
                d.conn.set_kind(k);
                d.conn.list = false;
            }
            Act::ConnShield => d.conn.list = false,
            _ => {}
        }
        Flow::Stay
    }

    fn render_conn(&self, s: &mut Screen, d: &ParamDlg) {
        use super::connect::{wrap_note, Item, NAMES, NOTES};
        // nothing under the window answers while it is open
        self.hit(0, 0, 80, 25, Act::ConnShield);
        window(s, 8, 3, 64, 18, "", false);
        let title = self.t(CText::ConnTitle);
        label(
            s,
            8 + 64usize.saturating_sub(str_width(title)) / 2,
            4,
            title,
        );
        let ring = d.conn.ring();
        let focus = d.conn.item();
        let at = |i: Item| ring.iter().position(|x| *x == i).unwrap_or(0);
        caption(s, 11, 6, self.t(CText::ConnType), focus == Item::Kind);
        field(
            s,
            20,
            6,
            32,
            self.t(NAMES[d.conn.kind]),
            focus == Item::Kind,
        );
        s.put(53, 6, '\u{25bc}', BLACK, GREEN);
        self.hit(11, 6, 44, 1, Act::Conn(at(Item::Kind)));
        // what the way chosen is, in one or two lines
        let (first, second) = wrap_note(self.t(NOTES[d.conn.kind]), 58);
        label(s, 11, 8, &first);
        label(s, 11, 9, &second);
        for (k, (name, v)) in d.conn.fields().iter().enumerate() {
            let y = 11 + 2 * k;
            caption(s, 11, y, self.t(*name), focus == Item::Field(k));
            match (&d.conn.edit, focus == Item::Field(k)) {
                (Some(e), true) => edit_field(s, 22, y, 38, e),
                // a token is never shown: it is as many dots as it has letters
                _ if *name == CText::ConnToken => field(
                    s,
                    22,
                    y,
                    38,
                    &"\u{2022}".repeat(d.conn.vals[*v].chars().count()),
                    false,
                ),
                _ => field(s, 22, y, 38, &d.conn.vals[*v], false),
            }
            self.hit(11, y, 51, 1, Act::Conn(at(Item::Field(k))));
        }
        if let Some(e) = d.conn.error {
            s.text(11, 16, self.t(e), RED, GREY);
        }
        for (x, it, text) in [
            (22usize, Item::Ok, CText::BOk),
            (40, Item::Cancel, CText::BAbort),
        ] {
            button(s, x, 18, 9, self.t(text), focus == it, true);
            self.hit(x, 18, 10, 2, Act::Conn(at(it)));
        }
        // the list of the five ways, open over what is under the drop-down
        if d.conn.list {
            s.fill(20, 7, 34, 5, ' ', BLACK, CYAN);
            for (i, n) in NAMES.iter().enumerate() {
                let on = i == d.conn.kind;
                let (fg, bg) = if on { (WHITE, BLUE) } else { (BLACK, CYAN) };
                s.fill(20, 7 + i, 34, 1, ' ', fg, bg);
                s.text(21, 7 + i, self.t(*n), fg, bg);
                self.hit(20, 7 + i, 34, 1, Act::ConnKind(i));
            }
        }
    }

    // ------------------------------------------------------------------------------------------ Panelization

    pub(crate) fn open_panel(&mut self) -> Dialog {
        let (x0, y0, x1, y1) = self.engine.bounds().unwrap_or((0.0, 0.0, 0.0, 0.0));
        let mut d = PanelDlg {
            values: [
                format!("{:06.2}", x1 - x0),
                format!("{:06.2}", y1 - y0),
                "01.00".into(),
                "01.00".into(),
                "0000".into(),
                "0000".into(),
            ],
            ring: vec![
                PzCtl::Value(0),
                PzCtl::Value(1),
                PzCtl::Value(2),
                PzCtl::Value(3),
                PzCtl::Btn(Dlg::Calc),
                PzCtl::Value(4),
                PzCtl::Value(5),
                PzCtl::Btn(Dlg::Ok),
                PzCtl::Btn(Dlg::Abort),
            ],
            focus: None,
            edit: None,
            error: None,
        };
        self.panel_calc(&mut d);
        Dialog::Panel(d)
    }

    /// Copies that fit the working area (inferred from the original's figures: area / (board - spacing)).
    fn panel_calc(&self, d: &mut PanelDlg) {
        let num = |i: usize| -> f64 { d.values[i].trim().parse().unwrap_or(0.0) };
        let n = |a: f64, b: f64, s: f64| {
            let k = b - s;
            if k > 0.0 {
                (a / k).floor().clamp(0.0, 9999.0) as u32
            } else {
                0
            }
        };
        let (nx, ny) = (
            n(self.area.0, num(0), num(2)),
            n(self.area.1, num(1), num(3)),
        );
        d.values[4] = format!("{nx:04}");
        d.values[5] = format!("{ny:04}");
    }

    fn pz_spec(i: usize) -> (usize, f64, f64, usize, usize) {
        // max chars, min, max, decimals, width
        match i {
            0 | 1 => (6, 0.0, 999.99, 2, 6),
            2 | 3 => (5, 0.0, 99.99, 2, 5),
            _ => (4, 0.0, 9999.0, 0, 4),
        }
    }

    fn pz_commit(&mut self, d: &mut PanelDlg) -> bool {
        let (Some(i), Some(e)) = (d.focus, d.edit.take()) else {
            return true;
        };
        let PzCtl::Value(k) = d.ring[i] else {
            return true;
        };
        let (_, lo, hi, dec, width) = Self::pz_spec(k);
        match e.text.trim().replace(',', ".").parse::<f64>() {
            Ok(v) if v >= lo && v <= hi => {
                d.values[k] = pad_number(v, dec, width);
                d.error = None;
                true
            }
            _ => {
                d.error = Some(format!(
                    "{} {}{hi} {}{lo}",
                    self.t(CText::InputError),
                    self.t(CText::Max),
                    self.t(CText::Min)
                ));
                d.edit = Some(e);
                false
            }
        }
    }

    fn pz_focus(&mut self, d: &mut PanelDlg, idx: usize) {
        if !self.pz_commit(d) {
            return;
        }
        d.focus = Some(idx);
        d.edit = match d.ring[idx] {
            PzCtl::Value(k) => {
                let (max, ..) = Self::pz_spec(k);
                Some(LineEdit::new(&d.values[k], EditKind::Number, max))
            }
            PzCtl::Btn(_) => None,
        };
    }

    fn pz_step(&mut self, d: &mut PanelDlg, delta: isize) {
        let n = d.ring.len() as isize;
        let next = match d.focus {
            Some(i) => (i as isize + delta).rem_euclid(n) as usize,
            None => 0,
        };
        self.pz_focus(d, next);
    }

    fn pz_button(&mut self, d: &mut PanelDlg, b: Dlg) -> Flow {
        match b {
            Dlg::Calc => {
                if self.pz_commit(d) {
                    self.panel_calc(d);
                }
                Flow::Stay
            }
            Dlg::Ok => {
                if self.pz_commit(d) {
                    let num = |i: usize| d.values[i].trim().parse::<f64>().unwrap_or(0.0);
                    let (columns, rows) = (num(4) as u32, num(5) as u32);
                    // the copies are `pitch` apart: the board less the spacing, as the original's count works out
                    self.panel = (columns * rows > 1).then_some(engine::Panel {
                        columns,
                        rows,
                        pitch_mm: (num(0) - num(2), num(1) - num(3)),
                    });
                    Flow::Close
                } else {
                    Flow::Stay
                }
            }
            _ => Flow::Close,
        }
    }

    fn panel_key(&mut self, d: &mut PanelDlg, input: Input) -> Flow {
        if let Some(e) = d.edit.as_mut() {
            if e.key(input) {
                return Flow::Stay;
            }
        }
        let cur = d.focus.map(|i| d.ring[i]);
        let editing = d.edit.is_some();
        let hot = |app: &Self, c: char| -> Option<Dlg> {
            let c = c.to_ascii_lowercase();
            [
                (CText::PzCalc, Dlg::Calc),
                (CText::BOk, Dlg::Ok),
                (CText::BAbort, Dlg::Abort),
            ]
            .into_iter()
            .find(|(k, _)| hot_letter(app.t(*k)) == Some(c))
            .map(|(_, b)| b)
        };
        match input {
            Input::Esc => {
                if editing {
                    d.edit = None;
                    d.focus = None;
                    d.error = None;
                    Flow::Stay
                } else {
                    Flow::Close
                }
            }
            Input::Tab | Input::Down | Input::Right => {
                self.pz_step(d, 1);
                Flow::Stay
            }
            Input::BackTab | Input::Up | Input::Left => {
                self.pz_step(d, -1);
                Flow::Stay
            }
            Input::Enter => match cur {
                Some(PzCtl::Btn(b)) => self.pz_button(d, b),
                Some(_) => {
                    self.pz_step(d, 1);
                    Flow::Stay
                }
                None => self.pz_button(d, Dlg::Ok),
            },
            Input::Alt(c) => match hot(self, c) {
                Some(b) => self.pz_button(d, b),
                None => Flow::Stay,
            },
            Input::Char(c) if !editing => match hot(self, c) {
                Some(b) => self.pz_button(d, b),
                None => Flow::Stay,
            },
            _ => Flow::Stay,
        }
    }

    fn panel_click(&mut self, d: &mut PanelDlg, act: Act) -> Flow {
        match act {
            Act::Pz(i) => {
                if d.focus != Some(i) {
                    self.pz_focus(d, i);
                }
                Flow::Stay
            }
            Act::Dlg(b) => self.pz_button(d, b),
            _ => Flow::Stay,
        }
    }

    fn render_panel(&self, s: &mut Screen, d: &PanelDlg) {
        window(s, 23, 2, 34, 18, "", false);
        label(s, 29, 4, self.t(CText::PzTitle));
        let rows = [
            (6, CText::PzBoard, "X", 0usize, 6usize),
            (7, CText::PzBoard, "Y", 1, 6),
            (9, CText::PzSpacing, "X", 2, 5),
            (10, CText::PzSpacing, "Y", 3, 5),
            (14, CText::PzNumber, "X", 4, 4),
            (15, CText::PzNumber, "Y", 5, 4),
        ];
        let ring_at = |c: PzCtl| d.ring.iter().position(|x| *x == c).unwrap_or(0);
        for (y, k, axis, v, w) in rows {
            label(s, 29, y, self.t(k));
            label(s, 37, y, axis);
            let focus = d.focus.map(|i| d.ring[i]) == Some(PzCtl::Value(v));
            match (&d.edit, focus) {
                (Some(e), true) => edit_field(s, 41, y, w, e),
                _ => field(s, 41, y, w, &d.values[v], false),
            }
            self.hit(41, y, w + 2, 1, Act::Pz(ring_at(PzCtl::Value(v))));
        }
        label(s, 30, 8, "(mm)");
        let btns = [
            (29usize, 12usize, Dlg::Calc, CText::PzCalc),
            (29, 17, Dlg::Ok, CText::BOk),
            (41, 17, Dlg::Abort, CText::BAbort),
        ];
        for (x, y, b, text) in btns {
            let on = match d.focus {
                Some(i) => d.ring[i] == PzCtl::Btn(b),
                None => b == Dlg::Ok,
            };
            button(s, x, y, 9, self.t(text), on, true);
            self.hit(x, y, 10, 2, Act::Dlg(b));
        }
        if let Some(e) = &d.error {
            s.text(25, 19, &e.chars().take(30).collect::<String>(), RED, GREY);
        }
    }

    // ------------------------------------------------------------------------------------------ the job

    fn render_running(&self, s: &mut Screen) {
        if !self.engine.started() {
            // the homing: a flat window with the title and the red strip
            window(s, 10, 9, 60, 4, "", false);
            let title = self.t(CText::HomingXy);
            label(
                s,
                10 + (60usize.saturating_sub(str_width(title))) / 2,
                10,
                title,
            );
            let hint = self.t(CText::PressEsc);
            let n = str_width(hint);
            let x = 10 + 60usize.saturating_sub(n).div_ceil(2);
            alert_strip(s, x - 1, 11, n + 2, hint);
            return;
        }
        window(s, 12, 4, 55, 15, "", false);
        let status = if self.engine.done() == 0 {
            CText::JStartPos
        } else {
            CText::Drilling
        };
        label(s, 20, 6, self.t(status));
        // the figures: the position follows the job; the tool's own values appear once it has started drilling
        let view = self.engine.job_view();
        let live = view.as_ref().filter(|_| self.engine.done() > 0);
        label(s, 52, 6, &format!("{:03}", live.map_or(0, |v| v.percent)));
        label(s, 56, 6, "%");
        label(s, 20, 8, self.t(CText::JTool));
        label(s, 39, 8, self.t(CText::JPosition));
        label(s, 22, 9, self.t(CText::JNo));
        label(s, 29, 9, ":");
        label(
            s,
            31,
            9,
            &live
                .and_then(|v| v.tool)
                .map_or("--".to_string(), |t| format!("{t:02}")),
        );
        label(s, 22, 10, self.t(CText::JStroke));
        label(s, 29, 10, ":");
        label(
            s,
            31,
            10,
            &format!("{:05.2}", live.map_or(0.0, |v| v.stroke_mm)),
        );
        let (px, py) = view.as_ref().map_or((0.0, 0.0), |v| (v.x_mm, v.y_mm));
        for (y, axis, v) in [(9, "X", px), (10, "Y", py)] {
            label(s, 41, y, axis);
            label(s, 45, y, "(mm):");
            label(s, 51, y, &format!("{v:06.2}"));
        }
        label(s, 20, 12, self.t(CText::JSpeed));
        label(s, 39, 12, self.t(CText::JCount));
        label(s, 22, 13, "Z");
        label(s, 29, 13, ":");
        label(s, 31, 13, &format!("{:04}", live.map_or(0, |v| v.z_feed)));
        label(s, 22, 14, "1/min");
        label(s, 29, 14, ":");
        label(s, 31, 14, &format!("{:05}", live.map_or(0, |v| v.rpm)));
        label(s, 41, 13, self.t(CText::LblDone));
        label(s, 49, 13, ":");
        label(s, 51, 13, &format!("{:05}", self.engine.done()));
        label(s, 41, 14, self.t(CText::JUse));
        label(s, 45, 14, "(mm):");
        label(
            s,
            51,
            14,
            &format!("{:05.0}", live.map_or(0.0, |v| v.used_mm)),
        );
        // the red strip: one cell of margin on the left, two on the right
        let hint = self.t(CText::PressEsc);
        s.fill(20, 16, 20, 1, ' ', RED, RED);
        s.text(21, 16, hint, YELLOW, RED);
        label(s, 50, 16, &self.file_name());
        if self.interrupted {
            self.render_interrupted(s);
        } else if let Some(q) = self.engine.question() {
            self.render_question(s, q);
        }
    }
}

/// A heading of the Tool Data table: column, text, and the unit line (column, text) under it.
type Heading = (usize, CText, Option<(usize, &'static str)>);

/// Takes a file of a job archive out into the work folder of this run and says where it went.
fn extract_to_temp(archive: &std::path::Path, name: &str) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("eh3d-zip-{}", std::process::id()));
    eh3d_archive::Archive::open(archive).and_then(|mut a| a.extract(name, &dir))
}

/// A path that fits `width` cells: a long one keeps its end (the folder one is in), after `~`.
fn shorten_path(path: &str, width: usize) -> String {
    let n = path.chars().count();
    if n <= width {
        path.to_string()
    } else {
        format!(
            "~{}",
            path.chars().skip(n - (width - 1)).collect::<String>()
        )
    }
}

/// The file name of a path, for the "saved" notes.
fn file_label_of(p: &std::path::Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
