// controller.rs — Controller layer
// Semua logika bisnis: login, setup PIN, enkripsi, dekripsi.
// Tidak ada kode egui di sini. Controller memodifikasi AppState
// dan memanggil crypto/db layer.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use zeroize::Zeroize;

use crate::app_state::{AppScreen, AppState};
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
}

impl Controller {
    pub fn new(db: VaultDb) -> Self {
        Self { db: Arc::new(Mutex::new(db)) }
    }

    // ── Auth ──────────────────────────────────────────────

    /// Cek apakah PIN sudah pernah di-setup
    pub fn is_pin_set(&self) -> bool {
        self.db.lock().unwrap().is_pin_set()
    }

    /// Login via numpad — returns true jika berhasil
    pub fn try_login(&self, state: &mut AppState) -> bool {
        let db          = self.db.lock().unwrap();
        let pin_hash_db = db.get_pin_hash().unwrap_or(None);
        let salt_hex_db = db.get_pin_salt().unwrap_or(None);
        drop(db);

        let (Some(stored_hash), Some(salt_hex)) = (pin_hash_db, salt_hex_db) else {
            state.pin_error = Some("Data PIN tidak ditemukan.".into());
            return false;
        };

        let salt_bytes = hex::decode(&salt_hex).unwrap_or_default();
        if salt_bytes.len() != SALT_LEN {
            state.pin_error = Some("Data vault rusak.".into());
            return false;
        }

        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&salt_bytes);

        let computed = hash_pin(&state.pin_input, &salt);
        if computed == stored_hash {
            let key           = derive_key(&state.pin_input, &salt);
            state.session_key  = Some(key);
            state.session_salt = Some(salt);
            state.pin_input.zeroize();
            state.pin_error = None;
            self.load_files(state);
            // Cek apakah TOTP aktif → arahkan ke verifikasi 2FA
            state.totp_enabled = self.is_totp_enabled();
            if state.totp_enabled {
                state.totp_code.clear();
                state.totp_error = None;
                state.screen = AppScreen::TotpVerify;
                self.log_action("LOGIN", "Berhasil login ke dalam vault via PIN.");
                state.screen = AppScreen::Dashboard;
            }
            true
        } else {
            self.log_action("FAIL_LOGIN", "Gagal mencoba login. PIN salah.");
            state.pin_error = Some("PIN salah. Coba lagi.".into());
            state.pin_input.zeroize();
            false
        }
    }

    /// Setup PIN baru untuk vault kosong
    pub fn setup_pin(&self, state: &mut AppState) {
        if state.pin_input.len() != 6 {
            state.pin_error = Some("PIN harus tepat 6 digit.".into());
            return;
        }
        if !state.pin_input.chars().all(|c| c.is_ascii_digit()) {
            state.pin_error = Some("PIN hanya boleh angka.".into());
            return;
        }
        if state.pin_input != state.pin_confirm {
            state.pin_error = Some("PIN tidak cocok.".into());
            state.pin_confirm.zeroize();
            return;
        }

        let salt     = generate_salt();
        let pin_hash = hash_pin(&state.pin_input, &salt);
        let salt_hex = hex::encode(salt);

        {
            let db = self.db.lock().unwrap();
            db.set_pin(&pin_hash, &salt_hex).expect("Gagal simpan PIN");
        }

        let key            = derive_key(&state.pin_input, &salt);
        state.session_key  = Some(key);
        state.session_salt = Some(salt);
        state.pin_input.zeroize();
        state.pin_confirm.zeroize();
        state.pin_error = None;
        self.load_files(state);
        self.log_action("SETUP", "PIN awal dikonfigurasi.");
        state.screen = AppScreen::Dashboard;
    }

    /// Logout: bersihkan sesi dari memori
    pub fn logout(&self, state: &mut AppState) {
        if let Some(mut k) = state.session_key.take() { k.zeroize(); }
        state.session_salt   = None;
        state.pin_digits     = String::new();
        state.pin_error      = None;
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
        state.pin_input = String::new();
        state.pin_confirm = String::new();
        state.screen = AppScreen::SetupPin;
        state.set_status("Vault telah di-reset. Silakan buat PIN baru.", true);
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
                { let db = self.db.lock().unwrap(); let _ = db.permanent_delete_file(&record.id); }
                let _ = std::fs::remove_file(&vault_path);
                self.load_files(state);
                self.log_action("DECRYPT", &format!("File dipulihkan: {}", record.original_name));
                state.set_status(
                    &format!("✅ File dipulihkan ke: {}", out_path.display()), true
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
        if let Some(dest) = rfd::FileDialog::new().set_file_name("vault_backup.db").save_file() {
            if let Err(e) = std::fs::copy(DB_PATH, dest) {
                state.set_status(&format!("❌ Gagal backup: {}", e), false);
            } else {
                state.set_status("✅ Backup database berhasil.", true);
                self.log_action("BACKUP", "Database dicadangkan oleh pengguna.");
            }
        }
    }

    pub fn change_pin(&self, state: &mut AppState) {
        if state.profile_new_pin.len() != 6 {
            state.profile_pin_error = Some("PIN baru harus 6 digit.".into());
            return;
        }
        if state.profile_new_pin != state.profile_confirm_pin {
            state.profile_pin_error = Some("PIN baru tidak cocok.".into());
            return;
        }

        let db = self.db.lock().unwrap();
        let pin_hash_db = db.get_pin_hash().unwrap_or(None);
        let salt_hex_db = db.get_pin_salt().unwrap_or(None);

        let (Some(stored_hash), Some(salt_hex)) = (pin_hash_db, salt_hex_db) else {
            state.profile_pin_error = Some("Data PIN lama tidak ditemukan.".into());
            return;
        };

        let salt_bytes = hex::decode(&salt_hex).unwrap_or_default();
        let mut old_salt = [0u8; crate::crypto::SALT_LEN];
        old_salt.copy_from_slice(&salt_bytes);

        if crate::crypto::hash_pin(&state.profile_old_pin, &old_salt) != stored_hash {
            state.profile_pin_error = Some("PIN lama salah.".into());
            return;
        }

        let new_salt = crate::crypto::generate_salt();
        let new_hash = crate::crypto::hash_pin(&state.profile_new_pin, &new_salt);
        let new_salt_hex = hex::encode(new_salt);

        db.set_pin(&new_hash, &new_salt_hex).expect("Gagal update PIN");
        drop(db);

        let key = crate::crypto::derive_key(&state.profile_new_pin, &new_salt);
        state.session_key  = Some(key);
        state.session_salt = Some(new_salt);

        state.profile_old_pin.clear();
        state.profile_new_pin.clear();
        state.profile_confirm_pin.clear();
        state.profile_pin_error = None;
        state.profile_pin_success = Some("PIN berhasil diubah.".into());
        self.log_action("CHANGE_PIN", "PIN utama berhasil diubah.");
    }

    pub fn decrypt_to_memory(&self, state: &mut AppState, vault_filename: &str) {
        let db = self.db.lock().unwrap();
        let record = match db.get_file(vault_filename) {
            Ok(Some(r)) => r,
            _ => { state.set_status("File tidak ditemukan", false); return; }
        };

        let key = match &state.session_key {
            Some(k) => k,
            None => { state.set_status("Kunci sesi tidak tersedia", false); return; }
        };

        let enc_path = std::path::Path::new(VAULT_DIR).join(&record.vault_filename);
        match crate::crypto::decrypt_to_memory(&enc_path, key, &record.sha256_hash) {
            Ok(data) => {
                state.preview_bytes = Some(data);
                state.preview_filename = record.original_name.clone();
                state.screen = crate::app_state::AppScreen::PreviewMedia;
                let _ = db.insert_audit_log("PREVIEW", &format!("Melihat pratinjau file: {}", record.original_name), &timestamp_now());
            }
            Err(_) => {
                state.set_status("Gagal mendekripsi file (password salah atau rusak)", false);
            }
        }
    }
}

// ── Timestamp helper ──────────────────────────────────────
pub fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs  = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_secs();
    let mins  = secs / 60;
    let hours = mins / 60;
    let days  = hours / 24;
    let h     = hours % 24;
    let m     = mins % 60;
    let y     = 1970 + days / 365;
    let d     = (days % 365) + 1;
    format!("{}-{:03} {:02}:{:02}", y, d, h, m)
}

// ── Size formatter ────────────────────────────────────────
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024           { format!("{} B",     bytes) }
    else if bytes < 1024*1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else                      { format!("{:.2} MB", bytes as f64 / (1024.0*1024.0)) }
}
