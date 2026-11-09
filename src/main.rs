//! RAM Altman: Memory Is All You Need — a tiny always-on-top desktop widget that shows RAM usage
//! and a cartoon Sam Altman whose mood depends on how much RAM you're using.
//! Unused RAM makes him furious. Only ≥90% utilization makes him happy.
//!
//! Run:            cargo run --release
//! Preview moods:  cargo run --release -- --demo      (sweeps 0→100%)
//!                 cargo run --release -- --pct 10     (fake a fixed usage)
//! Bigger widget:  cargo run --release -- --zoom 1.5
//!
//! Custom pictures: put `level0.png` … `level6.png` (or .jpg) in `assets/faces/`
//! (or `~/.config/ram-altman/faces`, `%APPDATA%\ram-altman\faces`, or `$RAM_ALTMAN_FACES_DIR`).
//! Any level without an image falls back
//! to the built-in cartoon.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Color32, CornerRadius, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
    TextureHandle, TextureOptions, UiBuilder, Vec2, epaint::CubicBezierShape,
    epaint::QuadraticBezierShape, pos2, vec2,
};
use sysinfo::System;

const LEVELS: usize = 7;

/// Mood levels, from furious (0) to happy (6), keyed on % of RAM in use.
const MOODS: [(&str, &str, Color32); LEVELS] = [
    ("FURIOUS", "Idle RAM?! Send it to a data center!", Color32::from_rgb(255, 70, 60)),
    ("VERY ANGRY", "That RAM belongs in a data center.", Color32::from_rgb(255, 105, 70)),
    ("ANGRY", "My data centers need this RAM more.", Color32::from_rgb(255, 140, 70)),
    ("ANNOYED", "Half idle. A GPU cluster wants it.", Color32::from_rgb(255, 180, 80)),
    ("GRUMPY", "Still some RAM to spare for servers.", Color32::from_rgb(240, 210, 90)),
    ("ALMOST THERE", "Almost too busy to give it away…", Color32::from_rgb(200, 225, 100)),
    ("HAPPY", "Fine, you can keep your RAM.", Color32::from_rgb(110, 230, 120)),
];

fn mood_level(used_pct: f32) -> usize {
    match used_pct {
        u if u < 15.0 => 0,
        u if u < 25.0 => 1,
        u if u < 40.0 => 2,
        u if u < 55.0 => 3,
        u if u < 70.0 => 4,
        u if u < 90.0 => 5,
        _ => 6,
    }
}

struct RamWidget {
    sys: System,
    last_refresh: Instant,
    total: u64,
    available: u64,
    faces: [Option<TextureHandle>; LEVELS],
    demo_start: Option<Instant>,
    /// `--pct N`: pretend N% of RAM is used (for previewing a single mood).
    fixed_pct: Option<f32>,
}

impl RamWidget {
    fn new(cc: &eframe::CreationContext<'_>, demo: bool, fixed_pct: Option<f32>) -> Self {
        let mut sys = System::new().expect("could not read system memory info");
        sys.refresh_memory();
        Self {
            total: sys.total_memory(),
            available: sys.available_memory(),
            sys,
            last_refresh: Instant::now(),
            faces: load_faces(&cc.egui_ctx),
            demo_start: demo.then(Instant::now),
            fixed_pct,
        }
    }

    fn used_pct(&self) -> f32 {
        if let Some(pct) = self.fixed_pct {
            return pct.clamp(0.0, 100.0);
        }
        if let Some(start) = self.demo_start {
            // Sweep 0 → 100% → 0 every 20 s so every expression can be previewed.
            let t = (start.elapsed().as_secs_f32() / 10.0) % 2.0;
            return if t < 1.0 { t * 100.0 } else { (2.0 - t) * 100.0 };
        }
        if self.total == 0 {
            return 0.0;
        }
        (self.total.saturating_sub(self.available)) as f32 / self.total as f32 * 100.0
    }

    /// Available bytes to display. In `--demo`/`--pct` mode this is derived from the
    /// simulated percentage so the numbers always agree with the bar and the face.
    fn shown_available(&self, used_pct: f32) -> u64 {
        if self.fixed_pct.is_some() || self.demo_start.is_some() {
            (self.total as f64 * (1.0 - used_pct as f64 / 100.0)).round() as u64
        } else {
            self.available
        }
    }
}

impl eframe::App for RamWidget {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4] // fully transparent window; we paint our own rounded card
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.last_refresh.elapsed() >= Duration::from_secs(1) {
            self.sys.refresh_memory();
            self.total = self.sys.total_memory();
            self.available = self.sys.available_memory();
            self.last_refresh = Instant::now();
        }

        let used_pct = self.used_pct();
        let level = mood_level(used_pct);
        let (mood_name, mood_line, mood_color) = MOODS[level];
        // 1.0 = furious, 0.0 = happy; animated so the face morphs smoothly.
        let target_anger = 1.0 - level as f32 / (LEVELS - 1) as f32;
        let anger = ctx.animate_value_with_time(egui::Id::unique("anger"), target_anger, 0.6);
        let time = ctx.input(|i| i.time) as f32;

        // ---- Card background + window dragging ----
        let card = ui.max_rect();
        let drag = ui.interact(card, egui::Id::unique("drag"), Sense::click_and_drag());
        if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        ui.painter()
            .rect_filled(card, 18.0, Color32::from_rgba_unmultiplied(22, 24, 30, 235));
        ui.painter().rect_stroke(
            card.shrink(1.0),
            18.0,
            Stroke::new(1.5, mood_color.gamma_multiply(0.6)),
            StrokeKind::Inside,
        );

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        ui.scope_builder(UiBuilder::new().max_rect(card.shrink2(vec2(14.0, 10.0))), |ui| {
            // Header row
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("RAM Altman")
                        .strong()
                        .size(14.0)
                        .color(Color32::from_gray(225)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = ui.add(
                        egui::Button::new(egui::RichText::new("×").size(18.0))
                            .frame(false),
                    );
                    if close.on_hover_text("Close (Esc)").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if self.demo_start.is_some() || self.fixed_pct.is_some() {
                        ui.label(egui::RichText::new("DEMO").size(10.0).color(Color32::YELLOW));
                    }
                });
            });
            ui.label(
                egui::RichText::new("Memory Is All You Need")
                    .italics()
                    .size(10.5)
                    .color(Color32::from_gray(150)),
            );
            ui.add_space(2.0);

            // Face
            let face_size = ui.available_width();
            let (face_rect, _) = ui.allocate_exact_size(Vec2::splat(face_size), Sense::hover());
            let painter = ui.painter_at(face_rect);
            painter.rect_filled(face_rect, 14.0, Color32::from_rgb(40, 44, 54));
            match &self.faces[level] {
                Some(tex) => draw_photo(&painter, face_rect, tex),
                None => draw_sam(&painter, face_rect, anger, time),
            }

            ui.add_space(6.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(mood_name).strong().size(18.0).color(mood_color));
                ui.label(egui::RichText::new(mood_line).italics().size(12.0).color(Color32::from_gray(190)));
            });
            ui.add_space(6.0);

            ui.add(
                egui::ProgressBar::new(used_pct / 100.0)
                    .fill(mood_color.gamma_multiply(0.85))
                    .corner_radius(CornerRadius::same(6))
                    .text(
                        egui::RichText::new(format!("{used_pct:.0}% used"))
                            .color(Color32::WHITE)
                            .strong(),
                    ),
            );
            ui.add_space(4.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Available / Total")
                        .size(10.5)
                        .color(Color32::from_gray(150)),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        fmt_gib(self.shown_available(used_pct)),
                        fmt_gib(self.total)
                    ))
                    .monospace()
                    .strong()
                    .size(14.0)
                    .color(Color32::from_gray(235)),
                );
            });
        });

        drag.context_menu(|ui| {
            if ui.button("Close").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        // Animate smoothly when there's motion (steam, shaking, sparkles, morphing);
        // otherwise only wake up once a second to re-read memory.
        let animating = (anger - target_anger).abs() > 0.001 || anger > 0.6 || anger < 0.1;
        if animating || self.demo_start.is_some() {
            ctx.request_repaint_after(Duration::from_millis(33));
        } else {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
}

fn fmt_gib(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

// ---------------------------------------------------------------------------
// Optional user-supplied pictures
// ---------------------------------------------------------------------------

/// Where custom face pictures are looked up, in order:
/// `$RAM_ALTMAN_FACES_DIR`, the per-user config folder
/// (`~/.config/ram-altman/faces` or `%APPDATA%\ram-altman\faces`),
/// `assets/faces` next to the executable, and `./assets/faces`.
fn faces_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("RAM_ALTMAN_FACES_DIR") {
        return Some(PathBuf::from(dir));
    }
    let mut candidates = Vec::new();
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    if let Some(cfg) = config_home {
        candidates.push(cfg.join("ram-altman").join("faces"));
    }
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        candidates.push(exe_dir.join("assets").join("faces"));
        candidates.push(exe_dir.join("../../assets/faces")); // target/release → project root
    }
    candidates.push(PathBuf::from("assets/faces"));
    candidates.into_iter().find(|p| p.is_dir())
}

fn load_faces(ctx: &egui::Context) -> [Option<TextureHandle>; LEVELS] {
    let mut faces: [Option<TextureHandle>; LEVELS] = Default::default();
    let Some(dir) = faces_dir() else { return faces };
    for (level, slot) in faces.iter_mut().enumerate() {
        let path = ["png", "jpg", "jpeg"]
            .iter()
            .map(|ext| dir.join(format!("level{level}.{ext}")))
            .find(|p| p.is_file());
        let Some(path) = path else { continue };
        match image::open(&path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                *slot = Some(ctx.load_texture(format!("face{level}"), color, TextureOptions::LINEAR));
            }
            Err(e) => eprintln!("could not load {}: {e}", path.display()),
        }
    }
    faces
}

/// Draw a picture "cover"-fitted into `rect`.
fn draw_photo(painter: &egui::Painter, rect: Rect, tex: &TextureHandle) {
    let [w, h] = tex.size();
    let (img_aspect, box_aspect) = (w as f32 / h as f32, rect.width() / rect.height());
    let uv = if img_aspect > box_aspect {
        let f = box_aspect / img_aspect;
        Rect::from_min_max(pos2((1.0 - f) / 2.0, 0.0), pos2((1.0 + f) / 2.0, 1.0))
    } else {
        let f = img_aspect / box_aspect;
        Rect::from_min_max(pos2(0.0, (1.0 - f) / 2.0), pos2(1.0, (1.0 + f) / 2.0))
    };
    painter.image(tex.id(), rect.shrink(2.0), uv, Color32::WHITE);
}

// ---------------------------------------------------------------------------
// Built-in cartoon: same face every time, only the expression changes.
// `anger` is continuous: 1.0 = furious … 0.0 = happy.
// ---------------------------------------------------------------------------

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| lerp(x as f32, y as f32, t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

fn draw_sam(painter: &egui::Painter, rect: Rect, anger: f32, time: f32) {
    let s = rect.height() * 0.42;
    let shake = if anger > 0.85 {
        vec2((time * 47.0).sin(), (time * 61.0).cos()) * s * 0.012 * (anger - 0.85) / 0.15
    } else {
        Vec2::ZERO
    };
    let c = rect.center() + vec2(0.0, s * 0.08) + shake;
    let p = |x: f32, y: f32| -> Pos2 { c + vec2(x * s, y * s) };

    let skin_base = Color32::from_rgb(243, 205, 178);
    let skin = lerp_color(skin_base, Color32::from_rgb(228, 92, 72), anger.powf(1.6));
    let skin_shade = lerp_color(skin, Color32::BLACK, 0.12);
    let hair = Color32::from_rgb(78, 58, 44);
    let ink = Color32::from_rgb(40, 30, 30);
    let ink_stroke = Stroke::new(s * 0.035, ink);

    // Sweater + neck
    painter.add(Shape::ellipse_filled(p(0.0, 1.22), vec2(0.95 * s, 0.5 * s), Color32::from_rgb(92, 96, 108)));
    painter.add(Shape::rect_filled(
        Rect::from_center_size(p(0.0, 0.72), vec2(0.36 * s, 0.3 * s)),
        0.0,
        skin_shade,
    ));
    painter.add(Shape::ellipse_filled(p(0.0, 0.84), vec2(0.2 * s, 0.07 * s), skin_shade));

    // Back hair, ears, head
    painter.add(Shape::ellipse_filled(p(0.0, -0.42), vec2(0.62 * s, 0.42 * s), hair));
    for side in [-1.0, 1.0] {
        painter.add(Shape::ellipse_filled(p(side * 0.57, 0.04), vec2(0.1 * s, 0.14 * s), skin_shade));
    }
    painter.add(Shape::ellipse_filled(p(0.0, 0.04), vec2(0.56 * s, 0.68 * s), skin));

    // Short swept fringe
    painter.add(CubicBezierShape::from_points_stroke(
        [p(-0.57, -0.22), p(-0.45, -0.78), p(0.35, -0.8), p(0.58, -0.25)],
        true,
        hair,
        Stroke::NONE,
    ));
    painter.add(QuadraticBezierShape::from_points_stroke(
        [p(-0.57, -0.24), p(0.0, -0.36), p(0.58, -0.27)],
        true,
        hair,
        Stroke::NONE,
    ));

    // Cheeks: pink when happy
    let blush = (1.0 - anger * 3.5).clamp(0.0, 1.0);
    if blush > 0.0 {
        for side in [-1.0, 1.0] {
            painter.add(Shape::ellipse_filled(
                p(side * 0.33, 0.2),
                vec2(0.1 * s, 0.06 * s),
                Color32::from_rgba_unmultiplied(255, 120, 130, (110.0 * blush) as u8),
            ));
        }
    }

    // Eyes (big and round; lids lower on the inner side as anger rises)
    let ey = -0.04;
    for side in [-1.0f32, 1.0] {
        let ex = side * 0.22;
        if anger < 0.08 {
            // ^ ^  happy closed eyes
            painter.add(QuadraticBezierShape::from_points_stroke(
                [p(ex - 0.11, ey + 0.03), p(ex, ey - 0.12), p(ex + 0.11, ey + 0.03)],
                false,
                Color32::TRANSPARENT,
                Stroke::new(s * 0.045, ink),
            ));
            continue;
        }
        painter.add(Shape::ellipse_filled(p(ex, ey), vec2(0.12 * s, 0.11 * s), Color32::WHITE));
        let pupil_r = lerp(0.062, 0.035, anger) * s;
        painter.add(Shape::circle_filled(p(ex + side * -0.01, ey + 0.01), pupil_r, ink));
        painter.add(Shape::circle_filled(p(ex + 0.02, ey - 0.02), pupil_r * 0.3, Color32::WHITE));

        // Eyelid: skin-coloured wedge, inner corner (toward nose) drops with anger.
        let inner = -side; // direction toward the nose
        let lid_inner = ey - 0.12 + 0.13 * anger;
        let lid_outer = ey - 0.12 + 0.035 * anger;
        let pts = vec![
            p(ex - inner * 0.15, ey - 0.2),
            p(ex + inner * 0.15, ey - 0.2),
            p(ex + inner * 0.15, lid_inner),
            p(ex - inner * 0.15, lid_outer),
        ];
        painter.add(Shape::convex_polygon(pts, skin, Stroke::NONE));
        painter.line_segment([p(ex + inner * 0.13, lid_inner), p(ex - inner * 0.13, lid_outer)], Stroke::new(s * 0.02, ink));
    }

    // Eyebrows: inner ends slam down with anger, arch up when happy.
    for side in [-1.0f32, 1.0] {
        let inner = -side;
        let bx = side * 0.22;
        let y_inner = ey - 0.22 + 0.15 * anger;
        let y_outer = ey - 0.24 - 0.04 * anger;
        let arch = (1.0 - anger) * 0.06;
        painter.add(QuadraticBezierShape::from_points_stroke(
            [
                p(bx - inner * 0.15, y_outer),
                p(bx, (y_inner + y_outer) / 2.0 - arch),
                p(bx + inner * 0.14, y_inner),
            ],
            false,
            Color32::TRANSPARENT,
            Stroke::new(s * lerp(0.04, 0.06, anger), hair),
        ));
    }

    // Nose
    painter.add(QuadraticBezierShape::from_points_stroke(
        [p(0.0, 0.03), p(0.06, 0.17), p(-0.02, 0.19)],
        false,
        Color32::TRANSPARENT,
        Stroke::new(s * 0.025, skin_shade.gamma_multiply(0.8)),
    ));

    // Mouth
    let my = 0.39;
    let teeth = Color32::from_rgb(250, 250, 245);
    let mouth_dark = Color32::from_rgb(90, 25, 30);
    if anger > 0.78 {
        // Shouting: open mouth, gets bigger the angrier he is
        let open = 0.07 + 0.08 * ((anger - 0.78) / 0.22).clamp(0.0, 1.0);
        let shout = (time * 9.0).sin() * 0.01 * anger;
        painter.add(Shape::ellipse_filled(p(0.0, my), vec2(0.17 * s, (open + shout) * s), mouth_dark));
        painter.add(Shape::rect_filled(
            Rect::from_center_size(p(0.0, my - open * 0.6), vec2(0.2 * s, 0.045 * s)),
            2.0,
            teeth,
        ));
        painter.add(Shape::ellipse_filled(p(0.0, my + open * 0.55), vec2(0.08 * s, 0.03 * s), Color32::from_rgb(200, 80, 90)));
    } else if anger < 0.2 {
        // Big grin
        let grin = lerp(0.2, 0.12, anger / 0.2);
        painter.add(QuadraticBezierShape::from_points_stroke(
            [p(-0.22, my - 0.04), p(0.0, my + grin * 1.6), p(0.22, my - 0.04)],
            true,
            mouth_dark,
            Stroke::new(s * 0.03, ink),
        ));
        painter.add(QuadraticBezierShape::from_points_stroke(
            [p(-0.19, my - 0.03), p(0.0, my + 0.05), p(0.19, my - 0.03)],
            true,
            teeth,
            Stroke::NONE,
        ));
    } else {
        // Frown → flat → slight smile
        let t = (anger - 0.2) / 0.58; // 0 (slight smile) .. 1 (deep frown)
        let curve = lerp(0.04, -0.12, t);
        painter.add(QuadraticBezierShape::from_points_stroke(
            [p(-0.17, my - curve * 0.3), p(0.0, my + curve), p(0.17, my - curve * 0.3)],
            false,
            Color32::TRANSPARENT,
            ink_stroke,
        ));
    }

    // --- Effects ---
    if anger > 0.6 {
        // Steam from the ears
        let k = ((anger - 0.6) / 0.4).clamp(0.0, 1.0);
        for side in [-1.0f32, 1.0] {
            for i in 0..4 {
                let phase = (time * 0.9 + i as f32 * 0.25 + if side > 0.0 { 0.12 } else { 0.0 }) % 1.0;
                let pos = p(side * (0.68 + phase * 0.25), -0.05 - phase * 0.65);
                let alpha = ((1.0 - phase) * 200.0 * k) as u8;
                painter.add(Shape::circle_filled(
                    pos,
                    s * (0.04 + phase * 0.09),
                    Color32::from_rgba_unmultiplied(235, 235, 240, alpha),
                ));
            }
        }
    }
    if anger > 0.9 {
        // Anger vein on forehead
        let v = p(0.3, -0.5);
        let r = s * 0.07;
        let red = Stroke::new(s * 0.03, Color32::from_rgb(200, 20, 30));
        for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let corner = v + vec2(dx * r, dy * r);
            painter.line_segment([corner, corner + vec2(-dx * r * 0.7, 0.0)], red);
            painter.line_segment([corner, corner + vec2(0.0, -dy * r * 0.7)], red);
        }
    }
    if anger < 0.1 {
        // Sparkles
        for (i, (x, y)) in [(-0.8, -0.6), (0.82, -0.45), (-0.7, 0.45), (0.75, 0.55)].iter().enumerate() {
            let tw = ((time * 3.0 + i as f32 * 1.7).sin() * 0.5 + 0.5) * 0.08 + 0.03;
            let center = p(*x, *y);
            let col = Color32::from_rgb(255, 225, 90);
            let st = Stroke::new(s * 0.02, col);
            painter.line_segment([center - vec2(tw * s, 0.0), center + vec2(tw * s, 0.0)], st);
            painter.line_segment([center - vec2(0.0, tw * s), center + vec2(0.0, tw * s)], st);
        }
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let demo = args.iter().any(|a| a == "--demo");
    let fixed_pct = args
        .iter()
        .position(|a| a == "--pct")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<f32>().ok());
    let zoom = args
        .iter()
        .position(|a| a == "--zoom")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(1.0)
        .clamp(0.5, 4.0);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RAM Altman: Memory Is All You Need")
            .with_inner_size([250.0 * zoom, 418.0 * zoom])
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_app_id("ram-altman")
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
                    .unwrap_or_default(),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "RAM Altman: Memory Is All You Need",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_zoom_factor(zoom);
            Ok(Box::new(RamWidget::new(cc, demo, fixed_pct)))
        }),
    )
}
