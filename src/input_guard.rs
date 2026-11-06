//! Limits on the files the programs read, checked before the reader sees them: a file that is far too big, that is not text, whose lines run on for ever
//! or that carries a number of absurd length is turned down with a stable code and the number of its line, instead of being handed to a reader that
//! was written for the files of the machines.
//!
//! The limits are wide: a drill file, a plot, a drawing or an outline of any real board is far inside them. The codes are part of the contract and do not
//! change: `input_too_large`, `input_binary`, `input_line_too_long` and `input_number_too_long`.

use std::fmt;

/// The most bytes a file may have (a drill file of a million holes is about 15 MB).
pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;

/// The most characters a line may have in the formats that are written by lines (drill files and drawings). A plot (HP-GL) and the outline of a board
/// (Gerber) put many commands on one line and are not held to this.
pub const MAX_LINE_CHARS: usize = 16_384;

/// The most digits in a row a number may have (a real coordinate has fewer than twenty).
pub const MAX_NUMBER_DIGITS: usize = 40;

/// The kind of file, which decides the limits that apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    /// Excellon drill file (`.NCD`).
    Ncd,
    /// HP-GL plot (`.PLT`).
    Hpgl,
    /// Drawing (`.DXF`).
    Dxf,
    /// Outline of a board in Gerber.
    Gerber,
}

impl InputKind {
    /// Whether the lines of the format are short by nature, so a very long one is a sign of a wrong file.
    fn by_lines(self) -> bool {
        matches!(self, InputKind::Ncd | InputKind::Dxf)
    }
}

/// Why a file is turned down. The lines count from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputProblem {
    /// The file has more bytes than [`MAX_INPUT_BYTES`] (its size).
    TooLarge(u64),
    /// A character 0 in a text format (the line it is in): a binary file, or text in two bytes per character.
    Binary(usize),
    /// A line with more than [`MAX_LINE_CHARS`] characters (its number).
    LineTooLong(usize),
    /// A number with more than [`MAX_NUMBER_DIGITS`] digits (the line it is in).
    NumberTooLong(usize),
}

impl InputProblem {
    /// The stable code.
    pub fn code(&self) -> &'static str {
        match self {
            InputProblem::TooLarge(_) => "input_too_large",
            InputProblem::Binary(_) => "input_binary",
            InputProblem::LineTooLong(_) => "input_line_too_long",
            InputProblem::NumberTooLong(_) => "input_number_too_long",
        }
    }

    /// The line the problem is on, when it is about a line.
    pub fn line(&self) -> Option<usize> {
        match self {
            InputProblem::TooLarge(_) => None,
            InputProblem::Binary(n)
            | InputProblem::LineTooLong(n)
            | InputProblem::NumberTooLong(n) => Some(*n),
        }
    }
}

impl fmt::Display for InputProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line() {
            Some(line) => write!(f, "{} (line {line})", self.code()),
            None => write!(f, "{}", self.code()),
        }
    }
}

/// Turns a file down when it has more bytes than [`MAX_INPUT_BYTES`].
pub fn check_len(bytes: u64) -> Result<(), InputProblem> {
    if bytes > MAX_INPUT_BYTES as u64 {
        Err(InputProblem::TooLarge(bytes))
    } else {
        Ok(())
    }
}

/// Checks the text of a file (one character per byte, as the programs read it) against the limits of its kind. The first problem found is the one told.
pub fn check(kind: InputKind, text: &str) -> Result<(), InputProblem> {
    check_len(text.len() as u64)?;
    for (index, line) in text.split('\n').enumerate() {
        let number = index + 1;
        if line.contains('\0') {
            return Err(InputProblem::Binary(number));
        }
        if kind.by_lines() && line.chars().count() > MAX_LINE_CHARS {
            return Err(InputProblem::LineTooLong(number));
        }
        let mut run = 0usize;
        for byte in line.bytes() {
            if byte.is_ascii_digit() {
                run += 1;
                if run > MAX_NUMBER_DIGITS {
                    return Err(InputProblem::NumberTooLong(number));
                }
            } else {
                run = 0;
            }
        }
    }
    Ok(())
}
