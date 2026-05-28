// main.rs — Launcher Aegis Vault untuk Desktop (Windows/macOS/Linux)
use aegis_vault::db::VaultDb;
use aegis_vault::VaultMvc;

fn main() -> Result<(), eframe::Error> {
    // ── Cek apakah dipanggil dengan file .vlt ─────────────
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let path = &args[1];
        if path.to_lowercase().ends_with(".vlt") && std::path::Path::new(path).exists() {
            return aegis_vault::file_handler::run_file_unlock(path);
        }
    }

    // ── Mode desktop normal ───────────────────────────────
    std::fs::create_dir_all(aegis_vault::controller::vault_dir()).ok();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([460.0, 680.0])
            .with_resizable(false)
            .with_title_shown(false),
        ..Default::default()
    };

    eframe::run_native(
        "Aegis Vault",
        options,
        Box::new(|cc| {
            aegis_vault::theme::apply(&cc.egui_ctx);
            let db = VaultDb::open(aegis_vault::controller::db_path())
                .expect("Gagal membuka database");
            Ok(Box::new(VaultMvc::new(db)))
        }),
    )
}