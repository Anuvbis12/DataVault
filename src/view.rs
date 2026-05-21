// view.rs — View layer
// Seluruh fungsi render egui. View hanya membaca AppState
// dan memanggil Controller untuk aksi. Tidak ada logika bisnis di sini.

use eframe::egui;
use egui::epaint::{Color32, FontId, FontFamily, Mesh, Rounding, Stroke, Vec2, Vertex};
#[cfg(not(target_os = "android"))]
use rfd::FileDialog;

use crate::app_state::{AppScreen, AppState, DashboardTab, ViewMode, SortOption};
use crate::controller::{format_size, Controller};
use crate::db::FileRecord;
use crate::theme::{self, *};

// ── Root render ───────────────────────────────────────────
pub fn render(
    ctx:        &egui::Context,
    state:      &mut AppState,
    controller: &Controller,
) {
    draw_background(ctx);

    // Overlay Virtual Keyboard (Secure Keyboard)
    if state.show_keyboard {
        render_virtual_keyboard(ctx, state);
    }

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

    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            let screen = state.screen.clone();
            match screen {
                AppScreen::Login             => render_login(ui, state, controller),
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

    // Update File Dialog
    let mut picked_path = None;
    if let Some(dialog) = &mut state.file_dialog {
        if let Some(path) = dialog.update(ctx).selected() {
            picked_path = Some(path.to_path_buf());
        }
    }
    if let Some(path) = picked_path {
        controller.encrypt_file(state, path);
    }
}

// ── Background gradien ────────────────────────────────────
fn draw_background(ctx: &egui::Context) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let rect    = ctx.screen_rect();
    let mut mesh = Mesh::default();
    mesh.vertices.extend([
        Vertex { pos: rect.left_top(),     uv: egui::pos2(0.,0.), color: Color32::from_rgb(14,16,22) },
        Vertex { pos: rect.right_top(),    uv: egui::pos2(1.,0.), color: Color32::from_rgb(14,16,22) },
        Vertex { pos: rect.right_bottom(), uv: egui::pos2(1.,1.), color: Color32::from_rgb(10,12,18) },
        Vertex { pos: rect.left_bottom(),  uv: egui::pos2(0.,1.), color: Color32::from_rgb(10,12,18) },
    ]);
    mesh.add_triangle(0,1,2);
    mesh.add_triangle(0,2,3);
    painter.add(egui::Shape::Mesh(mesh));
}

// ── Screen: Login ─────────────────────────────────────────
fn render_login(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let user_set = ctrl.is_user_set();
    let avail   = ui.available_rect_before_wrap();

    ui.allocate_ui_at_rect(avail, |ui| {
        ui.vertical_centered(|ui| {
            let content_h = if user_set { 380.0 } else { 200.0 };
            ui.add_space((avail.height() - content_h).max(0.0) / 2.0);

            // Shield icon
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(56.0), egui::Sense::hover());
            filled_rect(ui, icon_rect, teal_dark(), Stroke::NONE, 14.0);
            ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, "🛡",
                              FontId::new(26.0, FontFamily::Proportional), teal_faint());

            ui.add_space(14.0);
            ui.label(egui::RichText::new("Aegis Vault").size(20.0).color(crate::theme::text_body()).strong());
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Akses aman ke data kamu").size(13.0).color(crate::theme::text_muted()));

            if !user_set {
                ui.add_space(32.0);
                ui.label(egui::RichText::new("Vault baru terdeteksi.").color(warn_color()).size(13.0));
                ui.label(egui::RichText::new("Buat akun untuk memulai.").color(crate::theme::text_muted()).size(13.0));
                ui.add_space(20.0);
                if teal_btn(ui, "⚙  Buat Akun", 200.0).clicked() {
                    state.screen = AppScreen::SetupAccount;
                }
                return;
            }

            ui.add_space(32.0);

            let field_w = (avail.width() - 72.0).min(320.0);

            // Username field
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric((avail.width() - field_w) / 2.0, 0.0))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Username").size(12.0).color(crate::theme::text_muted()));
                    ui.add_space(6.0);
                    egui::Frame::none()
                        .fill(crate::theme::bg_surface()).stroke(Stroke::new(0.5, border_default()))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("👤").size(16.0).color(crate::theme::text_muted()));
                                ui.add_space(8.0);
                                let resp = ui.add(egui::TextEdit::singleline(&mut state.login_username)
                                    .hint_text("Masukkan username")
                                    .desired_width(field_w - 80.0)
                                    .font(FontId::new(16.0, FontFamily::Proportional))
                                    .interactive(true));
                                if resp.gained_focus() || resp.clicked() { state.focused_field = crate::app_state::FocusedField::LoginUsername; state.show_keyboard = true; }
                            });
                        });

                    ui.add_space(14.0);

                    // Password field
                    ui.label(egui::RichText::new("Password").size(12.0).color(crate::theme::text_muted()));
                    ui.add_space(6.0);
                    let accent = if !state.login_password.is_empty() { teal_strong() } else { border_default() };
                    egui::Frame::none()
                        .fill(crate::theme::bg_surface()).stroke(Stroke::new(0.5, accent))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("🔒").size(16.0).color(crate::theme::text_muted()));
                                ui.add_space(8.0);
                                let resp = ui.add(egui::TextEdit::singleline(&mut state.login_password)
                                    .password(true)
                                    .hint_text("Masukkan password")
                                    .desired_width(field_w - 80.0)
                                    .font(FontId::new(16.0, FontFamily::Proportional))
                                    .interactive(true));
                                if resp.gained_focus() || resp.clicked() { state.focused_field = crate::app_state::FocusedField::LoginPassword; state.show_keyboard = true; }
                            });
                        });

                    ui.add_space(16.0);

                    // Error label
                    if let Some(err) = &state.login_error {
                        ui.label(egui::RichText::new(err).color(error_color()).size(13.0));
                        ui.add_space(8.0);
                    }

                    ui.add_space(8.0);

                    if teal_btn(ui, "🔓  Masuk", ui.available_width()).clicked() {
                        let ok = ctrl.try_login(state);
                        if !ok { state.pin_shake_timer = 0.4; }
                    }
                });

            ui.add_space(20.0);
            let link_resp = ui.add(egui::Label::new(
                egui::RichText::new("Lupa Password? Reset Vault")
                    .size(12.0)
                    .color(crate::theme::text_muted())
            ).sense(egui::Sense::click()));
            if link_resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if link_resp.clicked() {
                state.show_reset_confirm = true;
            }
        });
    });

    // Overlay Reset Confirm
    if state.show_reset_confirm {
        egui::Area::new(egui::Id::new("reset_confirm_overlay"))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                let rect = ui.ctx().screen_rect();
                filled_rect(ui, rect, Color32::from_black_alpha(220), Stroke::NONE, 0.0);

                let dialog_size = egui::vec2(340.0, 180.0);
                let dialog_rect = egui::Rect::from_center_size(rect.center(), dialog_size);

                ui.allocate_ui_at_rect(dialog_rect, |ui| {
                    theme::card_frame().show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("⚠  Hapus Seluruh Vault?").color(warn_color()).size(18.0).strong());
                            ui.add_space(12.0);
                            ui.label(egui::RichText::new("Tindakan ini akan menghapus semua file yang ada di vault secara permanen karena password lama tidak dapat dipulihkan.").color(crate::theme::text_body()).size(13.0));
                            ui.add_space(24.0);
                            
                            // Letakkan tombol di tengah
                            ui.horizontal(|ui| {
                                ui.add_space((ui.available_width() - 260.0) / 2.0);
                                if ghost_btn(ui, "Batal", 120.0).clicked() {
                                    state.show_reset_confirm = false;
                                }
                                ui.add_space(20.0);
                                if teal_btn(ui, "Ya, Reset Vault", 120.0).clicked() {
                                    ctrl.reset_vault(state);
                                }
                            });
                            ui.add_space(10.0);
                        });
                    });
                });
            });
    }
}

// ── Screen: Setup Account ─────────────────────────────────
fn render_setup_account(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let avail   = ui.available_rect_before_wrap();
    let field_w = avail.width() - 72.0;

    egui::ScrollArea::vertical().show(ui, |ui| {
        let y_padding = (avail.height() - 480.0).max(0.0) / 2.0;
        ui.add_space(y_padding.max(32.0));

        ui.horizontal(|ui| {
            ui.add_space(36.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(38.0), egui::Sense::hover());
            filled_rect(ui, rect, bg_surface(), Stroke::new(0.5, border_default()), 10.0);
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "🔑",
                              FontId::new(18.0, FontFamily::Proportional), teal_strong());
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Buat Akun Baru").size(15.0).color(crate::theme::text_body()).strong());
                ui.label(egui::RichText::new("Setup username & password").size(12.0).color(crate::theme::text_muted()));
            });
        });

        ui.add_space(24.0);

        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(36.0, 0.0))
            .show(ui, |ui| {
                // Username
                ui.label(egui::RichText::new("Username").size(12.0).color(crate::theme::text_muted()));
                ui.add_space(6.0);
                egui::Frame::none()
                    .fill(crate::theme::bg_surface()).stroke(Stroke::new(0.5, border_default()))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("👤").size(16.0).color(crate::theme::text_muted()));
                            ui.add_space(8.0);
                            let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_username)
                                .hint_text("Min. 3 karakter")
                                .desired_width(field_w - 80.0)
                                .font(FontId::new(16.0, FontFamily::Proportional))
                                .interactive(true));
                            if resp.gained_focus() || resp.clicked() {
                                state.focused_field = crate::app_state::FocusedField::SetupUsername;
                                state.show_keyboard = true;
                            }
                        });
                    });

                ui.add_space(14.0);

                // Nama Lengkap
                ui.label(egui::RichText::new("Nama Lengkap").size(12.0).color(crate::theme::text_muted()));
                ui.add_space(6.0);
                egui::Frame::none()
                    .fill(crate::theme::bg_surface()).stroke(Stroke::new(0.5, border_default()))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("📝").size(16.0).color(crate::theme::text_muted()));
                            ui.add_space(8.0);
                            let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_display_name)
                                .hint_text("Nama untuk ditampilkan")
                                .desired_width(field_w - 80.0)
                                .font(FontId::new(16.0, FontFamily::Proportional))
                                .interactive(true));
                            if resp.gained_focus() || resp.clicked() {
                                state.focused_field = crate::app_state::FocusedField::SetupDisplayName;
                                state.show_keyboard = true;
                            }
                        });
                    });

                ui.add_space(14.0);

                // Password
                ui.label(egui::RichText::new("Password").size(12.0).color(crate::theme::text_muted()));
                ui.add_space(6.0);
                egui::Frame::none()
                    .fill(crate::theme::bg_surface()).stroke(Stroke::new(0.5, border_default()))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🔒").size(16.0).color(crate::theme::text_muted()));
                            ui.add_space(8.0);
                            let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_password)
                                .password(true).hint_text("Min. 4 karakter")
                                .desired_width(field_w - 80.0)
                                .font(FontId::new(16.0, FontFamily::Proportional))
                                .interactive(true));
                            if resp.gained_focus() || resp.clicked() {
                                state.focused_field = crate::app_state::FocusedField::SetupPassword;
                                state.show_keyboard = true;
                            }
                        });
                    });

                ui.add_space(14.0);

                // Konfirmasi Password
                ui.label(egui::RichText::new("Konfirmasi Password").size(12.0).color(crate::theme::text_muted()));
                ui.add_space(6.0);
                let accent = if !state.setup_password_confirm.is_empty() { teal_strong() } else { border_default() };
                let icon_c = if !state.setup_password_confirm.is_empty() { teal_strong() } else { text_muted() };
                egui::Frame::none()
                    .fill(crate::theme::bg_surface()).stroke(Stroke::new(0.5, accent))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🔒").size(16.0).color(icon_c));
                            ui.add_space(8.0);
                            let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_password_confirm)
                                .password(true).hint_text("Ulangi Password")
                                .desired_width(field_w - 80.0)
                                .font(FontId::new(16.0, FontFamily::Proportional))
                                .interactive(true));
                            if resp.gained_focus() || resp.clicked() {
                                state.focused_field = crate::app_state::FocusedField::SetupConfirmPassword;
                                state.show_keyboard = true;
                            }
                        });
                    });

                ui.add_space(16.0);

                // Info banner
                egui::Frame::none()
                    .fill(Color32::from_rgb(12, 31, 24))
                    .stroke(Stroke::new(0.5, border_accent()))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.label(egui::RichText::new("ℹ").size(16.0).color(teal_strong()));
                            ui.add_space(8.0);
                            ui.add(egui::Label::new(egui::RichText::new(
                                "Password di-hash dengan Argon2id dan salt unik. \
                                 Tidak ada cara memulihkan password yang hilang."
                            ).size(12.0).color(teal_light())).wrap(true));
                        });
                    });

                ui.add_space(24.0);

                if let Some(err) = state.setup_error.clone() {
                    ui.label(egui::RichText::new(&err).color(error_color()).size(13.0));
                    ui.add_space(8.0);
                }

                if teal_btn(ui, "Buat Akun & masuk", ui.available_width()).clicked() {
                    ctrl.setup_account(state);
                }
            });
    });
}

// ── Screen: Dashboard ─────────────────────────────────────
fn render_dashboard(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    ctrl.refresh_device_metrics(state);
    ui.ctx().request_repaint_after(std::time::Duration::from_secs(2));

    let avail = ui.available_rect_before_wrap();
    
    // ─ Topbar ─
    let topbar_h = 60.0;
    let topbar_rect = egui::Rect::from_min_size(avail.min, Vec2::new(avail.width(), topbar_h));
    filled_rect(ui, topbar_rect, Color32::from_rgb(14, 16, 22), Stroke::new(0.5, border_subtle()), 0.0);
    
    // Logo + Greeting
    let greeting = if state.display_name.is_empty() {
        "Aegis.Vault".to_string()
    } else {
        format!("Halo, {} 👋", state.display_name)
    };
    let brand_pos = egui::pos2(avail.left() + 20.0, topbar_rect.center().y);
    ui.painter().text(brand_pos, egui::Align2::LEFT_CENTER, &greeting,
                      FontId::new(18.0, FontFamily::Proportional), text_primary());
    
    // Topbar Icons
    let notif_rect = egui::Rect::from_center_size(egui::pos2(avail.right() - 60.0, topbar_rect.center().y), Vec2::splat(32.0));
    let notif_resp = ui.allocate_rect(notif_rect, egui::Sense::click());
    ui.painter().text(notif_rect.center(), egui::Align2::CENTER_CENTER, "🔔", FontId::new(18.0, FontFamily::Proportional), if notif_resp.hovered() { teal_strong() } else { text_muted() });
    if notif_resp.clicked() { 
        state.dashboard_tab = DashboardTab::Notifications; 
        ctrl.load_audit_logs(state);
    }

    let profile_rect = egui::Rect::from_center_size(egui::pos2(avail.right() - 20.0, topbar_rect.center().y), Vec2::splat(32.0));
    let profile_resp = ui.allocate_rect(profile_rect, egui::Sense::click());
    ui.painter().text(profile_rect.center(), egui::Align2::CENTER_CENTER, "👤", FontId::new(18.0, FontFamily::Proportional), if profile_resp.hovered() { teal_strong() } else { text_muted() });
    if profile_resp.clicked() { state.dashboard_tab = DashboardTab::Profile; }

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
        state.transition_start = Some(std::time::Instant::now());
    }

    let mut opacity = 1.0;
    if let Some(start) = state.transition_start {
        let elapsed = start.elapsed().as_secs_f32();
        let duration = 0.2; // 200ms
        if elapsed < duration {
            opacity = elapsed / duration;
            ui.ctx().request_repaint();
        } else {
            state.transition_start = None;
        }
    }

    // Render Content Area first so Bottom Navigation draws ON TOP of it (fixing FAB overlap)
    ui.allocate_ui_at_rect(content_rect, |ui| {
        ui.set_opacity(opacity);
        egui::ScrollArea::vertical().id_source("dashboard_scroll").show(ui, |ui| {

             ui.add_space(20.0);
             match state.dashboard_tab {
                 DashboardTab::Home => render_tab_home(ui, state, ctrl, &mut to_decrypt, &mut to_soft_delete),
                 DashboardTab::Vault => render_tab_vault(ui, state, ctrl, &mut to_decrypt, &mut to_soft_delete),
                 DashboardTab::Storage => render_tab_storage(ui, state, ctrl),
                 DashboardTab::Settings => render_tab_settings(ui, state, ctrl),
                 DashboardTab::Profile => render_tab_profile(ui, state, ctrl),
                 DashboardTab::Notifications => render_tab_notifications(ui, state, ctrl),
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
        (DashboardTab::Home, "🏠", "Home"),
        (DashboardTab::Vault, "🔒", "Vault"),
        (DashboardTab::Home, "➕", "Add"), // Placeholder for FAB
        (DashboardTab::Storage, "💽", "Storage"),
        (DashboardTab::Settings, "⚙", "Settings"),
    ];
    
    for (i, (tab, icon, label)) in tabs.iter().enumerate() {
        if i == 2 {
            // FAB (Add button)
            let fab_size = Vec2::splat(56.0);
            let fab_rect = egui::Rect::from_center_size(egui::pos2(tab_x, bottom_rect.top() - 10.0), fab_size);
            let fab_resp = ui.allocate_rect(fab_rect, egui::Sense::click());
            let fab_fill = if fab_resp.hovered() { teal_light() } else { teal_strong() };
            filled_rect(ui, fab_rect, fab_fill, Stroke::new(4.0, bg_base()), 28.0);
            ui.painter().text(fab_rect.center(), egui::Align2::CENTER_CENTER, "➕", FontId::new(24.0, FontFamily::Proportional), bg_base());
            if fab_resp.clicked() {
                if state.file_dialog.is_none() {
                    state.file_dialog = Some(egui_file_dialog::FileDialog::new());
                }
                if let Some(dialog) = &mut state.file_dialog {
                    dialog.select_file();
                }
            }
        } else {
            let item_rect = egui::Rect::from_center_size(egui::pos2(tab_x, tab_y), Vec2::new(tab_w, bottom_h));
            let item_resp = ui.allocate_rect(item_rect, egui::Sense::click());
            let is_active = state.dashboard_tab == *tab;
            let color = if is_active || item_resp.hovered() { teal_strong() } else { text_muted() };
            
            ui.painter().text(egui::pos2(tab_x, tab_y - 10.0), egui::Align2::CENTER_CENTER, *icon, FontId::new(20.0, FontFamily::Proportional), color);
            ui.painter().text(egui::pos2(tab_x, tab_y + 12.0), egui::Align2::CENTER_CENTER, *label, FontId::new(10.0, FontFamily::Proportional), color);
            
            if item_resp.clicked() {
                state.dashboard_tab = tab.clone();
            }
        }
        tab_x += tab_w;
    }

    if let Some(fname) = to_decrypt { ctrl.open_decrypt_panel(state, &fname); }
    if let Some(id) = to_soft_delete { ctrl.soft_delete_file(state, &id); }
}

fn render_tab_home(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller, to_decrypt: &mut Option<String>, to_soft_delete: &mut Option<String>) {
    let avail = ui.available_rect_before_wrap();
    let pad = 20.0;
    
    // Stat Cards
    let stat_w = (avail.width() - pad * 2.0 - 24.0) / 3.0;
    let stat_h = 80.0;
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let stats = [
            ("Locked Files", format!("{}", state.file_list.len()), "📄", teal_strong()),
            ("Encrypted", format_size(state.total_vault_size()), "💽", teal_strong()),
            ("Standard", "AES-256".to_string(), "🛡", teal_strong()),
        ];
        for (label, val, icon, color) in stats.iter() {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(stat_w, stat_h), egui::Sense::hover());
            filled_rect(ui, rect, bg_surface(), Stroke::new(1.0, border_default()), 16.0);
            ui.painter().text(egui::pos2(rect.center().x, rect.top() + 20.0), egui::Align2::CENTER_CENTER, *icon, FontId::new(20.0, FontFamily::Proportional), *color);
            ui.painter().text(egui::pos2(rect.center().x, rect.top() + 45.0), egui::Align2::CENTER_CENTER, val, FontId::new(18.0, FontFamily::Proportional), text_primary());
            ui.painter().text(egui::pos2(rect.center().x, rect.top() + 65.0), egui::Align2::CENTER_CENTER, *label, FontId::new(10.0, FontFamily::Proportional), text_muted());
            ui.add_space(12.0);
        }
    });

    ui.add_space(24.0);
    
    // Hardware Metrics
    ui.horizontal(|ui| { ui.add_space(pad); ui.label(egui::RichText::new("HARDWARE METRICS").size(12.0).color(crate::theme::text_muted()).strong()); });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 140.0), egui::Sense::hover());
        filled_rect(ui, rect, bg_surface(), Stroke::new(1.0, border_default()), 20.0);
        
        ui.painter().text(egui::pos2(rect.left() + 20.0, rect.top() + 25.0), egui::Align2::LEFT_CENTER, "⚙ Encryption Engine", FontId::new(14.0, FontFamily::Proportional), text_primary());
        
        let badge_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 40.0, rect.top() + 25.0), Vec2::new(60.0, 20.0));
        filled_rect(ui, badge_rect, Color32::from_rgba_unmultiplied(182, 102, 210, 25), Stroke::new(1.0, Color32::from_rgba_unmultiplied(182, 102, 210, 50)), 10.0);
        ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, "High-tier", FontId::new(10.0, FontFamily::Proportional), teal_strong());
        
        let metrics = [("CPU", state.cpu_usage, teal_strong()), ("RAM", state.ram_usage, success_color()), ("I/O", state.io_usage, warn_color())];
        for (i, (lbl, val, color)) in metrics.iter().enumerate() {
            let y = rect.top() + 60.0 + i as f32 * 25.0;
            ui.painter().text(egui::pos2(rect.left() + 20.0, y), egui::Align2::LEFT_CENTER, *lbl, FontId::new(12.0, FontFamily::Proportional), text_muted());
            let bar_bg = egui::Rect::from_min_size(egui::pos2(rect.left() + 60.0, y - 3.0), Vec2::new(rect.width() - 120.0, 6.0));
            filled_rect(ui, bar_bg, bg_card(), Stroke::NONE, 3.0);
            let bar_fg = egui::Rect::from_min_size(egui::pos2(rect.left() + 60.0, y - 3.0), Vec2::new((rect.width() - 120.0) * val, 6.0));
            filled_rect(ui, bar_fg, *color, Stroke::NONE, 3.0);
            ui.painter().text(egui::pos2(rect.right() - 20.0, y), egui::Align2::RIGHT_CENTER, format!("{}%", (val * 100.0) as i32), FontId::new(12.0, FontFamily::Proportional), text_primary());
        }
    });

    ui.add_space(24.0);

    // Active Vaults
    ui.horizontal(|ui| { ui.add_space(pad); ui.label(egui::RichText::new("ACTIVE VAULTS").size(12.0).color(crate::theme::text_muted()).strong()); });
    ui.add_space(12.0);
    egui::ScrollArea::horizontal().id_source("vaults_scroll").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(pad);
            let vaults = [
                ("Primary Vault", "32.5 GB / 50 GB", 0.65, true),
                ("Session Vault", "1.2 GB / 5 GB", 0.24, false),
            ];
            for (name, cap, prog, locked) in vaults.iter() {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(220.0, 140.0), egui::Sense::hover());
                let color = if *locked { teal_strong() } else { success_color() };
                let bg_color = if *locked { Color32::from_rgba_unmultiplied(182, 102, 210, 12) } else { Color32::from_rgba_unmultiplied(74, 222, 128, 12) };
                
                filled_rect(ui, rect, bg_color, Stroke::new(1.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 76)), 20.0);
                
                let icon_rect = egui::Rect::from_min_size(egui::pos2(rect.left() + 20.0, rect.top() + 20.0), Vec2::splat(40.0));
                filled_rect(ui, icon_rect, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 38), Stroke::NONE, 12.0);
                ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, if *locked { "🔒" } else { "🔓" }, FontId::new(20.0, FontFamily::Proportional), color);
                
                let badge_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 40.0, rect.top() + 40.0), Vec2::new(50.0, 20.0));
                filled_rect(ui, badge_rect, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 25), Stroke::NONE, 10.0);
                ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, if *locked { "Locked" } else { "Active" }, FontId::new(10.0, FontFamily::Proportional), color);

                ui.painter().text(egui::pos2(rect.left() + 20.0, rect.top() + 85.0), egui::Align2::LEFT_CENTER, *name, FontId::new(16.0, FontFamily::Proportional), text_primary());
                ui.painter().text(egui::pos2(rect.left() + 20.0, rect.top() + 105.0), egui::Align2::LEFT_CENTER, *cap, FontId::new(12.0, FontFamily::Proportional), text_muted());

                let bar_bg = egui::Rect::from_min_size(egui::pos2(rect.left() + 20.0, rect.bottom() - 20.0), Vec2::new(rect.width() - 40.0, 4.0));
                filled_rect(ui, bar_bg, bg_card(), Stroke::NONE, 2.0);
                let bar_fg = egui::Rect::from_min_size(egui::pos2(rect.left() + 20.0, rect.bottom() - 20.0), Vec2::new((rect.width() - 40.0) * prog, 4.0));
                filled_rect(ui, bar_fg, color, Stroke::NONE, 2.0);
                
                ui.add_space(12.0);
            }
        });
    });

    ui.add_space(24.0);

    // Quick Actions
    ui.horizontal(|ui| { ui.add_space(pad); ui.label(egui::RichText::new("QUICK ACTIONS").size(12.0).color(crate::theme::text_muted()).strong()); });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let btn_w = (avail.width() - pad * 2.0 - 12.0) / 2.0;
        let btn_h = 60.0;
        let actions = [("🔒", "Lock All", teal_strong()), ("🔓", "Unlock", success_color()), ("📱", "Setup 2FA", Color32::from_rgb(96, 165, 250)), ("✅", "Integrity Check", error_color())];
        
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, btn_h), egui::Sense::click());
                filled_rect(ui, rect, if resp.hovered() { bg_card() } else { bg_surface() }, Stroke::new(1.0, border_default()), 16.0);
                let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 30.0, rect.center().y), Vec2::splat(36.0));
                filled_rect(ui, icon_rect, bg_card(), Stroke::NONE, 10.0);
                ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, actions[0].0, FontId::new(18.0, FontFamily::Proportional), actions[0].2);
                ui.painter().text(egui::pos2(icon_rect.right() + 12.0, rect.center().y), egui::Align2::LEFT_CENTER, actions[0].1, FontId::new(14.0, FontFamily::Proportional), text_primary());
                if resp.clicked() { ctrl.logout(state); }

                let (rect2, resp2) = ui.allocate_exact_size(Vec2::new(btn_w, btn_h), egui::Sense::click());
                filled_rect(ui, rect2, if resp2.hovered() { bg_card() } else { bg_surface() }, Stroke::new(1.0, border_default()), 16.0);
                let icon_rect2 = egui::Rect::from_center_size(egui::pos2(rect2.left() + 30.0, rect2.center().y), Vec2::splat(36.0));
                filled_rect(ui, icon_rect2, bg_card(), Stroke::NONE, 10.0);
                ui.painter().text(icon_rect2.center(), egui::Align2::CENTER_CENTER, actions[1].0, FontId::new(18.0, FontFamily::Proportional), actions[1].2);
                ui.painter().text(egui::pos2(icon_rect2.right() + 12.0, rect2.center().y), egui::Align2::LEFT_CENTER, actions[1].1, FontId::new(14.0, FontFamily::Proportional), text_primary());
                // click action
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, btn_h), egui::Sense::click());
                filled_rect(ui, rect, if resp.hovered() { bg_card() } else { bg_surface() }, Stroke::new(1.0, border_default()), 16.0);
                let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 30.0, rect.center().y), Vec2::splat(36.0));
                filled_rect(ui, icon_rect, bg_card(), Stroke::NONE, 10.0);
                ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, actions[2].0, FontId::new(18.0, FontFamily::Proportional), actions[2].2);
                ui.painter().text(egui::pos2(icon_rect.right() + 12.0, rect.center().y), egui::Align2::LEFT_CENTER, actions[2].1, FontId::new(14.0, FontFamily::Proportional), text_primary());
                if resp.clicked() { if state.totp_enabled { ctrl.disable_totp(state); } else { ctrl.begin_totp_setup(state); } }

                let (rect2, resp2) = ui.allocate_exact_size(Vec2::new(btn_w, btn_h), egui::Sense::click());
                filled_rect(ui, rect2, if resp2.hovered() { bg_card() } else { bg_surface() }, Stroke::new(1.0, border_default()), 16.0);
                let icon_rect2 = egui::Rect::from_center_size(egui::pos2(rect2.left() + 30.0, rect2.center().y), Vec2::splat(36.0));
                filled_rect(ui, icon_rect2, bg_card(), Stroke::NONE, 10.0);
                ui.painter().text(icon_rect2.center(), egui::Align2::CENTER_CENTER, actions[3].0, FontId::new(18.0, FontFamily::Proportional), actions[3].2);
                ui.painter().text(egui::pos2(icon_rect2.right() + 12.0, rect2.center().y), egui::Align2::LEFT_CENTER, actions[3].1, FontId::new(14.0, FontFamily::Proportional), text_primary());
            });
        });
    });

    ui.add_space(24.0);

    // Recent Activity (Files)
    ui.horizontal(|ui| { ui.add_space(pad); ui.label(egui::RichText::new("RECENT ACTIVITY").size(12.0).color(crate::theme::text_muted()).strong()); });
    ui.add_space(12.0);
    
    let mut target_preview: Option<String> = None;
    if state.file_list.is_empty() {
        ui.horizontal(|ui| { ui.add_space(pad); ui.label(egui::RichText::new("Belum ada file di dalam brankas.").color(crate::theme::text_muted())); });
    } else {
        ui.vertical(|ui| {
            for record in state.file_list.iter() {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 68.0), egui::Sense::click());
                    let is_hover = resp.hovered();
                    filled_rect(ui, rect, if is_hover { bg_card() } else { bg_surface() }, Stroke::new(1.0, if is_hover { teal_strong() } else { border_default() }), 16.0);
                    
                    let ext = file_ext(&record.original_name);
                    let (icon, badge) = file_badge(ext);
                    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 34.0, rect.center().y), Vec2::splat(44.0));
                    filled_rect(ui, icon_rect, badge.0, Stroke::NONE, 12.0);
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(22.0, FontFamily::Proportional), badge.1);
                    
                    let name_truncated = if record.original_name.len() > 25 { format!("{}…", &record.original_name[..23]) } else { record.original_name.clone() };
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y - 10.0), egui::Align2::LEFT_CENTER, name_truncated, FontId::new(15.0, FontFamily::Proportional), text_primary());
                    
                    let meta = format!("{} • Encrypted {}", format_size(record.file_size as u64), &record.encrypted_at[..10]);
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y + 10.0), egui::Align2::LEFT_CENTER, meta, FontId::new(12.0, FontFamily::Proportional), text_muted());
                    
                    if is_hover {
                        let del_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 60.0, rect.center().y), Vec2::splat(30.0));
                        let del_resp = ui.allocate_rect(del_rect, egui::Sense::click());
                        ui.painter().text(del_rect.center(), egui::Align2::CENTER_CENTER, "🗑", FontId::new(18.0, FontFamily::Proportional), if del_resp.hovered() { error_color() } else { text_muted() });
                        
                        let icon_resp = ui.allocate_rect(egui::Rect::from_center_size(egui::pos2(rect.right() - 24.0, rect.center().y), Vec2::splat(30.0)), egui::Sense::click());
                        ui.painter().text(icon_resp.rect.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(20.0, FontFamily::Proportional), if icon_resp.hovered() { teal_strong() } else { text_muted() });
                        
                        if del_resp.clicked() {
                            *to_soft_delete = Some(record.id.clone());
                        } else if icon_resp.clicked() || (resp.clicked() && !del_resp.hovered()) {
                            if file_ext(&record.original_name) == "png" || file_ext(&record.original_name) == "jpg" || file_ext(&record.original_name) == "jpeg" || file_ext(&record.original_name) == "txt" {
                                target_preview = Some(record.vault_filename.clone());
                            } else {
                                *to_decrypt = Some(record.vault_filename.clone());
                            }
                        }
                    } else {
                        let action_icon = "🔒";
                        ui.painter().text(egui::pos2(rect.right() - 24.0, rect.center().y), egui::Align2::CENTER_CENTER, action_icon, FontId::new(20.0, FontFamily::Proportional), text_muted());
                        if resp.clicked() {
                            if file_ext(&record.original_name) == "png" || file_ext(&record.original_name) == "jpg" || file_ext(&record.original_name) == "jpeg" || file_ext(&record.original_name) == "txt" {
                                target_preview = Some(record.vault_filename.clone());
                            } else {
                                *to_decrypt = Some(record.vault_filename.clone());
                            }
                        }
                    }
                });
                ui.add_space(8.0);
            }
        });
    }
    if let Some(vault_filename) = target_preview { ctrl.decrypt_to_memory(state, &vault_filename); }
}

fn render_tab_vault(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller, to_decrypt: &mut Option<String>, to_soft_delete: &mut Option<String>) {
    let pad = 20.0;
    ui.add_space(20.0);
    
    // Header & Search
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("Brankas Anda").size(22.0).color(text_primary()).strong());
    });
    ui.add_space(10.0);
    
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let search_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width() - 240.0, 36.0));
        ui.allocate_ui_at_rect(search_rect, |ui| {
            ui.add(egui::TextEdit::singleline(&mut state.vault_search_query)
                .hint_text("🔍 Cari file...")
                .desired_width(search_rect.width())
                .margin(egui::Margin::symmetric(12.0, 8.0)));
        });
        
        ui.add_space(8.0);
        // Sort & View Toggles
        egui::ComboBox::from_id_source("sort_cb")
            .selected_text(match state.vault_sort_by {
                SortOption::DateDesc => "Tanggal (Baru)",
                SortOption::DateAsc  => "Tanggal (Lama)",
                SortOption::NameAsc  => "Nama (A-Z)",
                SortOption::SizeDesc => "Ukuran (Besar)",
            })
            .width(130.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.vault_sort_by, SortOption::DateDesc, "Tanggal (Baru)");
                ui.selectable_value(&mut state.vault_sort_by, SortOption::DateAsc,  "Tanggal (Lama)");
                ui.selectable_value(&mut state.vault_sort_by, SortOption::NameAsc,  "Nama (A-Z)");
                ui.selectable_value(&mut state.vault_sort_by, SortOption::SizeDesc, "Ukuran (Besar)");
            });
        
        ui.add_space(8.0);
        if ui.selectable_label(state.vault_view_mode == ViewMode::List, "📄").clicked() { state.vault_view_mode = ViewMode::List; }
        if ui.selectable_label(state.vault_view_mode == ViewMode::Grid, "🔲").clicked() { state.vault_view_mode = ViewMode::Grid; }
    });
    
    ui.add_space(16.0);
    
    // Filter & Sort Data
    let mut files: Vec<_> = state.file_list.iter().filter(|f| {
        if state.vault_search_query.is_empty() { return true; }
        f.original_name.to_lowercase().contains(&state.vault_search_query.to_lowercase())
    }).collect();
    
    files.sort_by(|a, b| {
        match state.vault_sort_by {
            SortOption::DateDesc => b.encrypted_at.cmp(&a.encrypted_at),
            SortOption::DateAsc  => a.encrypted_at.cmp(&b.encrypted_at),
            SortOption::NameAsc  => a.original_name.to_lowercase().cmp(&b.original_name.to_lowercase()),
            SortOption::SizeDesc => b.file_size.cmp(&a.file_size),
        }
    });

    if files.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("Belum ada file yang cocok.").color(crate::theme::text_muted()));
        });
        return;
    }
    
    // Render files
    let avail = ui.available_rect_before_wrap();
    
    let mut target_preview = None;

    if state.vault_view_mode == ViewMode::List {
        ui.vertical(|ui| {
            for record in files {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 68.0), egui::Sense::click());
                    let is_hover = resp.hovered();
                    filled_rect(ui, rect, if is_hover { bg_card() } else { bg_surface() }, Stroke::new(1.0, if is_hover { teal_strong() } else { border_default() }), 16.0);
                    
                    let ext = file_ext(&record.original_name);
                    let (icon, badge) = file_badge(ext);
                    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 34.0, rect.center().y), Vec2::splat(44.0));
                    filled_rect(ui, icon_rect, badge.0, Stroke::NONE, 12.0);
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(22.0, FontFamily::Proportional), badge.1);
                    
                    let name_truncated = if record.original_name.len() > 30 { format!("{}…", &record.original_name[..28]) } else { record.original_name.clone() };
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y - 10.0), egui::Align2::LEFT_CENTER, name_truncated, FontId::new(15.0, FontFamily::Proportional), text_primary());
                    
                    let meta = format!("{} • Encrypted {}", format_size(record.file_size as u64), &record.encrypted_at[..10]);
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y + 10.0), egui::Align2::LEFT_CENTER, meta, FontId::new(12.0, FontFamily::Proportional), text_muted());
                    
                    if is_hover {
                        let del_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 24.0, rect.center().y), Vec2::splat(30.0));
                        let del_resp = ui.allocate_rect(del_rect, egui::Sense::click());
                        ui.painter().text(del_rect.center(), egui::Align2::CENTER_CENTER, "🗑", FontId::new(18.0, FontFamily::Proportional), if del_resp.hovered() { error_color() } else { text_muted() });
                        
                        let extract_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 64.0, rect.center().y), Vec2::splat(30.0));
                        let extract_resp = ui.allocate_rect(extract_rect, egui::Sense::click());
                        ui.painter().text(extract_rect.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(20.0, FontFamily::Proportional), if extract_resp.hovered() { teal_strong() } else { text_muted() });
                        
                        let mut preview_clicked = false;
                        let preview_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 104.0, rect.center().y), Vec2::splat(30.0));
                        let preview_resp = ui.allocate_rect(preview_rect, egui::Sense::click());
                        ui.painter().text(preview_rect.center(), egui::Align2::CENTER_CENTER, "👁", FontId::new(20.0, FontFamily::Proportional), if preview_resp.hovered() { teal_strong() } else { text_muted() });
                        preview_clicked = preview_resp.clicked();

                        if del_resp.clicked() {
                            *to_soft_delete = Some(record.id.clone());
                        } else if extract_resp.clicked() {
                            *to_decrypt = Some(record.vault_filename.clone());
                        } else if preview_clicked {
                            target_preview = Some(record.vault_filename.clone());
                        }
                    } else if resp.clicked() {
                         // Default action on click the whole card
                         target_preview = Some(record.vault_filename.clone());
                    }
                });
                ui.add_space(8.0);
            }
        });
    } else {
        // Grid View
        ui.horizontal(|ui| { ui.add_space(pad); }); // padding for wrap
        ui.horizontal_wrapped(|ui| {
            ui.add_space(pad);
            let item_width = 100.0;
            let item_height = 120.0;
            for record in files {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(item_width, item_height), egui::Sense::click());
                let is_hover = resp.hovered();
                filled_rect(ui, rect, if is_hover { bg_card() } else { bg_surface() }, Stroke::new(1.0, if is_hover { teal_strong() } else { border_default() }), 16.0);
                
                let ext = file_ext(&record.original_name);
                let (icon, badge) = file_badge(ext);
                let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 40.0), Vec2::splat(50.0));
                filled_rect(ui, icon_rect, badge.0, Stroke::NONE, 14.0);
                ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(28.0, FontFamily::Proportional), badge.1);
                
                let name_truncated = if record.original_name.len() > 12 { format!("{}…", &record.original_name[..10]) } else { record.original_name.clone() };
                ui.painter().text(egui::pos2(rect.center().x, icon_rect.bottom() + 16.0), egui::Align2::CENTER_CENTER, name_truncated, FontId::new(13.0, FontFamily::Proportional), text_primary());
                ui.painter().text(egui::pos2(rect.center().x, icon_rect.bottom() + 32.0), egui::Align2::CENTER_CENTER, format_size(record.file_size as u64), FontId::new(11.0, FontFamily::Proportional), text_muted());
                
                if is_hover {
                    let del_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 16.0, rect.top() + 16.0), Vec2::splat(24.0));
                    let del_resp = ui.allocate_rect(del_rect, egui::Sense::click());
                    ui.painter().circle_filled(del_rect.center(), 12.0, bg_card());
                    ui.painter().text(del_rect.center(), egui::Align2::CENTER_CENTER, "🗑", FontId::new(12.0, FontFamily::Proportional), if del_resp.hovered() { error_color() } else { text_muted() });
                    
                    let extract_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 16.0, rect.top() + 44.0), Vec2::splat(24.0));
                    let extract_resp = ui.allocate_rect(extract_rect, egui::Sense::click());
                    ui.painter().circle_filled(extract_rect.center(), 12.0, bg_card());
                    ui.painter().text(extract_rect.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(12.0, FontFamily::Proportional), if extract_resp.hovered() { teal_strong() } else { text_muted() });

                    let mut preview_clicked = false;
                    let preview_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 16.0, rect.top() + 72.0), Vec2::splat(24.0));
                    let preview_resp = ui.allocate_rect(preview_rect, egui::Sense::click());
                    ui.painter().circle_filled(preview_rect.center(), 12.0, bg_card());
                    ui.painter().text(preview_rect.center(), egui::Align2::CENTER_CENTER, "👁", FontId::new(12.0, FontFamily::Proportional), if preview_resp.hovered() { teal_strong() } else { text_muted() });
                    preview_clicked = preview_resp.clicked();

                    if del_resp.clicked() {
                        *to_soft_delete = Some(record.id.clone());
                    } else if extract_resp.clicked() {
                        *to_decrypt = Some(record.vault_filename.clone());
                    } else if preview_clicked {
                        target_preview = Some(record.vault_filename.clone());
                    }
                } else if resp.clicked() {
                    // Default action on click the whole card
                    target_preview = Some(record.vault_filename.clone());
                }
                ui.add_space(8.0); // space between grid items
            }
        });
    }
    if let Some(vault_filename) = target_preview { ctrl.decrypt_to_memory(state, &vault_filename); }
}

fn draw_pie_chart(ui: &mut egui::Ui, rect: egui::Rect, data: &[(String, f32, Color32)]) {
    let center = rect.center();
    let radius = rect.width().min(rect.height()) / 2.0;
    let mut current_angle: f32 = -std::f32::consts::FRAC_PI_2; // Start from top
    let total: f32 = data.iter().map(|(_, v, _)| v).sum();
    
    if total == 0.0 {
        ui.painter().circle(center, radius, bg_surface(), Stroke::new(1.0, border_default()));
        ui.painter().text(center, egui::Align2::CENTER_CENTER, "Kosong", FontId::new(14.0, FontFamily::Proportional), text_muted());
        return;
    }
    
    for (_, value, color) in data {
        if *value <= 0.0 { continue; }
        let angle_span = (*value / total) * std::f32::consts::PI * 2.0;
        let mut mesh = egui::Mesh::default();
        let center_idx = mesh.vertices.len() as u32;
        mesh.vertices.push(egui::epaint::Vertex {
            pos: center,
            uv: egui::epaint::WHITE_UV,
            color: *color,
        });
        
        let segments = 32.max((angle_span * 10.0) as usize);
        for i in 0..=segments {
            let a = current_angle + angle_span * (i as f32 / segments as f32);
            mesh.vertices.push(egui::epaint::Vertex {
                pos: center + egui::Vec2::new(a.cos() * radius, a.sin() * radius),
                uv: egui::epaint::WHITE_UV,
                color: *color,
            });
            if i > 0 {
                let v_len = mesh.vertices.len() as u32;
                mesh.indices.extend_from_slice(&[center_idx, v_len - 2, v_len - 1]);
            }
        }
        ui.painter().add(mesh);
        current_angle += angle_span;
    }
    
    // Draw inner circle for donut chart look
    ui.painter().circle_filled(center, radius * 0.6, bg_base());
}

fn render_tab_storage(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller) {
    ui.add_space(20.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new("Storage Analysis").size(22.0).color(text_primary()).strong());
    });
    ui.add_space(20.0);

    // Device Storage Bar
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        let avail = ui.available_rect_before_wrap();
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(avail.width() - 20.0, 80.0), egui::Sense::hover());
        filled_rect(ui, rect, bg_surface(), Stroke::new(1.0, border_default()), 16.0);
        
        ui.painter().text(egui::pos2(rect.left() + 20.0, rect.top() + 20.0), egui::Align2::LEFT_CENTER, "📱 Device Storage", FontId::new(14.0, FontFamily::Proportional), text_primary());
        
        let total = state.device_disk_total;
        let free = state.device_disk_free;
        let used = total.saturating_sub(free);
        
        if total > 0 {
            ui.painter().text(egui::pos2(rect.right() - 20.0, rect.top() + 20.0), egui::Align2::RIGHT_CENTER, format!("{} / {}", format_size(used), format_size(total)), FontId::new(12.0, FontFamily::Proportional), text_muted());
            
            let bar_bg = egui::Rect::from_min_size(egui::pos2(rect.left() + 20.0, rect.top() + 50.0), egui::Vec2::new(rect.width() - 40.0, 8.0));
            filled_rect(ui, bar_bg, bg_card(), Stroke::NONE, 4.0);
            
            let fraction = (used as f32 / total as f32).clamp(0.0, 1.0);
            let bar_fg = egui::Rect::from_min_size(egui::pos2(rect.left() + 20.0, rect.top() + 50.0), egui::Vec2::new((rect.width() - 40.0) * fraction, 8.0));
            filled_rect(ui, bar_fg, Color32::from_rgb(96, 165, 250), Stroke::NONE, 4.0);
        } else {
            ui.painter().text(egui::pos2(rect.left() + 20.0, rect.top() + 50.0), egui::Align2::LEFT_CENTER, "Akses penyimpanan dibutuhkan / Izin ditolak", FontId::new(12.0, FontFamily::Proportional), warn_color());
        }
    });

    ui.add_space(30.0);
    ui.vertical_centered(|ui| {
        let vault_total = state.total_vault_size();
        ui.label(egui::RichText::new(format!("Vault Usage: {}", format_size(vault_total))).size(16.0).color(crate::theme::text_muted()));
    });
    
    ui.add_space(20.0);
    
    // Calculate stats
    let mut size_img = 0f32;
    let mut size_vid = 0f32;
    let mut size_doc = 0f32;
    let mut size_oth = 0f32;
    
    for f in &state.file_list {
        let ext = file_ext(&f.original_name);
        match ext {
            "png" | "jpg" | "jpeg" | "gif" | "webp" => size_img += f.file_size as f32,
            "mp4" | "mkv" | "avi" | "mov"           => size_vid += f.file_size as f32,
            "pdf" | "doc" | "docx" | "txt"          => size_doc += f.file_size as f32,
            _                                       => size_oth += f.file_size as f32,
        }
    }
    
    let chart_data = vec![
        ("Gambar".to_string(),  size_img, Color32::from_rgb(250, 190, 88)),
        ("Video".to_string(),   size_vid, Color32::from_rgb(235, 87, 87)),
        ("Dokumen".to_string(), size_doc, teal_strong()),
        ("Lainnya".to_string(), size_oth, Color32::from_rgb(140, 140, 160)),
    ];
    
    let chart_size = 200.0;
    
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(chart_size), egui::Sense::hover());
        draw_pie_chart(ui, rect, &chart_data);
    });
    
    ui.add_space(40.0);
    
    // Legend
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        ui.vertical(|ui| {
            for (label, val, color) in &chart_data {
                if *val > 0.0 {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 7.0, *color);
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(label).color(text_primary()).size(14.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(20.0);
                            ui.label(egui::RichText::new(format_size(*val as u64)).color(crate::theme::text_muted()).size(14.0));
                        });
                    });
                    ui.add_space(12.0);
                }
            }
        });
    });
}

fn render_tab_settings(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let avail = ui.available_rect_before_wrap();
    let pad = 20.0;
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(egui::RichText::new("Pengaturan").size(20.0).color(text_primary()).strong());
        ui.add_space(30.0);
        
        let btn_w = avail.width() - pad*2.0;
        
        ui.horizontal(|ui| {
            ui.add_space(pad);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 60.0), egui::Sense::click());
            filled_rect(ui, rect, if resp.hovered() { bg_card() } else { bg_surface() }, Stroke::new(1.0, border_default()), 16.0);
            ui.painter().text(egui::pos2(rect.left() + 30.0, rect.center().y), egui::Align2::CENTER_CENTER, "📱", FontId::new(20.0, FontFamily::Proportional), Color32::from_rgb(96, 165, 250));
            ui.painter().text(egui::pos2(rect.left() + 60.0, rect.center().y), egui::Align2::LEFT_CENTER, if state.totp_enabled { "Disable 2FA" } else { "Setup 2FA" }, FontId::new(16.0, FontFamily::Proportional), text_primary());
            if resp.clicked() { if state.totp_enabled { ctrl.disable_totp(state); } else { ctrl.begin_totp_setup(state); } }
        });
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            ui.add_space(pad);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 60.0), egui::Sense::click());
            filled_rect(ui, rect, if resp.hovered() { bg_card() } else { bg_surface() }, Stroke::new(1.0, border_default()), 16.0);
            ui.painter().text(egui::pos2(rect.left() + 30.0, rect.center().y), egui::Align2::CENTER_CENTER, "🗑", FontId::new(20.0, FontFamily::Proportional), error_color());
            ui.painter().text(egui::pos2(rect.left() + 60.0, rect.center().y), egui::Align2::LEFT_CENTER, "Recycle Bin", FontId::new(16.0, FontFamily::Proportional), text_primary());
            if resp.clicked() { ctrl.load_deleted_files(state); state.screen = AppScreen::RecycleBin; }
        });
        ui.add_space(10.0);

        // System Trash Scanner
        ui.horizontal(|ui| {
            ui.add_space(pad);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 60.0), egui::Sense::click());
            filled_rect(ui, rect, if resp.hovered() { bg_card() } else { bg_surface() }, Stroke::new(1.0, border_default()), 16.0);
            ui.painter().text(egui::pos2(rect.left() + 30.0, rect.center().y), egui::Align2::CENTER_CENTER, "🔍", FontId::new(20.0, FontFamily::Proportional), Color32::from_rgb(250, 190, 88));
            ui.painter().text(egui::pos2(rect.left() + 60.0, rect.center().y), egui::Align2::LEFT_CENTER, "System Trash Scanner", FontId::new(16.0, FontFamily::Proportional), text_primary());
            if resp.clicked() { ctrl.scan_system_trash(state); state.screen = AppScreen::SystemTrash; }
        });
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            ui.add_space(pad);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 60.0), egui::Sense::click());
            filled_rect(ui, rect, if resp.hovered() { bg_card() } else { bg_surface() }, Stroke::new(1.0, border_default()), 16.0);
            ui.painter().text(egui::pos2(rect.left() + 30.0, rect.center().y), egui::Align2::CENTER_CENTER, "🚪", FontId::new(20.0, FontFamily::Proportional), text_muted());
            ui.painter().text(egui::pos2(rect.left() + 60.0, rect.center().y), egui::Align2::LEFT_CENTER, "Logout / Kunci Vault", FontId::new(16.0, FontFamily::Proportional), text_primary());
            if resp.clicked() { ctrl.logout(state); }
        });
    });
}

fn render_tab_profile(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    ui.add_space(30.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new("Profil & Pengaturan").size(22.0).color(text_primary()).strong());
    });
    
    ui.add_space(30.0);
    let pad = 20.0;
    
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            // Pengaturan Tampilan
            ui.label(egui::RichText::new("Tampilan").color(teal_strong()).strong());
            ui.add_space(8.0);
            
            let mut is_light = state.is_light_mode;
            if ui.checkbox(&mut is_light, "☀ Mode Terang (Light Mode)").changed() {
                state.is_light_mode = is_light;
                crate::theme::set_light_mode(is_light);
            }
            ui.add_space(30.0);

            // Backup Database Section
            ui.label(egui::RichText::new("Data").color(teal_strong()).strong());
            ui.add_space(8.0);
            if teal_btn(ui, "💾  Backup Database", 200.0).clicked() {
                ctrl.backup_database(state);
            }
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Simpan cadangan .db di tempat aman.").size(12.0).color(crate::theme::text_muted()));
            
            ui.add_space(30.0);
            
            // Ubah Password Section
            ui.label(egui::RichText::new("Ubah Password").color(teal_strong()).strong());
            ui.add_space(10.0);
            
            ui.label(egui::RichText::new("Password Lama").size(12.0).color(crate::theme::text_muted()));
            ui.add(egui::TextEdit::singleline(&mut state.profile_old_password).password(true).desired_width(200.0));
            ui.add_space(8.0);
            
            ui.label(egui::RichText::new("Password Baru (min. 4 karakter)").size(12.0).color(crate::theme::text_muted()));
            ui.add(egui::TextEdit::singleline(&mut state.profile_new_password).password(true).desired_width(200.0));
            ui.add_space(8.0);
            
            ui.label(egui::RichText::new("Konfirmasi Password Baru").size(12.0).color(crate::theme::text_muted()));
            ui.add(egui::TextEdit::singleline(&mut state.profile_confirm_password).password(true).desired_width(200.0));
            ui.add_space(12.0);
            
            if teal_btn(ui, "🔑  Ubah Password", 200.0).clicked() {
                ctrl.change_password(state);
            }
            
            if let Some(err) = &state.profile_password_error {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(err).color(error_color()).size(13.0));
            }
            if let Some(msg) = &state.profile_password_success {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(msg).color(teal_light()).size(13.0));
            }
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

    let avail = ui.available_rect_before_wrap();
    let pad   = 28.0;

    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(pad, 28.0))
        .show(ui, |ui| {
            // Back + judul
            ui.horizontal(|ui| {
                let back_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(36.0, 30.0));
                let back_resp = ui.allocate_rect(back_rect, egui::Sense::click());
                filled_rect(ui, back_rect, Color32::TRANSPARENT, Stroke::new(0.5, border_default()), 7.0);
                ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "←",
                                  FontId::new(15.0, FontFamily::Proportional), text_muted());
                if back_resp.clicked() { state.screen = AppScreen::Dashboard; return; }
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Pulihkan file").size(15.0).color(crate::theme::text_body()).strong());
            });

            ui.add_space(24.0);

            // Info card file
            theme::card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let ext           = file_ext(&record.original_name);
                    let (icon, badge) = file_badge(ext);
                    let (badge_alloc, _) = ui.allocate_exact_size(Vec2::splat(34.0), egui::Sense::hover());
                    filled_rect(ui, badge_alloc, badge.0, Stroke::new(0.5, badge.1), 8.0);
                    ui.painter().text(badge_alloc.center(), egui::Align2::CENTER_CENTER, icon,
                                      FontId::new(15.0, FontFamily::Proportional), badge.1);
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&record.original_name)
                            .size(14.0).color(text_primary()).strong());
                        ui.label(egui::RichText::new(format_size(record.file_size as u64))
                            .size(11.0).color(text_dimmed()));
                    });
                });

                ui.add_space(10.0);
                ui.painter().line_segment(
                    [ui.cursor().min, ui.cursor().min + Vec2::new(ui.available_width(), 0.0)],
                    Stroke::new(0.5, Color32::from_rgb(30, 34, 53)),
                );
                ui.add_space(10.0);

                for (k, v) in &[
                    ("Vault file", format!("{}…{}", &record.vault_filename[..8], ".vlt")),
                    ("SHA-256",    format!("{}…", &record.sha256_hash[..8])),
                    ("Dienkripsi", record.encrypted_at.clone()),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(*k).size(11.0).color(crate::theme::text_muted()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(v).size(11.0).color(text_dimmed())
                                .text_style(egui::TextStyle::Monospace));
                        });
                    });
                }
            });

            ui.add_space(20.0);

            // Output name field
            ui.label(egui::RichText::new("Nama file output").size(12.0).color(crate::theme::text_muted()));
            ui.add_space(6.0);
            egui::Frame::none()
                .fill(crate::theme::bg_surface()).stroke(Stroke::new(0.5, border_default()))
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("📤").size(16.0).color(crate::theme::text_muted()));
                        ui.add_space(8.0);
                        ui.add(egui::TextEdit::singleline(&mut state.decrypt_out_name)
                            .desired_width(ui.available_width())
                            .hint_text("Nama file hasil dekripsi")
                            .font(FontId::new(14.0, FontFamily::Proportional))
                            .frame(false));
                    });
                });

            ui.add_space(16.0);

            // Warning banner
            egui::Frame::none()
                .fill(Color32::from_rgb(26, 18, 8))
                .stroke(Stroke::new(0.5, Color32::from_rgb(99, 56, 6)))
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.label(egui::RichText::new("⚠").size(16.0).color(warn_color()));
                        ui.add_space(8.0);
                        ui.add(egui::Label::new(egui::RichText::new(
                            "Hash SHA-256 divalidasi sebelum dekripsi. \
                             File asli dihapus permanen dari vault setelah dipulihkan."
                        ).size(12.0).color(Color32::from_rgb(186, 117, 23))).wrap(true));
                    });
                });

            // Status
            if let Some(s) = &state.status.clone() {
                ui.add_space(12.0);
                let color = if s.success { success_color() } else { error_color() };
                ui.label(egui::RichText::new(&s.text).size(12.0).color(color));
            }

            // Push buttons ke bawah
            let used_h    = ui.cursor().min.y - avail.top();
            let remaining = (avail.height() - used_h - 80.0).max(12.0);
            ui.add_space(remaining);

            // Tombol aksi
            ui.horizontal(|ui| {
                let w         = ui.available_width();
                let cancel_w  = (w - 12.0) * 0.35;
                let confirm_w = (w - 12.0) * 0.65;

                if ghost_btn(ui, "Batal", cancel_w).clicked() {
                    state.screen = AppScreen::Dashboard;
                }
                ui.add_space(12.0);
                if teal_btn(ui, "🔓  Pulihkan file", confirm_w).clicked() {
                    let out_name = if state.decrypt_out_name.trim().is_empty() {
                        record.original_name.clone()
                    } else {
                        state.decrypt_out_name.trim().to_string()
                    };

                    #[cfg(not(target_os = "android"))]
                    let out_dir = FileDialog::new()
                        .set_title("Pilih folder tujuan")
                        .pick_folder();
                    #[cfg(target_os = "android")]
                    let out_dir: Option<std::path::PathBuf> = { state.set_status("Memilih folder tujuan belum didukung di Android", false); None };
                    if let Some(out_dir) = out_dir
                    {
                        let rec = record.clone();
                        ctrl.decrypt_file(state, &rec, out_dir, &out_name);
                    } else {
                        state.set_status("Batal: folder tidak dipilih.", false);
                    }
                }
            });
        });
}

// ── Screen: TOTP Setup (QR code + verifikasi awal) ────────
fn render_totp_setup(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    if let Some(st) = state.totp_setup_time {
        if st.elapsed().as_secs() >= 30 {
            ctrl.begin_totp_setup(state);
            return;
        }
    }
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
                ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "←",
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
                            ).size(12.0).color(teal_light())).wrap(true));
                        });
                    });

                ui.add_space(16.0);

                // QR Code
                if let Some(matrix) = &state.totp_qr {
                    crate::totp::draw_qr(ui, matrix, 200.0);
                    if let Some(st) = state.totp_setup_time {
                        let left = 30u64.saturating_sub(st.elapsed().as_secs());
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(format!("⏳ QR berganti dalam {} detik", left)).size(11.0).color(warn_color()));
                    }
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
                    
                if let Some(st) = state.totp_setup_time {
                    let left = 30u64.saturating_sub(st.elapsed().as_secs());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("⏳ Kunci berganti dalam {} detik", left)).size(11.0).color(warn_color()));
                }

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

    ui.allocate_ui_at_rect(avail, |ui| {
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
            ui.ctx().request_repaint();

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
    ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "←",
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
        .id_source("trash_scroll")
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
                ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "←",
                                  FontId::new(15.0, FontFamily::Proportional), text_muted());
                if back_resp.clicked() { 
                    state.screen = AppScreen::Dashboard; 
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
                        let out_dir: Option<std::path::PathBuf> = { state.set_status("Memilih folder tujuan belum didukung di Android", false); None };
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
    ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "←",
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
        .id_source("system_trash_scroll")
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
                        
                        let display_label = match label.as_str() {
                            "SFT" => "⇧",
                            "DEL" => "⌫",
                            _ => label.as_str(),
                        };
                        
                        let bg_color = if label == "SFT" || label == "DEL" {
                            Color32::from_rgb(45, 50, 60)
                        } else {
                            crate::theme::bg_card()
                        };

                        let btn = egui::Button::new(egui::RichText::new(display_label).size(18.0).color(Color32::WHITE))
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
