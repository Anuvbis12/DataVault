mod crypto;

use eframe::egui;
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::fs;

// Konfigurasi Kunci Dummy (Harusnya dari Android Keystore/Secure Enclave)
const DUMMY_KEY: [u8; 32] = [
    1, 2, 3, 4, 5, 6, 7, 8,
    9, 10, 11, 12, 13, 14, 15, 16,
    17, 18, 19, 20, 21, 22, 23, 24,
    25, 26, 27, 28, 29, 30, 31, 32
];

fn main() -> Result<(), eframe::Error> {
    // Siapkan direktori vault
    let vault_dir = Path::new("vault_storage");
    if !vault_dir.exists() {
        let _ = fs::create_dir(vault_dir);
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([450.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Brankas Data Aman",
        options,
        Box::new(|_cc| Box::new(VaultApp::default())),
    )
}

// State Aplikasi GUI
enum AppScreen {
    Login,
    Dashboard,
}

struct SecuredFile {
    original_name: String,
    encrypted_name: String,
    hash: String,
}

struct VaultApp {
    screen: AppScreen,
    pin_input: String,
    pin_error: Option<String>,
    secured_files: Vec<SecuredFile>,
    status_message: Option<String>,
}

impl Default for VaultApp {
    fn default() -> Self {
        Self {
            screen: AppScreen::Login,
            pin_input: String::new(),
            pin_error: None,
            secured_files: Vec::new(),
            status_message: None,
        }
    }
}

impl eframe::App for VaultApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.screen {
            AppScreen::Login => self.render_login_screen(ctx),
            AppScreen::Dashboard => self.render_dashboard_screen(ctx),
        }
    }
}

impl VaultApp {
    fn render_login_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(150.0);
                ui.heading(egui::RichText::new("Brankas Data Aman").size(32.0).strong());
                ui.add_space(10.0);
                ui.label("Gunakan PIN untuk masuk");
                ui.add_space(30.0);

                let pin_field = ui.add_sized(
                    [200.0, 40.0],
                    egui::TextEdit::singleline(&mut self.pin_input)
                        .password(true)
                        .hint_text("Masukkan PIN"),
                );

                ui.add_space(20.0);

                if ui.add_sized([200.0, 40.0], egui::Button::new("Masuk")).clicked() || pin_field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // Simulasi autentikasi (PIN: 1234)
                    if self.pin_input == "1234" {
                        self.pin_error = None;
                        self.pin_input.clear();
                        self.screen = AppScreen::Dashboard;
                    } else {
                        self.pin_error = Some("PIN Salah. Coba lagi.".into());
                    }
                }

                ui.add_space(10.0);
                if ui.add_sized([200.0, 40.0], egui::Button::new("🔐 Login Sidik Jari")).clicked() {
                     // Simulasi bypass biometrik
                     self.pin_error = None;
                     self.pin_input.clear();
                     self.screen = AppScreen::Dashboard;
                }

                if let Some(error) = &self.pin_error {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(error).color(egui::Color32::RED));
                }
            });
        });
    }

    fn render_dashboard_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Dashboard Brankas");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Keluar").clicked() {
                        self.screen = AppScreen::Login;
                        self.status_message = None;
                    }
                });
            });
            ui.separator();
            ui.add_space(10.0);

            // Tombol Tambah File
            if ui.add_sized([ui.available_width(), 40.0], egui::Button::new("➕ Amankan File Baru")).clicked() {
                if let Some(path) = FileDialog::new().pick_file() {
                    self.encrypt_and_store(path);
                }
            }

            ui.add_space(10.0);
            if let Some(status) = &self.status_message {
                ui.label(egui::RichText::new(status).color(egui::Color32::GREEN));
                ui.add_space(10.0);
            }

            ui.separator();
            ui.label(egui::RichText::new("Daftar File Diamankan:").strong());
            ui.add_space(5.0);

            // Daftar File
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.secured_files.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.label("Belum ada file di dalam brankas.");
                    });
                } else {
                    for (_index, file) in self.secured_files.iter().enumerate() {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(&file.original_name).strong());
                                    ui.label(egui::RichText::new(format!("ID: {}", file.encrypted_name)).small().weak());
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("🔓 Kembalikan").clicked() {
                                        // Dalam aplikasi nyata, ini akan memanggil dekripsi
                                        // Untuk simulasi, kita tampilkan pesan saja
                                        self.status_message = Some(format!("Mendekripsi {}...", file.original_name));
                                    }
                                });
                            });
                        });
                        ui.add_space(5.0);
                    }
                }
            });
        });
    }

    fn encrypt_and_store(&mut self, source_path: PathBuf) {
        let vault_dir = Path::new("vault_storage");

        let file_name = source_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        match crate::crypto::secure_encrypt_file(&source_path, vault_dir, &DUMMY_KEY) {
            Ok(result) => {
                self.secured_files.push(SecuredFile {
                    original_name: file_name,
                    encrypted_name: result.encrypted_filename.clone(),
                    hash: result.file_hash,
                });
                self.status_message = Some("✅ File berhasil diamankan dan sumber asli dihapus!".into());
            }
            Err(e) => {
                self.status_message = Some(format!("❌ Gagal mengenkripsi: {}", e));
            }
        }
    }
}
