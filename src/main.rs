mod crypto;

use eframe::{egui, epaint::Color32};
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::fs;

// --- Palet Warna ---
const BG_COLOR: Color32 = Color32::from_rgb(24, 24, 34);
const FG_COLOR: Color32 = Color32::from_rgb(40, 42, 54);
const TEXT_COLOR: Color32 = Color32::from_rgb(248, 248, 242);
const ACCENT_COLOR: Color32 = Color32::from_rgb(139, 233, 253);
const ERROR_COLOR: Color32 = Color32::from_rgb(255, 85, 85);
const SUCCESS_COLOR: Color32 = Color32::from_rgb(80, 250, 123);

// --- Konfigurasi Kunci ---
const DUMMY_KEY: [u8; 32] = [1; 32];

fn main() -> Result<(), eframe::Error> {
    let vault_dir = Path::new("vault_storage");
    if !vault_dir.exists() { let _ = fs::create_dir(vault_dir); }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([450.0, 600.0])
            .with_transparent(true), // Untuk efek frosted glass
        ..Default::default()
    };

    eframe::run_native(
        "Brankas Data Aman",
        options,
        Box::new(|cc| {
            setup_custom_style(cc.egui_ctx.clone());
            Box::new(VaultApp::default())
        }),
    )
}

fn setup_custom_style(ctx: egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(TEXT_COLOR);
    visuals.window_fill = BG_COLOR;
    visuals.window_stroke = egui::Stroke::NONE;
    visuals.widgets.noninteractive.bg_fill = FG_COLOR;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_COLOR);
    visuals.widgets.inactive.bg_fill = FG_COLOR;
    visuals.widgets.hovered.bg_fill = Color32::from_gray(80);
    visuals.widgets.active.bg_fill = ACCENT_COLOR;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, BG_COLOR);
    visuals.selection.bg_fill = ACCENT_COLOR.linear_multiply(0.5);
    visuals.window_shadow = egui::epaint::Shadow::big_dark();

    style.text_styles = [
        (egui::TextStyle::Heading, egui::FontId::new(30.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Body, egui::FontId::new(18.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Button, egui::FontId::new(18.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Small, egui::FontId::new(14.0, egui::FontFamily::Proportional)),
    ].into();

    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.visuals = visuals;
    ctx.set_style(style);
}

#[derive(Default)]
struct VaultApp {
    screen: AppScreen,
    pin_input: String,
    pin_error: Option<String>,
    secured_files: Vec<SecuredFile>,
    status_message: Option<String>,
}

#[derive(Default, PartialEq)]
enum AppScreen { #[default] Login, Dashboard }

struct SecuredFile {
    original_name: String,
    encrypted_name: String,
}

impl eframe::App for VaultApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Perbaikan: Membuat array [f32; 4] secara manual
        [
            BG_COLOR.r() as f32 / 255.0,
            BG_COLOR.g() as f32 / 255.0,
            BG_COLOR.b() as f32 / 255.0,
            0.9, // Alpha untuk transparansi
        ]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                match self.screen {
                    AppScreen::Login => ui.add(LoginScreen::new(self)),
                    AppScreen::Dashboard => ui.add(DashboardScreen::new(self)),
                };
            });
    }
}

// --- WIDGET LAYAR LOGIN ---
struct LoginScreen<'a> { app: &'a mut VaultApp }
impl<'a> LoginScreen<'a> { fn new(app: &'a mut VaultApp) -> Self { Self { app } } }

impl<'a> egui::Widget for LoginScreen<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.add_space(120.0);
            ui.label(egui::RichText::new("🔐").size(80.0));
            ui.add_space(10.0);
            ui.heading("Brankas Data");
            ui.add_space(40.0);

            let pin_field = ui.add(
                egui::TextEdit::singleline(&mut self.app.pin_input)
                    .password(true)
                    .desired_width(160.0)
                    .font(egui::TextStyle::Heading),
            );
            ui.add_space(20.0);

            if ui.add_sized([200.0, 45.0], egui::Button::new("🔑 Masuk")).clicked() || (pin_field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                if self.app.pin_input == "1234" {
                    self.app.pin_error = None;
                    self.app.pin_input.clear();
                    self.app.screen = AppScreen::Dashboard;
                } else {
                    self.app.pin_error = Some("PIN Salah. Coba lagi.".into());
                }
            }

            if let Some(error) = &self.app.pin_error {
                ui.add_space(10.0);
                ui.label(egui::RichText::new(error).color(ERROR_COLOR));
            }
        }).response
    }
}

// --- WIDGET LAYAR DASHBOARD ---
struct DashboardScreen<'a> { app: &'a mut VaultApp }
impl<'a> DashboardScreen<'a> { fn new(app: &'a mut VaultApp) -> Self { Self { app } } }

impl<'a> egui::Widget for DashboardScreen<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        // --- Header ---
        egui::TopBottomPanel::top("header_panel")
            .frame(egui::Frame::none().fill(FG_COLOR))
            .show_inside(ui, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.heading("🗂️ Dashboard");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🚪 Keluar").clicked() {
                            self.app.screen = AppScreen::Login;
                        }
                    });
                });
            });

        // --- Konten Utama ---
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(15.0)))
            .show_inside(ui, |ui| {
                // Tombol Tambah File
                if ui.add_sized([ui.available_width(), 55.0], egui::Button::new("➕ Amankan File Baru")).clicked() {
                    if let Some(path) = FileDialog::new().pick_file() {
                        self.app.encrypt_and_store(path);
                    }
                }

                // Status Message
                if let Some(status) = &self.app.status_message {
                    let color = if status.contains('✅') { SUCCESS_COLOR } else { ERROR_COLOR };
                    ui.label(egui::RichText::new(status).color(color));
                }
                ui.separator();

                // Daftar File
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.app.secured_files.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(80.0);
                            ui.label(egui::RichText::new("Brankas Anda kosong.").size(20.0).weak());
                        });
                    } else {
                        for file in &self.app.secured_files {
                            egui::Frame::group(ui.style())
                                .rounding(egui::Rounding::same(8.0))
                                .outer_margin(egui::Margin::symmetric(0.0, 5.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("📄").size(32.0));
                                        ui.vertical(|ui| {
                                            ui.label(egui::RichText::new(&file.original_name).strong());
                                            ui.label(egui::RichText::new(&file.encrypted_name).small().weak());
                                        });
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("🔓").clicked() {
                                                self.app.status_message = Some(format!("Mendekripsi {}...", file.original_name));
                                            }
                                        });
                                    });
                                });
                        }
                    }
                });
            }).response
    }
}

impl VaultApp {
    fn encrypt_and_store(&mut self, source_path: PathBuf) {
        let vault_dir = Path::new("vault_storage");
        let file_name = source_path.file_name().unwrap_or_default().to_string_lossy().to_string();

        match crate::crypto::secure_encrypt_file(&source_path, vault_dir, &DUMMY_KEY) {
            Ok(result) => {
                self.secured_files.push(SecuredFile {
                    original_name: file_name,
                    encrypted_name: result.encrypted_filename,
                });
                self.status_message = Some("✅ File berhasil diamankan!".into());
            }
            Err(e) => {
                self.status_message = Some(format!("❌ Gagal: {}", e));
            }
        }
    }
}
