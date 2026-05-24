// app_state.rs — Model layer
// Menyimpan seluruh state aplikasi. Tidak ada dependency ke egui.
// Controller membaca dan memodifikasi state ini.
// View hanya membaca state untuk render.

use crate::crypto::{KEY_LEN, SALT_LEN};
use crate::db::{FileRecord, AuditLog};

// ── Screen enum ───────────────────────────────────────────
#[derive(Default, PartialEq, Clone)]
pub enum AppScreen {
    #[default]
    Login,
    SetupAccount,
    Dashboard,
    Decrypting(String), // vault_filename target
    TotpSetup,          // setup QR + verifikasi awal
    TotpVerify,         // verifikasi 2FA saat login
    RecycleBin,         // fitur trash vault
    SystemTrash,        // fitur scan Recycle Bin Windows
    PreviewMedia,
}

#[derive(Default, PartialEq, Clone)]
pub enum FocusedField {
    #[default]
    None,
    LoginUsername,
    LoginPassword,
    SetupUsername,
    SetupDisplayName,
    SetupPassword,
    SetupConfirmPassword,
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

#[derive(PartialEq, Clone)]
pub enum ViewMode {
    List,
    Grid,
}

impl Default for ViewMode {
    fn default() -> Self { Self::List }
}

#[derive(PartialEq, Clone)]
pub enum SortOption {
    DateDesc,
    DateAsc,
    NameAsc,
    SizeDesc,
}

impl Default for SortOption {
    fn default() -> Self { Self::DateDesc }
}


// ── Status message ────────────────────────────────────────
#[derive(Clone)]
pub struct StatusMsg {
    pub text:    String,
    pub success: bool,
}

// ── System Recycle Bin Item ────────────────────────────────
#[derive(Debug, Clone)]
pub struct RecycleBinItem {
    pub original_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub deleted_at: String,
    pub recycle_path: String, // $R file path for restore
    pub is_directory: bool,
}

// ── AppState (Model) ──────────────────────────────────────
pub struct AppState {
    // Navigasi
    pub screen: AppScreen,
    pub dashboard_tab: DashboardTab,

    // Auth input — Login
    pub login_username:  String,
    pub login_password:  String,
    pub login_error:     Option<String>,

    // Auth input — Setup Account
    pub setup_username:         String,
    pub setup_display_name:     String,
    pub setup_password:         String,
    pub setup_password_confirm: String,
    pub setup_error:            Option<String>,

    // Session info
    pub display_name: String,  // Nama user yang sedang login

    // Sesi aktif (crypto)
    pub session_key:  Option<Box<[u8; KEY_LEN]>>,
    pub session_salt: Option<[u8; SALT_LEN]>,

    // Data file
    pub file_list: Vec<FileRecord>,
    pub deleted_list: Vec<FileRecord>,

    // System Recycle Bin
    pub system_trash_items: Vec<RecycleBinItem>,
    pub system_trash_loading: bool,

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

    // Vault Tab
    pub vault_search_query: String,
    pub vault_view_mode: ViewMode,
    pub vault_sort_by: SortOption,

    // Profile Tab — Change Password
    pub profile_old_password: String,
    pub profile_new_password: String,
    pub profile_confirm_password: String,
    pub profile_password_error: Option<String>,
    pub profile_password_success: Option<String>,

    // Notifications Tab
    pub audit_logs: Vec<AuditLog>,
    pub is_light_mode: bool,
    pub preview_bytes: Option<Vec<u8>>,
    pub preview_filename: String,
    pub transition_start: Option<std::time::Instant>,
    pub previous_tab: DashboardTab,
    
    // Real-time device metrics
    pub cpu_usage: f32,
    pub ram_usage: f32,
    pub io_usage: f32,
    pub device_disk_total: u64,
    pub device_disk_free: u64,
    
    // Panic Button state
    pub last_esc_press: Option<std::time::Instant>,

    // Animation
    pub pin_shake_timer: f32,

    pub request_keyboard: bool,
    pub focused_field: FocusedField,
    pub show_keyboard: bool,
    
    // File Picker
    pub file_dialog: Option<egui_file_dialog::FileDialog>,
    
    // Anti-Tampering security violation details
    pub security_violation: Option<String>,

    // Pelacakan aktivitas untuk Auto-Lock
    pub last_activity: std::time::Instant,

    // P2P Sharing States
    pub share_active_record: Option<FileRecord>,
    pub share_pin: String,
    pub share_port: u16,
    pub share_ip: String,
    pub share_stop_signal: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            screen:           AppScreen::Login,
            dashboard_tab:    DashboardTab::Home,

            login_username:   String::new(),
            login_password:   String::new(),
            login_error:      None,

            setup_username:         String::new(),
            setup_display_name:     String::new(),
            setup_password:         String::new(),
            setup_password_confirm: String::new(),
            setup_error:            None,

            display_name:     String::new(),

            session_key:      None,
            session_salt:     None,
            file_list:        Vec::new(),
            deleted_list:     Vec::new(),

            system_trash_items: Vec::new(),
            system_trash_loading: false,

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
            vault_search_query: String::new(),
            vault_view_mode: ViewMode::List,
            vault_sort_by: SortOption::DateDesc,

            profile_old_password: String::new(),
            profile_new_password: String::new(),
            profile_confirm_password: String::new(),
            profile_password_error: None,
            profile_password_success: None,

            audit_logs: Vec::new(),
            is_light_mode: false,
            preview_bytes: None,
            preview_filename: String::new(),
            transition_start: None,
            previous_tab: DashboardTab::Home,
            cpu_usage: 0.0,
            ram_usage: 0.0,
            io_usage: 0.0,
            device_disk_total: 0,
            device_disk_free: 0,
            last_esc_press: None,
            pin_shake_timer: 0.0,
            request_keyboard: false,
            focused_field: FocusedField::None,
            show_keyboard: false,
            file_dialog: None,
            security_violation: None,
            last_activity: std::time::Instant::now(),
            share_active_record: None,
            share_pin: String::new(),
            share_port: 0,
            share_ip: String::new(),
            share_stop_signal: None,
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
