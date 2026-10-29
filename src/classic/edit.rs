//! One-line text editing for the classic fields, and the file name patterns of the Select File dialog.

use crate::app::Input;

/// What a field accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Digits, a decimal point or comma, and a sign.
    Number,
    /// Any printable character (a folder, a file name pattern).
    Text,
}

/// The text of a field being edited. A field that was just entered shows its text selected: the first character
/// typed replaces all of it, which is what makes retyping a value quick.
#[derive(Debug, Clone, PartialEq)]
pub struct LineEdit {
    pub text: String,
    /// Cursor position in characters.
    pub cursor: usize,
    /// The text is untouched so far: typing replaces it.
    pub fresh: bool,
    pub kind: Kind,
    pub max: usize,
}

impl LineEdit {
    pub fn new(text: &str, kind: Kind, max: usize) -> Self {
        LineEdit {
            text: text.to_string(),
            cursor: text.chars().count(),
            fresh: true,
            kind,
            max,
        }
    }

    fn accepts(&self, c: char) -> bool {
        match self.kind {
            Kind::Number => c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+'),
            Kind::Text => !c.is_control(),
        }
    }

    fn byte_at(&self, chars: usize) -> usize {
        self.text
            .char_indices()
            .nth(chars)
            .map_or(self.text.len(), |(i, _)| i)
    }

    /// Handles a key; true when the key was an editing key (it is then consumed).
    pub fn key(&mut self, input: Input) -> bool {
        let len = self.text.chars().count();
        match input {
            Input::Char(c) if self.accepts(c) => {
                if self.fresh {
                    self.text.clear();
                    self.cursor = 0;
                }
                self.fresh = false;
                if self.text.chars().count() < self.max {
                    let at = self.byte_at(self.cursor);
                    self.text.insert(at, c);
                    self.cursor += 1;
                }
                true
            }
            Input::Backspace => {
                if self.fresh {
                    self.text.clear();
                    self.cursor = 0;
                } else if self.cursor > 0 {
                    let at = self.byte_at(self.cursor - 1);
                    self.text.remove(at);
                    self.cursor -= 1;
                }
                self.fresh = false;
                true
            }
            Input::Delete => {
                if self.fresh {
                    self.text.clear();
                    self.cursor = 0;
                } else if self.cursor < len {
                    let at = self.byte_at(self.cursor);
                    self.text.remove(at);
                }
                self.fresh = false;
                true
            }
            Input::Left => {
                self.fresh = false;
                self.cursor = self.cursor.saturating_sub(1);
                true
            }
            Input::Right => {
                self.fresh = false;
                self.cursor = (self.cursor + 1).min(len);
                true
            }
            Input::Home => {
                self.fresh = false;
                self.cursor = 0;
                true
            }
            Input::End => {
                self.fresh = false;
                self.cursor = len;
                true
            }
            _ => false,
        }
    }
}

/// Matches `name` against a DOS-style pattern with `*` and `?`, ignoring capitals.
pub fn glob_matches(pattern: &str, name: &str) -> bool {
    fn go(p: &[char], n: &[char]) -> bool {
        match p.split_first() {
            None => n.is_empty(),
            Some(('*', rest)) => (0..=n.len()).any(|i| go(rest, &n[i..])),
            Some(('?', rest)) => !n.is_empty() && go(rest, &n[1..]),
            Some((c, rest)) => n.first().is_some_and(|x| x == c) && go(rest, &n[1..]),
        }
    }
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    go(&p, &n)
}
