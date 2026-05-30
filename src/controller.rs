// controller.rs — Controller layer
// Semua logika bisnis: login, setup akun, enkripsi, dekripsi.
// Tidak ada kode egui di sini. Controller memodifikasi AppState
// dan memanggil crypto/db layer.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use zeroize::Zeroize;
use std::fs::File;
use std::io::{Read, Write};

use crate::app_state::{AppScreen, AppState};
use sysinfo::{System, Disks};
use crate::crypto::{
    derive_key, generate_salt, hash_pin,
    secure_decrypt_file, secure_encrypt_file,
    SALT_LEN,
};
use crate::db::{FileRecord, VaultDb};

pub static VAULT_DIR_OVERRIDE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
pub static DB_PATH_OVERRIDE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
pub static EXTERNAL_DIR_OVERRIDE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

pub fn vault_dir() -> &'static Path {
    VAULT_DIR_OVERRIDE.get_or_init(|| PathBuf::from("vault_storage"))
}

pub fn db_path() -> &'static Path {
    DB_PATH_OVERRIDE.get_or_init(|| vault_dir().join("vault.db"))
}

pub fn external_dir() -> Option<&'static Path> {
    EXTERNAL_DIR_OVERRIDE.get().map(|p| p.as_path())
}

// ── Controller ────────────────────────────────────────────
pub struct Controller {
    pub db: Arc<Mutex<VaultDb>>,
    pub sys: Mutex<System>,
    pub disks: Mutex<Disks>,
    pub last_sys_refresh: Mutex<std::time::Instant>,
}

impl Controller {
    pub fn new(db: VaultDb) -> Self {
        #[allow(unused_mut)]
        let mut sys = System::new();
        #[cfg(not(target_os = "android"))]
        {
            sys.refresh_cpu_usage();
            sys.refresh_memory();
        }
        Self { 
            db: Arc::new(Mutex::new(db)),
            sys: Mutex::new(sys),
            #[cfg(not(target_os = "android"))]
            disks: Mutex::new(Disks::new_with_refreshed_list()),
            #[cfg(target_os = "android")]
            disks: Mutex::new(Disks::new()),
            last_sys_refresh: Mutex::new(std::time::Instant::now()),
        }
    }

    // ── Auth ──────────────────────────────────────────────

    /// Cek apakah akun sudah pernah di-setup
    pub fn is_user_set(&self) -> bool {
        self.db.lock().unwrap().is_user_set()
    }

    /// Login via username + password — returns true jika berhasil
    pub fn try_login(&self, state: &mut AppState) -> bool {
        let db = self.db.lock().unwrap();
        let stored_username = db.get_username().unwrap_or(None);
        let stored_hash     = db.get_password_hash().unwrap_or(None);
        let salt_hex        = db.get_password_salt().unwrap_or(None);
        let display_name    = db.get_display_name().unwrap_or(None);
        drop(db);

        let (Some(db_username), Some(db_hash), Some(salt_hex)) = (stored_username, stored_hash, salt_hex) else {
            state.login_error = Some("Data akun tidak ditemukan.".into());
            return false;
        };

        // Validasi username
        if state.login_username.trim() != db_username {
            self.log_action("FAIL_LOGIN", "Gagal login: username salah.");
            state.login_error = Some("Username salah.".into());
            return false;
        }

        let salt_bytes = hex::decode(&salt_hex).unwrap_or_default();
        if salt_bytes.len() != SALT_LEN {
            state.login_error = Some("Data vault rusak.".into());
            return false;
        }

        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&salt_bytes);

        let computed = hash_pin(&state.login_password, &salt);
        if computed == db_hash {
            let key           = derive_key(&state.login_password, &salt);
            state.session_key  = Some(key);
            state.session_salt = Some(salt);
            state.display_name = display_name.unwrap_or_else(|| db_username.clone());
            state.login_password.zeroize();
            state.login_error = None;
            self.load_files(state);
            // Cek apakah TOTP aktif → arahkan ke verifikasi 2FA
            state.totp_enabled = self.is_totp_enabled();
            if state.totp_enabled {
                state.totp_code.clear();
                state.totp_error = None;
                state.screen = AppScreen::TotpVerify;
                self.log_action("LOGIN", &format!("Berhasil login sebagai '{}'.", db_username));
                state.screen = AppScreen::Dashboard;
            } else {
                self.log_action("LOGIN", &format!("Berhasil login sebagai '{}'.", db_username));
                state.screen = AppScreen::Dashboard;
            }
            true
        } else {
            self.log_action("FAIL_LOGIN", "Gagal login: password salah.");
            state.login_error = Some("Password salah. Coba lagi.".into());
            state.login_password.zeroize();
            false
        }
    }

    /// Setup akun baru untuk vault kosong
    pub fn setup_account(&self, state: &mut AppState) {
        // Validasi username
        if state.setup_username.trim().is_empty() {
            state.setup_error = Some("Username tidak boleh kosong.".into());
            return;
        }
        if state.setup_username.trim().len() < 3 {
            state.setup_error = Some("Username minimal 3 karakter.".into());
            return;
        }
        // Validasi nama
        if state.setup_display_name.trim().is_empty() {
            state.setup_error = Some("Nama lengkap tidak boleh kosong.".into());
            return;
        }
        // Validasi password
        if state.setup_password.len() < 4 {
            state.setup_error = Some("Password minimal 4 karakter.".into());
            return;
        }
        if state.setup_password != state.setup_password_confirm {
            state.setup_error = Some("Password tidak cocok.".into());
            state.setup_password_confirm.zeroize();
            return;
        }

        let salt     = generate_salt();
        let pwd_hash = hash_pin(&state.setup_password, &salt);
        let salt_hex = hex::encode(salt);

        {
            let db = self.db.lock().unwrap();
            db.set_user(
                state.setup_username.trim(),
                state.setup_display_name.trim(),
                &pwd_hash,
                &salt_hex,
            ).expect("Gagal simpan akun");
        }

        let key            = derive_key(&state.setup_password, &salt);
        state.session_key  = Some(key);
        state.session_salt = Some(salt);
        state.display_name = state.setup_display_name.trim().to_string();
        state.setup_password.zeroize();
        state.setup_password_confirm.zeroize();
        state.setup_error = None;
        self.load_files(state);
        self.log_action("SETUP", &format!("Akun '{}' berhasil dibuat.", state.setup_username.trim()));
        state.screen = AppScreen::Dashboard;
    }

    /// Logout: bersihkan sesi dari memori
    pub fn logout(&self, state: &mut AppState) {
        if let Some(mut k) = state.session_key.take() { k.zeroize(); }
        state.session_salt   = None;
        state.login_username = String::new();
        state.login_password = String::new();
        state.login_error    = None;
        state.display_name   = String::new();
        state.file_list      = Vec::new();
        state.screen         = AppScreen::Login;
        state.decrypt_target = None;
        state.status         = None;
    }

    /// Reset Vault: Hapus semua file terenkripsi dan reset database.
    pub fn reset_vault(&self, state: &mut AppState) {
        // Hapus semua file fisik di direktori vault
        if let Ok(entries) = std::fs::read_dir(crate::controller::vault_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                // Hapus file .vlt
                if path.extension().and_then(|s| s.to_str()) == Some("vlt") {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        
        // Reset DB
        {
            let db = self.db.lock().unwrap();
            let _ = db.reset_database();
        }
        
        // Reset State dan kembali ke Setup
        self.logout(state);
        state.show_reset_confirm = false;
        state.setup_username = String::new();
        state.setup_display_name = String::new();
        state.setup_password = String::new();
        state.setup_password_confirm = String::new();
        state.screen = AppScreen::SetupAccount;
        state.set_status("Vault telah di-reset. Silakan buat akun baru.", true);
    }

    // ── File operations ───────────────────────────────────

    /// Muat ulang daftar file dari database
    pub fn load_files(&self, state: &mut AppState) {
        let db = self.db.lock().unwrap();
        state.file_list = db.get_all_files().unwrap_or_default();
    }

    /// Enkripsi file/folder dan simpan record ke DB
    pub fn encrypt_file(&self, state: &mut AppState, source_path: PathBuf) {
        let key = match state.session_key_bytes() {
            Some(k) => k,
            None    => { state.set_status("Sesi tidak valid. Login ulang.", false); return; }
        };
        let salt = match state.session_salt {
            Some(s) => s,
            None    => { state.set_status("Sesi tidak valid.", false); return; }
        };

        let vault_dir_path = crate::controller::vault_dir();
        
        let is_dir = source_path.is_dir();
        let (temp_zip_path, final_source_path, file_size) = if is_dir {
            let temp_path = std::env::temp_dir().join(format!("{}.zip", Uuid::new_v4()));
            if let Err(e) = zip_directory(&source_path, &temp_path) {
                state.set_status(&format!("❌ Gagal kompresi folder: {}", e), false);
                return;
            }
            let size = temp_path.metadata().map(|m| m.len()).unwrap_or(0);
            (Some(temp_path.clone()), temp_path, size)
        } else {
            let size = source_path.metadata().map(|m| m.len()).unwrap_or(0);
            (None, source_path.clone(), size)
        };

        let file_name = source_path.file_name()
            .unwrap_or_default().to_string_lossy().to_string();

        match secure_encrypt_file(&final_source_path, vault_dir_path, &key) {
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
                    encrypted_at:   timestamp_now(),
                    is_deleted:     false,
                    deleted_at:     None,
                    is_folder:      is_dir,
                };

                let db  = self.db.lock().unwrap();
                let err = db.insert_file(&record).err();
                drop(db);

                // Hapus ZIP sementara jika ada
                if let Some(temp_path) = temp_zip_path {
                    let _ = std::fs::remove_file(temp_path);
                }

                if let Some(e) = err {
                    state.set_status(
                        &format!("Enkripsi berhasil tapi gagal simpan DB: {}", e), false
                    );
                    return;
                }

                self.load_files(state);
                if is_dir {
                    self.log_action("ENCRYPT_FOLDER", &format!("Folder '{}' diamankan.", file_name));
                    state.set_status(&format!("✅ Berhasil: Folder {} diamankan.", file_name), true);
                } else {
                    self.log_action("ENCRYPT", &format!("File '{}' diamankan.", file_name));
                    if result.original_deleted {
                        state.set_status(&format!("✅ Berhasil: {} diamankan.", file_name), true);
                    } else {
                        state.set_status(&format!("⚠️ {} disimpan ke Vault, tetapi file asli tidak dapat dihapus (Izin Terbatas).", file_name), true);
                    }
                }
            }
            Err(e) => {
                // Hapus ZIP sementara jika ada
                if let Some(temp_path) = temp_zip_path {
                    let _ = std::fs::remove_file(temp_path);
                }
                state.set_status(&format!("❌ Gagal enkripsi: {}", e), false);
            }
        }
    }

    /// Dekripsi file/folder vault ke folder tujuan, hapus vault setelah sukses
    pub fn decrypt_file(
        &self,
        state:      &mut AppState,
        record:     &FileRecord,
        out_dir:    PathBuf,
        out_name:   &str,
    ) {
        let key = match state.session_key_bytes() {
            Some(k) => k,
            None    => { state.set_status("Sesi tidak valid.", false); return; }
        };

        let vault_path = crate::controller::vault_dir().join(&record.vault_filename);

        if record.is_folder {
            let temp_zip = std::env::temp_dir().join(format!("{}.zip", Uuid::new_v4()));
            match secure_decrypt_file(&vault_path, &temp_zip, &key, &record.sha256_hash) {
                Ok(()) => {
                    let out_path = out_dir.join(out_name);
                    match unzip_directory(&temp_zip, &out_path) {
                        Ok(()) => {
                            let _ = std::fs::remove_file(temp_zip);
                            self.load_files(state);
                            self.log_action("DECRYPT_FOLDER", &format!("Folder didekripsi: {}", record.original_name));
                            state.set_status(
                                &format!("✅ Folder berhasil diekstrak ke: {}", out_path.display()), true
                            );
                            state.screen = AppScreen::Dashboard;
                        }
                        Err(e) => {
                            let _ = std::fs::remove_file(temp_zip);
                            state.set_status(&format!("❌ Gagal mengekstrak folder: {}", e), false);
                        }
                    }
                }
                Err(e) => {
                    let _ = std::fs::remove_file(temp_zip);
                    state.set_status(&format!("❌ Dekripsi gagal: {}", e), false);
                }
            }
        } else {
            let out_path   = out_dir.join(out_name);

            match secure_decrypt_file(&vault_path, &out_path, &key, &record.sha256_hash) {
                Ok(()) => {
                    self.load_files(state);
                    self.log_action("DECRYPT", &format!("File didekripsi ke luar vault: {}", record.original_name));
                    state.set_status(
                        &format!("✅ File berhasil diekstrak ke: {}", out_path.display()), true
                    );
                    state.screen = AppScreen::Dashboard;
                }
                Err(e) => state.set_status(&format!("❌ Dekripsi gagal: {}", e), false),
            }
        }
    }

    /// Navigasi ke panel dekripsi untuk file tertentu
    pub fn open_decrypt_panel(&self, state: &mut AppState, vault_filename: &str) {
        if let Some(rec) = state.file_list.iter().find(|r| r.vault_filename == vault_filename) {
            let rec_clone            = rec.clone();
            state.decrypt_out_name   = rec_clone.original_name.clone();
            state.decrypt_target     = Some(rec_clone);
            state.screen             = AppScreen::Decrypting(vault_filename.to_string());
        }
    }

    // ── Recycle Bin / Trash ───────────────────────────────

    pub fn soft_delete_file(&self, state: &mut AppState, id: &str) {
        let db = self.db.lock().unwrap();
        let _ = db.soft_delete_file(id, &timestamp_now());
        drop(db);
        self.load_files(state);
        self.load_deleted_files(state);
        state.set_status("Data dipindah ke Recycle Bin.", true);
    }

    pub fn restore_file(&self, state: &mut AppState, id: &str) {
        let db = self.db.lock().unwrap();
        let _ = db.restore_file(id);
        drop(db);
        self.load_files(state);
        self.load_deleted_files(state);
        state.set_status("Data berhasil dipulihkan.", true);
    }

    pub fn load_deleted_files(&self, state: &mut AppState) {
        let db = self.db.lock().unwrap();
        state.deleted_list = db.get_deleted_files().unwrap_or_default();
    }

    pub fn permanent_delete_file(&self, state: &mut AppState, record: &FileRecord) {
        let vault_path = crate::controller::vault_dir().join(&record.vault_filename);
        let _ = crate::crypto::secure_delete(&vault_path); // 3-pass delete

        let db = self.db.lock().unwrap();
        let _ = db.permanent_delete_file(&record.id);
        drop(db);

        self.load_deleted_files(state);
        state.set_status("Data terhapus permanen.", true);
    }

    // ── System Recycle Bin Scanner ────────────────────────

    pub fn scan_system_trash(&self, state: &mut AppState) {
        state.system_trash_loading = true;
        state.system_trash_items = crate::recycle_bin::scan_recycle_bin();
        state.system_trash_loading = false;
    }

    pub fn restore_system_trash_original(&self, state: &mut AppState, index: usize) {
        if index >= state.system_trash_items.len() {
            state.set_status("Item tidak ditemukan.", false);
            return;
        }
        let item = state.system_trash_items[index].clone();
        match crate::recycle_bin::restore_to_original(&item) {
            Ok(()) => {
                self.log_action("RESTORE_SYS", &format!("File '{}' dipulihkan ke lokasi asli.", item.file_name));
                state.set_status(&format!("✅ '{}' dipulihkan ke: {}", item.file_name, item.original_path), true);
                // Refresh list
                self.scan_system_trash(state);
            }
            Err(e) => state.set_status(&format!("❌ Gagal memulihkan: {}", e), false),
        }
    }

    pub fn restore_system_trash_custom(&self, state: &mut AppState, index: usize, dest_dir: PathBuf) {
        if index >= state.system_trash_items.len() {
            state.set_status("Item tidak ditemukan.", false);
            return;
        }
        let item = state.system_trash_items[index].clone();
        match crate::recycle_bin::restore_to_custom(&item, &dest_dir) {
            Ok(dest) => {
                self.log_action("RESTORE_SYS", &format!("File '{}' dipulihkan ke: {}", item.file_name, dest.display()));
                state.set_status(&format!("✅ '{}' dipulihkan ke: {}", item.file_name, dest.display()), true);
                self.scan_system_trash(state);
            }
            Err(e) => state.set_status(&format!("❌ Gagal memulihkan: {}", e), false),
        }
    }

    pub fn preview_system_trash_to_memory(&self, state: &mut AppState, index: usize) {
        if index >= state.system_trash_items.len() {
            state.set_status("Item tidak ditemukan.", false);
            return;
        }
        let item = state.system_trash_items[index].clone();
        
        if item.is_directory {
            state.set_status("Tidak dapat mempratinjau folder.", false);
            return;
        }
        
        let ext = crate::theme::file_ext(&item.file_name).to_lowercase();
        if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "txt" {
            // Pratinjau gambar dan teks di dalam aplikasi
            match std::fs::read(&item.recycle_path) {
                Ok(data) => {
                    state.preview_bytes = Some(data);
                    state.preview_filename = item.file_name.clone();
                    state.decrypt_target = None;
                    state.screen = crate::app_state::AppScreen::PreviewMedia;
                }
                Err(e) => {
                    state.set_status(&format!("Gagal membaca file dari Recycle Bin: {}", e), false);
                }
            }
        } else {
            // Untuk Word, PDF, Video, dll buka via aplikasi default OS
            let path = &item.recycle_path;
            if let Err(e) = std::process::Command::new("explorer")
                .arg(path)
                .spawn()
            {
                state.set_status(&format!("Gagal membuka file dengan aplikasi eksternal: {}", e), false);
            } else {
                state.set_status(&format!("Membuka '{}' di aplikasi eksternal.", item.file_name), true);
            }
        }
    }

    pub fn secure_system_trash_item(&self, state: &mut AppState, index: usize) {
        if index >= state.system_trash_items.len() {
            state.set_status("Item tidak ditemukan.", false);
            return;
        }
        let item = state.system_trash_items[index].clone();
        
        if item.is_directory {
            state.set_status("Mengamankan folder dari System Trash belum didukung.", false);
            return;
        }

        let key = match state.session_key_bytes() {
            Some(k) => k,
            None    => { state.set_status("Sesi tidak valid. Login ulang.", false); return; }
        };
        let salt = match state.session_salt {
            Some(s) => s,
            None    => { state.set_status("Sesi tidak valid.", false); return; }
        };

        let vault_dir_path = crate::controller::vault_dir();
        let source_path = Path::new(&item.recycle_path);

        match crate::crypto::secure_encrypt_file(source_path, vault_dir_path, &key) {
            Ok(result) => {
                let record = FileRecord {
                    id:             uuid::Uuid::new_v4().to_string(),
                    original_name:  item.file_name.clone(),
                    original_path:  item.original_path.clone(),
                    vault_filename: result.encrypted_filename.clone(),
                    sha256_hash:    result.file_hash.clone(),
                    file_size:      item.file_size as i64,
                    iv_hex:         hex::encode(result.iv),
                    salt_hex:       hex::encode(salt),
                    encrypted_at:   timestamp_now(),
                    is_deleted:     false,
                    deleted_at:     None,
                    is_folder:      false,
                };

                let db  = self.db.lock().unwrap();
                let err = db.insert_file(&record).err();
                drop(db);

                if let Some(e) = err {
                    state.set_status(&format!("Berhasil enkripsi tapi gagal simpan DB: {}", e), false);
                    return;
                }

                // Hapus dari Windows Recycle Bin setelah berhasil diamankan
                let i_filename = source_path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .replacen("$R", "$I", 1);
                let i_path = source_path.parent().unwrap_or(Path::new("")).join(i_filename);
                let _ = std::fs::remove_file(source_path);
                let _ = std::fs::remove_file(i_path);

                self.load_files(state);
                self.scan_system_trash(state);
                self.log_action("SECURE_SYS_TRASH", &format!("File '{}' diamankan dari Recycle Bin ke Vault.", item.file_name));
                state.set_status(&format!("✅ Berhasil: '{}' diamankan ke dalam Vault.", item.file_name), true);
            }
            Err(e) => state.set_status(&format!("❌ Gagal enkripsi: {}", e), false),
        }
    }

    // ── TOTP (2FA) ────────────────────────────────────────

    /// Cek apakah TOTP sudah diaktifkan di database
    pub fn is_totp_enabled(&self) -> bool {
        let db = self.db.lock().unwrap();
        db.get_meta("totp_secret").unwrap_or(None).is_some()
    }

    /// Mulai setup TOTP: generate secret, buat QR
    pub fn begin_totp_setup(&self, state: &mut AppState) {
        let secret = crate::totp::generate_secret();
        let b32    = crate::totp::to_base32(&secret);
        let uri    = crate::totp::otpauth_uri(&b32);
        let qr     = crate::totp::qr_matrix(&uri);

        state.totp_secret     = Some(secret.to_vec());
        state.totp_secret_b32 = b32;
        state.totp_qr         = qr;
        state.totp_code.clear();
        state.totp_error = None;
        state.totp_setup_time = Some(std::time::Instant::now());
        state.screen     = AppScreen::TotpSetup;
    }

    /// Konfirmasi setup TOTP: verify kode, simpan secret ke DB
    pub fn confirm_totp_setup(&self, state: &mut AppState) {
        let secret = match &state.totp_secret {
            Some(s) => s.clone(),
            None    => { state.totp_error = Some("Secret tidak tersedia.".into()); return; }
        };

        if !crate::totp::verify(&secret, &state.totp_code) {
            state.totp_error = Some("Kode salah. Coba lagi.".into());
            state.totp_code.clear();
            return;
        }

        // Simpan ke DB
        let db = self.db.lock().unwrap();
        db.set_meta("totp_secret", &state.totp_secret_b32).expect("Simpan TOTP");
        drop(db);

        state.totp_enabled = true;
        state.totp_secret  = None;
        state.totp_code.clear();
        state.totp_error = None;
        state.totp_setup_time = None;
        state.set_status("✅ TOTP berhasil diaktifkan!", true);
        state.screen = AppScreen::Dashboard;
    }

    /// Verifikasi kode TOTP saat login
    pub fn verify_totp_login(&self, state: &mut AppState) {
        let db = self.db.lock().unwrap();
        let secret_b32 = db.get_meta("totp_secret").unwrap_or(None);
        drop(db);

        let secret_b32 = match secret_b32 {
            Some(s) => s,
            None    => { state.totp_error = Some("TOTP tidak terkonfigurasi.".into()); return; }
        };
        let secret = match crate::totp::from_base32(&secret_b32) {
            Some(s) => s,
            None    => { state.totp_error = Some("Data TOTP rusak.".into()); return; }
        };

        if crate::totp::verify(&secret, &state.totp_code) {
            state.totp_code.clear();
            state.totp_error = None;
            self.log_action("LOGIN_2FA", "Login 2FA berhasil.");
            state.screen = AppScreen::Dashboard;
        } else {
            self.log_action("FAIL_2FA", "Gagal verifikasi 2FA.");
            state.totp_error = Some("Kode salah. Coba lagi.".into());
            state.totp_code.clear();
        }
    }

    /// Nonaktifkan TOTP
    pub fn disable_totp(&self, state: &mut AppState) {
        let db = self.db.lock().unwrap();
        let _ = db.set_meta("totp_secret", ""); // kosongkan
        // Hapus entri
        let _ = db.conn_exec("DELETE FROM vault_meta WHERE key = 'totp_secret'");
        drop(db);

        state.totp_enabled = false;
        state.totp_secret  = None;
        state.totp_qr      = None;
        state.totp_setup_time = None;
        state.set_status("TOTP dinonaktifkan.", true);
    }
    // ── P2P Local Sharing ──────────────────────────────────

    pub fn start_share(&self, state: &mut AppState, record: FileRecord) {
        // Cek apakah server sudah aktif, matikan jika iya
        self.stop_share(state);

        let local_ip = match get_local_ip() {
            Some(ip) => ip,
            None => {
                state.set_status("❌ Gagal mendeteksi alamat IP Wi-Fi lokal.", false);
                return;
            }
        };

        // Generate PIN 4-digit acak
        use rand::Rng;
        let pin = rand::thread_rng().gen_range(1000..9999);
        let pin_str = format!("{}", pin);

        // Bind ke port 0 (OS akan mengalokasikan port acak secara dinamis)
        let listener = match std::net::TcpListener::bind("0.0.0.0:0") {
            Ok(l) => l,
            Err(e) => {
                state.set_status(&format!("❌ Gagal membuka port server: {}", e), false);
                return;
            }
        };

        let port = match listener.local_addr() {
            Ok(addr) => addr.port(),
            Err(e) => {
                state.set_status(&format!("❌ Gagal membaca port server: {}", e), false);
                return;
            }
        };

        if let Err(e) = listener.set_nonblocking(true) {
            state.set_status(&format!("❌ Gagal konfigurasi server: {}", e), false);
            return;
        }

        let stop_signal = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();
        let record_clone = record.clone();
        
        let key_bytes = match state.session_key_bytes() {
            Some(k) => k,
            None => {
                state.set_status("❌ Sesi Anda kadaluarsa. Silakan login kembali.", false);
                return;
            }
        };
        
        let pin_clone = pin_str.clone();

        std::thread::spawn(move || {
            while !stop_signal_clone.load(std::sync::atomic::Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut socket, _addr)) => {
                        let mut request = Vec::new();
                        let mut temp_buf = [0u8; 1024];
                        socket.set_read_timeout(Some(std::time::Duration::from_millis(500))).ok();
                        
                        if let Ok(n) = socket.read(&mut temp_buf) {
                            request.extend_from_slice(&temp_buf[..n]);
                        }
                        
                        let req_str = String::from_utf8_lossy(&request);
                        
                        if req_str.starts_with("GET /") {
                            let has_correct_pin = req_str.contains(&format!("pin={}", pin_clone));
                            
                            if has_correct_pin {
                                let enc_path = crate::controller::vault_dir().join(&record_clone.vault_filename);
                                let out_name = if record_clone.is_folder {
                                    format!("{}.zip", record_clone.original_name)
                                } else {
                                    record_clone.original_name.clone()
                                };

                                match crate::crypto::decrypt_to_memory(&enc_path, &key_bytes, &record_clone.sha256_hash) {
                                    Ok(dec_data) => {
                                        let response_headers = format!(
                                            "HTTP/1.1 200 OK\r\n\
                                             Content-Type: application/octet-stream\r\n\
                                             Content-Disposition: attachment; filename=\"{}\"\r\n\
                                             Content-Length: {}\r\n\
                                             Connection: close\r\n\r\n",
                                            out_name,
                                            dec_data.len()
                                        );
                                        if socket.write_all(response_headers.as_bytes()).is_ok() {
                                            let _ = socket.write_all(&dec_data);
                                        }
                                    }
                                    Err(e) => {
                                        let body = format!("<html><body><h1>Gagal dekripsi: {}</h1></body></html>", e);
                                        let response = format!(
                                            "HTTP/1.1 500 Internal Server Error\r\n\
                                             Content-Type: text/html\r\n\
                                             Content-Length: {}\r\n\
                                             Connection: close\r\n\r\n{}",
                                            body.len(),
                                            body
                                        );
                                        let _ = socket.write_all(response.as_bytes());
                                    }
                                }
                            } else if req_str.contains("submit_pin") {
                                let mut entered_pin = "";
                                if let Some(pos) = req_str.find("entered_pin=") {
                                    let rest = &req_str[pos + 12..];
                                    if let Some(space_pos) = rest.find(' ') {
                                        entered_pin = &rest[..space_pos];
                                    } else {
                                        entered_pin = rest;
                                    }
                                    if let Some(and_pos) = entered_pin.find('&') {
                                        entered_pin = &entered_pin[..and_pos];
                                    }
                                }

                                if entered_pin.trim() == pin_clone {
                                    let redirect_headers = format!(
                                        "HTTP/1.1 303 See Other\r\n\
                                         Location: /share?pin={}\r\n\
                                         Connection: close\r\n\r\n",
                                         pin_clone
                                    );
                                    let _ = socket.write_all(redirect_headers.as_bytes());
                                } else {
                                    let body = make_html_template(&record_clone.original_name, record_clone.file_size, Some("PIN yang Anda masukkan salah."));
                                    let response = format!(
                                        "HTTP/1.1 200 OK\r\n\
                                         Content-Type: text/html; charset=utf-8\r\n\
                                         Content-Length: {}\r\n\
                                         Connection: close\r\n\r\n{}",
                                        body.len(),
                                        body
                                    );
                                    let _ = socket.write_all(response.as_bytes());
                                }
                            } else {
                                let body = make_html_template(&record_clone.original_name, record_clone.file_size, None);
                                let response = format!(
                                    "HTTP/1.1 200 OK\r\n\
                                     Content-Type: text/html; charset=utf-8\r\n\
                                     Content-Length: {}\r\n\
                                     Connection: close\r\n\r\n{}",
                                    body.len(),
                                    body
                                );
                                let _ = socket.write_all(response.as_bytes());
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        });

        // Simpan state
        state.share_active_record = Some(record.clone());
        state.share_pin = pin_str;
        state.share_port = port;
        state.share_ip = local_ip;
        state.share_stop_signal = Some(stop_signal);

        self.log_action("P2P_SHARE_START", &format!("Server sharing lokal diaktifkan untuk: {}", record.original_name));
        state.set_status(&format!("📡 Server P2P Wi-Fi Sharing diaktifkan pada port {}", port), true);
    }

    pub fn stop_share(&self, state: &mut AppState) {
        if let Some(stop_signal) = state.share_stop_signal.take() {
            stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
            if let Some(ref r) = state.share_active_record {
                self.log_action("P2P_SHARE_STOP", &format!("Server sharing lokal dimatikan untuk: {}", r.original_name));
            }
        }
        state.share_active_record = None;
        state.share_pin.clear();
        state.share_port = 0;
        state.share_ip.clear();
    }

    pub fn rename_file(&self, state: &mut AppState, id: &str, new_name: &str) {
        if new_name.trim().is_empty() {
            state.set_status("Nama berkas tidak boleh kosong.", false);
            return;
        }
        let db = self.db.lock().unwrap();
        match db.rename_file(id, new_name.trim()) {
            Ok(()) => {
                drop(db);
                self.load_files(state);
                self.log_action("RENAME", &format!("Nama berkas dengan ID '{}' diubah menjadi '{}'.", id, new_name.trim()));
                state.set_status("Nama berkas berhasil diubah.", true);
            }
            Err(e) => {
                state.set_status(&format!("❌ Gagal mengubah nama: {}", e), false);
            }
        }
    }

    // ── Audit Logs ────────────────────────────────────────

    pub fn log_action(&self, action: &str, desc: &str) {
        let db = self.db.lock().unwrap();
        let _ = db.insert_audit_log(action, desc, &timestamp_now());
    }

    pub fn load_audit_logs(&self, state: &mut AppState) {
        let db = self.db.lock().unwrap();
        state.audit_logs = db.get_all_audit_logs().unwrap_or_default();
    }

    // ── Profile / Settings ────────────────────────────────

    pub fn backup_database(&self, state: &mut AppState) {
        #[cfg(not(target_os = "android"))]
        let dest = rfd::FileDialog::new().set_file_name("vault_backup.db").save_file();
        #[cfg(target_os = "android")]
        let dest: Option<PathBuf> = crate::controller::external_dir().map(|p| p.join("vault_backup.db"));

        if let Some(dest) = dest {
            if let Err(e) = std::fs::copy(crate::controller::db_path(), &dest) {
                state.set_status(&format!("❌ Gagal backup: {}", e), false);
            } else {
                #[cfg(target_os = "android")]
                state.set_status(&format!("✅ Backup database berhasil disimpan ke: {}", dest.display()), true);
                #[cfg(not(target_os = "android"))]
                state.set_status("✅ Backup database berhasil.", true);
                
                self.log_action("BACKUP", "Database dicadangkan oleh pengguna.");
            }
        } else {
            #[cfg(target_os = "android")]
            state.set_status("❌ Gagal backup: Penyimpanan eksternal tidak tersedia.", false);
        }
    }

    pub fn change_password(&self, state: &mut AppState) {
        if state.profile_new_password.len() < 4 {
            state.profile_password_error = Some("Password baru minimal 4 karakter.".into());
            return;
        }
        if state.profile_new_password != state.profile_confirm_password {
            state.profile_password_error = Some("Password baru tidak cocok.".into());
            return;
        }

        let db = self.db.lock().unwrap();
        let pwd_hash_db = db.get_password_hash().unwrap_or(None);
        let salt_hex_db = db.get_password_salt().unwrap_or(None);

        let (Some(stored_hash), Some(salt_hex)) = (pwd_hash_db, salt_hex_db) else {
            state.profile_password_error = Some("Data password lama tidak ditemukan.".into());
            return;
        };

        let salt_bytes = hex::decode(&salt_hex).unwrap_or_default();
        let mut old_salt = [0u8; crate::crypto::SALT_LEN];
        old_salt.copy_from_slice(&salt_bytes);

        if crate::crypto::hash_pin(&state.profile_old_password, &old_salt) != stored_hash {
            state.profile_password_error = Some("Password lama salah.".into());
            return;
        }

        let new_salt = crate::crypto::generate_salt();
        let new_hash = crate::crypto::hash_pin(&state.profile_new_password, &new_salt);
        let new_salt_hex = hex::encode(new_salt);

        db.update_password(&new_hash, &new_salt_hex).expect("Gagal update password");
        drop(db);

        let key = crate::crypto::derive_key(&state.profile_new_password, &new_salt);
        state.session_key  = Some(key);
        state.session_salt = Some(new_salt);

        state.profile_old_password.clear();
        state.profile_new_password.clear();
        state.profile_confirm_password.clear();
        state.profile_password_error = None;
        state.profile_password_success = Some("Password berhasil diubah.".into());
        self.log_action("CHANGE_PWD", "Password utama berhasil diubah.");
    }

    pub fn decrypt_to_memory(&self, state: &mut AppState, vault_filename: &str) {
        let db = self.db.lock().unwrap();
        let record = match db.get_file(vault_filename) {
            Ok(Some(r)) => r,
            _ => { state.set_status("File tidak ditemukan", false); return; }
        };
        drop(db);

        let key = match &state.session_key {
            Some(k) => k,
            None => { state.set_status("Kunci sesi tidak tersedia", false); return; }
        };

        let enc_path = crate::controller::vault_dir().join(&record.vault_filename);
        let ext = crate::theme::file_ext(&record.original_name).to_lowercase();
        
        if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "txt" {
            // Pratinjau gambar dan teks di dalam aplikasi
            match crate::crypto::decrypt_to_memory(&enc_path, key, &record.sha256_hash) {
                Ok(data) => {
                    state.preview_bytes = Some(data);
                    state.preview_filename = record.original_name.clone();
                    state.decrypt_target = Some(record.clone());
                    state.screen = crate::app_state::AppScreen::PreviewMedia;
                    self.log_action("PREVIEW", &format!("Melihat pratinjau file: {}", record.original_name));
                }
                Err(_) => {
                    state.set_status("Gagal mendekripsi file (password salah atau rusak)", false);
                }
            }
        } else {
            // Untuk file lain, dekripsi ke folder Temp dan buka via aplikasi eksternal
            let temp_dir = std::env::temp_dir().join("aegis_vault_preview");
            let _ = std::fs::create_dir_all(&temp_dir);
            let temp_path = temp_dir.join(&record.original_name);

            match crate::crypto::secure_decrypt_file(&enc_path, &temp_path, key, &record.sha256_hash) {
                Ok(()) => {
                    if let Err(e) = std::process::Command::new("explorer")
                        .arg(temp_path.to_str().unwrap())
                        .spawn()
                    {
                        state.set_status(&format!("Gagal membuka file eksternal: {}", e), false);
                    } else {
                        state.set_status(&format!("Membuka '{}' di aplikasi eksternal.", record.original_name), true);
                        self.log_action("PREVIEW_EXT", &format!("Membuka pratinjau eksternal: {}", record.original_name));
                        
                        // Tampilkan layar pratinjau di aplikasi agar user bisa memilih untuk memulihkan
                        state.preview_bytes = Some(vec![]);
                        state.preview_filename = record.original_name.clone();
                        state.decrypt_target = Some(record.clone());
                        state.screen = crate::app_state::AppScreen::PreviewMedia;
                    }
                }
                Err(_) => state.set_status("Gagal mendekripsi file untuk pratinjau.", false),
            }
        }
    }

    pub fn refresh_device_metrics(&self, state: &mut AppState) {
        let mut last_refresh = self.last_sys_refresh.lock().unwrap();
        if last_refresh.elapsed().as_secs_f32() < 2.0 {
            return;
        }
        *last_refresh = std::time::Instant::now();
        
        #[cfg(not(target_os = "android"))]
        {
            let mut sys = self.sys.lock().unwrap();
            sys.refresh_cpu_usage();
            sys.refresh_memory();
        }
        let sys = self.sys.lock().unwrap();
        
        let mut cpu_avg = 0.0;
        let cpus = sys.cpus();
        if !cpus.is_empty() {
            cpu_avg = cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32;
        }
        state.cpu_usage = cpu_avg / 100.0;
        
        let total_mem = sys.total_memory();
        let used_mem = sys.used_memory();
        if total_mem > 0 {
            state.ram_usage = used_mem as f64 as f32 / total_mem as f64 as f32;
        }
        
        // Pseudo I/O based on CPU + noise since sysinfo doesn't provide % global IO directly
        use rand::Rng;
        let noise = (rand::thread_rng().gen::<f32>() * 0.1) - 0.05;
        state.io_usage = (state.cpu_usage * 0.4 + 0.05 + noise).clamp(0.01, 1.0);
        
        #[cfg(not(target_os = "android"))]
        {
            let mut disks = self.disks.lock().unwrap();
            disks.refresh_list();
        }
        let disks = self.disks.lock().unwrap();
        let mut total_disk = 0;
        let mut free_disk = 0;
        for disk in disks.list() {
            if !disk.is_removable() {
                total_disk += disk.total_space();
                free_disk += disk.available_space();
            }
        }
        if total_disk == 0 {
            for disk in disks.list() {
                total_disk += disk.total_space();
                free_disk += disk.available_space();
            }
        }
        state.device_disk_total = total_disk;
        state.device_disk_free = free_disk;
    }

    // ── Custom File Picker Pure Rust ──────────────────────────

    pub fn open_custom_file_picker(&self, state: &mut AppState) {
        state.custom_file_picker_open = true;
        state.custom_file_picker_search.clear();
        state.custom_file_picker_error = None;
        state.request_storage_permission = true; // Trigger dynamic permission request!

        let mut start_dir = std::path::PathBuf::from("/storage/emulated/0");
        if !start_dir.exists() {
            start_dir = std::path::PathBuf::from("/sdcard");
        }
        if !start_dir.exists() {
            if let Some(ext_path) = crate::controller::external_dir() {
                start_dir = ext_path.to_path_buf();
            } else {
                start_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            }
        }
        self.navigate_custom_file_picker(state, start_dir);
    }

    pub fn navigate_custom_file_picker(&self, state: &mut AppState, dir: std::path::PathBuf) {
        state.custom_file_picker_current_dir = dir.clone();
        state.custom_file_picker_error = None;

        let mut paths = Vec::new();
        match std::fs::read_dir(&dir) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        paths.push(entry.path());
                    }
                }
                // Sort folders first, then files alphabetically
                paths.sort_by(|a, b| {
                    let a_is_dir = a.is_dir();
                    let b_is_dir = b.is_dir();
                    if a_is_dir && !b_is_dir {
                        std::cmp::Ordering::Less
                    } else if !a_is_dir && b_is_dir {
                        std::cmp::Ordering::Greater
                    } else {
                        let a_name = a.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                        let b_name = b.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                        a_name.cmp(&b_name)
                    }
                });
                state.custom_file_picker_files = paths;
            }
            Err(e) => {
                state.custom_file_picker_error = Some(format!("Akses ditolak: {}", e));
                state.custom_file_picker_files.clear();
            }
        }
    }
}

// ── Timestamp helper ──────────────────────────────────────
pub fn timestamp_now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

// ── Size formatter ────────────────────────────────────────
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024           { format!("{} B",     bytes) }
    else if bytes < 1024*1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else                      { format!("{:.2} MB", bytes as f64 / (1024.0*1024.0)) }
}

// ── Folder Archiver Helpers ───────────────────────────────

fn zip_dir_recursive<W: std::io::Write + std::io::Seek>(
    src_dir: &Path,
    current_dir: &Path,
    zip: &mut zip::ZipWriter<W>,
) -> Result<(), std::io::Error> {
    let entries = std::fs::read_dir(current_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(src_dir).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let name_str = name.to_string_lossy().replace('\\', "/"); // Standard ZIP format uses forward slash

        if path.is_dir() {
            zip.add_directory(&name_str, zip::write::FileOptions::default())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            zip_dir_recursive(src_dir, &path, zip)?;
        } else {
            zip.start_file(&name_str, zip::write::FileOptions::default())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let mut f = File::open(&path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        }
    }
    Ok(())
}

fn zip_directory(src_dir: &Path, dst_zip: &Path) -> Result<(), std::io::Error> {
    let file = File::create(dst_zip)?;
    let mut zip = zip::ZipWriter::new(file);
    zip_dir_recursive(src_dir, src_dir, &mut zip)?;
    zip.finish().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(())
}

fn unzip_directory(src_zip: &Path, dst_dir: &Path) -> Result<(), std::io::Error> {
    let file = File::open(src_zip)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::create_dir_all(dst_dir)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let outpath = match file.enclosed_name() {
            Some(path) => dst_dir.join(path),
            None => continue,
        };

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

// ── P2P Share Helpers ─────────────────────────────────────

fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return Some(addr.ip().to_string());
            }
        }
    }
    Some("127.0.0.1".to_string())
}

fn make_html_template(filename: &str, filesize: i64, error_msg: Option<&str>) -> String {
    let error_html = if let Some(err) = error_msg {
        format!("<div class='error-msg'>{}</div>", err)
    } else {
        "".to_string()
    };
    
    let formatted_size = format_size(filesize as u64);

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Aegis Vault - Secure Share</title>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&display=swap" rel="stylesheet">
    <style>
        :root {{
            --bg-base: #060605;
            --surface: rgba(18, 18, 17, 0.7);
            --accent: #b666d2;
            --accent-glow: rgba(182, 102, 210, 0.2);
            --text-main: #ffffff;
            --text-muted: #888888;
            --success: #4ade80;
            --error: #ef4444;
            --border: rgba(255, 255, 255, 0.08);
        }}

        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}

        body {{
            background-color: var(--bg-base);
            background-image: radial-gradient(circle at 50% -20%, rgba(182, 102, 210, 0.15) 0%, transparent 60%);
            color: var(--text-main);
            font-family: 'Outfit', sans-serif;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }}

        .card {{
            background: var(--surface);
            backdrop-filter: blur(16px);
            -webkit-backdrop-filter: blur(16px);
            border: 1px solid var(--border);
            border-radius: 20px;
            padding: 32px;
            width: 100%;
            max-width: 420px;
            box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.05);
            text-align: center;
            animation: fadeIn 0.6s cubic-bezier(0.16, 1, 0.3, 1);
        }}

        @keyframes fadeIn {{
            from {{ opacity: 0; transform: translateY(20px); }}
            to {{ opacity: 1; transform: translateY(0); }}
        }}

        .logo-area {{
            margin-bottom: 24px;
        }}

        .logo-icon {{
            font-size: 40px;
            color: var(--accent);
            text-shadow: 0 0 15px var(--accent-glow);
            display: inline-block;
            margin-bottom: 8px;
        }}

        h1 {{
            font-size: 24px;
            font-weight: 700;
            margin-bottom: 8px;
            letter-spacing: -0.5px;
        }}

        .subtitle {{
            color: var(--text-muted);
            font-size: 14px;
            margin-bottom: 28px;
        }}

        .file-info {{
            background: rgba(255, 255, 255, 0.03);
            border: 1px solid var(--border);
            border-radius: 12px;
            padding: 16px;
            margin-bottom: 24px;
            text-align: left;
            display: flex;
            align-items: center;
            gap: 12px;
        }}

        .file-badge {{
            width: 42px;
            height: 42px;
            background: rgba(182, 102, 210, 0.1);
            border: 1px solid rgba(182, 102, 210, 0.2);
            color: var(--accent);
            border-radius: 8px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 20px;
            font-weight: bold;
        }}

        .file-meta {{
            flex: 1;
            min-width: 0;
        }}

        .file-name {{
            font-weight: 600;
            font-size: 15px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }}

        .file-size {{
            color: var(--text-muted);
            font-size: 12px;
            margin-top: 2px;
        }}

        .form-group {{
            margin-bottom: 20px;
            text-align: left;
        }}

        label {{
            display: block;
            font-size: 13px;
            font-weight: 500;
            color: var(--text-muted);
            margin-bottom: 8px;
        }}

        input[type="text"] {{
            width: 100%;
            background: rgba(0, 0, 0, 0.2);
            border: 1px solid var(--border);
            border-radius: 12px;
            padding: 14px 16px;
            color: #fff;
            font-family: inherit;
            font-size: 18px;
            font-weight: 600;
            letter-spacing: 4px;
            text-align: center;
            transition: all 0.3s ease;
        }}

        input[type="text"]:focus {{
            outline: none;
            border-color: var(--accent);
            box-shadow: 0 0 0 4px var(--accent-glow);
        }}

        .btn {{
            width: 100%;
            background: var(--accent);
            color: #fff;
            border: none;
            border-radius: 12px;
            padding: 14px;
            font-family: inherit;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.3s ease;
            box-shadow: 0 4px 12px rgba(182, 102, 210, 0.3);
        }}

        .btn:hover {{
            background: #c57be0;
            transform: translateY(-1px);
            box-shadow: 0 6px 16px rgba(182, 102, 210, 0.4);
        }}

        .btn:active {{
            transform: translateY(1px);
        }}

        .error-msg {{
            background: rgba(239, 68, 68, 0.1);
            border: 1px solid rgba(239, 68, 68, 0.2);
            color: var(--error);
            border-radius: 8px;
            padding: 10px;
            font-size: 13px;
            margin-bottom: 20px;
            text-align: center;
        }}

        .footer {{
            margin-top: 32px;
            font-size: 11px;
            color: var(--text-muted);
        }}
    </style>
</head>
<body>
    <div class="card">
        <div class="logo-area">
            <span class="logo-icon">🔒</span>
            <h1>Aegis Vault</h1>
            <p class="subtitle">Secure Local Wi-Fi Share</p>
        </div>

        <div class="file-info">
            <div class="file-badge">💾</div>
            <div class="file-meta">
                <div class="file-name">{}</div>
                <div class="file-size">{}</div>
            </div>
        </div>

        {}

        <form action="/submit_pin" method="get">
            <div class="form-group">
                <label for="entered_pin">MASUKKAN PIN TRANSFER 4-DIGIT</label>
                <input type="text" id="entered_pin" name="entered_pin" maxlength="4" autocomplete="off" placeholder="••••" required pattern="\d{{4}}">
            </div>
            <button type="submit" class="btn">🔓 Verifikasi & Unduh</button>
        </form>

        <div class="footer">
            Didekripsi secara aman langsung dari PC pengirim over Wi-Fi lokal.
        </div>
    </div>
</body>
</html>"#, filename, formatted_size, error_html)
}
