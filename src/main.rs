mod crypto;

use eframe::{egui, epaint::{Color32, Stroke, Rounding, Vec2, FontId, FontFamily, Shadow, Mesh, Vertex}};
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::fs;

// --- Palet Warna Elegan ---
const GRADIENT_TOP: Color32 = Color32::from_rgb(30, 35, 50);
const GRADIENT_BOTTOM: Color32 = Color32::from_rgb(20, 20, 30);
const GLASS_FILL: Color32 = Color32::from_rgba_premultiplied(40, 45, 60, 150);
const TEXT_COLOR: Color32 = Color32::from_rgb(230, 230, 255);
const ACCENT_COLOR: Color32 = Color32::from_rgb(0, 200, 220); // Teal
const SUCCESS_COLOR: Color32 = Color32::from_rgb(80, 250, 123);
const ERROR_COLOR: Color32 = Color32::from_rgb(255, 100, 100);
const SUBTLE_TEXT_COLOR: Color32 = Color32::from_rgb(160, 165, 190);

// --- Konfigurasi Kunci ---
const DUMMY_KEY: [u8; 32] = [1; 32];

fn main() -> Result<(), eframe::Error> {
    let vault_dir = Path::new("vault_storage");
    if !vault_dir.exists() { let _ = fs::create_dir(vault_dir); }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Aegis Vault",
        options,
        Box::new(|cc| {
            setup_elegant_style(cc.egui_ctx.clone());
            Box::new(VaultApp::default())
        }),
    )
}

fn setup_elegant_style(ctx: egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(TEXT_COLOR);
    visuals.window_fill = GRADIENT_TOP;
    visuals.window_stroke = Stroke::NONE;
    visuals.window_shadow = Shadow::big_dark();

    let widget_visuals = &mut visuals.widgets;
    widget_visuals.noninteractive.bg_fill = GLASS_FILL;
    widget_visuals.noninteractive.fg_stroke = Stroke::new(1.0, SUBTLE_TEXT_COLOR);
    widget_visuals.noninteractive.rounding = Rounding::same(6.0);

    widget_visuals.inactive = widget_visuals.noninteractive;
    widget_visuals.inactive.bg_fill = Color32::TRANSPARENT; // Tombol default transparan

    widget_visuals.hovered.bg_fill = GLASS_FILL;
    widget_visuals.hovered.bg_stroke = Stroke::new(1.0, ACCENT_COLOR);

    widget_visuals.active.bg_fill = ACCENT_COLOR;
    widget_visuals.active.fg_stroke = Stroke::new(1.0, Color32::BLACK);

    style.text_styles = [
        (egui::TextStyle::Heading, FontId::new(32.0, FontFamily::Proportional)),
        (egui::TextStyle::Name("Subheading".into()), FontId::new(24.0, FontFamily::Proportional)),
        (egui::TextStyle::Body, FontId::new(18.0, FontFamily::Proportional)),
        (egui::TextStyle::Button, FontId::new(18.0, FontFamily::Proportional)),
        (egui::TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
    ].into();

    style.spacing.item_spacing = Vec2::new(15.0, 15.0);
    style.spacing.window_margin = egui::Margin::same(20.0);
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
    hash: String,
}

impl eframe::App for VaultApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Latar Belakang Gradien
        let painter = ctx.layer_painter(egui::LayerId::background());
        let rect = ctx.screen_rect();

        let mut mesh = Mesh::default();
        mesh.vertices.push(Vertex {
            pos: rect.left_top(),
            uv: egui::pos2(0.0, 0.0),
            color: GRADIENT_TOP,
        });
        mesh.vertices.push(Vertex {
            pos: rect.right_top(),
            uv: egui::pos2(1.0, 0.0),
            color: GRADIENT_TOP,
        });
        mesh.vertices.push(Vertex {
            pos: rect.right_bottom(),
            uv: egui::pos2(1.0, 1.0),
            color: GRADIENT_BOTTOM,
        });
        mesh.vertices.push(Vertex {
            pos: rect.left_bottom(),
            uv: egui::pos2(0.0, 1.0),
            color: GRADIENT_BOTTOM,
        });
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(egui::Shape::Mesh(mesh));

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.screen {
                AppScreen::Login => self.render_login_screen(ui),
                AppScreen::Dashboard => self.render_dashboard_screen(ui),
            };
        });
    }
}

impl VaultApp {
    fn render_login_screen(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(40.0, 80.0))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("🛡").size(80.0));
                    ui.add_space(10.0);
                    ui.heading("Aegis Vault");
                    ui.label(egui::RichText::new("Akses Aman ke Data Anda").color(SUBTLE_TEXT_COLOR));
                    ui.add_space(60.0);

                    let pin_field = ui.add(
                        egui::TextEdit::singleline(&mut self.pin_input)
                            .password(true)
                            .desired_width(200.0)
                            .font(FontId::new(30.0, FontFamily::Proportional))
                            .frame(false), // Hapus frame bawaan
                    );
                    // Gambar garis bawah manual
                    let painter = ui.painter();
                    let underline_rect = pin_field.rect;
                    painter.line_segment(
                        [underline_rect.left_bottom(), underline_rect.right_bottom()],
                        Stroke::new(1.0, SUBTLE_TEXT_COLOR)
                    );

                    ui.add_space(30.0);

                    let login_button = ui.add_sized([220.0, 50.0], egui::Button::new(egui::RichText::new("🔑 Buka").strong().color(Color32::WHITE)));
                    if login_button.clicked() || (pin_field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                        if self.pin_input == "1234" {
                            self.pin_error = None;
                            self.pin_input.clear();
                            self.screen = AppScreen::Dashboard;
                        } else {
                            self.pin_error = Some("PIN yang Anda masukkan salah.".into());
                        }
                    }

                    if let Some(error) = &self.pin_error {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new(error).color(ERROR_COLOR));
                    }
                });
            });
    }

    fn render_dashboard_screen(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("header_panel").show_inside(ui, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label(egui::RichText::new("🗂️  File Terenkripsi").text_style(egui::TextStyle::Name("Subheading".into())));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🚪").on_hover_text("Keluar").clicked() {
                        self.screen = AppScreen::Login;
                    }
                });
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(status) = &self.status_message {
                let color = if status.contains("Berhasil") { SUCCESS_COLOR } else { ERROR_COLOR };
                ui.label(egui::RichText::new(status).color(color));
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);
                if self.secured_files.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.label(egui::RichText::new("Brankas Kosong").size(20.0).color(SUBTLE_TEXT_COLOR));
                        ui.label(egui::RichText::new("Tekan tombol di bawah untuk memulai.").color(SUBTLE_TEXT_COLOR));
                    });
                } else {
                    for file in &self.secured_files {
                        egui::Frame::none()
                            .fill(GLASS_FILL)
                            .rounding(Rounding::same(8.0))
                            .inner_margin(egui::Margin::same(15.0))
                            .outer_margin(egui::Margin::symmetric(0.0, 5.0))
                            .shadow(Shadow::small_dark())
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("📄").size(40.0));
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(&file.original_name).strong());
                                        ui.label(egui::RichText::new(&file.encrypted_name).small().color(SUBTLE_TEXT_COLOR));
                                        ui.label(egui::RichText::new(format!("Hash: {}", &file.hash[..12])).small().color(SUBTLE_TEXT_COLOR));
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("🔓").on_hover_text("Dekripsi & Kembalikan").clicked() {
                                            self.status_message = Some(format!("Memulai proses dekripsi untuk {}...", file.original_name));
                                        }
                                    });
                                });
                            });
                    }
                }
            });
        });

        // Tombol Aksi Mengambang (Floating Action Button)
        egui::Area::new("fab")
            .fixed_pos(ui.ctx().screen_rect().right_bottom() - Vec2::new(50.0, 50.0))
            .show(ui.ctx(), |ui| {
                let fab = ui.add(egui::Button::new(egui::RichText::new("➕").size(30.0)).frame(true));
                if fab.on_hover_text("Tambah & Enkripsi File Baru").clicked() {
                    if let Some(path) = FileDialog::new().pick_file() {
                        self.encrypt_and_store(path);
                    }
                }
            });
    }

    fn encrypt_and_store(&mut self, source_path: PathBuf) {
        let vault_dir = Path::new("vault_storage");
        let file_name = source_path.file_name().unwrap_or_default().to_string_lossy().to_string();

        match crypto::secure_encrypt_file(&source_path, vault_dir, &DUMMY_KEY) {
            Ok(result) => {
                self.secured_files.push(SecuredFile {
                    original_name: file_name,
                    encrypted_name: result.encrypted_filename,
                    hash: result.file_hash,
                });
                self.status_message = Some("✅ Berhasil: File telah diamankan.".into());
            }
            Err(e) => {
                self.status_message = Some(format!("❌ Gagal: {}", e));
            }
        }
    }
}
