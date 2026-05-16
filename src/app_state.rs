// app_state.rs — Model layer
// Menyimpan seluruh state aplikasi. Tidak ada dependency ke egui.
// Controller membaca dan memodifikasi state ini.
// View hanya membaca state untuk render.

use crate::crypto::{KEY_LEN, SALT_LEN};
use crate::db::FileRecord;

// ── Screen enum ───────────────────────────────────────────
#[derive(Default, PartialEq, Clone)]
pub enum AppScreen {
    #[default]
    Login,
    SetupPin,
    Dashboard,
    Decrypting(String), // vault_filename target
    TotpSetup,          // setup QR + verifikasi awal
    TotpVerify,         // verifikasi 2FA saat login
    RecycleBin,         // fitur trash
}

#[derive(Default, PartialEq, Clone)]
pub enum DashboardTab {
    #[default]
    Home,
    Vault,
    Storage,
    Settings,
    Profile,
    Notifications,
}

// ── Status message ────────────────────────────────────────
#[derive(Clone)]
pub struct StatusMsg {
    pub text:    String,
    pub success: bool,
}

// ── AppState (Model) ──────────────────────────────────────
pub struct AppState {
    // Navigasi
    pub screen: AppScreen,
    pub dashboard_tab: DashboardTab,

    // Auth input
    pub pin_digits:      String,   // numpad accumulator (max 6)
    pub pin_input:       String,   // setup field 1
    pub pin_confirm:     String,   // setup field 2
    pub pin_error:       Option<String>,
    pub pin_shake_timer: f32,      // countdown animasi shake

    // Sesi aktif
    pub session_key:  Option<Box<[u8; KEY_LEN]>>,
    pub session_salt: Option<[u8; SALT_LEN]>,

    // Data file
    pub file_list: Vec<FileRecord>,
    pub deleted_list: Vec<FileRecord>,

    // Status bar & Toast
    pub status: Option<StatusMsg>,
    pub toast_message: Option<String>,
    pub toast_timer: f32,

    // Dekripsi
    pub decrypt_target:   Option<FileRecord>,
    pub decrypt_out_name: String,

    // TOTP (2FA)
    pub totp_enabled:    bool,
    pub totp_secret:     Option<Vec<u8>>,      // raw bytes (saat setup)
    pub totp_secret_b32: String,               // base32 (untuk display)
    pub totp_qr:         Option<Vec<Vec<bool>>>, // QR matrix
    pub totp_code:       String,               // input 6-digit
    pub totp_error:      Option<String>,
    pub totp_setup_time: Option<std::time::Instant>,

    // Reset Vault
    pub show_reset_confirm: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            screen:           AppScreen::Login,
            dashboard_tab:    DashboardTab::Home,
            pin_digits:       String::new(),
            pin_input:        String::new(),
            pin_confirm:      String::new(),
            pin_error:        None,
            pin_shake_timer:  0.0,
            session_key:      None,
            session_salt:     None,
            file_list:        Vec::new(),
            deleted_list:     Vec::new(),
            status:           None,
            toast_message:    None,
            toast_timer:      0.0,
            decrypt_target:   None,
            decrypt_out_name: String::new(),
            totp_enabled:     false,
            totp_secret:      None,
            totp_secret_b32:  String::new(),
            totp_qr:          None,
            totp_code:        String::new(),
            totp_error:       None,
            totp_setup_time:  None,
            show_reset_confirm: false,
        }
    }
}

impl AppState {
    pub fn set_status(&mut self, text: &str, success: bool) {
        self.status = Some(StatusMsg { text: text.to_string(), success });
        // Set juga untuk toast notification
        self.toast_message = Some(text.to_string());
        self.toast_timer = 3.0; // Tampil selama 3 detik
    }

    #[allow(dead_code)]
    pub fn clear_status(&mut self) {
        self.status = None;
        self.toast_message = None;
    }

    #[allow(dead_code)]
    pub fn is_authenticated(&self) -> bool {
        self.session_key.is_some()
    }

    pub fn total_vault_size(&self) -> u64 {
        self.file_list.iter().map(|r| r.file_size as u64).sum()
    }

    pub fn session_key_bytes(&self) -> Option<[u8; KEY_LEN]> {
        self.session_key.as_ref().map(|k| {
            let mut arr = [0u8; KEY_LEN];
            arr.copy_from_slice(k.as_ref());
            arr
        })
    }
}
