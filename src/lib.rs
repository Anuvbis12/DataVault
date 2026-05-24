// lib.rs — Pintu masuk utama untuk Library & Android Aegis Vault
pub mod anti_tamper;
pub mod app_state;
pub mod controller;
pub mod crypto;
pub mod db;
pub mod file_handler;
pub mod recycle_bin;
pub mod theme;
pub mod totp;
pub mod view;


// ── Root struct MVC (harus public agar bisa dibaca main.rs) ──
pub struct VaultMvc {
    pub state:      app_state::AppState,
    pub controller: controller::Controller,
    #[cfg(target_os = "android")]
    pub android_app: Option<android_activity::AndroidApp>,
}

impl VaultMvc {
    pub fn new(db: db::VaultDb) -> Self {
        let mut state = app_state::AppState::default();
        state.security_violation = anti_tamper::check_security_violation();
        Self {
            state,
            controller: controller::Controller::new(db),
            #[cfg(target_os = "android")]
            android_app: None,
        }
    }
}

impl eframe::App for VaultMvc {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // ── Auto-Lock: Deteksi Aktivitas Input Pengguna ──
        let has_activity = ctx.input(|i| {
            !i.events.is_empty() || i.pointer.any_click() || i.pointer.any_down()
        });
        if has_activity {
            self.state.last_activity = std::time::Instant::now();
        }

        // ── Auto-Lock: Timer Ketidakaktifan (Inactivity Timeout 2 Menit) ──
        if self.state.is_authenticated() {
            let elapsed = self.state.last_activity.elapsed().as_secs();
            if elapsed >= 120 { // 120 detik = 2 menit
                self.controller.logout(&mut self.state);
                self.state.set_status("Brankas dikunci otomatis karena tidak ada aktivitas.", false);
            } else {
                // Request repaint 1 detik ke depan untuk mengevaluasi timer secara real-time
                ctx.request_repaint_after(std::time::Duration::from_secs(1));
            }
        } else {
            // Jika belum login, selalu segarkan last_activity
            self.state.last_activity = std::time::Instant::now();
        }

        // Panic Button (Double Esc to Lock & Minimize)
        if ctx.input(|i| i.key_pressed(eframe::egui::Key::Escape)) {
            if let Some(last) = self.state.last_esc_press {
                if last.elapsed().as_millis() < 500 {
                    self.controller.logout(&mut self.state);
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(true));
                    self.state.last_esc_press = None;
                } else {
                    self.state.last_esc_press = Some(std::time::Instant::now());
                }
            } else {
                self.state.last_esc_press = Some(std::time::Instant::now());
            }
        }

        // Handle Drag & Drop
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
            let dropped = ctx.input(|i| i.raw.dropped_files.clone());
            for file in dropped {
                if let Some(path) = file.path {
                    self.controller.encrypt_file(&mut self.state, path);
                }
            }
        }
        if self.state.request_keyboard {
            #[cfg(target_os = "android")]
            if let Some(app) = &self.android_app {
                app.show_soft_input(true);
            }
            self.state.request_keyboard = false;
        }

        view::render(ctx, &mut self.state, &self.controller);

        // 🛡️ FITUR KEAMANAN LINTAS PLATFORM (Termasuk iOS, Android, & Desktop)
        // Jika aplikasi kehilangan fokus (masuk ke background / Recent Apps), 
        // kunci otomatis brankas dan tutup seluruh layar dengan warna hitam untuk mencegah intip.
        if !ctx.input(|i| i.focused) {
            // Auto-lock instan saat kehilangan fokus / minimize
            if self.state.is_authenticated() {
                self.controller.logout(&mut self.state);
            }

            let screen_rect = ctx.screen_rect();
            let painter = ctx.layer_painter(eframe::egui::LayerId::new(
                eframe::egui::Order::Tooltip,
                eframe::egui::Id::new("privacy_screen"),
            ));
            
            // 1. Gambar latar belakang hitam penuh
            painter.rect_filled(screen_rect, 0.0, eframe::egui::Color32::BLACK);
            
            // 2. Tambahkan teks/ikon di tengah agar pengguna tidak mengira aplikasi error
            painter.text(
                screen_rect.center(),
                eframe::egui::Align2::CENTER_CENTER,
                "🔒 DataVault Secured",
                eframe::egui::FontId::proportional(28.0),
                eframe::egui::Color32::WHITE,
            );
        }
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Trace)
            .with_tag("rust.aegis_vault")
    );
    log::info!("Aegis Vault Android is starting...");

    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let data_path = app.internal_data_path().unwrap_or_else(|| std::path::PathBuf::from("/data/data/rust.aegis_vault/files"));
        std::fs::create_dir_all(&data_path).expect("Gagal membuat direktori internal_data_path");

        // Set absolute paths based on Android internal storage
        let vault_p = data_path.join("vault_storage");
        let _ = crate::controller::VAULT_DIR_OVERRIDE.set(vault_p.clone());
        let _ = crate::controller::DB_PATH_OVERRIDE.set(vault_p.join("vault.db"));
        
        std::fs::create_dir_all(&vault_p).expect("Gagal membuat VAULT_DIR");

        let mut options = eframe::NativeOptions::default();
        let app_clone = app.clone();
        options.event_loop_builder = Some(Box::new(move |builder| {
            use winit::platform::android::EventLoopBuilderExtAndroid;
            builder.with_android_app(app_clone);
        }));

        eframe::run_native(
            "Aegis Vault",
            options,
            Box::new(move |cc| {
                crate::theme::apply(&cc.egui_ctx);
                
                // Tangkap error SQLite ke file teks
                let db_res = crate::db::VaultDb::open(crate::controller::db_path());
                if let Err(e) = &db_res {
                    log::error!("SQLite Error: {:?}", e);
                    panic!("SQLite Error: {:?}", e);
                }
                let db = db_res.unwrap();
                
                let mut mvc = VaultMvc::new(db);
                mvc.android_app = Some(app);
                Box::new(mvc)
            }),
        ).unwrap();
    }));

    if let Err(e) = res {
        log::error!("CAUGHT PANIC IN ANDROID MAIN: {:?}", e);
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}

// ── ENTRY POINT KHUSUS IOS ──
#[cfg(target_os = "ios")]
#[no_mangle]
pub extern "C" fn start_app_ios() {
    // Pastikan folder vault tersedia
    std::fs::create_dir_all(controller::vault_dir()).ok();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Aegis Vault",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            let db = db::VaultDb::open(controller::db_path())
                .expect("Gagal membuka database iOS");
            Box::new(VaultMvc::new(db))
        }),
    ).unwrap();
}
