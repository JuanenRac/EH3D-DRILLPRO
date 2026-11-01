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
    /// The text the field had when it was entered (what Undo brings back).
    original: String,
}

impl LineEdit {
    pub fn new(text: &str, kind: Kind, max: usize) -> Self {
        LineEdit {
            text: text.to_string(),
            cursor: text.chars().count(),
            fresh: true,
            kind,
            max,
            original: text.to_string(),
        }
    }

    /// The text that is selected: all of it while the field is untouched, nothing once the cursor has moved.
    pub fn selected_text(&self) -> Option<&str> {
        (self.fresh && !self.text.is_empty()).then_some(self.text.as_str())
    }

    /// Selects all the text (the next character typed or pasted replaces it).
    pub fn select_all(&mut self) {
        self.fresh = true;
        self.cursor = self.text.chars().count();
    }

    /// Takes the selected text out of the field and gives it back.
    pub fn cut(&mut self) -> Option<String> {
        let t = self.selected_text()?.to_string();
        self.text.clear();
        self.cursor = 0;
        self.fresh = false;
        Some(t)
    }

    /// Puts pasted text in the field at the cursor, in place of the selection: only the first line, without the spaces round it,
    /// only the characters the field takes, and no more than the field can hold. Nothing changes when none of it fits the field.
    pub fn paste(&mut self, pasted: &str) {
        let line = pasted.lines().next().unwrap_or("").trim();
        let take: Vec<char> = line.chars().filter(|c| self.accepts(*c)).collect();
        for c in take {
            self.key(Input::Char(c));
        }
    }

    /// Brings back the text the field had when it was entered, selected.
    pub fn undo(&mut self) {
        self.text = self.original.clone();
        self.select_all();
    }

    /// Something to undo: the text is not the one the field had.
    pub fn can_undo(&self) -> bool {
        self.text != self.original
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

#[cfg(test)]
mod clipboard_tests {
    use super::*;

    #[test]
    fn a_pasted_text_replaces_the_selection() {
        let mut e = LineEdit::new("12.5", Kind::Number, 8);
        assert_eq!(e.selected_text(), Some("12.5"));
        e.paste("7.25");
        assert_eq!(e.text, "7.25");
        assert_eq!(e.selected_text(), None);
    }

    #[test]
    fn a_pasted_text_goes_in_at_the_cursor() {
        let mut e = LineEdit::new("abcd", Kind::Text, 20);
        e.key(Input::Home);
        e.key(Input::Right);
        e.key(Input::Right);
        e.paste("XY");
        assert_eq!(e.text, "abXYcd");
        assert_eq!(e.cursor, 4);
    }

    #[test]
    fn a_pasted_text_stops_at_the_length_of_the_field() {
        let mut e = LineEdit::new("", Kind::Text, 5);
        e.paste("abcdefgh");
        assert_eq!(e.text, "abcde");
        e.paste("zz");
        assert_eq!(e.text, "abcde");
    }

    #[test]
    fn a_number_field_keeps_only_what_a_number_has() {
        let mut e = LineEdit::new("", Kind::Number, 10);
        e.paste("  -12,5 mm\nsecond line");
        assert_eq!(e.text, "-12,5");
        let mut e = LineEdit::new("3", Kind::Number, 10);
        e.paste("abc");
        assert_eq!(
            e.text, "3",
            "nothing that fits: the field is left as it was"
        );
        assert!(e.fresh);
    }

    #[test]
    fn cut_gives_the_selection_and_empties_the_field() {
        let mut e = LineEdit::new("path", Kind::Text, 20);
        assert_eq!(e.cut().as_deref(), Some("path"));
        assert_eq!(e.text, "");
        assert_eq!(e.cut(), None);
    }

    #[test]
    fn select_all_and_undo() {
        let mut e = LineEdit::new("abc", Kind::Text, 20);
        e.key(Input::End);
        e.key(Input::Char('d'));
        assert!(e.can_undo());
        assert_eq!(e.selected_text(), None);
        e.select_all();
        assert_eq!(e.selected_text(), Some("abcd"));
        e.undo();
        assert_eq!(e.text, "abc");
        assert!(!e.can_undo());
    }
}
