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
pub mod splash;
pub mod view;

#[cfg(target_os = "android")]
pub static PENDING_FILE_RESULT: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn Java_com_aegis_vault_MainActivity_onFileSelectedNative(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    uri: jni::objects::JString,
) {
    if let Ok(uri_str) = env.get_string(&uri) {
        let uri_string: String = uri_str.into();
        if let Ok(mut pending) = PENDING_FILE_RESULT.lock() {
            *pending = Some(uri_string);
        }
    }
}


#[cfg(target_os = "android")]
pub fn request_file_picker(app: &android_activity::AndroidApp) {
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut _).unwrap() };
    let mut env = vm.attach_current_thread().unwrap();
    
    // Get the Activity jobject from ndk_context
    let context_ptr = ndk_context::android_context().context();
    let activity_obj = unsafe { jni::objects::JObject::from_raw(context_ptr as jni::sys::jobject) };
    
    // Get the class of the Activity to avoid ClassNotFoundException on background thread
    if let Ok(class) = env.get_object_class(&activity_obj) {
        let _ = env.call_static_method(class, "requestFilePicker", "()V", &[]);
    }
}

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
        #[cfg(target_os = "android")]
        if let Some(app) = &self.android_app {
            // Process file picker request
            if self.state.request_android_file_picker {
                self.state.request_android_file_picker = false;
                crate::request_file_picker(app);
            }

            // Check for file picker result
            if let Ok(mut pending) = crate::PENDING_FILE_RESULT.lock() {
                if let Some(uri) = pending.take() {
                    if !uri.is_empty() {
                        self.state.android_file_picker_result = Some(uri);
                    }
                }
            }
        }

        // ── Android Safe Area: Baca status bar height dari content_rect ──
        // content_rect().top dikembalikan dalam PHYSICAL pixels, harus dibagi pixels_per_point
        // agar sesuai dengan sistem koordinat logical (dp) yang dipakai egui.
        #[cfg(target_os = "android")]
        if let Some(app) = &self.android_app {
            let content_top = app.content_rect().top as f32;
            let ppp = ctx.pixels_per_point();
            // Konversi ke logical pixels dan batasi ke nilai yang wajar (max 60dp)
            self.state.status_bar_height = (content_top / ppp).clamp(0.0, 60.0);
        }

        // ── Auto-Lock: Deteksi Aktivitas Input Pengguna ──
        let has_activity = ctx.input(|i| {
            !i.events.is_empty() || i.pointer.any_click() || i.pointer.any_down()
        });
        if has_activity {
            self.state.last_activity = std::time::Instant::now();
        }

        // ── Auto-Lock: Timer Ketidakaktifan (Inactivity Timeout 2 Menit) ──
        if self.state.is_authenticated() {
            if ctx.input(|i| i.focused) {
                let elapsed = self.state.last_activity.elapsed().as_secs();
                if elapsed >= 120 { // 120 detik = 2 menit
                    self.controller.logout(&mut self.state);
                    self.state.set_status("Brankas dikunci otomatis karena tidak ada aktivitas.", false);
                } else {
                    // Request repaint 1 detik ke depan untuk mengevaluasi timer secara real-time
                    ctx.request_repaint_after(std::time::Duration::from_secs(1));
                }
            } else {
                // Jika tidak fokus (misal sedang membuka notifikasi atau lockscreen), segarkan last_activity agar tidak ter-lock
                self.state.last_activity = std::time::Instant::now();
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

        // 🛡️ PRIVACY SCREEN: Tutup layar saat kehilangan fokus (panel notifikasi, recent apps, dll)
        // PENTING: Tidak melakukan logout — sesi tetap aktif.
        // Layar hitam hanya mencegah isi brankas terlihat saat app tidak di foreground.
        // Logout hanya terjadi via: inactivity timer 2 menit, tombol Lock, atau double-Esc.
        if !ctx.input(|i| i.focused) {
            let screen_rect = ctx.screen_rect();
            let painter = ctx.layer_painter(eframe::egui::LayerId::new(
                eframe::egui::Order::Tooltip,
                eframe::egui::Id::new("privacy_screen"),
            ));
            
            // Gambar latar belakang hitam penuh menutupi seluruh konten
            painter.rect_filled(screen_rect, 0.0, eframe::egui::Color32::BLACK);
            
            // Teks informasi agar pengguna tidak bingung
            painter.text(
                screen_rect.center(),
                eframe::egui::Align2::CENTER_CENTER,
                "🔒 DataVault Secured",
                eframe::egui::FontId::proportional(28.0),
                eframe::egui::Color32::from_rgb(100, 100, 110),
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

    let app_for_panic = app.clone();
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let data_path = app.internal_data_path().unwrap_or_else(|| std::path::PathBuf::from("/data/data/rust.aegis_vault/files"));
        std::fs::create_dir_all(&data_path).expect("Gagal membuat direktori internal_data_path");

        // Set absolute paths based on Android internal storage
        let vault_p = data_path.join("vault_storage");
        let _ = crate::controller::VAULT_DIR_OVERRIDE.set(vault_p.clone());
        let _ = crate::controller::DB_PATH_OVERRIDE.set(vault_p.join("vault.db"));
        
        std::fs::create_dir_all(&vault_p).expect("Gagal membuat VAULT_DIR");

        let mut options = eframe::NativeOptions::default();
        #[cfg(target_os = "android")]
        {
            options.renderer = eframe::Renderer::Glow;
        }
        
        let app_clone = app_for_panic.clone();
        options.event_loop_builder = Some(Box::new(move |builder| {
            use winit::platform::android::EventLoopBuilderExtAndroid;
            builder.with_android_app(app_clone);
        }));

        let app_clone2 = app_for_panic.clone();
        eframe::run_native(
            "Aegis Vault",
            options,
            Box::new(move |cc| {
                crate::theme::apply(&cc.egui_ctx);
                
                // Tangkap error SQLite dan jangan panic (menghindari JNI abort).
                // Jika error, gunakan database memory sementara untuk menampilkan error di UI.
                let db_path_override = crate::controller::db_path();
                let db_res = crate::db::VaultDb::open(db_path_override);
                let (db, err_msg) = match db_res {
                    Ok(d) => (d, None),
                    Err(e) => {
                        let msg = format!("SQLite Error on path {:?}: {:?}", db_path_override, e);
                        log::error!("{}", msg);
                        (crate::db::VaultDb::open(std::path::Path::new(":memory:")).unwrap(), Some(msg))
                    }
                };
                
                let mut mvc = VaultMvc::new(db);
                if let Some(msg) = err_msg {
                    mvc.state.set_status(&msg, true);
                }
                mvc.android_app = Some(app_clone2);
                Ok(Box::new(mvc))
            }),
        ).unwrap();
    }));

    if let Err(e) = res {
        let msg = if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        log::error!("CAUGHT PANIC IN ANDROID MAIN: {}", msg);
        
        if let Some(ext_path) = app_for_panic.external_data_path() {
            let _ = std::fs::create_dir_all(&ext_path);
            let log_file = ext_path.join("CRASH_LOG.txt");
            let _ = std::fs::write(&log_file, format!("Aegis Vault Crash:\n{}", msg));
            log::error!("WROTE CRASH LOG TO: {:?}", log_file);
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}

// ── ENTRY POINT KHUSUS IOS ──
#[cfg(target_os = "ios")]
#[no_mangle]
pub extern "C" fn start_app_ios() {
    // Pastikan folder vault tersedia
    std::fs::create_dir_all(controller::vault_dir()).ok();

    let mut options = eframe::NativeOptions::default();
        let app_clone = app.clone();
        options.event_loop_builder = Some(Box::new(move |builder| {
            use winit::platform::android::EventLoopBuilderExtAndroid;
            builder.with_android_app(app_clone);
        }));

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
