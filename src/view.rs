// view.rs — View layer
// Seluruh fungsi render egui. View hanya membaca AppState
// dan memanggil Controller untuk aksi. Tidak ada logika bisnis di sini.

use eframe::egui;
use egui::epaint::{Color32, FontId, FontFamily, Mesh, Rounding, Stroke, Vec2, Vertex};
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
                    filled_rect(ui, rect, Color32::from_rgb(20, 25, 35), Stroke::new(1.0, TEAL_STRONG), 22.0);
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
                AppScreen::SetupPin          => render_setup_pin(ui, state, controller),
                AppScreen::Dashboard         => render_dashboard(ui, state, controller),
                AppScreen::Decrypting(fname) => render_decrypt_panel(ui, state, controller, &fname.clone()),
                AppScreen::TotpSetup         => render_totp_setup(ui, state, controller),
                AppScreen::TotpVerify        => render_totp_verify(ui, state, controller),
                AppScreen::RecycleBin        => render_recycle_bin(ui, state, controller),
            }
        });
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
    let pin_set = ctrl.is_pin_set();
    let avail   = ui.available_rect_before_wrap();

    ui.allocate_ui_at_rect(avail, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(44.0);

            // Shield icon
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(56.0), egui::Sense::hover());
            filled_rect(ui, icon_rect, TEAL_DARK, Stroke::NONE, 14.0);
            ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, "🛡",
                              FontId::new(26.0, FontFamily::Proportional), TEAL_FAINT);

            ui.add_space(14.0);
            ui.label(egui::RichText::new("Aegis Vault").size(20.0).color(TEXT_BODY).strong());
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Akses aman ke data kamu").size(13.0).color(TEXT_MUTED));

            if !pin_set {
                ui.add_space(32.0);
                ui.label(egui::RichText::new("Vault baru terdeteksi.").color(WARN_COLOR).size(13.0));
                ui.label(egui::RichText::new("Buat PIN untuk memulai.").color(TEXT_MUTED).size(13.0));
                ui.add_space(20.0);
                if teal_btn(ui, "⚙  Setup PIN", 200.0).clicked() {
                    state.screen = AppScreen::SetupPin;
                }
                return;
            }

            ui.add_space(32.0);

            // PIN dots indicator
            let dot_total   = 6usize;
            let dot_size    = 12.0f32;
            let dot_gap     = 10.0f32;
            let total_w     = dot_total as f32 * dot_size + (dot_total - 1) as f32 * dot_gap;
            let shake_off   = if state.pin_shake_timer > 0.0 {
                (state.pin_shake_timer * 30.0).sin() * 6.0
            } else { 0.0 };

            let (dots_rect, _) = ui.allocate_exact_size(
                Vec2::new(total_w + shake_off.abs() * 2.0, dot_size), egui::Sense::hover()
            );
            for i in 0..dot_total {
                let cx = dots_rect.left() + shake_off + i as f32 * (dot_size + dot_gap) + dot_size / 2.0;
                let cy = dots_rect.center().y;
                let filled = i < state.pin_digits.len();
                let color  = if state.pin_error.is_some() { ERROR_COLOR }
                             else if filled               { TEAL_STRONG }
                             else                         { BORDER_DEFAULT };
                let fill   = if filled { color } else { Color32::TRANSPARENT };
                ui.painter().circle(egui::pos2(cx, cy), dot_size / 2.0, fill, Stroke::new(1.5, color));
            }

            // Error label
            if let Some(err) = &state.pin_error {
                ui.add_space(10.0);
                ui.label(egui::RichText::new(err).color(ERROR_COLOR).size(13.0));
            }

            ui.add_space(24.0);

            // Numpad 3x4
            let keys: &[&[&str]] = &[
                &["1","2","3"],
                &["4","5","6"],
                &["7","8","9"],
                &["⌫","0","hapus"],
            ];
            for row in keys {
                ui.horizontal(|ui| {
                    let total_w = 72.0 * 3.0 + ui.spacing().item_spacing.x * 2.0;
                    ui.add_space((ui.available_width() - total_w) / 2.0);
                    for key in *row {
                        let resp = numpad_btn(ui, key);
                        if resp.clicked() {
                            match *key {
                                "⌫"    => { state.pin_digits.pop(); state.pin_error = None; }
                                "hapus" => { state.pin_digits.clear(); state.pin_error = None; }
                                d => {
                                    if state.pin_digits.len() < 6 {
                                        state.pin_digits.push_str(d);
                                        state.pin_error = None;
                                    }
                                    if state.pin_digits.len() == 6 {
                                        state.pin_input = state.pin_digits.clone();
                                        let ok = ctrl.try_login(state);
                                        if !ok { state.pin_shake_timer = 0.4; }
                                        state.pin_digits.clear();
                                    }
                                }
                            }
                        }
                    }
                });
                ui.add_space(10.0);
            }

            ui.add_space(20.0);
            let link_resp = ui.add(egui::Label::new(
                egui::RichText::new("Lupa PIN? Reset Vault")
                    .size(12.0)
                    .color(TEXT_MUTED)
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
                            ui.label(egui::RichText::new("⚠  Hapus Seluruh Vault?").color(WARN_COLOR).size(18.0).strong());
                            ui.add_space(12.0);
                            ui.label(egui::RichText::new("Tindakan ini akan menghapus semua file yang ada di vault secara permanen karena PIN lama tidak dapat dipulihkan.").color(TEXT_BODY).size(13.0));
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

// ── Screen: Setup PIN ─────────────────────────────────────
fn render_setup_pin(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let avail   = ui.available_rect_before_wrap();
    let field_w = avail.width() - 72.0;

    ui.allocate_ui_at_rect(avail, |ui| {
        ui.add_space(32.0);

        ui.horizontal(|ui| {
            ui.add_space(36.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(38.0), egui::Sense::hover());
            filled_rect(ui, rect, BG_SURFACE, Stroke::new(0.5, BORDER_DEFAULT), 10.0);
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "🔑",
                              FontId::new(18.0, FontFamily::Proportional), TEAL_STRONG);
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Buat PIN baru").size(15.0).color(TEXT_BODY).strong());
                ui.label(egui::RichText::new("Harus 6 digit angka").size(12.0).color(TEXT_MUTED));
            });
        });

        ui.add_space(24.0);

        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(36.0, 0.0))
            .show(ui, |ui| {
                // Field PIN baru
                ui.label(egui::RichText::new("PIN baru").size(12.0).color(TEXT_MUTED));
                ui.add_space(6.0);
                egui::Frame::none()
                    .fill(BG_SURFACE).stroke(Stroke::new(0.5, BORDER_DEFAULT))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🔒").size(16.0).color(TEXT_MUTED));
                            ui.add_space(8.0);
                            ui.add(egui::TextEdit::singleline(&mut state.pin_input)
                                .password(true).hint_text("6 digit angka")
                                .desired_width(field_w - 80.0)
                                .font(FontId::new(16.0, FontFamily::Proportional))
                                .frame(false));
                        });
                    });

                ui.add_space(14.0);

                // Field konfirmasi
                ui.label(egui::RichText::new("Konfirmasi PIN").size(12.0).color(TEXT_MUTED));
                ui.add_space(6.0);
                let accent = if !state.pin_confirm.is_empty() { TEAL_STRONG } else { BORDER_DEFAULT };
                let icon_c = if !state.pin_confirm.is_empty() { TEAL_STRONG } else { TEXT_MUTED };
                egui::Frame::none()
                    .fill(BG_SURFACE).stroke(Stroke::new(0.5, accent))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🔒").size(16.0).color(icon_c));
                            ui.add_space(8.0);
                            ui.add(egui::TextEdit::singleline(&mut state.pin_confirm)
                                .password(true).hint_text("Ulangi PIN")
                                .desired_width(field_w - 80.0)
                                .font(FontId::new(16.0, FontFamily::Proportional))
                                .frame(false));
                        });
                    });

                ui.add_space(16.0);

                // Info banner
                egui::Frame::none()
                    .fill(Color32::from_rgb(12, 31, 24))
                    .stroke(Stroke::new(0.5, BORDER_ACCENT))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.label(egui::RichText::new("ℹ").size(16.0).color(TEAL_STRONG));
                            ui.add_space(8.0);
                            ui.add(egui::Label::new(egui::RichText::new(
                                "PIN di-hash dengan PBKDF2-HMAC-SHA256 (310.000 iterasi) \
                                 dan salt unik. Tidak ada cara memulihkan PIN yang hilang."
                            ).size(12.0).color(TEAL_LIGHT)).wrap(true));
                        });
                    });

                ui.add_space(24.0);

                if let Some(err) = state.pin_error.clone() {
                    ui.label(egui::RichText::new(&err).color(ERROR_COLOR).size(13.0));
                    ui.add_space(8.0);
                }

                if teal_btn(ui, "Simpan PIN & masuk", ui.available_width()).clicked() {
                    ctrl.setup_pin(state);
                }
            });
    });
}

// ── Screen: Dashboard ─────────────────────────────────────
fn render_dashboard(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    let avail = ui.available_rect_before_wrap();
    
    // ─ Topbar ─
    let topbar_h = 60.0;
    let topbar_rect = egui::Rect::from_min_size(avail.min, Vec2::new(avail.width(), topbar_h));
    filled_rect(ui, topbar_rect, Color32::from_rgb(14, 16, 22), Stroke::new(0.5, BORDER_SUBTLE), 0.0);
    
    // Logo
    let brand_pos = egui::pos2(avail.left() + 20.0, topbar_rect.center().y);
    ui.painter().text(brand_pos, egui::Align2::LEFT_CENTER, "Aegis.Vault",
                      FontId::new(22.0, FontFamily::Proportional), TEXT_PRIMARY);
    
    // Topbar Icons
    let notif_rect = egui::Rect::from_center_size(egui::pos2(avail.right() - 60.0, topbar_rect.center().y), Vec2::splat(32.0));
    let notif_resp = ui.allocate_rect(notif_rect, egui::Sense::click());
    ui.painter().text(notif_rect.center(), egui::Align2::CENTER_CENTER, "🔔", FontId::new(18.0, FontFamily::Proportional), if notif_resp.hovered() { TEAL_STRONG } else { TEXT_MUTED });
    if notif_resp.clicked() { 
        state.dashboard_tab = DashboardTab::Notifications; 
        ctrl.load_audit_logs(state);
    }

    let profile_rect = egui::Rect::from_center_size(egui::pos2(avail.right() - 20.0, topbar_rect.center().y), Vec2::splat(32.0));
    let profile_resp = ui.allocate_rect(profile_rect, egui::Sense::click());
    ui.painter().text(profile_rect.center(), egui::Align2::CENTER_CENTER, "👤", FontId::new(18.0, FontFamily::Proportional), if profile_resp.hovered() { TEAL_STRONG } else { TEXT_MUTED });
    if profile_resp.clicked() { state.dashboard_tab = DashboardTab::Profile; }

    // ─ Bottom Navigation ─
    let bottom_h = 80.0;
    let bottom_rect = egui::Rect::from_min_size(
        egui::pos2(avail.left(), avail.bottom() - bottom_h),
        Vec2::new(avail.width(), bottom_h),
    );
    filled_rect(ui, bottom_rect, Color32::from_rgb(18, 18, 17), Stroke::new(1.0, BORDER_SUBTLE), 0.0);
    
    let tab_w = avail.width() / 5.0;
    let mut tab_x = avail.left() + tab_w / 2.0;
    let tab_y = bottom_rect.center().y;
    
    let tabs = [
        (DashboardTab::Home, "🏠", "Home"),
        (DashboardTab::Vault, "🔒", "Vault"),
        (DashboardTab::Home, "➕", "Add"), // Placeholder for FAB
        (DashboardTab::Storage, "💽", "Storage"),
        (DashboardTab::Settings, "⚙️", "Settings"),
    ];
    
    for (i, (tab, icon, label)) in tabs.iter().enumerate() {
        if i == 2 {
            // FAB (Add button)
            let fab_size = Vec2::splat(56.0);
            let fab_rect = egui::Rect::from_center_size(egui::pos2(tab_x, bottom_rect.top() - 10.0), fab_size);
            let fab_resp = ui.allocate_rect(fab_rect, egui::Sense::click());
            let fab_fill = if fab_resp.hovered() { TEAL_LIGHT } else { TEAL_STRONG };
            filled_rect(ui, fab_rect, fab_fill, Stroke::new(4.0, BG_BASE), 28.0);
            ui.painter().text(fab_rect.center(), egui::Align2::CENTER_CENTER, "➕", FontId::new(24.0, FontFamily::Proportional), BG_BASE);
            if fab_resp.clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    ctrl.encrypt_file(state, path);
                }
            }
        } else {
            let item_rect = egui::Rect::from_center_size(egui::pos2(tab_x, tab_y), Vec2::new(tab_w, bottom_h));
            let item_resp = ui.allocate_rect(item_rect, egui::Sense::click());
            let is_active = state.dashboard_tab == *tab;
            let color = if is_active || item_resp.hovered() { TEAL_STRONG } else { TEXT_MUTED };
            
            ui.painter().text(egui::pos2(tab_x, tab_y - 10.0), egui::Align2::CENTER_CENTER, *icon, FontId::new(20.0, FontFamily::Proportional), color);
            ui.painter().text(egui::pos2(tab_x, tab_y + 12.0), egui::Align2::CENTER_CENTER, *label, FontId::new(10.0, FontFamily::Proportional), color);
            
            if item_resp.clicked() {
                state.dashboard_tab = tab.clone();
            }
        }
        tab_x += tab_w;
    }
    
    // ─ Content Area (Scrollable) ─
    let content_rect = egui::Rect::from_min_max(
        egui::pos2(avail.left(), topbar_rect.bottom()),
        egui::pos2(avail.right(), bottom_rect.top()),
    );
    
    let mut to_decrypt: Option<String> = None;
    let mut to_soft_delete: Option<String> = None;

    ui.allocate_ui_at_rect(content_rect, |ui| {
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

    if let Some(fname) = to_decrypt { ctrl.open_decrypt_panel(state, &fname); }
    if let Some(id) = to_soft_delete { ctrl.soft_delete_file(state, &id); }
}

fn render_tab_home(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller, to_decrypt: &mut Option<String>, _to_soft_delete: &mut Option<String>) {
    let avail = ui.available_rect_before_wrap();
    let pad = 20.0;
    
    // Stat Cards
    let stat_w = (avail.width() - pad * 2.0 - 24.0) / 3.0;
    let stat_h = 80.0;
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let stats = [
            ("Locked Files", format!("{}", state.file_list.len()), "📄", TEAL_STRONG),
            ("Encrypted", format_size(state.total_vault_size()), "💽", TEAL_STRONG),
            ("Standard", "AES-256".to_string(), "🛡️", TEAL_STRONG),
        ];
        for (label, val, icon, color) in stats.iter() {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(stat_w, stat_h), egui::Sense::hover());
            filled_rect(ui, rect, BG_SURFACE, Stroke::new(1.0, BORDER_DEFAULT), 16.0);
            ui.painter().text(egui::pos2(rect.center().x, rect.top() + 20.0), egui::Align2::CENTER_CENTER, *icon, FontId::new(20.0, FontFamily::Proportional), *color);
            ui.painter().text(egui::pos2(rect.center().x, rect.top() + 45.0), egui::Align2::CENTER_CENTER, val, FontId::new(18.0, FontFamily::Proportional), TEXT_PRIMARY);
            ui.painter().text(egui::pos2(rect.center().x, rect.top() + 65.0), egui::Align2::CENTER_CENTER, *label, FontId::new(10.0, FontFamily::Proportional), TEXT_MUTED);
            ui.add_space(12.0);
        }
    });

    ui.add_space(24.0);
    
    // Hardware Metrics
    ui.horizontal(|ui| { ui.add_space(pad); ui.label(egui::RichText::new("HARDWARE METRICS").size(12.0).color(TEXT_MUTED).strong()); });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 140.0), egui::Sense::hover());
        filled_rect(ui, rect, BG_SURFACE, Stroke::new(1.0, BORDER_DEFAULT), 20.0);
        
        ui.painter().text(egui::pos2(rect.left() + 20.0, rect.top() + 25.0), egui::Align2::LEFT_CENTER, "⚙️ Encryption Engine", FontId::new(14.0, FontFamily::Proportional), TEXT_PRIMARY);
        
        let badge_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 40.0, rect.top() + 25.0), Vec2::new(60.0, 20.0));
        filled_rect(ui, badge_rect, Color32::from_rgba_unmultiplied(182, 102, 210, 25), Stroke::new(1.0, Color32::from_rgba_unmultiplied(182, 102, 210, 50)), 10.0);
        ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, "High-tier", FontId::new(10.0, FontFamily::Proportional), TEAL_STRONG);
        
        let metrics = [("CPU", 0.55, TEAL_STRONG), ("RAM", 0.78, SUCCESS_COLOR), ("I/O", 0.32, WARN_COLOR)];
        for (i, (lbl, val, color)) in metrics.iter().enumerate() {
            let y = rect.top() + 60.0 + i as f32 * 25.0;
            ui.painter().text(egui::pos2(rect.left() + 20.0, y), egui::Align2::LEFT_CENTER, *lbl, FontId::new(12.0, FontFamily::Proportional), TEXT_MUTED);
            let bar_bg = egui::Rect::from_min_size(egui::pos2(rect.left() + 60.0, y - 3.0), Vec2::new(rect.width() - 120.0, 6.0));
            filled_rect(ui, bar_bg, BG_CARD, Stroke::NONE, 3.0);
            let bar_fg = egui::Rect::from_min_size(egui::pos2(rect.left() + 60.0, y - 3.0), Vec2::new((rect.width() - 120.0) * val, 6.0));
            filled_rect(ui, bar_fg, *color, Stroke::NONE, 3.0);
            ui.painter().text(egui::pos2(rect.right() - 20.0, y), egui::Align2::RIGHT_CENTER, format!("{}%", (val * 100.0) as i32), FontId::new(12.0, FontFamily::Proportional), TEXT_PRIMARY);
        }
    });

    ui.add_space(24.0);

    // Active Vaults
    ui.horizontal(|ui| { ui.add_space(pad); ui.label(egui::RichText::new("ACTIVE VAULTS").size(12.0).color(TEXT_MUTED).strong()); });
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
                let color = if *locked { TEAL_STRONG } else { SUCCESS_COLOR };
                let bg_color = if *locked { Color32::from_rgba_unmultiplied(182, 102, 210, 12) } else { Color32::from_rgba_unmultiplied(74, 222, 128, 12) };
                
                filled_rect(ui, rect, bg_color, Stroke::new(1.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 76)), 20.0);
                
                let icon_rect = egui::Rect::from_min_size(egui::pos2(rect.left() + 20.0, rect.top() + 20.0), Vec2::splat(40.0));
                filled_rect(ui, icon_rect, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 38), Stroke::NONE, 12.0);
                ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, if *locked { "🔒" } else { "🔓" }, FontId::new(20.0, FontFamily::Proportional), color);
                
                let badge_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 40.0, rect.top() + 40.0), Vec2::new(50.0, 20.0));
                filled_rect(ui, badge_rect, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 25), Stroke::NONE, 10.0);
                ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, if *locked { "Locked" } else { "Active" }, FontId::new(10.0, FontFamily::Proportional), color);

                ui.painter().text(egui::pos2(rect.left() + 20.0, rect.top() + 85.0), egui::Align2::LEFT_CENTER, *name, FontId::new(16.0, FontFamily::Proportional), TEXT_PRIMARY);
                ui.painter().text(egui::pos2(rect.left() + 20.0, rect.top() + 105.0), egui::Align2::LEFT_CENTER, *cap, FontId::new(12.0, FontFamily::Proportional), TEXT_MUTED);

                let bar_bg = egui::Rect::from_min_size(egui::pos2(rect.left() + 20.0, rect.bottom() - 20.0), Vec2::new(rect.width() - 40.0, 4.0));
                filled_rect(ui, bar_bg, BG_CARD, Stroke::NONE, 2.0);
                let bar_fg = egui::Rect::from_min_size(egui::pos2(rect.left() + 20.0, rect.bottom() - 20.0), Vec2::new((rect.width() - 40.0) * prog, 4.0));
                filled_rect(ui, bar_fg, color, Stroke::NONE, 2.0);
                
                ui.add_space(12.0);
            }
        });
    });

    ui.add_space(24.0);

    // Quick Actions
    ui.horizontal(|ui| { ui.add_space(pad); ui.label(egui::RichText::new("QUICK ACTIONS").size(12.0).color(TEXT_MUTED).strong()); });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let btn_w = (avail.width() - pad * 2.0 - 12.0) / 2.0;
        let btn_h = 60.0;
        let actions = [("🔒", "Lock All", TEAL_STRONG), ("🔓", "Unlock", SUCCESS_COLOR), ("📱", "Setup 2FA", Color32::from_rgb(96, 165, 250)), ("✅", "Integrity Check", ERROR_COLOR)];
        
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, btn_h), egui::Sense::click());
                filled_rect(ui, rect, if resp.hovered() { BG_CARD } else { BG_SURFACE }, Stroke::new(1.0, BORDER_DEFAULT), 16.0);
                let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 30.0, rect.center().y), Vec2::splat(36.0));
                filled_rect(ui, icon_rect, BG_CARD, Stroke::NONE, 10.0);
                ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, actions[0].0, FontId::new(18.0, FontFamily::Proportional), actions[0].2);
                ui.painter().text(egui::pos2(icon_rect.right() + 12.0, rect.center().y), egui::Align2::LEFT_CENTER, actions[0].1, FontId::new(14.0, FontFamily::Proportional), TEXT_PRIMARY);
                if resp.clicked() { ctrl.logout(state); }

                let (rect2, resp2) = ui.allocate_exact_size(Vec2::new(btn_w, btn_h), egui::Sense::click());
                filled_rect(ui, rect2, if resp2.hovered() { BG_CARD } else { BG_SURFACE }, Stroke::new(1.0, BORDER_DEFAULT), 16.0);
                let icon_rect2 = egui::Rect::from_center_size(egui::pos2(rect2.left() + 30.0, rect2.center().y), Vec2::splat(36.0));
                filled_rect(ui, icon_rect2, BG_CARD, Stroke::NONE, 10.0);
                ui.painter().text(icon_rect2.center(), egui::Align2::CENTER_CENTER, actions[1].0, FontId::new(18.0, FontFamily::Proportional), actions[1].2);
                ui.painter().text(egui::pos2(icon_rect2.right() + 12.0, rect2.center().y), egui::Align2::LEFT_CENTER, actions[1].1, FontId::new(14.0, FontFamily::Proportional), TEXT_PRIMARY);
                // click action
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, btn_h), egui::Sense::click());
                filled_rect(ui, rect, if resp.hovered() { BG_CARD } else { BG_SURFACE }, Stroke::new(1.0, BORDER_DEFAULT), 16.0);
                let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 30.0, rect.center().y), Vec2::splat(36.0));
                filled_rect(ui, icon_rect, BG_CARD, Stroke::NONE, 10.0);
                ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, actions[2].0, FontId::new(18.0, FontFamily::Proportional), actions[2].2);
                ui.painter().text(egui::pos2(icon_rect.right() + 12.0, rect.center().y), egui::Align2::LEFT_CENTER, actions[2].1, FontId::new(14.0, FontFamily::Proportional), TEXT_PRIMARY);
                if resp.clicked() { if state.totp_enabled { ctrl.disable_totp(state); } else { ctrl.begin_totp_setup(state); } }

                let (rect2, resp2) = ui.allocate_exact_size(Vec2::new(btn_w, btn_h), egui::Sense::click());
                filled_rect(ui, rect2, if resp2.hovered() { BG_CARD } else { BG_SURFACE }, Stroke::new(1.0, BORDER_DEFAULT), 16.0);
                let icon_rect2 = egui::Rect::from_center_size(egui::pos2(rect2.left() + 30.0, rect2.center().y), Vec2::splat(36.0));
                filled_rect(ui, icon_rect2, BG_CARD, Stroke::NONE, 10.0);
                ui.painter().text(icon_rect2.center(), egui::Align2::CENTER_CENTER, actions[3].0, FontId::new(18.0, FontFamily::Proportional), actions[3].2);
                ui.painter().text(egui::pos2(icon_rect2.right() + 12.0, rect2.center().y), egui::Align2::LEFT_CENTER, actions[3].1, FontId::new(14.0, FontFamily::Proportional), TEXT_PRIMARY);
            });
        });
    });

    ui.add_space(24.0);

    // Recent Activity (Files)
    ui.horizontal(|ui| { ui.add_space(pad); ui.label(egui::RichText::new("RECENT ACTIVITY").size(12.0).color(TEXT_MUTED).strong()); });
    ui.add_space(12.0);
    
    if state.file_list.is_empty() {
        ui.horizontal(|ui| { ui.add_space(pad); ui.label(egui::RichText::new("Belum ada file di dalam brankas.").color(TEXT_MUTED)); });
    } else {
        ui.vertical(|ui| {
            for record in state.file_list.iter() {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 68.0), egui::Sense::click());
                    let is_hover = resp.hovered();
                    filled_rect(ui, rect, if is_hover { BG_CARD } else { BG_SURFACE }, Stroke::new(1.0, if is_hover { TEAL_STRONG } else { BORDER_DEFAULT }), 16.0);
                    
                    let ext = file_ext(&record.original_name);
                    let (icon, badge) = file_badge(ext);
                    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 34.0, rect.center().y), Vec2::splat(44.0));
                    filled_rect(ui, icon_rect, badge.0, Stroke::NONE, 12.0);
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(22.0, FontFamily::Proportional), badge.1);
                    
                    let name_truncated = if record.original_name.len() > 25 { format!("{}…", &record.original_name[..23]) } else { record.original_name.clone() };
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y - 10.0), egui::Align2::LEFT_CENTER, name_truncated, FontId::new(15.0, FontFamily::Proportional), TEXT_PRIMARY);
                    
                    let meta = format!("{} • Encrypted {}", format_size(record.file_size as u64), &record.encrypted_at[..10]);
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y + 10.0), egui::Align2::LEFT_CENTER, meta, FontId::new(12.0, FontFamily::Proportional), TEXT_MUTED);
                    
                    let action_icon = if is_hover { "🔓" } else { "🔒" };
                    let icon_resp = ui.allocate_rect(egui::Rect::from_center_size(egui::pos2(rect.right() - 24.0, rect.center().y), Vec2::splat(30.0)), egui::Sense::click());
                    ui.painter().text(egui::pos2(rect.right() - 24.0, rect.center().y), egui::Align2::CENTER_CENTER, action_icon, FontId::new(20.0, FontFamily::Proportional), TEXT_MUTED);
                    
                    if icon_resp.clicked() {
                        *to_decrypt = Some(record.vault_filename.clone());
                    } else if resp.clicked() {
                        *to_decrypt = Some(record.vault_filename.clone());
                    }
                });
                ui.add_space(8.0);
            }
        });
    }
}

fn render_tab_vault(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller, to_decrypt: &mut Option<String>, _to_soft_delete: &mut Option<String>) {
    let pad = 20.0;
    ui.add_space(20.0);
    
    // Header & Search
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(egui::RichText::new("Brankas Anda").size(22.0).color(TEXT_PRIMARY).strong());
    });
    ui.add_space(10.0);
    
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let search_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width() - pad - 100.0, 36.0));
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
            ui.label(egui::RichText::new("Belum ada file yang cocok.").color(TEXT_MUTED));
        });
        return;
    }
    
    // Render files
    let avail = ui.available_rect_before_wrap();
    
    if state.vault_view_mode == ViewMode::List {
        ui.vertical(|ui| {
            for record in files {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 68.0), egui::Sense::click());
                    let is_hover = resp.hovered();
                    filled_rect(ui, rect, if is_hover { BG_CARD } else { BG_SURFACE }, Stroke::new(1.0, if is_hover { TEAL_STRONG } else { BORDER_DEFAULT }), 16.0);
                    
                    let ext = file_ext(&record.original_name);
                    let (icon, badge) = file_badge(ext);
                    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 34.0, rect.center().y), Vec2::splat(44.0));
                    filled_rect(ui, icon_rect, badge.0, Stroke::NONE, 12.0);
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(22.0, FontFamily::Proportional), badge.1);
                    
                    let name_truncated = if record.original_name.len() > 30 { format!("{}…", &record.original_name[..28]) } else { record.original_name.clone() };
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y - 10.0), egui::Align2::LEFT_CENTER, name_truncated, FontId::new(15.0, FontFamily::Proportional), TEXT_PRIMARY);
                    
                    let meta = format!("{} • Encrypted {}", format_size(record.file_size as u64), &record.encrypted_at[..10]);
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y + 10.0), egui::Align2::LEFT_CENTER, meta, FontId::new(12.0, FontFamily::Proportional), TEXT_MUTED);
                    
                    if is_hover {
                        let action_icon = "🔓";
                        let icon_resp = ui.allocate_rect(egui::Rect::from_center_size(egui::pos2(rect.right() - 24.0, rect.center().y), Vec2::splat(30.0)), egui::Sense::click());
                        ui.painter().text(egui::pos2(rect.right() - 24.0, rect.center().y), egui::Align2::CENTER_CENTER, action_icon, FontId::new(20.0, FontFamily::Proportional), TEXT_MUTED);
                        if icon_resp.clicked() || resp.clicked() {
                            *to_decrypt = Some(record.vault_filename.clone());
                        }
                    } else if resp.clicked() {
                         *to_decrypt = Some(record.vault_filename.clone());
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
                filled_rect(ui, rect, if is_hover { BG_CARD } else { BG_SURFACE }, Stroke::new(1.0, if is_hover { TEAL_STRONG } else { BORDER_DEFAULT }), 16.0);
                
                let ext = file_ext(&record.original_name);
                let (icon, badge) = file_badge(ext);
                let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 40.0), Vec2::splat(50.0));
                filled_rect(ui, icon_rect, badge.0, Stroke::NONE, 14.0);
                ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(28.0, FontFamily::Proportional), badge.1);
                
                let name_truncated = if record.original_name.len() > 12 { format!("{}…", &record.original_name[..10]) } else { record.original_name.clone() };
                ui.painter().text(egui::pos2(rect.center().x, icon_rect.bottom() + 16.0), egui::Align2::CENTER_CENTER, name_truncated, FontId::new(13.0, FontFamily::Proportional), TEXT_PRIMARY);
                ui.painter().text(egui::pos2(rect.center().x, icon_rect.bottom() + 32.0), egui::Align2::CENTER_CENTER, format_size(record.file_size as u64), FontId::new(11.0, FontFamily::Proportional), TEXT_MUTED);
                
                if resp.clicked() {
                    *to_decrypt = Some(record.vault_filename.clone());
                }
                ui.add_space(8.0); // space between grid items
            }
        });
    }
}

fn draw_pie_chart(ui: &mut egui::Ui, rect: egui::Rect, data: &[(String, f32, Color32)]) {
    let center = rect.center();
    let radius = rect.width().min(rect.height()) / 2.0;
    let mut current_angle: f32 = -std::f32::consts::FRAC_PI_2; // Start from top
    let total: f32 = data.iter().map(|(_, v, _)| v).sum();
    
    if total == 0.0 {
        ui.painter().circle(center, radius, BG_SURFACE, Stroke::new(1.0, BORDER_DEFAULT));
        ui.painter().text(center, egui::Align2::CENTER_CENTER, "Kosong", FontId::new(14.0, FontFamily::Proportional), TEXT_MUTED);
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
    ui.painter().circle_filled(center, radius * 0.6, BG_BASE);
}

fn render_tab_storage(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new("Storage Analysis").size(22.0).color(TEXT_PRIMARY).strong());
        ui.add_space(8.0);
        let total = state.total_vault_size();
        ui.label(egui::RichText::new(format!("Total: {}", format_size(total))).size(16.0).color(TEXT_MUTED));
    });
    
    ui.add_space(40.0);
    
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
        ("Dokumen".to_string(), size_doc, TEAL_STRONG),
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
                        ui.label(egui::RichText::new(label).color(TEXT_PRIMARY).size(14.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(20.0);
                            ui.label(egui::RichText::new(format_size(*val as u64)).color(TEXT_MUTED).size(14.0));
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
        ui.label(egui::RichText::new("Pengaturan").size(20.0).color(TEXT_PRIMARY).strong());
        ui.add_space(30.0);
        
        let btn_w = avail.width() - pad*2.0;
        
        ui.horizontal(|ui| {
            ui.add_space(pad);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 60.0), egui::Sense::click());
            filled_rect(ui, rect, if resp.hovered() { BG_CARD } else { BG_SURFACE }, Stroke::new(1.0, BORDER_DEFAULT), 16.0);
            ui.painter().text(egui::pos2(rect.left() + 30.0, rect.center().y), egui::Align2::CENTER_CENTER, "📱", FontId::new(20.0, FontFamily::Proportional), Color32::from_rgb(96, 165, 250));
            ui.painter().text(egui::pos2(rect.left() + 60.0, rect.center().y), egui::Align2::LEFT_CENTER, if state.totp_enabled { "Disable 2FA" } else { "Setup 2FA" }, FontId::new(16.0, FontFamily::Proportional), TEXT_PRIMARY);
            if resp.clicked() { if state.totp_enabled { ctrl.disable_totp(state); } else { ctrl.begin_totp_setup(state); } }
        });
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            ui.add_space(pad);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 60.0), egui::Sense::click());
            filled_rect(ui, rect, if resp.hovered() { BG_CARD } else { BG_SURFACE }, Stroke::new(1.0, BORDER_DEFAULT), 16.0);
            ui.painter().text(egui::pos2(rect.left() + 30.0, rect.center().y), egui::Align2::CENTER_CENTER, "🗑", FontId::new(20.0, FontFamily::Proportional), ERROR_COLOR);
            ui.painter().text(egui::pos2(rect.left() + 60.0, rect.center().y), egui::Align2::LEFT_CENTER, "Recycle Bin", FontId::new(16.0, FontFamily::Proportional), TEXT_PRIMARY);
            if resp.clicked() { ctrl.load_deleted_files(state); state.screen = AppScreen::RecycleBin; }
        });
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            ui.add_space(pad);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 60.0), egui::Sense::click());
            filled_rect(ui, rect, if resp.hovered() { BG_CARD } else { BG_SURFACE }, Stroke::new(1.0, BORDER_DEFAULT), 16.0);
            ui.painter().text(egui::pos2(rect.left() + 30.0, rect.center().y), egui::Align2::CENTER_CENTER, "🚪", FontId::new(20.0, FontFamily::Proportional), TEXT_MUTED);
            ui.painter().text(egui::pos2(rect.left() + 60.0, rect.center().y), egui::Align2::LEFT_CENTER, "Logout / Kunci Vault", FontId::new(16.0, FontFamily::Proportional), TEXT_PRIMARY);
            if resp.clicked() { ctrl.logout(state); }
        });
    });
}

fn render_tab_profile(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {
    ui.add_space(30.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new("Profil & Pengaturan").size(22.0).color(TEXT_PRIMARY).strong());
    });
    
    ui.add_space(30.0);
    let pad = 20.0;
    
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            // Backup Database Section
            ui.label(egui::RichText::new("Data").color(TEAL_STRONG).strong());
            ui.add_space(8.0);
            if teal_btn(ui, "💾  Backup Database", 200.0).clicked() {
                ctrl.backup_database(state);
            }
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Simpan cadangan .db di tempat aman.").size(12.0).color(TEXT_MUTED));
            
            ui.add_space(30.0);
            
            // Ubah PIN Section
            ui.label(egui::RichText::new("Ubah PIN Utama").color(TEAL_STRONG).strong());
            ui.add_space(10.0);
            
            ui.label(egui::RichText::new("PIN Lama").size(12.0).color(TEXT_MUTED));
            ui.add(egui::TextEdit::singleline(&mut state.profile_old_pin).password(true).desired_width(200.0));
            ui.add_space(8.0);
            
            ui.label(egui::RichText::new("PIN Baru (6 digit)").size(12.0).color(TEXT_MUTED));
            ui.add(egui::TextEdit::singleline(&mut state.profile_new_pin).password(true).desired_width(200.0));
            ui.add_space(8.0);
            
            ui.label(egui::RichText::new("Konfirmasi PIN Baru").size(12.0).color(TEXT_MUTED));
            ui.add(egui::TextEdit::singleline(&mut state.profile_confirm_pin).password(true).desired_width(200.0));
            ui.add_space(12.0);
            
            if teal_btn(ui, "🔑  Ubah PIN", 200.0).clicked() {
                ctrl.change_pin(state);
            }
            
            if let Some(err) = &state.profile_pin_error {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(err).color(ERROR_COLOR).size(13.0));
            }
            if let Some(msg) = &state.profile_pin_success {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(msg).color(TEAL_LIGHT).size(13.0));
            }
        });
    });
}

fn render_tab_notifications(ui: &mut egui::Ui, state: &mut AppState, _ctrl: &Controller) {
    ui.add_space(30.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new("Audit Log Keamanan").size(22.0).color(TEXT_PRIMARY).strong());
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Aktivitas terbaru di dalam brankas.").color(TEXT_MUTED));
    });
    
    ui.add_space(30.0);
    let pad = 20.0;
    let avail = ui.available_rect_before_wrap();
    
    if state.audit_logs.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("Belum ada catatan aktivitas.").color(TEXT_MUTED));
        });
    } else {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for log in &state.audit_logs {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail.width() - pad*2.0, 60.0), egui::Sense::hover());
                    
                    let (icon, color) = match log.action_type.as_str() {
                        "FAIL_LOGIN" | "FAIL_2FA" => ("⚠", ERROR_COLOR),
                        "LOGIN" | "LOGIN_2FA"     => ("👤", TEAL_STRONG),
                        "ENCRYPT"                 => ("🔒", Color32::from_rgb(250, 190, 88)),
                        "DECRYPT"                 => ("🔓", Color32::from_rgb(100, 200, 100)),
                        "BACKUP"                  => ("💾", Color32::from_rgb(100, 150, 250)),
                        "CHANGE_PIN" | "SETUP"    => ("🔑", Color32::from_rgb(200, 100, 250)),
                        _                         => ("ℹ", TEXT_MUTED),
                    };
                    
                    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 24.0, rect.center().y), Vec2::splat(36.0));
                    filled_rect(ui, icon_rect, color.linear_multiply(0.15), Stroke::NONE, 18.0);
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon, FontId::new(18.0, FontFamily::Proportional), color);
                    
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y - 10.0), egui::Align2::LEFT_CENTER, &log.description, FontId::new(14.0, FontFamily::Proportional), TEXT_PRIMARY);
                    ui.painter().text(egui::pos2(icon_rect.right() + 16.0, rect.center().y + 10.0), egui::Align2::LEFT_CENTER, &log.timestamp, FontId::new(11.0, FontFamily::Proportional), TEXT_MUTED);
                    
                    // Separator line
                    ui.painter().line_segment([egui::pos2(rect.left(), rect.bottom()), egui::pos2(rect.right(), rect.bottom())], Stroke::new(0.5, BORDER_SUBTLE));
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
                filled_rect(ui, back_rect, Color32::TRANSPARENT, Stroke::new(0.5, BORDER_DEFAULT), 7.0);
                ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "←",
                                  FontId::new(15.0, FontFamily::Proportional), TEXT_MUTED);
                if back_resp.clicked() { state.screen = AppScreen::Dashboard; return; }
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Pulihkan file").size(15.0).color(TEXT_BODY).strong());
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
                            .size(14.0).color(TEXT_PRIMARY).strong());
                        ui.label(egui::RichText::new(format_size(record.file_size as u64))
                            .size(11.0).color(TEXT_DIMMED));
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
                        ui.label(egui::RichText::new(*k).size(11.0).color(TEXT_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(v).size(11.0).color(TEXT_DIMMED)
                                .text_style(egui::TextStyle::Monospace));
                        });
                    });
                }
            });

            ui.add_space(20.0);

            // Output name field
            ui.label(egui::RichText::new("Nama file output").size(12.0).color(TEXT_MUTED));
            ui.add_space(6.0);
            egui::Frame::none()
                .fill(BG_SURFACE).stroke(Stroke::new(0.5, BORDER_DEFAULT))
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("📤").size(16.0).color(TEXT_MUTED));
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
                        ui.label(egui::RichText::new("⚠").size(16.0).color(WARN_COLOR));
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
                let color = if s.success { SUCCESS_COLOR } else { ERROR_COLOR };
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

                    if let Some(out_dir) = FileDialog::new()
                        .set_title("Pilih folder tujuan")
                        .pick_folder()
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
                            Stroke::new(0.5, BORDER_DEFAULT), 7.0);
                ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "←",
                                  FontId::new(15.0, FontFamily::Proportional), TEXT_MUTED);
                if back_resp.clicked() { state.screen = AppScreen::Dashboard; return; }
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Setup Autentikasi 2FA").size(15.0).color(TEXT_BODY).strong());
            });

            ui.add_space(16.0);

            ui.vertical_centered(|ui| {
                // Info banner
                egui::Frame::none()
                    .fill(Color32::from_rgb(12, 31, 24))
                    .stroke(Stroke::new(0.5, BORDER_ACCENT))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_width(avail.width() - 80.0);
                        ui.horizontal_top(|ui| {
                            ui.label(egui::RichText::new("ℹ").size(16.0).color(TEAL_STRONG));
                            ui.add_space(6.0);
                            ui.add(egui::Label::new(egui::RichText::new(
                                "Scan QR code ini dengan Google Authenticator,\nAuthy, atau aplikasi TOTP lainnya."
                            ).size(12.0).color(TEAL_LIGHT)).wrap(true));
                        });
                    });

                ui.add_space(16.0);

                // QR Code
                if let Some(matrix) = &state.totp_qr {
                    crate::totp::draw_qr(ui, matrix, 200.0);
                    if let Some(st) = state.totp_setup_time {
                        let left = 30u64.saturating_sub(st.elapsed().as_secs());
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(format!("⏳ QR berganti dalam {} detik", left)).size(11.0).color(WARN_COLOR));
                    }
                } else {
                    ui.label(egui::RichText::new("Gagal generate QR code").color(ERROR_COLOR));
                }

                ui.add_space(12.0);

                // Manual secret key
                ui.label(egui::RichText::new("Atau masukkan kunci manual:").size(11.0).color(TEXT_MUTED));
                ui.add_space(4.0);
                egui::Frame::none()
                    .fill(BG_SURFACE)
                    .stroke(Stroke::new(0.5, BORDER_DEFAULT))
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
                            .size(12.0).color(TEAL_FAINT)
                            .text_style(egui::TextStyle::Monospace));
                    });
                    
                if let Some(st) = state.totp_setup_time {
                    let left = 30u64.saturating_sub(st.elapsed().as_secs());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("⏳ Kunci berganti dalam {} detik", left)).size(11.0).color(WARN_COLOR));
                }

                ui.add_space(20.0);

                // Verify input
                ui.label(egui::RichText::new("Masukkan kode 6-digit dari app:").size(12.0).color(TEXT_MUTED));
                ui.add_space(6.0);
                egui::Frame::none()
                    .fill(BG_SURFACE)
                    .stroke(Stroke::new(0.5, if state.totp_code.len() == 6 { TEAL_STRONG } else { BORDER_DEFAULT }))
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
                    ui.label(egui::RichText::new(err).color(ERROR_COLOR).size(12.0));
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
            filled_rect(ui, icon_rect, TEAL_DARK, Stroke::NONE, 14.0);
            ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, "🔐",
                              FontId::new(26.0, FontFamily::Proportional), TEAL_FAINT);

            ui.add_space(14.0);
            ui.label(egui::RichText::new("Verifikasi 2FA").size(18.0).color(TEXT_BODY).strong());
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Masukkan kode dari aplikasi authenticator")
                .size(13.0).color(TEXT_MUTED));

            ui.add_space(8.0);

            // Timer countdown
            let secs = crate::totp::seconds_left();
            let timer_color = if secs <= 5 { ERROR_COLOR } else if secs <= 10 { WARN_COLOR } else { TEAL_LIGHT };
            ui.label(egui::RichText::new(format!("Kode berubah dalam {} detik", secs))
                .size(11.0).color(timer_color));
            ui.ctx().request_repaint();

            ui.add_space(20.0);

            // Code input
            egui::Frame::none()
                .fill(BG_SURFACE)
                .stroke(Stroke::new(0.5, if state.totp_code.len() == 6 { TEAL_STRONG } else { BORDER_DEFAULT }))
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
                ui.label(egui::RichText::new(err).color(ERROR_COLOR).size(13.0));
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
    filled_rect(ui, topbar_rect, Color32::from_rgb(14,16,22), Stroke::new(0.5, BORDER_SUBTLE), 0.0);

    let back_rect = egui::Rect::from_min_size(topbar_rect.min + Vec2::new(18.0, 12.0), Vec2::splat(28.0));
    let back_resp = ui.allocate_rect(back_rect, egui::Sense::click());
    filled_rect(ui, back_rect, Color32::TRANSPARENT, Stroke::new(0.5, BORDER_DEFAULT), 7.0);
    ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "←",
                      FontId::new(15.0, FontFamily::Proportional), TEXT_MUTED);
    if back_resp.clicked() { state.screen = AppScreen::Dashboard; return; }

    ui.painter().text(
        egui::pos2(back_rect.right() + 10.0, topbar_rect.center().y),
        egui::Align2::LEFT_CENTER, "Recycle Bin",
        FontId::new(16.0, FontFamily::Proportional), TEXT_PRIMARY,
    );

    let mut cursor_y = topbar_rect.bottom() + 14.0;
    
    // Warning banner
    let banner_rect = egui::Rect::from_min_size(
        egui::pos2(avail.left() + pad, cursor_y),
        Vec2::new(avail.width() - pad * 2.0, 44.0),
    );
    filled_rect(ui, banner_rect, Color32::from_rgb(30, 20, 20), Stroke::new(0.5, ERROR_COLOR), 8.0);
    ui.painter().text(banner_rect.center(), egui::Align2::CENTER_CENTER,
                      "⚠️ File di bawah dapat dipulihkan atau dihapus permanen.",
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
                                  FontId::new(16.0, FontFamily::Proportional), TEXT_MUTED);
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
                    let card_fill    = if card_hovered { BG_CARD } else { BG_SURFACE };
                    let card_stroke  = if card_hovered {
                        Stroke::new(0.5, WARN_COLOR)
                    } else {
                        Stroke::new(0.5, BORDER_DEFAULT)
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
                                      FontId::new(14.0, FontFamily::Proportional), TEXT_PRIMARY);
                    let meta = format!("{}…  ·  Dihapus: {}",
                                       &record.sha256_hash[..6],
                                       record.deleted_at.as_deref().unwrap_or(""));
                    ui.painter().text(egui::pos2(info_x, card_rect.top() + 36.0),
                                      egui::Align2::LEFT_TOP, &meta,
                                      FontId::new(11.0, FontFamily::Proportional), ERROR_COLOR);

                    // Tombol Hapus Permanen
                    let perm_del_rect = egui::Rect::from_min_size(
                        egui::pos2(card_rect.right() - 94.0, card_rect.center().y - 16.0),
                        Vec2::new(38.0, 32.0),
                    );
                    let perm_del_resp = ui.allocate_rect(perm_del_rect, egui::Sense::click());
                    let perm_del_border = if perm_del_resp.hovered() { ERROR_COLOR } else { BORDER_DEFAULT };
                    let perm_del_icon_c = if perm_del_resp.hovered() { ERROR_COLOR } else { TEXT_MUTED };
                    filled_rect(ui, perm_del_rect, BG_SURFACE, Stroke::new(0.5, perm_del_border), 7.0);
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
                    let restore_border = if restore_resp.hovered() { TEAL_STRONG } else { BORDER_DEFAULT };
                    let restore_icon_c = if restore_resp.hovered() { TEAL_STRONG } else { TEXT_MUTED };
                    filled_rect(ui, restore_rect, BG_SURFACE, Stroke::new(0.5, restore_border), 7.0);
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
