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
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(name)
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
                    let named = match key {
                        Key::Enter => Some(Input::Enter),
                        Key::Escape => Some(Input::Esc),
                        Key::ArrowUp => Some(Input::Up),
                        Key::ArrowDown => Some(Input::Down),
                        Key::PageUp => Some(Input::PageUp),
                        Key::PageDown => Some(Input::PageDown),
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
                egui::Event::Text(t) => {
                    for c in t.chars().filter(|c| !c.is_control()) {
                        self.app.handle(Input::Char(c));
                    }
                }
                _ => {}
            }
        }
    }

    fn paint(&self, ui: &mut egui::Ui) -> Option<(usize, usize)> {
        let size = Vec2::new(CELL_W * COLS as f32, CELL_H * ROWS as f32);
        let (resp, painter) = ui.allocate_painter(size, Sense::click());
        let origin = resp.rect.min;
        let font = FontId::monospace(FONT_SIZE);
        let rect_of = |x: usize, y: usize| {
            Rect::from_min_size(
                origin + Vec2::new(x as f32 * CELL_W, y as f32 * CELL_H),
                Vec2::new(CELL_W, CELL_H),
            )
        };
        // Backgrounds first: a wide glyph must not be covered by the background of its second cell.
        for y in 0..ROWS {
            for x in 0..COLS {
                let cell = self.screen.get(x, y);
                let (bg, fg) = (color(cell.bg), color(cell.fg));
                let rect = rect_of(x, y);
                painter.rect_filled(rect, 0.0, bg);
                match cell.ch {
                    '█' => {
                        painter.rect_filled(rect, 0.0, fg);
                    }
                    '▓' => {
                        painter.rect_filled(rect, 0.0, mix(bg, fg, 0.75));
                    }
                    '▒' => {
                        painter.rect_filled(rect, 0.0, mix(bg, fg, 0.5));
                    }
                    '░' => {
                        painter.rect_filled(rect, 0.0, mix(bg, fg, 0.25));
                    }
                    _ => {}
                }
            }
        }
        for y in 0..ROWS {
            for x in 0..COLS {
                let cell = self.screen.get(x, y);
                if matches!(cell.ch, ' ' | CONTINUATION | '█' | '▓' | '▒' | '░') {
                    continue;
                }
                let rect = rect_of(x, y);
                painter.text(
                    Pos2::new(rect.min.x, rect.min.y + 1.0),
                    Align2::LEFT_TOP,
                    cell.ch.to_string(),
                    font.clone(),
                    color(cell.fg),
                );
            }
        }
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                return Some((
                    ((p.x - origin.x) / CELL_W) as usize,
                    ((p.y - origin.y) / CELL_H) as usize,
                ));
            }
        }
        None
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
            let _ = std::fs::write(settings_path(&self.settings), self.app.lang().code());
        }
        self.app.render(&mut self.screen);
        let clicked = egui::Frame::NONE
            .fill(Color32::BLACK)
            .show(ui, |ui| self.paint(ui))
            .inner;
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
    app: A,
    capture: Option<Capture>,
) -> eframe::Result {
    let size = Vec2::new(CELL_W * COLS as f32, CELL_H * ROWS as f32);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(size)
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
            }))
        }),
    )
}
