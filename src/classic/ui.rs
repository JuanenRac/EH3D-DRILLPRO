//! Widgets of the classic (MS-DOS style) look, drawn on the 80x25 [`Screen`].
//!
//! The proportions and colours follow the original programs: a grey desktop with a blue dither, grey
//! windows with a white double frame and a drop shadow, blue input fields, green buttons with a yellow
//! hot letter, cyan lists. Everything is drawn with the cells of [`Screen`], so tests can read it back.

use crate::i18n::{cell_width, str_width};
use crate::screen::*;

/// Window and desktop grey.
pub const GREY: u8 = LIGHT_GRAY;
/// Text of labels.
pub const INK: u8 = BLACK;
/// Hot letters and highlighted list entries.
pub const HOT: u8 = YELLOW;
/// Label of the focused control.
pub const FOCUS: u8 = WHITE;
/// Disabled button text.
pub const DISABLED: u8 = DARK_GRAY;

/// The desktop: a blue dither on 23 rows and a plain grey strip under it.
pub fn desktop(s: &mut Screen) {
    s.fill(0, 0, COLS, ROWS - 2, '\u{2591}', BLUE, GREY);
    s.fill(0, ROWS - 2, COLS, 2, ' ', GREY, GREY);
}

/// Grey window with a white double frame and, when `title_on_frame`, the title written into the top line.
/// The drop shadow is two cells wide on the right and one row high below.
pub fn window(
    s: &mut Screen,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    title: &str,
    title_on_frame: bool,
) {
    s.fill(x, y, w, h, ' ', GREY, GREY);
    s.frame(x, y, w, h, WHITE, GREY);
    if title_on_frame && !title.is_empty() {
        let t = format!(" {title} ");
        s.text_centered(y, x, w, &t, WHITE, GREY);
    }
    s.shadow(x, y, w, h);
    // the grey strip under the desktop is never shadowed
    s.fill(0, ROWS - 2, COLS, 2, ' ', GREY, GREY);
}

/// A label in the window ink.
pub fn label(s: &mut Screen, x: usize, y: usize, text: &str) -> usize {
    s.hot_text(x, y, text, INK, HOT, GREY)
}

/// A label whose first marked letter is the hot key; `focused` turns the body white.
pub fn caption(s: &mut Screen, x: usize, y: usize, text: &str, focused: bool) -> usize {
    s.hot_text(x, y, text, if focused { FOCUS } else { INK }, HOT, GREY)
}

/// Input field: one blue cell of margin on each side of the text; `width` is the text width.
/// While editing the field turns green and shows a cursor in the first cell.
pub fn field(s: &mut Screen, x: usize, y: usize, width: usize, text: &str, editing: bool) {
    let bg = if editing { GREEN } else { BLUE };
    let fg = if editing { BLACK } else { WHITE };
    s.fill(x, y, width + 2, 1, ' ', bg, bg);
    let shown: String = text.chars().take(width).collect();
    s.text(x + 1, y, &shown, fg, bg);
}

/// Right-aligned field text (numbers).
pub fn number_field(s: &mut Screen, x: usize, y: usize, width: usize, text: &str, editing: bool) {
    let n = str_width(text);
    let pad = width.saturating_sub(n);
    field(
        s,
        x,
        y,
        width,
        &format!("{}{text}", " ".repeat(pad)),
        editing,
    );
}

/// Green button of `width` cells with its drop shadow (one cell right, one row below).
/// `label` marks the hot letter with `~`. A disabled button shows only grey text.
pub fn button(
    s: &mut Screen,
    x: usize,
    y: usize,
    width: usize,
    label: &str,
    focused: bool,
    enabled: bool,
) {
    let plain: String = label.chars().filter(|&c| c != '~').collect();
    let n = str_width(&plain);
    let left = width.saturating_sub(n) / 2;
    if enabled {
        s.fill(x, y, width, 1, ' ', GREEN, GREEN);
        s.hot_text(
            x + left,
            y,
            label,
            if focused { FOCUS } else { INK },
            HOT,
            GREEN,
        );
    } else {
        s.text(x + left, y, &plain, DISABLED, GREY);
    }
    // right and bottom shadow: half cells, as the original draws them
    s.put(x + width, y, '\u{2580}', GREY, BLACK);
    for xx in x + 1..x + width + 1 {
        s.put(xx, y + 1, '\u{2580}', BLACK, GREY);
    }
}

/// A group of radio buttons on a cyan box; `rows` are the already translated labels.
pub fn radio_group(s: &mut Screen, x: usize, y: usize, w: usize, rows: &[String], selected: usize) {
    s.fill(x, y, w, rows.len(), ' ', CYAN, CYAN);
    for (i, r) in rows.iter().enumerate() {
        let mark = if i == selected { "(\u{2022})" } else { "( )" };
        s.text(x + 1, y + i, mark, BLACK, CYAN);
        s.text(x + 5, y + i, r, BLACK, CYAN);
    }
}

/// Dark-red strip with yellow text ("Press ESC to stop").
pub fn alert_strip(s: &mut Screen, x: usize, y: usize, w: usize, text: &str) {
    s.fill(x, y, w, 1, ' ', YELLOW, RED);
    s.text_centered(y, x, w, text, YELLOW, RED);
}

/// Width of a string in cells (re-exported for the dialogs).
pub fn width(text: &str) -> usize {
    text.chars().map(cell_width).sum()
}
