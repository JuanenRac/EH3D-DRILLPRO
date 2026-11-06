//! The contract of the job request between this program and the server: what a `rack` may say, what the server answers and which words it uses to say it.
//!
//! The pure parts live here so a test can hold the program and the server to the same sentences: the checks of a `rack` that the server makes
//! (`422 bad_rack` with a stable `detail`), the reasons the local planner refuses a rack, and the reading of the answer (`rack_applied` and the warnings
//! `rack_needs_v2`, `rack_no_changer`, `rack_unknown_field` and `request_version_newer`). Nothing here talks to a server.

use serde_json::Value;

use crate::i18n::{cell_width, str_width, Lang};
use crate::job::{JobOptions, RackRefusal};

/// The error code of a `rack` that does not fit.
pub const BAD_RACK: &str = "bad_rack";

/// The most places a `rack` may name by default (the tool changer of the machines has fourteen); the server may allow fewer.
pub const MAX_RACK_PLACES: usize = 14;

/// A `rack` that the server refuses before any job exists, with the sentence it gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RackProblem {
    /// `rack.first` above the limit (the limit).
    FirstTooBig(usize),
    /// More entries in `rack.slots` than the limit (the limit).
    TooManySlots(usize),
    /// Place 0, the test pin, given to a tool (the tool).
    TestPinSlot(u8),
    /// A tool given two places (the tool).
    ToolTwice(u8),
    /// A place given to two tools (the place).
    PlaceTwice(usize),
    /// `carry_slot` without `carry_in`.
    CarrySlotNeedsCarryIn,
    /// `carry_slot` 0.
    CarrySlotTestPin,
    /// `carry_slot` that is also the place of a tool (the place).
    CarrySlotTaken(usize),
}

impl RackProblem {
    /// The code of the error: always `bad_rack`.
    pub fn code(&self) -> &'static str {
        BAD_RACK
    }

    /// The stable sentence, the same the server puts in `detail`.
    pub fn detail(&self) -> String {
        match self {
            RackProblem::FirstTooBig(most) => format!("rack.first may be up to {most} places"),
            RackProblem::TooManySlots(most) => format!("rack.slots may name up to {most} places"),
            RackProblem::TestPinSlot(tool) => {
                format!("rack.slots: place 0 is the test pin, not a place for tool {tool}")
            }
            RackProblem::ToolTwice(tool) => {
                format!("rack.slots: tool {tool} is given two places")
            }
            RackProblem::PlaceTwice(place) => {
                format!("rack.slots: place {place} is given to two tools")
            }
            RackProblem::CarrySlotNeedsCarryIn => "rack.carry_slot needs rack.carry_in".into(),
            RackProblem::CarrySlotTestPin => "rack.carry_slot: place 0 is the test pin".into(),
            RackProblem::CarrySlotTaken(place) => {
                format!("rack.carry_slot: place {place} is also the place of a tool of this job")
            }
        }
    }
}

impl std::fmt::Display for RackProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.detail())
    }
}

/// The checks the server makes of the `rack` of a request, in the same order and with the same sentences: the first problem is the one that is told.
/// `most` is the limit of the server (`rack_max_slots`, 1 to 14; [`MAX_RACK_PLACES`] by default). The checks against the machine file (a place it does not
/// define or does not reach, more tools than the rack holds) are those of [`crate::job::job_rack_for`], told by [`refusal_detail`].
pub fn check_rack_options(options: &JobOptions, most: usize) -> Result<(), RackProblem> {
    if options.rack_first > most {
        return Err(RackProblem::FirstTooBig(most));
    }
    if options.rack_slots.len() > most {
        return Err(RackProblem::TooManySlots(most));
    }
    let mut tools: Vec<u8> = Vec::new();
    let mut places: Vec<usize> = Vec::new();
    for &(tool, place) in &options.rack_slots {
        if place == 0 {
            return Err(RackProblem::TestPinSlot(tool));
        }
        if tools.contains(&tool) {
            return Err(RackProblem::ToolTwice(tool));
        }
        if places.contains(&place) {
            return Err(RackProblem::PlaceTwice(place));
        }
        tools.push(tool);
        places.push(place);
    }
    if let Some(carried) = options.rack_carry_slot {
        if !options.rack_carry_in {
            return Err(RackProblem::CarrySlotNeedsCarryIn);
        }
        if carried == 0 {
            return Err(RackProblem::CarrySlotTestPin);
        }
        if places.contains(&carried) {
            return Err(RackProblem::CarrySlotTaken(carried));
        }
    }
    Ok(())
}

/// The sentence the server gives (`detail` of `bad_rack`) when the machine file does not fit the rack of a job: what the local planner refuses with a
/// [`RackRefusal`] is told in the same words.
pub fn refusal_detail(refusal: &RackRefusal) -> String {
    match refusal {
        RackRefusal::TooManyTools(n) => {
            format!("the rack of the machine holds {n} tools at the most")
        }
        RackRefusal::Gap(place) => format!(
            "place {place} of the rack is not defined in the configuration file of the machine"
        ),
        RackRefusal::Outside(place) => {
            format!("place {place} of the rack is out of the reach of the machine")
        }
    }
}

/// The codes of the warnings of the answer (`view.warnings`) that are about the request.
pub mod warning {
    pub const RACK_NEEDS_V2: &str = "rack_needs_v2";
    pub const RACK_NO_CHANGER: &str = "rack_no_changer";
    pub const RACK_UNKNOWN_FIELD: &str = "rack_unknown_field";
    pub const REQUEST_VERSION_NEWER: &str = "request_version_newer";
    pub const OUT_OF_COURSE: &str = "out_of_course";
}

/// A warning of the server about the request, by its code. A code this program does not know is kept as it came.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerWarning {
    /// `rack` came in a request of the first version.
    RackNeedsV2,
    /// The machine changes its tools by hand.
    RackNoChanger,
    /// A field of `rack` the server does not know (its name); the server ignored it.
    RackUnknownField(String),
    /// `request_version` above what the server speaks (the version asked for); it read the request as the latest it knows.
    RequestVersionNewer(u64),
    /// The work goes past the course of an axis of the machine (`axis`, `X`, `Y` or `Z`, and how far past it in thousandths of a millimetre).
    OutOfCourse { axis: String, excess_um: u64 },
    /// Any other code (a later version of the server).
    Other(String),
}

/// What the answer to a job request says about the `rack`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChainAnswer {
    /// `Some(true)` when the plan used the `rack`, `Some(false)` when it was left out, `None` when the answer does not say (the request had none, or an older server).
    pub applied: Option<bool>,
    /// The warnings about the request, in the order of the server.
    pub warnings: Vec<AnswerWarning>,
}

/// Reads `rack_applied` and the warnings of the answer of the server to a job request (the job, as `POST .../jobs` and `.../preview` give it). A field
/// that is missing or of another type is read as not said; nothing here fails.
pub fn chain_answer(job: &Value) -> ChainAnswer {
    let applied = job["rack_applied"]
        .as_bool()
        .or_else(|| job["view"]["rack_applied"].as_bool());
    let warnings = job["view"]["warnings"]
        .as_array()
        .map(|list| list.iter().filter_map(read_warning).collect())
        .unwrap_or_default();
    ChainAnswer { applied, warnings }
}

fn read_warning(entry: &Value) -> Option<AnswerWarning> {
    let code = entry["code"].as_str()?;
    Some(match code {
        warning::RACK_NEEDS_V2 => AnswerWarning::RackNeedsV2,
        warning::RACK_NO_CHANGER => AnswerWarning::RackNoChanger,
        warning::RACK_UNKNOWN_FIELD => {
            AnswerWarning::RackUnknownField(entry["detail"].as_str().unwrap_or("").to_string())
        }
        warning::REQUEST_VERSION_NEWER => {
            let asked = entry["values"][0].as_f64().unwrap_or(0.0);
            AnswerWarning::RequestVersionNewer(if asked >= 0.0 { asked as u64 } else { 0 })
        }
        warning::OUT_OF_COURSE => {
            let excess = entry["values"][0].as_f64().unwrap_or(0.0);
            AnswerWarning::OutOfCourse {
                axis: entry["axis"].as_str().unwrap_or("").to_string(),
                excess_um: if excess > 0.0 {
                    (excess * 1000.0).round() as u64
                } else {
                    0
                },
            }
        }
        other => AnswerWarning::Other(other.to_string()),
    })
}

/// The most rows of a screen that the warnings of the server take.
pub const WARNING_ROWS: usize = 4;

/// What a text of the server may bring into a line of the screen: no control characters and no more than `most` characters.
fn plain(text: &str, most: usize) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(most)
        .collect()
}

/// A warning of the server about the request in the words of a language (`None` for a code this program does not know, which is not shown). The text the server
/// sends along (the name of a field, the axis) goes into the line only after its control characters are taken out and its length cut.
pub fn warning_text(warning: &AnswerWarning, lang: Lang) -> Option<String> {
    use eh3d_view::texts::{tr, Text};
    let lang: eh3d_view::texts::Lang = lang.into();
    Some(match warning {
        AnswerWarning::RackNeedsV2 => tr(lang, Text::WarnRackNeedsV2).to_string(),
        AnswerWarning::RackNoChanger => tr(lang, Text::WarnRackNoChanger).to_string(),
        AnswerWarning::RackUnknownField(name) => {
            format!(
                "{} {}",
                tr(lang, Text::WarnRackUnknownField),
                plain(name, 32)
            )
        }
        AnswerWarning::RequestVersionNewer(asked) => {
            format!(
                "{} (request_version {asked})",
                tr(lang, Text::WarnRequestNewer)
            )
        }
        AnswerWarning::OutOfCourse { axis, excess_um } => format!(
            "{} {} (+{}.{:03} mm)",
            tr(lang, Text::WarnOutOfCourse),
            plain(axis, 3),
            excess_um / 1000,
            excess_um % 1000
        ),
        AnswerWarning::Other(_) => return None,
    })
}

/// A text cut into lines of at most `width` cells of the screen (a wide character takes two): at the blanks, and inside a word that is longer than a line (the
/// languages written without blanks).
pub fn wrap_cells(text: &str, width: usize) -> Vec<String> {
    let width = width.max(2);
    let mut lines = Vec::new();
    let (mut line, mut used) = (String::new(), 0usize);
    for word in text.split_whitespace() {
        let need = str_width(word);
        if !line.is_empty() && used + 1 + need <= width {
            line.push(' ');
            line.push_str(word);
            used += 1 + need;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            used = 0;
        }
        for c in word.chars() {
            let cells = cell_width(c);
            if used + cells > width {
                lines.push(std::mem::take(&mut line));
                used = 0;
            }
            line.push(c);
            used += cells;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// The rows of the screen that tell the warnings of the server, in order, each cut to `width` cells. When they do not fit in `most` rows the last row ends in
/// `...`.
pub fn warning_lines(
    warnings: &[AnswerWarning],
    lang: Lang,
    width: usize,
    most: usize,
) -> Vec<String> {
    let mut lines: Vec<String> = warnings
        .iter()
        .filter_map(|w| warning_text(w, lang))
        .flat_map(|text| wrap_cells(&text, width))
        .collect();
    if lines.len() > most {
        lines.truncate(most);
        if let Some(last) = lines.last_mut() {
            let mut kept = String::new();
            let mut used = 0usize;
            for c in last.chars() {
                let cells = cell_width(c);
                if used + cells + 3 > width {
                    break;
                }
                kept.push(c);
                used += cells;
            }
            *last = format!("{}...", kept.trim_end());
        }
    }
    lines
}

/// Whether the request asked for the chaining (it carries a `rack`) and the server did not say it used it.
pub fn rack_ignored(request: &Value, job: &Value) -> bool {
    request.get("rack").is_some() && chain_answer(job).applied != Some(true)
}

/// The fields of `rack` the server of this version reads (`JobRackV2`). A program sends no other: a field the server does not know is left out and
/// told with the warning `rack_unknown_field`.
pub const KNOWN_RACK_FIELDS: [&str; 5] = ["first", "carry_in", "carry_out", "slots", "carry_slot"];

/// The names of the fields of the `rack` of a request that the server of this version would not know (none when there is no `rack`).
pub fn unknown_rack_fields(request: &Value) -> Vec<String> {
    request["rack"]
        .as_object()
        .map(|rack| {
            rack.keys()
                .filter(|key| !KNOWN_RACK_FIELDS.contains(&key.as_str()))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// The chaining of a request as the options of a job, read the way the server reads it (every field optional, a missing one with its default): the inverse of
/// [`crate::server_link::with_chain`]. `None` when the request has no `rack`, or when its `rack` is not an object. A field of the wrong type, or a number
/// that is negative or does not fit, is read as missing.
pub fn chain_options(request: &Value) -> Option<JobOptions> {
    let rack = request["rack"].as_object()?;
    let number = |key: &str| {
        rack.get(key)
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
    };
    let flag = |key: &str| rack.get(key).and_then(Value::as_bool).unwrap_or(false);
    let slots = rack
        .get("slots")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|slot| {
                    let tool = u8::try_from(slot["tool"].as_u64()?).ok()?;
                    let place = usize::try_from(slot["place"].as_u64()?).ok()?;
                    Some((tool, place))
                })
                .collect()
        })
        .unwrap_or_default();
    Some(JobOptions {
        rack_first: number("first").unwrap_or(0),
        rack_carry_in: flag("carry_in"),
        rack_carry_out: flag("carry_out"),
        rack_slots: slots,
        rack_carry_slot: number("carry_slot"),
        ..JobOptions::default()
    })
}
