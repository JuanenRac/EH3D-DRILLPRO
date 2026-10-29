//! Excellon drill files (`.NCD`): imperial, integer coordinates in mils.
//!
//! Real-world files come in two shapes: a bare body (`%`, `T06`, `X..Y..`) and
//! an Excellon header (`M48 ... %`) with optional tool definitions (`T1C0.8`).

use std::fmt;

/// One mil is 0.001 inch.
pub const MIL_MM: f64 = 0.0254;

#[derive(Debug, Clone, PartialEq)]
pub struct Hole {
    pub tool: u8,
    pub x_mil: i32,
    pub y_mil: i32,
}

impl Hole {
    pub fn x_mm(&self) -> f64 {
        self.x_mil as f64 * MIL_MM
    }
    pub fn y_mm(&self) -> f64 {
        self.y_mil as f64 * MIL_MM
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct DrillFile {
    pub holes: Vec<Hole>,
    /// Tools in order of first use.
    pub tools: Vec<u8>,
    /// Diameters declared in the header (`T1C0.8`), as `(tool, value)` in the file's unit.
    pub diameters: Vec<(u8, f64)>,
    /// True when the header declares metric units.
    pub metric: bool,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    BadLine { line: usize, text: String },
    HoleBeforeTool { line: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::BadLine { line, text } => write!(f, "line {line}: cannot parse {text:?}"),
            ParseError::HoleBeforeTool { line } => {
                write!(f, "line {line}: coordinate before any tool")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses `X..Y..` where either axis may be omitted (modal, keeps the previous value).
fn parse_coord(s: &str, x: &mut i32, y: &mut i32) -> Option<()> {
    let mut rest = s;
    let mut seen = false;
    while let Some(c) = rest.chars().next() {
        if c != 'X' && c != 'Y' {
            return None;
        }
        let body = &rest[1..];
        let end = body
            .find(|d: char| !(d.is_ascii_digit() || d == '-' || d == '+'))
            .unwrap_or(body.len());
        let v: i32 = body[..end].parse().ok()?;
        if c == 'X' {
            *x = v;
        } else {
            *y = v;
        }
        seen = true;
        rest = &body[end..];
    }
    seen.then_some(())
}

pub fn parse(text: &str) -> Result<DrillFile, ParseError> {
    let mut out = DrillFile::default();
    let (mut tool, mut x, mut y) = (None::<u8>, 0, 0);
    let mut in_header = false;
    // Old teach-in files start with free text (title, unit note) that ends at the first `%`.
    let free_text = text.lines().next().is_some_and(|l| {
        let l = l.trim();
        l != "M48" && !l.starts_with('%')
    }) && text.lines().any(|l| l.trim().starts_with('%'))
        && !text.lines().any(|l| l.trim() == "M48");
    let mut skipping = free_text;
    for (n, raw) in text.lines().enumerate() {
        if skipping {
            skipping = !raw.trim().starts_with('%');
            continue;
        }
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let bad = || ParseError::BadLine {
            line: n + 1,
            text: raw.into(),
        };
        if line == "M48" {
            in_header = true;
            continue;
        }
        if line.starts_with('%') {
            in_header = false;
            continue;
        }
        if line.starts_with('M') {
            // M72 = inch, M71 = metric; M30 end of program; others carry no geometry.
            // `METRIC` of the header begins with an M too
            if line == "M71" || line.starts_with("METRIC") {
                out.metric = true;
            }
            continue;
        }
        if in_header {
            if line.starts_with("METRIC") {
                out.metric = true;
            }
            if let Some(def) = line.strip_prefix('T') {
                if let Some((num, dia)) = def.split_once('C') {
                    let t: u8 = num.parse().map_err(|_| bad())?;
                    // Trailing feed/speed words (F.., S..) may follow the diameter.
                    let end = dia
                        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                        .unwrap_or(dia.len());
                    out.diameters
                        .push((t, dia[..end].parse().map_err(|_| bad())?));
                }
            }
            continue;
        }
        if line.starts_with('G') || line.starts_with("INCH") || line.starts_with("METRIC") {
            continue;
        }
        if let Some(num) = line.strip_prefix('T') {
            let t: u8 = num.parse().map_err(|_| bad())?;
            if !out.tools.contains(&t) {
                out.tools.push(t);
            }
            tool = Some(t);
            continue;
        }
        let t = tool.ok_or(ParseError::HoleBeforeTool { line: n + 1 })?;
        parse_coord(line, &mut x, &mut y).ok_or_else(bad)?;
        out.holes.push(Hole {
            tool: t,
            x_mil: x,
            y_mil: y,
        });
    }
    Ok(out)
}
