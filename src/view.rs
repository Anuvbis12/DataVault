// view.rs — View layer
// Seluruh fungsi render egui. View hanya membaca AppState
// dan memanggil Controller untuk aksi. Tidak ada logika bisnis di sini.

use eframe::egui;
use egui::epaint::{Color32, FontId, FontFamily, Mesh, Rounding, Stroke, Vec2, Vertex};
use rfd::FileDialog;

use crate::app_state::{AppScreen, AppState};
use crate::controller::{format_size, Controller};
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
        });
    });
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
    let pad   = 16.0;

    // ─ Topbar ─
    let topbar_rect = egui::Rect::from_min_size(avail.min, Vec2::new(avail.width(), 52.0));
    filled_rect(ui, topbar_rect, Color32::from_rgb(14,16,22), Stroke::new(0.5, BORDER_SUBTLE), 0.0);

    let logo_rect = egui::Rect::from_min_size(topbar_rect.min + Vec2::new(18.0, 12.0), Vec2::splat(28.0));
    filled_rect(ui, logo_rect, TEAL_DARK, Stroke::NONE, 7.0);
    ui.painter().text(logo_rect.center(), egui::Align2::CENTER_CENTER, "🛡",
                      FontId::new(14.0, FontFamily::Proportional), TEAL_FAINT);
    ui.painter().text(
        egui::pos2(logo_rect.right() + 10.0, topbar_rect.center().y),
        egui::Align2::LEFT_CENTER, "Aegis Vault",
        FontId::new(14.0, FontFamily::Proportional), TEXT_PRIMARY,
    );

    // Sesi aktif badge
    let badge_rect = egui::Rect::from_center_size(
        egui::pos2(avail.right() - 110.0, topbar_rect.center().y), Vec2::new(90.0, 22.0)
    );
    filled_rect(ui, badge_rect, Color32::from_rgb(12,31,24), Stroke::new(0.5, BORDER_ACCENT), 20.0);
    ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, "🔒 Sesi aktif",
                      FontId::new(11.0, FontFamily::Proportional), TEAL_LIGHT);

    // Logout button
    let logout_rect = egui::Rect::from_center_size(
        egui::pos2(avail.right() - 26.0, topbar_rect.center().y), Vec2::new(32.0, 26.0)
    );
    filled_rect(ui, logout_rect, Color32::TRANSPARENT, Stroke::new(0.5, BORDER_DEFAULT), 6.0);
    ui.painter().text(logout_rect.center(), egui::Align2::CENTER_CENTER, "🚪",
                      FontId::new(14.0, FontFamily::Proportional), TEXT_MUTED);
    if ui.allocate_rect(logout_rect, egui::Sense::click()).clicked() {
        ctrl.logout(state);
        return;
    }

    // TOTP toggle button (kiri dari logout)
    let totp_label = if state.totp_enabled { "🔐 2FA" } else { "🔓 2FA" };
    let totp_rect = egui::Rect::from_center_size(
        egui::pos2(avail.right() - 74.0, topbar_rect.center().y), Vec2::new(36.0, 26.0),
    );
    let totp_resp = ui.allocate_rect(totp_rect, egui::Sense::click());
    let totp_fill = if state.totp_enabled { Color32::from_rgb(12,31,24) } else { Color32::TRANSPARENT };
    let totp_border = if totp_resp.hovered() { TEAL_STRONG } else if state.totp_enabled { BORDER_ACCENT } else { BORDER_DEFAULT };
    filled_rect(ui, totp_rect, totp_fill, Stroke::new(0.5, totp_border), 6.0);
    ui.painter().text(totp_rect.center(), egui::Align2::CENTER_CENTER, totp_label,
                      FontId::new(9.0, FontFamily::Proportional),
                      if state.totp_enabled { TEAL_LIGHT } else { TEXT_MUTED });
    if totp_resp.clicked() {
        if state.totp_enabled {
            ctrl.disable_totp(state);
        } else {
            ctrl.begin_totp_setup(state);
        }
        return;
    }

    let mut cursor_y = topbar_rect.bottom() + 14.0;

    // ─ Stat pills ─
    let pill_h   = 62.0;
    let pill_gap = 8.0;
    let pill_w   = (avail.width() - pad * 2.0 - pill_gap * 2.0) / 3.0;
    let stats = [
        ("File tersimpan",    format!("{}", state.file_list.len())),
        ("Total terenkripsi", format_size(state.total_vault_size())),
        ("Algoritma",         "AES-256".to_string()),
    ];
    for (i, (label, value)) in stats.iter().enumerate() {
        let pill_rect = egui::Rect::from_min_size(
            egui::pos2(avail.left() + pad + i as f32 * (pill_w + pill_gap), cursor_y),
            Vec2::new(pill_w, pill_h),
        );
        filled_rect(ui, pill_rect, BG_SURFACE, Stroke::NONE, 8.0);
        ui.painter().text(egui::pos2(pill_rect.left() + 14.0, pill_rect.top() + 14.0),
                          egui::Align2::LEFT_TOP, *label,
                          FontId::new(11.0, FontFamily::Proportional), TEXT_MUTED);
        ui.painter().text(egui::pos2(pill_rect.left() + 14.0, pill_rect.top() + 30.0),
                          egui::Align2::LEFT_TOP, value.as_str(),
                          FontId::new(if i == 0 { 22.0 } else { 15.0 }, FontFamily::Proportional),
                          TEXT_PRIMARY);
    }
    cursor_y += pill_h + 12.0;

    // ─ Divider ─
    ui.painter().line_segment(
        [egui::pos2(avail.left() + pad, cursor_y), egui::pos2(avail.right() - pad, cursor_y)],
        Stroke::new(0.5, BORDER_SUBTLE),
    );
    cursor_y += 10.0;

    // ─ Status message ─
    if let Some(s) = &state.status.clone() {
        let color = if s.success { SUCCESS_COLOR } else { ERROR_COLOR };
        ui.painter().text(
            egui::pos2(avail.center().x, cursor_y + 8.0),
            egui::Align2::CENTER_TOP, &s.text,
            FontId::new(12.0, FontFamily::Proportional), color,
        );
        cursor_y += 28.0;
    }

    // ─ File list (scroll) ─
    let footer_h      = 36.0;
    let fab_h         = 60.0;
    let scroll_rect   = egui::Rect::from_min_max(
        egui::pos2(avail.left(), cursor_y),
        egui::pos2(avail.right(), avail.bottom() - footer_h - fab_h),
    );

    let mut to_decrypt: Option<String> = None;

    egui::ScrollArea::vertical()
        .id_source("file_scroll")
        .show_viewport(ui, |ui, _vp| {
            ui.set_clip_rect(scroll_rect);
            if state.file_list.is_empty() {
                let c = scroll_rect.center();
                ui.painter().text(c - Vec2::new(0.0, 14.0), egui::Align2::CENTER_CENTER,
                                  "Brankas Kosong",
                                  FontId::new(18.0, FontFamily::Proportional), TEXT_MUTED);
                ui.painter().text(c + Vec2::new(0.0, 14.0), egui::Align2::CENTER_CENTER,
                                  "Tekan ➕ untuk menambah file.",
                                  FontId::new(13.0, FontFamily::Proportional), TEXT_MUTED);
            } else {
                let card_h   = 68.0;
                let card_gap = 8.0;
                for (idx, record) in state.file_list.iter().enumerate() {
                    let card_y = scroll_rect.top() + idx as f32 * (card_h + card_gap) + 4.0;
                    if card_y + card_h > scroll_rect.bottom() { break; }

                    let card_rect = egui::Rect::from_min_size(
                        egui::pos2(avail.left() + pad, card_y),
                        Vec2::new(avail.width() - pad * 2.0, card_h),
                    );
                    let card_hovered = ui.rect_contains_pointer(card_rect);
                    let card_fill    = if card_hovered { BG_CARD } else { BG_SURFACE };
                    let card_stroke  = if card_hovered {
                        Stroke::new(0.5, TEAL_STRONG)
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
                    let meta = format!("{}…  ·  {}  ·  {}",
                                       &record.sha256_hash[..6],
                                       format_size(record.file_size as u64),
                                       &record.encrypted_at);
                    ui.painter().text(egui::pos2(info_x, card_rect.top() + 36.0),
                                      egui::Align2::LEFT_TOP, &meta,
                                      FontId::new(11.0, FontFamily::Proportional), TEXT_DIMMED);

                    // Tombol dekripsi
                    let btn_rect = egui::Rect::from_min_size(
                        egui::pos2(card_rect.right() - 50.0, card_rect.center().y - 16.0),
                        Vec2::new(38.0, 32.0),
                    );
                    let btn_resp   = ui.allocate_rect(btn_rect, egui::Sense::click());
                    let btn_border = if btn_resp.hovered() { TEAL_STRONG } else { BORDER_DEFAULT };
                    let btn_icon_c = if btn_resp.hovered() { TEAL_STRONG } else { TEXT_MUTED };
                    filled_rect(ui, btn_rect, BG_SURFACE, Stroke::new(0.5, btn_border), 7.0);
                    ui.painter().text(btn_rect.center(), egui::Align2::CENTER_CENTER, "🔓",
                                      FontId::new(14.0, FontFamily::Proportional), btn_icon_c);
                    if btn_resp.clicked() {
                        to_decrypt = Some(record.vault_filename.clone());
                    }
                }
            }
        });

    // ─ Footer ─
    let footer_rect = egui::Rect::from_min_size(
        egui::pos2(avail.left(), avail.bottom() - footer_h),
        Vec2::new(avail.width(), footer_h),
    );
    ui.painter().line_segment(
        [footer_rect.left_top(), footer_rect.right_top()],
        Stroke::new(0.5, BORDER_SUBTLE),
    );
    ui.painter().text(footer_rect.center(), egui::Align2::CENTER_CENTER,
                      "3-pass secure delete · PBKDF2 · SHA-256 integrity check",
                      FontId::new(11.0, FontFamily::Proportional), TEXT_DIMMED);

    // ─ FAB ─
    let fab_size = Vec2::splat(48.0);
    let fab_rect = egui::Rect::from_min_size(
        egui::pos2(avail.right() - fab_size.x - 20.0, avail.bottom() - footer_h - fab_size.y - 12.0),
        fab_size,
    );
    let fab_resp = ui.allocate_rect(fab_rect, egui::Sense::click());
    let fab_fill = if fab_resp.is_pointer_button_down_on() { TEAL_DARK }
                   else if fab_resp.hovered()               { TEAL_STRONG }
                   else                                     { TEAL_DARK };
    filled_rect(ui, fab_rect, fab_fill, Stroke::NONE, 14.0);
    ui.painter().text(fab_rect.center(), egui::Align2::CENTER_CENTER, "➕",
                      FontId::new(22.0, FontFamily::Proportional), Color32::WHITE);
    if fab_resp.on_hover_text("Tambah & Enkripsi File Baru").clicked() {
        if let Some(path) = FileDialog::new().pick_file() {
            ctrl.encrypt_file(state, path);
        }
    }

    // Navigasi ke panel dekripsi
    if let Some(fname) = to_decrypt {
        ctrl.open_decrypt_panel(state, &fname);
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
