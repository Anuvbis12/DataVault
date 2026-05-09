// main.rs — Aegis Vault v2
// GUI temanmu dipertahankan sepenuhnya.
// Tambahan: setup PIN, PBKDF2 login, dekripsi + validasi hash,
// SQLite record, secure delete 3-pass.

mod crypto;
mod db;

use crypto::{
    derive_key, generate_salt, hash_pin, secure_decrypt_file,
    secure_encrypt_file, KEY_LEN, SALT_LEN,
};
use db::{FileRecord, VaultDb};
use eframe::{egui, epaint::{Color32, FontId, FontFamily, Mesh, Rounding, Shadow, Stroke, Vec2, Vertex}};
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use zeroize::Zeroize;

// ── Palet Warna (milik temanmu, dipertahankan) ────────────
const GRADIENT_TOP:    Color32 = Color32::from_rgb(30, 35, 50);
const GRADIENT_BOTTOM: Color32 = Color32::from_rgb(20, 20, 30);
const GLASS_FILL:      Color32 = Color32::from_rgba_premultiplied(40, 45, 60, 150);
const TEXT_COLOR:      Color32 = Color32::from_rgb(230, 230, 255);
const ACCENT_COLOR:    Color32 = Color32::from_rgb(0, 200, 220);
const SUCCESS_COLOR:   Color32 = Color32::from_rgb(80, 250, 123);
const ERROR_COLOR:     Color32 = Color32::from_rgb(255, 100, 100);
const SUBTLE_TEXT:     Color32 = Color32::from_rgb(160, 165, 190);
const WARNING_COLOR:   Color32 = Color32::from_rgb(255, 200, 80);

const VAULT_DIR: &str = "vault_storage";
const DB_PATH:   &str = "vault_storage/vault.db";

// ── Entry Point ───────────────────────────────────────────
fn main() -> Result<(), eframe::Error> {
    std::fs::create_dir_all(VAULT_DIR).ok();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 660.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Aegis Vault",
        options,
        Box::new(|cc| {
            setup_style(cc.egui_ctx.clone());
            let db  = VaultDb::open(Path::new(DB_PATH)).expect("Gagal buka database");
            Box::new(VaultApp::new(db))
        }),
    )
}

// ── Style Setup (milik temanmu) ───────────────────────────
fn setup_style(ctx: egui::Context) {
    let mut style   = (*ctx.style()).clone();
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color  = Some(TEXT_COLOR);
    visuals.window_fill           = GRADIENT_TOP;
    visuals.window_stroke         = Stroke::NONE;
    visuals.window_shadow         = Shadow {
        offset: Vec2::new(0.0, 8.0),
        blur: 16.0,
        spread: 0.0,
        color: Color32::from_black_alpha(96),
    };

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill      = GLASS_FILL;
    w.noninteractive.fg_stroke    = Stroke::new(1.0, SUBTLE_TEXT);
    w.noninteractive.rounding     = Rounding::same(6.0);
    w.inactive                    = w.noninteractive;
    w.inactive.bg_fill            = Color32::TRANSPARENT;
    w.hovered.bg_fill             = GLASS_FILL;
    w.hovered.bg_stroke           = Stroke::new(1.0, ACCENT_COLOR);
    w.active.bg_fill              = ACCENT_COLOR;
    w.active.fg_stroke            = Stroke::new(1.0, Color32::BLACK);

    style.text_styles = [
        (egui::TextStyle::Heading,  FontId::new(32.0, FontFamily::Proportional)),
        (egui::TextStyle::Name("Sub".into()), FontId::new(20.0, FontFamily::Proportional)),
        (egui::TextStyle::Body,     FontId::new(15.0, FontFamily::Proportional)),
        (egui::TextStyle::Button,   FontId::new(15.0, FontFamily::Proportional)),
        (egui::TextStyle::Small,    FontId::new(11.0, FontFamily::Proportional)),
    ].into();

    style.spacing.item_spacing   = Vec2::new(12.0, 12.0);
    style.spacing.window_margin  = egui::Margin::same(20.0);
    style.visuals                 = visuals;
    ctx.set_style(style);
}

// ── State Aplikasi ────────────────────────────────────────
#[derive(Default, PartialEq, Clone)]
enum AppScreen {
    #[default]
    Login,
    SetupPin,
    Dashboard,
    Decrypting(String), // vault_filename yang sedang didekripsi
}

struct VaultApp {
    db:               Arc<Mutex<VaultDb>>,
    screen:           AppScreen,

    // Input fields
    pin_input:        String,
    pin_confirm:      String,
    pin_error:        Option<String>,

    // Kunci sesi aktif (di-zeroize saat logout)
    session_key:      Option<Box<[u8; KEY_LEN]>>,
    session_salt:     Option<[u8; SALT_LEN]>,

    // Data tampilan
    file_list:        Vec<FileRecord>,
    status_message:   Option<(String, bool)>, // (pesan, is_success)

    // State dekripsi
    decrypt_target:   Option<FileRecord>,
    decrypt_out_name: String,
}

impl VaultApp {
    fn new(db: VaultDb) -> Self {
        Self {
            db:               Arc::new(Mutex::new(db)),
            screen:           AppScreen::Login,
            pin_input:        String::new(),
            pin_confirm:      String::new(),
            pin_error:        None,
            session_key:      None,
            session_salt:     None,
            file_list:        Vec::new(),
            status_message:   None,
            decrypt_target:   None,
            decrypt_out_name: String::new(),
        }
    }

    fn logout(&mut self) {
        if let Some(mut key) = self.session_key.take() {
            key.zeroize();
        }
        self.session_salt    = None;
        self.pin_input       = String::new();
        self.pin_error       = None;
        self.file_list       = Vec::new();
        self.screen          = AppScreen::Login;
        self.decrypt_target  = None;
    }

    fn load_files(&mut self) {
        let db = self.db.lock().unwrap();
        self.file_list = db.get_all_files().unwrap_or_default();
    }

    fn set_status(&mut self, msg: &str, success: bool) {
        self.status_message = Some((msg.to_string(), success));
    }
}

// ── Render Loop ───────────────────────────────────────────
impl eframe::App for VaultApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Gradien latar belakang (milik temanmu)
        let painter = ctx.layer_painter(egui::LayerId::background());
        let rect    = ctx.screen_rect();
        let mut mesh = Mesh::default();
        mesh.vertices.extend([
            Vertex { pos: rect.left_top(),     uv: egui::pos2(0.0, 0.0), color: GRADIENT_TOP },
            Vertex { pos: rect.right_top(),    uv: egui::pos2(1.0, 0.0), color: GRADIENT_TOP },
            Vertex { pos: rect.right_bottom(), uv: egui::pos2(1.0, 1.0), color: GRADIENT_BOTTOM },
            Vertex { pos: rect.left_bottom(),  uv: egui::pos2(0.0, 1.0), color: GRADIENT_BOTTOM },
        ]);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(egui::Shape::Mesh(mesh));

        egui::CentralPanel::default().show(ctx, |ui| {
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

// ── Screen: Login ─────────────────────────────────────────
impl VaultApp {
    fn render_login(&mut self, ui: &mut egui::Ui) {
        // Cek apakah PIN sudah di-setup
        let pin_set = self.db.lock().unwrap().is_pin_set();

        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(40.0, 60.0))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("🛡").size(80.0));
                    ui.add_space(8.0);
                    ui.heading("Aegis Vault");
                    ui.label(egui::RichText::new("Akses Aman ke Data Anda").color(SUBTLE_TEXT));
                    ui.add_space(50.0);

                    if !pin_set {
                        // Belum ada PIN — arahkan ke setup
                        ui.label(egui::RichText::new("Vault baru terdeteksi.").color(WARNING_COLOR));
                        ui.label(egui::RichText::new("Buat PIN untuk memulai.").color(SUBTLE_TEXT));
                        ui.add_space(20.0);
                        if ui.add_sized([220.0, 50.0],
                            egui::Button::new(egui::RichText::new("⚙ Setup PIN").strong().color(Color32::WHITE))
                        ).clicked() {
                            self.screen = AppScreen::SetupPin;
                        }
                        return;
                    }

                    // Field PIN
                    let pin_field = ui.add(
                        egui::TextEdit::singleline(&mut self.pin_input)
                            .password(true)
                            .hint_text("Masukkan PIN")
                            .desired_width(200.0)
                            .font(FontId::new(28.0, FontFamily::Proportional))
                            .frame(false),
                    );
                    let painter = ui.painter();
                    painter.line_segment(
                        [pin_field.rect.left_bottom(), pin_field.rect.right_bottom()],
                        Stroke::new(1.0, SUBTLE_TEXT),
                    );

                    ui.add_space(30.0);

                    let enter_pressed = pin_field.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    if ui.add_sized([220.0, 50.0],
                        egui::Button::new(egui::RichText::new("🔑 Buka").strong().color(Color32::WHITE))
                    ).clicked() || enter_pressed {
                        self.try_login();
                    }

                    if let Some(err) = &self.pin_error {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new(err).color(ERROR_COLOR));
                    }
                });
            });
    }

    fn try_login(&mut self) {
        let db          = self.db.lock().unwrap();
        let pin_hash_db = db.get_pin_hash().unwrap_or(None);
        let salt_hex_db = db.get_pin_salt().unwrap_or(None);
        drop(db);

        if let (Some(stored_hash), Some(salt_hex)) = (pin_hash_db, salt_hex_db) {
            let salt_bytes = hex::decode(&salt_hex).unwrap_or_default();
            if salt_bytes.len() != SALT_LEN {
                self.pin_error = Some("Data vault rusak.".into());
                return;
            }
            let mut salt = [0u8; SALT_LEN];
            salt.copy_from_slice(&salt_bytes);

            let computed = hash_pin(&self.pin_input, &salt);
            if computed == stored_hash {
                // Login berhasil
                let key          = derive_key(&self.pin_input, &salt);
                self.session_key  = Some(key);
                self.session_salt = Some(salt);
                self.pin_input.zeroize();
                self.pin_error = None;
                self.load_files();
                self.screen    = AppScreen::Dashboard;
            } else {
                self.pin_error = Some("PIN salah. Coba lagi.".into());
                self.pin_input.zeroize();
            }
        } else {
            self.pin_error = Some("Data PIN tidak ditemukan.".into());
        }
    }
}

// ── Screen: Setup PIN ─────────────────────────────────────
impl VaultApp {
    fn render_setup_pin(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(40.0, 60.0))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("⚙").size(60.0));
                    ui.add_space(8.0);
                    ui.heading("Buat PIN Baru");
                    ui.label(egui::RichText::new("PIN minimal 4 digit angka").color(SUBTLE_TEXT));
                    ui.add_space(40.0);

                    // Field PIN baru
                    ui.label("PIN Baru:");
                    let f1 = ui.add(
                        egui::TextEdit::singleline(&mut self.pin_input)
                            .password(true)
                            .hint_text("Minimal 4 digit")
                            .desired_width(200.0)
                            .font(FontId::new(24.0, FontFamily::Proportional))
                            .frame(false),
                    );
                    ui.painter().line_segment(
                        [f1.rect.left_bottom(), f1.rect.right_bottom()],
                        Stroke::new(1.0, SUBTLE_TEXT),
                    );
                    ui.add_space(15.0);

                    // Konfirmasi PIN
                    ui.label("Konfirmasi PIN:");
                    let f2 = ui.add(
                        egui::TextEdit::singleline(&mut self.pin_confirm)
                            .password(true)
                            .hint_text("Ulangi PIN")
                            .desired_width(200.0)
                            .font(FontId::new(24.0, FontFamily::Proportional))
                            .frame(false),
                    );
                    ui.painter().line_segment(
                        [f2.rect.left_bottom(), f2.rect.right_bottom()],
                        Stroke::new(1.0, SUBTLE_TEXT),
                    );
                    ui.add_space(30.0);

                    if ui.add_sized([220.0, 50.0],
                        egui::Button::new(egui::RichText::new("✅ Simpan PIN").strong().color(Color32::WHITE))
                    ).clicked() {
                        self.do_setup_pin();
                    }

                    if let Some(err) = &self.pin_error {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new(err).color(ERROR_COLOR));
                    }
                });
            });
    }

    fn do_setup_pin(&mut self) {
        // Validasi
        if self.pin_input.len() < 4 {
            self.pin_error = Some("PIN minimal 4 digit.".into());
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

        // Auto-login setelah setup
        let key           = derive_key(&self.pin_input, &salt);
        self.session_key  = Some(key);
        self.session_salt = Some(salt);
        self.pin_input.zeroize();
        self.pin_confirm.zeroize();
        self.pin_error = None;
        self.load_files();
        self.screen    = AppScreen::Dashboard;
    }
}

// ── Screen: Dashboard ─────────────────────────────────────
impl VaultApp {
    fn render_dashboard(&mut self, ui: &mut egui::Ui) {
        // Header
        egui::TopBottomPanel::top("header").show_inside(ui, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label(
                    egui::RichText::new("🗂  File Terenkripsi")
                        .text_style(egui::TextStyle::Name("Sub".into()))
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🚪").on_hover_text("Keluar").clicked() {
                        self.logout();
                    }
                });
            });
        });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show_inside(ui, |ui| {
            if let Some((msg, ok)) = &self.status_message {
                let color = if *ok { SUCCESS_COLOR } else { ERROR_COLOR };
                ui.label(egui::RichText::new(msg).color(color).small());
            } else {
                ui.label(
                    egui::RichText::new(format!("{} file diamankan", self.file_list.len()))
                        .color(SUBTLE_TEXT).small()
                );
            }
        });

        // Daftar file
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.file_list.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.label(egui::RichText::new("Brankas Kosong").size(20.0).color(SUBTLE_TEXT));
                        ui.label(egui::RichText::new("Tekan ➕ untuk menambah file.").color(SUBTLE_TEXT));
                    });
                } else {
                    let mut to_decrypt: Option<String> = None;

                    for record in &self.file_list {
                        let vault_fname = record.vault_filename.clone();
                        egui::Frame::none()
                            .fill(GLASS_FILL)
                            .rounding(Rounding::same(8.0))
                            .inner_margin(egui::Margin::same(12.0))
                            .outer_margin(egui::Margin::symmetric(0.0, 4.0))
                            .shadow(Shadow {
                                offset: Vec2::new(0.0, 2.0),
                                blur: 4.0,
                                spread: 0.0,
                                color: Color32::from_black_alpha(96),
                            })
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("📄").size(36.0));
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(&record.original_name).strong());
                                        ui.label(
                                            egui::RichText::new(&record.vault_filename)
                                                .small().color(SUBTLE_TEXT)
                                        );
                                        ui.label(
                                            egui::RichText::new(
                                                format!("SHA256: {}…  |  {}",
                                                    &record.sha256_hash[..12],
                                                    format_size(record.file_size as u64))
                                            ).small().color(SUBTLE_TEXT)
                                        );
                                        ui.label(
                                            egui::RichText::new(&record.encrypted_at)
                                                .small().color(SUBTLE_TEXT)
                                        );
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("🔓")
                                            .on_hover_text("Dekripsi & Kembalikan")
                                            .clicked()
                                        {
                                            to_decrypt = Some(vault_fname);
                                        }
                                    });
                                });
                            });
                    }

                    if let Some(fname) = to_decrypt {
                        // Siapkan record untuk panel dekripsi
                        if let Some(rec) = self.file_list.iter().find(|r| r.vault_filename == fname) {
                            self.decrypt_target   = Some(rec.clone());
                            self.decrypt_out_name = rec.original_name.clone();
                        }
                        self.screen = AppScreen::Decrypting(fname);
                    }
                }
            });
        });

        // FAB tombol tambah file
        egui::Area::new(egui::Id::new("fab"))
            .fixed_pos(ui.ctx().screen_rect().right_bottom() - Vec2::new(55.0, 55.0))
            .show(ui.ctx(), |ui| {
                if ui.add(egui::Button::new(egui::RichText::new("➕").size(28.0)).frame(true))
                    .on_hover_text("Tambah & Enkripsi File Baru")
                    .clicked()
                {
                    if let Some(path) = FileDialog::new().pick_file() {
                        self.do_encrypt(path);
                    }
                }
            });
    }

    fn do_encrypt(&mut self, source_path: PathBuf) {
        let key = match &self.session_key {
            Some(k) => {
                let mut arr = [0u8; KEY_LEN];
                arr.copy_from_slice(k.as_ref());
                arr
            }
            None => {
                self.set_status("Sesi tidak valid. Login ulang.", false);
                return;
            }
        };

        let salt = match self.session_salt {
            Some(s) => s,
            None    => { self.set_status("Sesi tidak valid.", false); return; }
        };

        let vault_dir   = Path::new(VAULT_DIR);
        let file_size   = source_path.metadata().map(|m| m.len()).unwrap_or(0);
        let file_name   = source_path.file_name()
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

                let insert_err = {
                    let db = self.db.lock().unwrap();
                    db.insert_file(&record).err()
                };
                if let Some(e) = insert_err {
                    self.set_status(&format!("Enkripsi berhasil tapi gagal simpan DB: {}", e), false);
                    return;
                }

                self.load_files();
                self.set_status(&format!("✅ Berhasil: {} diamankan.", file_name), true);
            }
            Err(e) => {
                self.set_status(&format!("❌ Gagal enkripsi: {}", e), false);
            }
        }
    }
}

// ── Screen: Dekripsi ──────────────────────────────────────
impl VaultApp {
    fn render_decrypt_panel(&mut self, ui: &mut egui::Ui, vault_filename: &str) {
        let record = match &self.decrypt_target {
            Some(r) if r.vault_filename == vault_filename => r.clone(),
            _ => {
                self.screen = AppScreen::Dashboard;
                return;
            }
        };

        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(30.0, 30.0))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("🔓").size(60.0));
                    ui.add_space(8.0);
                    ui.heading("Pulihkan File");
                    ui.add_space(20.0);

                    // Info file
                    egui::Frame::none()
                        .fill(GLASS_FILL)
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(&record.original_name).strong());
                            ui.label(egui::RichText::new(
                                format!("Vault: {}", &record.vault_filename)
                            ).small().color(SUBTLE_TEXT));
                            ui.label(egui::RichText::new(
                                format!("Hash: {}…", &record.sha256_hash[..16])
                            ).small().color(SUBTLE_TEXT));
                        });

                    ui.add_space(20.0);

                    // Nama file output
                    ui.label("Nama file output:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.decrypt_out_name)
                            .desired_width(280.0)
                            .hint_text("Nama file hasil dekripsi"),
                    );

                    ui.add_space(20.0);

                    ui.label(
                        egui::RichText::new("⚠ Hash akan divalidasi sebelum dekripsi.")
                            .color(WARNING_COLOR).small()
                    );

                    ui.add_space(20.0);

                    ui.horizontal(|ui| {
                        if ui.add_sized([130.0, 45.0],
                            egui::Button::new(egui::RichText::new("↩ Batal").color(Color32::WHITE))
                        ).clicked() {
                            self.screen = AppScreen::Dashboard;
                        }

                        ui.add_space(10.0);

                        if ui.add_sized([130.0, 45.0],
                            egui::Button::new(egui::RichText::new("🔓 Pulihkan").strong().color(Color32::WHITE))
                        ).clicked() {
                            self.do_decrypt(&record);
                        }
                    });

                    if let Some((msg, ok)) = &self.status_message {
                        ui.add_space(15.0);
                        let color = if *ok { SUCCESS_COLOR } else { ERROR_COLOR };
                        ui.label(egui::RichText::new(msg).color(color));
                    }
                });
            });
    }

    fn do_decrypt(&mut self, record: &FileRecord) {
        let key = match &self.session_key {
            Some(k) => {
                let mut arr = [0u8; KEY_LEN];
                arr.copy_from_slice(k.as_ref());
                arr
            }
            None => { self.set_status("Sesi tidak valid.", false); return; }
        };

        // Ambil salt dari record (bukan sesi) — salt per-file
        let salt_bytes = hex::decode(&record.salt_hex).unwrap_or_default();
        if salt_bytes.len() != SALT_LEN {
            self.set_status("Data salt tidak valid di database.", false);
            return;
        }
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&salt_bytes);

        // Derivasi ulang kunci dengan salt per-file
        let file_key = derive_key(
            // Gunakan kunci sesi sebagai "PIN proxy" — kita enkripsi dengan kunci sesi
            // Untuk simplikasi: gunakan kunci sesi langsung (salt per-file tidak mengubah kunci)
            // Di versi produksi: simpan PIN terenkripsi di sesi
            &hex::encode(key),
            &salt,
        );

        let vault_path  = Path::new(VAULT_DIR).join(&record.vault_filename);
        let out_name    = if self.decrypt_out_name.trim().is_empty() {
            record.original_name.clone()
        } else {
            self.decrypt_out_name.trim().to_string()
        };

        // Pilih folder output via dialog
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
                // Hapus record dari DB setelah berhasil dikembalikan
                {
                    let db = self.db.lock().unwrap();
                    let _ = db.delete_file(&record.id);
                }
                // Hapus file vault
                let _ = std::fs::remove_file(&vault_path);

                self.load_files();
                self.set_status(
                    &format!("✅ File dipulihkan ke: {}", out_path.display()), true
                );
                self.screen = AppScreen::Dashboard;
            }
            Err(e) => {
                self.set_status(&format!("❌ Dekripsi gagal: {}", e), false);
            }
        }

        drop(file_key);
    }
}

// ── Helper ────────────────────────────────────────────────
fn format_size(bytes: u64) -> String {
    if bytes < 1024            { format!("{} B",      bytes) }
    else if bytes < 1024*1024  { format!("{:.1} KB",  bytes as f64 / 1024.0) }
    else                       { format!("{:.2} MB",  bytes as f64 / (1024.0*1024.0)) }
}

fn chrono_now() -> String {
    // Tanpa chrono dependency: pakai SystemTime
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format manual: YYYY-MM-DD HH:MM
    let mins  = secs / 60;
    let hours = mins / 60;
    let days  = hours / 24;
    let h     = hours % 24;
    let m     = mins % 60;
    // Epoch days → date (approx, cukup untuk label)
    let y = 1970 + days / 365;
    let d = (days % 365) + 1;
    format!("{}-{:03} {:02}:{:02}", y, d, h, m)
}