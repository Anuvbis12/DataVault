// theme.rs — Konstanta visual dan helper widget
// Seluruh warna, style setup, dan fungsi widget reusable
// dikumpulkan di sini. View layer mengimport dari modul ini.

use eframe::egui;
use egui::epaint::{Color32, FontId, FontFamily, Rounding, Stroke, Vec2};
use std::sync::atomic::{AtomicBool, Ordering};

pub static IS_LIGHT_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_light_mode(light: bool) {
    IS_LIGHT_MODE.store(light, Ordering::Relaxed);
}

pub fn is_light() -> bool {
    IS_LIGHT_MODE.load(Ordering::Relaxed)
}


// ── Palet Warna ───────────────────────────────────────────
// Background layers
pub fn bg_base() -> Color32 {
    if is_light() { Color32::from_rgb(250, 250, 250) } else { Color32::from_rgb(7, 9, 19) }
}       // #070913
pub fn bg_surface() -> Color32 {
    if is_light() { Color32::from_rgb(240, 240, 240) } else { Color32::from_rgb(13, 15, 30) }
}    // #0d0f1e
pub fn bg_card() -> Color32 {
    if is_light() { Color32::from_rgb(255, 255, 255) } else { Color32::from_rgb(20, 22, 45) }
}    // #14162d
pub fn bg_input() -> Color32 {
    if is_light() { Color32::from_rgb(235, 235, 235) } else { Color32::from_rgb(26, 28, 54) }
}    // #1a1c36

// Borders
pub fn border_default() -> Color32 {
    if is_light() { Color32::from_rgba_unmultiplied(200, 200, 200, 255) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 15) }
} // white alpha 15
pub fn border_subtle() -> Color32 {
    if is_light() { Color32::from_rgba_unmultiplied(230, 230, 230, 255) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 8) }
} // white alpha 8
pub fn border_hover() -> Color32 {
    if is_light() { Color32::from_rgba_unmultiplied(100, 100, 100, 255) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 30) }
} // white alpha 30
pub fn border_accent() -> Color32 {
    if is_light() { Color32::from_rgb(99, 102, 241) } else { Color32::from_rgb(129, 140, 248) }
} // Indigo border

// Text
pub fn text_primary() -> Color32 {
    if is_light() { Color32::from_rgb(10, 10, 10) } else { Color32::from_rgb(255, 255, 255) }
} // #ffffff
pub fn text_body() -> Color32 {
    if is_light() { Color32::from_rgb(40, 40, 40) } else { Color32::from_rgb(115, 121, 150) }
} // #737996
pub fn text_muted() -> Color32 {
    if is_light() { Color32::from_rgb(100, 100, 100) } else { Color32::from_rgb(71, 77, 102) }
} // #474d66
pub fn text_dimmed() -> Color32 {
    if is_light() { Color32::from_rgb(140, 140, 140) } else { Color32::from_rgb(71, 77, 102) }
}
pub fn text_faint() -> Color32 {
    if is_light() { Color32::from_rgb(180, 180, 180) } else { Color32::from_rgb(51, 51, 50) }
} // #333332

// Accents — Indigo (using old TEAL name to avoid breaking view.rs)
pub fn teal_strong() -> Color32 {
    if is_light() { Color32::from_rgb(99, 102, 241) } else { Color32::from_rgb(129, 140, 248) }
} // #818cf8
pub fn teal_dark() -> Color32 {
    if is_light() { Color32::from_rgb(79, 70, 229) } else { Color32::from_rgb(99, 102, 241) }
} // #6366f1
pub fn teal_light() -> Color32 {
    if is_light() { Color32::from_rgb(129, 140, 248) } else { Color32::from_rgb(165, 180, 252) }
} // #a5b4fc
pub fn teal_faint() -> Color32 {
    if is_light() { Color32::from_rgb(224, 231, 255) } else { Color32::from_rgba_unmultiplied(129, 140, 248, 25) }
}

// Accent palette helpers
pub fn accent_purple() -> Color32 { Color32::from_rgb(168, 85, 247) } // #a855f7
pub fn accent_mint() -> Color32 { Color32::from_rgb(16, 185, 129) } // #10b981
pub fn accent_sky() -> Color32 { Color32::from_rgb(6, 182, 212) } // #06b6d4
pub fn accent_peach() -> Color32 { Color32::from_rgb(251, 146, 60) } // #fb923c
pub fn accent_gold() -> Color32 { Color32::from_rgb(251, 191, 36) } // #fbbf24
pub fn accent_rose() -> Color32 { Color32::from_rgb(244, 63, 94) } // #f43f5e

// Translucent variations
pub fn accent_purple_a() -> Color32 { Color32::from_rgba_unmultiplied(168, 85, 247, 25) }
pub fn accent_purple_b() -> Color32 { Color32::from_rgba_unmultiplied(168, 85, 247, 60) }
pub fn accent_mint_a() -> Color32 { Color32::from_rgba_unmultiplied(16, 185, 129, 25) }
pub fn accent_sky_a() -> Color32 { Color32::from_rgba_unmultiplied(6, 182, 212, 25) }
pub fn accent_peach_a() -> Color32 { Color32::from_rgba_unmultiplied(251, 146, 60, 25) }
pub fn accent_gold_a() -> Color32 { Color32::from_rgba_unmultiplied(251, 191, 36, 25) }
pub fn accent_rose_a() -> Color32 { Color32::from_rgba_unmultiplied(244, 63, 94, 25) }

// Status
pub fn error_color() -> Color32 {
    if is_light() { Color32::from_rgb(220, 30, 60) } else { Color32::from_rgb(239, 68, 68) }
}  // #ef4444
pub fn warn_color() -> Color32 {
    if is_light() { Color32::from_rgb(210, 150, 10) } else { Color32::from_rgb(251, 191, 36) }
}  // #fbbf24
pub fn success_color() -> Color32 {
    if is_light() { Color32::from_rgb(30, 160, 80) } else { Color32::from_rgb(16, 185, 129) }
}  // #10b981

// File type badge (fill, border/fg)
pub const BADGE_PURPLE: (Color32, Color32) = (Color32::from_rgb(32, 21, 36), Color32::from_rgb(182, 102, 210));
pub const BADGE_SKY:    (Color32, Color32) = (Color32::from_rgb(18, 31, 41), Color32::from_rgb(77, 184, 255));
pub const BADGE_ROSE:   (Color32, Color32) = (Color32::from_rgb(41, 20, 28), Color32::from_rgb(255, 95, 158));
pub const BADGE_GOLD:   (Color32, Color32) = (Color32::from_rgb(40, 33, 17), Color32::from_rgb(245, 200, 66));
pub const BADGE_MINT:   (Color32, Color32) = (Color32::from_rgb(8, 37, 28), Color32::from_rgb(0, 229, 160));
pub const BADGE_PEACH:  (Color32, Color32) = (Color32::from_rgb(41, 24, 20), Color32::from_rgb(255, 126, 95));

// Aliases for compatibility
pub const BADGE_BLUE:   (Color32, Color32) = BADGE_SKY;
pub const BADGE_GREEN:  (Color32, Color32) = BADGE_MINT;
pub const BADGE_ORANGE: (Color32, Color32) = BADGE_PEACH;

// ── Style Setup ───────────────────────────────────────────
pub fn apply(ctx: &egui::Context) {
    let mut style   = (*ctx.style()).clone();
    let mut visuals = if is_light() { egui::Visuals::light() } else { egui::Visuals::dark() };

    visuals.override_text_color = Some(text_body());
    visuals.window_fill          = bg_base();
    visuals.panel_fill           = Color32::TRANSPARENT;
    visuals.window_stroke        = Stroke::new(0.5, border_subtle());

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill   = bg_surface();
    w.noninteractive.fg_stroke = Stroke::new(1.0, text_muted());
    w.noninteractive.rounding  = Rounding::same(20.0);
    w.inactive.bg_fill         = bg_input();
    w.inactive.fg_stroke       = Stroke::new(0.5, border_default());
    w.inactive.rounding        = Rounding::same(18.0);
    w.hovered.bg_fill          = bg_card();
    w.hovered.bg_stroke        = Stroke::new(0.5, teal_strong());
    w.hovered.rounding         = Rounding::same(18.0);
    w.active.bg_fill           = teal_dark();
    w.active.fg_stroke         = Stroke::new(1.0, Color32::WHITE);
    w.active.rounding          = Rounding::same(18.0);

    style.text_styles = [
        (egui::TextStyle::Heading,   FontId::new(20.0, FontFamily::Proportional)),
        (egui::TextStyle::Body,      FontId::new(14.0, FontFamily::Proportional)),
        (egui::TextStyle::Button,    FontId::new(14.0, FontFamily::Proportional)),
        (egui::TextStyle::Small,     FontId::new(11.0, FontFamily::Proportional)),
        (egui::TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ].into();

    style.spacing.item_spacing   = Vec2::new(8.0, 8.0);
    style.spacing.window_margin  = egui::Margin::same(0.0);
    style.spacing.button_padding = Vec2::new(12.0, 8.0);
    style.visuals                = visuals;
    ctx.set_style(style);
}

// ── Widget Helpers ────────────────────────────────────────

/// Gambar filled rounded rect langsung ke painter
pub fn filled_rect(
    ui:       &mut egui::Ui,
    rect:     egui::Rect,
    fill:     Color32,
    stroke:   Stroke,
    rounding: f32,
) {
    ui.painter().rect(rect, Rounding::same(rounding), fill, stroke);
}

/// Card frame untuk seksi konten
pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(bg_surface())
        .stroke(Stroke::new(0.5, border_default()))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
}

/// Tombol teal utama dengan lebar custom
pub fn teal_btn(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    let desired       = Vec2::new(width, 42.0);
    let (rect, resp)  = ui.allocate_exact_size(desired, egui::Sense::click());
    let fill = if resp.is_pointer_button_down_on() { Color32::from_rgb(10, 80, 62) }
               else if resp.hovered()               { teal_strong() }
               else                                 { teal_dark() };
    ui.painter().rect(rect, Rounding::same(8.0), fill, Stroke::NONE);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label,
                      FontId::new(14.0, FontFamily::Proportional), Color32::WHITE);
    resp
}

/// Tombol ghost (outline) dengan lebar custom
pub fn ghost_btn(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    let desired       = Vec2::new(width, 42.0);
    let (rect, resp)  = ui.allocate_exact_size(desired, egui::Sense::click());
    let border  = if resp.hovered() { teal_strong()   } else { border_default() };
    let text_c  = if resp.hovered() { text_primary()  } else { text_muted() };
    ui.painter().rect(rect, Rounding::same(8.0), Color32::TRANSPARENT, Stroke::new(0.5, border));
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label,
                      FontId::new(14.0, FontFamily::Proportional), text_c);
    resp
}

/// Tombol numpad individual
pub fn numpad_btn(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let desired      = Vec2::new(72.0, 56.0);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());
    let fill = if resp.is_pointer_button_down_on() || resp.hovered() {
        Color32::from_rgb(34, 37, 56)
    } else {
        bg_input()
    };
    let border = if resp.hovered() {
        Stroke::new(0.5, Color32::from_rgb(58, 63, 88))
    } else {
        Stroke::new(0.5, border_default())
    };
    ui.painter().rect(rect, Rounding::same(10.0), fill, border);
    let font_size  = if label.len() > 1 { 12.0 } else { 20.0 };
    let text_color = if label.len() > 1 { text_muted() } else { text_primary() };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label,
                      FontId::new(font_size, FontFamily::Proportional), text_color);
    resp
}

/// Ikon dan warna badge berdasarkan ekstensi file
pub fn file_badge(ext: &str) -> (&'static str, (Color32, Color32)) {
    match ext {
        "pdf"|"doc"|"docx"|"txt"|"md"   => ("📄", BADGE_PURPLE),
        "zip"|"tar"|"gz"|"rar"|"7z"     => ("📦", BADGE_GOLD),
        "jpg"|"jpeg"|"png"|"gif"|"webp" => ("🖼",  BADGE_SKY),
        "mp4"|"mov"|"avi"|"mkv"         => ("🎬", BADGE_ROSE),
        "env"|"sh"|"rs"|"py"|"js"       => ("⚙",  BADGE_MINT),
        _                               => ("📁", BADGE_PEACH),
    }
}

/// Ekstrak ekstensi dari nama file
pub fn file_ext(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or("")
}
