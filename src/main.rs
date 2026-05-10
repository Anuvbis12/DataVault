// main.rs — Aegis Vault v3 (Redesigned UI)
// UI redesign: dark navy/teal theme, numpad PIN, stat pills,
// file cards with icon badges, decrypt panel.
// Logic (crypto, db, login, setup, encrypt, decrypt) tidak berubah.

mod crypto;
mod db;

use crypto::{
    derive_key, generate_salt, hash_pin, secure_decrypt_file,
    secure_encrypt_file, KEY_LEN, SALT_LEN,
};
use db::{FileRecord, VaultDb};
use eframe::{
    egui,
    epaint::{Color32, FontId, FontFamily, Mesh, Rounding, Stroke, Vec2, Vertex},
};
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use zeroize::Zeroize;

// ── Palet Warna — Aegis Vault Redesign ───────────────────────
// Background layers
const BG_BASE:        Color32 = Color32::from_rgb(14,  16,  22);   // #0e1016
const BG_SURFACE:     Color32 = Color32::from_rgb(25,  28,  40);   // #191c28
const BG_CARD:        Color32 = Color32::from_rgb(30,  33,  48);   // #1e2130 (approx)
const BG_INPUT:       Color32 = Color32::from_rgb(26,  29,  40);   // #1a1d28
// Borders
const BORDER_DEFAULT: Color32 = Color32::from_rgb(42,  46,  66);   // #2a2e42
const BORDER_SUBTLE:  Color32 = Color32::from_rgb(30,  33,  48);   // #1e2130
const BORDER_ACCENT:  Color32 = Color32::from_rgb(15, 110,  86);   // #0F6E56
// Text
const TEXT_PRIMARY:   Color32 = Color32::from_rgb(200, 205, 232);  // #c8cde8
const TEXT_BODY:      Color32 = Color32::from_rgb(226, 228, 240);  // #e2e4f0
const TEXT_MUTED:     Color32 = Color32::from_rgb(90,  96, 128);   // #5a6080
const TEXT_DIMMED:    Color32 = Color32::from_rgb(58,  64,  96);   // #3a4060

// Accents — teal
const TEAL_STRONG:    Color32 = Color32::from_rgb(29, 158, 117);   // #1D9E75
const TEAL_DARK:      Color32 = Color32::from_rgb(15, 110,  86);   // #0F6E56
const TEAL_LIGHT:     Color32 = Color32::from_rgb(93, 202, 165);   // #5DCAA5
const TEAL_FAINT:     Color32 = Color32::from_rgb(159, 225, 203);  // #9FE1CB
// Status
const ERROR_COLOR:    Color32 = Color32::from_rgb(226,  75,  74);  // #E24B4A
const WARN_COLOR:     Color32 = Color32::from_rgb(239, 159,  39);  // #EF9F27
const SUCCESS_COLOR:  Color32 = Color32::from_rgb(80,  250, 123);

// File type badge colours (background, border, icon-tint)
const BADGE_GREEN:  (Color32, Color32) = (Color32::from_rgb(12, 31, 24),  Color32::from_rgb(15, 110, 86));
const BADGE_PURPLE: (Color32, Color32) = (Color32::from_rgb(26, 20, 32),  Color32::from_rgb(58, 42, 72));
const BADGE_ORANGE: (Color32, Color32) = (Color32::from_rgb(26, 21, 8),   Color32::from_rgb(58, 46, 26));
const BADGE_BLUE:   (Color32, Color32) = (Color32::from_rgb(12, 20, 40),  Color32::from_rgb(26, 42, 72));

const VAULT_DIR: &str = "vault_storage";
const DB_PATH:   &str = "vault_storage/vault.db";

// ── Entry Point ───────────────────────────────────────────────
fn main() -> Result<(), eframe::Error> {
    std::fs::create_dir_all(VAULT_DIR).ok();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 680.0])
            .with_resizable(false)
            .with_title_shown(false),
        ..Default::default()
    };

    eframe::run_native(
        "Aegis Vault",
        options,
        Box::new(|cc| {
            setup_style(cc.egui_ctx.clone());
            let db = VaultDb::open(Path::new(DB_PATH)).expect("Gagal buka database");
            Box::new(VaultApp::new(db))
        }),
    )
}

// ── Style Setup ───────────────────────────────────────────────
fn setup_style(ctx: egui::Context) {
    let mut style   = (*ctx.style()).clone();
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(TEXT_BODY);
    visuals.window_fill          = BG_BASE;
    visuals.panel_fill           = Color32::TRANSPARENT;
    visuals.window_stroke        = Stroke::new(0.5, BORDER_SUBTLE);

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill   = BG_SURFACE;
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_MUTED);
    w.noninteractive.rounding  = Rounding::same(8.0);
    w.inactive.bg_fill         = BG_INPUT;
    w.inactive.fg_stroke       = Stroke::new(0.5, BORDER_DEFAULT);
    w.inactive.rounding        = Rounding::same(8.0);
    w.hovered.bg_fill          = BG_CARD;
    w.hovered.bg_stroke        = Stroke::new(0.5, TEAL_STRONG);
    w.hovered.rounding         = Rounding::same(8.0);
    w.active.bg_fill           = TEAL_DARK;
    w.active.fg_stroke         = Stroke::new(1.0, Color32::WHITE);
    w.active.rounding          = Rounding::same(8.0);

    style.text_styles = [
        (egui::TextStyle::Heading,  FontId::new(20.0, FontFamily::Proportional)),
        (egui::TextStyle::Body,     FontId::new(14.0, FontFamily::Proportional)),
        (egui::TextStyle::Button,   FontId::new(14.0, FontFamily::Proportional)),
        (egui::TextStyle::Small,    FontId::new(11.0, FontFamily::Proportional)),
        (egui::TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ].into();

    style.spacing.item_spacing   = Vec2::new(8.0, 8.0);
    style.spacing.window_margin  = egui::Margin::same(0.0);
    style.spacing.button_padding = Vec2::new(12.0, 8.0);
    style.visuals                = visuals;
    ctx.set_style(style);
}

// ── App State ─────────────────────────────────────────────────
#[derive(Default, PartialEq, Clone)]
enum AppScreen {
    #[default]
    Login,
    SetupPin,
    Dashboard,
    Decrypting(String),
}

struct VaultApp {
    db:               Arc<Mutex<VaultDb>>,
    screen:           AppScreen,

    pin_digits:       String,      // numpad accumulator (max 6)
    pin_input:        String,      // setup PIN field 1
    pin_confirm:      String,      // setup PIN field 2
    pin_error:        Option<String>,
    pin_shake_timer:  f32,         // countdown for shake animation

    session_key:      Option<Box<[u8; KEY_LEN]>>,
    session_salt:     Option<[u8; SALT_LEN]>,

    file_list:        Vec<FileRecord>,
    status_message:   Option<(String, bool)>,

    decrypt_target:   Option<FileRecord>,
    decrypt_out_name: String,
}

impl VaultApp {
    fn new(db: VaultDb) -> Self {
        Self {
            db:               Arc::new(Mutex::new(db)),
            screen:           AppScreen::Login,
            pin_digits:       String::new(),
            pin_input:        String::new(),
            pin_confirm:      String::new(),
            pin_error:        None,
            pin_shake_timer:  0.0,
            session_key:      None,
            session_salt:     None,
            file_list:        Vec::new(),
            status_message:   None,
            decrypt_target:   None,
            decrypt_out_name: String::new(),
        }
    }

    fn logout(&mut self) {
        if let Some(mut k) = self.session_key.take() { k.zeroize(); }
        self.session_salt   = None;
        self.pin_digits     = String::new();
        self.pin_error      = None;
        self.file_list      = Vec::new();
        self.screen         = AppScreen::Login;
        self.decrypt_target = None;
        self.status_message = None;
    }

    fn load_files(&mut self) {
        let db = self.db.lock().unwrap();
        self.file_list = db.get_all_files().unwrap_or_default();
    }

    fn set_status(&mut self, msg: &str, ok: bool) {
        self.status_message = Some((msg.to_string(), ok));
    }

    fn total_vault_size(&self) -> u64 {
        self.file_list.iter().map(|r| r.file_size as u64).sum()
    }
}

// ── Render Loop ───────────────────────────────────────────────
impl eframe::App for VaultApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Dark gradient background
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

        // Tick shake timer
        if self.pin_shake_timer > 0.0 {
            self.pin_shake_timer -= ctx.input(|i| i.stable_dt);
            ctx.request_repaint();
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let screen = self.screen.clone();
                match screen {
                    AppScreen::Login              => self.render_login(ui),
                    AppScreen::SetupPin           => self.render_setup_pin(ui),
                    AppScreen::Dashboard          => self.render_dashboard(ui),
                    AppScreen::Decrypting(fname)  => self.render_decrypt_panel(ui, &fname.clone()),
                }
            });
    }
}

// ── Helper: draw a filled rounded rect ───────────────────────
fn filled_rect(ui: &mut egui::Ui, rect: egui::Rect, fill: Color32, stroke: Stroke, rounding: f32) {
    ui.painter().rect(rect, Rounding::same(rounding), fill, stroke);
}

// ── Helper: section card frame ────────────────────────────────
fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(0.5, BORDER_DEFAULT))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
}

// ── Helper: teal button ───────────────────────────────────────
fn teal_btn(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    let desired = Vec2::new(width, 42.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        Color32::from_rgb(10, 80, 62)
    } else if response.hovered() {
        TEAL_STRONG
    } else {
        TEAL_DARK
    };
    ui.painter().rect(rect, Rounding::same(8.0), fill, Stroke::NONE);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::new(14.0, FontFamily::Proportional),
        Color32::WHITE,
    );
    response
}

// ── Helper: ghost button ──────────────────────────────────────
fn ghost_btn(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    let desired = Vec2::new(width, 42.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let border = if response.hovered() { TEAL_STRONG } else { BORDER_DEFAULT };
    let text_c = if response.hovered() { TEXT_PRIMARY } else { TEXT_MUTED };
    ui.painter().rect(rect, Rounding::same(8.0), Color32::TRANSPARENT, Stroke::new(0.5, border));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::new(14.0, FontFamily::Proportional),
        text_c,
    );
    response
}

// ── Helper: numpad button ─────────────────────────────────────
fn numpad_btn(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let desired = Vec2::new(72.0, 56.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        Color32::from_rgb(34, 37, 56)
    } else if response.hovered() {
        Color32::from_rgb(34, 37, 56)
    } else {
        BG_INPUT
    };
    let border = if response.hovered() {
        Stroke::new(0.5, Color32::from_rgb(58, 63, 88))
    } else {
        Stroke::new(0.5, BORDER_DEFAULT)
    };
    ui.painter().rect(rect, Rounding::same(10.0), fill, border);
    let font_size = if label.len() > 1 { 12.0 } else { 20.0 };
    let text_color = if label.len() > 1 { TEXT_MUTED } else { TEXT_PRIMARY };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::new(font_size, FontFamily::Proportional),
        text_color,
    );
    response
}

// ── Helper: file type icon + badge colour ─────────────────────
fn file_badge(ext: &str) -> (&'static str, (Color32, Color32)) {
    match ext {
        "pdf" | "doc" | "docx" | "txt" | "md"  => ("📄", BADGE_GREEN),
        "zip" | "tar" | "gz" | "rar" | "7z"     => ("📦", BADGE_PURPLE),
        "jpg" | "jpeg" | "png" | "gif" | "webp" => ("🖼", BADGE_BLUE),
        "mp4" | "mov" | "avi" | "mkv"           => ("🎬", BADGE_PURPLE),
        "env" | "sh"  | "rs"  | "py" | "js"     => ("⚙", BADGE_ORANGE),
        _                                        => ("📁", BADGE_BLUE),
    }
}

fn file_ext(name: &str) -> &str {

    // rsplit gives ext without dot
    name.rsplit('.').next().unwrap_or("")
}

// ── Screen: Login (numpad) ────────────────────────────────────
impl VaultApp {
    fn render_login(&mut self, ui: &mut egui::Ui) {
        let pin_set = self.db.lock().unwrap().is_pin_set();

        // Outer padding
        let avail = ui.available_rect_before_wrap();
        ui.allocate_ui_at_rect(avail, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(44.0);

                // Shield icon
                let icon_size = Vec2::splat(56.0);
                let (icon_rect, _) = ui.allocate_exact_size(icon_size, egui::Sense::hover());
                ui.painter().rect(icon_rect, Rounding::same(14.0), TEAL_DARK, Stroke::NONE);
                ui.painter().text(
                    icon_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "🛡",
                    FontId::new(26.0, FontFamily::Proportional),
                    TEAL_FAINT,
                );

                ui.add_space(14.0);
                ui.label(egui::RichText::new("Aegis Vault")
                    .size(20.0).color(TEXT_BODY).strong());
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Akses aman ke data kamu")
                    .size(13.0).color(TEXT_MUTED));

                if !pin_set {
                    ui.add_space(32.0);
                    ui.label(egui::RichText::new("Vault baru terdeteksi.").color(WARN_COLOR).size(13.0));
                    ui.label(egui::RichText::new("Buat PIN untuk memulai.").color(TEXT_MUTED).size(13.0));
                    ui.add_space(20.0);
                    if teal_btn(ui, "⚙  Setup PIN", 200.0).clicked() {
                        self.screen = AppScreen::SetupPin;
                    }
                    return;
                }

                ui.add_space(36.0);

                // PIN dot indicators (6 dots)
                let shake_offset = if self.pin_shake_timer > 0.0 {
                    let t = self.pin_shake_timer * 20.0;
                    (t.sin() * 6.0) as f32
                } else { 0.0 };

                let total_w = 6.0 * 12.0 + 5.0 * 12.0; // 132.0
                let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, 12.0), egui::Sense::hover());
                
                let start_x = rect.left() + shake_offset;
                let cy = rect.center().y;
                for i in 0..6usize {
                    let filled = i < self.pin_digits.len();
                    let dot_fill = if filled { TEAL_STRONG } else { Color32::TRANSPARENT };
                    let cx = start_x + 6.0 + i as f32 * 24.0;
                    ui.painter().circle(
                        egui::pos2(cx, cy),
                        5.5,
                        dot_fill,
                        Stroke::new(1.5, TEAL_STRONG),
                    );
                }

                ui.add_space(8.0);
                // Error text
                let err_text = self.pin_error.clone().unwrap_or_default();
                ui.label(egui::RichText::new(&err_text).size(13.0).color(ERROR_COLOR));

                ui.add_space(20.0);

                // Numpad grid (3x4)
                let numpad_keys = [
                    ["1","2","3"],
                    ["4","5","6"],
                    ["7","8","9"],
                    ["hapus","0","⌫"],
                ];

                for row in &numpad_keys {
                    let row_w = 3.0 * 72.0 + 2.0 * 10.0;
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(row_w, 56.0), egui::Sense::hover());
                    let mut child_ui = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center));
                    child_ui.spacing_mut().item_spacing.x = 10.0;
                    
                    for &key in row.iter() {
                        let resp = numpad_btn(&mut child_ui, key);
                        if resp.clicked() {
                            match key {
                                "⌫" => { self.pin_digits.pop(); self.pin_error = None; }
                                "hapus" => { self.pin_digits.clear(); self.pin_error = None; }
                                d => {
                                    if self.pin_digits.len() < 6 {
                                        self.pin_digits.push_str(d);
                                        self.pin_error = None;
                                    }
                                    if self.pin_digits.len() == 6 {
                                        self.pin_input = self.pin_digits.clone();
                                        let ok = self.try_login_numpad();
                                        if !ok {
                                            self.pin_shake_timer = 0.4;
                                        }
                                        self.pin_digits.clear();
                                    }
                                }
                            }
                        }
                    }
                    ui.add_space(10.0);
                }
            });
        });
    }

    /// Returns true on success
    fn try_login_numpad(&mut self) -> bool {
        let db          = self.db.lock().unwrap();
        let pin_hash_db = db.get_pin_hash().unwrap_or(None);
        let salt_hex_db = db.get_pin_salt().unwrap_or(None);
        drop(db);

        if let (Some(stored_hash), Some(salt_hex)) = (pin_hash_db, salt_hex_db) {
            let salt_bytes = hex::decode(&salt_hex).unwrap_or_default();
            if salt_bytes.len() != SALT_LEN {
                self.pin_error = Some("Data vault rusak.".into());
                return false;
            }
            let mut salt = [0u8; SALT_LEN];
            salt.copy_from_slice(&salt_bytes);
            let computed = hash_pin(&self.pin_input, &salt);
            if computed == stored_hash {
                let key = derive_key(&self.pin_input, &salt);
                self.session_key  = Some(key);
                self.session_salt = Some(salt);
                self.pin_input.zeroize();
                self.pin_error = None;
                self.load_files();
                self.screen = AppScreen::Dashboard;
                return true;
            } else {
                self.pin_error = Some("PIN salah. Coba lagi.".into());
                self.pin_input.zeroize();
                return false;
            }
        }
        self.pin_error = Some("Data PIN tidak ditemukan.".into());
        false
    }
}

// ── Screen: Setup PIN ─────────────────────────────────────────
impl VaultApp {
    fn render_setup_pin(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_rect_before_wrap();
        ui.allocate_ui_at_rect(avail, |ui| {
            ui.add_space(32.0);

            // Header row with icon
            ui.horizontal(|ui| {
                ui.add_space(36.0);
                // Key icon box
                let box_size = Vec2::splat(38.0);
                let (rect, _) = ui.allocate_exact_size(box_size, egui::Sense::hover());
                ui.painter().rect(rect, Rounding::same(10.0), BG_SURFACE,
                                  Stroke::new(0.5, BORDER_DEFAULT));
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "🔑",
                                  FontId::new(18.0, FontFamily::Proportional), TEAL_STRONG);
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Buat PIN baru").size(15.0).color(TEXT_BODY).strong());
                    ui.label(egui::RichText::new("Harus 6 digit angka").size(12.0).color(TEXT_MUTED));
                });
            });

            ui.add_space(24.0);

            // Input fields
            let field_w = avail.width() - 72.0;
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(36.0, 0.0))
                .show(ui, |ui| {
                    // PIN baru
                    ui.label(egui::RichText::new("PIN baru").size(12.0).color(TEXT_MUTED));
                    ui.add_space(6.0);
                    let f1_frame = egui::Frame::none()
                        .fill(BG_SURFACE)
                        .stroke(Stroke::new(0.5, BORDER_DEFAULT))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(14.0, 10.0));
                    f1_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🔒").size(16.0).color(TEXT_MUTED));
                            ui.add_space(8.0);
                            ui.add(egui::TextEdit::singleline(&mut self.pin_input)
                                .password(true)
                                .hint_text("6 digit angka")
                                .desired_width(field_w - 80.0)
                                .font(FontId::new(16.0, FontFamily::Proportional))
                                .frame(false));
                        });
                    });

                    ui.add_space(14.0);

                    // Konfirmasi PIN
                    ui.label(egui::RichText::new("Konfirmasi PIN").size(12.0).color(TEXT_MUTED));
                    ui.add_space(6.0);
                    let accent_border = if !self.pin_confirm.is_empty() {
                        TEAL_STRONG
                    } else {
                        BORDER_DEFAULT
                    };
                    let icon_color = if !self.pin_confirm.is_empty() { TEAL_STRONG } else { TEXT_MUTED };
                    egui::Frame::none()
                        .fill(BG_SURFACE)
                        .stroke(Stroke::new(0.5, accent_border))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("🔒").size(16.0).color(icon_color));
                                ui.add_space(8.0);
                                ui.add(egui::TextEdit::singleline(&mut self.pin_confirm)
                                    .password(true)
                                    .hint_text("Ulangi PIN")
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
                                    "PIN di-hash dengan PBKDF2-HMAC-SHA256 (310.000 iterasi) dan salt unik. Tidak ada cara memulihkan PIN yang hilang."
                                ).size(12.0).color(TEAL_LIGHT)).wrap(true));
                            });
                        });

                    ui.add_space(24.0);

                    if let Some(err) = &self.pin_error.clone() {
                        ui.label(egui::RichText::new(err).color(ERROR_COLOR).size(13.0));
                        ui.add_space(8.0);
                    }

                    if teal_btn(ui, "Simpan PIN & masuk", ui.available_width()).clicked() {
                        self.do_setup_pin();
                    }
                });
        });
    }

    fn do_setup_pin(&mut self) {
        if self.pin_input.len() != 6 {
            self.pin_error = Some("PIN harus tepat 6 digit.".into());
            return;
        }
        if !self.pin_input.chars().all(|c| c.is_ascii_digit()) {
            self.pin_error = Some("PIN hanya boleh angka.".into());
            return;
        }
        if self.pin_input != self.pin_confirm {
            self.pin_error = Some("PIN tidak cocok.".into());
            self.pin_confirm.zeroize();
            return;
        }

        let salt     = generate_salt();
        let pin_hash = hash_pin(&self.pin_input, &salt);
        let salt_hex = hex::encode(salt);
        {
            let db = self.db.lock().unwrap();
            db.set_pin(&pin_hash, &salt_hex).expect("Gagal simpan PIN");
        }
        let key = derive_key(&self.pin_input, &salt);
        self.session_key  = Some(key);
        self.session_salt = Some(salt);
        self.pin_input.zeroize();
        self.pin_confirm.zeroize();
        self.pin_error = None;
        self.load_files();
        self.screen = AppScreen::Dashboard;
    }
}

// ── Screen: Dashboard ─────────────────────────────────────────
impl VaultApp {
    fn render_dashboard(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_rect_before_wrap();

        // ─ Topbar ─
        let topbar_rect = egui::Rect::from_min_size(
            avail.min,
            Vec2::new(avail.width(), 52.0),
        );
        filled_rect(ui, topbar_rect,
                    Color32::from_rgb(14, 16, 22),
                    Stroke::new(0.5, BORDER_SUBTLE),
                    0.0,
        );
        // Logo + title
        let logo_rect = egui::Rect::from_min_size(
            topbar_rect.min + Vec2::new(18.0, 12.0),
            Vec2::splat(28.0),
        );
        filled_rect(ui, logo_rect, TEAL_DARK, Stroke::NONE, 7.0);
        ui.painter().text(logo_rect.center(), egui::Align2::CENTER_CENTER, "🛡",
                          FontId::new(14.0, FontFamily::Proportional), TEAL_FAINT);
        ui.painter().text(
            egui::pos2(logo_rect.right() + 10.0, topbar_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Aegis Vault",
            FontId::new(14.0, FontFamily::Proportional),
            TEXT_PRIMARY,
        );

        // Sesi aktif badge
        let badge_right = topbar_rect.right() - 50.0;
        let badge_y     = topbar_rect.center().y;
        let badge_rect  = egui::Rect::from_center_size(
            egui::pos2(badge_right - 30.0, badge_y),
            Vec2::new(90.0, 22.0),
        );
        filled_rect(ui, badge_rect, Color32::from_rgb(12, 31, 24),
                    Stroke::new(0.5, BORDER_ACCENT), 20.0);
        ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER,
                          "🔒 Sesi aktif",
                          FontId::new(11.0, FontFamily::Proportional), TEAL_LIGHT);

        // Logout button
        let logout_rect = egui::Rect::from_center_size(
            egui::pos2(avail.right() - 26.0, topbar_rect.center().y),
            Vec2::new(32.0, 26.0),
        );
        filled_rect(ui, logout_rect, Color32::TRANSPARENT,
                    Stroke::new(0.5, BORDER_DEFAULT), 6.0);
        ui.painter().text(logout_rect.center(), egui::Align2::CENTER_CENTER, "🚪",
                          FontId::new(14.0, FontFamily::Proportional), TEXT_MUTED);
        let logout_resp = ui.allocate_rect(logout_rect, egui::Sense::click());
        if logout_resp.clicked() { self.logout(); return; }

        let mut cursor_y = topbar_rect.bottom() + 14.0;

        // ─ Stat pills ─
        let pill_h   = 62.0;
        let pad      = 16.0;
        let pill_gap = 8.0;
        let pill_w   = (avail.width() - pad * 2.0 - pill_gap * 2.0) / 3.0;
        let stats = [
            ("File tersimpan", format!("{}", self.file_list.len())),
            ("Total terenkripsi", format_size(self.total_vault_size())),
            ("Algoritma", "AES-256".to_string()),
        ];
        for (i, (label, value)) in stats.iter().enumerate() {
            let pill_rect = egui::Rect::from_min_size(
                egui::pos2(avail.left() + pad + i as f32 * (pill_w + pill_gap), cursor_y),
                Vec2::new(pill_w, pill_h),
            );
            filled_rect(ui, pill_rect, BG_SURFACE, Stroke::NONE, 8.0);
            ui.painter().text(
                egui::pos2(pill_rect.left() + 14.0, pill_rect.top() + 14.0),
                egui::Align2::LEFT_TOP,
                *label,
                FontId::new(11.0, FontFamily::Proportional),
                TEXT_MUTED,
            );
            ui.painter().text(
                egui::pos2(pill_rect.left() + 14.0, pill_rect.top() + 30.0),
                egui::Align2::LEFT_TOP,
                value.as_str(),
                FontId::new(if i == 0 { 22.0 } else { 15.0 }, FontFamily::Proportional),
                TEXT_PRIMARY,
            );
        }
        cursor_y += pill_h + 12.0;

        // ─ Divider ─
        let div_y = cursor_y;
        ui.painter().line_segment(
            [egui::pos2(avail.left() + pad, div_y), egui::pos2(avail.right() - pad, div_y)],
            Stroke::new(0.5, BORDER_SUBTLE),
        );
        cursor_y += 10.0;

        // ─ Status message ─
        if let Some((msg, ok)) = &self.status_message.clone() {
            let color = if *ok { SUCCESS_COLOR } else { ERROR_COLOR };
            ui.painter().text(
                egui::pos2(avail.center().x, cursor_y + 8.0),
                egui::Align2::CENTER_TOP,
                msg.as_str(),
                FontId::new(12.0, FontFamily::Proportional),
                color,
            );
            cursor_y += 28.0;
        }

        // ─ File list (scroll area) ─
        let scroll_top    = cursor_y;
        let footer_h      = 36.0;
        let fab_h         = 60.0;
        let scroll_bottom = avail.bottom() - footer_h - fab_h;
        let scroll_rect   = egui::Rect::from_min_max(
            egui::pos2(avail.left(), scroll_top),
            egui::pos2(avail.right(), scroll_bottom),
        );

        let mut to_decrypt: Option<String> = None;

        egui::ScrollArea::vertical()
            .id_source("file_scroll")
            .show_viewport(ui, |ui, _vp| {
                ui.set_clip_rect(scroll_rect);
                if self.file_list.is_empty() {
                    let empty_center = scroll_rect.center();
                    ui.painter().text(empty_center - Vec2::new(0.0, 14.0),
                                      egui::Align2::CENTER_CENTER,
                                      "Brankas Kosong",
                                      FontId::new(18.0, FontFamily::Proportional), TEXT_MUTED);
                    ui.painter().text(empty_center + Vec2::new(0.0, 14.0),
                                      egui::Align2::CENTER_CENTER,
                                      "Tekan ➕ untuk menambah file.",
                                      FontId::new(13.0, FontFamily::Proportional), TEXT_MUTED);
                } else {
                    let card_h   = 68.0;
                    let card_gap = 8.0;
                    for (idx, record) in self.file_list.iter().enumerate() {
                        let card_y = scroll_rect.top() + idx as f32 * (card_h + card_gap) + 4.0;
                        if card_y + card_h > scroll_rect.bottom() { break; }

                        let card_rect = egui::Rect::from_min_size(
                            egui::pos2(avail.left() + pad, card_y),
                            Vec2::new(avail.width() - pad * 2.0, card_h),
                        );
                        let card_resp = ui.allocate_rect(card_rect, egui::Sense::hover());
                        let border_c  = if card_resp.hovered() {
                            Color32::from_rgb(42, 48, 80)
                        } else {
                            Color32::from_rgb(30, 34, 53)
                        };
                        filled_rect(ui, card_rect, BG_SURFACE,
                                    Stroke::new(0.5, border_c), 10.0);

                        // Badge icon
                        let ext           = file_ext(&record.original_name);
                        let (icon, badge) = file_badge(ext);
                        let badge_rect    = egui::Rect::from_min_size(
                            egui::pos2(card_rect.left() + 14.0, card_rect.top() + 16.0),
                            Vec2::splat(36.0),
                        );
                        filled_rect(ui, badge_rect, badge.0,
                                    Stroke::new(0.5, badge.1), 8.0);
                        ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER,
                                          icon, FontId::new(16.0, FontFamily::Proportional),
                                          badge.1);

                        // File info
                        let info_x = badge_rect.right() + 12.0;
                        let name_truncated = if record.original_name.len() > 28 {
                            format!("{}…", &record.original_name[..26])
                        } else {
                            record.original_name.clone()
                        };
                        ui.painter().text(
                            egui::pos2(info_x, card_rect.top() + 16.0),
                            egui::Align2::LEFT_TOP,
                            &name_truncated,
                            FontId::new(14.0, FontFamily::Proportional),
                            TEXT_PRIMARY,
                        );
                        let meta = format!("{}…  ·  {}  ·  {}",
                                           &record.sha256_hash[..6],
                                           format_size(record.file_size as u64),
                                           &record.encrypted_at);
                        ui.painter().text(
                            egui::pos2(info_x, card_rect.top() + 36.0),
                            egui::Align2::LEFT_TOP,
                            &meta,
                            FontId::new(11.0, FontFamily::Proportional),
                            TEXT_DIMMED,
                        );

                        // Decrypt button
                        let btn_w  = 38.0;
                        let btn_rect = egui::Rect::from_min_size(
                            egui::pos2(card_rect.right() - btn_w - 12.0,
                                       card_rect.center().y - 16.0),
                            Vec2::new(btn_w, 32.0),
                        );
                        let btn_resp = ui.allocate_rect(btn_rect, egui::Sense::click());
                        let btn_border = if btn_resp.hovered() {
                            TEAL_STRONG
                        } else {
                            BORDER_DEFAULT
                        };
                        let btn_icon_c = if btn_resp.hovered() { TEAL_STRONG } else { TEXT_MUTED };
                        filled_rect(ui, btn_rect, BG_SURFACE,
                                    Stroke::new(0.5, btn_border), 7.0);
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
        ui.painter().text(
            footer_rect.center(),
            egui::Align2::CENTER_CENTER,
            "3-pass secure delete · PBKDF2 · SHA-256 integrity check",
            FontId::new(11.0, FontFamily::Proportional),
            TEXT_DIMMED,
        );

        // ─ FAB button ─
        let fab_size = Vec2::splat(48.0);
        let fab_rect = egui::Rect::from_min_size(
            egui::pos2(avail.right() - fab_size.x - 20.0,
                       avail.bottom() - footer_h - fab_size.y - 12.0),
            fab_size,
        );
        let fab_resp = ui.allocate_rect(fab_rect, egui::Sense::click());
        let fab_fill = if fab_resp.is_pointer_button_down_on() { TEAL_DARK }
        else if fab_resp.hovered() { TEAL_STRONG }
        else { TEAL_DARK };
        filled_rect(ui, fab_rect, fab_fill, Stroke::NONE, 14.0);
        ui.painter().text(fab_rect.center(), egui::Align2::CENTER_CENTER, "➕",
                          FontId::new(22.0, FontFamily::Proportional), Color32::WHITE);
        if fab_resp.on_hover_text("Tambah & Enkripsi File Baru").clicked() {
            if let Some(path) = FileDialog::new().pick_file() {
                self.do_encrypt(path);
            }
        }

        // Handle decrypt navigation
        if let Some(fname) = to_decrypt {
            if let Some(rec) = self.file_list.iter().find(|r| r.vault_filename == fname) {
                self.decrypt_target   = Some(rec.clone());
                self.decrypt_out_name = rec.original_name.clone();
            }
            self.screen = AppScreen::Decrypting(fname);
        }
    }

    fn do_encrypt(&mut self, source_path: PathBuf) {
        let key = match &self.session_key {
            Some(k) => { let mut a = [0u8; KEY_LEN]; a.copy_from_slice(k.as_ref()); a }
            None    => { self.set_status("Sesi tidak valid. Login ulang.", false); return; }
        };
        let salt = match self.session_salt {
            Some(s) => s,
            None    => { self.set_status("Sesi tidak valid.", false); return; }
        };
        let vault_dir = Path::new(VAULT_DIR);
        let file_size = source_path.metadata().map(|m| m.len()).unwrap_or(0);
        let file_name = source_path.file_name()
            .unwrap_or_default().to_string_lossy().to_string();

        match secure_encrypt_file(&source_path, vault_dir, &key) {
            Ok(result) => {
                let record = FileRecord {
                    id:             Uuid::new_v4().to_string(),
                    original_name:  file_name.clone(),
                    original_path:  source_path.to_string_lossy().to_string(),
                    vault_filename: result.encrypted_filename.clone(),
                    sha256_hash:    result.file_hash.clone(),
                    file_size:      file_size as i64,
                    iv_hex:         hex::encode(result.iv),
                    salt_hex:       hex::encode(salt),
                    encrypted_at:   chrono_now(),
                };
                let insert_err = { let db = self.db.lock().unwrap(); db.insert_file(&record).err() };
                if let Some(e) = insert_err {
                    self.set_status(&format!("Enkripsi berhasil tapi gagal simpan DB: {}", e), false);
                    return;
                }
                self.load_files();
                self.set_status(&format!("✅ Berhasil: {} diamankan.", file_name), true);
            }
            Err(e) => self.set_status(&format!("❌ Gagal enkripsi: {}", e), false),
        }
    }
}

// ── Screen: Decrypt Panel ─────────────────────────────────────
impl VaultApp {
    fn render_decrypt_panel(&mut self, ui: &mut egui::Ui, vault_filename: &str) {
        let record = match &self.decrypt_target {
            Some(r) if r.vault_filename == vault_filename => r.clone(),
            _ => { self.screen = AppScreen::Dashboard; return; }
        };

        let avail = ui.available_rect_before_wrap();
        let pad   = 28.0;

        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(pad, 28.0))
            .show(ui, |ui| {
                // Back button + title
                ui.horizontal(|ui| {
                    let back_rect = egui::Rect::from_min_size(
                        ui.cursor().min,
                        Vec2::new(36.0, 30.0),
                    );
                    let back_resp = ui.allocate_rect(back_rect, egui::Sense::click());
                    filled_rect(ui, back_rect, Color32::TRANSPARENT,
                                Stroke::new(0.5, BORDER_DEFAULT), 7.0);
                    ui.painter().text(back_rect.center(), egui::Align2::CENTER_CENTER, "←",
                                      FontId::new(15.0, FontFamily::Proportional), TEXT_MUTED);
                    if back_resp.clicked() { self.screen = AppScreen::Dashboard; return; }
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("Pulihkan file").size(15.0).color(TEXT_BODY).strong());
                });

                ui.add_space(24.0);

                // File info card
                card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Icon badge
                        let ext           = file_ext(&record.original_name);
                        let (icon, badge) = file_badge(ext);

                        let (badge_alloc, _) = ui.allocate_exact_size(Vec2::splat(34.0), egui::Sense::hover());
                        filled_rect(ui, badge_alloc, badge.0, Stroke::new(0.5, badge.1), 8.0);
                        ui.painter().text(badge_alloc.center(), egui::Align2::CENTER_CENTER, icon,
                                          FontId::new(15.0, FontFamily::Proportional), badge.1);

                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&record.original_name).size(14.0)
                                .color(TEXT_PRIMARY).strong());
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

                    let meta_rows = [
                        ("Vault file",   format!("{}…{}", &record.vault_filename[..8], ".vlt")),
                        ("SHA-256",      format!("{}…", &record.sha256_hash[..8])),
                        ("Dienkripsi",   record.encrypted_at.clone()),
                    ];
                    for (k, v) in &meta_rows {
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
                    .fill(BG_SURFACE)
                    .stroke(Stroke::new(0.5, BORDER_DEFAULT))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("📤").size(16.0).color(TEXT_MUTED));
                            ui.add_space(8.0);
                            ui.add(egui::TextEdit::singleline(&mut self.decrypt_out_name)
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
                                "Hash SHA-256 divalidasi sebelum dekripsi. File asli dihapus permanen dari vault setelah dipulihkan."
                            ).size(12.0).color(Color32::from_rgb(186, 117, 23))).wrap(true));
                        });
                    });

                if let Some((msg, ok)) = &self.status_message.clone() {
                    ui.add_space(12.0);
                    let color = if *ok { SUCCESS_COLOR } else { ERROR_COLOR };
                    ui.label(egui::RichText::new(msg).size(12.0).color(color));
                }

                // Push buttons to bottom
                let used_h = ui.cursor().min.y - avail.top();
                let remaining = (avail.height() - used_h - 80.0).max(12.0);
                ui.add_space(remaining);

                // Action buttons
                ui.horizontal(|ui| {
                    let w = ui.available_width();
                    let cancel_w  = (w - 12.0) * 0.35;
                    let confirm_w = (w - 12.0) * 0.65;

                    if ghost_btn(ui, "Batal", cancel_w).clicked() {
                        self.screen = AppScreen::Dashboard;
                    }
                    ui.add_space(12.0);
                    if teal_btn(ui, "🔓  Pulihkan file", confirm_w).clicked() {
                        let rec = record.clone();
                        self.do_decrypt(&rec);
                    }
                });
            });
    }

    fn do_decrypt(&mut self, record: &FileRecord) {
        let key = match &self.session_key {
            Some(k) => { let mut a = [0u8; KEY_LEN]; a.copy_from_slice(k.as_ref()); a }
            None    => { self.set_status("Sesi tidak valid.", false); return; }
        };

        let salt_bytes = hex::decode(&record.salt_hex).unwrap_or_default();
        if salt_bytes.len() != SALT_LEN {
            self.set_status("Data salt tidak valid di database.", false);
            return;
        }
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&salt_bytes);

        let file_key = derive_key(&hex::encode(key), &salt);

        let vault_path = Path::new(VAULT_DIR).join(&record.vault_filename);
        let out_name   = if self.decrypt_out_name.trim().is_empty() {
            record.original_name.clone()
        } else {
            self.decrypt_out_name.trim().to_string()
        };

        let out_dir = FileDialog::new()
            .set_title("Pilih folder tujuan")
            .pick_folder();
        let out_dir = match out_dir {
            Some(d) => d,
            None    => { self.set_status("Batal: folder tidak dipilih.", false); return; }
        };

        let out_path = out_dir.join(&out_name);

        match secure_decrypt_file(&vault_path, &out_path, &key, &record.sha256_hash) {
            Ok(()) => {
                { let db = self.db.lock().unwrap(); let _ = db.delete_file(&record.id); }
                let _ = std::fs::remove_file(&vault_path);
                self.load_files();
                self.set_status(&format!("✅ File dipulihkan ke: {}", out_path.display()), true);
                self.screen = AppScreen::Dashboard;
            }
            Err(e) => self.set_status(&format!("❌ Dekripsi gagal: {}", e), false),
        }

        drop(file_key);
    }
}

// ── Helpers ───────────────────────────────────────────────────
fn format_size(bytes: u64) -> String {
    if bytes < 1024           { format!("{} B",     bytes) }
    else if bytes < 1024*1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else                      { format!("{:.2} MB", bytes as f64 / (1024.0*1024.0)) }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs  = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let mins  = secs / 60;
    let hours = mins / 60;
    let days  = hours / 24;
    let h     = hours % 24;
    let m     = mins % 60;
    let y     = 1970 + days / 365;
    let d     = (days % 365) + 1;
    format!("{}-{:03} {:02}:{:02}", y, d, h, m)
}