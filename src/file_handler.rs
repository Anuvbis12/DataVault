// file_handler.rs — Tampilan "Diamankan oleh" saat .vlt dibuka dari Explorer
// Module terpisah: menampilkan UI unlock dengan logo geometris,
// input PIN numpad, dan tombol untuk membuka/memulihkan file.

use eframe::egui;
use egui::epaint::{Color32, FontId, FontFamily, Mesh, Rounding, Stroke, Vec2, Vertex};
#[cfg(not(target_os = "android"))]
#[cfg(not(target_os = "android"))]
use rfd::FileDialog;
use std::path::PathBuf;
use zeroize::Zeroize;

use crate::crypto::{derive_key, hash_pin, secure_decrypt_file, SALT_LEN};
use crate::db::{FileRecord, VaultDb};
use crate::theme::*;

// ── Public entry point ───────────────────────────────────
pub fn run_file_unlock(vlt_path: &str) -> Result<(), eframe::Error> {
    let vault_file = PathBuf::from(vlt_path);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 620.0])
            .with_resizable(false)
            .with_title_shown(false),
        ..Default::default()
    };

    eframe::run_native(
        "Aegis Vault — File Terkunci",
        options,
        Box::new(move |cc| {
            crate::theme::apply(&cc.egui_ctx);
            Ok(Box::new(FileUnlockApp::new(vault_file)))
        }),
    )
}

// ── App struct ────────────────────────────────────────────
struct FileUnlockApp {
    vault_file:      PathBuf,
    vault_filename:  String,
    file_record:     Option<FileRecord>,
    db_error:        Option<String>,

    pin_digits:      String,
    pin_error:       Option<String>,
    pin_shake_timer: f32,

    status:          Option<(String, bool)>,
    unlocked:        bool,
}

impl FileUnlockApp {
    fn new(vault_file: PathBuf) -> Self {
        let vault_filename = vault_file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let (file_record, db_error) = Self::lookup_record(&vault_filename);

        Self {
            vault_file,
            vault_filename,
            file_record,
            db_error,
            pin_digits:      String::new(),
            pin_error:       None,
            pin_shake_timer: 0.0,
            status:          None,
            unlocked:        false,
        }
    }

    fn lookup_record(vault_filename: &str) -> (Option<FileRecord>, Option<String>) {
        let db_path = match find_db_path() {
            Some(p) => p,
            None => return (None, Some("Database vault tidak ditemukan.".into())),
        };
        let db = match VaultDb::open(&db_path) {
            Ok(db) => db,
            Err(e) => return (None, Some(format!("Gagal buka database: {}", e))),
        };
        match db.find_by_vault_filename(vault_filename) {
            Ok(Some(rec)) => (Some(rec), None),
            Ok(None)      => (None, Some("File tidak terdaftar di vault.".into())),
            Err(e)        => (None, Some(format!("Error database: {}", e))),
        }
    }

    fn try_unlock(&mut self) {
        let record = match &self.file_record {
            Some(r) => r.clone(),
            None    => { self.pin_error = Some("Data file tidak tersedia.".into()); return; }
        };

        let db_path = match find_db_path() {
            Some(p) => p,
            None    => { self.pin_error = Some("Database tidak ditemukan.".into()); return; }
        };
        let db = match VaultDb::open(&db_path) {
            Ok(db)  => db,
            Err(_)  => { self.pin_error = Some("Gagal buka database.".into()); return; }
        };

        let pwd_hash_db = db.get_password_hash().unwrap_or(None);
        let salt_hex_db = db.get_password_salt().unwrap_or(None);

        let (Some(stored_hash), Some(salt_hex)) = (pwd_hash_db, salt_hex_db) else {
            self.pin_error = Some("Data password tidak ditemukan.".into());
            return;
        };

        let salt_bytes = hex::decode(&salt_hex).unwrap_or_default();
        if salt_bytes.len() != SALT_LEN {
            self.pin_error = Some("Data vault rusak.".into());
            return;
        }
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&salt_bytes);

        let computed = hash_pin(&self.pin_digits, &salt);
        if computed != stored_hash {
            self.pin_error = Some("Password salah. Coba lagi.".into());
            self.pin_shake_timer = 0.4;
            self.pin_digits.clear();
            return;
        }

        // Password benar — derive key
        let key = derive_key(&self.pin_digits, &salt);
        self.pin_digits.zeroize();
        self.pin_error = None;

        // Dialog pilih folder output
        #[cfg(not(target_os = "android"))]
        let out_dir = FileDialog::new()
            .set_title("Pilih folder tujuan")
            .pick_folder();
        #[cfg(target_os = "android")]
        let out_dir: Option<PathBuf> = { self.status = Some(("Memilih folder tujuan belum didukung di Android.".into(), false)); None };
        let out_dir = match out_dir {
            Some(d) => d,
            None    => {
                self.status = Some(("Batal: folder tidak dipilih.".into(), false));
                return;
            }
        };

        let out_path = out_dir.join(&record.original_name);

        match secure_decrypt_file(&self.vault_file, &out_path, key.as_ref(), &record.sha256_hash) {
            Ok(()) => {
                self.status = Some((
                    format!("✅ File dipulihkan ke: {}", out_path.display()),
                    true,
                ));
                self.unlocked = true;
            }
            Err(e) => {
                self.status = Some((format!("❌ Gagal: {}", e), false));
            }
        }
    }
}

// ── eframe::App impl ─────────────────────────────────────
impl eframe::App for FileUnlockApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Background gradient
        let painter = ctx.layer_painter(egui::LayerId::background());
        let rect    = ctx.screen_rect();
        let mut mesh = Mesh::default();
        mesh.vertices.extend([
            Vertex { pos: rect.left_top(),     uv: egui::pos2(0.,0.), color: Color32::from_rgb(12,14,20) },
            Vertex { pos: rect.right_top(),    uv: egui::pos2(1.,0.), color: Color32::from_rgb(14,16,24) },
            Vertex { pos: rect.right_bottom(), uv: egui::pos2(1.,1.), color: Color32::from_rgb(8,10,16)  },
            Vertex { pos: rect.left_bottom(),  uv: egui::pos2(0.,1.), color: Color32::from_rgb(10,12,18) },
        ]);
        mesh.add_triangle(0,1,2);
        mesh.add_triangle(0,2,3);
        painter.add(egui::Shape::Mesh(mesh));

        // Tick shake
        if self.pin_shake_timer > 0.0 {
            self.pin_shake_timer -= ctx.input(|i| i.stable_dt);
            ctx.request_repaint();
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                if self.unlocked {
                    self.render_success(ui);
                } else if self.db_error.is_some() {
                    self.render_error(ui);
                } else {
                    self.render_unlock(ui);
                }
            });
    }
}

// ── Render functions ──────────────────────────────────────
impl FileUnlockApp {
    /// Layar utama: logo + PIN + numpad
    fn render_unlock(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_rect_before_wrap();

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(avail), |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(32.0);

                // ── Logo geometris ──
                let logo_size = 80.0;
                let (logo_rect, _) = ui.allocate_exact_size(
                    Vec2::splat(logo_size), egui::Sense::hover(),
                );
                draw_aegis_logo(ui.painter(), logo_rect.center(), logo_size / 2.0);

                ui.add_space(16.0);

                // ── "Diamankan oleh" ──
                ui.label(
                    egui::RichText::new("Diamankan oleh")
                        .size(13.0).color(text_muted()),
                );
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("AEGIS VAULT")
                        .size(22.0).color(teal_light()).strong(),
                );

                ui.add_space(16.0);

                // ── File info card ──
                if let Some(rec) = &self.file_record {
                    let card_w = 300.0;
                    let (card_rect, _) = ui.allocate_exact_size(
                        Vec2::new(card_w, 52.0), egui::Sense::hover(),
                    );
                    filled_rect(ui, card_rect, bg_surface(),
                                Stroke::new(0.5, border_default()), 10.0);

                    let ext = file_ext(&rec.original_name);
                    let (icon, badge) = file_badge(ext);

                    // Badge icon
                    let badge_rect = egui::Rect::from_min_size(
                        egui::pos2(card_rect.left() + 10.0, card_rect.center().y - 15.0),
                        Vec2::splat(30.0),
                    );
                    filled_rect(ui, badge_rect, badge.0,
                                Stroke::new(0.5, badge.1), 7.0);
                    ui.painter().text(
                        badge_rect.center(), egui::Align2::CENTER_CENTER,
                        icon, FontId::new(14.0, FontFamily::Proportional), badge.1,
                    );

                    // File name + size
                    let info_x = badge_rect.right() + 10.0;
                    let name_display = if rec.original_name.len() > 30 {
                        format!("{}…", &rec.original_name[..28])
                    } else {
                        rec.original_name.clone()
                    };
                    ui.painter().text(
                        egui::pos2(info_x, card_rect.top() + 12.0),
                        egui::Align2::LEFT_TOP, &name_display,
                        FontId::new(13.0, FontFamily::Proportional), text_primary(),
                    );
                    ui.painter().text(
                        egui::pos2(info_x, card_rect.top() + 30.0),
                        egui::Align2::LEFT_TOP,
                        &crate::controller::format_size(rec.file_size as u64),
                        FontId::new(11.0, FontFamily::Proportional), text_dimmed(),
                    );
                }

                ui.add_space(20.0);

                // ── "Masukkan PIN" ──
                ui.label(
                    egui::RichText::new("Masukkan PIN untuk membuka")
                        .size(13.0).color(text_muted()),
                );

                ui.add_space(12.0);

                // ── PIN dots (6) ──
                let dot_total = 6usize;
                let dot_r     = 6.0f32;
                let dot_gap   = 14.0f32;
                let total_w   = dot_total as f32 * dot_r * 2.0
                    + (dot_total - 1) as f32 * dot_gap;
                let shake = if self.pin_shake_timer > 0.0 {
                    (self.pin_shake_timer * 30.0).sin() * 6.0
                } else {
                    0.0
                };

                let (dots_rect, _) = ui.allocate_exact_size(
                    Vec2::new(total_w + 12.0, dot_r * 2.0 + 4.0),
                    egui::Sense::hover(),
                );
                for i in 0..dot_total {
                    let cx = dots_rect.left() + shake
                        + i as f32 * (dot_r * 2.0 + dot_gap) + dot_r;
                    let cy = dots_rect.center().y;
                    let filled = i < self.pin_digits.len();
                    let color = if self.pin_error.is_some() {
                        error_color()
                    } else if filled {
                        teal_strong()
                    } else {
                        border_default()
                    };
                    let fill = if filled { color } else { Color32::TRANSPARENT };
                    ui.painter().circle(
                        egui::pos2(cx, cy), dot_r, fill, Stroke::new(1.5, color),
                    );
                }

                // Error label
                if let Some(err) = &self.pin_error {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(err).color(error_color()).size(12.0),
                    );
                }

                ui.add_space(16.0);

                // ── Numpad 3×4 ──
                let keys: &[&[&str]] = &[
                    &["1","2","3"],
                    &["4","5","6"],
                    &["7","8","9"],
                    &["del","0","hapus"],
                ];
                for row in keys {
                    ui.horizontal(|ui| {
                        for key in *row {
                            let resp = numpad_btn(ui, key);
                            if resp.clicked() {
                                match *key {
                                    "del" => {
                                        self.pin_digits.pop();
                                        self.pin_error = None;
                                    }
                                    "hapus" => {
                                        self.pin_digits.clear();
                                        self.pin_error = None;
                                    }
                                    d => {
                                        if self.pin_digits.len() < 6 {
                                            self.pin_digits.push_str(d);
                                            self.pin_error = None;
                                        }
                                        if self.pin_digits.len() == 6 {
                                            self.try_unlock();
                                        }
                                    }
                                }
                            }
                        }
                    });
                    ui.add_space(6.0);
                }

                ui.add_space(12.0);

                // ── Tombol Masuk ──
                if teal_btn(ui, "🔓  Masuk", 240.0).clicked() {
                    if self.pin_digits.len() == 6 {
                        self.try_unlock();
                    } else {
                        self.pin_error = Some("PIN harus 6 digit.".into());
                    }
                }

                // Status
                if let Some((msg, ok)) = &self.status {
                    ui.add_space(10.0);
                    let c = if *ok { success_color() } else { error_color() };
                    ui.label(egui::RichText::new(msg).color(c).size(12.0));
                }
            });
        });
    }

    /// Layar sukses setelah file berhasil dipulihkan
    fn render_success(&self, ui: &mut egui::Ui) {
        let avail = ui.available_rect_before_wrap();
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(avail), |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(avail.height() * 0.25);

                ui.label(
                    egui::RichText::new("✅").size(48.0),
                );
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new("File Berhasil Dipulihkan")
                        .size(18.0).color(success_color()).strong(),
                );
                ui.add_space(8.0);
                if let Some((msg, _)) = &self.status {
                    ui.label(
                        egui::RichText::new(msg).size(12.0).color(text_muted()),
                    );
                }
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Anda bisa menutup jendela ini.")
                        .size(12.0).color(text_dimmed()),
                );
            });
        });
    }

    /// Layar error jika DB/file tidak ditemukan
    fn render_error(&self, ui: &mut egui::Ui) {
        let avail = ui.available_rect_before_wrap();
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(avail), |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(avail.height() * 0.20);

                // Logo tetap ditampilkan
                let (logo_rect, _) = ui.allocate_exact_size(
                    Vec2::splat(80.0), egui::Sense::hover(),
                );
                draw_aegis_logo(ui.painter(), logo_rect.center(), 40.0);

                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new("Diamankan oleh")
                        .size(13.0).color(text_muted()),
                );
                ui.label(
                    egui::RichText::new("AEGIS VAULT")
                        .size(22.0).color(teal_light()).strong(),
                );

                ui.add_space(24.0);

                // Error card
                let pad = 40.0;
                egui::Frame::none()
                    .fill(Color32::from_rgb(26, 18, 8))
                    .stroke(Stroke::new(0.5, Color32::from_rgb(99, 56, 6)))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::symmetric(16.0, 14.0))
                    .show(ui, |ui| {
                        ui.set_width(avail.width() - pad * 2.0);
                        ui.horizontal_top(|ui| {
                            ui.label(
                                egui::RichText::new("⚠").size(18.0).color(warn_color()),
                            );
                            ui.add_space(8.0);
                            let msg = self.db_error.as_deref()
                                .unwrap_or("Tidak dapat membuka file ini.");
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(msg)
                                        .size(13.0)
                                        .color(Color32::from_rgb(186, 117, 23)),
                                ).wrap(),
                            );
                        });
                    });

                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new(format!("File: {}", self.vault_filename))
                        .size(11.0).color(text_dimmed()),
                );
            });
        });
    }
}

// ── Cari database vault ───────────────────────────────────
fn find_db_path() -> Option<PathBuf> {
    // Coba relatif terhadap executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("vault_storage").join("vault.db");
            if p.exists() { return Some(p); }
        }
    }
    // Coba working directory
    let p = PathBuf::from("vault_storage/vault.db");
    if p.exists() { return Some(p); }
    None
}

// ── Logo Geometris Aegis Vault ────────────────────────────
// Menggambar logo hexagonal dengan pola garis internal
// dan aksen teal di tengah, mirip logo yang diberikan.
fn draw_aegis_logo(painter: &egui::Painter, center: egui::Pos2, radius: f32) {
    let line_color   = Color32::from_rgb(190, 200, 220);
    let accent_color = Color32::from_rgb(100, 185, 215);
    let stroke       = Stroke::new(1.8, line_color);
    let thin_stroke  = Stroke::new(1.2, Color32::from_rgb(140, 150, 175));
    let accent       = Stroke::new(2.2, accent_color);



    // Helper: generate polygon points
    let poly = |r: f32, offset: f32, n: usize| -> Vec<egui::Pos2> {
        (0..n)
            .map(|i| {
                let a = offset + i as f32 * (std::f32::consts::TAU / n as f32);
                egui::pos2(center.x + r * a.cos(), center.y + r * a.sin())
            })
            .collect()
    };

    // Outer hexagon (pointy-top: offset = -PI/2)
    let outer = poly(radius, -std::f32::consts::FRAC_PI_2, 6);
    // Inner hexagon (smaller)
    let inner = poly(radius * 0.42, -std::f32::consts::FRAC_PI_2, 6);

    // Draw outer hexagon
    for i in 0..6 {
        painter.line_segment([outer[i], outer[(i + 1) % 6]], stroke);
    }

    // Draw inner hexagon
    for i in 0..6 {
        painter.line_segment([inner[i], inner[(i + 1) % 6]], thin_stroke);
    }

    // Connect outer to inner vertices (triangulation)
    for i in 0..6 {
        painter.line_segment([outer[i], inner[i]], thin_stroke);
    }

    // Cross-connect: outer[i] → inner[(i+1)%6] (creates faceted look)
    for i in 0..6 {
        painter.line_segment([outer[i], inner[(i + 1) % 6]], thin_stroke);
    }

    // Top triangle accent: outer[0] → midpoints of outer[5]-outer[1]
    let top = outer[0]; // top vertex
    let mid_l = egui::pos2(
        (outer[5].x + inner[5].x) * 0.5,
        (outer[5].y + inner[5].y) * 0.5,
    );
    let mid_r = egui::pos2(
        (outer[1].x + inner[1].x) * 0.5,
        (outer[1].y + inner[1].y) * 0.5,
    );
    painter.line_segment([top, mid_l], thin_stroke);
    painter.line_segment([top, mid_r], thin_stroke);

    // Teal accent: small eye/diamond shape in lower-center
    let ac_y  = center.y + radius * 0.12;
    let ac_rx = radius * 0.14;
    let ac_ry = radius * 0.10;
    // Upper arc (two lines forming a ^)
    painter.line_segment(
        [egui::pos2(center.x - ac_rx, ac_y),
         egui::pos2(center.x, ac_y - ac_ry)],
        accent,
    );
    painter.line_segment(
        [egui::pos2(center.x, ac_y - ac_ry),
         egui::pos2(center.x + ac_rx, ac_y)],
        accent,
    );
    // Lower arc (two lines forming a v)
    painter.line_segment(
        [egui::pos2(center.x - ac_rx, ac_y),
         egui::pos2(center.x, ac_y + ac_ry)],
        accent,
    );
    painter.line_segment(
        [egui::pos2(center.x, ac_y + ac_ry),
         egui::pos2(center.x + ac_rx, ac_y)],
        accent,
    );
}
