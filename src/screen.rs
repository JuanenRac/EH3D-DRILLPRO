//! 80x25 text-mode screen with the 16 CGA colours, as used by the original DOS program.

use crate::i18n::{cell_width, str_width};

/// Marks the right half of a double-width character.
pub const CONTINUATION: char = '\u{0}';

pub const COLS: usize = 80;
pub const ROWS: usize = 25;

pub const BLACK: u8 = 0;
pub const BLUE: u8 = 1;
pub const GREEN: u8 = 2;
pub const CYAN: u8 = 3;
pub const RED: u8 = 4;
pub const MAGENTA: u8 = 5;
pub const BROWN: u8 = 6;
pub const LIGHT_GRAY: u8 = 7;
pub const DARK_GRAY: u8 = 8;
pub const LIGHT_BLUE: u8 = 9;
pub const LIGHT_GREEN: u8 = 10;
pub const LIGHT_CYAN: u8 = 11;
pub const LIGHT_RED: u8 = 12;
pub const YELLOW: u8 = 14;
pub const WHITE: u8 = 15;

/// Standard CGA/VGA palette as RGB.
pub const PALETTE: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0xAA],
    [0x00, 0xAA, 0x00],
    [0x00, 0xAA, 0xAA],
    [0xAA, 0x00, 0x00],
    [0xAA, 0x00, 0xAA],
    [0xAA, 0x55, 0x00],
    [0xAA, 0xAA, 0xAA],
    [0x55, 0x55, 0x55],
    [0x55, 0x55, 0xFF],
    [0x55, 0xFF, 0x55],
    [0x55, 0xFF, 0xFF],
    [0xFF, 0x55, 0x55],
    [0xFF, 0x55, 0xFF],
    [0xFF, 0xFF, 0x55],
    [0xFF, 0xFF, 0xFF],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: u8,
    pub bg: u8,
}

pub struct Screen {
    cells: Vec<Cell>,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            cells: vec![
                Cell {
                    ch: ' ',
                    fg: LIGHT_GRAY,
                    bg: BLACK
                };
                COLS * ROWS
            ],
        }
    }
}

impl Screen {
    pub fn get(&self, x: usize, y: usize) -> Cell {
        self.cells[y * COLS + x]
    }

    pub fn put(&mut self, x: usize, y: usize, ch: char, fg: u8, bg: u8) {
        if x < COLS && y < ROWS {
            self.cells[y * COLS + x] = Cell { ch, fg, bg };
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fill(&mut self, x: usize, y: usize, w: usize, h: usize, ch: char, fg: u8, bg: u8) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.put(xx, yy, ch, fg, bg);
            }
        }
    }

    pub fn text(&mut self, x: usize, y: usize, s: &str, fg: u8, bg: u8) {
        let mut col = x;
        for ch in s.chars() {
            self.put_wide(col, y, ch, fg, bg);
            col += cell_width(ch);
        }
    }

    /// Puts a character; double-width characters also claim the next cell.
    pub fn put_wide(&mut self, x: usize, y: usize, ch: char, fg: u8, bg: u8) {
        if cell_width(ch) == 2 && x + 1 >= COLS {
            self.put(x, y, ' ', fg, bg);
            return;
        }
        self.put(x, y, ch, fg, bg);
        if cell_width(ch) == 2 {
            self.put(x + 1, y, CONTINUATION, fg, bg);
        }
    }

    /// Text where `~X~` marks the hot key, drawn in `hot` colour. Returns the width drawn.
    pub fn hot_text(&mut self, x: usize, y: usize, s: &str, fg: u8, hot: u8, bg: u8) -> usize {
        let (mut col, mut on) = (x, false);
        for ch in s.chars() {
            if ch == '~' {
                on = !on;
                continue;
            }
            self.put_wide(col, y, ch, if on { hot } else { fg }, bg);
            col += cell_width(ch);
        }
        col - x
    }

    pub fn text_centered(&mut self, y: usize, x0: usize, w: usize, s: &str, fg: u8, bg: u8) {
        let n = str_width(s);
        self.text(x0 + w.saturating_sub(n) / 2, y, s, fg, bg);
    }

    /// Double-line box, as used by Turbo Vision windows.
    pub fn frame(&mut self, x: usize, y: usize, w: usize, h: usize, fg: u8, bg: u8) {
        if w < 2 || h < 2 {
            return;
        }
        self.put(x, y, '╔', fg, bg);
        self.put(x + w - 1, y, '╗', fg, bg);
        self.put(x, y + h - 1, '╚', fg, bg);
        self.put(x + w - 1, y + h - 1, '╝', fg, bg);
        for xx in x + 1..x + w - 1 {
            self.put(xx, y, '═', fg, bg);
            self.put(xx, y + h - 1, '═', fg, bg);
        }
        for yy in y + 1..y + h - 1 {
            self.put(x, yy, '║', fg, bg);
            self.put(x + w - 1, yy, '║', fg, bg);
        }
    }

    /// Window with title, frame and the classic drop shadow.
    #[allow(clippy::too_many_arguments)]
    pub fn window(&mut self, x: usize, y: usize, w: usize, h: usize, title: &str, fg: u8, bg: u8) {
        self.fill(x, y, w, h, ' ', fg, bg);
        self.frame(x, y, w, h, WHITE, bg);
        if !title.is_empty() {
            let t = format!(" {title} ");
            self.text_centered(y, x, w, &t, fg, bg);
        }
        self.shadow(x, y, w, h);
    }

    /// Darkens the cells right of and below a rectangle.
    pub fn shadow(&mut self, x: usize, y: usize, w: usize, h: usize) {
        let dim = |s: &mut Screen, xx: usize, yy: usize| {
            if xx < COLS && yy < ROWS {
                let c = s.get(xx, yy);
                s.put(xx, yy, c.ch, DARK_GRAY, BLACK);
            }
        };
        for yy in y + 1..y + h + 1 {
            dim(self, x + w, yy);
            dim(self, x + w + 1, yy);
        }
        for xx in x + 2..x + w + 2 {
            dim(self, xx, y + h);
        }
    }

    /// The button under the cell `(c, r)`: the first and the end (not included) of the cells of its row that have the green of a button,
    /// or nothing when the cell is not one of them. The window lights it when the pointer is on it and sinks it when it is pressed.
    pub fn button_at(&self, c: usize, r: usize) -> Option<(usize, usize)> {
        if c >= COLS || r >= ROWS || self.get(c, r).bg != GREEN {
            return None;
        }
        let (mut x0, mut x1) = (c, c + 1);
        while x0 > 0 && self.get(x0 - 1, r).bg == GREEN {
            x0 -= 1;
        }
        while x1 < COLS && self.get(x1, r).bg == GREEN {
            x1 += 1;
        }
        Some((x0, x1))
    }

    /// The screen as plain text, one line per row (for tests and `--dump`).
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        for y in 0..ROWS {
            let line: String = (0..COLS)
                .map(|x| self.get(x, y).ch)
                .filter(|&c| c != CONTINUATION)
                .collect();
            s.push_str(line.trim_end());
            s.push('\n');
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classic::ui::button;

    #[test]
    fn a_button_is_found_from_any_of_its_cells_and_nothing_else_is() {
        let mut s = Screen::default();
        button(&mut s, 10, 5, 12, "~Start", false, true);
        for c in 10..22 {
            assert_eq!(s.button_at(c, 5), Some((10, 22)), "cell {c}");
        }
        // the shadow, the cells beside it and another row are not the button
        assert_eq!(s.button_at(22, 5), None);
        assert_eq!(s.button_at(12, 6), None);
        assert_eq!(s.button_at(9, 5), None);
        assert_eq!(s.button_at(500, 5), None);
        // a disabled one is grey: no light for it
        button(&mut s, 30, 8, 10, "Load", false, false);
        assert_eq!(s.button_at(33, 8), None);
    }
}
