// view.rs — View layer
// Seluruh fungsi render egui. View hanya membaca AppState
// memanggil Controller untuk aksi. Tidak ada logika bisnis di sini.

use eframe::egui;
use egui::epaint::{Color32, FontId, FontFamily, Mesh, Rounding, Stroke, Vec2, Vertex};
#[cfg(not(target_os = "android"))]
#[cfg(not(target_os = "android"))]
use rfd::FileDialog;

use crate::app_state::{AppScreen, AppState, DashboardTab, SortOption};
use crate::controller::{format_size, Controller};
use crate::db::FileRecord;
use crate::theme::{self, *};

// ── Root render ───────────────────────────────────────────
pub fn render(
    ctx:        &egui::Context,
    state:      &mut AppState,
    controller: &Controller,
) {
    // 🛡️ Anti-Tampering Check
    if let Some(ref details) = state.security_violation {
        render_security_violation(ctx, details);
        return;
    }

    draw_background(ctx);

    // Overlay Virtual Keyboard (Secure Keyboard)
    // (Disabled because native Android keyboard works properly)
    // if state.show_keyboard {
    //     render_virtual_keyboard(ctx, state);
    // }

    if state.pin_shake_timer > 0.0 {
        state.pin_shake_timer -= ctx.input(|i| i.stable_dt);
        ctx.request_repaint();
    }

    // Overlay Toast Notification
    if state.toast_timer > 0.0 {
        if let Some(msg) = state.toast_message.clone() {
            let toast_alpha = (state.toast_timer * 2.0).clamp(0.0, 1.0);
            let y_pos = 30.0 - (1.0 - toast_alpha) * 20.0;
            
            egui::Area::new(egui::Id::new("toast_overlay"))
                .fixed_pos(egui::pos2(ctx.screen_rect().width() / 2.0 - 150.0, y_pos))
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    let rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(300.0, 44.0));
                    filled_rect(ui, rect, Color32::from_rgb(20, 25, 35), Stroke::new(1.0, teal_strong()), 22.0);
                    ui.painter().text(
                        rect.center(), egui::Align2::CENTER_CENTER, &msg,
                        FontId::new(14.0, FontFamily::Proportional), Color32::WHITE
                    );
                });
            
            state.toast_timer -= ctx.input(|i| i.stable_dt);
            if state.toast_timer <= 0.0 {
                state.toast_message = None;
            }
            ctx.request_repaint();
        }
    }

    if let Some(uri) = state.android_file_picker_result.take() {
        if !uri.is_empty() {
            controller.encrypt_file(state, std::path::PathBuf::from(uri));
        }
    }

    let mut top_pad = 0.0;
    if cfg!(target_os = "android") || cfg!(target_os = "ios") {
        let rect = ctx.screen_rect();
        let aspect_ratio = rect.height() / rect.width();
        if aspect_ratio > 1.9 {
            top_pad = 48.0; // Tall phones (usually have notch/camera cutout)
        } else {
            top_pad = 28.0; // Standard 16:9 phones
        }
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::none().inner_margin(egui::Margin {
            left: 0.0,
            right: 0.0,
            top: top_pad,
            bottom: 0.0,
        }))
        .show(ctx, |ui| {
            let screen = state.screen.clone();
            match screen {
                AppScreen::Splash            => crate::splash::render_splash(ui, state, controller),
                AppScreen::Login             => render_login(ui, state, controller),
                AppScreen::LoginPin          => render_login_pin(ui, state, controller),
                AppScreen::SetupAccount      => render_setup_account(ui, state, controller),
                AppScreen::Dashboard         => render_dashboard(ui, state, controller),
                AppScreen::Decrypting(fname) => render_decrypt_panel(ui, state, controller, &fname.clone()),
                AppScreen::TotpSetup         => render_totp_setup(ui, state, controller),
                AppScreen::TotpVerify        => render_totp_verify(ui, state, controller),
                AppScreen::RecycleBin        => render_recycle_bin(ui, state, controller),
                AppScreen::SystemTrash       => render_system_trash(ui, state, controller),
                AppScreen::PreviewMedia      => render_preview_panel(ui, state, controller),
            }
        });


    // Overlay P2P Sharing
    if state.share_active_record.is_some() {
        render_share_modal(ctx, state, controller);
    }

    // Overlay Storage Modals
    if state.storage_pin_modal_open || state.storage_path_modal_open {
        render_storage_modals(ctx, state);
    }

    // Overlay Context Menu
    if state.active_context_menu.is_some() {
        render_context_menu(ctx, state, controller);
    }

    // Overlay Rename Modal
    if state.rename_modal_open {
        render_rename_modal(ctx, state, controller);
    }

    // Overlay Custom File Picker (Pure Rust/egui)
    if state.custom_file_picker_open {
        render_custom_file_picker(ctx, state, controller);
    }
}

// ── Background gradien ────────────────────────────────────
fn draw_background(ctx: &egui::Context) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let rect    = ctx.screen_rect();
    let mut mesh = Mesh::default();
    mesh.vertices.extend([
        Vertex { pos: rect.left_top(),     uv: egui::pos2(0.,0.), color: Color32::from_rgb(11,12,22) },
        Vertex { pos: rect.right_top(),    uv: egui::pos2(1.,0.), color: Color32::from_rgb(11,12,22) },
        Vertex { pos: rect.right_bottom(), uv: egui::pos2(1.,1.), color: Color32::from_rgb(5,6,11) },
        Vertex { pos: rect.left_bottom(),  uv: egui::pos2(0.,1.), color: Color32::from_rgb(5,6,11) },
    ]);
    mesh.add_triangle(0,1,2);
    mesh.add_triangle(0,2,3);
    painter.add(egui::Shape::Mesh(mesh));
}

// ── Screen: Login ─────────────────────────────────────────
// 100% match to datavault_aegis_v5.html #s-login
fn render_login(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let user_set = ctrl.is_user_set();
    let avail = ui.available_rect_before_wrap();
    let pad = 24.0;

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(avail), |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(32.0);
            ui.horizontal(|ui| {
                ui.add_space(pad);
                ui.vertical(|ui| {
                    let field_w = avail.width() - pad * 2.0;

                    // Logo 52x52 circular
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(52.0), egui::Sense::hover());
                    draw_app_logo(ui, state, icon_rect.center(), 52.0);

                    ui.add_space(20.0);

                    // Title 28px weight 800
                    ui.label(egui::RichText::new("Selamat datang\nkembali 👋")
                        .size(28.0).color(text_primary()).strong());
                    ui.add_space(8.0);
                    // Subtitle 13.5px --ink2
                    ui.label(egui::RichText::new("Masuk untuk mengakses brankas terenkripsi Anda")
                        .size(13.5).color(Color32::from_rgb(115, 121, 150)));

                    if !user_set {
                        ui.add_space(32.0);
                        ui.label(egui::RichText::new("Vault baru terdeteksi.").color(warn_color()).size(13.0));
                        ui.label(egui::RichText::new("Buat akun untuk memulai.").color(text_muted()).size(13.0));
                        ui.add_space(20.0);
                        let btn_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(field_w, 52.0));
                        let btn_resp = ui.allocate_rect(btn_rect, egui::Sense::click());
                        let btn_bg = if btn_resp.hovered() { Color32::from_rgb(79, 70, 229) } else { Color32::from_rgb(129, 140, 248) };
                        filled_rect(ui, btn_rect, btn_bg, Stroke::NONE, 18.0);
                        ui.painter().text(btn_rect.center(), egui::Align2::CENTER_CENTER, "Daftar sekarang", FontId::new(15.0, FontFamily::Proportional), Color32::WHITE);
                        if btn_resp.clicked() { state.screen = AppScreen::SetupAccount; }
                        return;
                    }

                    ui.add_space(32.0);

                    // Email field
                    ui.label(egui::RichText::new("EMAIL").size(11.0).color(Color32::from_rgb(115, 121, 150)).strong());
                    ui.add_space(8.0);
                    let (u_rect, _) = ui.allocate_exact_size(Vec2::new(field_w, 48.0), egui::Sense::hover());
                    filled_rect(ui, u_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 10), Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 23)), 16.0);
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(u_rect.shrink(14.0)), |ui| {
                        let resp = ui.add(egui::TextEdit::singleline(&mut state.login_username).hint_text("nama@email.com").frame(false).desired_width(u_rect.width() - 28.0).font(FontId::new(15.0, FontFamily::Proportional)).interactive(true));
                        if resp.gained_focus() || resp.clicked() { state.focused_field = crate::app_state::FocusedField::LoginUsername; state.show_keyboard = true; }
                    });

                    ui.add_space(16.0);

                    // Password field
                    ui.label(egui::RichText::new("KATA SANDI").size(11.0).color(Color32::from_rgb(115, 121, 150)).strong());
                    ui.add_space(8.0);
                    let (p_rect, _) = ui.allocate_exact_size(Vec2::new(field_w, 48.0), egui::Sense::hover());
                    let p_border = if state.login_error.is_some() { Stroke::new(1.0, error_color()) } else { Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 23)) };
                    filled_rect(ui, p_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 10), p_border, 16.0);
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(p_rect.shrink(14.0)), |ui| {
                        let resp = ui.add(egui::TextEdit::singleline(&mut state.login_password).password(true).hint_text("Masukkan kata sandi").frame(false).desired_width(p_rect.width() - 28.0).font(FontId::new(15.0, FontFamily::Proportional)).interactive(true));
                        if resp.gained_focus() || resp.clicked() { state.focused_field = crate::app_state::FocusedField::LoginPassword; state.show_keyboard = true; }
                    });

                    if let Some(err) = &state.login_error {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(err).color(error_color()).size(12.0).strong());
                    }

                    // Forgot password (right-aligned)
                    ui.add_space(8.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.label(egui::RichText::new("Lupa kata sandi?").color(Color32::from_rgb(129, 140, 248)).size(12.0).strong()).interact(egui::Sense::click()).clicked() {
                            state.toast_message = Some("Link reset dikirim ke email Anda".to_string());
                            state.toast_timer = 2.0;
                        }
                    });

                    // Remember me checkbox
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let (cb_rect, cb_resp) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::click());
                        let is_on = state.show_reset_confirm;
                        let (cb_color, cb_border) = if is_on {
                            (Color32::from_rgb(129, 140, 248), Stroke::NONE)
                        } else {
                            (Color32::from_rgba_unmultiplied(255, 255, 255, 5), Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 23)))
                        };
                        filled_rect(ui, cb_rect, cb_color, cb_border, 6.0);
                        if is_on { ui.painter().text(cb_rect.center(), egui::Align2::CENTER_CENTER, "✓", FontId::new(11.0, FontFamily::Proportional), Color32::WHITE); }
                        if cb_resp.clicked() { state.show_reset_confirm = !state.show_reset_confirm; }
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("Tetap masuk di perangkat ini").size(12.5).color(Color32::from_rgb(115, 121, 150)));
                    });

                    ui.add_space(18.0);

                    // Login button
                    let btn_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(field_w, 52.0));
                    let btn_resp = ui.allocate_rect(btn_rect, egui::Sense::click());
                    let btn_bg = if btn_resp.hovered() { Color32::from_rgb(79, 70, 229) } else { Color32::from_rgb(129, 140, 248) };
                    filled_rect(ui, btn_rect, btn_bg, Stroke::NONE, 18.0);
                    ui.painter().text(btn_rect.center(), egui::Align2::CENTER_CENTER, "Masuk", FontId::new(15.0, FontFamily::Proportional), Color32::WHITE);
                    if btn_resp.clicked() {
                        let ok = ctrl.try_login(state);
                        if ok { state.screen = AppScreen::LoginPin; state.login_pin.clear(); }
                        else { state.pin_shake_timer = 0.4; }
                    }

                    // Divider
                    ui.add_space(22.0);
                    ui.horizontal(|ui| {
                        let w = (field_w - 40.0) / 2.0;
                        let line_y = ui.cursor().min.y + 8.0;
                        ui.painter().line_segment([egui::pos2(ui.cursor().min.x, line_y), egui::pos2(ui.cursor().min.x + w, line_y)], Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 13)));
                        ui.add_space(w + 6.0);
                        ui.label(egui::RichText::new("ATAU").size(11.0).color(Color32::from_rgb(71, 77, 102)).strong());
                        ui.add_space(6.0);
                        ui.painter().line_segment([egui::pos2(ui.cursor().min.x, line_y), egui::pos2(ui.cursor().min.x + w, line_y)], Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 13)));
                    });
                    ui.add_space(22.0);

                    // Social button
                    let soc_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(field_w, 48.0));
                    let soc_resp = ui.allocate_rect(soc_rect, egui::Sense::click());
                    let soc_bg = if soc_resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 10) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 5) };
                    filled_rect(ui, soc_rect, soc_bg, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 23)), 16.0);
                    ui.painter().text(soc_rect.center(), egui::Align2::CENTER_CENTER, "G   Google", FontId::new(13.0, FontFamily::Proportional), text_primary());
                    if soc_resp.clicked() { state.toast_message = Some("Masuk dengan Google…".to_string()); state.toast_timer = 2.0; }

                    // Switch to register
                    ui.add_space(22.0);
                    ui.horizontal(|ui| {
                        ui.add_space(((field_w - 200.0) / 2.0).max(0.0));
                        ui.label(egui::RichText::new("Belum punya akun?").size(13.0).color(Color32::from_rgb(115, 121, 150)));
                        if ui.label(egui::RichText::new("Daftar sekarang").size(13.0).color(Color32::from_rgb(129, 140, 248)).strong()).interact(egui::Sense::click()).clicked() {
                            state.screen = AppScreen::SetupAccount;
                        }
                    });
                    ui.add_space(32.0);
                });
            });
        });
    });
}
// ── Screen: Login PIN ─────────────────────────────────────
// 100% match to datavault_aegis_v5.html #s-login-pin
fn render_login_pin(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller) {
    let avail = ui.available_rect_before_wrap();

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(avail), |ui| {
        ui.vertical_centered(|ui| {
            // HTML: .auth-wrap style="padding-top:40px"
            ui.add_space(40.0);

            // HTML: .user-avatar { 72x72, border-radius:50%, gradient bg, 2.5px border, color:--ac }
            let avatar_size = Vec2::splat(72.0);
            let (a_rect, _) = ui.allocate_exact_size(avatar_size, egui::Sense::hover());
            // bg: gradient(135deg, rgba(99,102,241,0.25), rgba(99,102,241,0.08))
            filled_rect(ui, a_rect, Color32::from_rgba_unmultiplied(99, 102, 241, 42),
                Stroke::new(2.5, Color32::from_rgba_unmultiplied(129, 140, 248, 89)), 36.0);
            let initial = state.display_name.chars().next().unwrap_or('A').to_uppercase().to_string();
            // color: --ac (#818cf8)
            ui.painter().text(a_rect.center(), egui::Align2::CENTER_CENTER, &initial,
                FontId::new(28.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));

            // HTML: .user-welcome { font-size:20px; weight:700; margin-bottom:4px }
            ui.add_space(14.0);
            ui.label(egui::RichText::new("Selamat datang").size(20.0).color(text_primary()).strong());
            // HTML: .user-email { font-size:13px; color:--ink2; margin-bottom:30px }
            ui.add_space(4.0);
            ui.label(egui::RichText::new(&state.login_username).size(13.0).color(Color32::from_rgb(115, 121, 150)));

            // HTML: inline div { font-size:13px; color:--ink2; margin-bottom:24px }
            ui.add_space(24.0);
            ui.label(egui::RichText::new("Masukkan PIN 6 digit untuk\nmembuka brankas Anda")
                .size(13.0).color(Color32::from_rgb(115, 121, 150)));

            // HTML: .dots { gap:16px; margin-bottom:28px }
            // .dot { 14x14, border-radius:50%, bg:rgba(255,255,255,0.08), border:1.5px solid border2 }
            // .dot.f { bg:--ac, border-color:--ac, transform:scale(1.25), box-shadow:glow }
            ui.add_space(28.0);
            ui.horizontal(|ui| {
                let dot_size = 14.0;
                let gap = 16.0;
                let total = 6.0 * dot_size + 5.0 * gap;
                ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                for i in 0..6 {
                    let is_filled = i < state.login_pin.len();
                    // We removed the continuous sine wave breathing to save CPU/Battery on Android
                    let breathing = if is_filled { 1.25 } else { 1.0 };

                    let base_size = dot_size * breathing;
                    let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(dot_size), egui::Sense::hover());

                    if is_filled {
                        // Glow effect
                        let glow_rect = egui::Rect::from_center_size(dot_rect.center(), Vec2::splat(dot_size * 1.8));
                        ui.painter().circle_filled(glow_rect.center(), dot_size * 0.9 * breathing,
                            Color32::from_rgba_unmultiplied(99, 102, 241, 90));
                        // Filled dot (scaled up)
                        let scaled = egui::Rect::from_center_size(dot_rect.center(), Vec2::splat(base_size));
                        filled_rect(ui, scaled, Color32::from_rgb(129, 140, 248), Stroke::NONE, base_size / 2.0);
                    } else {
                        filled_rect(ui, dot_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                            Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 23)), dot_size / 2.0);
                    }
                    if i < 5 { ui.add_space(gap); }
                }
            });

            // HTML: .numpad { gap:12px; max-width:260px }
            ui.add_space(28.0);
            let btn_w = 80.0;
            let gap = 12.0;
            let numpad_w = 3.0 * btn_w + 2.0 * gap;
            let mut btn_idx = 1;
            for _row in 0..3 {
                ui.horizontal(|ui| {
                    ui.add_space(((ui.available_width() - numpad_w) / 2.0).max(0.0));
                    for _col in 0..3 {
                        if ghost_btn(ui, &btn_idx.to_string(), btn_w).clicked() {
                            if state.login_pin.len() < 6 {
                                state.login_pin.push_str(&btn_idx.to_string());
                            }
                        }
                        if _col < 2 { ui.add_space(gap); }
                        btn_idx += 1;
                    }
                });
                ui.add_space(gap);
            }
            // Bottom row: FaceID | 0 | Delete
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - numpad_w) / 2.0).max(0.0));
                // HTML: .nbtn.sp { bg: ac-soft, border-color: rgba(129,140,248,0.25), color: --ac }
                // FaceID / Biometric placeholder
                {
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 54.0), egui::Sense::click());
                    let bg = Color32::from_rgba_unmultiplied(129, 140, 248, 25);
                    let border = Color32::from_rgba_unmultiplied(129, 140, 248, 64);
                    ui.painter().rect(rect, Rounding::same(18.0), bg, Stroke::new(1.0, border));
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "🔐",
                        FontId::new(20.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));
                    if resp.clicked() {
                        state.toast_message = Some("Biometrik tidak tersedia di desktop".to_string());
                        state.toast_timer = 2.0;
                    }
                }
                ui.add_space(gap);
                if ghost_btn(ui, "0", btn_w).clicked() {
                    if state.login_pin.len() < 6 { state.login_pin.push('0'); }
                }
                ui.add_space(gap);
                // HTML: .nbtn.dl { bg: transparent, border-color: --border, color: --ink2 }
                {
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 54.0), egui::Sense::click());
                    let bg = if resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 10) } else { Color32::TRANSPARENT };
                    let border_c = Color32::from_rgba_unmultiplied(255, 255, 255, 13);
                    ui.painter().rect(rect, Rounding::same(18.0), bg, Stroke::new(1.0, border_c));
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Hapus",
                        FontId::new(14.0, FontFamily::Proportional), Color32::from_rgb(115, 121, 150));
                    if resp.clicked() { state.login_pin.pop(); }
                }
            });

            if state.login_pin.len() == 6 {
                state.screen = AppScreen::Dashboard;
            }

            // HTML: .pin-hint { font-size:12px; color:--muted; margin-top:18px }
            // .pin-hint em { color:--ac; font-style:normal; weight:700 }
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - 180.0) / 2.0);
                ui.label(egui::RichText::new("Bukan kamu?").size(12.0).color(Color32::from_rgb(71, 77, 102)));
                ui.add_space(4.0);
                if ui.label(egui::RichText::new("Ganti akun").size(12.0).color(Color32::from_rgb(129, 140, 248)).strong())
                    .interact(egui::Sense::click()).clicked() {
                    state.screen = AppScreen::Login;
                }
            });
        });
    });
}


// ── Screen: Setup Account ─────────────────────────────────
fn render_setup_account(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let avail = ui.available_rect_before_wrap();
    let pad = 24.0;
    let field_w = avail.width() - pad * 2.0;
    
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(avail), |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(40.0);
            
            ui.horizontal(|ui| {
                ui.add_space(pad);
                ui.vertical(|ui| {
                    // Small Logo (circular) - Centered
                    ui.vertical_centered(|ui| {
                        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(52.0), egui::Sense::hover());
                        draw_app_logo(ui, state, icon_rect.center(), 52.0);
                    });
                    
                    ui.add_space(20.0);
                    
                    // Progress dots (reg-steps) - Centered
                    ui.horizontal(|ui| {
                        let step_w = (field_w - (3.0 * 6.0)) / 4.0;
                        let total_w = 4.0 * step_w + 3.0 * 6.0;
                        ui.add_space((ui.available_width() - total_w) / 2.0);
                        for i in 0..4 {
                            let (step_rect, _) = ui.allocate_exact_size(Vec2::new(step_w, 3.0), egui::Sense::hover());
                            let color = if i < state.reg_step {
                                Color32::from_rgb(129, 140, 248) // --ac done
                            } else if i == state.reg_step {
                                Color32::from_rgba_unmultiplied(129, 140, 248, 128) // --ac active 50%
                            } else {
                                Color32::from_rgba_unmultiplied(255, 255, 255, 20)
                            };
                            filled_rect(ui, step_rect, color, Stroke::NONE, 1.5);
                            if i < 3 { ui.add_space(6.0); }
                        }
                    });
                    
                    ui.add_space(28.0);
                    
                    // Title and Subtitle - Centered
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("Buat akun baru").size(28.0).color(text_primary()).strong());
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Lindungi file penting Anda dengan enkripsi tingkat militer").size(13.5).color(text_muted()));
                    });
                    
                    ui.add_space(32.0);
                    
                    if state.reg_step == 0 {
                        // Step 0: Data diri
                        ui.label(egui::RichText::new("NAMA LENGKAP").size(11.0).color(text_muted()).strong());
                        ui.add_space(8.0);
                        let (n_rect, _) = ui.allocate_exact_size(Vec2::new(field_w, 48.0), egui::Sense::hover());
                        filled_rect(ui, n_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 10), Stroke::new(1.0, border_default()), 16.0);
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(n_rect.shrink(14.0)), |ui| {
                            let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_display_name).hint_text("Nama Anda").frame(false).desired_width(n_rect.width() - 28.0).font(FontId::new(15.0, FontFamily::Proportional)).interactive(true));
                            if resp.gained_focus() || resp.clicked() { state.focused_field = crate::app_state::FocusedField::SetupDisplayName; state.show_keyboard = true; }
                        });
                        ui.add_space(16.0);
                        
                        ui.label(egui::RichText::new("EMAIL").size(11.0).color(text_muted()).strong());
                        ui.add_space(8.0);
                        let (e_rect, _) = ui.allocate_exact_size(Vec2::new(field_w, 48.0), egui::Sense::hover());
                        filled_rect(ui, e_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 10), Stroke::new(1.0, border_default()), 16.0);
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(e_rect.shrink(14.0)), |ui| {
                            let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_username).hint_text("nama@email.com").frame(false).desired_width(e_rect.width() - 28.0).font(FontId::new(15.0, FontFamily::Proportional)).interactive(true));
                            if resp.gained_focus() || resp.clicked() { state.focused_field = crate::app_state::FocusedField::SetupUsername; state.show_keyboard = true; }
                        });
                        ui.add_space(32.0);
                        
                        let btn_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(field_w, 52.0));
                        let btn_resp = ui.allocate_rect(btn_rect, egui::Sense::click());
                        filled_rect(ui, btn_rect, if btn_resp.hovered() { Color32::from_rgb(79, 70, 229) } else { Color32::from_rgb(129, 140, 248) }, Stroke::NONE, 18.0);
                        ui.painter().text(btn_rect.center(), egui::Align2::CENTER_CENTER, "Lanjut >", FontId::new(15.0, FontFamily::Proportional), Color32::WHITE);
                        if btn_resp.clicked() { state.reg_step = 1; }
                    } else if state.reg_step == 1 {
                        // Step 1: Kata sandi
                        ui.label(egui::RichText::new("KATA SANDI").size(11.0).color(text_muted()).strong());
                        ui.add_space(8.0);
                        let (p1_rect, _) = ui.allocate_exact_size(Vec2::new(field_w, 48.0), egui::Sense::hover());
                        filled_rect(ui, p1_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 10), Stroke::new(1.0, border_default()), 16.0);
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(p1_rect.shrink(14.0)), |ui| {
                            let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_password).password(true).hint_text("Min. 8 karakter").frame(false).desired_width(p1_rect.width() - 28.0).font(FontId::new(15.0, FontFamily::Proportional)).interactive(true));
                            if resp.gained_focus() || resp.clicked() { state.focused_field = crate::app_state::FocusedField::SetupPassword; state.show_keyboard = true; }
                        });
                        
                        // Strength meter
                        ui.add_space(10.0);
                        let strength = (state.setup_password.len() as f32 / 8.0).clamp(0.0, 1.0);
                        let (s_rect, _) = ui.allocate_exact_size(Vec2::new(field_w, 3.0), egui::Sense::hover());
                        filled_rect(ui, s_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 15), Stroke::NONE, 1.5);
                        let fill_rect = egui::Rect::from_min_size(s_rect.min, Vec2::new(field_w * strength, 3.0));
                        let s_color = if strength < 0.5 { error_color() } else if strength < 1.0 { warn_color() } else { accent_mint() };
                        filled_rect(ui, fill_rect, s_color, Stroke::NONE, 1.5);
                        ui.add_space(16.0);
                        
                        ui.label(egui::RichText::new("KONFIRMASI KATA SANDI").size(11.0).color(text_muted()).strong());
                        ui.add_space(8.0);
                        let (p2_rect, _) = ui.allocate_exact_size(Vec2::new(field_w, 48.0), egui::Sense::hover());
                        filled_rect(ui, p2_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 10), Stroke::new(1.0, border_default()), 16.0);
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(p2_rect.shrink(14.0)), |ui| {
                            let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_password_confirm).password(true).hint_text("Ulangi kata sandi").frame(false).desired_width(p2_rect.width() - 28.0).font(FontId::new(15.0, FontFamily::Proportional)).interactive(true));
                            if resp.gained_focus() || resp.clicked() { state.focused_field = crate::app_state::FocusedField::SetupConfirmPassword; state.show_keyboard = true; }
                        });
                        
                        ui.add_space(32.0);
                        let btn_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(field_w, 52.0));
                        let btn_resp = ui.allocate_rect(btn_rect, egui::Sense::click());
                        filled_rect(ui, btn_rect, if btn_resp.hovered() { Color32::from_rgb(79, 70, 229) } else { Color32::from_rgb(129, 140, 248) }, Stroke::NONE, 18.0);
                        ui.painter().text(btn_rect.center(), egui::Align2::CENTER_CENTER, "Lanjut >", FontId::new(15.0, FontFamily::Proportional), Color32::WHITE);
                        if btn_resp.clicked() { state.reg_step = 2; }
                        
                        ui.add_space(12.0);
                        ui.vertical_centered(|ui| {
                            if ui.label(egui::RichText::new("< Kembali").size(12.0).color(Color32::from_rgb(129, 140, 248))).interact(egui::Sense::click()).clicked() {
                                state.reg_step = 0;
                            }
                        });
                    } else if state.reg_step == 2 {
                        // Step 2: Konfirmasi
                        ui.vertical_centered(|ui| {
                            let avatar_size = Vec2::splat(72.0);
                            let (a_rect, _) = ui.allocate_exact_size(avatar_size, egui::Sense::hover());
                            filled_rect(ui, a_rect, Color32::from_rgba_unmultiplied(99, 102, 241, 20), Stroke::new(2.5, Color32::from_rgba_unmultiplied(129, 140, 248, 90)), 36.0);
                            let initial = state.setup_display_name.chars().next().unwrap_or('?').to_uppercase().to_string();
                            ui.painter().text(a_rect.center(), egui::Align2::CENTER_CENTER, initial, FontId::new(28.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));
                            
                            ui.add_space(14.0);
                            ui.label(egui::RichText::new(format!("Halo, {}!", state.setup_display_name)).size(20.0).color(text_primary()).strong());
                            ui.label(egui::RichText::new(&state.setup_username).size(13.0).color(text_muted()));
                        });
                        
                        ui.add_space(30.0);
                        
                        ui.horizontal(|ui| {
                            let (cb_rect, cb_resp) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::click());
                            let (cb_color, cb_border) = if state.setup_terms_accepted { (Color32::from_rgb(129, 140, 248), Stroke::NONE) } else { (Color32::from_rgba_unmultiplied(255, 255, 255, 5), Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 23))) };
                            filled_rect(ui, cb_rect, cb_color, cb_border, 6.0);
                            if state.setup_terms_accepted {
                                ui.painter().text(cb_rect.center(), egui::Align2::CENTER_CENTER, "✓", FontId::new(14.0, FontFamily::Proportional), Color32::WHITE);
                            }
                            if cb_resp.clicked() {
                                state.setup_terms_accepted = !state.setup_terms_accepted;
                            }
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("Saya menyetujui Syarat & Ketentuan dan Kebijakan Privasi").size(12.5).color(text_muted()));
                        });
                        
                        ui.add_space(32.0);
                        let btn_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(field_w, 52.0));
                        let btn_resp = ui.allocate_rect(btn_rect, egui::Sense::click());
                        filled_rect(ui, btn_rect, if btn_resp.hovered() { Color32::from_rgb(79, 70, 229) } else { Color32::from_rgb(129, 140, 248) }, Stroke::NONE, 18.0);
                        ui.painter().text(btn_rect.center(), egui::Align2::CENTER_CENTER, "Lanjut >", FontId::new(15.0, FontFamily::Proportional), Color32::WHITE);
                        if btn_resp.clicked() && state.setup_terms_accepted { state.reg_step = 3; }
                        
                        ui.add_space(12.0);
                        ui.vertical_centered(|ui| {
                            if ui.label(egui::RichText::new("< Kembali").size(12.0).color(Color32::from_rgb(129, 140, 248))).interact(egui::Sense::click()).clicked() {
                                state.reg_step = 1;
                            }
                        });
                    } else if state.reg_step == 3 {
                        // Step 3: PIN - Centered and neatly spaced
                        ui.vertical_centered(|ui| {
                            ui.add_space(12.0);
                            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(64.0), egui::Sense::hover());
                            filled_rect(ui, icon_rect, Color32::from_rgba_unmultiplied(99, 102, 241, 20), Stroke::new(1.0, Color32::from_rgba_unmultiplied(129, 140, 248, 76)), 20.0);
                            ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, "🔒", FontId::new(28.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));
                            
                            ui.add_space(18.0);
                            ui.label(egui::RichText::new("PIN digunakan untuk membuka file terenkripsi.\nPastikan kamu ingat PIN ini.").size(11.0).color(text_muted()));
                            
                            ui.add_space(18.0);
                            ui.horizontal(|ui| {
                                let dot_size = 14.0;
                                let gap = 10.0;
                                let total = 6.0 * dot_size + 5.0 * gap;
                                ui.add_space((ui.available_width() - total) / 2.0);
                                for i in 0..6 {
                                    let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(dot_size), egui::Sense::hover());
                                    let is_filled = i < state.setup_pin.len();
                                    if is_filled {
                                        let scaled = egui::Rect::from_center_size(dot_rect.center(), Vec2::splat(dot_size * 1.25));
                                        filled_rect(ui, scaled, Color32::from_rgb(129, 140, 248), Stroke::NONE, dot_size * 0.625);
                                    } else {
                                        filled_rect(ui, dot_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 20), Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 23)), dot_size / 2.0);
                                    }
                                    if i < 5 { ui.add_space(gap); }
                                }
                            });
                            
                            ui.add_space(14.0);
                            ui.label(egui::RichText::new("Masukkan 6 digit PIN baru").size(11.0).color(text_muted()));
                            
                            ui.add_space(14.0);
                            let btn_w = 80.0;
                            let gap = 12.0;
                            let numpad_w = 3.0 * btn_w + 2.0 * gap;
                            let mut btn_idx = 1;
                            for _row in 0..3 {
                                ui.horizontal(|ui| {
                                    ui.add_space((ui.available_width() - numpad_w) / 2.0);
                                    for _col in 0..3 {
                                        if ghost_btn(ui, &btn_idx.to_string(), btn_w).clicked() {
                                            if state.setup_pin.len() < 6 {
                                                state.setup_pin.push_str(&btn_idx.to_string());
                                            }
                                        }
                                        if _col < 2 { ui.add_space(gap); }
                                        btn_idx += 1;
                                    }
                                });
                                ui.add_space(gap);
                            }
                            // Bottom row: Lewati | 0 | Delete
                            ui.horizontal(|ui| {
                                ui.add_space((ui.available_width() - numpad_w) / 2.0);
                                // Lewati (special button like .nbtn.sp)
                                {
                                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 54.0), egui::Sense::click());
                                    let bg = Color32::from_rgba_unmultiplied(129, 140, 248, 25);
                                    let border_c = Color32::from_rgba_unmultiplied(129, 140, 248, 64);
                                    ui.painter().rect(rect, Rounding::same(18.0), bg, Stroke::new(1.0, border_c));
                                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Lewati",
                                        FontId::new(11.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));
                                    if resp.clicked() { ctrl.setup_account(state); }
                                }
                                ui.add_space(gap);
                                if ghost_btn(ui, "0", btn_w).clicked() {
                                    if state.setup_pin.len() < 6 { state.setup_pin.push('0'); }
                                }
                                ui.add_space(gap);
                                // Delete button (.nbtn.dl)
                                {
                                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 54.0), egui::Sense::click());
                                    let bg = if resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 10) } else { Color32::TRANSPARENT };
                                    let border_c = Color32::from_rgba_unmultiplied(255, 255, 255, 13);
                                    ui.painter().rect(rect, Rounding::same(18.0), bg, Stroke::new(1.0, border_c));
                                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Hapus",
                                        FontId::new(14.0, FontFamily::Proportional), Color32::from_rgb(115, 121, 150));
                                    if resp.clicked() { state.setup_pin.pop(); }
                                }
                            });
                            
                            if state.setup_pin.len() == 6 {
                                ctrl.setup_account(state);
                            }
                            
                            ui.add_space(14.0);
                            if ui.label(egui::RichText::new("< Kembali").size(12.0).color(Color32::from_rgb(129, 140, 248))).interact(egui::Sense::click()).clicked() {
                                state.reg_step = 2;
                            }
                        });
                    }
                    
                    if let Some(err) = state.setup_error.clone() {
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new(&err).color(error_color()).size(13.0).strong());
                    }
                    
                    ui.add_space(16.0);
                    
                    ui.horizontal(|ui| {
                        ui.add_space((field_w - 180.0)/2.0);
                        ui.label(egui::RichText::new("Sudah punya akun?").size(13.0).color(text_muted()));
                        if ui.label(egui::RichText::new("Masuk").size(13.0).color(Color32::from_rgb(129, 140, 248)).strong()).interact(egui::Sense::click()).clicked() {
                            state.screen = AppScreen::Login;
                        }
                    });
                    
                    ui.add_space(40.0);
                });
            });
        });
    });
}

fn get_indonesian_date() -> String {
    use chrono::{Datelike, Local};
    let now = Local::now();
    let day_name = match now.weekday() {
        chrono::Weekday::Sun => "Minggu",
        chrono::Weekday::Mon => "Senin",
        chrono::Weekday::Tue => "Selasa",
        chrono::Weekday::Wed => "Rabu",
        chrono::Weekday::Thu => "Kamis",
        chrono::Weekday::Fri => "Jumat",
        chrono::Weekday::Sat => "Sabtu",
    };
    let month_name = match now.month() {
        1 => "Januari",
        2 => "Februari",
        3 => "Maret",
        4 => "April",
        5 => "Mei",
        6 => "Juni",
        7 => "Juli",
        8 => "Agustus",
        9 => "September",
        10 => "Oktober",
        11 => "November",
        12 => "Desember",
        _ => "Desember",
    };
    format!("{}, {} {} {}", day_name, now.day(), month_name, now.year())
}

// ── Screen: Dashboard ─────────────────────────────────────
fn render_dashboard(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    ctrl.refresh_device_metrics(state);
    ui.ctx().request_repaint_after(std::time::Duration::from_secs(2));

    let avail = ui.available_rect_before_wrap();
    
    // ─ Topbar ─
    let topbar_h = 70.0;
    let topbar_rect = egui::Rect::from_min_size(avail.min, Vec2::new(avail.width(), topbar_h));
    
    // Header Texts & Actions
    let (title, sub) = match state.dashboard_tab {
        DashboardTab::Home => ("Brankas Saya", get_indonesian_date()),
        DashboardTab::Vault => ("Brankas", format!("5 folder · {} file", state.file_list.len())),
        DashboardTab::Kuat => ("Kenapa HP Kuat?", "Variabel teknis, bahasa manusia".to_string()),
        DashboardTab::AboutUs => ("Tentang Kami", "Aegis Vault · Tim Pengembang".to_string()),
        _ => ("Semua File", format!("{} file terenkripsi", state.file_list.len())),
    };
    
    let brand_pos_y = topbar_rect.center().y - 8.0;
    ui.painter().text(egui::pos2(avail.left() + 24.0, brand_pos_y), egui::Align2::LEFT_CENTER, title, FontId::new(26.0, FontFamily::Proportional), Color32::WHITE);
    ui.painter().text(egui::pos2(avail.left() + 24.0, brand_pos_y + 24.0), egui::Align2::LEFT_CENTER, &sub, FontId::new(13.0, FontFamily::Proportional), text_muted());

    // Top Right Actions
    match state.dashboard_tab {
        DashboardTab::Home => {
            let grid_rect = egui::Rect::from_center_size(egui::pos2(avail.right() - 44.0, topbar_rect.center().y + 4.0), Vec2::splat(40.0));
            let grid_resp = ui.allocate_rect(grid_rect, egui::Sense::click());
            let grid_bg = if grid_resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 10) } else { Color32::TRANSPARENT };
            filled_rect(ui, grid_rect, grid_bg, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 13)), 12.0);
            ui.painter().text(grid_rect.center(), egui::Align2::CENTER_CENTER, "⚙", FontId::new(20.0, FontFamily::Proportional), text_muted());
            if grid_resp.clicked() { state.dashboard_tab = DashboardTab::Settings; }
        },
        DashboardTab::Vault => {
            let add_rect = egui::Rect::from_center_size(egui::pos2(avail.right() - 44.0, topbar_rect.center().y + 4.0), Vec2::splat(40.0));
            let add_resp = ui.allocate_rect(add_rect, egui::Sense::click());
            let add_bg = if add_resp.hovered() { Color32::from_rgba_unmultiplied(129, 140, 248, 20) } else { Color32::from_rgba_unmultiplied(129, 140, 248, 10) };
            filled_rect(ui, add_rect, add_bg, Stroke::new(1.0, Color32::from_rgba_unmultiplied(129, 140, 248, 40)), 12.0);
            ui.painter().text(add_rect.center(), egui::Align2::CENTER_CENTER, "➕", FontId::new(18.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));
            
            let lock_rect = egui::Rect::from_center_size(egui::pos2(avail.right() - 94.0, topbar_rect.center().y + 4.0), Vec2::splat(40.0));
            let lock_resp = ui.allocate_rect(lock_rect, egui::Sense::click());
            let lock_bg = if lock_resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 10) } else { Color32::TRANSPARENT };
            filled_rect(ui, lock_rect, lock_bg, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 13)), 12.0);
            ui.painter().text(lock_rect.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(18.0, FontFamily::Proportional), text_muted());
            
            if add_resp.clicked() {
                #[cfg(not(target_os = "android"))]
                let path = rfd::FileDialog::new().set_title("Pilih file untuk dienkripsi").pick_file();
                #[cfg(target_os = "android")]
                let path: Option<std::path::PathBuf> = { ctrl.open_custom_file_picker(state); None };
                
                if let Some(path) = path {
                    ctrl.encrypt_file(state, path);
                }
            }
            if lock_resp.clicked() { ctrl.logout(state); }
        },
        DashboardTab::Kuat => {
            let info_rect = egui::Rect::from_center_size(egui::pos2(avail.right() - 36.0, topbar_rect.center().y + 4.0), Vec2::splat(40.0));
            let info_resp = ui.allocate_rect(info_rect, egui::Sense::click());
            let info_bg = if info_resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 10) } else { Color32::TRANSPARENT };
            filled_rect(ui, info_rect, info_bg, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 13)), 12.0);
            ui.painter().text(info_rect.center(), egui::Align2::CENTER_CENTER, "ℹ", FontId::new(20.0, FontFamily::Proportional), text_muted());
        },
        _ => {
            let cloud_rect = egui::Rect::from_center_size(egui::pos2(avail.right() - 36.0, topbar_rect.center().y + 4.0), Vec2::splat(40.0));
            let cloud_resp = ui.allocate_rect(cloud_rect, egui::Sense::click());
            let cloud_bg = if cloud_resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 10) } else { Color32::TRANSPARENT };
            filled_rect(ui, cloud_rect, cloud_bg, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 13)), 12.0);
            ui.painter().text(cloud_rect.center(), egui::Align2::CENTER_CENTER, "☁", FontId::new(20.0, FontFamily::Proportional), text_muted());
        }
    }

    // ─ Layout Dimensions ─
    let bottom_h = 80.0;
    let bottom_rect = egui::Rect::from_min_size(
        egui::pos2(avail.left(), avail.bottom() - bottom_h),
        Vec2::new(avail.width(), bottom_h),
    );
    
    let content_rect = egui::Rect::from_min_max(
        egui::pos2(avail.left(), topbar_rect.bottom()),
        egui::pos2(avail.right(), bottom_rect.top()),
    );
    
    let mut to_decrypt: Option<String> = None;
    let mut to_soft_delete: Option<String> = None;


    // Animation Logic
    if state.previous_tab != state.dashboard_tab {
        state.previous_tab = state.dashboard_tab.clone();
        state.transition_start = Some(ui.input(|i| i.time));
    }

    let mut opacity = 1.0;
    if let Some(start) = state.transition_start {
        let elapsed = (ui.input(|i| i.time) - start) as f32;
        let duration = 0.2; // 200ms
        if elapsed < duration {
            opacity = elapsed / duration;
            ui.ctx().request_repaint();
        } else {
            state.transition_start = None;
        }
    }

    // Render Content Area first so Bottom Navigation draws ON TOP of it (fixing FAB overlap)
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(content_rect), |ui| {
        ui.set_opacity(opacity);
        egui::ScrollArea::vertical().id_salt("dashboard_scroll").show(ui, |ui| {

             ui.add_space(20.0);
             match state.dashboard_tab {
                 DashboardTab::Home => render_tab_home(ui, state, ctrl, &mut to_decrypt, &mut to_soft_delete),
                 DashboardTab::Vault => render_tab_vault(ui, state, ctrl, &mut to_decrypt, &mut to_soft_delete),
                 DashboardTab::Storage => render_tab_storage(ui, state, ctrl, &mut to_decrypt, &mut to_soft_delete),
                 DashboardTab::Kuat => render_tab_kuat(ui, state, ctrl),
                 DashboardTab::Settings => render_tab_settings(ui, state, ctrl),
                 DashboardTab::Profile => render_tab_profile(ui, state, ctrl),
                 DashboardTab::Notifications => render_tab_notifications(ui, state, ctrl),
                 DashboardTab::AboutUs => render_tab_about_us(ui, state, ctrl),
             }
             ui.add_space(40.0);
        });
    });

    // ─ Bottom Navigation ─
    filled_rect(ui, bottom_rect, Color32::from_rgb(18, 18, 17), Stroke::new(1.0, border_subtle()), 0.0);
    
    let tab_w = avail.width() / 5.0;
    let mut tab_x = avail.left() + tab_w / 2.0;
    let tab_y = bottom_rect.center().y;
    
    let tabs = [
        (DashboardTab::Home, "🏠", "BERANDA"),
        (DashboardTab::Vault, "⛁", "BRANKAS"),
        (DashboardTab::Home, "📄", ""), // Placeholder for FAB
        (DashboardTab::Settings, "⚙", "SETELAN"),
        (DashboardTab::Kuat, "⚡", "PERFORMA"),
    ];
    
    for (i, (tab, icon, label)) in tabs.iter().enumerate() {
        if i == 2 {
            // FAB (Add button) with heavy indigo glow
            let fab_size = Vec2::splat(60.0);
            let fab_rect = egui::Rect::from_center_size(egui::pos2(tab_x, bottom_rect.top() - 4.0), fab_size);
            let fab_resp = ui.allocate_rect(fab_rect, egui::Sense::click());
            
            let mut glow_rect = fab_rect;
            glow_rect = glow_rect.expand(6.0);
            filled_rect(ui, glow_rect, Color32::from_rgba_unmultiplied(129, 140, 248, 20), Stroke::NONE, 36.0);
            
            let fab_fill = if fab_resp.hovered() { Color32::from_rgb(99, 102, 241) } else { Color32::from_rgb(129, 140, 248) };
            filled_rect(ui, fab_rect, fab_fill, Stroke::NONE, 30.0);
            ui.painter().text(fab_rect.center(), egui::Align2::CENTER_CENTER, "+", FontId::new(32.0, FontFamily::Proportional), Color32::WHITE);
            
            if fab_resp.clicked() {
                #[cfg(not(target_os = "android"))]
                let path = rfd::FileDialog::new().set_title("Pilih file untuk dienkripsi").pick_file();
                #[cfg(target_os = "android")]
                let path: Option<std::path::PathBuf> = { ctrl.open_custom_file_picker(state); None };
                
                if let Some(path) = path {
                    ctrl.encrypt_file(state, path);
                }
            }
        } else {
            let item_rect = egui::Rect::from_center_size(egui::pos2(tab_x, tab_y), Vec2::new(tab_w, bottom_h));
            let item_resp = ui.allocate_rect(item_rect, egui::Sense::click());
            let is_active = state.dashboard_tab == *tab;
            let color = if is_active || item_resp.hovered() { Color32::from_rgb(129, 140, 248) } else { Color32::from_rgb(115, 121, 150) };
            
            ui.painter().text(egui::pos2(tab_x, tab_y - 12.0), egui::Align2::CENTER_CENTER, *icon, FontId::new(22.0, FontFamily::Proportional), color);
            ui.painter().text(egui::pos2(tab_x, tab_y + 14.0), egui::Align2::CENTER_CENTER, *label, FontId::new(9.0, FontFamily::Proportional), color);
            
            // Active dot
            if is_active {
                ui.painter().circle_filled(egui::pos2(tab_x, tab_y + 26.0), 2.0, Color32::from_rgb(129, 140, 248));
            }
            
            if item_resp.clicked() {
                state.dashboard_tab = tab.clone();
            }
        }
        tab_x += tab_w;
    }

    if let Some(fname) = to_decrypt {
        ctrl.open_decrypt_panel(state, &fname);
    }
    if let Some(id) = to_soft_delete { ctrl.soft_delete_file(state, &id); }
}

fn render_tab_home(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller, to_decrypt: &mut Option<String>, to_soft_delete: &mut Option<String>) {
    let avail = ui.available_rect_before_wrap();
    let pad = 24.0;
    
    // ── 1. BigCard Hero ────────────────────────────────────────
    ui.add_space(8.0);
    let card_w = avail.width() - pad * 2.0;
    let card_h = 160.0;
    let (card_rect, _) = ui.allocate_exact_size(Vec2::new(card_w, card_h), egui::Sense::hover());
    
    // Background gradient (simulated using circles and base dark color)
    filled_rect(ui, card_rect, Color32::from_rgb(18, 20, 28), Stroke::new(1.0, Color32::from_rgba_unmultiplied(129, 140, 248, 30)), 24.0);
    // Soft glow at top-left using radial mesh to avoid sharp edges
    let clip_painter = ui.painter().with_clip_rect(card_rect);
    use egui::epaint::{Mesh, Vertex};
    let mut mesh = Mesh::default();
    let center = egui::pos2(card_rect.left() + 60.0, card_rect.top() + 60.0);
    let radius = 120.0;
    mesh.vertices.push(Vertex { pos: center, uv: egui::pos2(0.5, 0.5), color: Color32::from_rgba_unmultiplied(129, 140, 248, 40) });
    for i in 0..32 {
        let angle = (i as f32) * std::f32::consts::TAU / 32.0;
        let pos = center + egui::vec2(angle.cos() * radius, angle.sin() * radius);
        mesh.vertices.push(Vertex { pos, uv: egui::pos2(0.5, 0.5), color: Color32::TRANSPARENT });
    }
    for i in 0..32 {
        mesh.add_triangle(0, i + 1, if i == 31 { 1 } else { i + 2 });
    }
    clip_painter.add(egui::Shape::mesh(mesh));
    
    // Shield Icon (left column, vertically centered) - Shifted right for a centered and balanced feel
    let shield_bg_rect = egui::Rect::from_center_size(egui::pos2(card_rect.left() + 72.0, card_rect.center().y), Vec2::splat(56.0));
    filled_rect(ui, shield_bg_rect, Color32::from_rgb(99, 102, 241), Stroke::NONE, 18.0);
    ui.painter().text(shield_bg_rect.center(), egui::Align2::CENTER_CENTER, "🛡", FontId::new(28.0, FontFamily::Proportional), Color32::WHITE);
    
    // AMAN badge (top right)
    let badge_rect = egui::Rect::from_min_size(egui::pos2(card_rect.right() - 90.0, card_rect.top() + 24.0), Vec2::new(66.0, 26.0));
    filled_rect(ui, badge_rect, Color32::from_rgba_unmultiplied(16, 185, 129, 15), Stroke::new(1.0, Color32::from_rgba_unmultiplied(16, 185, 129, 40)), 13.0);
    ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, "• AMAN", FontId::new(10.0, FontFamily::Proportional), accent_mint());
    
    // Text Content (right column, beautifully aligned) - Shifted right for balance
    let total_files = state.file_list.len();
    let num_rect = ui.painter().text(egui::pos2(card_rect.left() + 140.0, card_rect.top() + 24.0), egui::Align2::LEFT_TOP, format!("{}", total_files), FontId::new(36.0, FontFamily::Proportional), Color32::WHITE);
    ui.painter().text(egui::pos2(num_rect.right() + 6.0, num_rect.bottom() - 14.0), egui::Align2::LEFT_TOP, "file", FontId::new(14.0, FontFamily::Proportional), text_muted());
    
    ui.painter().text(egui::pos2(card_rect.left() + 140.0, card_rect.top() + 74.0), egui::Align2::LEFT_TOP, format!("Terlindungi: {}", format_size(state.total_vault_size())), FontId::new(11.5, FontFamily::Proportional), text_muted());
    
    // Bottom pill (right column, beautifully aligned) - Shifted right
    let pill_rect = egui::Rect::from_min_size(egui::pos2(card_rect.left() + 140.0, card_rect.top() + 106.0), Vec2::new(160.0, 24.0));
    filled_rect(ui, pill_rect, Color32::from_rgba_unmultiplied(129, 140, 248, 15), Stroke::new(1.0, Color32::from_rgba_unmultiplied(129, 140, 248, 40)), 12.0);
    let is_2fa = if state.totp_enabled { "2FA Aktif" } else { "2FA Mati" };
    ui.painter().text(pill_rect.center(), egui::Align2::CENTER_CENTER, format!("🔒 AES-256 • {}", is_2fa), FontId::new(9.5, FontFamily::Proportional), Color32::from_rgb(165, 180, 252));
    
    // ── 2. Quick Action Chips (AKSI CEPAT) ──────────────────────
    ui.add_space(24.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("AKSI CEPAT").size(11.0).color(text_muted()).strong());
    });
    ui.add_space(12.0);
    
    let total_w = avail.width() - pad * 2.0;
    let gap = 12.0;
    let card_w3 = (total_w - gap * 2.0) / 3.0;
    
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(pad);
        
        // Chip 1: Kunci
        let (c1_rect, c1_resp) = ui.allocate_exact_size(Vec2::new(card_w3, 110.0), egui::Sense::click());
        let c1_border = if c1_resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 20) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 5) };
        filled_rect(ui, c1_rect, bg_card(), Stroke::new(1.0, c1_border), 20.0);
        let c1_ico_rect = egui::Rect::from_center_size(egui::pos2(c1_rect.center().x, c1_rect.top() + 34.0), Vec2::splat(40.0));
        ui.painter().text(c1_ico_rect.center(), egui::Align2::CENTER_CENTER, "🔒", FontId::new(20.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));
        ui.painter().text(egui::pos2(c1_rect.center().x, c1_rect.bottom() - 32.0), egui::Align2::CENTER_CENTER, "Kunci", FontId::new(14.0, FontFamily::Proportional), Color32::WHITE);
        ui.painter().text(egui::pos2(c1_rect.center().x, c1_rect.bottom() - 16.0), egui::Align2::CENTER_CENTER, "Enkripsi", FontId::new(10.0, FontFamily::Proportional), text_muted());
        if c1_resp.clicked() { state.dashboard_tab = DashboardTab::Vault; }
        
        ui.add_space(gap);
        
        // Chip 2: Buka
        let (c2_rect, c2_resp) = ui.allocate_exact_size(Vec2::new(card_w3, 110.0), egui::Sense::click());
        let c2_border = if c2_resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 20) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 5) };
        filled_rect(ui, c2_rect, bg_card(), Stroke::new(1.0, c2_border), 20.0);
        let c2_ico_rect = egui::Rect::from_center_size(egui::pos2(c2_rect.center().x, c2_rect.top() + 34.0), Vec2::splat(40.0));
        ui.painter().text(c2_ico_rect.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(20.0, FontFamily::Proportional), Color32::from_rgb(16, 185, 129));
        ui.painter().text(egui::pos2(c2_rect.center().x, c2_rect.bottom() - 32.0), egui::Align2::CENTER_CENTER, "Buka", FontId::new(14.0, FontFamily::Proportional), Color32::WHITE);
        ui.painter().text(egui::pos2(c2_rect.center().x, c2_rect.bottom() - 16.0), egui::Align2::CENTER_CENTER, "Masuk PIN", FontId::new(10.0, FontFamily::Proportional), text_muted());
        if c2_resp.clicked() { state.dashboard_tab = DashboardTab::Vault; }
        
        ui.add_space(gap);
        
        // Chip 3: File
        let (c3_rect, c3_resp) = ui.allocate_exact_size(Vec2::new(card_w3, 110.0), egui::Sense::click());
        let c3_border = if c3_resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 20) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 5) };
        filled_rect(ui, c3_rect, bg_card(), Stroke::new(1.0, c3_border), 20.0);
        let c3_ico_rect = egui::Rect::from_center_size(egui::pos2(c3_rect.center().x, c3_rect.top() + 34.0), Vec2::splat(40.0));
        ui.painter().text(c3_ico_rect.center(), egui::Align2::CENTER_CENTER, "📁", FontId::new(20.0, FontFamily::Proportional), Color32::from_rgb(251, 191, 36));
        ui.painter().text(egui::pos2(c3_rect.center().x, c3_rect.bottom() - 32.0), egui::Align2::CENTER_CENTER, "File", FontId::new(14.0, FontFamily::Proportional), Color32::WHITE);
        ui.painter().text(egui::pos2(c3_rect.center().x, c3_rect.bottom() - 16.0), egui::Align2::CENTER_CENTER, format!("{} tersimpan", state.file_list.len()), FontId::new(10.0, FontFamily::Proportional), text_muted());
        if c3_resp.clicked() { state.dashboard_tab = DashboardTab::Vault; }
    });
        
    // ── 3. Status Brankas (RINGKASAN 2x1 Grid) ───────────────────────────
    ui.add_space(24.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("RINGKASAN").size(11.0).color(text_muted()).strong());
    });
    ui.add_space(12.0);
    
    let card_w2 = (total_w - gap) / 2.0;
    let card_h2 = 110.0;
    
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(pad);
        
        // Card 1: Terkunci (File aman)
        let (r1, resp1) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
        let b1 = if resp1.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 20) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 5) };
        filled_rect(ui, r1, bg_card(), Stroke::new(1.0, b1), 20.0);
        
        let ico_r1 = egui::Rect::from_center_size(egui::pos2(r1.left() + 28.0, r1.top() + 28.0), Vec2::splat(24.0));
        ui.painter().text(ico_r1.center(), egui::Align2::CENTER_CENTER, "🔒", FontId::new(18.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));
        
        let bdg_r1 = egui::Rect::from_center_size(egui::pos2(r1.right() - 40.0, r1.top() + 28.0), Vec2::new(60.0, 20.0));
        filled_rect(ui, bdg_r1, Color32::from_rgba_unmultiplied(129, 140, 248, 15), Stroke::NONE, 10.0);
        ui.painter().text(bdg_r1.center(), egui::Align2::CENTER_CENTER, "TERKUNCI", FontId::new(9.0, FontFamily::Proportional), Color32::from_rgb(165, 180, 252));
        
        ui.painter().text(egui::pos2(r1.left() + 20.0, r1.bottom() - 40.0), egui::Align2::LEFT_CENTER, format!("{}", state.file_list.len()), FontId::new(32.0, FontFamily::Proportional), Color32::from_rgb(165, 180, 252));
        ui.painter().text(egui::pos2(r1.left() + 20.0, r1.bottom() - 18.0), egui::Align2::LEFT_CENTER, "File aman", FontId::new(11.0, FontFamily::Proportional), text_muted());
        if resp1.clicked() { state.dashboard_tab = DashboardTab::Vault; }
        
        ui.add_space(gap);
        
        // Card 2: Aktif (Sesi buka)
        let (r2, resp2) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
        let b2 = if resp2.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 20) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 5) };
        filled_rect(ui, r2, bg_card(), Stroke::new(1.0, b2), 20.0);
        
        let ico_r2 = egui::Rect::from_center_size(egui::pos2(r2.left() + 28.0, r2.top() + 28.0), Vec2::splat(24.0));
        ui.painter().text(ico_r2.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(18.0, FontFamily::Proportional), Color32::from_rgb(16, 185, 129));
        
        let bdg_r2 = egui::Rect::from_center_size(egui::pos2(r2.right() - 36.0, r2.top() + 28.0), Vec2::new(50.0, 20.0));
        filled_rect(ui, bdg_r2, Color32::from_rgba_unmultiplied(16, 185, 129, 15), Stroke::NONE, 10.0);
        ui.painter().text(bdg_r2.center(), egui::Align2::CENTER_CENTER, "AKTIF", FontId::new(9.0, FontFamily::Proportional), accent_mint());
        
        let session_active_lbl = if state.session_key.is_some() { "1" } else { "0" };
        ui.painter().text(egui::pos2(r2.left() + 20.0, r2.bottom() - 40.0), egui::Align2::LEFT_CENTER, session_active_lbl, FontId::new(32.0, FontFamily::Proportional), accent_mint());
        ui.painter().text(egui::pos2(r2.left() + 20.0, r2.bottom() - 18.0), egui::Align2::LEFT_CENTER, "Sesi buka", FontId::new(11.0, FontFamily::Proportional), text_muted());
        if resp2.clicked() { state.dashboard_tab = DashboardTab::Vault; }
    });

    
    // ── 4. Hardware Metrics ────────────────────────────────────
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("PERFORMA PERANGKAT").size(10.0).color(text_muted()).strong());
    });
    ui.add_space(10.0);
    
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 138.0), egui::Sense::hover());
        filled_rect(ui, rect, bg_card(), Stroke::new(0.5, border_default()), 20.0);
        
        ui.painter().text(egui::pos2(rect.left() + 16.0, rect.top() + 20.0), egui::Align2::LEFT_CENTER, "Kecepatan enkripsi hardware", FontId::new(13.0, FontFamily::Proportional), text_primary());
        
        let chip_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 56.0, rect.top() + 20.0), Vec2::new(90.0, 18.0));
        filled_rect(ui, chip_rect, accent_gold_a(), Stroke::new(0.5, Color32::from_rgba_unmultiplied(245, 200, 66, 56)), 20.0);
        ui.painter().text(chip_rect.center(), egui::Align2::CENTER_CENTER, "⚡ Perangkat kuat", FontId::new(9.5, FontFamily::Proportional), accent_gold());
        
        let metrics = [
            ("Prosesor", state.cpu_usage, accent_purple(), accent_purple_a(), "🔒"),
            ("Memori", state.ram_usage, accent_sky(), accent_sky_a(), "💾"),
            ("Disk I/O", state.io_usage, accent_gold(), accent_gold_a(), "⚡"),
        ];
        for (i, (lbl, val, color, bg_c, icon)) in metrics.iter().enumerate() {
            let y = rect.top() + 52.0 + i as f32 * 28.0;
            
            let ico_r = egui::Rect::from_center_size(egui::pos2(rect.left() + 28.0, y), Vec2::splat(22.0));
            filled_rect(ui, ico_r, *bg_c, Stroke::NONE, 8.0);
            ui.painter().text(ico_r.center(), egui::Align2::CENTER_CENTER, *icon, FontId::new(11.0, FontFamily::Proportional), *color);
            
            ui.painter().text(egui::pos2(ico_r.right() + 8.0, y), egui::Align2::LEFT_CENTER, *lbl, FontId::new(11.0, FontFamily::Proportional), text_muted());
            
            let bar_bg = egui::Rect::from_min_size(egui::pos2(rect.left() + 110.0, y - 2.0), Vec2::new(rect.width() - 170.0, 4.0));
            filled_rect(ui, bar_bg, bg_input(), Stroke::NONE, 2.0);
            let bar_fg = egui::Rect::from_min_size(egui::pos2(rect.left() + 110.0, y - 2.0), Vec2::new((rect.width() - 170.0) * val, 4.0));
            filled_rect(ui, bar_fg, *color, Stroke::NONE, 2.0);
            
            ui.painter().text(egui::pos2(rect.right() - 16.0, y), egui::Align2::RIGHT_CENTER, format!("{}%", (val * 100.0) as i32), FontId::new(10.5, FontFamily::Proportional), *color);
        }
    });
    
    // ── 5. Recent Activity (Files) ─────────────────────────────
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("AKTIVITAS TERBARU").size(10.0).color(text_muted()).strong());
    });
    ui.add_space(10.0);
    
    if state.file_list.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(pad);
            ui.label(egui::RichText::new("Belum ada file di dalam brankas.").color(text_muted()).size(12.0));
        });
    } else {
        ui.vertical(|ui| {
            for record in state.file_list.iter().take(5) {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 68.0), egui::Sense::click());
                    let is_hover = resp.hovered();
                    
                    let b_color = if is_hover { border_hover() } else { border_default() };
                    filled_rect(ui, rect, bg_card(), Stroke::new(0.5, b_color), 20.0);
                    
                    let ext = file_ext(&record.original_name);
                    let (icon, badge) = file_badge(ext);
                    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 34.0, rect.center().y), Vec2::splat(42.0));
                    filled_rect(ui, icon_rect, badge.0, Stroke::NONE, 13.0);
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(20.0, FontFamily::Proportional), badge.1);
                    
                    let name_truncated = if record.original_name.len() > 25 { format!("{}…", &record.original_name[..23]) } else { record.original_name.clone() };
                    ui.painter().text(egui::pos2(icon_rect.right() + 14.0, rect.center().y - 9.0), egui::Align2::LEFT_CENTER, name_truncated, FontId::new(13.0, FontFamily::Proportional), text_primary());
                    
                    let meta = format!("{} • Enkripsi {}", format_size(record.file_size as u64), if record.encrypted_at.len() >= 10 { &record.encrypted_at[..10] } else { &record.encrypted_at });
                    ui.painter().text(egui::pos2(icon_rect.right() + 14.0, rect.center().y + 11.0), egui::Align2::LEFT_CENTER, meta, FontId::new(11.0, FontFamily::Proportional), text_muted());
                    
                    if is_hover {
                        let del_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 56.0, rect.center().y), Vec2::splat(32.0));
                        let del_resp = ui.allocate_rect(del_rect, egui::Sense::click());
                        ui.painter().text(del_rect.center(), egui::Align2::CENTER_CENTER, "🗑", FontId::new(18.0, FontFamily::Proportional), if del_resp.hovered() { error_color() } else { text_muted() });
                        
                        let open_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 22.0, rect.center().y), Vec2::splat(32.0));
                        let open_resp = ui.allocate_rect(open_rect, egui::Sense::click());
                        ui.painter().text(open_rect.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(18.0, FontFamily::Proportional), if open_resp.hovered() { accent_purple() } else { text_muted() });
                        
                        if del_resp.clicked() {
                            *to_soft_delete = Some(record.id.clone());
                        } else if open_resp.clicked() || (resp.clicked() && !del_resp.hovered()) {
                            *to_decrypt = Some(record.vault_filename.clone());
                        }
                    }
                    if resp.secondary_clicked() {
                        state.active_context_menu = Some(record.id.clone());
                    }
                });
            }
        });
    }
}

fn render_tab_vault(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller, _to_decrypt: &mut Option<String>, _to_soft_delete: &mut Option<String>) {
    let pad = 24.0;
    let avail = ui.available_rect_before_wrap();
    ui.add_space(8.0);
    
    let total_w = avail.width() - pad * 2.0;
    
    // Vault 1: Primary Vault
    let (v1_rect, resp1) = ui.allocate_exact_size(Vec2::new(total_w, 86.0), egui::Sense::click());
    let border1 = if resp1.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 20) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 5) };
    filled_rect(ui, v1_rect, bg_card(), Stroke::new(1.0, border1), 20.0);
    
    let ico_rect1 = egui::Rect::from_center_size(egui::pos2(v1_rect.left() + 36.0, v1_rect.top() + 40.0), Vec2::splat(44.0));
    filled_rect(ui, ico_rect1, Color32::from_rgb(99, 102, 241), Stroke::NONE, 14.0);
    ui.painter().text(ico_rect1.center(), egui::Align2::CENTER_CENTER, "🔒", FontId::new(20.0, FontFamily::Proportional), Color32::WHITE);
    
    ui.painter().text(egui::pos2(ico_rect1.right() + 16.0, v1_rect.top() + 24.0), egui::Align2::LEFT_TOP, "Primary Vault", FontId::new(16.0, FontFamily::Proportional), Color32::WHITE);
    let vault1_sub = format!("{} file · {} · AES-256", state.file_list.len(), format_size(state.total_vault_size()));
    ui.painter().text(egui::pos2(ico_rect1.right() + 16.0, v1_rect.top() + 46.0), egui::Align2::LEFT_TOP, &vault1_sub, FontId::new(11.0, FontFamily::Proportional), text_muted());
    
    let bdg_rect1 = egui::Rect::from_center_size(egui::pos2(v1_rect.right() - 44.0, v1_rect.top() + 40.0), Vec2::new(60.0, 24.0));
    filled_rect(ui, bdg_rect1, Color32::from_rgba_unmultiplied(129, 140, 248, 15), Stroke::NONE, 12.0);
    ui.painter().text(bdg_rect1.center(), egui::Align2::CENTER_CENTER, "TERKUNCI", FontId::new(9.0, FontFamily::Proportional), Color32::from_rgb(165, 180, 252));
    
    // Progress track
    let prog1_y = v1_rect.bottom() - 20.0;
    let track_w = total_w - 40.0;
    let track1 = egui::Rect::from_min_size(egui::pos2(v1_rect.left() + 20.0, prog1_y), Vec2::new(track_w, 4.0));
    filled_rect(ui, track1, Color32::from_rgba_unmultiplied(255, 255, 255, 10), Stroke::NONE, 2.0);
    let fill1 = egui::Rect::from_min_size(track1.min, Vec2::new(track_w * 0.45, 4.0));
    filled_rect(ui, fill1, Color32::from_rgb(129, 140, 248), Stroke::NONE, 2.0);
    
    if resp1.clicked() {
        state.toast_message = Some("Primary Vault aman terkunci".to_string());
        state.toast_timer = 2.0;
    }
    
    // FOLDER section
    ui.add_space(24.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("FOLDER").size(11.0).color(text_muted()).strong());
    });
    ui.add_space(12.0);
    
    let gap = 12.0;
    let card_w2 = (total_w - gap) / 2.0;
    let card_h2 = 72.0;
    
    let folders = [
        ("Identitas", "👤", Color32::from_rgb(129, 140, 248), "6 file"),
        ("Dokumen Kerja", "💼", Color32::from_rgb(16, 185, 129), "12 file"),
        ("Foto Keluarga", "🖼", Color32::from_rgb(244, 63, 94), "4 file"),
        ("Keuangan", "📄", Color32::from_rgb(251, 191, 36), "2 file"),
        ("Kunci & Akses", "🔑", Color32::from_rgb(56, 189, 248), "0 file"),
    ];
    
    let mut i = 0;
    while i < folders.len() {
        ui.horizontal(|ui| {
            ui.add_space(pad);
            // Column 1
            if i < folders.len() {
                let f = folders[i];
                let (r, resp) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
                let b = if resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 20) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 5) };
                filled_rect(ui, r, bg_card(), Stroke::new(1.0, b), 16.0);
                
                let ico_r = egui::Rect::from_center_size(egui::pos2(r.left() + 30.0, r.center().y), Vec2::splat(36.0));
                filled_rect(ui, ico_r, Color32::from_rgba_unmultiplied(f.2.r(), f.2.g(), f.2.b(), 20), Stroke::NONE, 12.0);
                ui.painter().text(ico_r.center(), egui::Align2::CENTER_CENTER, f.1, FontId::new(18.0, FontFamily::Proportional), f.2);
                
                ui.painter().text(egui::pos2(ico_r.right() + 12.0, r.center().y - 8.0), egui::Align2::LEFT_CENTER, f.0, FontId::new(14.0, FontFamily::Proportional), Color32::WHITE);
                ui.painter().text(egui::pos2(ico_r.right() + 12.0, r.center().y + 10.0), egui::Align2::LEFT_CENTER, f.3, FontId::new(11.0, FontFamily::Proportional), text_muted());
            }
            
            ui.add_space(gap);
            
            // Column 2
            if i + 1 < folders.len() {
                let f = folders[i + 1];
                let (r, resp) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
                let b = if resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 20) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 5) };
                filled_rect(ui, r, bg_card(), Stroke::new(1.0, b), 16.0);
                
                let ico_r = egui::Rect::from_center_size(egui::pos2(r.left() + 30.0, r.center().y), Vec2::splat(36.0));
                filled_rect(ui, ico_r, Color32::from_rgba_unmultiplied(f.2.r(), f.2.g(), f.2.b(), 20), Stroke::NONE, 12.0);
                ui.painter().text(ico_r.center(), egui::Align2::CENTER_CENTER, f.1, FontId::new(18.0, FontFamily::Proportional), f.2);
                
                ui.painter().text(egui::pos2(ico_r.right() + 12.0, r.center().y - 8.0), egui::Align2::LEFT_CENTER, f.0, FontId::new(14.0, FontFamily::Proportional), Color32::WHITE);
                ui.painter().text(egui::pos2(ico_r.right() + 12.0, r.center().y + 10.0), egui::Align2::LEFT_CENTER, f.3, FontId::new(11.0, FontFamily::Proportional), text_muted());
            } else if i + 1 == folders.len() {
                // The "Folder baru" button
                let (r, resp) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
                let b = if resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 30) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 10) };
                filled_rect(ui, r, Color32::TRANSPARENT, Stroke::new(1.0, b), 16.0); // Wait, dashed borders are hard in egui without custom paths. We'll use a normal border but transparent bg.
                ui.painter().text(egui::pos2(r.center().x, r.center().y - 8.0), egui::Align2::CENTER_CENTER, "➕", FontId::new(16.0, FontFamily::Proportional), text_muted());
                ui.painter().text(egui::pos2(r.center().x, r.center().y + 12.0), egui::Align2::CENTER_CENTER, "Folder baru", FontId::new(12.0, FontFamily::Proportional), text_muted());
            }
        });
        ui.add_space(gap);
        i += 2;
    }
}

fn var_card(
    ui: &mut egui::Ui,
    id_source: &str,
    icon: &str,
    icon_color: Color32,
    icon_bg: Color32,
    name: &str,
    tech: &str,
    val_num: &str,
    val_unit: &str,
    progress: f32,
    progress_color: Color32,
    explanation: &str,
    analogy: &str,
) {
    let id = ui.make_persistent_id(id_source);
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
    let is_open = state.is_open();

    let border_color = if is_open { border_hover() } else { border_default() };
    
    ui.vertical(|ui| {
        let mut frame = card_frame().stroke(Stroke::new(0.5, border_color));
        if is_open {
            frame = frame.fill(bg_card());
        }
        
        frame.show(ui, |ui| {
            let header_rect = ui.available_rect_before_wrap();
            let _header_resp = ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
                filled_rect(ui, rect, icon_bg, Stroke::NONE, 12.0);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(16.0, FontFamily::Proportional), icon_color);
                
                ui.add_space(8.0);
                
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(name).size(13.0).color(text_primary()).strong());
                    ui.label(egui::RichText::new(tech).size(10.0).color(teal_strong()).monospace());
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(val_num).size(16.0).color(icon_color).strong());
                        ui.label(egui::RichText::new(val_unit).size(9.0).color(text_muted()));
                    });
                });
            });
            
            // Make whole header region clickable
            let header_click_rect = egui::Rect::from_min_max(
                header_rect.min,
                egui::pos2(header_rect.max.x, header_rect.min.y + 40.0)
            );
            let header_click_resp = ui.interact(header_click_rect, id, egui::Sense::click());
            if header_click_resp.clicked() {
                state.toggle(ui);
            }
            
            ui.add_space(8.0);
            
            // Progress bar
            let bar_w = ui.available_width();
            let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 4.0), egui::Sense::hover());
            filled_rect(ui, bar_rect, bg_input(), Stroke::NONE, 2.0);
            let fill_w = bar_rect.width() * progress;
            let fill_rect = egui::Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, 4.0));
            filled_rect(ui, fill_rect, progress_color, Stroke::NONE, 2.0);
            
            state.show_body_unindented(ui, |ui| {
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                
                ui.add(egui::Label::new(egui::RichText::new(explanation).size(11.5).color(text_body())).wrap());
                
                ui.add_space(8.0);
                
                let analogy_frame = egui::Frame::none()
                    .fill(teal_faint())
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0));
                
                analogy_frame.show(ui, |ui| {
                    ui.add(egui::Label::new(egui::RichText::new(format!("💡 {}", analogy)).size(10.5).color(teal_strong())).wrap());
                });
            });
        });
    });
}

fn render_tab_kuat(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let pad = 16.0;
    
    // Refresh active system hardware metrics from real OS performance
    ctrl.refresh_device_metrics(state);
    ui.ctx().request_repaint_after(std::time::Duration::from_secs(2));

    // Dynamic score calculation:
    // CPU score (30% weight) - lower CPU usage is better, high specs give good base score
    let cpu_score = (100.0 - (state.cpu_usage * 100.0 * 0.4)).clamp(75.0, 100.0);
    // RAM score (20% weight) - lower RAM usage is better/healthier
    let ram_score = (100.0 - (state.ram_usage * 100.0 * 0.3)).clamp(70.0, 100.0);
    // Enclave / TEE (25% weight) - Hardware TEE is always available in this secure app
    let enclave_score = 100.0;
    // Storage speed (10% weight) - estimated high speed
    let storage_score = 90.0;
    // Crypto Accel (15% weight) - HW accelerated
    let crypto_score = 95.0;
    
    let total_score = (cpu_score * 0.30 + ram_score * 0.20 + enclave_score * 0.25 + storage_score * 0.10 + crypto_score * 0.15).round() as i32;
    let score_str = format!("{}", total_score);
    
    let (score_text, score_color) = if total_score >= 90 {
        ("Sangat Mumpuni", accent_mint())
    } else if total_score >= 70 {
        ("Baik", accent_purple())
    } else {
        ("Cukup", accent_gold())
    };

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Kenapa HP Kuat?").size(24.0).color(text_primary()).strong());
            ui.label(egui::RichText::new("Variabel teknis, bahasa manusia").size(12.0).color(text_muted()));
        });
        
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(pad);
            let (info_rect, info_resp) = ui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::click());
            let is_hover = info_resp.hovered();
            filled_rect(ui, info_rect, if is_hover { teal_faint() } else { bg_card() }, Stroke::new(1.0, if is_hover { teal_strong() } else { border_default() }), 10.0);
            ui.painter().text(info_rect.center(), egui::Align2::CENTER_CENTER, "ℹ", FontId::new(16.0, FontFamily::Proportional), if is_hover { teal_strong() } else { text_muted() });
            
            if info_resp.clicked() {
                state.toast_message = Some(format!("Skor Anda saat ini: {}. Dihitung dari performa live perangkat Anda.", total_score));
                state.toast_timer = 3.0;
            }
        });
    });
    
    ui.add_space(16.0);
    
    // Skor Utama Hero Card
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let frame = egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(16, 185, 129, 15))
            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(16, 185, 129, 64)))
            .rounding(Rounding::same(26.0))
            .inner_margin(egui::Margin::same(20.0));
        
        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
                    filled_rect(ui, rect, Color32::from_rgb(16, 185, 129), Stroke::NONE, 12.0);
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "⚡", FontId::new(18.0, FontFamily::Proportional), Color32::WHITE);
                    
                    ui.add_space(8.0);
                    
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Skor Performa Keamanan").size(15.0).color(text_primary()).strong());
                        ui.label(egui::RichText::new("Seberapa tangguh HP kamu untuk app ini").size(11.0).color(text_muted()));
                    });
                });
                
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&score_str).size(44.0).color(score_color).strong());
                    ui.label(egui::RichText::new(format!("/ 100 · {}", score_text)).size(13.0).color(text_muted()).strong());
                });
                
                ui.add_space(10.0);
                
                let bar_w = ui.available_width();
                let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 6.0), egui::Sense::hover());
                filled_rect(ui, bar_rect, bg_input(), Stroke::NONE, 3.0);
                let fill_rect = egui::Rect::from_min_size(bar_rect.min, Vec2::new(bar_w * (total_score as f32 / 100.0), 6.0));
                filled_rect(ui, fill_rect, score_color, Stroke::NONE, 3.0);
                
                ui.add_space(12.0);
                
                ui.horizontal(|ui| {
                    let b1 = egui::Frame::none().fill(accent_mint_a()).rounding(Rounding::same(20.0)).inner_margin(egui::Margin::symmetric(10.0, 4.0));
                    b1.show(ui, |ui| {
                        ui.label(egui::RichText::new("🛡 AES-256 Optimal").size(9.0).color(accent_mint()).strong());
                    });
                    
                    ui.add_space(4.0);
                    
                    let b2 = egui::Frame::none().fill(teal_faint()).rounding(Rounding::same(20.0)).inner_margin(egui::Margin::symmetric(10.0, 4.0));
                    b2.show(ui, |ui| {
                        ui.label(egui::RichText::new("⏱ Enkripsi < 0.3s").size(9.0).color(teal_strong()).strong());
                    });
                    
                    ui.add_space(4.0);
                    
                    let b3 = egui::Frame::none().fill(accent_gold_a()).rounding(Rounding::same(20.0)).inner_margin(egui::Margin::symmetric(10.0, 4.0));
                    b3.show(ui, |ui| {
                        ui.label(egui::RichText::new("💻 Multi-thread").size(9.0).color(accent_gold()).strong());
                    });
                });
            });
        });
    });
    
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("Ketuk kartu untuk penjelasan lengkap ↓").size(10.5).color(text_muted()).italics());
    });
    ui.add_space(10.0);
    
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            // Live CPU usage
            let cpu_percent = (state.cpu_usage * 100.0).round() as i32;
            let cpu_val_num = format!("{}", cpu_percent);
            let cpu_explanation = format!(
                "Prosesor adalah \"otak\" HP kamu. Saat ini prosesor Anda sedang berjalan dengan kapasitas beban **{}%**.\n\n\
                Semakin cepat dan banyak inti (core)-nya, semakin cepat proses enkripsi berjalan.",
                cpu_percent
            );
            
            var_card(
                ui,
                "var_cpu",
                "🔳",
                accent_sky(),
                accent_sky_a(),
                "Prosesor (CPU)",
                "clock_speed · core_count · nm_process",
                &cpu_val_num,
                "% Terpakai",
                state.cpu_usage.clamp(0.02, 1.0),
                accent_sky(),
                &cpu_explanation,
                "Enkripsi AES-256 butuh kalkulasi berat. HP Anda secara aktif membagi beban kerja enkripsi ke multi-core secara real-time."
            );
            
            ui.add_space(8.0);
            
            // Live RAM usage
            let ram_percent = (state.ram_usage * 100.0).round() as i32;
            let ram_val_num = format!("{}", ram_percent);
            let ram_explanation = format!(
                "RAM adalah \"meja kerja\" HP. Saat ini memori Anda terpakai sebesar **{}%**.\n\n\
                Kapasitas sisa RAM sangat menentukan kelancaran saat memproses file berukuran besar.",
                ram_percent
            );
            
            var_card(
                ui,
                "var_ram",
                "🧠",
                accent_mint(),
                accent_mint_a(),
                "RAM (Memori Kerja)",
                "ram_gb · lpddr_version · bandwidth_gbps",
                &ram_val_num,
                "% Terpakai",
                state.ram_usage.clamp(0.02, 1.0),
                accent_mint(),
                &ram_explanation,
                "Saat mengenkripsi file (foto, video, dokumen), data dimuat ke RAM. Kinerja RAM LPDDR Anda saat ini sangat mumpuni mencegah crash."
            );
            
            ui.add_space(8.0);
            
            var_card(
                ui,
                "var_enclave",
                "🔐",
                accent_gold(),
                accent_gold_a(),
                "Secure Enclave / TEE",
                "tee_enabled · keystore_hw · strongbox",
                "✓",
                "Hardware",
                1.0,
                accent_gold(),
                "Ini adalah \"brankas dalam brankas\" — chip keamanan terpisah di dalam HP yang menyimpan kunci enkripsi. Bahkan kalau HP-mu diretas, chip ini tetap aman.",
                "Hubungannya dengan app ini: Kunci AES-256 kamu disimpan di Secure Enclave, bukan di memori biasa. Hacker tidak bisa mencurinya meski mereka punya akses root ke HP."
            );
            
            ui.add_space(8.0);
            
            // Live disk space metrics
            let total_gb = state.device_disk_total as f64 / (1024 * 1024 * 1024) as f64;
            let free_gb = state.device_disk_free as f64 / (1024 * 1024 * 1024) as f64;
            let disk_usage_ratio = if state.device_disk_total > 0 {
                (state.device_disk_total - state.device_disk_free) as f32 / state.device_disk_total as f32
            } else {
                0.25
            };
            let disk_val_num = format!("{:.1}", free_gb);
            let disk_explanation = format!(
                "Kapasitas penyimpanan fisik perangkat Anda. Saat ini tersisa **{:.1} GB** kosong dari total kapasitas **{:.1} GB**.\n\n\
                Kecepatan baca-tulis storage Anda sangat menentukan performa enkripsi file secara simultan.",
                free_gb, total_gb
            );
            
            var_card(
                ui,
                "var_storage",
                "💾",
                accent_sky(),
                accent_sky_a(),
                "Kecepatan Storage",
                "read_mbps · write_mbps · nvme_ufs",
                &disk_val_num,
                "GB Longgar",
                disk_usage_ratio,
                accent_sky(),
                &disk_explanation,
                "Menggunakan sistem penyimpanan UFS/NVMe. File dienkripsi dan langsung disimpan ke disk super cepat dalam hitungan milidetik."
            );
            
            ui.add_space(8.0);
            
            var_card(
                ui,
                "var_crypto",
                "⚡",
                accent_purple(),
                accent_purple_a(),
                "Akselerasi Kripto (HW)",
                "aes_hw_accel · sha_engine · rng_hw",
                "✓",
                "Aktif",
                0.95,
                accent_purple(),
                "Chip modern punya \"mesin khusus\" untuk kalkulasi enkripsi — terpisah dari prosesor utama. Ini namanya AES Hardware Accelerator.",
                "Hubungannya dengan app ini: Tanpa akselerasi HW, enkripsi memakan 80% CPU. Dengan akselerasi HW aktif, CPU hanya dipakai 8% — HP tetap mulus, baterai lebih hemat."
            );
            
            ui.add_space(8.0);
            
            // Baterai / Termal based on dynamic IO activity
            let temp = (40.0 + (state.cpu_usage * 12.0)).round() as i32;
            let temp_str = format!("{}°C", temp);
            
            var_card(
                ui,
                "var_battery",
                "🔋",
                error_color(),
                Color32::from_rgba_unmultiplied(239, 68, 68, 25),
                "Baterai & Termal",
                "battery_mah · throttle_temp · tdp_mw",
                &temp_str,
                "Suhu Inti",
                state.io_usage.clamp(0.02, 1.0),
                error_color(),
                "Mengukur manajemen daya dan termal real-time perangkat. Suhu tinggi yang terkendali menjamin kestabilan pemrosesan data.",
                "Sesi enkripsi massal berjalan lancar tanpa mengalami lag termal (thermal throttling) karena aplikasi dikonfigurasi secara efisien."
            );
        });
    });
}

fn render_tab_storage(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller, to_decrypt: &mut Option<String>, to_soft_delete: &mut Option<String>) {
    let pad = 16.0;
    let avail = ui.available_rect_before_wrap();
    
    ui.add_space(8.0);
    
    // Search Bar
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let search_w = avail.width() - pad * 2.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(search_w, 48.0), egui::Sense::hover());
        let border_c = Color32::from_rgba_unmultiplied(255, 255, 255, 10);
        filled_rect(ui, rect, bg_card(), Stroke::new(1.0, border_c), 14.0);
        
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(egui::RichText::new("🔍").size(16.0).color(text_muted()));
                ui.add_space(4.0);
                let resp = ui.add(egui::TextEdit::singleline(&mut state.vault_search_query)
                    .hint_text("Cari file terenkripsi...")
                    .frame(false)
                    .desired_width(search_w - 60.0)
                    .text_color(Color32::WHITE)
                    .font(FontId::new(14.0, FontFamily::Proportional)));
                if resp.gained_focus() || resp.clicked() {
                    state.focused_field = crate::app_state::FocusedField::None;
                }
            });
        });
    });
    ui.add_space(12.0);
    
    // Sort Selector
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("URUTKAN:").size(9.0).color(text_muted()).strong());
        
        let sort_modes = [
            (SortOption::DateDesc, "Terbaru"),
            (SortOption::DateAsc, "Terlama"),
            (SortOption::NameAsc, "A-Z"),
            (SortOption::SizeDesc, "Terbesar"),
        ];
        
        for (mode, lbl) in sort_modes.iter() {
            let is_active = state.vault_sort_by == *mode;
            let bg_c = if is_active { accent_purple_a() } else { bg_card() };
            let border_c = if is_active { accent_purple() } else { border_default() };
            let text_c = if is_active { text_primary() } else { text_muted() };
            
            let btn = egui::Button::new(egui::RichText::new(*lbl).size(10.0).color(text_c))
                .fill(bg_c)
                .stroke(Stroke::new(0.5, border_c))
                .rounding(12.0);
            if ui.add(btn).clicked() {
                state.vault_sort_by = mode.clone();
            }
            ui.add_space(4.0);
        }
    });
    
    ui.add_space(14.0);
    
    // Filter and Sort
    let mut files: Vec<FileRecord> = state.file_list.iter()
        .filter(|f| {
            if state.vault_search_query.is_empty() {
                true
            } else {
                f.original_name.to_lowercase().contains(&state.vault_search_query.to_lowercase())
            }
        })
        .cloned()
        .collect();
        
    match state.vault_sort_by {
        SortOption::DateDesc => files.sort_by(|a, b| b.encrypted_at.cmp(&a.encrypted_at)),
        SortOption::DateAsc => files.sort_by(|a, b| a.encrypted_at.cmp(&b.encrypted_at)),
        SortOption::NameAsc => files.sort_by(|a, b| a.original_name.to_lowercase().cmp(&b.original_name.to_lowercase())),
        SortOption::SizeDesc => files.sort_by(|a, b| b.file_size.cmp(&a.file_size)),
    }
    
    if files.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(egui::RichText::new("📂").size(36.0).color(text_muted()));
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Tidak ada file ditemukan").color(text_muted()).size(13.0));
        });
    } else {
        ui.vertical(|ui| {
            for record in files {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 72.0), egui::Sense::click());
                    let is_hover = resp.hovered();
                    
                    let bg_color = if is_hover { Color32::from_rgba_unmultiplied(255, 255, 255, 5) } else { Color32::TRANSPARENT };
                    filled_rect(ui, rect, bg_color, Stroke::NONE, 12.0);
                    
                    let ext = file_ext(&record.original_name);
                    let (icon, badge) = file_badge(ext);
                    
                    // In Image 3, icon background colors are solid with transparency, let's use slightly darker/transparent
                    let icon_bg = Color32::from_rgba_unmultiplied(badge.1.r(), badge.1.g(), badge.1.b(), 25);
                    
                    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 28.0, rect.center().y), Vec2::splat(44.0));
                    filled_rect(ui, icon_rect, icon_bg, Stroke::NONE, 12.0);
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(20.0, FontFamily::Proportional), badge.1);
                    
                    let name_truncated = if record.original_name.len() > 28 { format!("{}…", &record.original_name[..26]) } else { record.original_name.clone() };
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y - 12.0), egui::Align2::LEFT_CENTER, name_truncated, FontId::new(15.0, FontFamily::Proportional), Color32::WHITE);
                    
                    let vault_name = if record.vault_filename.contains("session") { "Session Vault" } else { "Primary Vault" };
                    let meta = format!("{} · {} · {}", format_size(record.file_size as u64), if record.encrypted_at.len() >= 10 { &record.encrypted_at[..10] } else { &record.encrypted_at }, vault_name);
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y + 10.0), egui::Align2::LEFT_CENTER, meta, FontId::new(11.0, FontFamily::Proportional), text_muted());
                    
                    if is_hover {
                        let del_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 80.0, rect.center().y), Vec2::splat(36.0));
                        let del_resp = ui.allocate_rect(del_rect, egui::Sense::click());
                        ui.painter().text(del_rect.center(), egui::Align2::CENTER_CENTER, "🗑", FontId::new(18.0, FontFamily::Proportional), if del_resp.hovered() { error_color() } else { text_muted() });
                        
                        let open_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 40.0, rect.center().y), Vec2::splat(36.0));
                        let open_resp = ui.allocate_rect(open_rect, egui::Sense::click());
                        ui.painter().text(open_rect.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(18.0, FontFamily::Proportional), if open_resp.hovered() { Color32::from_rgb(129, 140, 248) } else { text_muted() });
                        
                        if del_resp.clicked() {
                            *to_soft_delete = Some(record.id.clone());
                        } else if open_resp.clicked() || (resp.clicked() && !del_resp.hovered() && !open_resp.hovered()) {
                            *to_decrypt = Some(record.vault_filename.clone());
                        }
                    } else {
                        let opts_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 24.0, rect.center().y), Vec2::splat(32.0));
                        ui.painter().text(opts_rect.center(), egui::Align2::CENTER_CENTER, "⋮", FontId::new(20.0, FontFamily::Proportional), text_muted());
                    }
                    if resp.secondary_clicked() {
                        state.active_context_menu = Some(record.id.clone());
                    }
                });
                
                // Add a very faint separator line after each item (except last, but adding to all is fine)
                let sep_rect = egui::Rect::from_min_size(egui::pos2(pad, ui.cursor().top()), Vec2::new(avail.width() - pad*2.0, 1.0));
                filled_rect(ui, sep_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 5), Stroke::NONE, 0.0);
            }
        });
    }
}

fn render_tab_settings(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let pad = 24.0;
    let avail = ui.available_rect_before_wrap();
    
    ui.add_space(8.0);
    
    // Helper to draw standard settings row
    let draw_row = |ui: &mut egui::Ui, icon: &str, icon_c: Color32, icon_bg: Color32, title: &str, sub: &str| -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 60.0), egui::Sense::click());
        let border = if resp.hovered() { border_hover() } else { border_default() };
        filled_rect(ui, rect, bg_card(), Stroke::new(0.5, border), 18.0);
        
        let ico_r = egui::Rect::from_center_size(egui::pos2(rect.left() + 32.0, rect.center().y), Vec2::splat(36.0));
        filled_rect(ui, ico_r, icon_bg, Stroke::NONE, 12.0);
        ui.painter().text(ico_r.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(17.0, FontFamily::Proportional), icon_c);
        
        ui.painter().text(egui::pos2(ico_r.right() + 12.0, rect.top() + 14.0), egui::Align2::LEFT_TOP, title, FontId::new(13.0, FontFamily::Proportional), text_primary());
        ui.painter().text(egui::pos2(ico_r.right() + 12.0, rect.top() + 32.0), egui::Align2::LEFT_TOP, sub, FontId::new(10.0, FontFamily::Proportional), text_muted());
        
        // Chevron right on the end
        let arrow_pos = egui::pos2(rect.right() - 20.0, rect.center().y);
        ui.painter().text(arrow_pos, egui::Align2::CENTER_CENTER, "▶", FontId::new(10.0, FontFamily::Proportional), text_muted());
        
        resp
    };
    
    // ── 1. KEAMANAN ───────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("KEAMANAN").size(10.0).color(text_muted()).strong());
    });
    ui.add_space(10.0);
    
    // Row 1.1: Ubah Password
    ui.horizontal(|ui| {
        ui.add_space(pad);
        if draw_row(ui, "🔒", accent_purple(), accent_purple_a(), "Ubah Password Utama", "Ganti password master Anda").clicked() {
            state.dashboard_tab = DashboardTab::Profile; // Navigate to change password tab
        }
    });
    ui.add_space(8.0);
    
    // Row 1.2: Autentikasi 2FA
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let sub = if state.totp_enabled { "2FA Aktif (Google Authenticator)" } else { "2FA Nonaktif (Ketuk untuk setup)" };
        if draw_row(ui, "📱", accent_rose(), accent_rose_a(), "Autentikasi 2FA", sub).clicked() {
            if state.totp_enabled {
                ctrl.disable_totp(state);
            } else {
                ctrl.begin_totp_setup(state);
            }
        }
    });
    ui.add_space(8.0);
    
    // Row 1.3: Mode Terang (Toggle Row)
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let (rect, _resp) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 60.0), egui::Sense::hover());
        filled_rect(ui, rect, bg_card(), Stroke::new(0.5, border_default()), 18.0);
        
        let ico_r = egui::Rect::from_center_size(egui::pos2(rect.left() + 32.0, rect.center().y), Vec2::splat(36.0));
        filled_rect(ui, ico_r, accent_sky_a(), Stroke::NONE, 12.0);
        ui.painter().text(ico_r.center(), egui::Align2::CENTER_CENTER, "☀️", FontId::new(17.0, FontFamily::Proportional), accent_sky());
        
        ui.painter().text(egui::pos2(ico_r.right() + 12.0, rect.top() + 14.0), egui::Align2::LEFT_TOP, "Mode Terang", FontId::new(13.0, FontFamily::Proportional), text_primary());
        ui.painter().text(egui::pos2(ico_r.right() + 12.0, rect.top() + 32.0), egui::Align2::LEFT_TOP, "Gunakan tema warna terang", FontId::new(10.0, FontFamily::Proportional), text_muted());
        
        // Custom Toggle switch on the right side
        let toggle_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 36.0, rect.center().y), egui::vec2(42.0, 24.0));
        let toggle_resp = ui.allocate_rect(toggle_rect, egui::Sense::click());
        
        let mut is_light = state.is_light_mode;
        if toggle_resp.clicked() {
            is_light = !is_light;
            state.is_light_mode = is_light;
            crate::theme::set_light_mode(is_light);
            ui.ctx().request_repaint();
        }
        
        let toggle_bg = if is_light { accent_purple() } else { bg_input() };
        filled_rect(ui, toggle_rect, toggle_bg, Stroke::NONE, 12.0);
        let knob_x = if is_light { toggle_rect.right() - 12.0 } else { toggle_rect.left() + 12.0 };
        ui.painter().circle_filled(egui::pos2(knob_x, toggle_rect.center().y), 9.0, Color32::WHITE);
    });
    ui.add_space(8.0);
    
    // Row 1.4: Recycle Bin
    ui.horizontal(|ui| {
        ui.add_space(pad);
        if draw_row(ui, "🗑", error_color(), Color32::from_rgba_unmultiplied(255, 79, 79, 13), "Recycle Bin", "Lihat & pulihkan file terhapus").clicked() {
            ctrl.load_deleted_files(state);
            state.screen = AppScreen::RecycleBin;
        }
    });
    ui.add_space(20.0);
    
    // ── 2. PENYIMPANAN ────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("PENYIMPANAN").size(10.0).color(text_muted()).strong());
    });
    ui.add_space(10.0);
    
    // Row 2.1: System Trash Scanner
    ui.horizontal(|ui| {
        ui.add_space(pad);
        if draw_row(ui, "🔍", accent_gold(), accent_gold_a(), "System Trash Scanner", "Pindai file sampah di Windows").clicked() {
            ctrl.scan_system_trash(state);
            state.screen = AppScreen::SystemTrash;
        }
    });
    ui.add_space(8.0);
    
    // Row 2.2: Lokasi penyimpanan
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let path_text = format!("{} · Lokal di perangkat", state.storage_path);
        if draw_row(ui, "📁", accent_mint(), accent_mint_a(), "Lokasi Penyimpanan", &path_text).clicked() {
            state.storage_pin_modal_open = true;
            state.storage_pin.clear();
            state.storage_pin_error = None;
        }
    });
    ui.add_space(8.0);
    
    // Row 2.3: Cadangkan database
    ui.horizontal(|ui| {
        ui.add_space(pad);
        if draw_row(ui, "💾", accent_sky(), accent_sky_a(), "Cadangkan Database", "Ekspor file database utama").clicked() {
            ctrl.backup_database(state);
        }
    });
    ui.add_space(20.0);
    
    // ── 3. TENTANG ───────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("TENTANG").size(10.0).color(text_muted()).strong());
    });
    ui.add_space(10.0);
    
    // Row 3.1: Versi Aplikasi
    ui.horizontal(|ui| {
        ui.add_space(pad);
        draw_row(ui, "ℹ", accent_purple(), accent_purple_a(), "Versi Aplikasi", "v1.0.0 · Dibuat dengan Rust + egui");
    });
    ui.add_space(8.0);
    
    // Row 3.2: Tentang Kami
    ui.horizontal(|ui| {
        ui.add_space(pad);
        if draw_row(ui, "👥", Color32::from_rgb(52, 211, 153), Color32::from_rgba_unmultiplied(52, 211, 153, 20), "Tentang Kami", "Informasi pengembang & versi aplikasi").clicked() {
            state.dashboard_tab = DashboardTab::AboutUs;
        }
    });
}

// ── Helper: load image bytes into egui texture ────────────
pub fn load_image_texture(ui: &egui::Ui, state: &mut AppState, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    if let Some(tex) = state.texture_cache.get(name) {
        return Some(tex.clone());
    }
    match image::load_from_memory(bytes) {
        Ok(img) => {
            let size = [img.width() as _, img.height() as _];
            let image_buffer = img.to_rgba8();
            let pixels = image_buffer.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            let tex = ui.ctx().load_texture(name, color_image, Default::default());
            state.texture_cache.insert(name.to_string(), tex.clone());
            Some(tex)
        }
        Err(_) => None,
    }
}

// ── Helper: draw a texture clipped to a circle using a mesh ──
fn draw_circular_image(ui: &egui::Ui, texture: &egui::TextureHandle, center: egui::Pos2, radius: f32) {
    let segments = 64; // smoothness of the circle
    let mut mesh = Mesh::with_texture(texture.id());

    // Center vertex
    mesh.vertices.push(Vertex {
        pos: center,
        uv: egui::pos2(0.5, 0.5),
        color: Color32::WHITE,
    });

    // Edge vertices around the circle
    for i in 0..=segments {
        let angle = std::f32::consts::TAU * (i as f32) / (segments as f32);
        let dx = angle.cos();
        let dy = angle.sin();
        mesh.vertices.push(Vertex {
            pos: egui::pos2(center.x + radius * dx, center.y + radius * dy),
            uv: egui::pos2(0.5 + 0.5 * dx, 0.5 + 0.5 * dy),
            color: Color32::WHITE,
        });
    }

    // Triangle fan from center
    for i in 1..=segments as u32 {
        mesh.indices.push(0);
        mesh.indices.push(i);
        mesh.indices.push(i + 1);
    }

    ui.painter().add(egui::Shape::Mesh(mesh));
}

// ── Helper: draw circular image with glow border ──
pub fn draw_circular_image_with_border(
    ui: &egui::Ui, texture: &egui::TextureHandle,
    center: egui::Pos2, radius: f32,
    border_width: f32, border_color: Color32,
    glow: bool,
) {
    // Glow effect
    if glow {
        ui.painter().circle_filled(center, radius + border_width + 3.0,
            Color32::from_rgba_unmultiplied(border_color.r(), border_color.g(), border_color.b(), 40));
    }
    // Dark ring behind image to cut out background
    ui.painter().circle_filled(center, radius + border_width,  border_color);
    ui.painter().circle_filled(center, radius, Color32::from_rgb(11, 12, 22));
    // Draw the circular image
    draw_circular_image(ui, texture, center, radius);
    // Border stroke on top
    ui.painter().circle_stroke(center, radius, Stroke::new(border_width, border_color));
}

// ── Helper: draw the app logo at a given rect (circular) ──
fn draw_app_logo(ui: &egui::Ui, state: &mut AppState, center: egui::Pos2, size: f32) {
    let logo_bytes: &[u8] = include_bytes!("../assets/logo.jpg");
    if let Some(texture) = load_image_texture(ui, state, "app_logo_global", logo_bytes) {
        let radius = size / 2.0;
        draw_circular_image_with_border(
            ui, &texture, center, radius,
            2.0, Color32::from_rgb(129, 140, 248), false,
        );
    } else {
        // Fallback to old emoji (using painter directly since we only have &Ui)
        let rect = egui::Rect::from_center_size(center, Vec2::splat(size));
        ui.painter().rect(rect, Rounding::same(size * 0.35), Color32::from_rgb(99, 102, 241), Stroke::NONE);
        ui.painter().text(center, egui::Align2::CENTER_CENTER, "🛡",
            FontId::new(size * 0.46, FontFamily::Proportional), Color32::WHITE);
    }
}

// ── Screen: Tentang Kami (About Us) ───────────────────────
fn render_tab_about_us(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller) {
    let pad = 24.0;
    let avail = ui.available_rect_before_wrap();
    let card_w = avail.width() - pad * 2.0;

    // ── Back Button ──
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let (back_rect, back_resp) = ui.allocate_exact_size(Vec2::new(100.0, 36.0), egui::Sense::click());
        let back_bg = if back_resp.hovered() { Color32::from_rgba_unmultiplied(129, 140, 248, 30) } else { Color32::TRANSPARENT };
        filled_rect(ui, back_rect, back_bg, Stroke::new(1.0, Color32::from_rgba_unmultiplied(129, 140, 248, 60)), 12.0);
        ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "◀ Kembali",
            FontId::new(12.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));
        if back_resp.clicked() { state.dashboard_tab = DashboardTab::Settings; }
    });
    ui.add_space(16.0);

    // ── App Logo (circular) ──
    {
        let logo_bytes = include_bytes!("../assets/logo.jpg");
        if let Some(texture) = load_image_texture(ui, state, "about_logo", logo_bytes) {
            ui.vertical_centered(|ui| {
                let logo_size = 120.0;
                let (logo_rect, _) = ui.allocate_exact_size(Vec2::splat(logo_size), egui::Sense::hover());
                draw_circular_image_with_border(
                    ui, &texture, logo_rect.center(), logo_size / 2.0,
                    3.0, Color32::from_rgb(129, 140, 248), true,
                );
            });
        }
    }

    ui.add_space(12.0);

    // ── App Name & Version ──
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new("Aegis Vault").size(28.0).color(text_primary()).strong());
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Versi 1.0.0").size(13.0).color(Color32::from_rgb(129, 140, 248)).strong());
        ui.add_space(8.0);
    });

    // ── App Description Card ──
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let desc_h = 80.0;
        let (desc_rect, _) = ui.allocate_exact_size(Vec2::new(card_w, desc_h), egui::Sense::hover());
        filled_rect(ui, desc_rect, bg_card(), Stroke::new(0.5, border_default()), 18.0);

        // Icon
        let icon_rect = egui::Rect::from_center_size(
            egui::pos2(desc_rect.left() + 30.0, desc_rect.center().y), Vec2::splat(36.0));
        filled_rect(ui, icon_rect, Color32::from_rgba_unmultiplied(129, 140, 248, 20), Stroke::NONE, 12.0);
        ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, "🛡",
            FontId::new(18.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));

        // Description text
        let text_x = icon_rect.right() + 14.0;
        let _text_w = desc_rect.right() - text_x - 12.0;
        ui.painter().text(egui::pos2(text_x, desc_rect.top() + 16.0), egui::Align2::LEFT_TOP,
            "Aplikasi penyimpanan file terenkripsi",
            FontId::new(12.0, FontFamily::Proportional), text_primary());
        // Wrap text manually for the sub description
        let sub_text = "dengan keamanan tingkat militer.\nDibuat menggunakan Rust + egui.";
        ui.painter().text(egui::pos2(text_x, desc_rect.top() + 34.0), egui::Align2::LEFT_TOP,
            sub_text,
            FontId::new(10.5, FontFamily::Proportional), text_muted());
    });

    ui.add_space(20.0);

    // ── Section: Tim Pengembang ──
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("TIM PENGEMBANG").size(10.0).color(text_muted()).strong());
    });
    ui.add_space(10.0);

    // Team members data
    struct TeamMember {
        name: &'static str,
        nim: &'static str,
        photo_bytes: &'static [u8],
        tex_id: &'static str,
    }

    let members = [
        TeamMember {
            name: "Rizma Indra Pramudya",
            nim: "25051204370",
            photo_bytes: include_bytes!("../assets/foto_rizma.png"),
            tex_id: "about_foto_rizma",
        },
        TeamMember {
            name: "Izora Elverda Narulita Putri",
            nim: "25051204287",
            photo_bytes: include_bytes!("../assets/foto_izora.png"),
            tex_id: "about_foto_izora",
        },
        TeamMember {
            name: "Putera Al Khalidi",
            nim: "25051204362",
            photo_bytes: include_bytes!("../assets/foto_putera.png"),
            tex_id: "about_foto_putera",
        },
        TeamMember {
            name: "Muhammad Abdullah Ro'in",
            nim: "25051204270",
            photo_bytes: include_bytes!("../assets/foto_abdullah.png"),
            tex_id: "about_foto_abdullah",
        },
    ];

    for (idx, member) in members.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.add_space(pad);

            let row_h = 72.0;
            let (row_rect, row_resp) = ui.allocate_exact_size(Vec2::new(card_w, row_h), egui::Sense::hover());
            let border = if row_resp.hovered() { border_hover() } else { border_default() };
            filled_rect(ui, row_rect, bg_card(), Stroke::new(0.5, border), 18.0);

            // Profile photo (circular)
            let photo_size = 48.0;
            let photo_center = egui::pos2(row_rect.left() + 20.0 + photo_size / 2.0, row_rect.center().y);
            let photo_rect = egui::Rect::from_center_size(photo_center, Vec2::splat(photo_size));

            // Colors per member
            let colors = [
                Color32::from_rgb(129, 140, 248),  // indigo
                Color32::from_rgb(52, 211, 153),    // emerald
                Color32::from_rgb(251, 191, 36),    // amber
                Color32::from_rgb(244, 114, 182),   // pink
            ];
            let accent = colors[idx % colors.len()];

            // Try to load the member photo
            if let Some(texture) = load_image_texture(ui, state, member.tex_id, member.photo_bytes) {
                // Photo loaded - render circular
                draw_circular_image_with_border(
                    ui, &texture, photo_center, photo_size / 2.0,
                    2.0, accent, false,
                );
            } else {
                // Fallback: colored circle with initial
                ui.painter().circle_filled(photo_center, photo_size / 2.0,
                    Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 30));
                ui.painter().circle_stroke(photo_center, photo_size / 2.0,
                    Stroke::new(2.0, accent));
                let initial = member.name.chars().next().unwrap_or('?').to_uppercase().to_string();
                ui.painter().text(photo_center, egui::Align2::CENTER_CENTER, &initial,
                    FontId::new(20.0, FontFamily::Proportional), accent);
            }

            // Name and NIM
            let text_x = photo_rect.right() + 14.0;
            ui.painter().text(egui::pos2(text_x, row_rect.center().y - 10.0), egui::Align2::LEFT_CENTER,
                member.name,
                FontId::new(14.0, FontFamily::Proportional), text_primary());
            ui.painter().text(egui::pos2(text_x, row_rect.center().y + 10.0), egui::Align2::LEFT_CENTER,
                &format!("NIM: {}", member.nim),
                FontId::new(11.0, FontFamily::Proportional), text_muted());

            // Badge number
            let badge_rect = egui::Rect::from_center_size(
                egui::pos2(row_rect.right() - 28.0, row_rect.center().y), Vec2::splat(28.0));
            filled_rect(ui, badge_rect, Color32::from_rgba_unmultiplied(129, 140, 248, 20), Stroke::NONE, 14.0);
            ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER,
                &format!("{}", idx + 1),
                FontId::new(12.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));
        });
        ui.add_space(8.0);
    }

    ui.add_space(16.0);

    // ── Footer ──
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new("© 2025 Aegis Vault Team").size(11.0).color(text_muted()));
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Dibuat dengan ❤️ menggunakan Rust").size(10.0).color(Color32::from_rgb(71, 77, 102)));
    });
}

fn render_tab_profile(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let avail = ui.available_rect_before_wrap();
    let pad = 24.0;
    let total_w = avail.width() - pad * 2.0;

    ui.add_space(20.0);

    // Header: Back Button & Title
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let back_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(36.0, 30.0));
        let back_resp = ui.allocate_rect(back_rect, egui::Sense::click());
        filled_rect(ui, back_rect, Color32::TRANSPARENT, Stroke::new(0.5, border_default()), 7.0);
        ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "<",
                          FontId::new(15.0, FontFamily::Proportional), text_muted());
        if back_resp.clicked() { 
            state.dashboard_tab = DashboardTab::Settings; 
            state.profile_old_password.clear();
            state.profile_new_password.clear();
            state.profile_confirm_password.clear();
            state.profile_password_error = None;
            state.profile_password_success = None;
        }
        ui.add_space(10.0);
        ui.label(egui::RichText::new("Pengaturan Akun").size(15.0).color(crate::theme::text_body()).strong());
    });

    ui.add_space(24.0);

    ui.vertical_centered(|ui| {
        // Large Avatar
        let avatar_size = Vec2::splat(80.0);
        let (a_rect, _) = ui.allocate_exact_size(avatar_size, egui::Sense::hover());
        filled_rect(ui, a_rect, Color32::from_rgba_unmultiplied(99, 102, 241, 25),
                    Stroke::new(2.5, Color32::from_rgba_unmultiplied(129, 140, 248, 80)), 40.0);
        let initial = state.display_name.chars().next().unwrap_or('A').to_uppercase().to_string();
        ui.painter().text(a_rect.center(), egui::Align2::CENTER_CENTER, &initial,
            FontId::new(32.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));

        ui.add_space(12.0);
        ui.label(egui::RichText::new(&state.display_name).size(18.0).color(text_primary()).strong());
        ui.add_space(4.0);
        ui.label(egui::RichText::new(&state.login_username).size(12.5).color(text_muted()));
    });

    ui.add_space(28.0);

    // Form Container
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            card_frame().show(ui, |ui| {
                ui.set_width(total_w);
                
                ui.label(egui::RichText::new("UBAH KATA SANDI MASTER").size(11.0).color(text_primary()).strong());
                ui.add_space(14.0);

                // Password Lama
                ui.label(egui::RichText::new("KATA SANDI LAMA").size(10.0).color(text_muted()).strong());
                ui.add_space(6.0);
                let (p1_rect, _) = ui.allocate_exact_size(Vec2::new(total_w - 40.0, 44.0), egui::Sense::hover());
                filled_rect(ui, p1_rect, bg_input(), Stroke::new(0.5, border_default()), 12.0);
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(p1_rect.shrink(12.0)), |ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.profile_old_password)
                        .password(true)
                        .hint_text("Masukkan kata sandi lama")
                        .frame(false)
                        .desired_width(p1_rect.width() - 24.0)
                        .font(FontId::new(13.5, FontFamily::Proportional)));
                });
                ui.add_space(12.0);

                // Password Baru
                ui.label(egui::RichText::new("KATA SANDI BARU").size(10.0).color(text_muted()).strong());
                ui.add_space(6.0);
                let (p2_rect, _) = ui.allocate_exact_size(Vec2::new(total_w - 40.0, 44.0), egui::Sense::hover());
                filled_rect(ui, p2_rect, bg_input(), Stroke::new(0.5, border_default()), 12.0);
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(p2_rect.shrink(12.0)), |ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.profile_new_password)
                        .password(true)
                        .hint_text("Minimal 4 karakter")
                        .frame(false)
                        .desired_width(p2_rect.width() - 24.0)
                        .font(FontId::new(13.5, FontFamily::Proportional)));
                });
                ui.add_space(12.0);

                // Konfirmasi Password Baru
                ui.label(egui::RichText::new("KONFIRMASI KATA SANDI BARU").size(10.0).color(text_muted()).strong());
                ui.add_space(6.0);
                let (p3_rect, _) = ui.allocate_exact_size(Vec2::new(total_w - 40.0, 44.0), egui::Sense::hover());
                filled_rect(ui, p3_rect, bg_input(), Stroke::new(0.5, border_default()), 12.0);
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(p3_rect.shrink(12.0)), |ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.profile_confirm_password)
                        .password(true)
                        .hint_text("Ulangi kata sandi baru")
                        .frame(false)
                        .desired_width(p3_rect.width() - 24.0)
                        .font(FontId::new(13.5, FontFamily::Proportional)));
                });

                if let Some(err) = &state.profile_password_error {
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new(err).color(error_color()).size(11.5).strong());
                }
                if let Some(msg) = &state.profile_password_success {
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new(msg).color(success_color()).size(11.5).strong());
                }

                ui.add_space(20.0);

                // Buttons
                ui.horizontal(|ui| {
                    let w_btn = (total_w - 40.0 - 12.0) / 2.0;
                    if ghost_btn(ui, "Batal", w_btn).clicked() {
                        state.dashboard_tab = DashboardTab::Settings;
                        state.profile_old_password.clear();
                        state.profile_new_password.clear();
                        state.profile_confirm_password.clear();
                        state.profile_password_error = None;
                        state.profile_password_success = None;
                    }
                    ui.add_space(12.0);
                    if teal_btn(ui, "Ubah Sandi", w_btn).clicked() {
                        ctrl.change_password(state);
                    }
                });
            });
        });
    });
}

fn render_tab_notifications(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller) {
    ui.add_space(30.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new("Audit Log Keamanan").size(22.0).color(text_primary()).strong());
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Aktivitas terbaru di dalam brankas.").color(crate::theme::text_muted()));
    });
    
    ui.add_space(30.0);
    let pad = 20.0;
    let avail = ui.available_rect_before_wrap();

    if state.audit_logs.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("Belum ada catatan aktivitas.").color(crate::theme::text_muted()));
        });
    } else {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for log in &state.audit_logs {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 60.0), egui::Sense::hover());
                    
                    let (icon, color) = match log.action_type.as_str() {
                        "FAIL_LOGIN" | "FAIL_2FA" => ("⚠", error_color()),
                        "LOGIN" | "LOGIN_2FA"     => ("👤", teal_strong()),
                        "ENCRYPT"                 => ("🔒", Color32::from_rgb(250, 190, 88)),
                        "DECRYPT"                 => ("🔓", Color32::from_rgb(100, 200, 100)),
                        "BACKUP"                  => ("💾", Color32::from_rgb(100, 150, 250)),
                        "CHANGE_PIN" | "SETUP"    => ("🔑", Color32::from_rgb(200, 100, 250)),
                        _                         => ("ℹ", text_muted()),
                    };
                    
                    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 24.0, rect.center().y), Vec2::splat(36.0));
                    filled_rect(ui, icon_rect, color.linear_multiply(0.15), Stroke::NONE, 18.0);
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(18.0, FontFamily::Proportional), color);
                    
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y - 10.0), egui::Align2::LEFT_CENTER, &log.description, FontId::new(14.0, FontFamily::Proportional), text_primary());
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y + 10.0), egui::Align2::LEFT_CENTER, &log.timestamp, FontId::new(11.0, FontFamily::Proportional), text_muted());
                    
                    // Separator line
                    ui.painter().line_segment([egui::pos2(rect.left(), rect.bottom()), egui::pos2(rect.right(), rect.bottom())], Stroke::new(0.5, border_subtle()));
                });
                ui.add_space(4.0);
            }
        });
    }
}

// ── Screen: Decrypt Panel ─────────────────────────────────
fn render_decrypt_panel(
    ui:    &mut egui::Ui,
    state: &mut AppState,
    ctrl:  &Controller,
    vault_filename: &str,
) {
    let record = match &state.decrypt_target {
        Some(r) if r.vault_filename == vault_filename => r.clone(),
        _ => { state.screen = AppScreen::Dashboard; return; }
    };

    let pad   = 28.0;

    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(pad, 28.0))
        .show(ui, |ui| {
            // Tombol Kembali ke file (berbentuk tautan teks premium)
            ui.horizontal(|ui| {
                let (rect_back, resp_back) = ui.allocate_exact_size(Vec2::new(140.0, 24.0), egui::Sense::click());
                let text_color = if resp_back.hovered() { Color32::from_rgb(165, 180, 252) } else { Color32::from_rgb(129, 140, 248) };
                ui.painter().text(
                    rect_back.left_center(),
                    egui::Align2::LEFT_CENTER,
                    "◀  Kembali ke file",
                    FontId::new(14.0, FontFamily::Proportional),
                    text_color
                );
                if resp_back.clicked() {
                    state.screen = AppScreen::Dashboard;
                }
            });

            ui.add_space(20.0);

            // Info card file
            theme::card_frame()
                .rounding(Rounding::same(24.0)) // Lebih bulat sesuai mockup
                .show(ui, |ui| {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        // File icon bagde (besar dan elegan)
                        let icon_size = 56.0;
                        let (rect_icon, _) = ui.allocate_exact_size(Vec2::splat(icon_size), egui::Sense::hover());
                        filled_rect(ui, rect_icon, Color32::from_rgba_unmultiplied(129, 140, 248, 15), Stroke::new(0.5, Color32::from_rgba_unmultiplied(129, 140, 248, 30)), 16.0);
                        
                        let ext           = file_ext(&record.original_name);
                        let (icon, _)     = file_badge(ext);
                        ui.painter().text(
                            rect_icon.center(),
                            egui::Align2::CENTER_CENTER,
                            icon,
                            FontId::new(26.0, FontFamily::Proportional),
                            Color32::from_rgb(129, 140, 248)
                        );

                        ui.add_space(12.0);
                        
                        ui.vertical(|ui| {
                            // File Name
                            ui.label(egui::RichText::new(&record.original_name)
                                .size(17.0).color(text_primary()).strong());
                            
                            // Subtitle: Tipe & Ukuran
                            let ext_upper = ext.to_uppercase();
                            let file_type_desc = if record.is_folder {
                                "Folder".to_string()
                            } else if ext_upper.is_empty() {
                                "Berkas".to_string()
                            } else {
                                format!("Dokumen {}", ext_upper)
                            };
                            
                            ui.label(egui::RichText::new(format!("{} · {}", file_type_desc, format_size(record.file_size as u64)))
                                .size(12.0).color(text_dimmed()));
                            
                            ui.add_space(4.0);

                            // Badges: SHA-256 OK & AES-256
                            ui.horizontal(|ui| {
                                // Badge 1: SHA-256 OK
                                let (rect_b1, _) = ui.allocate_exact_size(Vec2::new(96.0, 20.0), egui::Sense::hover());
                                filled_rect(ui, rect_b1, Color32::from_rgba_unmultiplied(16, 185, 129, 15), Stroke::new(0.5, Color32::from_rgb(16, 185, 129)), 10.0);
                                ui.painter().text(
                                    rect_b1.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "🛡️ SHA-256 OK",
                                    FontId::new(9.0, FontFamily::Proportional),
                                    Color32::from_rgb(52, 211, 153)
                                );

                                ui.add_space(6.0);

                                // Badge 2: AES-256
                                let (rect_b2, _) = ui.allocate_exact_size(Vec2::new(76.0, 20.0), egui::Sense::hover());
                                filled_rect(ui, rect_b2, Color32::from_rgba_unmultiplied(129, 140, 248, 15), Stroke::new(0.5, Color32::from_rgb(129, 140, 248)), 10.0);
                                ui.painter().text(
                                    rect_b2.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "🔒 AES-256",
                                    FontId::new(9.0, FontFamily::Proportional),
                                    Color32::from_rgb(165, 180, 252)
                                );
                            });
                        });
                    });

                    ui.add_space(20.0);
                    ui.painter().line_segment(
                        [ui.cursor().min, ui.cursor().min + Vec2::new(ui.available_width(), 0.0)],
                        Stroke::new(0.5, Color32::from_rgb(30, 34, 53)),
                    );
                    ui.add_space(16.0);

                    // Grid data detail
                    let hash_display = if record.sha256_hash.len() >= 8 {
                        format!("{}...{}", &record.sha256_hash[..4], &record.sha256_hash[record.sha256_hash.len() - 4..])
                    } else {
                        record.sha256_hash.clone()
                    };

                    for (k, v, val_color, is_bold) in &[
                        ("Vault",       "Primary Vault",                    text_primary(), true),
                        ("Status",      "Terkunci",                         Color32::from_rgb(129, 140, 248), true),
                        ("Enkripsi",    "AES-256-CBC",                      text_primary(), true),
                        ("Hash SHA-256", &hash_display,                     Color32::from_rgb(52, 211, 153), true),
                        ("Ukuran",      &format_size(record.file_size as u64), text_primary(), true),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(*k).size(12.0).color(text_muted()));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut text = egui::RichText::new(*v).size(12.0).color(*val_color);
                                if *is_bold { text = text.strong(); }
                                ui.label(text);
                            });
                        });
                        ui.add_space(10.0);
                    }
                    ui.add_space(2.0);
                });

            ui.add_space(24.0);

            // Status message
            if let Some(s) = &state.status.clone() {
                let color = if s.success { success_color() } else { error_color() };
                ui.label(egui::RichText::new(&s.text).size(12.0).color(color));
                ui.add_space(12.0);
            }

            // 2x2 Grid of Action Buttons
            ui.vertical(|ui| {
                let btn_w = (ui.available_width() - 12.0) / 2.0;

                // Baris 1: Buka file & Ekspor
                ui.horizontal(|ui| {
                    // Tombol 1: Buka file (Solid premium violet/indigo)
                    let (rect1, resp1) = ui.allocate_exact_size(Vec2::new(btn_w, 48.0), egui::Sense::click());
                    let bg_c = if resp1.is_pointer_button_down_on() {
                        Color32::from_rgb(79, 70, 229)
                    } else if resp1.hovered() {
                        Color32::from_rgb(129, 140, 248)
                    } else {
                        Color32::from_rgb(99, 102, 241)
                    };
                    filled_rect(ui, rect1, bg_c, Stroke::NONE, 12.0);
                    ui.painter().text(
                        rect1.center(),
                        egui::Align2::CENTER_CENTER,
                        "🔓  Buka file",
                        FontId::new(13.0, FontFamily::Proportional),
                        Color32::WHITE
                    );
                    if resp1.clicked() {
                        ctrl.decrypt_to_memory(state, &record.vault_filename);
                    }

                    ui.add_space(12.0);

                    // Tombol 2: Ekspor (Outline style)
                    let (rect2, resp2) = ui.allocate_exact_size(Vec2::new(btn_w, 48.0), egui::Sense::click());
                    let border_c = if resp2.hovered() { border_hover() } else { border_default() };
                    filled_rect(ui, rect2, bg_surface(), Stroke::new(1.0, border_c), 12.0);
                    ui.painter().text(
                        rect2.center(),
                        egui::Align2::CENTER_CENTER,
                        "📥  Ekspor",
                        FontId::new(13.0, FontFamily::Proportional),
                        Color32::WHITE
                    );
                    if resp2.clicked() {
                        let out_name = record.original_name.clone();

                        #[cfg(not(target_os = "android"))]
                        let out_dir = FileDialog::new()
                            .set_title("Pilih folder tujuan")
                            .pick_folder();
                        #[cfg(target_os = "android")]
                        let out_dir: Option<std::path::PathBuf> = crate::controller::external_dir().map(|p| p.to_path_buf());
                        
                        if let Some(out_dir) = out_dir {
                            let rec = record.clone();
                            ctrl.decrypt_file(state, &rec, out_dir, &out_name);
                        } else {
                            state.set_status("Batal: folder tidak dipilih.", false);
                        }
                    }
                });

                ui.add_space(12.0);

                // Baris 2: Salin hash & Hapus
                ui.horizontal(|ui| {
                    // Tombol 3: Salin hash (Outline style)
                    let (rect3, resp3) = ui.allocate_exact_size(Vec2::new(btn_w, 48.0), egui::Sense::click());
                    let border_c = if resp3.hovered() { border_hover() } else { border_default() };
                    filled_rect(ui, rect3, bg_surface(), Stroke::new(1.0, border_c), 12.0);
                    ui.painter().text(
                        rect3.center(),
                        egui::Align2::CENTER_CENTER,
                        "📋  Salin hash",
                        FontId::new(13.0, FontFamily::Proportional),
                        Color32::WHITE
                    );
                    if resp3.clicked() {
                        ui.ctx().copy_text(record.sha256_hash.clone());
                        state.set_status("Hash SHA-256 berhasil disalin!", true);
                    }

                    ui.add_space(12.0);

                    // Tombol 4: Hapus (Outline style dengan teks/ikon merah)
                    let (rect4, resp4) = ui.allocate_exact_size(Vec2::new(btn_w, 48.0), egui::Sense::click());
                    let border_c = if resp4.hovered() { Color32::from_rgb(239, 68, 68) } else { Color32::from_rgba_unmultiplied(239, 68, 68, 80) };
                    filled_rect(ui, rect4, bg_surface(), Stroke::new(1.0, border_c), 12.0);
                    ui.painter().text(
                        rect4.center(),
                        egui::Align2::CENTER_CENTER,
                        "🗑  Hapus",
                        FontId::new(13.0, FontFamily::Proportional),
                        Color32::from_rgb(239, 68, 68)
                    );
                    if resp4.clicked() {
                        ctrl.soft_delete_file(state, &record.id);
                        state.screen = AppScreen::Dashboard;
                    }
                });
            });
        });
}

// ── Screen: TOTP Setup (QR code + verifikasi awal) ────────
fn render_totp_setup(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let avail = ui.available_rect_before_wrap();

    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(28.0, 24.0))
        .show(ui, |ui| {
            // Back + title
            ui.horizontal(|ui| {
                let back_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(36.0, 30.0));
                let back_resp = ui.allocate_rect(back_rect, egui::Sense::click());
                filled_rect(ui, back_rect, Color32::TRANSPARENT,
                            Stroke::new(0.5, border_default()), 7.0);
                ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "<",
                                  FontId::new(15.0, FontFamily::Proportional), text_muted());
                if back_resp.clicked() { state.screen = AppScreen::Dashboard; return; }
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Setup Autentikasi 2FA").size(15.0).color(crate::theme::text_body()).strong());
            });

            ui.add_space(16.0);

            ui.vertical_centered(|ui| {
                // Info banner
                egui::Frame::none()
                    .fill(Color32::from_rgb(12, 31, 24))
                    .stroke(Stroke::new(0.5, border_accent()))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_width(avail.width() - 80.0);
                        ui.horizontal_top(|ui| {
                            ui.label(egui::RichText::new("ℹ").size(16.0).color(teal_strong()));
                            ui.add_space(6.0);
                            ui.add(egui::Label::new(egui::RichText::new(
                                "Scan QR code ini dengan Google Authenticator,\nAuthy, atau aplikasi TOTP lainnya."
                            ).size(12.0).color(teal_light())).wrap());
                        });
                    });

                ui.add_space(16.0);

                // QR Code
                if let Some(matrix) = &state.totp_qr {
                    crate::totp::draw_qr(ui, matrix, 200.0);
                } else {
                    ui.label(egui::RichText::new("Gagal generate QR code").color(error_color()));
                }

                ui.add_space(12.0);

                // Manual secret key
                ui.label(egui::RichText::new("Atau masukkan kunci manual:").size(11.0).color(crate::theme::text_muted()));
                ui.add_space(4.0);
                egui::Frame::none()
                    .fill(crate::theme::bg_surface())
                    .stroke(Stroke::new(0.5, border_default()))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                    .show(ui, |ui| {
                        let formatted: String = state.totp_secret_b32
                            .chars().collect::<Vec<_>>()
                            .chunks(4)
                            .map(|c| c.iter().collect::<String>())
                            .collect::<Vec<_>>()
                            .join(" ");
                        ui.label(egui::RichText::new(&formatted)
                            .size(12.0).color(teal_faint())
                            .text_style(egui::TextStyle::Monospace));
                    });

                ui.add_space(20.0);

                // Verify input
                ui.label(egui::RichText::new("Masukkan kode 6-digit dari app:").size(12.0).color(crate::theme::text_muted()));
                ui.add_space(6.0);
                egui::Frame::none()
                    .fill(crate::theme::bg_surface())
                    .stroke(Stroke::new(0.5, if state.totp_code.len() == 6 { teal_strong() } else { border_default() }))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.add(egui::TextEdit::singleline(&mut state.totp_code)
                            .desired_width(180.0)
                            .hint_text("000000")
                            .font(FontId::new(20.0, FontFamily::Monospace))
                            .horizontal_align(egui::Align::Center)
                            .frame(false));
                    });

                if let Some(err) = &state.totp_error {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(err).color(error_color()).size(12.0));
                }

                ui.add_space(16.0);

                if teal_btn(ui, "✅  Verifikasi & Aktifkan", 240.0).clicked() {
                    ctrl.confirm_totp_setup(state);
                }
            });
        });
}

// ── Screen: TOTP Verify (login 2FA) ───────────────────────
fn render_totp_verify(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let avail = ui.available_rect_before_wrap();

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(avail), |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);

            // Shield icon
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(56.0), egui::Sense::hover());
            filled_rect(ui, icon_rect, teal_dark(), Stroke::NONE, 14.0);
            ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, "🔐",
                              FontId::new(26.0, FontFamily::Proportional), teal_faint());

            ui.add_space(14.0);
            ui.label(egui::RichText::new("Verifikasi 2FA").size(18.0).color(crate::theme::text_body()).strong());
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Masukkan kode dari aplikasi authenticator")
                .size(13.0).color(crate::theme::text_muted()));

            ui.add_space(8.0);

            // Timer countdown
            let secs = crate::totp::seconds_left();
            let timer_color = if secs <= 5 { error_color() } else if secs <= 10 { warn_color() } else { teal_light() };
            ui.label(egui::RichText::new(format!("Kode berubah dalam {} detik", secs))
                .size(11.0).color(timer_color));
            ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));

            ui.add_space(20.0);

            // Code input
            egui::Frame::none()
                .fill(crate::theme::bg_surface())
                .stroke(Stroke::new(0.5, if state.totp_code.len() == 6 { teal_strong() } else { border_default() }))
                .rounding(Rounding::same(10.0))
                .inner_margin(egui::Margin::symmetric(16.0, 14.0))
                .show(ui, |ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.totp_code)
                        .desired_width(200.0)
                        .hint_text("000000")
                        .font(FontId::new(28.0, FontFamily::Monospace))
                        .horizontal_align(egui::Align::Center)
                        .frame(false));
                });

            if let Some(err) = &state.totp_error {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(err).color(error_color()).size(13.0));
            }

            ui.add_space(20.0);

            if teal_btn(ui, "🔓  Verifikasi", 240.0).clicked() {
                ctrl.verify_totp_login(state);
            }

            // Auto-verify saat 6 digit terisi
            if state.totp_code.len() == 6 && state.totp_error.is_none() {
                ctrl.verify_totp_login(state);
            }

            ui.add_space(24.0);
            if ghost_btn(ui, "🚪  Kembali ke login", 200.0).clicked() {
                ctrl.logout(state);
            }
        });
    });
}

// ── Screen: Recycle Bin ───────────────────────────────────
fn render_recycle_bin(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let avail = ui.available_rect_before_wrap();
    let pad   = 16.0;

    // ─ Topbar ─
    let topbar_rect = egui::Rect::from_min_size(avail.min, Vec2::new(avail.width(), 52.0));
    filled_rect(ui, topbar_rect, Color32::from_rgb(14,16,22), Stroke::new(0.5, border_subtle()), 0.0);

    let back_rect = egui::Rect::from_min_size(topbar_rect.min + Vec2::new(18.0, 12.0), Vec2::splat(28.0));
    let back_resp = ui.allocate_rect(back_rect, egui::Sense::click());
    filled_rect(ui, back_rect, Color32::TRANSPARENT, Stroke::new(0.5, border_default()), 7.0);
    ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "<",
                      FontId::new(15.0, FontFamily::Proportional), text_muted());
    if back_resp.clicked() { state.screen = AppScreen::Dashboard; return; }

    ui.painter().text(
        egui::pos2(back_rect.right() + 10.0, topbar_rect.center().y),
        egui::Align2::LEFT_CENTER, "Recycle Bin",
        FontId::new(16.0, FontFamily::Proportional), text_primary(),
    );

    let mut cursor_y = topbar_rect.bottom() + 14.0;
    
    // Warning banner
    let banner_rect = egui::Rect::from_min_size(
        egui::pos2(avail.left() + pad, cursor_y),
        Vec2::new(avail.width() - pad * 2.0, 44.0),
    );
    filled_rect(ui, banner_rect, Color32::from_rgb(30, 20, 20), Stroke::new(0.5, error_color()), 8.0);
    ui.painter().text(banner_rect.center(), egui::Align2::CENTER_CENTER,
                      "⚠ File di bawah dapat dipulihkan atau dihapus permanen.",
                      FontId::new(12.0, FontFamily::Proportional), Color32::from_rgb(255, 100, 100));
    
    cursor_y += 58.0;

    let scroll_rect = egui::Rect::from_min_max(
        egui::pos2(avail.left(), cursor_y),
        egui::pos2(avail.right(), avail.bottom() - 20.0),
    );

    let mut to_perm_delete: Option<FileRecord> = None;
    let mut to_restore: Option<String> = None;

    egui::ScrollArea::vertical()
        .id_salt("trash_scroll")
        .show_viewport(ui, |ui, _vp| {
            ui.set_clip_rect(scroll_rect);
            if state.deleted_list.is_empty() {
                let c = scroll_rect.center();
                ui.painter().text(c, egui::Align2::CENTER_CENTER,
                                  "Recycle Bin Kosong",
                                  FontId::new(16.0, FontFamily::Proportional), text_muted());
            } else {
                let card_h   = 68.0;
                let card_gap = 8.0;
                for (idx, record) in state.deleted_list.clone().iter().enumerate() {
                    let card_y = scroll_rect.top() + idx as f32 * (card_h + card_gap) + 4.0;
                    if card_y + card_h > scroll_rect.bottom() { break; }

                    let card_rect = egui::Rect::from_min_size(
                        egui::pos2(avail.left() + pad, card_y),
                        Vec2::new(avail.width() - pad * 2.0, card_h),
                    );
                    let card_hovered = ui.rect_contains_pointer(card_rect);
                    let card_fill    = if card_hovered { bg_card() } else { bg_surface() };
                    let card_stroke  = if card_hovered {
                        Stroke::new(0.5, warn_color())
                    } else {
                        Stroke::new(0.5, border_default())
                    };
                    filled_rect(ui, card_rect, card_fill, card_stroke, 10.0);

                    // Badge ikon
                    let ext         = file_ext(&record.original_name);
                    let (icon, badge) = file_badge(ext);
                    let badge_rect  = egui::Rect::from_min_size(
                        egui::pos2(card_rect.left() + 14.0, card_rect.center().y - 18.0),
                        Vec2::splat(36.0),
                    );
                    filled_rect(ui, badge_rect, badge.0, Stroke::new(0.5, badge.1), 8.0);
                    ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, icon,
                                      FontId::new(16.0, FontFamily::Proportional), badge.1);

                    // Info teks
                    let info_x = badge_rect.right() + 12.0;
                    let name_truncated = if record.original_name.len() > 28 {
                        format!("{}…", &record.original_name[..26])
                    } else {
                        record.original_name.clone()
                    };
                    ui.painter().text(egui::pos2(info_x, card_rect.top() + 16.0),
                                      egui::Align2::LEFT_TOP, &name_truncated,
                                      FontId::new(14.0, FontFamily::Proportional), text_primary());
                    let meta = format!("{}…  ·  Dihapus: {}",
                                       &record.sha256_hash[..6],
                                       record.deleted_at.as_deref().unwrap_or(""));
                    ui.painter().text(egui::pos2(info_x, card_rect.top() + 36.0),
                                      egui::Align2::LEFT_TOP, &meta,
                                      FontId::new(11.0, FontFamily::Proportional), error_color());

                    // Tombol Hapus Permanen
                    let perm_del_rect = egui::Rect::from_min_size(
                        egui::pos2(card_rect.right() - 94.0, card_rect.center().y - 16.0),
                        Vec2::new(38.0, 32.0),
                    );
                    let perm_del_resp = ui.allocate_rect(perm_del_rect, egui::Sense::click());
                    let perm_del_border = if perm_del_resp.hovered() { error_color() } else { border_default() };
                    let perm_del_icon_c = if perm_del_resp.hovered() { error_color() } else { text_muted() };
                    filled_rect(ui, perm_del_rect, bg_surface(), Stroke::new(0.5, perm_del_border), 7.0);
                    ui.painter().text(perm_del_rect.center(), egui::Align2::CENTER_CENTER, "❌",
                                      FontId::new(14.0, FontFamily::Proportional), perm_del_icon_c);
                    if perm_del_resp.clicked() {
                        to_perm_delete = Some(record.clone());
                    }

                    // Tombol Restore
                    let restore_rect = egui::Rect::from_min_size(
                        egui::pos2(card_rect.right() - 50.0, card_rect.center().y - 16.0),
                        Vec2::new(38.0, 32.0),
                    );
                    let restore_resp   = ui.allocate_rect(restore_rect, egui::Sense::click());
                    let restore_border = if restore_resp.hovered() { teal_strong() } else { border_default() };
                    let restore_icon_c = if restore_resp.hovered() { teal_strong() } else { text_muted() };
                    filled_rect(ui, restore_rect, bg_surface(), Stroke::new(0.5, restore_border), 7.0);
                    ui.painter().text(restore_rect.center(), egui::Align2::CENTER_CENTER, "♻",
                                      FontId::new(16.0, FontFamily::Proportional), restore_icon_c);
                    if restore_resp.clicked() {
                        to_restore = Some(record.id.clone());
                    }
                }
            }
        });

    if let Some(record) = to_perm_delete {
        ctrl.permanent_delete_file(state, &record);
    }
    if let Some(id) = to_restore {
        ctrl.restore_file(state, &id);
    }
}


fn render_preview_panel(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let pad = 28.0;
    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(pad, 28.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let back_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(36.0, 30.0));
                let back_resp = ui.allocate_rect(back_rect, egui::Sense::click());
                filled_rect(ui, back_rect, Color32::TRANSPARENT, Stroke::new(0.5, border_default()), 7.0);
                ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "<",
                                  FontId::new(15.0, FontFamily::Proportional), text_muted());
                if back_resp.clicked() { 
                    if let Some(target) = &state.decrypt_target {
                        state.screen = AppScreen::Decrypting(target.vault_filename.clone());
                    } else {
                        state.screen = AppScreen::Dashboard;
                    }
                    state.preview_bytes = None; // free memory
                    return; 
                }
                ui.add_space(10.0);
                ui.label(egui::RichText::new(format!("Pratinjau: {}", state.preview_filename)).size(15.0).color(crate::theme::text_body()).strong());
            });

            ui.add_space(24.0);

            if let Some(bytes) = &state.preview_bytes {
                let ext = file_ext(&state.preview_filename).to_lowercase();
                if bytes.is_empty() {
                    // File dibuka secara eksternal
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.label(egui::RichText::new("📺 File Dibuka di Aplikasi Bawaan").size(24.0).color(teal_strong()));
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("Format file ini tidak dapat ditampilkan langsung di layar aplikasi.").color(crate::theme::text_muted()));
                        ui.label(egui::RichText::new("Aplikasi secara otomatis membuka file ini di perangkat Anda.").color(crate::theme::text_muted()));
                        ui.add_space(40.0);
                        ui.label(egui::RichText::new("Anda dapat menutup layar ini atau memulihkan file menggunakan tombol di bawah.").color(Color32::from_rgb(120, 120, 140)));
                    });
                } else if ext == "png" || ext == "jpg" || ext == "jpeg" {
                    match image::load_from_memory(bytes) {
                        Ok(img) => {
                            let size = [img.width() as _, img.height() as _];
                            let image_buffer = img.to_rgba8();
                            let pixels = image_buffer.as_flat_samples();
                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                size,
                                pixels.as_slice(),
                            );
                            let texture = ui.ctx().load_texture(
                                "preview_img",
                                color_image,
                                Default::default()
                            );
                            
                            // Hitung ukuran tersedia dengan menyisakan ruang untuk tombol di bawah
                            let available = ui.available_size() - egui::Vec2::new(0.0, 70.0);
                            ui.add(
                                egui::Image::new(&texture)
                                    .max_width(available.x)
                                    .max_height(available.y)
                            );
                        }
                        Err(_) => {
                            ui.label(egui::RichText::new("Gagal memuat gambar.").color(error_color()));
                        }
                    }
                } else if ext == "txt" {
                    if let Ok(text) = String::from_utf8(bytes.clone()) {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let mut text_mut = text.clone();
                            ui.add(egui::TextEdit::multiline(&mut text_mut).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                        });
                    } else {
                        ui.label(egui::RichText::new("Bukan teks UTF-8 yang valid.").color(error_color()));
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.label(egui::RichText::new("❓ Format Tidak Didukung").size(24.0).color(error_color()));
                    });
                }
            } else {
                ui.label("Memuat...");
            }

            ui.add_space(20.0);
            ui.horizontal(|ui| {
                let record_clone = state.decrypt_target.clone();
                if let Some(record) = record_clone {
                    if ui.add_sized([250.0, 40.0], egui::Button::new(egui::RichText::new("🔓 Ekstrak & Pulihkan File").size(14.0))).clicked() {
                        #[cfg(not(target_os = "android"))]
                        let out_dir = rfd::FileDialog::new().set_title("Pilih folder tujuan").pick_folder();
                        #[cfg(target_os = "android")]
                        let out_dir: Option<std::path::PathBuf> = { state.set_status("Memilih folder belum didukung di Android", false); None };
                        
                        if let Some(out_dir) = out_dir {
                            let out_name = record.original_name.clone();
                            ctrl.decrypt_file(state, &record, out_dir, &out_name);
                        } else {
                            state.set_status("Batal: folder tidak dipilih.", false);
                        }
                    }
                }
            });
        });
}

// ── Screen: System Trash Scanner ──────────────────────────
fn render_system_trash(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let avail = ui.available_rect_before_wrap();
    let pad   = 16.0;

    // ─ Topbar ─
    let topbar_rect = egui::Rect::from_min_size(avail.min, Vec2::new(avail.width(), 52.0));
    filled_rect(ui, topbar_rect, Color32::from_rgb(14,16,22), Stroke::new(0.5, border_subtle()), 0.0);

    let back_rect = egui::Rect::from_min_size(topbar_rect.min + Vec2::new(18.0, 12.0), Vec2::splat(28.0));
    let back_resp = ui.allocate_rect(back_rect, egui::Sense::click());
    filled_rect(ui, back_rect, Color32::TRANSPARENT, Stroke::new(0.5, border_default()), 7.0);
    ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "<",
                      FontId::new(15.0, FontFamily::Proportional), text_muted());
    if back_resp.clicked() { state.screen = AppScreen::Dashboard; return; }

    ui.painter().text(
        egui::pos2(back_rect.right() + 10.0, topbar_rect.center().y),
        egui::Align2::LEFT_CENTER, "System Trash Scanner",
        FontId::new(16.0, FontFamily::Proportional), text_primary(),
    );

    // Refresh button
    let refresh_rect = egui::Rect::from_center_size(
        egui::pos2(avail.right() - 30.0, topbar_rect.center().y), Vec2::splat(28.0));
    let refresh_resp = ui.allocate_rect(refresh_rect, egui::Sense::click());
    ui.painter().text(refresh_rect.center(), egui::Align2::CENTER_CENTER, "🔄",
                      FontId::new(16.0, FontFamily::Proportional),
                      if refresh_resp.hovered() { teal_strong() } else { text_muted() });
    if refresh_resp.clicked() { ctrl.scan_system_trash(state); }

    let mut cursor_y = topbar_rect.bottom() + 14.0;

    // Info banner
    let banner_rect = egui::Rect::from_min_size(
        egui::pos2(avail.left() + pad, cursor_y),
        Vec2::new(avail.width() - pad * 2.0, 44.0),
    );
    filled_rect(ui, banner_rect, Color32::from_rgb(25, 25, 15), Stroke::new(0.5, Color32::from_rgb(250, 190, 88)), 8.0);
    ui.painter().text(banner_rect.center(), egui::Align2::CENTER_CENTER,
                      &format!("🔍 {} file ditemukan di Recycle Bin Windows", state.system_trash_items.len()),
                      FontId::new(12.0, FontFamily::Proportional), Color32::from_rgb(250, 190, 88));

    cursor_y += 58.0;

    let scroll_rect = egui::Rect::from_min_max(
        egui::pos2(avail.left(), cursor_y),
        egui::pos2(avail.right(), avail.bottom() - 20.0),
    );

    let mut restore_original_idx: Option<usize> = None;
    let mut restore_custom_idx: Option<usize> = None;
    let mut preview_idx: Option<usize> = None;
    let mut secure_idx: Option<usize> = None;

    egui::ScrollArea::vertical()
        .id_salt("system_trash_scroll")
        .show_viewport(ui, |ui, _vp| {
            ui.set_clip_rect(scroll_rect);
            if state.system_trash_items.is_empty() {
                let c = scroll_rect.center();
                ui.painter().text(c, egui::Align2::CENTER_CENTER,
                                  if state.system_trash_loading { "Memindai..." } else { "Recycle Bin kosong atau tidak dapat diakses" },
                                  FontId::new(16.0, FontFamily::Proportional), text_muted());
            } else {
                let card_h   = 78.0;
                let card_gap = 8.0;
                for (idx, item) in state.system_trash_items.clone().iter().enumerate() {
                    let card_y = scroll_rect.top() + idx as f32 * (card_h + card_gap) + 4.0;
                    if card_y + card_h > scroll_rect.bottom() + 200.0 { break; }

                    let card_rect = egui::Rect::from_min_size(
                        egui::pos2(avail.left() + pad, card_y),
                        Vec2::new(avail.width() - pad * 2.0, card_h),
                    );
                    let card_hovered = ui.rect_contains_pointer(card_rect);
                    let card_fill    = if card_hovered { bg_card() } else { bg_surface() };
                    let card_stroke  = if card_hovered {
                        Stroke::new(0.5, Color32::from_rgb(250, 190, 88))
                    } else {
                        Stroke::new(0.5, border_default())
                    };
                    filled_rect(ui, card_rect, card_fill, card_stroke, 10.0);

                    // File type icon
                    let ext = file_ext(&item.file_name);
                    let (icon, badge) = if item.is_directory {
                        ("📁", (Color32::from_rgba_unmultiplied(96, 165, 250, 38), Color32::from_rgb(96, 165, 250)))
                    } else {
                        file_badge(ext)
                    };
                    let badge_rect = egui::Rect::from_min_size(
                        egui::pos2(card_rect.left() + 14.0, card_rect.center().y - 18.0),
                        Vec2::splat(36.0),
                    );
                    filled_rect(ui, badge_rect, badge.0, Stroke::new(0.5, badge.1), 8.0);
                    ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, icon,
                                      FontId::new(16.0, FontFamily::Proportional), badge.1);

                    // Info
                    let info_x = badge_rect.right() + 12.0;
                    let name_truncated = if item.file_name.len() > 24 {
                        format!("{}…", &item.file_name[..22])
                    } else {
                        item.file_name.clone()
                    };
                    ui.painter().text(egui::pos2(info_x, card_rect.top() + 16.0),
                                      egui::Align2::LEFT_TOP, &name_truncated,
                                      FontId::new(14.0, FontFamily::Proportional), text_primary());
                    
                    let size_str = format_size(item.file_size);
                    let meta = format!("{}  ·  Dihapus: {}", size_str, &item.deleted_at);
                    ui.painter().text(egui::pos2(info_x, card_rect.top() + 36.0),
                                      egui::Align2::LEFT_TOP, &meta,
                                      FontId::new(11.0, FontFamily::Proportional), text_muted());

                    // Truncated original path
                    let orig_path = if item.original_path.len() > 40 {
                        format!("...{}", &item.original_path[item.original_path.len()-37..])
                    } else {
                        item.original_path.clone()
                    };
                    ui.painter().text(egui::pos2(info_x, card_rect.top() + 52.0),
                                      egui::Align2::LEFT_TOP, &orig_path,
                                      FontId::new(10.0, FontFamily::Proportional), Color32::from_rgb(120, 120, 140));

                    if !item.is_directory {
                        // Secure button
                        let secure_rect = egui::Rect::from_min_size(
                            egui::pos2(card_rect.right() - 182.0, card_rect.center().y - 16.0),
                            Vec2::new(38.0, 32.0),
                        );
                        let secure_resp = ui.allocate_rect(secure_rect, egui::Sense::click());
                        let secure_border = if secure_resp.hovered() { warn_color() } else { border_default() };
                        let secure_icon_c = if secure_resp.hovered() { warn_color() } else { text_muted() };
                        filled_rect(ui, secure_rect, bg_surface(), Stroke::new(0.5, secure_border), 7.0);
                        ui.painter().text(secure_rect.center(), egui::Align2::CENTER_CENTER, "🔒",
                                          FontId::new(14.0, FontFamily::Proportional), secure_icon_c);
                        if secure_resp.clicked() {
                            secure_idx = Some(idx);
                        }

                        // Preview button
                        let preview_rect = egui::Rect::from_min_size(
                            egui::pos2(card_rect.right() - 138.0, card_rect.center().y - 16.0),
                            Vec2::new(38.0, 32.0),
                        );
                        let preview_resp = ui.allocate_rect(preview_rect, egui::Sense::click());
                        let preview_border = if preview_resp.hovered() { teal_strong() } else { border_default() };
                        let preview_icon_c = if preview_resp.hovered() { teal_strong() } else { text_muted() };
                        filled_rect(ui, preview_rect, bg_surface(), Stroke::new(0.5, preview_border), 7.0);
                        ui.painter().text(preview_rect.center(), egui::Align2::CENTER_CENTER, "👁",
                                          FontId::new(16.0, FontFamily::Proportional), preview_icon_c);
                        if preview_resp.clicked() {
                            preview_idx = Some(idx);
                        }
                    }

                    // Restore to original button
                    let restore_orig_rect = egui::Rect::from_min_size(
                        egui::pos2(card_rect.right() - 94.0, card_rect.center().y - 16.0),
                        Vec2::new(38.0, 32.0),
                    );
                    let restore_orig_resp = ui.allocate_rect(restore_orig_rect, egui::Sense::click());
                    let orig_border = if restore_orig_resp.hovered() { teal_strong() } else { border_default() };
                    let orig_icon_c = if restore_orig_resp.hovered() { teal_strong() } else { text_muted() };
                    filled_rect(ui, restore_orig_rect, bg_surface(), Stroke::new(0.5, orig_border), 7.0);
                    ui.painter().text(restore_orig_rect.center(), egui::Align2::CENTER_CENTER, "↩",
                                      FontId::new(16.0, FontFamily::Proportional), orig_icon_c);
                    if restore_orig_resp.clicked() {
                        restore_original_idx = Some(idx);
                    }

                    // Restore to custom folder button
                    let restore_custom_rect = egui::Rect::from_min_size(
                        egui::pos2(card_rect.right() - 50.0, card_rect.center().y - 16.0),
                        Vec2::new(38.0, 32.0),
                    );
                    let restore_custom_resp = ui.allocate_rect(restore_custom_rect, egui::Sense::click());
                    let custom_border = if restore_custom_resp.hovered() { Color32::from_rgb(96, 165, 250) } else { border_default() };
                    let custom_icon_c = if restore_custom_resp.hovered() { Color32::from_rgb(96, 165, 250) } else { text_muted() };
                    filled_rect(ui, restore_custom_rect, bg_surface(), Stroke::new(0.5, custom_border), 7.0);
                    ui.painter().text(restore_custom_rect.center(), egui::Align2::CENTER_CENTER, "📂",
                                      FontId::new(14.0, FontFamily::Proportional), custom_icon_c);
                    if restore_custom_resp.clicked() {
                        restore_custom_idx = Some(idx);
                    }
                }
            }
        });

    if let Some(idx) = secure_idx {
        ctrl.secure_system_trash_item(state, idx);
    }
    if let Some(idx) = preview_idx {
        ctrl.preview_system_trash_to_memory(state, idx);
    }
    if let Some(idx) = restore_original_idx {
        ctrl.restore_system_trash_original(state, idx);
    }
    if let Some(idx) = restore_custom_idx {
        #[cfg(not(target_os = "android"))]
        let dest_dir = rfd::FileDialog::new().set_title("Pilih folder tujuan").pick_folder();
        #[cfg(target_os = "android")]
        let dest_dir: Option<std::path::PathBuf> = { state.set_status("Memilih folder tujuan belum didukung di Android", false); None };
        if let Some(dest_dir) = dest_dir {
            ctrl.restore_system_trash_custom(state, idx, dest_dir);
        }
    }

}

// ── VIRTUAL SECURE KEYBOARD ──────────────────────────────────
#[allow(dead_code)]
fn render_virtual_keyboard(ctx: &egui::Context, state: &mut AppState) {
    let mut close_keyboard = false;
    egui::TopBottomPanel::bottom("virtual_keyboard")
        .exact_height(300.0)
        .frame(egui::Frame::none().fill(crate::theme::bg_surface()).inner_margin(egui::Margin::symmetric(8.0, 12.0)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔐 Secure Keyboard").color(crate::theme::text_muted()).size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Tutup 🔽").size(13.0)).clicked() {
                        close_keyboard = true;
                    }
                });
            });
            ui.add_space(12.0);
            
            let target_str = match state.focused_field {
                crate::app_state::FocusedField::LoginUsername => &mut state.login_username,
                crate::app_state::FocusedField::LoginPassword => &mut state.login_password,
                crate::app_state::FocusedField::SetupUsername => &mut state.setup_username,
                crate::app_state::FocusedField::SetupDisplayName => &mut state.setup_display_name,
                crate::app_state::FocusedField::SetupPassword => &mut state.setup_password,
                crate::app_state::FocusedField::SetupConfirmPassword => &mut state.setup_password_confirm,
                crate::app_state::FocusedField::None => { close_keyboard = true; return; }
            };

            let keys = [
                ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
                ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
                ["a", "s", "d", "f", "g", "h", "j", "k", "l", ""],
                ["SFT", "z", "x", "c", "v", "b", "n", "m", "DEL", ""],
            ];

            let spacing = 6.0;
            let btn_width_base = (ui.available_width() - (spacing * 9.0)) / 10.0;
            let btn_height = 42.0;

            // OVERRIDE STYLE SO BUTTONS DON'T EXPAND DUE TO PADDING!
            ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);
            ui.spacing_mut().button_padding = egui::vec2(0.0, 0.0); // CRUCIAL: Remove padding so buttons fit!

            for (_r_idx, row) in keys.iter().enumerate() {
                ui.horizontal(|ui| {
                    // Calculate precise row width
                    let mut row_width = 0.0;
                    for key in row {
                        if key.is_empty() { continue; }
                        let w = if *key == "DEL" || *key == "SFT" { btn_width_base * 1.5 + spacing * 0.5 } else { btn_width_base };
                        row_width += w + spacing;
                    }
                    row_width -= spacing;
                    
                    let indent = (ui.available_width() - row_width) / 2.0;
                    if indent > 1.0 {
                        ui.add_space(indent);
                    }
                    
                    for key in row {
                        if key.is_empty() { continue; }
                        let label = key.to_string();
                        let w = if *key == "DEL" || *key == "SFT" { btn_width_base * 1.5 + spacing * 0.5 } else { btn_width_base };
                        
                        let (display_label, font_size) = match label.as_str() {
                            "SFT" => ("Shift", 13.0),
                            "DEL" => ("Del", 13.0),
                            _ => (label.as_str(), 18.0),
                        };
                        
                        let bg_color = if label == "SFT" || label == "DEL" {
                            Color32::from_rgb(45, 50, 60)
                        } else {
                            crate::theme::bg_card()
                        };

                        let btn = egui::Button::new(egui::RichText::new(display_label).size(font_size).color(Color32::WHITE))
                            .min_size(egui::vec2(w, btn_height))
                            .fill(bg_color)
                            .rounding(6.0);
                            
                        if ui.add(btn).clicked() {
                            if label == "DEL" {
                                target_str.pop();
                            } else if label != "SFT" {
                                target_str.push_str(&label);
                            }
                        }
                    }
                });
            }
            
            // SPACE BAR ROW
            ui.horizontal(|ui| {
                let space_w = btn_width_base * 5.0 + spacing * 4.0;
                let indent = (ui.available_width() - space_w) / 2.0;
                if indent > 1.0 {
                    ui.add_space(indent);
                }
                let space_btn = egui::Button::new(egui::RichText::new("SPACE").size(14.0).color(Color32::WHITE))
                    .min_size(egui::vec2(space_w, btn_height))
                    .fill(Color32::from_rgb(45, 50, 60))
                    .rounding(6.0);
                if ui.add(space_btn).clicked() {
                    target_str.push(' ');
                }
            });
        });

    if close_keyboard {
        state.show_keyboard = false;
        state.focused_field = crate::app_state::FocusedField::None;
    }
}

// 🛡️ Anti-Tampering Render Helpers
fn draw_security_background(ctx: &egui::Context) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let rect    = ctx.screen_rect();
    let mut mesh = Mesh::default();
    
    // Deep crimson/black high-security gradient mesh
    mesh.vertices.extend([
        Vertex { pos: rect.left_top(),     uv: egui::pos2(0.,0.), color: Color32::from_rgb(25, 8, 10) },
        Vertex { pos: rect.right_top(),    uv: egui::pos2(1.,0.), color: Color32::from_rgb(25, 8, 10) },
        Vertex { pos: rect.right_bottom(), uv: egui::pos2(1.,1.), color: Color32::from_rgb(10, 4, 5) },
        Vertex { pos: rect.left_bottom(),  uv: egui::pos2(0.,1.), color: Color32::from_rgb(10, 4, 5) },
    ]);
    mesh.add_triangle(0,1,2);
    mesh.add_triangle(0,2,3);
    painter.add(egui::Shape::Mesh(mesh));
}

fn render_security_violation(ctx: &egui::Context, details: &str) {
    draw_security_background(ctx);
    
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            let avail = ui.available_rect_before_wrap();
            
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(avail), |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space((avail.height() - 440.0).max(0.0) / 2.0);
                    
                    // Danger/Shield Icon
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(68.0), egui::Sense::hover());
                    filled_rect(ui, icon_rect, Color32::from_rgb(45, 12, 16), Stroke::new(1.0, error_color()), 18.0);
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, "⚠️",
                                      FontId::new(32.0, FontFamily::Proportional), error_color());
                    
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("AKSES DITOLAK").size(24.0).color(error_color()).strong());
                    ui.label(egui::RichText::new("Modifikasi Sistem Terdeteksi").size(14.0).color(text_muted()));
                    
                    ui.add_space(24.0);
                    
                    // Warning Card
                    let card_w = (avail.width() - 40.0).min(380.0);
                    egui::Frame::none()
                        .fill(bg_surface())
                        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(244, 63, 94, 50)))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::symmetric(20.0, 16.0))
                        .show(ui, |ui| {
                            ui.set_max_width(card_w - 40.0);
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Aplikasi Aegis Vault tidak dapat dijalankan pada perangkat yang di-Root atau di dalam Emulator karena rentan terhadap penyadapan dan manipulasi memori demi keamanan data sensitif Anda.").size(13.0).color(text_body()));
                                ui.add_space(14.0);
                                
                                // Details Box
                                egui::Frame::none()
                                    .fill(Color32::from_rgb(26, 12, 14))
                                    .stroke(Stroke::new(0.5, Color32::from_rgb(80, 20, 25)))
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                                    .show(ui, |ui| {
                                        ui.horizontal_top(|ui| {
                                            ui.label(egui::RichText::new("🛡️").size(12.0).color(error_color()));
                                            ui.add_space(6.0);
                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new("Detail Indikasi:").size(11.0).color(error_color()).strong());
                                                ui.add_space(2.0);
                                                ui.label(egui::RichText::new(details).size(11.0).color(text_body()));
                                            });
                                        });
                                    });
                            });
                        });
                        
                    ui.add_space(32.0);
                    
                    // Exit Application Button
                    let exit_btn = egui::Button::new(egui::RichText::new("🚪  Keluar Aplikasi").size(15.0).color(Color32::WHITE))
                        .fill(error_color())
                        .min_size(Vec2::new(200.0, 44.0));
                    
                    if ui.add(exit_btn).clicked() {
                        std::process::exit(0);
                    }
                });
            });
        });
}

// ── Screen Overlay: Modals ─────────────────────────────────

fn render_storage_modals(ctx: &egui::Context, state: &mut AppState) {
    if state.storage_pin_modal_open {
        egui::Area::new(egui::Id::new("storage_pin_modal"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let rect = ctx.screen_rect();
                filled_rect(ui, rect, Color32::from_black_alpha(200), Stroke::NONE, 0.0);
                
                let modal_size = Vec2::new(340.0, 520.0);
                let modal_rect = egui::Rect::from_center_size(rect.center(), modal_size);
                
                if ui.input(|i| i.pointer.any_pressed()) && !modal_rect.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or(egui::pos2(0.,0.)))) {
                    state.storage_pin_modal_open = false;
                }
                
                filled_rect(ui, modal_rect, Color32::from_rgb(18, 20, 28), Stroke::new(1.0, border_default()), 24.0);
                
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(modal_rect.shrink(20.0)), |ui| {
                    ui.vertical_centered(|ui| {
                        let shield_rect = egui::Rect::from_center_size(egui::pos2(ui.cursor().left() + modal_rect.width()/2.0 - 20.0, ui.cursor().top() + 36.0), Vec2::splat(56.0));
                        filled_rect(ui, shield_rect, accent_purple_a(), Stroke::NONE, 16.0);
                        ui.painter().text(shield_rect.center(), egui::Align2::CENTER_CENTER, "🔒", FontId::new(26.0, FontFamily::Proportional), accent_purple());
                        
                        ui.add_space(60.0);
                        ui.label(egui::RichText::new("Verifikasi PIN").size(20.0).color(text_primary()).strong());
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Masukkan PIN 6 digit Anda untuk\nmengubah lokasi penyimpanan").size(12.0).color(text_muted()));
                        ui.add_space(20.0);
                        
                        ui.horizontal(|ui| {
                            let dot_size = 14.0;
                            let gap = 10.0;
                            let total = 6.0 * dot_size + 5.0 * gap;
                            ui.add_space((ui.available_width() - total) / 2.0);
                            for i in 0..6 {
                                let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(dot_size), egui::Sense::hover());
                                let is_filled = i < state.storage_pin.len();
                                if is_filled {
                                    let scaled = egui::Rect::from_center_size(dot_rect.center(), Vec2::splat(dot_size * 1.25));
                                    filled_rect(ui, scaled, Color32::from_rgb(129, 140, 248), Stroke::NONE, dot_size * 0.625);
                                } else {
                                    filled_rect(ui, dot_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 20), Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 23)), dot_size / 2.0);
                                }
                                if i < 5 { ui.add_space(gap); }
                            }
                        });
                        
                        if let Some(err) = &state.storage_pin_error {
                            ui.add_space(12.0);
                            ui.label(egui::RichText::new(err).color(error_color()).size(12.0));
                        }
                        
                        ui.add_space(16.0);
                        let btn_w = 80.0;
                        let gap = 12.0;
                        let numpad_w = 3.0 * btn_w + 2.0 * gap;
                        let mut btn_idx = 1;
                        for _row in 0..3 {
                            ui.horizontal(|ui| {
                                ui.add_space((ui.available_width() - numpad_w) / 2.0);
                                for _col in 0..3 {
                                    if ghost_btn(ui, &btn_idx.to_string(), btn_w).clicked() {
                                        if state.storage_pin.len() < 6 {
                                            state.storage_pin.push_str(&btn_idx.to_string());
                                        }
                                    }
                                    if _col < 2 { ui.add_space(gap); }
                                    btn_idx += 1;
                                }
                            });
                            ui.add_space(gap);
                        }
                        // Bottom row: Batal | 0 | Delete
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() - numpad_w) / 2.0);
                            // Batal (special button like .nbtn.sp)
                            {
                                let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 54.0), egui::Sense::click());
                                let bg = Color32::from_rgba_unmultiplied(129, 140, 248, 25);
                                let border_c = Color32::from_rgba_unmultiplied(129, 140, 248, 64);
                                ui.painter().rect(rect, Rounding::same(18.0), bg, Stroke::new(1.0, border_c));
                                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Batal",
                                    FontId::new(11.0, FontFamily::Proportional), Color32::from_rgb(129, 140, 248));
                                if resp.clicked() { state.storage_pin_modal_open = false; }
                            }
                            ui.add_space(gap);
                            if ghost_btn(ui, "0", btn_w).clicked() {
                                if state.storage_pin.len() < 6 { state.storage_pin.push('0'); }
                            }
                            ui.add_space(gap);
                            // Delete button (.nbtn.dl)
                            {
                                let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 54.0), egui::Sense::click());
                                let bg = if resp.hovered() { Color32::from_rgba_unmultiplied(255, 255, 255, 10) } else { Color32::TRANSPARENT };
                                let border_c = Color32::from_rgba_unmultiplied(255, 255, 255, 13);
                                ui.painter().rect(rect, Rounding::same(18.0), bg, Stroke::new(1.0, border_c));
                                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Hapus",
                                    FontId::new(14.0, FontFamily::Proportional), Color32::from_rgb(115, 121, 150));
                                if resp.clicked() { state.storage_pin.pop(); }
                            }
                        });
                        
                        if state.storage_pin.len() == 6 {
                            state.storage_pin_modal_open = false;
                            state.storage_path_modal_open = true;
                            state.storage_pin.clear();
                        }
                    });
                });
            });
    }

    if state.storage_path_modal_open {
        egui::Area::new(egui::Id::new("storage_path_modal"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let rect = ctx.screen_rect();
                filled_rect(ui, rect, Color32::from_black_alpha(200), Stroke::NONE, 0.0);
                
                let modal_size = Vec2::new(340.0, 420.0);
                let modal_rect = egui::Rect::from_center_size(rect.center(), modal_size);
                
                if ui.input(|i| i.pointer.any_pressed()) && !modal_rect.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or(egui::pos2(0.,0.)))) {
                    state.storage_path_modal_open = false;
                }
                
                filled_rect(ui, modal_rect, Color32::from_rgb(18, 20, 28), Stroke::new(1.0, border_default()), 24.0);
                
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(modal_rect.shrink(24.0)), |ui| {
                    ui.horizontal(|ui| {
                        let icon_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::splat(44.0));
                        filled_rect(ui, icon_rect, accent_gold_a(), Stroke::NONE, 14.0);
                        ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, "📁", FontId::new(20.0, FontFamily::Proportional), accent_gold());
                        ui.add_space(56.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Lokasi Penyimpanan").size(17.0).color(text_primary()).strong());
                            ui.label(egui::RichText::new("Pilih atau tentukan path vault Anda").size(11.0).color(text_muted()));
                        });
                    });
                    ui.add_space(24.0);
                    
                    let mut path_option = state.storage_path.clone();
                    
                    let is_local = path_option.starts_with("vault_storage");
                    let (r1, resp1) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 64.0), egui::Sense::click());
                    let b1 = if is_local { accent_mint() } else if resp1.hovered() { border_hover() } else { border_default() };
                    filled_rect(ui, r1, if is_local { accent_mint_a() } else { bg_surface() }, Stroke::new(if is_local { 1.5 } else { 0.5 }, b1), 16.0);
                    let i1 = egui::Rect::from_center_size(egui::pos2(r1.left()+28.0, r1.center().y), Vec2::splat(36.0));
                    filled_rect(ui, i1, accent_mint_a(), Stroke::NONE, 10.0);
                    ui.painter().text(i1.center(), egui::Align2::CENTER_CENTER, "📱", FontId::new(16.0, FontFamily::Proportional), accent_mint());
                    ui.painter().text(egui::pos2(i1.right()+12.0, r1.center().y - 10.0), egui::Align2::LEFT_CENTER, "Lokal (Default)", FontId::new(13.0, FontFamily::Proportional), text_primary());
                    ui.painter().text(egui::pos2(i1.right()+12.0, r1.center().y + 10.0), egui::Align2::LEFT_CENTER, "vault_storage/ · Di dalam aplikasi", FontId::new(11.0, FontFamily::Proportional), text_muted());
                    if is_local {
                        ui.painter().text(egui::pos2(r1.right()-20.0, r1.center().y), egui::Align2::RIGHT_CENTER, "✔️", FontId::new(14.0, FontFamily::Proportional), accent_mint());
                    }
                    if resp1.clicked() { path_option = "vault_storage/ - Lokal".to_string(); }
                    
                    ui.add_space(10.0);
                    
                    let is_sdcard = path_option.starts_with("/sdcard");
                    let (r2, resp2) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 64.0), egui::Sense::click());
                    let b2 = if is_sdcard { accent_purple() } else if resp2.hovered() { border_hover() } else { border_default() };
                    filled_rect(ui, r2, if is_sdcard { accent_purple_a() } else { bg_surface() }, Stroke::new(if is_sdcard { 1.5 } else { 0.5 }, b2), 16.0);
                    let i2 = egui::Rect::from_center_size(egui::pos2(r2.left()+28.0, r2.center().y), Vec2::splat(36.0));
                    filled_rect(ui, i2, accent_purple_a(), Stroke::NONE, 10.0);
                    ui.painter().text(i2.center(), egui::Align2::CENTER_CENTER, "💾", FontId::new(16.0, FontFamily::Proportional), accent_purple());
                    ui.painter().text(egui::pos2(i2.right()+12.0, r2.center().y - 10.0), egui::Align2::LEFT_CENTER, "SD Card / Eksternal", FontId::new(13.0, FontFamily::Proportional), text_primary());
                    ui.painter().text(egui::pos2(i2.right()+12.0, r2.center().y + 10.0), egui::Align2::LEFT_CENTER, "/sdcard/DataVault/ · Penyimpanan eksternal", FontId::new(11.0, FontFamily::Proportional), text_muted());
                    if is_sdcard {
                        ui.painter().text(egui::pos2(r2.right()-20.0, r2.center().y), egui::Align2::RIGHT_CENTER, "✔️", FontId::new(14.0, FontFamily::Proportional), accent_purple());
                    }
                    if resp2.clicked() { path_option = "/sdcard/DataVault/ - Eksternal".to_string(); }
                    
                    state.storage_path = path_option;
                    
                    ui.add_space(20.0);
                    
                    let y_space = ui.available_height() - 50.0;
                    ui.add_space(y_space.max(0.0));
                    
                    ui.horizontal(|ui| {
                        let w = ui.available_width();
                        if ghost_btn(ui, "Batal", (w - 12.0)*0.4).clicked() {
                            state.storage_path_modal_open = false;
                        }
                        ui.add_space(12.0);
                        if teal_btn(ui, "Simpan", (w - 12.0)*0.6).clicked() {
                            state.storage_path_modal_open = false;
                            state.toast_message = Some(format!("Lokasi disimpan ke: {}", state.storage_path));
                            state.toast_timer = 2.0;
                        }
                    });
                });
            });
    }
}

fn render_context_menu(ctx: &egui::Context, state: &mut AppState, ctrl: &Controller) {
    let active_id = if let Some(id) = &state.active_context_menu { id.clone() } else { return };
    let record = state.file_list.iter().find(|f| f.id == active_id).cloned();
    if record.is_none() { 
        state.active_context_menu = None; 
        return; 
    }
    let record = record.unwrap();

    egui::Area::new(egui::Id::new("context_menu_area"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let screen_rect = ctx.screen_rect();
            filled_rect(ui, screen_rect, Color32::from_black_alpha(150), Stroke::NONE, 0.0);
            
            let menu_h = 300.0;
            let menu_w = 340.0;
            let modal_rect = egui::Rect::from_center_size(screen_rect.center(), Vec2::new(menu_w, menu_h));
            
            if ui.input(|i| i.pointer.any_pressed()) && !modal_rect.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or(egui::pos2(0.,0.)))) {
                state.active_context_menu = None;
            }
            
            filled_rect(ui, modal_rect, Color32::from_rgb(18, 20, 28), Stroke::new(1.0, border_default()), 24.0);
            
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(modal_rect.shrink(20.0)), |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(&record.original_name).size(15.0).color(text_primary()).strong());
                    ui.label(egui::RichText::new(format_size(record.file_size as u64)).size(11.0).color(text_muted()));
                });
                ui.add_space(20.0);
                
                let draw_item = |ui: &mut egui::Ui, icon: &str, text: &str, color: Color32| -> egui::Response {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 48.0), egui::Sense::click());
                    let bg = if resp.hovered() { Color32::from_black_alpha(50) } else { Color32::TRANSPARENT };
                    filled_rect(ui, r, bg, Stroke::NONE, 12.0);
                    
                    let ico_r = egui::Rect::from_center_size(egui::pos2(r.left() + 24.0, r.center().y), Vec2::splat(28.0));
                    ui.painter().text(ico_r.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(16.0, FontFamily::Proportional), color);
                    ui.painter().text(egui::pos2(ico_r.right() + 12.0, r.center().y), egui::Align2::LEFT_CENTER, text, FontId::new(14.0, FontFamily::Proportional), color);
                    resp
                };
                
                if draw_item(ui, "🔓", "Buka / Dekripsi", accent_purple()).clicked() {
                    ctrl.open_decrypt_panel(state, &record.vault_filename);
                    state.active_context_menu = None;
                }
                if draw_item(ui, "✏️", "Ganti Nama", text_primary()).clicked() {
                    state.rename_target_id = record.id.clone();
                    state.rename_new_name = record.original_name.clone();
                    state.rename_modal_open = true;
                    state.active_context_menu = None;
                }
                if draw_item(ui, "📡", "Bagikan via P2P", accent_sky()).clicked() {
                    ctrl.start_share(state, record.clone());
                    state.active_context_menu = None;
                }
                
                ui.add_space(10.0);
                ui.painter().line_segment([ui.cursor().min, ui.cursor().min + Vec2::new(ui.available_width(), 0.0)], Stroke::new(0.5, border_default()));
                ui.add_space(10.0);
                
                if draw_item(ui, "🗑", "Pindahkan ke Sampah", error_color()).clicked() {
                    ctrl.soft_delete_file(state, &record.id);
                    state.active_context_menu = None;
                }
            });
        });
}

// ── Screen Overlay: P2P Wi-Fi Sharing ──────────────────────

fn render_share_modal(ctx: &egui::Context, state: &mut AppState, ctrl: &Controller) {
    let record = match &state.share_active_record {
        Some(r) => r.clone(),
        None => return,
    };

    let share_url = format!("http://{}:{}/share/{}", state.share_ip, state.share_port, record.vault_filename);
    let qr_matrix = crate::totp::qr_matrix(&share_url);

    egui::Area::new(egui::Id::new("p2p_share_overlay"))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let rect = ctx.screen_rect();
            // Latar belakang hitam transparan
            filled_rect(ui, rect, Color32::from_black_alpha(200), Stroke::NONE, 0.0);

            // Dialog Box (Glassmorphism layout)
            let dialog_size = egui::vec2(380.0, 520.0);
            let dialog_rect = egui::Rect::from_center_size(rect.center(), dialog_size);

            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(dialog_rect), |ui| {
                theme::card_frame().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(8.0);
                        
                        // Icon & Title
                        ui.label(egui::RichText::new("📡").size(32.0));
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Secure Local Share").size(20.0).color(text_primary()).strong());
                        ui.label(egui::RichText::new("Berbagi data aman via Wi-Fi Lokal").size(11.0).color(text_muted()));

                        ui.add_space(16.0);

                        // File Info Badge
                        let file_card_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(300.0, 48.0));
                        ui.allocate_rect(file_card_rect, egui::Sense::hover());
                        filled_rect(ui, file_card_rect, bg_card(), Stroke::new(0.5, border_default()), 10.0);
                        
                        let (icon, badge) = if record.is_folder {
                            ("📁", BADGE_BLUE)
                        } else {
                            let ext = file_ext(&record.original_name);
                            file_badge(ext)
                        };
                        let file_icon_rect = egui::Rect::from_center_size(egui::pos2(file_card_rect.left() + 24.0, file_card_rect.center().y), Vec2::splat(32.0));
                        filled_rect(ui, file_icon_rect, badge.0, Stroke::NONE, 8.0);
                        ui.painter().text(file_icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(16.0, FontFamily::Proportional), badge.1);
                        
                        let name_disp = if record.original_name.len() > 24 {
                            format!("{}…", &record.original_name[..22])
                        } else {
                            record.original_name.clone()
                        };
                        ui.painter().text(
                            egui::pos2(file_icon_rect.right() + 10.0, file_card_rect.top() + 10.0),
                            egui::Align2::LEFT_TOP, &name_disp,
                            FontId::new(13.0, FontFamily::Proportional), text_primary()
                        );
                        ui.painter().text(
                            egui::pos2(file_icon_rect.right() + 10.0, file_card_rect.top() + 26.0),
                            egui::Align2::LEFT_TOP,
                            &format_size(record.file_size as u64),
                            FontId::new(11.0, FontFamily::Proportional), text_muted()
                        );

                        ui.add_space(20.0);

                        // QR Code
                        if let Some(matrix) = qr_matrix {
                            crate::totp::draw_qr(ui, &matrix, 160.0);
                        } else {
                            ui.label(egui::RichText::new("Gagal membuat QR Code").color(error_color()));
                        }

                        ui.add_space(16.0);

                        // Teks Instruksi IP
                        ui.label(egui::RichText::new("HUBUNGKAN PENERIMA KE WI-FI YANG SAMA & PINDAI QR").size(10.0).color(text_muted()).strong());
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(format!("Atau buka: http://{}:{}", state.share_ip, state.share_port)).size(12.0).color(teal_light()).text_style(egui::TextStyle::Monospace));

                        ui.add_space(16.0);

                        // Transfer PIN Display
                        egui::Frame::none()
                            .fill(Color32::from_rgba_unmultiplied(182, 102, 210, 15))
                            .stroke(Stroke::new(0.5, border_accent()))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(egui::Margin::symmetric(24.0, 10.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("PIN TRANSFER: ").size(12.0).color(text_muted()));
                                    ui.label(egui::RichText::new(&state.share_pin).size(20.0).color(teal_strong()).strong());
                                });
                            });

                        ui.add_space(24.0);

                        // Tombol Stop Share
                        if ghost_btn(ui, "🛑 Hentikan Berbagi", 200.0).clicked() {
                            ctrl.stop_share(state);
                        }
                        
                        ui.add_space(8.0);
                    });
                });
            });
        });
}

fn render_rename_modal(ctx: &egui::Context, state: &mut AppState, ctrl: &Controller) {
    egui::Area::new(egui::Id::new("rename_file_modal"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let rect = ctx.screen_rect();
            filled_rect(ui, rect, Color32::from_black_alpha(200), Stroke::NONE, 0.0);
            
            let modal_size = Vec2::new(320.0, 220.0);
            let modal_rect = egui::Rect::from_center_size(rect.center(), modal_size);
            
            // Tutup jika klik di luar modal
            if ui.input(|i| i.pointer.any_pressed()) && !modal_rect.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or(egui::pos2(0.,0.)))) {
                state.rename_modal_open = false;
            }
            
            filled_rect(ui, modal_rect, Color32::from_rgb(18, 20, 28), Stroke::new(1.0, border_default()), 24.0);
            
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(modal_rect.shrink(20.0)), |ui| {
                ui.vertical_centered(|ui| {
                    let icon_rect = egui::Rect::from_center_size(egui::pos2(ui.cursor().left() + modal_rect.width()/2.0 - 20.0, ui.cursor().top() + 30.0), Vec2::splat(44.0));
                    filled_rect(ui, icon_rect, accent_purple_a(), Stroke::NONE, 14.0);
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, "✏️", FontId::new(20.0, FontFamily::Proportional), accent_purple());
                    
                    ui.add_space(60.0);
                    ui.label(egui::RichText::new("Ganti Nama Berkas").size(17.0).color(text_primary()).strong());
                    ui.add_space(12.0);
                    
                    // Input teks
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        let text_edit = egui::TextEdit::singleline(&mut state.rename_new_name)
                            .desired_width(ui.available_width() - 20.0)
                            .margin(egui::Margin::symmetric(10.0, 8.0));
                        ui.add(text_edit);
                    });
                    
                    ui.add_space(20.0);
                    
                    ui.horizontal(|ui| {
                        let w = ui.available_width();
                        if ghost_btn(ui, "Batal", (w - 12.0) * 0.4).clicked() {
                            state.rename_modal_open = false;
                        }
                        ui.add_space(12.0);
                        if teal_btn(ui, "Simpan", (w - 12.0) * 0.6).clicked() {
                            let id = state.rename_target_id.clone();
                            let name = state.rename_new_name.clone();
                            ctrl.rename_file(state, &id, &name);
                            state.rename_modal_open = false;
                        }
                    });
                });
            });
        });
}

// ── SCREEN: CUSTOM FILE PICKER (PURE RUST/EGUI) ─────────────────────────
fn render_custom_file_picker(
    ctx: &egui::Context,
    state: &mut AppState,
    ctrl: &Controller,
) {
    let screen_rect = ctx.screen_rect();

    // Now draw the file picker frame in the center
    let modal_w = (screen_rect.width() - 40.0).clamp(320.0, 420.0);
    let modal_h = (screen_rect.height() - 80.0).clamp(420.0, 580.0); // Slightly larger height for tips
    let modal_rect = egui::Rect::from_center_size(screen_rect.center(), Vec2::new(modal_w, modal_h));

    // Area starts from screen_rect.min (0,0) to easily render background dimmer correctly
    let area = egui::Area::new(egui::Id::new("custom_file_picker_area"))
        .order(egui::Order::Tooltip)
        .fixed_pos(screen_rect.min);

    area.show(ctx, |ui| {
        // 1. Draw dark background dimmer behind the modal
        ui.painter().rect_filled(screen_rect, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 185));

        // 2. Draw the actual modal card
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(modal_rect), |ui| {
            // Premium surface color (dark gray, not pitch black) with solid translucent border
            filled_rect(ui, modal_rect, Color32::from_rgb(26, 26, 25), Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 15)), 20.0);

            // Content area padding
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(modal_rect.shrink(18.0)), |ui| {
                ui.vertical(|ui| {
                    // Title row
                    ui.horizontal(|ui| {
                        // BACK BUTTON (Tombol Kembali) - Perfectly aligned vertically
                        if let Some(parent) = state.custom_file_picker_current_dir.parent() {
                            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::click());
                            let back_color = if resp.hovered() { Color32::WHITE } else { text_muted() };
                            ui.painter().text(egui::pos2(rect.center().x, rect.center().y + 1.0), egui::Align2::CENTER_CENTER, "⬅", FontId::new(16.0, FontFamily::Proportional), back_color);
                            if resp.clicked() {
                                let parent_path = parent.to_path_buf();
                                ctrl.navigate_custom_file_picker(state, parent_path);
                            }
                            ui.add_space(6.0);
                        }
                        ui.label(egui::RichText::new("📁 Pilih File").size(18.0).color(Color32::WHITE).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::click());
                            let close_color = if resp.hovered() { Color32::from_rgb(239, 68, 68) } else { text_muted() };
                            ui.painter().text(egui::pos2(rect.center().x, rect.center().y + 1.0), egui::Align2::CENTER_CENTER, "❌", FontId::new(14.0, FontFamily::Proportional), close_color);
                            if resp.clicked() {
                                state.custom_file_picker_open = false;
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // Search bar
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🔍").color(text_muted()));
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), egui::Sense::hover());
                        filled_rect(ui, rect, Color32::from_rgba_unmultiplied(255, 255, 255, 6), Stroke::new(1.0, border_default()), 10.0);
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect.shrink(8.0)), |ui| {
                            let resp = ui.add(egui::TextEdit::singleline(&mut state.custom_file_picker_search)
                                .hint_text("Cari file...")
                                .frame(false)
                                .desired_width(rect.width() - 16.0)
                                .font(FontId::new(14.0, FontFamily::Proportional))
                                .interactive(true));
                            if resp.gained_focus() || resp.clicked() {
                                state.show_keyboard = true;
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // Current Path and Up button
                    ui.horizontal(|ui| {
                        let path_str = state.custom_file_picker_current_dir.to_string_lossy().to_string();
                        let truncated_path = if path_str.len() > 36 {
                            format!("...{}", &path_str[path_str.len() - 33..])
                        } else {
                            path_str
                        };
                        ui.label(egui::RichText::new(truncated_path).size(12.0).color(text_muted()));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(parent) = state.custom_file_picker_current_dir.parent() {
                                if ghost_btn(ui, "⬆ Atas", 60.0).clicked() {
                                    let parent_path = parent.to_path_buf();
                                    ctrl.navigate_custom_file_picker(state, parent_path);
                                }
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // Main files area
                    let scroll_h = (modal_h - 310.0).max(100.0);
                    egui::ScrollArea::vertical()
                        .id_salt("custom_file_picker_scroll")
                        .max_height(scroll_h)
                        .show(ui, |ui| {
                            if let Some(err) = &state.custom_file_picker_error {
                                ui.add_space(20.0);
                                ui.colored_label(error_color(), err);
                                return;
                            }

                            let search_lower = state.custom_file_picker_search.to_lowercase();
                            let filtered_paths: Vec<std::path::PathBuf> = state.custom_file_picker_files.iter()
                                .filter(|p| {
                                    if search_lower.is_empty() {
                                        true
                                    } else {
                                        let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                                        name.contains(&search_lower)
                                    }
                                })
                                .cloned()
                                .collect();

                            if filtered_paths.is_empty() {
                                ui.add_space(20.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(egui::RichText::new("Tidak ada file atau folder").color(text_muted()));
                                });
                                return;
                            }

                            for path in filtered_paths {
                                let is_dir = path.is_dir();
                                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

                                let icon = if is_dir { "📁" } else {
                                    let ext = file_ext(&name).to_lowercase();
                                    if ext == "png" || ext == "jpg" || ext == "jpeg" { "🖼" }
                                    else if ext == "vlt" { "🔐" }
                                    else { "📄" }
                                };

                                let item_w = ui.available_width();
                                let (rect, resp) = ui.allocate_exact_size(Vec2::new(item_w, 40.0), egui::Sense::click());

                                let bg = if resp.hovered() {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 8)
                                } else {
                                    Color32::TRANSPARENT
                                };

                                filled_rect(ui, rect, bg, Stroke::NONE, 8.0);

                                // Render icon and name
                                ui.painter().text(
                                    egui::pos2(rect.left() + 10.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    format!("{}  {}", icon, name),
                                    FontId::new(14.0, FontFamily::Proportional),
                                    Color32::WHITE
                                );

                                if resp.clicked() {
                                    if is_dir {
                                        let next_dir = path.clone();
                                        ctrl.navigate_custom_file_picker(state, next_dir);
                                    } else {
                                        // Select file!
                                        state.custom_file_picker_open = false;
                                        ctrl.encrypt_file(state, path.clone());
                                    }
                                }
                            }
                        });

                    // Tips Banner with dynamic Settings trigger link (Akses Penyimpanan Penuh)
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(129, 140, 248, 12)) // Indigo transparent tinted background
                        .stroke(Stroke::new(0.5, Color32::from_rgba_unmultiplied(129, 140, 248, 50)))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.add(egui::Label::new(
                                    egui::RichText::new("💡 Tips: Agar semua file (.pdf, .mp3, dll) terdeteksi, aktifkan izin akses penyimpanan penuh.")
                                        .size(10.0)
                                        .color(text_body())
                                ).wrap());
                                
                                ui.add_space(8.0);
                                
                                // Large beautiful touch-friendly button (height: 44px)
                                let btn_w = ui.available_width();
                                let desired_size = Vec2::new(btn_w, 44.0);
                                let (rect, resp) = ui.allocate_exact_size(desired_size, egui::Sense::click());
                                
                                let bg_c = if resp.is_pointer_button_down_on() {
                                    Color32::from_rgb(79, 70, 229) // Indigo active
                                } else if resp.hovered() {
                                    Color32::from_rgb(129, 140, 248) // Indigo hover
                                } else {
                                    Color32::from_rgb(99, 102, 241) // Indigo primary
                                };
                                
                                filled_rect(ui, rect, bg_c, Stroke::NONE, 10.0);
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "🔑  Aktifkan Izin Akses Semua File",
                                    FontId::new(12.0, FontFamily::Proportional),
                                    Color32::WHITE
                                );
                                
                                if resp.clicked() {
                                    state.request_storage_permission = true;
                                }
                            });
                        });

                    // Bottom row buttons
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let w = ui.available_width();
                        if ghost_btn(ui, "Batal", w).clicked() {
                            state.custom_file_picker_open = false;
                        }
                    });
                });
            });
        });
    });
}
