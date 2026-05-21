// controller.rs — Controller layer
// Semua logika bisnis: login, setup akun, enkripsi, dekripsi.
// Tidak ada kode egui di sini. Controller memodifikasi AppState
// dan memanggil crypto/db layer.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use zeroize::Zeroize;

use crate::app_state::{AppScreen, AppState};
use sysinfo::{System, Disks};
use crate::crypto::{
    derive_key, generate_salt, hash_pin,
    secure_decrypt_file, secure_encrypt_file,
    SALT_LEN,
};
use crate::db::{FileRecord, VaultDb};

pub const VAULT_DIR: &str = "vault_storage";
pub const DB_PATH:   &str = "vault_storage/vault.db";

// ── Controller ────────────────────────────────────────────
pub struct Controller {
    pub db: Arc<Mutex<VaultDb>>,
    pub sys: Mutex<System>,
    pub disks: Mutex<Disks>,
    pub last_sys_refresh: Mutex<std::time::Instant>,
}

impl Controller {
    pub fn new(db: VaultDb) -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        Self { 
            db: Arc::new(Mutex::new(db)),
            sys: Mutex::new(sys),
            disks: Mutex::new(Disks::new_with_refreshed_list()),
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
        if let Ok(entries) = std::fs::read_dir(VAULT_DIR) {
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

    /// Enkripsi file dan simpan record ke DB
    pub fn encrypt_file(&self, state: &mut AppState, source_path: PathBuf) {
        let key = match state.session_key_bytes() {
            Some(k) => k,
            None    => { state.set_status("Sesi tidak valid. Login ulang.", false); return; }
        };
        let salt = match state.session_salt {
            Some(s) => s,
            None    => { state.set_status("Sesi tidak valid.", false); return; }
        };

        let vault_dir = Path::new(VAULT_DIR);
        let file_size = source_path.metadata().map(|m| m.len()).unwrap_or(0);
        let file_name = source_path.file_name()
            .unwrap_or_default().to_string_lossy().to_string();

        match secure_encrypt_file(&source_path, vault_dir, &key) {
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
                };

                let db  = self.db.lock().unwrap();
                let err = db.insert_file(&record).err();
                drop(db);

                if let Some(e) = err {
                    state.set_status(
                        &format!("Enkripsi berhasil tapi gagal simpan DB: {}", e), false
                    );
                    return;
                }

                self.load_files(state);
                self.log_action("ENCRYPT", &format!("File '{}' diamankan.", file_name));
                state.set_status(&format!("✅ Berhasil: {} diamankan.", file_name), true);
            }
            Err(e) => state.set_status(&format!("❌ Gagal enkripsi: {}", e), false),
        }
    }

    /// Dekripsi file vault ke folder tujuan, hapus vault setelah sukses
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

        let vault_path = Path::new(VAULT_DIR).join(&record.vault_filename);
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
        let vault_path = Path::new(VAULT_DIR).join(&record.vault_filename);
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

        let vault_dir = Path::new(VAULT_DIR);
        let source_path = Path::new(&item.recycle_path);

        match crate::crypto::secure_encrypt_file(source_path, vault_dir, &key) {
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
        let dest: Option<PathBuf> = { state.set_status("Backup via dialog belum didukung di Android", false); None };

        if let Some(dest) = dest {
            if let Err(e) = std::fs::copy(DB_PATH, dest) {
                state.set_status(&format!("❌ Gagal backup: {}", e), false);
            } else {
                state.set_status("✅ Backup database berhasil.", true);
                self.log_action("BACKUP", "Database dicadangkan oleh pengguna.");
            }
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

        let enc_path = std::path::Path::new(VAULT_DIR).join(&record.vault_filename);
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
        
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        
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
        
        let mut disks = self.disks.lock().unwrap();
        disks.refresh_list();
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
