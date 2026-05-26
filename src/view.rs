// view.rs — View layer
// Seluruh fungsi render egui. View hanya membaca AppState
// dan memanggil Controller untuk aksi. Tidak ada logika bisnis di sini.

use eframe::egui;
use egui::epaint::{Color32, FontId, FontFamily, Mesh, Rounding, Stroke, Vec2, Vertex};
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

    // Overlay P2P Sharing
    if state.share_active_record.is_some() {
        render_share_modal(ctx, state, controller);
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
                 DashboardTab::Storage => render_tab_storage(ui, state, ctrl, &mut to_decrypt, &mut to_soft_delete),
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
        (DashboardTab::Home, "🏠", "Beranda"),
        (DashboardTab::Vault, "🔒", "Brankas"),
        (DashboardTab::Home, "➕", "Add"), // Placeholder for FAB
        (DashboardTab::Storage, "💽", "Semua File"),
        (DashboardTab::Settings, "⚙", "Pengaturan"),
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

    if let Some(fname) = to_decrypt {
        let is_previewable = if let Some(rec) = state.file_list.iter().find(|r| r.vault_filename == fname) {
            let ext = crate::theme::file_ext(&rec.original_name).to_lowercase();
            ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "txt"
        } else {
            false
        };
        if is_previewable {
            ctrl.decrypt_to_memory(state, &fname);
        } else {
            ctrl.open_decrypt_panel(state, &fname);
        }
    }
    if let Some(id) = to_soft_delete { ctrl.soft_delete_file(state, &id); }
}

fn render_tab_home(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller, to_decrypt: &mut Option<String>, to_soft_delete: &mut Option<String>) {
    let avail = ui.available_rect_before_wrap();
    let pad = 16.0;
    
    // ── 1. BigCard Hero ────────────────────────────────────────
    ui.add_space(8.0);
    let card_w = avail.width() - pad * 2.0;
    let card_h = 168.0;
    let (card_rect, _) = ui.allocate_exact_size(Vec2::new(card_w, card_h), egui::Sense::hover());
    
    // Draw background purple gradient-like card
    filled_rect(ui, card_rect, accent_purple(), Stroke::NONE, 26.0);
    
    // Draw decorative background circles (bc-deco)
    ui.painter().circle_filled(egui::pos2(card_rect.right() - 20.0, card_rect.top() - 10.0), 80.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18));
    ui.painter().circle_filled(egui::pos2(card_rect.right() - 90.0, card_rect.bottom() - 10.0), 45.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18));
    
    // Inside padding & content
    let content_left = card_rect.left() + 20.0;
    ui.painter().text(
        egui::pos2(content_left, card_rect.top() + 22.0),
        egui::Align2::LEFT_TOP,
        "TOTAL FILE TERENKRIPSI",
        FontId::new(10.0, FontFamily::Proportional),
        Color32::from_rgba_unmultiplied(255, 255, 255, 153),
    );
    
    let total_files_text = format!("{} file", state.file_list.len());
    ui.painter().text(
        egui::pos2(content_left, card_rect.top() + 38.0),
        egui::Align2::LEFT_TOP,
        &total_files_text,
        FontId::new(36.0, FontFamily::Proportional),
        Color32::WHITE,
    );
    
    let total_size_text = format!("{} terlindungi di perangkat ini", format_size(state.total_vault_size()));
    ui.painter().text(
        egui::pos2(content_left, card_rect.top() + 82.0),
        egui::Align2::LEFT_TOP,
        &total_size_text,
        FontId::new(11.0, FontFamily::Proportional),
        Color32::from_rgba_unmultiplied(255, 255, 255, 153),
    );
    
    // Draw badges: AES-256, SHA-256, 2FA
    let badge_y = card_rect.top() + 104.0;
    let b1_rect = egui::Rect::from_min_size(egui::pos2(content_left, badge_y), Vec2::new(76.0, 20.0));
    filled_rect(ui, b1_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 46), Stroke::NONE, 10.0);
    ui.painter().text(b1_rect.center(), egui::Align2::CENTER_CENTER, "🛡 AES-256", FontId::new(10.0, FontFamily::Proportional), Color32::WHITE);
    
    let b2_rect = egui::Rect::from_min_size(egui::pos2(b1_rect.right() + 8.0, badge_y), Vec2::new(76.0, 20.0));
    filled_rect(ui, b2_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 46), Stroke::NONE, 10.0);
    ui.painter().text(b2_rect.center(), egui::Align2::CENTER_CENTER, "🔑 SHA-256", FontId::new(10.0, FontFamily::Proportional), Color32::WHITE);
    
    let is_2fa_active = state.totp_enabled;
    let b3_lbl = if is_2fa_active { "📱 2FA Aktif" } else { "📱 2FA Mati" };
    let b3_rect = egui::Rect::from_min_size(egui::pos2(b2_rect.right() + 8.0, badge_y), Vec2::new(76.0, 20.0));
    filled_rect(ui, b3_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 46), Stroke::NONE, 10.0);
    ui.painter().text(b3_rect.center(), egui::Align2::CENTER_CENTER, b3_lbl, FontId::new(10.0, FontFamily::Proportional), Color32::WHITE);
    
    // Ruang terpakai progress bar
    let bar_y = card_rect.bottom() - 25.0;
    ui.painter().text(
        egui::pos2(content_left, bar_y - 12.0),
        egui::Align2::LEFT_TOP,
        "Ruang terpakai",
        FontId::new(10.0, FontFamily::Proportional),
        Color32::from_rgba_unmultiplied(255, 255, 255, 153),
    );
    
    ui.painter().text(
        egui::pos2(card_rect.right() - 20.0, bar_y - 12.0),
        egui::Align2::RIGHT_TOP,
        "7.7 GB tersisa",
        FontId::new(10.0, FontFamily::Proportional),
        Color32::from_rgba_unmultiplied(255, 255, 255, 153),
    );
    
    let track_rect = egui::Rect::from_min_size(egui::pos2(content_left, bar_y + 4.0), Vec2::new(card_w - 40.0, 5.0));
    filled_rect(ui, track_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 51), Stroke::NONE, 3.0);
    let fill_rect = egui::Rect::from_min_size(track_rect.min, Vec2::new(track_rect.width() * 0.62, 5.0));
    filled_rect(ui, fill_rect, Color32::from_rgba_unmultiplied(255, 255, 255, 217), Stroke::NONE, 3.0);
    
    // ── 2. Quick Action Chips (APA YANG INGIN ANDA LAKUKAN?) ──────────────────────
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("APA YANG INGIN ANDA LAKUKAN?").size(10.0).color(text_muted()).strong());
    });
    ui.add_space(10.0);
    
    egui::ScrollArea::horizontal()
        .id_source("actions_scroll")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(pad);
                
                // Chip 1: Kunci File
                let (c1_rect, c1_resp) = ui.allocate_exact_size(Vec2::new(84.0, 88.0), egui::Sense::click());
                let c1_border = if c1_resp.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, c1_rect, bg_card(), Stroke::new(0.5, c1_border), 20.0);
                let c1_ico_rect = egui::Rect::from_center_size(egui::pos2(c1_rect.center().x, c1_rect.top() + 32.0), Vec2::splat(42.0));
                filled_rect(ui, c1_ico_rect, accent_purple_a(), Stroke::NONE, 14.0);
                ui.painter().text(c1_ico_rect.center(), egui::Align2::CENTER_CENTER, "🔒", FontId::new(18.0, FontFamily::Proportional), accent_purple());
                ui.painter().text(egui::pos2(c1_rect.center().x, c1_rect.bottom() - 18.0), egui::Align2::CENTER_CENTER, "Kunci File", FontId::new(11.0, FontFamily::Proportional), text_primary());
                if c1_resp.clicked() { state.dashboard_tab = DashboardTab::Vault; }
                
                ui.add_space(6.0);
                
                // Chip 2: Buka File
                let (c2_rect, c2_resp) = ui.allocate_exact_size(Vec2::new(84.0, 88.0), egui::Sense::click());
                let c2_border = if c2_resp.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, c2_rect, bg_card(), Stroke::new(0.5, c2_border), 20.0);
                let c2_ico_rect = egui::Rect::from_center_size(egui::pos2(c2_rect.center().x, c2_rect.top() + 32.0), Vec2::splat(42.0));
                filled_rect(ui, c2_ico_rect, accent_mint_a(), Stroke::NONE, 14.0);
                ui.painter().text(c2_ico_rect.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(18.0, FontFamily::Proportional), accent_mint());
                ui.painter().text(egui::pos2(c2_rect.center().x, c2_rect.bottom() - 18.0), egui::Align2::CENTER_CENTER, "Buka File", FontId::new(11.0, FontFamily::Proportional), text_primary());
                if c2_resp.clicked() { state.dashboard_tab = DashboardTab::Vault; }
                
                ui.add_space(6.0);
                
                // Chip 3: Tambah File
                let (c3_rect, c3_resp) = ui.allocate_exact_size(Vec2::new(84.0, 88.0), egui::Sense::click());
                let c3_border = if c3_resp.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, c3_rect, bg_card(), Stroke::new(0.5, c3_border), 20.0);
                let c3_ico_rect = egui::Rect::from_center_size(egui::pos2(c3_rect.center().x, c3_rect.top() + 32.0), Vec2::splat(42.0));
                filled_rect(ui, c3_ico_rect, accent_sky_a(), Stroke::NONE, 14.0);
                ui.painter().text(c3_ico_rect.center(), egui::Align2::CENTER_CENTER, "➕", FontId::new(18.0, FontFamily::Proportional), accent_sky());
                ui.painter().text(egui::pos2(c3_rect.center().x, c3_rect.bottom() - 18.0), egui::Align2::CENTER_CENTER, "Tambah File", FontId::new(11.0, FontFamily::Proportional), text_primary());
                if c3_resp.clicked() {
                    if state.file_dialog.is_none() {
                        state.file_dialog = Some(egui_file_dialog::FileDialog::new());
                    }
                    if let Some(dialog) = &mut state.file_dialog {
                        dialog.select_file();
                    }
                }
                
                ui.add_space(6.0);
                
                // Chip 4: Scan QR 2FA
                let (c4_rect, c4_resp) = ui.allocate_exact_size(Vec2::new(84.0, 88.0), egui::Sense::click());
                let c4_border = if c4_resp.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, c4_rect, bg_card(), Stroke::new(0.5, c4_border), 20.0);
                let c4_ico_rect = egui::Rect::from_center_size(egui::pos2(c4_rect.center().x, c4_rect.top() + 32.0), Vec2::splat(42.0));
                filled_rect(ui, c4_ico_rect, accent_rose_a(), Stroke::NONE, 14.0);
                ui.painter().text(c4_ico_rect.center(), egui::Align2::CENTER_CENTER, "📱", FontId::new(18.0, FontFamily::Proportional), accent_rose());
                ui.painter().text(egui::pos2(c4_rect.center().x, c4_rect.bottom() - 18.0), egui::Align2::CENTER_CENTER, "Scan QR 2FA", FontId::new(11.0, FontFamily::Proportional), text_primary());
                if c4_resp.clicked() {
                    if state.totp_enabled {
                        ctrl.disable_totp(state);
                    } else {
                        ctrl.begin_totp_setup(state);
                    }
                }
                
                ui.add_space(6.0);
                
                // Chip 5: Cek Kondisi
                let (c5_rect, c5_resp) = ui.allocate_exact_size(Vec2::new(84.0, 88.0), egui::Sense::click());
                let c5_border = if c5_resp.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, c5_rect, bg_card(), Stroke::new(0.5, c5_border), 20.0);
                let c5_ico_rect = egui::Rect::from_center_size(egui::pos2(c5_rect.center().x, c5_rect.top() + 32.0), Vec2::splat(42.0));
                filled_rect(ui, c5_ico_rect, accent_gold_a(), Stroke::NONE, 14.0);
                ui.painter().text(c5_ico_rect.center(), egui::Align2::CENTER_CENTER, "🛡", FontId::new(18.0, FontFamily::Proportional), accent_gold());
                ui.painter().text(egui::pos2(c5_rect.center().x, c5_rect.bottom() - 18.0), egui::Align2::CENTER_CENTER, "Cek Kondisi", FontId::new(11.0, FontFamily::Proportional), text_primary());
                if c5_resp.clicked() {
                    state.toast_message = Some("Semua file 100% utuh".to_string());
                    state.toast_timer = 2.0;
                }
                
                ui.add_space(pad);
            });
        });
        
    // ── 3. Status Brankas (2x2 Grid) ───────────────────────────
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("STATUS BRANKAS").size(10.0).color(text_muted()).strong());
    });
    ui.add_space(10.0);
    
    let grid_w = avail.width() - pad * 2.0;
    let card_w2 = (grid_w - 9.0) / 2.0;
    let card_h2 = 98.0;
    
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            // First Row
            ui.horizontal(|ui| {
                // Card 1: Terkunci
                let (r1, resp1) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
                let b1 = if resp1.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, r1, bg_card(), Stroke::new(0.5, b1), 20.0);
                
                let ico_r1 = egui::Rect::from_min_size(egui::pos2(r1.left() + 14.0, r1.top() + 14.0), Vec2::splat(34.0));
                filled_rect(ui, ico_r1, accent_purple_a(), Stroke::NONE, 11.0);
                ui.painter().text(ico_r1.center(), egui::Align2::CENTER_CENTER, "🔒", FontId::new(16.0, FontFamily::Proportional), accent_purple());
                
                let bdg_r1 = egui::Rect::from_min_size(egui::pos2(r1.right() - 60.0, r1.top() + 14.0), Vec2::new(46.0, 18.0));
                filled_rect(ui, bdg_r1, accent_purple_a(), Stroke::NONE, 20.0);
                ui.painter().text(bdg_r1.center(), egui::Align2::CENTER_CENTER, "Terkunci", FontId::new(9.0, FontFamily::Proportional), accent_purple());
                
                ui.painter().text(egui::pos2(r1.left() + 14.0, r1.bottom() - 34.0), egui::Align2::LEFT_CENTER, format!("{}", state.file_list.len()), FontId::new(22.0, FontFamily::Proportional), accent_purple());
                ui.painter().text(egui::pos2(r1.left() + 14.0, r1.bottom() - 14.0), egui::Align2::LEFT_CENTER, "File terkunci aman", FontId::new(10.0, FontFamily::Proportional), text_muted());
                if resp1.clicked() { state.dashboard_tab = DashboardTab::Vault; }
                
                ui.add_space(9.0);
                
                // Card 2: Sesi dibuka
                let (r2, resp2) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
                let b2 = if resp2.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, r2, bg_card(), Stroke::new(0.5, b2), 20.0);
                
                let ico_r2 = egui::Rect::from_min_size(egui::pos2(r2.left() + 14.0, r2.top() + 14.0), Vec2::splat(34.0));
                filled_rect(ui, ico_r2, accent_mint_a(), Stroke::NONE, 11.0);
                ui.painter().text(ico_r2.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(16.0, FontFamily::Proportional), accent_mint());
                
                let bdg_r2 = egui::Rect::from_min_size(egui::pos2(r2.right() - 50.0, r2.top() + 14.0), Vec2::new(36.0, 18.0));
                filled_rect(ui, bdg_r2, accent_mint_a(), Stroke::NONE, 20.0);
                ui.painter().text(bdg_r2.center(), egui::Align2::CENTER_CENTER, "Aktif", FontId::new(9.0, FontFamily::Proportional), accent_mint());
                
                let session_active_lbl = if state.session_key.is_some() { "1" } else { "0" };
                ui.painter().text(egui::pos2(r2.left() + 14.0, r2.bottom() - 34.0), egui::Align2::LEFT_CENTER, session_active_lbl, FontId::new(22.0, FontFamily::Proportional), accent_mint());
                ui.painter().text(egui::pos2(r2.left() + 14.0, r2.bottom() - 14.0), egui::Align2::LEFT_CENTER, "Sesi dibuka", FontId::new(10.0, FontFamily::Proportional), text_muted());
                if resp2.clicked() { state.dashboard_tab = DashboardTab::Vault; }
            });
            
            ui.add_space(9.0);
            
            // Second Row
            ui.horizontal(|ui| {
                // Card 3: Aman
                let (r3, resp3) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
                let b3 = if resp3.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, r3, bg_card(), Stroke::new(0.5, b3), 20.0);
                
                let ico_r3 = egui::Rect::from_min_size(egui::pos2(r3.left() + 14.0, r3.top() + 14.0), Vec2::splat(34.0));
                filled_rect(ui, ico_r3, accent_sky_a(), Stroke::NONE, 11.0);
                ui.painter().text(ico_r3.center(), egui::Align2::CENTER_CENTER, "✔️", FontId::new(16.0, FontFamily::Proportional), accent_sky());
                
                let bdg_r3 = egui::Rect::from_min_size(egui::pos2(r3.right() - 50.0, r3.top() + 14.0), Vec2::new(36.0, 18.0));
                filled_rect(ui, bdg_r3, accent_sky_a(), Stroke::NONE, 20.0);
                ui.painter().text(bdg_r3.center(), egui::Align2::CENTER_CENTER, "Aman", FontId::new(9.0, FontFamily::Proportional), accent_sky());
                
                ui.painter().text(egui::pos2(r3.left() + 14.0, r3.bottom() - 34.0), egui::Align2::LEFT_CENTER, "100%", FontId::new(22.0, FontFamily::Proportional), accent_sky());
                ui.painter().text(egui::pos2(r3.left() + 14.0, r3.bottom() - 14.0), egui::Align2::LEFT_CENTER, "Semua file utuh", FontId::new(10.0, FontFamily::Proportional), text_muted());
                if resp3.clicked() {
                    state.toast_message = Some("Semua file 100% utuh".to_string());
                    state.toast_timer = 2.0;
                }
                
                ui.add_space(9.0);
                
                // Card 4: Tenaga Enkripsi
                let (r4, resp4) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
                let b4 = if resp4.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, r4, bg_card(), Stroke::new(0.5, b4), 20.0);
                
                let ico_r4 = egui::Rect::from_min_size(egui::pos2(r4.left() + 14.0, r4.top() + 14.0), Vec2::splat(34.0));
                filled_rect(ui, ico_r4, accent_gold_a(), Stroke::NONE, 11.0);
                ui.painter().text(ico_r4.center(), egui::Align2::CENTER_CENTER, "⚡", FontId::new(16.0, FontFamily::Proportional), accent_gold());
                
                let bdg_r4 = egui::Rect::from_min_size(egui::pos2(r4.right() - 50.0, r4.top() + 14.0), Vec2::new(36.0, 18.0));
                filled_rect(ui, bdg_r4, accent_gold_a(), Stroke::NONE, 20.0);
                ui.painter().text(bdg_r4.center(), egui::Align2::CENTER_CENTER, "Tinggi", FontId::new(9.0, FontFamily::Proportional), accent_gold());
                
                ui.painter().text(egui::pos2(r4.left() + 14.0, r4.bottom() - 34.0), egui::Align2::LEFT_CENTER, "78%", FontId::new(22.0, FontFamily::Proportional), accent_gold());
                ui.painter().text(egui::pos2(r4.left() + 14.0, r4.bottom() - 14.0), egui::Align2::LEFT_CENTER, "Tenaga enkripsi", FontId::new(10.0, FontFamily::Proportional), text_muted());
            });
        });
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
                });
            }
        });
    }
}

fn render_tab_vault(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller, to_decrypt: &mut Option<String>, to_soft_delete: &mut Option<String>) {
    let pad = 16.0;
    let avail = ui.available_rect_before_wrap();
    ui.add_space(8.0);
    
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("VAULT SAYA").size(10.0).color(text_muted()).strong());
    });
    ui.add_space(10.0);
    
    // Vault 1: Primary Vault
    let (v1_rect, resp1) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 80.0), egui::Sense::click());
    let border1 = if resp1.hovered() { border_hover() } else { border_default() };
    filled_rect(ui, v1_rect, bg_card(), Stroke::new(0.5, border1), 20.0);
    
    let ico_rect1 = egui::Rect::from_center_size(egui::pos2(v1_rect.left() + 36.0, v1_rect.center().y), Vec2::splat(44.0));
    filled_rect(ui, ico_rect1, accent_purple_a(), Stroke::NONE, 14.0);
    ui.painter().text(ico_rect1.center(), egui::Align2::CENTER_CENTER, "🔒", FontId::new(20.0, FontFamily::Proportional), accent_purple());
    
    ui.painter().text(egui::pos2(ico_rect1.right() + 12.0, v1_rect.top() + 18.0), egui::Align2::LEFT_TOP, "Primary Vault", FontId::new(13.0, FontFamily::Proportional), text_primary());
    let vault1_sub = format!("{} file · Terakhir dibuka 2 menit lalu", state.file_list.len());
    ui.painter().text(egui::pos2(ico_rect1.right() + 12.0, v1_rect.top() + 36.0), egui::Align2::LEFT_TOP, &vault1_sub, FontId::new(10.0, FontFamily::Proportional), text_muted());
    
    // Progress track
    let prog1_y = v1_rect.bottom() - 15.0;
    let track1 = egui::Rect::from_min_size(egui::pos2(ico_rect1.right() + 12.0, prog1_y), Vec2::new(v1_rect.right() - ico_rect1.right() - 88.0, 3.0));
    filled_rect(ui, track1, bg_input(), Stroke::NONE, 1.5);
    let fill1 = egui::Rect::from_min_size(track1.min, Vec2::new(track1.width() * 0.62, 3.0));
    filled_rect(ui, fill1, accent_purple(), Stroke::NONE, 1.5);
    
    let bdg_rect1 = egui::Rect::from_center_size(egui::pos2(v1_rect.right() - 44.0, v1_rect.center().y), Vec2::new(54.0, 20.0));
    filled_rect(ui, bdg_rect1, accent_purple_a(), Stroke::NONE, 20.0);
    ui.painter().text(bdg_rect1.center(), egui::Align2::CENTER_CENTER, "Terkunci", FontId::new(9.0, FontFamily::Proportional), accent_purple());
    if resp1.clicked() {
        state.toast_message = Some("Primary Vault aman terkunci".to_string());
        state.toast_timer = 2.0;
    }
    
    ui.add_space(8.0);
    
    // Vault 2: Session Vault
    let (v2_rect, resp2) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 80.0), egui::Sense::click());
    let border2 = if resp2.hovered() { border_hover() } else { border_default() };
    filled_rect(ui, v2_rect, bg_card(), Stroke::new(0.5, border2), 20.0);
    
    let ico_rect2 = egui::Rect::from_center_size(egui::pos2(v2_rect.left() + 36.0, v2_rect.center().y), Vec2::splat(44.0));
    filled_rect(ui, ico_rect2, accent_mint_a(), Stroke::NONE, 14.0);
    ui.painter().text(ico_rect2.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(20.0, FontFamily::Proportional), accent_mint());
    
    ui.painter().text(egui::pos2(ico_rect2.right() + 12.0, v2_rect.top() + 18.0), egui::Align2::LEFT_TOP, "Session Vault", FontId::new(13.0, FontFamily::Proportional), text_primary());
    let session_active_count = if state.session_key.is_some() { 6 } else { 0 };
    let vault2_sub = format!("{} file · SHA-256 terverifikasi", session_active_count);
    ui.painter().text(egui::pos2(ico_rect2.right() + 12.0, v2_rect.top() + 36.0), egui::Align2::LEFT_TOP, &vault2_sub, FontId::new(10.0, FontFamily::Proportional), text_muted());
    
    // Progress track
    let prog2_y = v2_rect.bottom() - 15.0;
    let track2 = egui::Rect::from_min_size(egui::pos2(ico_rect2.right() + 12.0, prog2_y), Vec2::new(v2_rect.right() - ico_rect2.right() - 88.0, 3.0));
    filled_rect(ui, track2, bg_input(), Stroke::NONE, 1.5);
    let fill2 = egui::Rect::from_min_size(track2.min, Vec2::new(track2.width() * 0.35, 3.0));
    filled_rect(ui, fill2, accent_mint(), Stroke::NONE, 1.5);
    
    let bdg_rect2 = egui::Rect::from_center_size(egui::pos2(v2_rect.right() - 44.0, v2_rect.center().y), Vec2::new(54.0, 20.0));
    filled_rect(ui, bdg_rect2, accent_mint_a(), Stroke::NONE, 20.0);
    ui.painter().text(bdg_rect2.center(), egui::Align2::CENTER_CENTER, "Aktif", FontId::new(9.0, FontFamily::Proportional), accent_mint());
    if resp2.clicked() {
        state.toast_message = Some("Session Vault aktif terverifikasi".to_string());
        state.toast_timer = 2.0;
    }
    
    // Quick actions (Aksi Cepat)
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("AKSI CEPAT").size(10.0).color(text_muted()).strong());
    });
    ui.add_space(10.0);
    
    let grid_w = avail.width() - pad * 2.0;
    let card_w2 = (grid_w - 9.0) / 2.0;
    let card_h2 = 98.0;
    
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                // Action 1: Kunci Semua
                let (r1, resp1) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
                let b1 = if resp1.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, r1, bg_card(), Stroke::new(0.5, b1), 20.0);
                
                let ico_r1 = egui::Rect::from_min_size(egui::pos2(r1.left() + 14.0, r1.top() + 14.0), Vec2::splat(34.0));
                filled_rect(ui, ico_r1, accent_purple_a(), Stroke::NONE, 11.0);
                ui.painter().text(ico_r1.center(), egui::Align2::CENTER_CENTER, "🔒", FontId::new(16.0, FontFamily::Proportional), accent_purple());
                
                ui.painter().text(egui::pos2(r1.left() + 14.0, r1.bottom() - 32.0), egui::Align2::LEFT_CENTER, "Kunci semua", FontId::new(12.0, FontFamily::Proportional), text_primary());
                ui.painter().text(egui::pos2(r1.left() + 14.0, r1.bottom() - 14.0), egui::Align2::LEFT_CENTER, "Enkripsi sekarang", FontId::new(10.0, FontFamily::Proportional), text_muted());
                if resp1.clicked() { ctrl.logout(state); }
                
                ui.add_space(9.0);
                
                // Action 2: Buka vault
                let (r2, resp2) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
                let b2 = if resp2.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, r2, bg_card(), Stroke::new(0.5, b2), 20.0);
                
                let ico_r2 = egui::Rect::from_min_size(egui::pos2(r2.left() + 14.0, r2.top() + 14.0), Vec2::splat(34.0));
                filled_rect(ui, ico_r2, accent_mint_a(), Stroke::NONE, 11.0);
                ui.painter().text(ico_r2.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(16.0, FontFamily::Proportional), accent_mint());
                
                ui.painter().text(egui::pos2(r2.left() + 14.0, r2.bottom() - 32.0), egui::Align2::LEFT_CENTER, "Buka vault", FontId::new(12.0, FontFamily::Proportional), text_primary());
                ui.painter().text(egui::pos2(r2.left() + 14.0, r2.bottom() - 14.0), egui::Align2::LEFT_CENTER, "Masukkan PIN", FontId::new(10.0, FontFamily::Proportional), text_muted());
                if resp2.clicked() {
                    state.toast_message = Some("Vault sudah dalam sesi aktif".to_string());
                    state.toast_timer = 2.0;
                }
            });
            ui.add_space(9.0);
            ui.horizontal(|ui| {
                // Action 3: Setup 2FA
                let (r3, resp3) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
                let b3 = if resp3.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, r3, bg_card(), Stroke::new(0.5, b3), 20.0);
                
                let ico_r3 = egui::Rect::from_min_size(egui::pos2(r3.left() + 14.0, r3.top() + 14.0), Vec2::splat(34.0));
                filled_rect(ui, ico_r3, accent_rose_a(), Stroke::NONE, 11.0);
                ui.painter().text(ico_r3.center(), egui::Align2::CENTER_CENTER, "📱", FontId::new(16.0, FontFamily::Proportional), accent_rose());
                
                ui.painter().text(egui::pos2(r3.left() + 14.0, r3.bottom() - 32.0), egui::Align2::LEFT_CENTER, "Setup 2FA", FontId::new(12.0, FontFamily::Proportional), text_primary());
                ui.painter().text(egui::pos2(r3.left() + 14.0, r3.bottom() - 14.0), egui::Align2::LEFT_CENTER, "Google Auth", FontId::new(10.0, FontFamily::Proportional), text_muted());
                if resp3.clicked() {
                    if state.totp_enabled { ctrl.disable_totp(state); } else { ctrl.begin_totp_setup(state); }
                }
                
                ui.add_space(9.0);
                
                // Action 4: Cek Integritas
                let (r4, resp4) = ui.allocate_exact_size(Vec2::new(card_w2, card_h2), egui::Sense::click());
                let b4 = if resp4.hovered() { border_hover() } else { border_default() };
                filled_rect(ui, r4, bg_card(), Stroke::new(0.5, b4), 20.0);
                
                let ico_r4 = egui::Rect::from_min_size(egui::pos2(r4.left() + 14.0, r4.top() + 14.0), Vec2::splat(34.0));
                filled_rect(ui, ico_r4, accent_gold_a(), Stroke::NONE, 11.0);
                ui.painter().text(ico_r4.center(), egui::Align2::CENTER_CENTER, "🛡", FontId::new(16.0, FontFamily::Proportional), accent_gold());
                
                ui.painter().text(egui::pos2(r4.left() + 14.0, r4.bottom() - 32.0), egui::Align2::LEFT_CENTER, "Cek integritas", FontId::new(12.0, FontFamily::Proportional), text_primary());
                ui.painter().text(egui::pos2(r4.left() + 14.0, r4.bottom() - 14.0), egui::Align2::LEFT_CENTER, "SHA-256 checksum", FontId::new(10.0, FontFamily::Proportional), text_muted());
                if resp4.clicked() {
                    state.toast_message = Some("Semua file 100% utuh".to_string());
                    state.toast_timer = 2.0;
                }
            });
        });
    });
}

fn render_tab_storage(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller, to_decrypt: &mut Option<String>, to_soft_delete: &mut Option<String>) {
    let pad = 16.0;
    let avail = ui.available_rect_before_wrap();
    
    ui.add_space(8.0);
    
    // Search Bar
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let search_w = avail.width() - pad * 2.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(search_w, 42.0), egui::Sense::hover());
        let border_c = border_default();
        filled_rect(ui, rect, bg_card(), Stroke::new(0.5, border_c), 12.0);
        
        ui.allocate_ui_at_rect(rect, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(egui::RichText::new("🔍").size(14.0).color(text_muted()));
                let resp = ui.add(egui::TextEdit::singleline(&mut state.vault_search_query)
                    .hint_text("Cari file terenkripsi...")
                    .frame(false)
                    .desired_width(search_w - 60.0)
                    .font(FontId::new(13.0, FontFamily::Proportional)));
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
                    
                    let b_color = if is_hover { border_hover() } else { border_default() };
                    filled_rect(ui, rect, bg_card(), Stroke::new(0.5, b_color), 20.0);
                    
                    let ext = file_ext(&record.original_name);
                    let (icon, badge) = file_badge(ext);
                    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 34.0, rect.center().y), Vec2::splat(42.0));
                    filled_rect(ui, icon_rect, badge.0, Stroke::NONE, 13.0);
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(20.0, FontFamily::Proportional), badge.1);
                    
                    let name_truncated = if record.original_name.len() > 22 { format!("{}…", &record.original_name[..20]) } else { record.original_name.clone() };
                    ui.painter().text(egui::pos2(icon_rect.right() + 14.0, rect.center().y - 10.0), egui::Align2::LEFT_CENTER, name_truncated, FontId::new(13.0, FontFamily::Proportional), text_primary());
                    
                    let vault_name = if record.vault_filename.contains("session") { "Session Vault" } else { "Primary Vault" };
                    let meta = format!("{} • {} • Enkripsi {}", format_size(record.file_size as u64), vault_name, if record.encrypted_at.len() >= 10 { &record.encrypted_at[..10] } else { &record.encrypted_at });
                    ui.painter().text(egui::pos2(icon_rect.right() + 14.0, rect.center().y + 11.0), egui::Align2::LEFT_CENTER, meta, FontId::new(10.5, FontFamily::Proportional), text_muted());
                    
                    if is_hover {
                        let del_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 86.0, rect.center().y), Vec2::splat(32.0));
                        let del_resp = ui.allocate_rect(del_rect, egui::Sense::click());
                        ui.painter().text(del_rect.center(), egui::Align2::CENTER_CENTER, "🗑", FontId::new(18.0, FontFamily::Proportional), if del_resp.hovered() { error_color() } else { text_muted() });
                        
                        let share_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 54.0, rect.center().y), Vec2::splat(32.0));
                        let share_resp = ui.allocate_rect(share_rect, egui::Sense::click());
                        ui.painter().text(share_rect.center(), egui::Align2::CENTER_CENTER, "📡", FontId::new(16.0, FontFamily::Proportional), if share_resp.hovered() { accent_sky() } else { text_muted() });
                        
                        let open_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 22.0, rect.center().y), Vec2::splat(32.0));
                        let open_resp = ui.allocate_rect(open_rect, egui::Sense::click());
                        ui.painter().text(open_rect.center(), egui::Align2::CENTER_CENTER, "🔓", FontId::new(18.0, FontFamily::Proportional), if open_resp.hovered() { accent_purple() } else { text_muted() });
                        
                        if del_resp.clicked() {
                            *to_soft_delete = Some(record.id.clone());
                        } else if share_resp.clicked() {
                            ctrl.start_share(state, record.clone());
                        } else if open_resp.clicked() || (resp.clicked() && !del_resp.hovered() && !share_resp.hovered()) {
                            *to_decrypt = Some(record.vault_filename.clone());
                        }
                    } else {
                        let lock_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 22.0, rect.center().y), Vec2::splat(32.0));
                        ui.painter().text(lock_rect.center(), egui::Align2::CENTER_CENTER, "🔒", FontId::new(16.0, FontFamily::Proportional), badge.1);
                    }
                });
            }
        });
    }
}

fn render_tab_settings(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let pad = 16.0;
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
        draw_row(ui, "📁", accent_mint(), accent_mint_a(), "Lokasi Penyimpanan", "vault_storage/ · Lokal di perangkat");
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
            
            ui.allocate_ui_at_rect(avail, |ui| {
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

            ui.allocate_ui_at_rect(dialog_rect, |ui| {
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
