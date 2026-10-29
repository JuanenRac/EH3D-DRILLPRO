//! The classic front end of DRILLPRO and ROUTEPRO: the screens of the MS-DOS programs drawn on the 80x25 text screen.
//!
//! It is a layer over an engine ([`engine::ClassicEngine`]): the job planning, the machine link and the file formats
//! stay in the engine; this module owns what is seen and typed. Everything can be done with the keyboard (letters,
//! `Tab`, arrows, `Enter`, `Esc`) and with the mouse (every field, button, radio button and list entry).

mod dialogs;
pub mod edit;
pub mod engine;
mod teach;
pub mod texts;
pub mod ui;

use std::cell::{Cell, RefCell};
use std::time::Instant;

use crate::app::{App, Input};
use crate::i18n::{str_width, Lang};
use crate::screen::*;
use crate::textapp::TextApp;
use edit::{Kind as EditKind, LineEdit};
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

    /// Which label the field belongs to: 0 Scale, 1 Offset, 2 Clip, 3 Vectors, 4 Board, 5 Base.
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

    /// Cell of the field on the main screen: (column, row, text width).
    fn cell(self) -> (usize, usize, usize) {
        match self {
            Field::ScaleX => (12, 10, 6),
            Field::ScaleY => (12, 11, 6),
            Field::OffsetX => (22, 10, 7),
            Field::OffsetY => (22, 11, 7),
            Field::ClipX => (33, 10, 6),
            Field::ClipY => (33, 11, 6),
            Field::From => (47, 10, 5),
            Field::To => (47, 11, 5),
            Field::Board => (12, 14, 5),
            Field::Base => (22, 14, 5),
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

/// The main-screen buttons, in the order of the original (About is ours, in the free place under Load).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Config,
    Tools,
    Start,
    Load,
    Panelz,
    Offset,
    Quit,
    About,
}

impl Button {
    const ALL: [Button; 8] = [
        Button::Config,
        Button::Tools,
        Button::Start,
        Button::Load,
        Button::Panelz,
        Button::Offset,
        Button::Quit,
        Button::About,
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
            Button::About => CText::BtnAbout,
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
            Button::About => (62, 19),
        }
    }
}

/// Buttons of the dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dlg {
    Load,
    Save,
    Ok,
    Abort,
    Calc,
    Yes,
    No,
    Cancel,
}

/// What a clickable region of the screen does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Act {
    Field(Field),
    Btn(Button),
    Dlg(Dlg),
    /// A control of the Parameter dialog, by its place in the focus ring.
    Ctl(usize),
    Unit(usize),
    Sort(usize),
    /// A cell of the Tool Data table (row, column).
    Cell(usize, usize),
    /// The name field and the entries of the Select File dialog.
    Name,
    /// The arrow at the end of the name field: the list of files takes the keys.
    Drop,
    Item(usize),
    /// A control of the Panelization dialog, by its place in the focus ring.
    Pz(usize),
    /// The arrows at the ends of the language field: the previous (-1) or the next (1) language.
    LangStep(isize),
    /// A control of the Offset Teach window.
    Teach(teach::TeachBtn),
}

struct Hit {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    act: Act,
}

pub(crate) use dialogs::Dialog;

pub struct ClassicApp<E: ClassicEngine = App> {
    pub engine: E,
    pub form: Form,
    focus: Option<Field>,
    /// The button that Enter presses: Load, and Start once a file is loaded.
    button: Button,
    edit: Option<LineEdit>,
    dialog: Dialog,
    area: (f64, f64),
    quit: bool,
    /// The copies the Panelization dialog set for the next job (columns, rows, pitch in mm).
    panel: Option<engine::Panel>,
    /// The settings or the form changed since the files were read or written.
    dirty: bool,
    /// Esc was pressed during the job: it waits for Cont. or Abort.
    interrupted: bool,
    /// Name of the machine shown where the original shows the licence number.
    pub machine_name: String,
    hits: RefCell<Vec<Hit>>,
    /// What the Select File window is open for: a job, a tool file or a configuration file.
    file_purpose: dialogs::Purpose,
    last_click: Cell<Option<(usize, usize, Instant)>>,
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
        let form = Self::form_of(&engine);
        let area = (form.clip.0, form.clip.1);
        let machine_name = engine
            .settings()
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
            panel: None,
            dirty: false,
            interrupted: false,
            machine_name,
            hits: RefCell::new(Vec::new()),
            file_purpose: dialogs::Purpose::Job,
            last_click: Cell::new(None),
        }
    }

    fn form_of(engine: &E) -> Form {
        let s = engine.settings();
        Form {
            scale: s.map_or((1.0, 1.0), |m| m.scale),
            offset: s.map_or((2.5, 2.5), |m| m.offset_mm),
            clip: s.map_or((555.0, 405.0), |m| m.clip_mm),
            from: 0,
            to: 0,
            board: s.map_or(1.0, |m| m.board_mm),
            base: s.map_or(12.0, |m| m.base_mm),
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

    fn hit(&self, x: usize, y: usize, w: usize, h: usize, act: Act) {
        self.hits.borrow_mut().push(Hit { x, y, w, h, act });
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
        for f in Field::ALL {
            let (x, y, w) = f.cell();
            let editing = self.focus == Some(f);
            match (&self.edit, editing) {
                (Some(e), true) => edit_field(s, x, y, w, e),
                (None, true) => {
                    field(s, x, y, w, &self.form.text(f), true);
                    s.put(x + 1, y, '\u{2595}', GREEN, BLACK);
                }
                _ => field(s, x, y, w, &self.form.text(f), false),
            }
            self.hit(x, y, w + 2, 1, Act::Field(f));
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
            self.hit(x, y, 13, 2, Act::Btn(b));
        }
        if let Some(e) = self.engine.error() {
            let shown: String = e.chars().take(70).collect();
            s.text(5, 21, &shown, RED, GREY);
        }
    }

    /// Bounds of the vectors in mm and the file name to show.
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

    // ---------------------------------------------------------------- the main form

    fn commit_edit(&mut self) {
        let (Some(f), Some(e)) = (self.focus, self.edit.take()) else {
            return;
        };
        let (lo, hi) = f.range(self.area);
        match e.text.trim().replace(',', ".").parse::<f64>() {
            Ok(v) if v >= lo && v <= hi => {
                self.form.set(f, v);
                self.apply_form();
                self.dirty = true;
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

    /// Puts the focus on a field with its text selected, so that typing replaces it.
    fn focus_field(&mut self, f: Field) {
        self.focus = Some(f);
        self.edit = Some(LineEdit::new(
            &self.form.text(f),
            EditKind::Number,
            f.cell().2,
        ));
    }

    fn move_focus(&mut self, delta: isize) {
        let i = self
            .focus
            .and_then(|f| Field::ALL.iter().position(|&x| x == f))
            .map_or(0isize, |i| i as isize + delta);
        let n = Field::ALL.len() as isize;
        let next = Field::ALL[i.rem_euclid(n) as usize];
        self.commit_edit();
        if matches!(self.dialog, Dialog::None) {
            self.focus_field(next);
        }
    }

    fn press(&mut self, b: Button) {
        if !self.enabled(b) {
            // a button that cannot work now says why instead of staying silent
            if matches!(self.dialog, Dialog::None) {
                self.dialog = Dialog::Message(self.t(CText::NoFile).to_string());
            }
            return;
        }
        self.commit_edit();
        if !matches!(self.dialog, Dialog::None) {
            return;
        }
        self.focus = None;
        match b {
            Button::Load => {
                self.file_purpose = dialogs::Purpose::Job;
                self.dialog = self.open_files();
            }
            Button::Tools => self.dialog = self.open_tools(),
            Button::Config => self.dialog = self.open_parameter(),
            Button::Quit => self.dialog = Dialog::Quit { yes: true },
            Button::Start => {
                self.engine.set_job_options(self.job_options());
                self.engine.start();
                self.dialog = Dialog::Running;
            }
            Button::Panelz => self.dialog = self.open_panel(),
            Button::Offset => {
                self.dialog = self.open_offset();
            }
            Button::About => self.dialog = Dialog::About,
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
        if self.focus.is_some() {
            if let Some(e) = self.edit.as_mut() {
                if e.key(input) {
                    return;
                }
            }
            match input {
                // Enter accepts the field and goes on to the next one, as in the original; the last one just accepts
                Input::Enter => {
                    if self.focus == Field::ALL.last().copied() {
                        return self.commit_edit();
                    }
                    return self.move_focus(1);
                }
                Input::Esc => {
                    self.edit = None;
                    self.focus = None;
                    return;
                }
                Input::Tab | Input::Down => return self.move_focus(1),
                Input::BackTab | Input::Up => return self.move_focus(-1),
                Input::Alt(c) => {
                    if let Some(b) = self.button_for_char(c) {
                        return self.press(b);
                    }
                    if let Some(f) = self.field_for_char(c) {
                        self.commit_edit();
                        if matches!(self.dialog, Dialog::None) {
                            self.focus_field(f);
                        }
                    }
                    return;
                }
                _ => return,
            }
        }
        match input {
            Input::Char(c) | Input::Alt(c) => {
                if let Some(b) = self.button_for_char(c) {
                    self.press(b);
                } else if let Some(f) = self.field_for_char(c) {
                    self.focus_field(f);
                }
            }
            Input::Tab | Input::Down => self.focus_field(Field::ALL[0]),
            Input::BackTab | Input::Up => self.focus_field(Field::ALL[Field::ALL.len() - 1]),
            Input::Left => self.step_button(-1),
            Input::Right => self.step_button(1),
            Input::Enter => self.press(self.button),
            Input::Esc => self.dialog = Dialog::Quit { yes: true },
            _ => {}
        }
    }

    /// Moves the highlighted button (the one Enter presses) along the buttons.
    fn step_button(&mut self, delta: isize) {
        let all = Button::ALL;
        let at = all.iter().position(|&b| b == self.button).unwrap_or(0) as isize;
        let n = all.len() as isize;
        for k in 1..=n {
            let b = all[(at + delta * k).rem_euclid(n) as usize];
            if self.enabled(b) {
                self.button = b;
                return;
            }
        }
    }

    fn click_main(&mut self, act: Act) {
        match act {
            Act::Field(f) => {
                if self.focus != Some(f) {
                    self.commit_edit();
                    if matches!(self.dialog, Dialog::None) {
                        self.focus_field(f);
                    }
                }
            }
            Act::Btn(b) => {
                self.button = if self.enabled(b) { b } else { self.button };
                self.press(b);
            }
            _ => {}
        }
    }

    /// What the form and the Panelization dialog add to the next job.
    fn job_options(&self) -> engine::JobOptions {
        engine::JobOptions {
            from: self.form.from,
            to: self.form.to,
            clip_mm: Some(self.form.clip),
            panel: self.panel,
            ask_operator: true,
        }
    }

    /// The finished-with-the-dialog path: asks to save the changed settings, or quits.
    fn leave(&mut self) -> Dialog {
        if self.dirty {
            Dialog::SaveAsk { yes: true }
        } else {
            self.quit = true;
            Dialog::None
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
pub(crate) fn box_single(s: &mut Screen, x: usize, y: usize, w: usize, h: usize) {
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
pub(crate) fn split_two(text: &str, width: usize) -> (String, String) {
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
        self.handle_input(input);
    }

    fn click(&mut self, col: usize, row: usize) {
        // the clickable places of the screen as it is now
        self.render(&mut Screen::default());
        let act = self
            .hits
            .borrow()
            .iter()
            .rev()
            .find(|h| col >= h.x && col < h.x + h.w && row >= h.y && row < h.y + h.h)
            .map(|h| h.act);
        let double = match self.last_click.get() {
            Some((c, r, at)) => c == col && r == row && at.elapsed().as_millis() < 500,
            None => false,
        };
        self.last_click.set(Some((col, row, Instant::now())));
        match act {
            Some(act) => self.handle_click(act, double),
            // a click on the bare desktop leaves the field that is being edited
            None => {
                if matches!(self.dialog, Dialog::None) {
                    self.commit_edit();
                }
            }
        }
    }

    fn tick(&mut self) -> bool {
        if matches!(self.dialog, Dialog::Running) {
            if self.interrupted {
                // the job waits for the operator
                return true;
            }
            let more = self.engine.tick(1);
            if !more {
                self.interrupted = false;
                self.dialog = Dialog::None;
            }
            return more;
        }
        false
    }

    fn scroll(&mut self, lines: isize) {
        self.scroll_list(lines);
    }

    fn render(&self, screen: &mut Screen) {
        self.hits.borrow_mut().clear();
        // the dialogs replace the main screen: only the desktop stays behind them
        if matches!(self.dialog, Dialog::None) {
            self.render_main(screen);
        } else {
            desktop(screen);
            self.render_dialog(screen);
        }
    }

    fn lang(&self) -> Lang {
        ClassicApp::<E>::lang(self)
    }

    fn wants_quit(&self) -> bool {
        self.quit
    }
}
