//! The Offset button of the classic screens: "Delete old offset?", the Offset Teach window (the head is moved by hand in
//! steps of 1, 10, 100 or 1000 mil, a mil being 0.001 inch) and the Teach-In mode that records positions into a file.
//!
//! Ok copies the position reached into the Offset fields of the main screen. In the original the machine homes before
//! the window opens and again when Ok is pressed; here the engine moves the machine ([`ClassicEngine::jog`]).

use super::dialogs::{Dialog, Flow};
use super::*;
use crate::config::CoordUnit;

/// One mil (0.001 inch) in millimetres: the step of the machine.
const MIL_MM: f64 = 0.0254;
/// The steps the radio group offers, in mil.
const STEPS: [i32; 4] = [1, 10, 100, 1000];

/// The controls of the Offset Teach window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TeachBtn {
    /// Y positive.
    Left,
    /// X negative.
    Back,
    /// X positive.
    Forw,
    /// Y negative.
    Right,
    Up,
    Down,
    Teach,
    Abort,
    Ok,
    PlusPos,
    MinusPos,
    Mod,
    /// A step of the radio group (index into [`STEPS`]).
    Step(usize),
}

pub(crate) struct TeachDlg {
    /// Position of the head in mil from the home of X and Y; Z in mil below its top.
    x: i32,
    y: i32,
    z: i32,
    step: usize,
    /// Teach-In mode: the positions are relative to `origin` and `+ Pos` records them.
    teach: bool,
    origin: (i32, i32),
    points: Vec<(i32, i32)>,
    tool: u8,
    /// Depth of Z in mm.
    max_z: f64,
}

impl TeachDlg {
    fn mm(v: i32) -> f64 {
        v as f64 * MIL_MM
    }

    fn z_limit(&self) -> i32 {
        (self.max_z / MIL_MM).floor() as i32
    }

    fn jog(&mut self, dx: i32, dy: i32, dz: i32) {
        self.x += dx;
        self.y += dy;
        self.z = (self.z + dz).clamp(0, self.z_limit());
    }

    fn relative(&self) -> (i32, i32) {
        if self.teach {
            (self.x - self.origin.0, self.y - self.origin.1)
        } else {
            (self.x, self.y)
        }
    }
}

impl<E: ClassicEngine> ClassicApp<E> {
    // ------------------------------------------------------------------------------------------ Delete old offset?

    /// The Offset button: asks whether the old offset goes, then opens the window.
    pub(crate) fn open_offset(&mut self) -> Dialog {
        Dialog::OffsetAsk { yes: true }
    }

    pub(crate) fn offset_ask_key(&mut self, yes: &mut bool, input: Input) -> Flow {
        let hot_yes = hot_letter(self.t(CText::BYes));
        let hot_no = hot_letter(self.t(CText::BNo));
        match input {
            Input::Enter => self.offset_answer(*yes),
            Input::Esc => Flow::Close,
            Input::Left | Input::Right | Input::Up | Input::Down | Input::Tab | Input::BackTab => {
                *yes = !*yes;
                Flow::Stay
            }
            Input::Char(c) | Input::Alt(c) => {
                let c = c.to_ascii_lowercase();
                if Some(c) == hot_yes {
                    self.offset_answer(true)
                } else if Some(c) == hot_no {
                    self.offset_answer(false)
                } else {
                    Flow::Stay
                }
            }
            _ => Flow::Stay,
        }
    }

    pub(crate) fn offset_ask_click(&mut self, act: Act) -> Flow {
        match act {
            Act::Dlg(Dlg::Yes) => self.offset_answer(true),
            Act::Dlg(Dlg::No) => self.offset_answer(false),
            _ => Flow::Stay,
        }
    }

    /// Yes deletes the old offset (the window starts at home); No starts at the offset the form has.
    fn offset_answer(&mut self, delete: bool) -> Flow {
        let (x, y) = if delete {
            (0, 0)
        } else {
            (
                (self.form.offset.0 / MIL_MM).round() as i32,
                (self.form.offset.1 / MIL_MM).round() as i32,
            )
        };
        let max_z = self.engine.settings().map_or(37.3, |m| m.z_max_depth_mm);
        // on a real machine it homes now and follows the window from here on
        self.engine.teach_begin();
        Flow::to(Dialog::Teach(Box::new(TeachDlg {
            x,
            y,
            z: 0,
            step: STEPS.len() - 1,
            teach: false,
            origin: (0, 0),
            points: Vec::new(),
            tool: 1,
            max_z,
        })))
    }

    // ------------------------------------------------------------------------------------------ the window

    pub(crate) fn teach_key(&mut self, t: &mut TeachDlg, input: Input) -> Flow {
        let btn = match input {
            Input::Up => Some(TeachBtn::Left),
            Input::Down => Some(TeachBtn::Right),
            Input::Left => Some(TeachBtn::Back),
            Input::Right => Some(TeachBtn::Forw),
            Input::PageUp => Some(TeachBtn::Up),
            Input::PageDown => Some(TeachBtn::Down),
            Input::Enter => Some(TeachBtn::Ok),
            Input::Esc => Some(TeachBtn::Abort),
            Input::Char(c) | Input::Alt(c) => self.teach_char(t, c),
            _ => None,
        };
        match btn {
            Some(b) => self.teach_press(t, b),
            None => Flow::Stay,
        }
    }

    fn teach_char(&self, t: &TeachDlg, c: char) -> Option<TeachBtn> {
        let c = c.to_ascii_lowercase();
        if let Some(d) = c.to_digit(10) {
            return (1..=4).contains(&d).then(|| TeachBtn::Step(d as usize - 1));
        }
        let hot = |k: CText| hot_letter(self.t(k)) == Some(c);
        Some(match c {
            'e' => TeachBtn::Left,
            's' => TeachBtn::Back,
            'd' => TeachBtn::Forw,
            'x' => TeachBtn::Right,
            'r' => TeachBtn::Up,
            'c' => TeachBtn::Down,
            '+' | '=' => TeachBtn::PlusPos,
            '-' => TeachBtn::MinusPos,
            _ if hot(CText::BTeach) => TeachBtn::Teach,
            _ if hot(CText::BAbort) => TeachBtn::Abort,
            _ if hot(CText::BOk) => TeachBtn::Ok,
            _ if t.teach && hot(CText::BMod) => TeachBtn::Mod,
            _ => return None,
        })
    }

    pub(crate) fn teach_click(&mut self, t: &mut TeachDlg, act: Act) -> Flow {
        match act {
            Act::Teach(b) => self.teach_press(t, b),
            _ => Flow::Stay,
        }
    }

    fn teach_press(&mut self, t: &mut TeachDlg, b: TeachBtn) -> Flow {
        let n = STEPS[t.step];
        match b {
            TeachBtn::Left => self.teach_jog(t, 0, n, 0),
            TeachBtn::Right => self.teach_jog(t, 0, -n, 0),
            TeachBtn::Forw => self.teach_jog(t, n, 0, 0),
            TeachBtn::Back => self.teach_jog(t, -n, 0, 0),
            TeachBtn::Up => self.teach_jog(t, 0, 0, -n),
            TeachBtn::Down => self.teach_jog(t, 0, 0, n),
            TeachBtn::Step(i) => t.step = i,
            TeachBtn::Teach => {
                t.teach = true;
                t.origin = (t.x, t.y);
                t.points.clear();
            }
            TeachBtn::PlusPos if t.teach => t.points.push(t.relative()),
            TeachBtn::MinusPos if t.teach => {
                t.points.pop();
            }
            TeachBtn::Mod if t.teach => {
                let here = t.relative();
                if let Some(last) = t.points.last_mut() {
                    *last = here;
                }
            }
            TeachBtn::Abort if !t.teach => return Flow::Close,
            TeachBtn::Ok => return self.teach_ok(t),
            _ => {}
        }
        Flow::Stay
    }

    fn teach_jog(&mut self, t: &mut TeachDlg, dx: i32, dy: i32, dz: i32) {
        let before = (t.x, t.y, t.z);
        t.jog(dx, dy, dz);
        // the machine follows by what the position really moved
        self.engine
            .jog(t.x - before.0, t.y - before.1, t.z - before.2);
    }

    /// Ok: in Teach-In mode the file is offered; otherwise the position becomes the Offset of the form.
    fn teach_ok(&mut self, t: &mut TeachDlg) -> Flow {
        if t.teach {
            let copy = TeachDlg {
                points: t.points.clone(),
                ..*t
            };
            return Flow::to(Dialog::TeachWrite {
                yes: true,
                teach: Box::new(copy),
            });
        }
        if !t.teach {
            let round = |mil: i32| (TeachDlg::mm(mil) * 100.0).round() / 100.0;
            self.form.offset = (round(t.x), round(t.y));
            self.apply_form();
            self.dirty = true;
        }
        Flow::Close
    }

    // ------------------------------------------------------------------------------------------ Write Teach-In file?

    pub(crate) fn teach_write_key(
        &mut self,
        yes: &mut bool,
        teach: &TeachDlg,
        input: Input,
    ) -> Flow {
        let hot_yes = hot_letter(self.t(CText::BYes));
        let hot_no = hot_letter(self.t(CText::BNo));
        match input {
            Input::Enter => self.teach_write_answer(*yes, teach),
            Input::Esc => Flow::Close,
            Input::Left | Input::Right | Input::Up | Input::Down | Input::Tab | Input::BackTab => {
                *yes = !*yes;
                Flow::Stay
            }
            Input::Char(c) | Input::Alt(c) => {
                let c = c.to_ascii_lowercase();
                if Some(c) == hot_yes {
                    self.teach_write_answer(true, teach)
                } else if Some(c) == hot_no {
                    Flow::Close
                } else {
                    Flow::Stay
                }
            }
            _ => Flow::Stay,
        }
    }

    pub(crate) fn teach_write_click(&mut self, teach: &TeachDlg, act: Act) -> Flow {
        match act {
            Act::Dlg(Dlg::Yes) => self.teach_write_answer(true, teach),
            Act::Dlg(Dlg::No) => Flow::Close,
            _ => Flow::Stay,
        }
    }

    fn teach_write_answer(&mut self, yes: bool, teach: &TeachDlg) -> Flow {
        if !yes {
            return Flow::Close;
        }
        let path = self.engine.dir().join("TEACH.NCD");
        let msg = match std::fs::write(&path, self.teach_file(teach)) {
            Ok(()) => format!("{} TEACH.NCD", self.t(CText::Saved)),
            Err(e) => format!("{}: {e}", self.t(CText::ErrWrite)),
        };
        Flow::to(Dialog::Message(msg))
    }

    /// The Teach-In file: a title line, then the holes in the unit of the configuration (a drill file this program
    /// reads again).
    fn teach_file(&self, t: &TeachDlg) -> String {
        let unit = self
            .engine
            .settings()
            .map_or(CoordUnit::Mil, |m| m.unit)
            .mm();
        let mut out = String::from("BUNGARD CCD TEACH-IN FILE\r\n%\r\n");
        out.push_str(&format!("T{:02}\r\n", t.tool));
        for (x, y) in &t.points {
            let u = |mil: i32| (TeachDlg::mm(mil) / unit).round() as i64;
            out.push_str(&format!("X{}Y{}\r\n", u(*x), u(*y)));
        }
        out.push_str("M30\r\n");
        out
    }

    // ------------------------------------------------------------------------------------------ drawing

    pub(crate) fn render_teach(&self, s: &mut Screen, t: &TeachDlg) {
        window(s, 5, 1, 70, 21, "", false);
        label(s, 9, 3, self.t(CText::TeachTitle));
        let (rx, ry) = t.relative();
        label(s, 52, 3, &format!("X: {:06.2}", TeachDlg::mm(rx)));
        label(s, 52, 4, &format!("Y: {:06.2}", TeachDlg::mm(ry)));
        label(
            s,
            51,
            5,
            &format!("(Z:{:>7.2})", t.max_z - TeachDlg::mm(t.z)),
        );
        for y in 3..=5 {
            s.put(62, y, '\u{2502}', INK, GREY);
        }
        label(s, 64, 4, "mm");
        label(s, 37, 6, "Y");
        label(s, 51, 9, "X");
        label(s, 66, 7, "Z");
        s.text(
            28,
            10,
            "\u{2500}\u{2500}\u{2500}\u{2502}\u{2500}\u{2500}\u{2500}",
            INK,
            GREY,
        );

        let word = |k: CText| self.t(k);
        let moves = [
            (24, 7, 14, "E (8)", CText::TeachLeft, TeachBtn::Left),
            (11, 10, 14, "S (4)", CText::TeachBack, TeachBtn::Back),
            (37, 10, 14, "D (6)", CText::TeachForw, TeachBtn::Forw),
            (24, 13, 14, "X (2)", CText::TeachRight, TeachBtn::Right),
            (56, 8, 13, "R (9)", CText::TeachUp, TeachBtn::Up),
            (56, 12, 13, "C (3)", CText::TeachDown, TeachBtn::Down),
        ];
        for (x, y, w, key, text, b) in moves {
            // the letter is the hot key: it is written with the marker the buttons understand
            let (letter, rest) = key.split_at(1);
            let caption = format!("~{letter}~{rest} {}", word(text));
            button(s, x, y, w, &caption, false, true);
            self.hit(x, y, w + 1, 2, Act::Teach(b));
        }

        match self.engine.teach_state() {
            Some(crate::runner::TeachState::Homing) => {
                label(s, 9, 20, self.t(CText::HomingXy));
            }
            Some(crate::runner::TeachState::Failed(e)) => {
                let e: String = e.chars().take(44).collect();
                label(s, 9, 20, &e);
            }
            _ => {}
        }
        label(s, 10, 15, self.t(CText::TeachMil));
        let rows: Vec<String> = STEPS.iter().map(|n| format!("{n:>4}")).collect();
        radio_group(s, 10, 16, 13, &rows, t.step, false);
        for (i, _) in STEPS.iter().enumerate() {
            s.text(21, 16 + i, &(i + 1).to_string(), HOT, CYAN);
            self.hit(10, 16 + i, 13, 1, Act::Teach(TeachBtn::Step(i)));
        }

        button(s, 56, 15, 13, self.t(CText::BTeach), false, true);
        self.hit(56, 15, 14, 2, Act::Teach(TeachBtn::Teach));
        button(s, 56, 17, 13, self.t(CText::BAbort), false, !t.teach);
        if !t.teach {
            self.hit(56, 17, 14, 2, Act::Teach(TeachBtn::Abort));
        }
        button(s, 56, 19, 13, self.t(CText::BOk), !t.teach, true);
        self.hit(56, 19, 14, 2, Act::Teach(TeachBtn::Ok));

        if t.teach {
            let last = t.points.last().copied().unwrap_or((0, 0));
            label(s, 28, 16, "N:");
            label(s, 32, 16, &format!("{:04}", t.points.len()));
            label(s, 28, 17, "T:");
            field(s, 32, 17, 2, &format!("{:02}", t.tool), false);
            label(s, 28, 18, "X:");
            field(s, 32, 18, 5, &format!("{:05}", last.0.abs()), false);
            label(s, 28, 19, "Y:");
            field(s, 32, 19, 5, &format!("{:05}", last.1.abs()), false);
            for (y, text, b) in [
                (15, CText::BPlusPos, TeachBtn::PlusPos),
                (17, CText::BMinusPos, TeachBtn::MinusPos),
                (19, CText::BMod, TeachBtn::Mod),
            ] {
                button(s, 43, y, 7, self.t(text), false, true);
                self.hit(43, y, 8, 2, Act::Teach(b));
            }
        }
    }
}
