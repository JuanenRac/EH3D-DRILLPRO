//! egui front end shared by DRILLPRO and ROUTEPRO: draws the 80x25 text-mode screen of a [`TextApp`].

use std::path::PathBuf;
use std::time::Duration;

use eframe::egui::{
    self, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Key, Pos2, Rect, Sense,
    Vec2,
};

use crate::app::Input;
use crate::i18n::Lang;
use crate::screen::{Screen, COLS, CONTINUATION, PALETTE, ROWS};
use crate::textapp::TextApp;

pub const CELL_W: f32 = 10.0;
pub const CELL_H: f32 = 20.0;
const FONT_SIZE: f32 = 16.0;

/// File where the chosen language is remembered between runs.
pub fn settings_path(name: &str) -> PathBuf {
    crate::cli::settings_path(name)
}

fn color(i: u8) -> Color32 {
    let [r, g, b] = PALETTE[(i & 15) as usize];
    Color32::from_rgb(r, g, b)
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 * (1.0 - t) + y as f32 * t) as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// Adds a system CJK font as a fallback so Chinese and Japanese texts can be drawn.
fn install_fonts(ctx: &egui::Context) {
    let candidates = [
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/YuGothM.ttc",
        "C:/Windows/Fonts/meiryo.ttc",
        "C:/Windows/Fonts/simsun.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = FontDefinitions::default();
            fonts.font_data.insert(
                "cjk".into(),
                std::sync::Arc::new(FontData::from_owned(bytes)),
            );
            for family in [FontFamily::Monospace, FontFamily::Proportional] {
                fonts.families.entry(family).or_default().push("cjk".into());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

/// Scripted run for screenshots: feeds `keys`, waits `wait` frames, saves the window to `file` and exits.
pub struct Capture {
    pub keys: Vec<Input>,
    pub file: PathBuf,
    pub wait: u32,
    /// The pointer of the picture: the cell it rests on and whether it is pressed (the window of a person has the real one).
    pub point: Option<(usize, usize, bool)>,
}

/// Parses `alt+v,enter,down,esc,a` into inputs (for [`Capture`]).
pub fn parse_keys(spec: &str) -> Vec<Input> {
    spec.split(',')
        .filter_map(|t| {
            let t = t.trim().to_ascii_lowercase();
            if let Some(c) = t.strip_prefix("alt+") {
                return c.chars().next().map(Input::Alt);
            }
            match t.as_str() {
                "enter" => Some(Input::Enter),
                "esc" => Some(Input::Esc),
                "tab" => Some(Input::Tab),
                "backtab" => Some(Input::BackTab),
                "up" => Some(Input::Up),
                "down" => Some(Input::Down),
                "pgup" => Some(Input::PageUp),
                "pgdn" => Some(Input::PageDown),
                other if other.chars().count() == 1 => other.chars().next().map(Input::Char),
                _ => None,
            }
        })
        .collect()
}

fn save_png(img: &egui::ColorImage, path: &std::path::Path) -> std::io::Result<()> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, img.size[0] as u32, img.size[1] as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(std::io::Error::other)?;
    let data: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
    w.write_image_data(&data).map_err(std::io::Error::other)
}

struct Gui<A: TextApp> {
    capture: Option<Capture>,
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    frame: u32,
    app: A,
    settings: String,
    screen: Screen,
    lang_saved: Lang,
    /// When the window opened: the cursor of a field blinks by it.
    started: std::time::Instant,
    /// The state of the window the last time it was set (a change of the application is applied and remembered).
    fullscreen_applied: bool,
}

/// Does the clipboard hold text? True when that cannot be known (so that Paste is not taken away for nothing).
fn clipboard_has_text() -> bool {
    match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
        Ok(t) => !t.is_empty(),
        Err(arboard::Error::ContentNotAvailable) => false,
        Err(_) => true,
    }
}

impl<A: TextApp> Gui<A> {
    fn translate_events(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            match ev {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    // Ctrl+A selects all the text of the field being edited (Ctrl+C, Ctrl+X and Ctrl+V arrive as their own events)
                    if modifiers.command && key == Key::A {
                        self.app.field_select_all();
                        continue;
                    }
                    if key == Key::F11 {
                        if let Some(on) = self.app.fullscreen() {
                            self.app.set_fullscreen(!on);
                        }
                        continue;
                    }
                    let named = match key {
                        Key::Enter => Some(Input::Enter),
                        Key::Escape => Some(Input::Esc),
                        Key::ArrowUp => Some(Input::Up),
                        Key::ArrowDown => Some(Input::Down),
                        Key::PageUp => Some(Input::PageUp),
                        Key::PageDown => Some(Input::PageDown),
                        Key::Tab if modifiers.shift => Some(Input::BackTab),
                        Key::Tab => Some(Input::Tab),
                        Key::Backspace => Some(Input::Backspace),
                        Key::Delete => Some(Input::Delete),
                        Key::ArrowLeft => Some(Input::Left),
                        Key::ArrowRight => Some(Input::Right),
                        Key::Home => Some(Input::Home),
                        Key::End => Some(Input::End),
                        _ => None,
                    };
                    if let Some(i) = named {
                        self.app.handle(i);
                    } else if modifiers.alt {
                        if let Some(c) = key
                            .name()
                            .chars()
                            .next()
                            .filter(|c| c.is_ascii_alphabetic())
                        {
                            self.app.handle(Input::Alt(c));
                        }
                    }
                }
                egui::Event::MouseWheel { delta, .. } => {
                    // a notch is three lines; the wheel scrolls only what has something to scroll
                    let lines = (delta.y.abs().ceil() as isize).clamp(1, 3);
                    self.app.scroll(if delta.y > 0.0 { -lines } else { lines });
                }
                egui::Event::Paste(t) => self.app.field_paste(&t),
                egui::Event::Copy => {
                    if let Some(t) = self.app.field_copy() {
                        ctx.copy_text(t);
                    }
                }
                egui::Event::Cut => {
                    if let Some(t) = self.app.field_cut() {
                        ctx.copy_text(t);
                    }
                }
                egui::Event::Text(t) => {
                    for c in t.chars().filter(|c| !c.is_control()) {
                        self.app.handle(Input::Char(c));
                    }
                }
                _ => {}
            }
        }
    }

    /// Draws the screen scaled to the room the window gives it (the 80x25 proportions are kept and the rest is black),
    /// and returns the cell that was clicked.
    fn paint(&self, ui: &mut egui::Ui) -> (Option<(usize, usize)>, egui::Response) {
        let avail = ui.available_size();
        let base = Vec2::new(CELL_W * COLS as f32, CELL_H * ROWS as f32);
        let scale = (avail.x / base.x).min(avail.y / base.y).max(0.4);
        let size = base * scale;
        let (resp, painter) = ui.allocate_painter(avail.max(size), Sense::click());
        // the screen sits in the middle of the room
        let origin = resp.rect.min + (resp.rect.size() - size) * 0.5;
        let (cw, ch) = (CELL_W * scale, CELL_H * scale);
        let font = FontId::monospace(FONT_SIZE * scale);
        // the button under the pointer: it lights up, and it sinks (loses its shadow and moves a little) while it is pressed
        let fake = self.capture.as_ref().and_then(|c| c.point);
        let down = fake.map_or_else(|| ui.input(|i| i.pointer.primary_down()), |(_, _, d)| d);
        let at: Option<(usize, usize)> = match fake {
            Some((c, r, _)) => Some((c, r)),
            None => resp.hover_pos().and_then(|p| {
                let (dx, dy) = (p.x - origin.x, p.y - origin.y);
                (dx >= 0.0 && dy >= 0.0).then(|| ((dx / cw) as usize, (dy / ch) as usize))
            }),
        };
        let hot: Option<(usize, usize, usize)> =
            at.and_then(|(c, r)| self.screen.button_at(c, r).map(|(x0, x1)| (r, x0, x1)));
        let in_hot =
            |x: usize, y: usize| hot.is_some_and(|(r, x0, x1)| y == r && x >= x0 && x < x1);
        let in_shadow = |x: usize, y: usize| {
            down && hot
                .is_some_and(|(r, x0, x1)| (y == r && x == x1) || (y == r + 1 && x > x0 && x <= x1))
        };
        let cell_at = |x: usize, y: usize| {
            if in_shadow(x, y) {
                crate::screen::Cell {
                    ch: ' ',
                    fg: crate::classic::ui::GREY,
                    bg: crate::classic::ui::GREY,
                }
            } else {
                self.screen.get(x, y)
            }
        };
        let rect_of = |x: usize, y: usize| {
            Rect::from_min_size(
                origin + Vec2::new(x as f32 * cw, y as f32 * ch),
                Vec2::new(cw, ch),
            )
        };
        // Backgrounds first: a wide glyph must not be covered by the background of its second cell.
        for y in 0..ROWS {
            for x in 0..COLS {
                let cell = cell_at(x, y);
                let (mut bg, fg) = (color(cell.bg), color(cell.fg));
                if in_hot(x, y) {
                    bg = if down {
                        mix(bg, Color32::BLACK, 0.35)
                    } else {
                        mix(bg, Color32::WHITE, 0.30)
                    };
                }
                let rect = rect_of(x, y);
                painter.rect_filled(rect, 0.0, bg);
                match cell.ch {
                    '\u{2588}' => {
                        painter.rect_filled(rect, 0.0, fg);
                    }
                    '\u{2593}' => {
                        painter.rect_filled(rect, 0.0, mix(bg, fg, 0.75));
                    }
                    '\u{2592}' => {
                        painter.rect_filled(rect, 0.0, mix(bg, fg, 0.5));
                    }
                    '\u{2591}' => {
                        painter.rect_filled(rect, 0.0, mix(bg, fg, 0.25));
                    }
                    _ => {}
                }
            }
        }
        for y in 0..ROWS {
            for x in 0..COLS {
                let cell = cell_at(x, y);
                if matches!(
                    cell.ch,
                    ' ' | CONTINUATION | '\u{2588}' | '\u{2593}' | '\u{2592}' | '\u{2591}'
                ) {
                    continue;
                }
                let rect = rect_of(x, y);
                // a pressed button moves its letters a little, as if it went down
                let sink = if down && in_hot(x, y) {
                    1.5 * scale
                } else {
                    0.0
                };
                painter.text(
                    Pos2::new(rect.min.x + sink, rect.min.y + scale + sink),
                    Align2::LEFT_TOP,
                    cell.ch.to_string(),
                    font.clone(),
                    color(cell.fg),
                );
            }
        }
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                let (dx, dy) = (p.x - origin.x, p.y - origin.y);
                if dx >= 0.0 && dy >= 0.0 {
                    let (c, r) = ((dx / cw) as usize, (dy / ch) as usize);
                    if c < COLS && r < ROWS {
                        return (Some((c, r)), resp);
                    }
                }
            }
        }
        (None, resp)
    }

    /// The menu of the right button over the field being edited: Cut, Copy, Paste, Select all and Undo.
    fn field_menu(&mut self, ctx: &egui::Context, resp: &egui::Response) {
        let clip_id = egui::Id::unique("field-clipboard-has-text");
        if resp.secondary_clicked() {
            let has = clipboard_has_text();
            ctx.data_mut(|d| d.insert_temp(clip_id, has));
        }
        let Some(f) = self.app.field_state() else {
            return;
        };
        let has_clip = ctx.data(|d| d.get_temp::<bool>(clip_id)).unwrap_or(true);
        let lang = self.app.lang();
        let word = |t| crate::texts::tr(lang, t);
        use crate::texts::Text;
        let app = &mut self.app;
        resp.context_menu(|ui| {
            if ui
                .add_enabled(f.selected, egui::Button::new(word(Text::ClipCut)))
                .clicked()
            {
                if let Some(t) = app.field_cut() {
                    ctx.copy_text(t);
                }
                ui.close();
            }
            if ui
                .add_enabled(f.selected, egui::Button::new(word(Text::ClipCopy)))
                .clicked()
            {
                if let Some(t) = app.field_copy() {
                    ctx.copy_text(t);
                }
                ui.close();
            }
            if ui
                .add_enabled(has_clip, egui::Button::new(word(Text::ClipPaste)))
                .clicked()
            {
                // the window asks the system for the clipboard and the text comes back as a paste
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                ui.close();
            }
            ui.separator();
            if ui
                .add_enabled(f.has_text, egui::Button::new(word(Text::ClipSelectAll)))
                .clicked()
            {
                app.field_select_all();
                ui.close();
            }
            if ui
                .add_enabled(f.can_undo, egui::Button::new(word(Text::ClipUndo)))
                .clicked()
            {
                app.field_undo();
                ui.close();
            }
        });
    }
}

impl<A: TextApp> eframe::App for Gui<A> {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.frame += 1;
        if let Some(cap) = &mut self.capture {
            if self.frame == 3 {
                for k in std::mem::take(&mut cap.keys) {
                    self.app.handle(k);
                }
            }
            if self.frame == 3 + cap.wait {
                let file = cap.file.clone();
                let done = self.done.clone();
                let c2 = ctx.clone();
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                    egui::ScreenshotCallback::new(move |img| {
                        if let Err(e) = save_png(&img, &file) {
                            eprintln!("cannot save {}: {e}", file.display());
                        }
                        done.store(true, std::sync::atomic::Ordering::SeqCst);
                        c2.request_repaint();
                    }),
                ));
            }
            if self.done.load(std::sync::atomic::Ordering::SeqCst) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else {
            self.translate_events(&ctx);
        }
        let busy = self.app.tick();
        if self.app.wants_quit() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.app.lang() != self.lang_saved {
            self.lang_saved = self.app.lang();
            crate::cli::save_prefs(&self.settings, self.app.lang(), self.fullscreen_applied);
        }
        // the cursor of a field being edited blinks
        crate::classic::ui::set_cursor_on(crate::classic::ui::cursor_phase(
            self.started.elapsed().as_millis(),
        ));
        self.app.render(&mut self.screen);
        let (clicked, resp) = egui::Frame::NONE
            .fill(Color32::BLACK)
            .show(ui, |ui| self.paint(ui))
            .inner;
        if self.capture.is_none() {
            self.field_menu(&ctx, &resp);
            // the window follows the setting of the application (F11 and the Parameter window change it) and remembers it
            if let Some(on) = self.app.fullscreen() {
                if on != self.fullscreen_applied {
                    self.fullscreen_applied = on;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(on));
                    if !on {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                    }
                    crate::cli::save_prefs(&self.settings, self.app.lang(), on);
                }
            }
        }
        if let Some((c, r)) = clicked {
            self.app.click(c, r);
        }
        ctx.request_repaint_after(Duration::from_millis(if busy { 16 } else { 100 }));
    }
}

/// Opens the window and runs `app` until it asks to quit. `settings` names the file that remembers the language.
pub fn run<A: TextApp + 'static>(title: &str, settings: &str, app: A) -> eframe::Result {
    run_capture(title, settings, app, None)
}

/// Like [`run`]; with a [`Capture`] it plays the key script, saves a PNG of the window and exits.
pub fn run_capture<A: TextApp + 'static>(
    title: &str,
    settings: &str,
    mut app: A,
    capture: Option<Capture>,
) -> eframe::Result {
    // A capture keeps the classic size so that pictures stay the same; a normal run starts roomier and can be
    // resized or maximized: the screen scales between 640x400 and any larger size (720p, 1080p and beyond).
    let size = if capture.is_some() {
        Vec2::new(CELL_W * COLS as f32, CELL_H * ROWS as f32)
    } else {
        Vec2::new(1040.0, 650.0)
    };
    // the window of a person starts as it was left (full screen or maximized); a capture always keeps its size
    let fullscreen = capture.is_none() && crate::cli::load_prefs(settings).fullscreen;
    if capture.is_none() {
        app.set_fullscreen(fullscreen);
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(size)
            .with_min_inner_size([640.0, 400.0])
            .with_resizable(true)
            .with_maximize_button(true)
            .with_maximized(capture.is_none() && !fullscreen)
            .with_fullscreen(fullscreen)
            .with_title(title),
        ..Default::default()
    };
    let lang = app.lang();
    let settings = settings.to_string();
    eframe::run_native(
        title,
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            Ok(Box::new(Gui {
                capture,
                done: Default::default(),
                frame: 0,
                app,
                settings,
                screen: Screen::default(),
                lang_saved: lang,
                started: std::time::Instant::now(),
                fullscreen_applied: fullscreen,
            }))
        }),
    )
}
