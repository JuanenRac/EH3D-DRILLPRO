//! The classic front end of DRILLPRO: the screens of the MS-DOS program drawn on the 80x25 text screen.
//!
//! It is a thin layer over [`crate::app::App`]: the job planning, the machine link and the file formats
//! stay in the engine; this module owns what is seen and typed (main form, Parameter, Tool Data,
//! Select File, Quit and the job dialogs). The modern front end will sit next to it.

pub mod engine;
pub mod texts;
pub mod ui;

use std::path::PathBuf;

use crate::app::{App, Input};
use crate::i18n::{str_width, Lang};
use crate::screen::*;
use crate::textapp::TextApp;
use engine::{ClassicEngine, Kind};
use texts::{ctr, CText};
use ui::*;

/// A numeric field of the main form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    ScaleX,
    ScaleY,
    OffsetX,
    OffsetY,
    ClipX,
    ClipY,
    From,
    To,
    Board,
    Base,
}

impl Field {
    pub const ALL: [Field; 10] = [
        Field::ScaleX,
        Field::ScaleY,
        Field::OffsetX,
        Field::OffsetY,
        Field::ClipX,
        Field::ClipY,
        Field::From,
        Field::To,
        Field::Board,
        Field::Base,
    ];

    /// Column of the field group (label column) it belongs to: 0 Scale, 1 Offset, 2 Clip, 3 Vectors, 4 Board, 5 Base.
    fn group(self) -> usize {
        match self {
            Field::ScaleX | Field::ScaleY => 0,
            Field::OffsetX | Field::OffsetY => 1,
            Field::ClipX | Field::ClipY => 2,
            Field::From | Field::To => 3,
            Field::Board => 4,
            Field::Base => 5,
        }
    }

    /// Inclusive range accepted when the field is edited.
    fn range(self, area: (f64, f64)) -> (f64, f64) {
        match self {
            Field::ScaleX | Field::ScaleY => (0.0, 10.0),
            Field::OffsetX => (-area.0, area.0),
            Field::OffsetY => (-area.1, area.1),
            Field::ClipX => (0.0, area.0),
            Field::ClipY => (0.0, area.1),
            Field::From | Field::To => (0.0, 99999.0),
            Field::Board | Field::Base => (0.0, 99.99),
        }
    }
}

/// Values of the main form, in the units shown on the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    pub scale: (f64, f64),
    pub offset: (f64, f64),
    pub clip: (f64, f64),
    pub from: usize,
    pub to: usize,
    pub board: f64,
    pub base: f64,
}

impl Form {
    fn get(&self, f: Field) -> f64 {
        match f {
            Field::ScaleX => self.scale.0,
            Field::ScaleY => self.scale.1,
            Field::OffsetX => self.offset.0,
            Field::OffsetY => self.offset.1,
            Field::ClipX => self.clip.0,
            Field::ClipY => self.clip.1,
            Field::From => self.from as f64,
            Field::To => self.to as f64,
            Field::Board => self.board,
            Field::Base => self.base,
        }
    }

    fn set(&mut self, f: Field, v: f64) {
        match f {
            Field::ScaleX => self.scale.0 = v,
            Field::ScaleY => self.scale.1 = v,
            Field::OffsetX => self.offset.0 = v,
            Field::OffsetY => self.offset.1 = v,
            Field::ClipX => self.clip.0 = v,
            Field::ClipY => self.clip.1 = v,
            Field::From => self.from = v as usize,
            Field::To => self.to = v as usize,
            Field::Board => self.board = v,
            Field::Base => self.base = v,
        }
    }

    /// The text of a field as the original shows it.
    pub fn text(&self, f: Field) -> String {
        let v = self.get(f);
        match f {
            Field::ScaleX | Field::ScaleY => format!("{v:.4}"),
            Field::OffsetX | Field::OffsetY => format!("{v:+07.2}"),
            Field::ClipX | Field::ClipY => format!("{v:06.2}"),
            Field::From | Field::To => format!("{:05}", v as usize),
            Field::Board | Field::Base => format!("{v:05.2}"),
        }
    }
}

/// The main-screen buttons, in the order of the original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Config,
    Tools,
    Start,
    Load,
    Panelz,
    Offset,
    Quit,
}

impl Button {
    const ALL: [Button; 7] = [
        Button::Config,
        Button::Tools,
        Button::Start,
        Button::Load,
        Button::Panelz,
        Button::Offset,
        Button::Quit,
    ];

    fn text(self) -> CText {
        match self {
            Button::Config => CText::BtnConfig,
            Button::Tools => CText::BtnTools,
            Button::Start => CText::BtnStart,
            Button::Load => CText::BtnLoad,
            Button::Panelz => CText::BtnPanelz,
            Button::Offset => CText::BtnOffset,
            Button::Quit => CText::BtnQuit,
        }
    }

    /// Cell of the left edge of the button (column, row) on the main screen.
    fn pos(self) -> (usize, usize) {
        match self {
            Button::Config => (7, 17),
            Button::Tools => (25, 17),
            Button::Start => (43, 17),
            Button::Load => (62, 17),
            Button::Panelz => (7, 19),
            Button::Offset => (25, 19),
            Button::Quit => (43, 19),
        }
    }
}

/// What the Select File dialog shows.
struct FileDialog {
    name: String,
    sel: usize,
    /// The folder's entries (name, is a folder), read when the dialog opens.
    entries: Vec<(String, bool)>,
    /// The text of the name field was not typed yet (it is shown selected).
    fresh: bool,
}

/// The Parameter dialog works on a copy of the machine settings; Ok writes it back.
struct ParamDialog {
    unit: usize,
    sorting: usize,
    focus: usize,
    values: Vec<String>,
}

/// A heading of the Tool Data table: column, text, and the unit line (column, text) under it.
type Heading = (usize, CText, Option<(usize, &'static str)>);

/// The Panelization dialog: how many copies of the board fit the working area.
struct PanelDialog {
    board: (f64, f64),
    spacing: (f64, f64),
    number: (u32, u32),
}

impl PanelDialog {
    /// Copies that fit in `area` (inferred from the original's figures: area / (board - spacing)).
    fn calc(&mut self, area: (f64, f64)) {
        let n = |a: f64, b: f64, s: f64| {
            let d = b - s;
            if d > 0.0 {
                (a / d).floor().clamp(0.0, 9999.0) as u32
            } else {
                0
            }
        };
        self.number = (
            n(area.0, self.board.0, self.spacing.0),
            n(area.1, self.board.1, self.spacing.1),
        );
    }
}

enum Dialog {
    None,
    Panel(PanelDialog),
    Files(FileDialog),
    Tools { focus: usize },
    Parameter(ParamDialog),
    Quit { yes: bool },
    Message(String),
    Running,
}

pub struct ClassicApp<E: ClassicEngine = App> {
    pub engine: E,
    pub form: Form,
    focus: Option<Field>,
    /// The button that Enter presses: Load, and Start once a file is loaded.
    button: Button,
    edit: Option<String>,
    dialog: Dialog,
    area: (f64, f64),
    quit: bool,
    /// Name of the machine shown where the original shows the licence number.
    pub machine_name: String,
}

impl ClassicApp<App> {
    /// DRILLPRO: the classic screens over the drilling engine; `data_dir` holds the tool table and the configuration.
    pub fn new(lang: Lang, data_dir: &std::path::Path) -> Self {
        Self::with_engine(App::new(lang, data_dir))
    }
}

impl<E: ClassicEngine> ClassicApp<E> {
    /// The classic screens over any engine (ROUTEPRO brings its own).
    pub fn with_engine(engine: E) -> Self {
        let s = engine.settings().cloned();
        let form = Form {
            scale: s.as_ref().map_or((1.0, 1.0), |m| m.scale),
            offset: s.as_ref().map_or((2.5, 2.5), |m| m.offset_mm),
            clip: s.as_ref().map_or((555.0, 405.0), |m| m.clip_mm),
            from: 0,
            to: 0,
            board: s.as_ref().map_or(1.0, |m| m.board_mm),
            base: s.as_ref().map_or(12.0, |m| m.base_mm),
        };
        let area = (form.clip.0, form.clip.1);
        let machine_name = s
            .as_ref()
            .map(|m| m.serial_number.clone())
            .unwrap_or_default();
        ClassicApp {
            engine,
            form,
            focus: None,
            button: Button::Load,
            edit: None,
            dialog: Dialog::None,
            area,
            quit: false,
            machine_name,
        }
    }

    pub fn load_file(&mut self, path: &std::path::Path) {
        self.engine.load(path);
        self.sync_vectors();
        if self.engine.file_loaded() {
            self.button = Button::Start;
        }
    }

    fn lang(&self) -> Lang {
        self.engine.lang()
    }

    fn t(&self, k: CText) -> &'static str {
        ctr(self.engine.lang(), k)
    }

    fn holes(&self) -> usize {
        self.engine.vectors()
    }

    fn sync_vectors(&mut self) {
        let n = self.holes();
        self.form.from = usize::from(n > 0);
        self.form.to = n;
    }

    fn file_name(&self) -> String {
        self.engine
            .file_name()
            .unwrap_or_else(|| "--------.---".to_string())
    }

    fn enabled(&self, b: Button) -> bool {
        match b {
            Button::Start | Button::Panelz => self.engine.file_loaded(),
            _ => true,
        }
    }

    // ---------------------------------------------------------------- rendering

    /// The main screen.
    pub fn render_main(&self, s: &mut Screen) {
        desktop(s);
        window(s, 1, 1, 78, 21, "", false);
        // heading
        s.text(5, 3, self.t(CText::Project), INK, GREY);
        s.text(5, 4, self.t(CText::Licence), INK, GREY);
        box_single(s, 31, 3, 21, 3);
        s.text(33, 4, self.t(self.title()), INK, GREY);
        s.text(62, 3, self.t(CText::MachineNr), INK, GREY);
        let machine: String = self.machine_name.chars().take(12).collect();
        s.text(62, 4, &machine, INK, GREY);
        // form
        box_single(s, 5, 7, 53, 9);
        let focus_group = self.focus.map(Field::group);
        let caps = [
            (12, 8, CText::LblScale),
            (22, 8, CText::LblOffset),
            (33, 8, CText::LblClip),
            (43, 8, CText::LblVectors),
            (12, 13, CText::LblBoard),
            (22, 13, CText::LblBase),
        ];
        for (i, (x, y, k)) in caps.iter().enumerate() {
            caption(s, *x, *y, self.t(*k), focus_group == Some(i));
        }
        label(s, 22, 9, self.t(CText::Mm));
        label(s, 33, 9, self.t(CText::Mm));
        label(s, 8, 10, "X:");
        label(s, 8, 11, "Y:");
        label(s, 8, 14, "Z:");
        label(s, 43, 10, self.t(CText::LblFrm));
        label(s, 43, 11, self.t(CText::LblTo));
        let cells = [
            (Field::ScaleX, 12, 10, 6),
            (Field::ScaleY, 12, 11, 6),
            (Field::OffsetX, 22, 10, 7),
            (Field::OffsetY, 22, 11, 7),
            (Field::ClipX, 33, 10, 6),
            (Field::ClipY, 33, 11, 6),
            (Field::From, 47, 10, 5),
            (Field::To, 47, 11, 5),
            (Field::Board, 12, 14, 5),
            (Field::Base, 22, 14, 5),
        ];
        for (f, x, y, w) in cells {
            let editing = self.focus == Some(f);
            let text = match (&self.edit, editing) {
                (Some(e), true) => e.clone(),
                _ => self.form.text(f),
            };
            field(s, x, y, w, &text, editing);
            if editing {
                s.put(x + 1, y, '\u{2595}', GREEN, BLACK);
            }
        }
        label(s, 30, 14, self.t(CText::Mm));
        // limits of the loaded file
        let (b, file) = self.bounds();
        let fmt = |v: f64| format!("{v:06.2}");
        label(s, 33, 13, &fmt(b.0));
        label(s, 40, 13, "<");
        label(s, 43, 13, "X");
        label(s, 46, 13, "<");
        label(s, 48, 13, &fmt(b.2));
        label(s, 33, 14, &fmt(b.1));
        label(s, 40, 14, "<");
        label(s, 43, 14, "Y");
        label(s, 46, 14, "<");
        label(s, 48, 14, &fmt(b.3));
        // counters
        label(s, 62, 8, self.t(CText::LblCount));
        label(s, 62, 10, self.t(CText::LblTotal));
        label(s, 69, 10, &format!("{:05}", self.holes()));
        label(s, 62, 11, self.t(CText::LblDone));
        label(s, 69, 11, &format!("{:05}", self.engine.done()));
        let file_label = self.t(CText::LblFile);
        label(s, 62, 13, file_label);
        label(s, (62 + str_width(file_label) + 1).max(67), 13, ":");
        label(s, 62, 14, &file);
        // buttons
        for b in Button::ALL {
            let (x, y) = b.pos();
            button(
                s,
                x,
                y,
                12,
                self.t(b.text()),
                b == self.button && self.focus.is_none(),
                self.enabled(b),
            );
        }
        if let Some(e) = self.engine.error() {
            let shown: String = e.chars().take(70).collect();
            s.text(5, 21, &shown, RED, GREY);
        }
    }

    /// Bounds of the holes in mm and the file name to show.
    fn bounds(&self) -> ((f64, f64, f64, f64), String) {
        (
            self.engine.bounds().unwrap_or((0.0, 0.0, 0.0, 0.0)),
            self.file_name(),
        )
    }

    fn title(&self) -> CText {
        match self.engine.kind() {
            Kind::Drill => CText::Title,
            Kind::Route => CText::TitleRoute,
        }
    }

    fn render_quit(&self, s: &mut Screen, yes: bool) {
        window(s, 19, 6, 41, 11, "", false);
        // the STOP sign: a double frame with its own shadow
        s.frame(21, 8, 12, 3, LIGHT_GREEN, GREY);
        s.text(23, 9, self.t(CText::QStop), INK, GREY);
        for xx in 23..35 {
            s.put(xx, 11, ' ', BLACK, BLACK);
        }
        for yy in [9, 10] {
            s.put(33, yy, ' ', BLACK, BLACK);
            s.put(34, yy, ' ', BLACK, BLACK);
        }
        let ask = self.t(CText::QAsk);
        let (l1, l2) = split_two(ask, 18);
        s.text(38, 9, &l1, INK, GREY);
        s.text(38, 10, &l2, INK, GREY);
        button(s, 29, 14, 10, self.t(CText::BYes), yes, true);
        button(s, 42, 14, 10, self.t(CText::BNo), !yes, true);
    }

    fn render_files(&self, s: &mut Screen, d: &FileDialog) {
        window(s, 15, 2, 49, 19, self.t(CText::FTitle), true);
        caption(s, 18, 4, self.t(CText::FName), true);
        // name field: 22 cells wide + the drop-down button
        s.fill(18, 5, 28, 1, ' ', BLUE, BLUE);
        if d.fresh {
            s.text(19, 5, &d.name, WHITE, GREEN);
        } else {
            s.text(19, 5, &format!("{}_", d.name), WHITE, BLUE);
        }
        s.put(46, 5, '\u{258c}', GREY, GREEN);
        s.put(47, 5, '\u{2193}', BLACK, GREEN);
        s.put(48, 5, '\u{258c}', GREEN, GREY);
        button(s, 51, 5, 9, self.t(CText::BOkUpper), false, true);
        s.put(54, 5, 'O', LIGHT_CYAN, GREEN);
        caption(s, 18, 7, self.t(CText::FFiles), false);
        // two-column list on cyan
        s.fill(18, 8, 15, 8, ' ', CYAN, CYAN);
        s.fill(33, 8, 1, 8, '\u{2502}', BLUE, CYAN);
        s.fill(34, 8, 15, 8, ' ', CYAN, CYAN);
        for (i, (name, dir)) in d.entries.iter().take(16).enumerate() {
            let (col, row) = (i / 8, i % 8);
            let label = if *dir {
                format!("{name}\\")
            } else {
                name.clone()
            };
            let (fg, x) = (if i == d.sel { YELLOW } else { BLACK }, 19 + col * 16);
            s.text(x, 8 + row, &label, fg, CYAN);
        }
        // scroll bar
        s.fill(18, 16, 31, 1, '\u{2592}', CYAN, BLUE);
        s.put(18, 16, '\u{25c4}', CYAN, BLUE);
        s.put(19, 16, '\u{25a0}', CYAN, BLUE);
        s.put(48, 16, '\u{25ba}', CYAN, BLUE);
        button(s, 51, 8, 9, self.t(CText::BCancel), false, true);
        // path and file line
        s.fill(16, 18, 47, 2, ' ', BLUE, BLUE);
        let path = format!("{}\\{}", self.engine.dir().display(), self.engine.pattern());
        s.text(
            17,
            18,
            &path.chars().take(44).collect::<String>(),
            CYAN,
            BLUE,
        );
        if let Some((name, _)) = d.entries.get(d.sel) {
            s.text(17, 19, name, CYAN, BLUE);
        }
    }

    fn render_tools(&self, s: &mut Screen, _focus: usize) {
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
        for (i, row) in self.engine.tool_rows().iter().enumerate() {
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
                field(s, *fx, y, *fw, text, false);
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
        button(s, bx[0], 20, 9, self.t(CText::BLoad), false, true);
        button(s, bx[1], 20, 9, self.t(CText::BSave), false, true);
        button(s, bx[2], 20, 9, self.t(CText::BOk), true, true);
        button(s, bx[3], 20, 9, self.t(CText::BAbort), false, true);
    }

    fn render_parameter(&self, s: &mut Screen, d: &ParamDialog) {
        window(s, 5, 1, 70, 21, "", false);
        let title = self.t(CText::PTitle);
        label(
            s,
            5 + (70usize.saturating_sub(str_width(title))) / 2 - 1,
            2,
            title,
        );
        caption(s, 10, 4, self.t(CText::PFile), false);
        caption(s, 35, 4, self.t(CText::PCoordinates), false);
        let route = self.engine.kind() == Kind::Route;
        caption(
            s,
            54,
            4,
            self.t(if route {
                CText::PChord
            } else {
                CText::PSorting
            }),
            false,
        );
        label(s, 11, 5, self.t(CText::PDirectory));
        field(s, 11, 6, 17, &d.values[0], d.focus == 0);
        label(s, 11, 7, self.t(CText::PPattern));
        field(s, 11, 8, 12, &d.values[1], d.focus == 1);
        let units: Vec<String> = [
            CText::PUnit110,
            CText::PUnit1,
            CText::PUnit0025,
            CText::PUnit001,
        ]
        .iter()
        .map(|k| self.t(*k).to_string())
        .collect();
        radio_group(s, 36, 5, 14, &units, d.unit);
        if route {
            field(s, 55, 5, 3, &d.values[16], d.focus == 16);
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
            radio_group(s, 55, 5, 14, &sorts, d.sorting);
        }
        caption(s, 10, 10, self.t(CText::PSpeed), false);
        caption(s, 35, 10, self.t(CText::PRamp), false);
        caption(s, 54, 10, self.t(CText::PStroke), false);
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
            field(s, 21, y, 4, &d.values[i], d.focus == i);
        }
        label(s, 36, 11, "XY");
        field(s, 40, 11, 2, &d.values[8], d.focus == 8);
        label(s, 36, 12, "Z");
        field(s, 40, 12, 2, &d.values[9], d.focus == 9);
        label(s, 55, 11, self.t(CText::PLimit));
        field(s, 62, 11, 5, &d.values[13], d.focus == 13);
        label(s, 55, 12, self.t(CText::PFree));
        field(s, 62, 12, 5, &d.values[14], d.focus == 14);
        caption(s, 35, 14, self.t(CText::PCorrect), false);
        caption(s, 54, 14, self.t(CText::PSpindle), false);
        for (y, name, i) in [(15, "X", 10usize), (16, "Y", 11), (17, "Z", 12)] {
            label(s, 36, y, name);
            field(s, 40, y, 6, &d.values[i], d.focus == i);
        }
        label(s, 55, 15, self.t(CText::PDelay));
        field(s, 62, 15, 3, &d.values[15], d.focus == 15);
        label(s, 68, 15, self.t(CText::PSecond));
        button(s, 11, 19, 9, self.t(CText::BLoad), false, true);
        button(s, 26, 19, 9, self.t(CText::BSave), false, true);
        button(s, 43, 19, 9, self.t(CText::BOk), false, true);
        button(s, 59, 19, 9, self.t(CText::BAbort), true, true);
    }

    fn render_panel(&self, s: &mut Screen, d: &PanelDialog) {
        window(s, 23, 2, 34, 18, "", false);
        label(s, 29, 4, self.t(CText::PzTitle));
        let rows = [
            (6, CText::PzBoard, "X", format!("{:06.2}", d.board.0), 6),
            (7, CText::PzBoard, "Y", format!("{:06.2}", d.board.1), 6),
            (9, CText::PzSpacing, "X", format!("{:05.2}", d.spacing.0), 5),
            (
                10,
                CText::PzSpacing,
                "Y",
                format!("{:05.2}", d.spacing.1),
                5,
            ),
            (14, CText::PzNumber, "X", format!("{:04}", d.number.0), 4),
            (15, CText::PzNumber, "Y", format!("{:04}", d.number.1), 4),
        ];
        for (y, k, axis, text, w) in rows {
            label(s, 29, y, self.t(k));
            label(s, 37, y, axis);
            field(s, 41, y, w, &text, false);
        }
        label(s, 30, 8, "(mm)");
        button(s, 29, 12, 9, self.t(CText::PzCalc), false, true);
        button(s, 29, 17, 9, self.t(CText::BOk), true, true);
        button(s, 41, 17, 9, self.t(CText::BAbort), false, true);
    }

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
        label(s, 52, 6, "000");
        label(s, 56, 6, "%");
        label(s, 20, 8, self.t(CText::JTool));
        label(s, 39, 8, self.t(CText::JPosition));
        label(s, 22, 9, self.t(CText::JNo));
        label(s, 29, 9, ":");
        label(s, 31, 9, "--");
        label(s, 22, 10, self.t(CText::JStroke));
        label(s, 29, 10, ":");
        label(s, 31, 10, "00.00");
        for (y, axis) in [(9, "X"), (10, "Y")] {
            label(s, 41, y, axis);
            label(s, 45, y, "(mm):");
            label(s, 51, y, "000.00");
        }
        label(s, 20, 12, self.t(CText::JSpeed));
        label(s, 39, 12, self.t(CText::JCount));
        label(s, 22, 13, "Z");
        label(s, 29, 13, ":");
        label(s, 31, 13, "0000");
        label(s, 22, 14, "1/min");
        label(s, 29, 14, ":");
        label(s, 31, 14, "00000");
        label(s, 41, 13, self.t(CText::LblDone));
        label(s, 49, 13, ":");
        label(s, 51, 13, &format!("{:05}", self.engine.done()));
        label(s, 41, 14, self.t(CText::JUse));
        label(s, 45, 14, "(mm):");
        label(s, 51, 14, "00000");
        // the red strip: one cell of margin on the left, two on the right
        let hint = self.t(CText::PressEsc);
        s.fill(20, 16, 20, 1, ' ', RED, RED);
        s.text(21, 16, hint, YELLOW, RED);
        label(s, 50, 16, &self.file_name());
    }

    // ---------------------------------------------------------------- input

    fn commit_edit(&mut self) {
        let (Some(f), Some(text)) = (self.focus, self.edit.take()) else {
            return;
        };
        let (lo, hi) = f.range(self.area);
        match text.trim().replace(',', ".").parse::<f64>() {
            Ok(v) if v >= lo && v <= hi => {
                self.form.set(f, v);
                self.apply_form();
                self.focus = None;
            }
            _ => {
                self.focus = None;
                let msg = format!(
                    "{} {}{:.2} {}{:.2}",
                    self.t(CText::InputError),
                    self.t(CText::Max),
                    hi,
                    self.t(CText::Min),
                    lo
                );
                self.dialog = Dialog::Message(msg);
            }
        }
    }

    /// Copies the form into the settings the engine plans the job with.
    fn apply_form(&mut self) {
        if let Some(m) = self.engine.settings_mut() {
            m.scale = self.form.scale;
            m.offset_mm = self.form.offset;
            m.clip_mm = self.form.clip;
            m.board_mm = self.form.board;
            m.base_mm = self.form.base;
        }
        self.engine.set_board(self.form.board);
    }

    fn focus_field(&mut self, f: Field) {
        self.edit = None;
        self.focus = Some(f);
    }

    fn move_focus(&mut self, delta: isize) {
        let i = self
            .focus
            .and_then(|f| Field::ALL.iter().position(|&x| x == f))
            .map_or(0isize, |i| i as isize + delta);
        let n = Field::ALL.len() as isize;
        let next = Field::ALL[i.rem_euclid(n) as usize];
        self.commit_edit_quiet();
        self.focus_field(next);
    }

    fn commit_edit_quiet(&mut self) {
        if self.edit.is_some() {
            self.commit_edit();
        }
    }

    fn press(&mut self, b: Button) {
        if !self.enabled(b) {
            return;
        }
        self.commit_edit_quiet();
        self.focus = None;
        match b {
            Button::Load => self.open_files(),
            Button::Tools => self.dialog = Dialog::Tools { focus: 0 },
            Button::Config => self.dialog = Dialog::Parameter(self.param_from_settings()),
            Button::Quit => self.dialog = Dialog::Quit { yes: true },
            Button::Start => {
                self.engine.start();
                self.dialog = Dialog::Running;
            }
            Button::Panelz => {
                let (x0, y0, x1, y1) = self.engine.bounds().unwrap_or((0.0, 0.0, 0.0, 0.0));
                let mut d = PanelDialog {
                    board: (x1 - x0, y1 - y0),
                    spacing: (1.0, 1.0),
                    number: (0, 0),
                };
                d.calc(self.area);
                self.dialog = Dialog::Panel(d);
            }
            Button::Offset => {
                self.dialog = Dialog::Message(self.t(CText::SimNote).to_string());
            }
        }
    }

    fn open_files(&mut self) {
        let entries = self.engine.entries();
        self.dialog = Dialog::Files(FileDialog {
            name: self.engine.pattern().into(),
            sel: 0,
            entries,
            fresh: true,
        });
    }

    fn param_from_settings(&self) -> ParamDialog {
        let m = self.engine.settings().cloned();
        let c = self.engine.config().cloned();
        let v = |x: Option<String>| x.unwrap_or_default();
        let values = vec![
            v(c.as_ref().map(|c| c.directory.clone())),
            v(c.as_ref().map(|c| c.default_extension.clone())),
            v(m.as_ref().map(|m| m.speed_xy.0.to_string())),
            v(m.as_ref().map(|m| m.speed_xy.1.to_string())),
            v(m.as_ref().map(|m| m.speed_xy.2.to_string())),
            v(m.as_ref().map(|m| m.speed_z.0.to_string())),
            v(m.as_ref().map(|m| m.speed_z.1.to_string())),
            v(m.as_ref().map(|m| m.speed_z.2.to_string())),
            v(m.as_ref().map(|m| m.accel.0.to_string())),
            v(m.as_ref().map(|m| m.accel.1.to_string())),
            v(m.as_ref().map(|m| format!("{:.4}", m.correction.0))),
            v(m.as_ref().map(|m| format!("{:.4}", m.correction.1))),
            v(m.as_ref().map(|m| format!("{:.4}", m.correction.2))),
            v(m.as_ref().map(|m| format!("{:.2}", m.z_max_depth_mm))),
            v(m.as_ref()
                .map(|m| format!("{:.2}", m.distance_tool_board_mm))),
            v(m.as_ref().map(|m| format!("{}", m.spindle_start_time))),
            v(m.as_ref().map(|m| format!("{:+03}", m.chord_angle))),
        ];
        let unit = m.as_ref().map_or(1, |m| match m.unit {
            crate::config::CoordUnit::TenthMil => 0,
            crate::config::CoordUnit::Mil => 1,
            crate::config::CoordUnit::PlotterStep => 2,
            crate::config::CoordUnit::HundredthMm => 3,
        });
        let sorting = m.as_ref().map_or(3, |m| match m.sorting {
            crate::config::SortMode::None => 0,
            crate::config::SortMode::XFirst => 1,
            crate::config::SortMode::YFirst => 2,
            crate::config::SortMode::Special => 3,
        });
        ParamDialog {
            unit,
            sorting,
            focus: usize::MAX,
            values,
        }
    }

    fn button_for_char(&self, c: char) -> Option<Button> {
        let c = c.to_ascii_lowercase();
        Button::ALL
            .into_iter()
            .find(|b| hot_letter(self.t(b.text())) == Some(c))
    }

    fn field_for_char(&self, c: char) -> Option<Field> {
        let c = c.to_ascii_lowercase();
        let caps = [
            (CText::LblScale, Field::ScaleX),
            (CText::LblOffset, Field::OffsetX),
            (CText::LblClip, Field::ClipX),
            (CText::LblVectors, Field::From),
            (CText::LblBoard, Field::Board),
            (CText::LblBase, Field::Base),
        ];
        caps.into_iter()
            .find(|(k, _)| hot_letter(self.t(*k)) == Some(c))
            .map(|(_, f)| f)
    }

    fn key_main(&mut self, input: Input) {
        // while a field is being edited the typed characters go to it
        if let Some(f) = self.focus {
            match input {
                Input::Char(c) if c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+') => {
                    let fresh = self.edit.is_none();
                    let e = self.edit.get_or_insert_with(String::new);
                    if fresh {
                        e.clear();
                    }
                    if e.len() < 9 {
                        e.push(c);
                    }
                    return;
                }
                Input::Enter => {
                    if self.edit.is_some() {
                        self.commit_edit();
                    } else {
                        self.move_focus(1);
                    }
                    return;
                }
                Input::Esc => {
                    self.edit = None;
                    self.focus = None;
                    return;
                }
                Input::Down => return self.move_focus(1),
                Input::Up => return self.move_focus(-1),
                _ => {}
            }
            let _ = f;
        }
        match input {
            Input::Char(c) | Input::Alt(c) => {
                if let Some(b) = self.button_for_char(c) {
                    self.press(b);
                } else if let Some(f) = self.field_for_char(c) {
                    self.commit_edit_quiet();
                    self.focus_field(f);
                }
            }
            Input::Enter => self.press(self.button),
            Input::Esc => self.dialog = Dialog::Quit { yes: true },
            _ => {}
        }
    }

    fn key_files(&mut self, input: Input) {
        let Dialog::Files(d) = &mut self.dialog else {
            return;
        };
        match input {
            Input::Esc => self.dialog = Dialog::None,
            Input::Down => d.sel = (d.sel + 1).min(d.entries.len().saturating_sub(1)),
            Input::Up => d.sel = d.sel.saturating_sub(1),
            Input::Char(c) => {
                if d.fresh {
                    d.name.clear();
                    d.fresh = false;
                }
                d.name.push(c);
            }
            Input::Enter => {
                let typed = d.name.clone();
                let picked = d.entries.get(d.sel).cloned();
                let dir = self.engine.dir();
                let mut path: Option<PathBuf> = None;
                if !typed.contains('*') && !typed.is_empty() {
                    let p = dir.join(&typed);
                    if p.exists() {
                        path = Some(p);
                    }
                }
                if path.is_none() {
                    if let Some((name, is_dir)) = picked {
                        if is_dir {
                            let next = if name == ".." {
                                dir.parent().map(|p| p.to_path_buf()).unwrap_or(dir)
                            } else {
                                dir.join(name)
                            };
                            self.engine.set_dir(next);
                            self.open_files();
                            return;
                        }
                        path = Some(dir.join(name));
                    }
                }
                if let Some(p) = path {
                    self.load_file(&p);
                    // the original shows its tool data right after loading
                    self.dialog = Dialog::Tools { focus: 0 };
                } else {
                    self.dialog = Dialog::None;
                }
            }
            _ => {}
        }
    }

    fn key_parameter(&mut self, input: Input) {
        let Dialog::Parameter(d) = &mut self.dialog else {
            return;
        };
        match input {
            Input::Esc => self.dialog = Dialog::None,
            Input::Enter => {
                self.dialog = Dialog::None;
            }
            Input::Char(c) | Input::Alt(c) => {
                let c = c.to_ascii_lowercase();
                if c == 'o' || c == 'a' {
                    self.dialog = Dialog::None;
                } else if c == 'c' {
                    d.unit = (d.unit + 1) % 4;
                } else if c == 's' {
                    d.sorting = (d.sorting + 1) % 4;
                }
            }
            _ => {}
        }
    }
}

/// The hot letter of a label, lower case.
pub fn hot_letter(label: &str) -> Option<char> {
    let mut it = label.chars();
    while let Some(c) = it.next() {
        if c == '~' {
            return it.next().map(|c| c.to_ascii_lowercase());
        }
    }
    None
}

/// Single-line box in the window ink (the frames inside the main window).
fn box_single(s: &mut Screen, x: usize, y: usize, w: usize, h: usize) {
    s.put(x, y, '\u{250c}', INK, GREY);
    s.put(x + w - 1, y, '\u{2510}', INK, GREY);
    s.put(x, y + h - 1, '\u{2514}', INK, GREY);
    s.put(x + w - 1, y + h - 1, '\u{2518}', INK, GREY);
    for xx in x + 1..x + w - 1 {
        s.put(xx, y, '\u{2500}', INK, GREY);
        s.put(xx, y + h - 1, '\u{2500}', INK, GREY);
    }
    for yy in y + 1..y + h - 1 {
        s.put(x, yy, '\u{2502}', INK, GREY);
        s.put(x + w - 1, yy, '\u{2502}', INK, GREY);
    }
}

/// Splits `text` into two lines of at most `width` cells at a space.
fn split_two(text: &str, width: usize) -> (String, String) {
    let mut a = String::new();
    let mut words = text.split(' ').peekable();
    while let Some(w) = words.peek() {
        let cand = if a.is_empty() {
            (*w).to_string()
        } else {
            format!("{a} {w}")
        };
        if str_width(&cand) > width && !a.is_empty() {
            break;
        }
        a = cand;
        words.next();
    }
    (a, words.collect::<Vec<_>>().join(" "))
}

impl<E: ClassicEngine> TextApp for ClassicApp<E> {
    fn handle(&mut self, input: Input) {
        match &self.dialog {
            Dialog::None => self.key_main(input),
            Dialog::Files(_) => self.key_files(input),
            Dialog::Parameter(_) => self.key_parameter(input),
            Dialog::Tools { .. } => {
                if matches!(
                    input,
                    Input::Enter | Input::Esc | Input::Char(_) | Input::Alt(_)
                ) {
                    self.dialog = Dialog::None;
                }
            }
            Dialog::Quit { yes } => {
                let y = *yes;
                match input {
                    Input::Enter if y => self.quit = true,
                    Input::Enter | Input::Esc => self.dialog = Dialog::None,
                    Input::Char(c) | Input::Alt(c) => match c.to_ascii_lowercase() {
                        'y' | 's' | 'j' | 'o' => self.quit = true,
                        'n' => self.dialog = Dialog::None,
                        _ => {}
                    },
                    Input::Up | Input::Down => self.dialog = Dialog::Quit { yes: !y },
                    _ => {}
                }
            }
            Dialog::Panel(_) => match input {
                Input::Esc | Input::Enter => self.dialog = Dialog::None,
                Input::Char(c) | Input::Alt(c) => match c.to_ascii_lowercase() {
                    'o' | 'a' => self.dialog = Dialog::None,
                    'c' => {
                        let area = self.area;
                        if let Dialog::Panel(d) = &mut self.dialog {
                            d.calc(area);
                        }
                    }
                    _ => {}
                },
                _ => {}
            },
            Dialog::Message(_) => self.dialog = Dialog::None,
            Dialog::Running => {
                if input == Input::Esc {
                    self.engine.stop();
                    self.dialog = Dialog::None;
                }
            }
        }
    }

    fn click(&mut self, col: usize, row: usize) {
        if matches!(self.dialog, Dialog::None) {
            for b in Button::ALL {
                let (x, y) = b.pos();
                if row == y && col >= x && col < x + 12 {
                    self.press(b);
                    return;
                }
            }
        }
    }

    fn tick(&mut self) -> bool {
        if matches!(self.dialog, Dialog::Running) {
            let more = self.engine.tick(1);
            if !more {
                self.dialog = Dialog::None;
            }
            return more;
        }
        false
    }

    fn render(&self, screen: &mut Screen) {
        // the dialogs replace the main screen: only the desktop stays behind them
        if matches!(self.dialog, Dialog::None) {
            self.render_main(screen);
        } else {
            desktop(screen);
        }
        match &self.dialog {
            Dialog::None => {}
            Dialog::Files(d) => self.render_files(screen, d),
            Dialog::Panel(d) => self.render_panel(screen, d),
            Dialog::Tools { focus } => self.render_tools(screen, *focus),
            Dialog::Parameter(d) => self.render_parameter(screen, d),
            Dialog::Quit { yes } => self.render_quit(screen, *yes),
            Dialog::Message(m) => {
                let w = str_width(m) + 6;
                let x = (COLS - w.min(COLS)) / 2;
                window(screen, x, 10, w.min(COLS), 5, "", false);
                screen.text_centered(12, x, w.min(COLS), m, INK, GREY);
            }
            Dialog::Running => self.render_running(screen),
        }
    }

    fn lang(&self) -> Lang {
        ClassicApp::<E>::lang(self)
    }

    fn wants_quit(&self) -> bool {
        self.quit
    }
}
