use eframe::egui;
use egui::epaint::{Color32, FontId, FontFamily, Stroke, Vec2};
use crate::app_state::{AppScreen, AppState};
use crate::controller::Controller;
use crate::theme::{text_body, filled_rect};

pub fn render_splash(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller) {
    let now = std::time::Instant::now();
    let start = *state.splash_start.get_or_insert_with(|| now);
    let elapsed = start.elapsed().as_secs_f32();

    if elapsed >= 2.5 {
        state.screen = AppScreen::Login;
        ui.ctx().request_repaint();
        return;
    }

    let avail = ui.available_rect_before_wrap();
    ui.allocate_ui_at_rect(avail, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(avail.height() * 0.38); // Adjusted to center better

            // Splash Logo
            let logo_size = Vec2::splat(64.0);
            let (logo_rect, _) = ui.allocate_exact_size(logo_size, egui::Sense::hover());
            
            // Draw a subtle glow behind the logo
            let glow_color = Color32::from_rgba_unmultiplied(99, 102, 241, (elapsed * 50.0).min(30.0) as u8);
            ui.painter().circle_filled(logo_rect.center(), 50.0, glow_color);

            ui.painter().text(
                logo_rect.center(),
                egui::Align2::CENTER_CENTER,
                "🛡", 
                FontId::new(42.0, FontFamily::Proportional),
                Color32::WHITE,
            );

            ui.add_space(20.0);

            // Splash Name
            ui.label(egui::RichText::new("DataVault Aegis")
                .size(26.0)
                .color(Color32::WHITE)
                .strong());

            ui.add_space(8.0);

            // Splash Tagline
            ui.label(egui::RichText::new("Enkripsi militer. Sederhana digunakan.")
                .size(13.0)
                .color(text_body()));

            ui.add_space(40.0);

            // Wave/Pulsing Dot Loader animation matching HTML
            ui.horizontal(|ui| {
                // Center the dots
                let dots_width = 3.0 * 12.0 + 2.0 * 8.0;
                ui.add_space((ui.available_width() - dots_width) / 2.0);

                for i in 0..3 {
                    // HTML uses 1.2s animation, delays: 0, 0.15s, 0.3s
                    let delay = i as f32 * 0.15;
                    let dot_time = (elapsed - delay).max(0.0) % 1.2;
                    
                    // Wave: 0% -> 50% -> 100%
                    let progress = dot_time / 1.2;
                    let (scale, alpha) = if progress < 0.5 {
                        let t = progress / 0.5; // 0 to 1
                        (1.0 + t * 0.6, 0.4 + t * 0.6)
                    } else {
                        let t = (progress - 0.5) / 0.5; // 0 to 1
                        (1.6 - t * 0.6, 1.0 - t * 0.6)
                    };
                    
                    let dot_size = Vec2::splat(8.0 * scale);
                    let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover()); // Fixed container size
                    
                    let dot_color = Color32::from_rgba_unmultiplied(99, 102, 241, (alpha * 255.0) as u8);
                    
                    // Draw centered in the 12x12 container
                    let actual_dot = egui::Rect::from_center_size(dot_rect.center(), dot_size);
                    filled_rect(ui, actual_dot, dot_color, Stroke::NONE, actual_dot.width() / 2.0);
                    ui.add_space(8.0);
                }
            });
        });
    });

    ui.ctx().request_repaint();
}
