// main.rs — Entry point Aegis Vault (MVC)
// Bertanggung jawab untuk:
//   1. Cek CLI args — jika ada file .vlt, buka FileUnlockApp
//   2. Inisialisasi direktori dan database
//   3. Setup eframe/egui window
//   4. Menghubungkan Model (AppState) + Controller + View

mod app_state;
mod controller;
mod crypto;
mod db;
mod file_handler;
mod theme;
mod totp;
mod view;

use app_state::AppState;
use controller::Controller;
use db::VaultDb;
use eframe::egui;
use std::path::Path;

fn main() -> Result<(), eframe::Error> {
    // ── Cek apakah dipanggil dengan file .vlt ─────────────
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let path = &args[1];
        if path.to_lowercase().ends_with(".vlt") && std::path::Path::new(path).exists() {
            return file_handler::run_file_unlock(path);
        }
    }

    // ── Mode normal: buka aplikasi vault utama ────────────
    std::fs::create_dir_all(controller::VAULT_DIR).ok();

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
            theme::apply(&cc.egui_ctx);
            let db = VaultDb::open(Path::new(controller::DB_PATH))
                .expect("Gagal membuka database");
            Box::new(VaultMvc::new(db))
        }),
    )
}

// ── Root struct MVC ───────────────────────────────────────
struct VaultMvc {
    state:      AppState,
    controller: Controller,
}

impl VaultMvc {
    fn new(db: VaultDb) -> Self {
        Self {
            state:      AppState::default(),
            controller: Controller::new(db),
        }
    }
}

impl eframe::App for VaultMvc {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        view::render(ctx, &mut self.state, &self.controller);
    }
}