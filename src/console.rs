//! The console front end: the 80x25 screen of a [`TextApp`] drawn in a terminal, with the keyboard and the mouse.
//!
//! It needs no window system, so it runs over SSH, on a text console and on old PCs; it draws the same classic screens
//! as the window. The terminal must be at least 80 columns by 25 rows; a larger one shows the screen in the middle.

use std::io::{self, Write};
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};

use crate::app::Input;
use crate::screen::{Screen, COLS, CONTINUATION, ROWS};
use crate::textapp::TextApp;

/// The terminal's own 16 colours in the order of the CGA palette the screen uses (CGA blue is ANSI 4, and so on).
const ANSI: [u8; 16] = [0, 4, 2, 6, 1, 5, 3, 7, 8, 12, 10, 14, 9, 13, 11, 15];

/// The screen as the text a terminal draws, with its top-left cell at `origin` (column, row; both from 1).
pub fn ansi_frame(screen: &Screen, origin: (u16, u16)) -> String {
    let mut out = String::with_capacity(COLS * ROWS * 6);
    let mut last: Option<(u8, u8)> = None;
    for y in 0..ROWS {
        out.push_str(&format!("\x1b[{};{}H", origin.1 as usize + y, origin.0));
        for x in 0..COLS {
            let cell = screen.get(x, y);
            if cell.ch == CONTINUATION {
                continue;
            }
            if last != Some((cell.fg, cell.bg)) {
                out.push_str(&format!(
                    "\x1b[38;5;{};48;5;{}m",
                    ANSI[(cell.fg & 15) as usize],
                    ANSI[(cell.bg & 15) as usize]
                ));
                last = Some((cell.fg, cell.bg));
            }
            out.push(cell.ch);
        }
    }
    out.push_str("\x1b[0m");
    out
}

/// The input a key stands for; `None` for keys the programs do not use.
pub fn map_key(k: KeyEvent) -> Option<Input> {
    if k.kind == KeyEventKind::Release {
        return None;
    }
    let alt = k.modifiers.contains(KeyModifiers::ALT);
    Some(match k.code {
        KeyCode::Char(c) if alt => Input::Alt(c.to_ascii_lowercase()),
        KeyCode::Char(c) => Input::Char(c),
        KeyCode::Enter => Input::Enter,
        KeyCode::Esc => Input::Esc,
        KeyCode::Up => Input::Up,
        KeyCode::Down => Input::Down,
        KeyCode::Left => Input::Left,
        KeyCode::Right => Input::Right,
        KeyCode::PageUp => Input::PageUp,
        KeyCode::PageDown => Input::PageDown,
        KeyCode::Tab => Input::Tab,
        KeyCode::BackTab => Input::BackTab,
        KeyCode::Backspace => Input::Backspace,
        KeyCode::Delete => Input::Delete,
        KeyCode::Home => Input::Home,
        KeyCode::End => Input::End,
        _ => return None,
    })
}

/// Where the screen's top-left cell goes in a terminal of `size` (columns, rows): the middle, from 1.
pub fn origin_for(size: (u16, u16)) -> (u16, u16) {
    (
        (size.0.saturating_sub(COLS as u16) / 2) + 1,
        (size.1.saturating_sub(ROWS as u16) / 2) + 1,
    )
}

/// The cell of the screen under a click at terminal cell (`col`, `row`), counted from 0.
pub fn cell_at(col: u16, row: u16, origin: (u16, u16)) -> Option<(usize, usize)> {
    let (x, y) = (
        col.checked_sub(origin.0 - 1)? as usize,
        row.checked_sub(origin.1 - 1)? as usize,
    );
    (x < COLS && y < ROWS).then_some((x, y))
}

/// Runs `app` in the terminal until it asks to quit or `Ctrl+C` is pressed. `settings` names the file that remembers
/// the language.
pub fn run<A: TextApp>(mut app: A, settings: &str) -> io::Result<()> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture, Hide)?;
    let result = event_loop(&mut app, settings, &mut out);
    let _ = execute!(out, Show, DisableMouseCapture, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    result
}

fn event_loop<A: TextApp>(app: &mut A, settings: &str, out: &mut io::Stdout) -> io::Result<()> {
    let mut screen = Screen::default();
    let mut last_frame = String::new();
    let mut lang = app.lang();
    loop {
        let size = crossterm::terminal::size()?;
        let origin = origin_for(size);
        let frame = if size.0 < COLS as u16 || size.1 < ROWS as u16 {
            format!(
                "\x1b[2J\x1b[1;1HThe terminal needs 80 columns by 25 rows (it has {}x{}).",
                size.0, size.1
            )
        } else {
            app.render(&mut screen);
            ansi_frame(&screen, origin)
        };
        if frame != last_frame {
            if last_frame.is_empty() || size.0 > COLS as u16 || size.1 > ROWS as u16 {
                // a larger terminal keeps the old picture around the screen: clear it when the picture changes place
                write!(out, "\x1b[0m\x1b[2J")?;
            }
            write!(out, "{frame}")?;
            out.flush()?;
            last_frame = frame;
        }
        let busy = app.tick();
        if app.wants_quit() {
            return Ok(());
        }
        if app.lang() != lang {
            lang = app.lang();
            let _ = std::fs::write(crate::cli::settings_path(settings), lang.code());
        }
        if event::poll(Duration::from_millis(if busy { 16 } else { 80 }))? {
            match event::read()? {
                Event::Key(k) => {
                    if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
                        return Ok(());
                    }
                    if let Some(i) = map_key(k) {
                        app.handle(i);
                    }
                }
                Event::Mouse(m) if m.kind == MouseEventKind::ScrollUp => app.scroll(-1),
                Event::Mouse(m) if m.kind == MouseEventKind::ScrollDown => app.scroll(1),
                Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => {
                    if let Some((x, y)) = cell_at(m.column, m.row, origin) {
                        app.click(x, y);
                    }
                }
                Event::Resize(..) => last_frame.clear(),
                _ => {}
            }
        }
    }
}
