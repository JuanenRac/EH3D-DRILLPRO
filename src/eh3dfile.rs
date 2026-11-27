//! The `.eh3d` project of the suite: the drill layers are read from the job of the project (the branch `manufacturing/`) and the plan made on paper from one of them is kept in the
//! branch `drillpro/` only, with a copy of the layer and a reference to the content of the job, so that a job that changes makes the plans **out of date**. The reading, the writing and
//! the references are the ones every manufacturing program shares (`eh3d_project::manufacturing` of EH3D-COMMON). A plan is a text: no file for the machine is made and no machine is
//! needed. ROUTEPRO uses [`ProjectJob`] with its own branch.

use std::path::{Path, PathBuf};

use eh3d_project::appbranch::BranchKit;
use eh3d_project::manufacturing::{self as suite, JobFile};
use eh3d_project::suitetext::{fill, S};
use eh3d_project::suiteview;

use crate::app::Input;
use crate::i18n::{cell_width, str_width, Lang};
use crate::screen::{Screen, BLACK, COLS, LIGHT_GRAY, RED, WHITE, YELLOW};
use crate::texts::{tr, Text};

/// The branch of this program.
pub const KIT: BranchKit<'static> =
    suite::kit("drillpro", "drillpro.json", env!("CARGO_PKG_VERSION"));

/// The drill layers of the job of a project; an error when the job has none.
pub fn drill_layers(path: &Path) -> Result<Vec<JobFile>, String> {
    suite::layers_of(path, "drill")
}

/// The drill layer of the job called `layer` (its path inside the job or its file name). A file name that two layers share is refused: the path says which.
pub fn choose<'a>(layers: &'a [JobFile], layer: &str) -> Result<&'a JobFile, String> {
    let shared: Vec<&str> = layers
        .iter()
        .filter(|f| f.file_name() == layer)
        .map(|f| f.rel.as_str())
        .collect();
    if shared.len() > 1 {
        return Err(format!(
            "{layer} is the name of more than one drill layer of the job ({}): give its path",
            shared.join(", ")
        ));
    }
    suite::choose(layers, "drill", layer)
}

/// The name of the plan of a layer inside `result/`: `drill-<name of the layer>.txt`. When another of the `layers` would get the same name (`board.drl` and `board.xln`), the
/// whole path of the layer is kept after `drill+`, so that the plan of one layer never takes the place of the plan of another.
pub fn result_name(layers: &[JobFile], layer: &JobFile) -> String {
    let short = suite::result_name("drill", layer, "txt");
    let shared = layers.iter().any(|other| {
        other.rel != layer.rel
            && suite::result_name("drill", other, "txt").to_lowercase() == short.to_lowercase()
    });
    if !shared {
        return short;
    }
    let whole = layer.rel.strip_prefix("layers/").unwrap_or(&layer.rel);
    format!("drill+{}.txt", whole.replace('/', "+"))
}

/// [`result_name`] with the layers of the job as the project at `path` has them now.
pub fn plan_name(path: &Path, layer: &JobFile) -> String {
    let layers = drill_layers(path).unwrap_or_default();
    result_name(&layers, layer)
}

/// The mark of the project at `path` as the program of `kit` sees it now: it is taken before the layers are read and again after a write.
pub fn mark_of(kit: &BranchKit<'_>, path: &Path) -> Result<String, String> {
    let view = suiteview::view(path, kit.branch)?;
    Ok(view.own_hash.unwrap_or_default())
}

/// Keeps the plan of one layer in the branch of this program. The plans of the other layers stay while the job is the one they were made from. Returns what a person should be told.
///
/// `seen` is the mark of the project as it was read ([`mark_of`]). When another program saved the file since, nothing is written (`EH3D-PKG-025`); what this program wrote
/// itself does not count.
pub fn save_plan_since(
    path: &Path,
    seen: &str,
    layer: &JobFile,
    lines: &[String],
) -> Result<Vec<String>, String> {
    suite::keep_result_since(
        &KIT,
        path,
        seen,
        layer,
        &plan_name(path, layer),
        plan_bytes(lines),
    )
}

/// What keeping a plan in the project at `path` would take away (its signature, or the one of the branch of this program), in the language of a code such as `es`: the person has to
/// agree to it before anything is written. Empty when there is nothing to ask. Refused when another program saved the project since the mark `seen` was taken
/// (see [`suiteview::is_changed_refusal`]).
pub fn signature_warnings(path: &Path, seen: &str, code: &str) -> Result<Vec<String>, String> {
    let said = suiteview::warnings(path, Some(path), Some(seen), KIT.branch)?;
    Ok(said.iter().map(|w| w.text(code)).collect())
}

/// The bytes of a plan as it is kept: one line of the report on each line.
pub fn plan_bytes(lines: &[String]) -> Vec<u8> {
    let mut text = lines.join("\n");
    text.push('\n');
    text.into_bytes()
}

/// The layer of the job of a project that a window has open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectJob {
    /// The project.
    pub path: PathBuf,
    /// The layer that is open: the first of its kind in the job.
    pub layer: JobFile,
    /// How many layers of that kind the job has.
    pub layers: usize,
    /// Whether the job changed since the plans that are kept were made.
    pub out_of_date: bool,
    /// The mark of the project as it was read, or as this program last wrote it: a project that another program saved since is not written over.
    pub seen: String,
}

impl ProjectJob {
    /// Reads the first layer of a kind from the job, with the state of the branch of `kit`.
    pub fn open(kit: &BranchKit<'_>, path: &Path, kind: &str) -> Result<ProjectJob, String> {
        // the mark is taken before the layers are read: what another program saves after it is not written over
        let seen = mark_of(kit, path);
        let mut layers = suite::layers_of(path, kind)?;
        let out_of_date = suite::state(kit, path)?.out_of_date;
        let count = layers.len();
        Ok(ProjectJob {
            path: path.to_path_buf(),
            layer: layers.swap_remove(0),
            layers: count,
            out_of_date,
            seen: seen?,
        })
    }

    /// What the window says once the layer is open, in the language of `code`.
    pub fn notice(&self, code: &str) -> String {
        if self.out_of_date {
            S::JobStale.tr(code).to_string()
        } else {
            fill(S::JobOpened.tr(code), &[&self.layers.to_string()])
        }
    }

    /// What the window says once the plan called `name` is kept.
    pub fn saved_notice(name: &str, code: &str) -> String {
        fill(S::JobSaved.tr(code), &[name])
    }
}

/// What a screen offers when a write is refused because another program saved the project, in the order it is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Read the project again; what the other program saved stays.
    ReadAgain,
    /// Keep the work in a copy of the project under the name in the box.
    SaveAs,
    /// Nothing is read and nothing is written.
    Close,
}

const CHOICES: [Choice; 3] = [Choice::ReadAgain, Choice::SaveAs, Choice::Close];
/// The longest name that can be typed.
const MAX_NAME: usize = 200;
/// The window: its width, its place and the room inside it.
const WIDTH: usize = 72;
const LEFT: usize = (COLS - WIDTH) / 2;
const TOP: usize = 3;
const ROOM: usize = WIDTH - 4;

/// The window of a project that another program saved since it was read (`EH3D-PKG-025`): it says so in the language of the screen and offers to read the
/// project again, to save a copy under another name or to close. Typing changes the name; `Enter` presses the button in focus, which is `Close` at first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed {
    path: PathBuf,
    name: String,
    focus: usize,
    problem: Option<String>,
}

impl Changed {
    /// The window for the project at `path`, with a name that is not taken in the box.
    pub fn new(path: &Path) -> Changed {
        let stem = path
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let name = (2..1000)
            .map(|n| format!("{stem}-{n}.eh3d"))
            .find(|name| !path.with_file_name(name).exists())
            .unwrap_or_default();
        Changed {
            path: path.to_path_buf(),
            name,
            focus: 2,
            problem: None,
        }
    }

    /// The project that another program saved.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The name in the box.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Says why a choice could not be carried out.
    pub fn say(&mut self, why: String) {
        self.problem = Some(why);
    }

    /// The refusal in the language of the screen.
    pub fn message(&self, lang: Lang) -> String {
        fill(
            S::ChangedOnDisk.tr(lang.code()),
            &[&suiteview::name_of(&self.path)],
        )
    }

    /// Copies the project, as it is now, under the name in the box and gives the path of the copy. A file that is there is never written over.
    pub fn copy(&self, lang: Lang) -> Result<PathBuf, String> {
        let name = self.name.trim();
        let target = suiteview::with_extension(&self.path.with_file_name(name));
        if name.is_empty() || !suiteview::is_project(&target) {
            return Err(S::NeedExtension.tr(lang.code()).to_string());
        }
        if target.exists() {
            return Err(tr(lang, Text::ProjectExists).replace("{0}", &suiteview::name_of(&target)));
        }
        suiteview::copy_project(&self.path, &target)?;
        Ok(target)
    }

    /// A key; the choice when one is made.
    pub fn input(&mut self, input: Input) -> Option<Choice> {
        match input {
            Input::Esc => return Some(Choice::Close),
            Input::Enter => return Some(CHOICES[self.focus]),
            Input::Up | Input::BackTab => self.focus = (self.focus + 2) % CHOICES.len(),
            Input::Down | Input::Tab => self.focus = (self.focus + 1) % CHOICES.len(),
            Input::Char(c) if !c.is_control() => {
                if self.name.chars().count() < MAX_NAME {
                    self.name.push(c);
                }
                self.focus = 1;
            }
            Input::Backspace => {
                self.name.pop();
                self.focus = 1;
            }
            _ => {}
        }
        None
    }

    /// The choice of the button at a cell, when there is one.
    pub fn click(&self, col: usize, row: usize, lang: Lang) -> Option<Choice> {
        let first = self.first_button_row(lang);
        CHOICES.iter().enumerate().find_map(|(i, choice)| {
            let width = str_width(&button(*choice, lang));
            let x = LEFT + WIDTH.saturating_sub(width) / 2;
            (row == first + i && (x..x + width).contains(&col)).then_some(*choice)
        })
    }

    fn said(&self, lang: Lang) -> (Vec<String>, Vec<String>) {
        let mut message = wrap(&self.message(lang), ROOM);
        message.truncate(6);
        let mut problem = self
            .problem
            .as_deref()
            .map_or_else(Vec::new, |p| wrap(p, ROOM));
        problem.truncate(3);
        (message, problem)
    }

    fn first_button_row(&self, lang: Lang) -> usize {
        let (message, problem) = self.said(lang);
        // the border, the message, a gap, the name, the problem and a gap
        TOP + 1 + message.len() + 1 + 1 + problem.len() + 1
    }

    /// Draws the window.
    pub fn render(&self, s: &mut Screen, lang: Lang) {
        let (message, problem) = self.said(lang);
        let first = self.first_button_row(lang);
        let title = wrap(&suiteview::name_of(&self.path), ROOM - 4);
        s.window(
            LEFT,
            TOP,
            WIDTH,
            first + CHOICES.len() + 1 - TOP,
            title.first().map_or("", String::as_str),
            WHITE,
            RED,
        );
        let mut row = TOP + 1;
        for line in &message {
            s.text(LEFT + 2, row, line, WHITE, RED);
            row += 1;
        }
        row += 1;
        // the end of a long name stays in sight, where it is typed
        let mut shown = String::new();
        for c in self.name.chars().rev() {
            if str_width(&shown) + cell_width(c) >= ROOM {
                break;
            }
            shown.insert(0, c);
        }
        s.fill(LEFT + 2, row, ROOM, 1, ' ', BLACK, LIGHT_GRAY);
        s.text(LEFT + 2, row, &format!("{shown}_"), BLACK, LIGHT_GRAY);
        row += 1;
        for line in &problem {
            s.text(LEFT + 2, row, line, YELLOW, RED);
            row += 1;
        }
        for (i, choice) in CHOICES.iter().enumerate() {
            let (fg, bg) = if i == self.focus {
                (BLACK, LIGHT_GRAY)
            } else {
                (YELLOW, RED)
            };
            let label = button(*choice, lang);
            let x = LEFT + WIDTH.saturating_sub(str_width(&label)) / 2;
            s.text(x, first + i, &label, fg, bg);
        }
    }
}

fn button(choice: Choice, lang: Lang) -> String {
    let word = match choice {
        Choice::ReadAgain => S::ReadAgain,
        Choice::SaveAs => S::MenuSaveAs,
        Choice::Close => S::Close,
    };
    format!("[ {} ]", word.tr(lang.code()))
}

/// Lines of at most `width` cells, cut at a space where there is one.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() {
            if str_width(&line) + 1 + str_width(word) <= width {
                line.push(' ');
                line.push_str(word);
                continue;
            }
            lines.push(std::mem::take(&mut line));
        }
        for c in word.chars() {
            if str_width(&line) + cell_width(c) > width {
                lines.push(std::mem::take(&mut line));
            }
            line.push(c);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
