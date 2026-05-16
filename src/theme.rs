// theme.rs — Konstanta visual dan helper widget
// Seluruh warna, style setup, dan fungsi widget reusable
// dikumpulkan di sini. View layer mengimport dari modul ini.

use eframe::egui;
use egui::epaint::{Color32, FontId, FontFamily, Rounding, Stroke, Vec2};

// ── Palet Warna ───────────────────────────────────────────
// Background layers
pub const BG_BASE:        Color32 = Color32::from_rgb(6, 6, 5);       // #060605
pub const BG_SURFACE:     Color32 = Color32::from_rgb(18, 18, 17);    // #121211
pub const BG_CARD:        Color32 = Color32::from_rgb(30, 30, 29);    // #1e1e1d
pub const BG_INPUT:       Color32 = Color32::from_rgb(26, 26, 25);

// Borders
pub const BORDER_DEFAULT: Color32 = Color32::from_rgba_premultiplied(20, 20, 20, 20); // white alpha 20
pub const BORDER_SUBTLE:  Color32 = Color32::from_rgba_premultiplied(10, 10, 10, 10); // white alpha 10
pub const BORDER_ACCENT:  Color32 = Color32::from_rgb(182, 102, 210); // Purple border

// Text
pub const TEXT_PRIMARY:   Color32 = Color32::from_rgb(255, 255, 255); // #ffffff
pub const TEXT_BODY:      Color32 = Color32::from_rgb(220, 220, 220); 
pub const TEXT_MUTED:     Color32 = Color32::from_rgb(136, 136, 136); // #888888
pub const TEXT_DIMMED:    Color32 = Color32::from_rgb(100, 100, 100);

// Accents — Purple (using old TEAL name to avoid breaking view.rs)
pub const TEAL_STRONG:    Color32 = Color32::from_rgb(182, 102, 210); // #b666d2
pub const TEAL_DARK:      Color32 = Color32::from_rgb(130, 70,  150);
pub const TEAL_LIGHT:     Color32 = Color32::from_rgb(200, 130, 220);
pub const TEAL_FAINT:     Color32 = Color32::from_rgb(220, 180, 240);

// Status
pub const ERROR_COLOR:    Color32 = Color32::from_rgb(244,  63,  94);  // #f43f5e
pub const WARN_COLOR:     Color32 = Color32::from_rgb(250, 204,  21);  // #facc15
pub const SUCCESS_COLOR:  Color32 = Color32::from_rgb(74,  222, 128);  // #4ade80

// File type badge (fill, border)
pub const BADGE_GREEN:    (Color32, Color32) = (Color32::from_rgb(12, 31, 24), Color32::from_rgb(15, 110, 86));
pub const BADGE_PURPLE:   (Color32, Color32) = (Color32::from_rgb(26, 20, 32), Color32::from_rgb(58, 42, 72));
pub const BADGE_ORANGE:   (Color32, Color32) = (Color32::from_rgb(26, 21,  8), Color32::from_rgb(58, 46, 26));
pub const BADGE_BLUE:     (Color32, Color32) = (Color32::from_rgb(12, 20, 40), Color32::from_rgb(26, 42, 72));

// ── Style Setup ───────────────────────────────────────────
pub fn apply(ctx: &egui::Context) {
    let mut style   = (*ctx.style()).clone();
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(TEXT_BODY);
    visuals.window_fill          = BG_BASE;
    visuals.panel_fill           = Color32::TRANSPARENT;
    visuals.window_stroke        = Stroke::new(0.5, BORDER_SUBTLE);

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill   = BG_SURFACE;
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_MUTED);
    w.noninteractive.rounding  = Rounding::same(16.0);
    w.inactive.bg_fill         = BG_INPUT;
    w.inactive.fg_stroke       = Stroke::new(0.5, BORDER_DEFAULT);
    w.inactive.rounding        = Rounding::same(16.0);
    w.hovered.bg_fill          = BG_CARD;
    w.hovered.bg_stroke        = Stroke::new(0.5, TEAL_STRONG);
    w.hovered.rounding         = Rounding::same(16.0);
    w.active.bg_fill           = TEAL_DARK;
    w.active.fg_stroke         = Stroke::new(1.0, Color32::WHITE);
    w.active.rounding          = Rounding::same(16.0);

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
        .fill(BG_SURFACE)
        .stroke(Stroke::new(0.5, BORDER_DEFAULT))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
}

/// Tombol teal utama dengan lebar custom
pub fn teal_btn(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    let desired       = Vec2::new(width, 42.0);
    let (rect, resp)  = ui.allocate_exact_size(desired, egui::Sense::click());
    let fill = if resp.is_pointer_button_down_on() { Color32::from_rgb(10, 80, 62) }
               else if resp.hovered()               { TEAL_STRONG }
               else                                 { TEAL_DARK };
    ui.painter().rect(rect, Rounding::same(8.0), fill, Stroke::NONE);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label,
                      FontId::new(14.0, FontFamily::Proportional), Color32::WHITE);
    resp
}

/// Tombol ghost (outline) dengan lebar custom
pub fn ghost_btn(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    let desired       = Vec2::new(width, 42.0);
    let (rect, resp)  = ui.allocate_exact_size(desired, egui::Sense::click());
    let border  = if resp.hovered() { TEAL_STRONG   } else { BORDER_DEFAULT };
    let text_c  = if resp.hovered() { TEXT_PRIMARY  } else { TEXT_MUTED };
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
        BG_INPUT
    };
    let border = if resp.hovered() {
        Stroke::new(0.5, Color32::from_rgb(58, 63, 88))
    } else {
        Stroke::new(0.5, BORDER_DEFAULT)
    };
    ui.painter().rect(rect, Rounding::same(10.0), fill, border);
    let font_size  = if label.len() > 1 { 12.0 } else { 20.0 };
    let text_color = if label.len() > 1 { TEXT_MUTED } else { TEXT_PRIMARY };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label,
                      FontId::new(font_size, FontFamily::Proportional), text_color);
    resp
}

/// Ikon dan warna badge berdasarkan ekstensi file
pub fn file_badge(ext: &str) -> (&'static str, (Color32, Color32)) {
    match ext {
        "pdf"|"doc"|"docx"|"txt"|"md"   => ("📄", BADGE_GREEN),
        "zip"|"tar"|"gz"|"rar"|"7z"     => ("📦", BADGE_PURPLE),
        "jpg"|"jpeg"|"png"|"gif"|"webp" => ("🖼",  BADGE_BLUE),
        "mp4"|"mov"|"avi"|"mkv"         => ("🎬", BADGE_PURPLE),
        "env"|"sh"|"rs"|"py"|"js"       => ("⚙",  BADGE_ORANGE),
        _                               => ("📁", BADGE_BLUE),
    }
}

/// Ekstrak ekstensi dari nama file
pub fn file_ext(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or("")
}
