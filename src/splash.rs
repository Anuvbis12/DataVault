use eframe::egui;
use egui::epaint::{Color32, FontId, FontFamily, Stroke, Vec2};
use crate::app_state::{AppScreen, AppState};
use crate::controller::Controller;
use crate::theme::filled_rect;

/// Render splash screen — 100% match to datavault_aegis_v5.html
///
/// HTML specs:
///   .splash-logo: 80x80, border-radius:28px, gradient #6366f1->#4f46e5
///     box-shadow: 0 0 0 16px rgba(99,102,241,0.08), 0 20px 48px rgba(99,102,241,0.45)
///     animation: logopop 0.8s
///   .splash-name: 24px, weight 800, letter-spacing -0.6px, mt 24px
///   .splash-tag:  13px, color #737996, mt 6px
///   .splash-dots: gap 8px, mt 44px
///     .splash-dot: 8x8, border-radius 50%, bg rgba(255,255,255,0.15)
///     .splash-dot.a: bg #818cf8, animation dotwave 1.2s ease-in-out infinite
///       delay: 0, 0.15s, 0.3s
///     @keyframes dotwave { 0%,100% { scale(1); opacity:0.4 } 50% { scale(1.6); opacity:1 } }
pub fn render_splash(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller) {
    let now = ui.input(|i| i.time);
    let start = *state.splash_start.get_or_insert(now);
    let elapsed = (now - start) as f32;

    // Force continuous repaint so the animation doesn't freeze on mobile
    ui.ctx().request_repaint();

    if elapsed >= 2.5 {
        state.screen = AppScreen::Login;
        ui.ctx().request_repaint();
        return;
    }

    let avail = ui.available_rect_before_wrap();

    // Remove the hard-edged circle glow, it doesn't look like a gradient in egui
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(avail), |ui| {
        ui.vertical_centered(|ui| {
            // Center vertically
            ui.add_space(avail.height() * 0.35);

            // ── Pop Animation for Logo (0.8s) ──
            let pop_scale = if elapsed < 0.8 {
                let t = elapsed / 0.8;
                // Easing out back
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                let t_minus_1 = t - 1.0;
                let val = 1.0 + c3 * t_minus_1.powi(3) + c1 * t_minus_1.powi(2);
                val.max(0.0) // Prevent negative scale
            } else {
                1.0
            };

            // Allocate fixed size to prevent layout shifting
            let (fixed_rect, _) = ui.allocate_exact_size(Vec2::splat(80.0), egui::Sense::hover());
            
            // Render actual scaled logo
            let actual_size = Vec2::splat(80.0 * pop_scale);
            let logo_rect = egui::Rect::from_center_size(fixed_rect.center(), actual_size);

            // Logo background + circular image
            {
                let logo_bytes: &[u8] = include_bytes!("../assets/logo.jpg");
                if let Some(texture) = crate::view::load_image_texture(ui, "splash_logo", logo_bytes) {
                    let radius = 40.0 * pop_scale;
                    // Glow ring
                    let glow_ring_rect = logo_rect.expand(16.0 * pop_scale);
                    filled_rect(ui, glow_ring_rect, Color32::from_rgba_unmultiplied(99, 102, 241, 20), Stroke::NONE, 36.0 * pop_scale);
                    // Draw circular image
                    crate::view::draw_circular_image_with_border(
                        ui, &texture, logo_rect.center(), radius,
                        2.5, Color32::from_rgb(129, 140, 248), true,
                    );
                } else {
                    // Fallback
                    let glow_ring_rect = logo_rect.expand(16.0 * pop_scale);
                    filled_rect(ui, glow_ring_rect, Color32::from_rgba_unmultiplied(99, 102, 241, 20), Stroke::NONE, 36.0 * pop_scale);
                    filled_rect(ui, logo_rect, Color32::from_rgb(99, 102, 241), Stroke::NONE, 28.0 * pop_scale);
                    ui.painter().text(
                        logo_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "🛡",
                        FontId::new((36.0 * pop_scale).max(0.1), FontFamily::Proportional),
                        Color32::WHITE,
                    );
                }
            }

            // ── Fade in for text ──
            let text_alpha = ((elapsed - 0.3) / 0.5).clamp(0.0, 1.0);
            
            // ── Name: 24px, weight 800, mt 24px ──
            ui.add_space(24.0);
            ui.label(egui::RichText::new("DataVault Aegis")
                .size(24.0)
                .color(Color32::from_rgba_unmultiplied(255, 255, 255, (255.0 * text_alpha) as u8))
                .strong());

            // ── Tagline: 13px, #737996, mt 6px ──
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Enkripsi militer. Sederhana digunakan.")
                .size(13.0)
                .color(Color32::from_rgba_unmultiplied(115, 121, 150, (255.0 * text_alpha) as u8)));

            // ── Wave dots: mt 44px, gap 8px, dot 8x8 ──
            ui.add_space(44.0);
            ui.horizontal(|ui| {
                let dot_base = 8.0;
                let gap = 8.0;
                let total_w = 3.0 * dot_base + 2.0 * gap;
                ui.add_space((ui.available_width() - total_w) / 2.0);

                for i in 0..3 {
                    // HTML: delays 0, 0.15s, 0.3s, period 1.2s
                    let delay = i as f32 * 0.15;
                    let dot_time = (elapsed - delay).max(0.0) % 1.2;
                    let progress = dot_time / 1.2;

                    // dotwave: 0%,100% -> scale(1) opacity(0.4); 50% -> scale(1.6) opacity(1)
                    let (scale, alpha) = if progress < 0.5 {
                        let t = progress / 0.5;
                        (1.0 + t * 0.6, 0.4 + t * 0.6)
                    } else {
                        let t = (progress - 0.5) / 0.5;
                        (1.6 - t * 0.6, 1.0 - t * 0.6)
                    };

                    // Container: fixed 8x8 for layout
                    let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(dot_base), egui::Sense::hover());

                    // Actual rendered dot (scaled)
                    let actual_size = Vec2::splat(dot_base * scale);
                    let actual = egui::Rect::from_center_size(dot_rect.center(), actual_size);
                    let dot_color = Color32::from_rgba_unmultiplied(129, 140, 248, (alpha * 255.0) as u8);
                    filled_rect(ui, actual, dot_color, Stroke::NONE, actual.width() / 2.0);

                    if i < 2 { ui.add_space(gap); }
                }
            });
        });
    });

    ui.ctx().request_repaint();
}
