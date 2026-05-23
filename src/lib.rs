// lib.rs — Pintu masuk utama untuk Library & Android Aegis Vault
pub mod app_state;
pub mod controller;
pub mod crypto;
pub mod db;
pub mod file_handler;
pub mod recycle_bin;
pub mod theme;
pub mod totp;
pub mod view;

#[cfg(target_os = "android")]
use std::path::Path;

// ── Root struct MVC (harus public agar bisa dibaca main.rs) ──
pub struct VaultMvc {
    pub state:      app_state::AppState,
    pub controller: controller::Controller,
    #[cfg(target_os = "android")]
    pub android_app: Option<android_activity::AndroidApp>,
}

impl VaultMvc {
    pub fn new(db: db::VaultDb) -> Self {
        Self {
            state:      app_state::AppState::default(),
            controller: controller::Controller::new(db),
            #[cfg(target_os = "android")]
            android_app: None,
        }
    }
}

impl eframe::App for VaultMvc {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
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
    }
}

// ── ENTRY POINT KHUSUS ANDROID ──
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    if let Some(path) = app.internal_data_path() {
        std::fs::create_dir_all(&path).ok();
        std::env::set_current_dir(&path).ok();
    }
    std::fs::create_dir_all(controller::VAULT_DIR).ok();

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
            theme::apply(&cc.egui_ctx);
            // Pada Android, letakkan database di root/sandbox penyimpanan internal aplikasi
            let db = db::VaultDb::open(Path::new(controller::DB_PATH))
                .expect("Gagal membuka database Android");
            let mut mvc = VaultMvc::new(db);
            mvc.android_app = Some(app);
            Box::new(mvc)
        }),
    ).unwrap();
}

// ── ENTRY POINT KHUSUS IOS ──
#[cfg(target_os = "ios")]
#[no_mangle]
pub extern "C" fn start_app_ios() {
    // Pastikan folder vault tersedia
    std::fs::create_dir_all(controller::VAULT_DIR).ok();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Aegis Vault",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            let db = db::VaultDb::open(std::path::Path::new(controller::DB_PATH))
                .expect("Gagal membuka database iOS");
            Box::new(VaultMvc::new(db))
        }),
    ).unwrap();
}
