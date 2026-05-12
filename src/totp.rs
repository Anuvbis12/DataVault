// totp.rs — TOTP (Time-based One-Time Password) RFC 6238
// Kompatibel dengan Google Authenticator, Authy, dll.
// Fitur: generate secret, verify code, QR code rendering untuk egui.

use eframe::egui;
use egui::epaint::{Color32, Vec2};
use hmac::{Hmac, Mac};
use rand::{thread_rng, RngCore};
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};

const PERIOD: u64 = 30;
const DIGITS: u32 = 6;
const SECRET_LEN: usize = 20; // 160-bit (standar TOTP)

type HmacSha1 = Hmac<Sha1>;

// ── Secret Management ─────────────────────────────────────

/// Generate secret acak 20 byte
pub fn generate_secret() -> [u8; SECRET_LEN] {
    let mut s = [0u8; SECRET_LEN];
    thread_rng().fill_bytes(&mut s);
    s
}

/// Encode bytes → base32 (tanpa padding, standar Google Auth)
pub fn to_base32(secret: &[u8]) -> String {
    data_encoding::BASE32_NOPAD.encode(secret)
}

/// Decode base32 → bytes
pub fn from_base32(encoded: &str) -> Option<Vec<u8>> {
    data_encoding::BASE32_NOPAD
        .decode(encoded.trim().to_uppercase().as_bytes())
        .ok()
}

// ── TOTP Algorithm ────────────────────────────────────────

/// Generate kode TOTP 6-digit untuk timestamp tertentu
pub fn code_at(secret: &[u8], timestamp: u64) -> String {
    let counter = timestamp / PERIOD;
    let mut mac = HmacSha1::new_from_slice(secret).expect("HMAC key valid");
    mac.update(&counter.to_be_bytes());
    let hash = mac.finalize().into_bytes();

    let off = (hash[19] & 0x0f) as usize;
    let bin = ((hash[off] as u32 & 0x7f) << 24)
        | ((hash[off + 1] as u32) << 16)
        | ((hash[off + 2] as u32) << 8)
        | (hash[off + 3] as u32);

    format!("{:06}", bin % 10u32.pow(DIGITS))
}

/// Verifikasi kode. Toleransi ±1 time step (untuk clock skew).
pub fn verify(secret: &[u8], code: &str) -> bool {
    let now = unix_now();
    for d in [-1i64, 0, 1] {
        let t = (now as i64 + d * PERIOD as i64) as u64;
        if code_at(secret, t) == code {
            return true;
        }
    }
    false
}

/// Sisa detik sebelum kode berubah (0–30)
pub fn seconds_left() -> u32 {
    let now = unix_now();
    (PERIOD - (now % PERIOD)) as u32
}

/// Generate otpauth URI untuk QR code
pub fn otpauth_uri(secret_b32: &str) -> String {
    format!(
        "otpauth://totp/AegisVault:Vault?secret={}&issuer=AegisVault&algorithm=SHA1&digits=6&period=30",
        secret_b32
    )
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── QR Code ───────────────────────────────────────────────

/// Generate QR matrix dari string data. true = dark module.
pub fn qr_matrix(data: &str) -> Option<Vec<Vec<bool>>> {
    let code = qrcode::QrCode::new(data.as_bytes()).ok()?;
    let w = code.width();
    let colors = code.into_colors();
    Some(
        colors
            .chunks(w)
            .map(|row| {
                row.iter()
                    .map(|c| *c == qrcode::Color::Dark)
                    .collect()
            })
            .collect(),
    )
}

/// Render QR matrix ke egui sebagai kotak hitam-putih
pub fn draw_qr(ui: &mut egui::Ui, matrix: &[Vec<bool>], size: f32) {
    let rows = matrix.len();
    if rows == 0 {
        return;
    }

    let quiet = 2usize; // white border (quiet zone)
    let total = rows + quiet * 2;
    let cell = size / total as f32;

    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());

    // White background
    ui.painter()
        .rect_filled(rect, egui::Rounding::same(6.0), Color32::WHITE);

    // Dark modules
    for (y, row) in matrix.iter().enumerate() {
        for (x, &dark) in row.iter().enumerate() {
            if dark {
                let cell_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.left() + (x + quiet) as f32 * cell,
                        rect.top() + (y + quiet) as f32 * cell,
                    ),
                    Vec2::splat(cell),
                );
                ui.painter().rect_filled(cell_rect, 0.0, Color32::BLACK);
            }
        }
    }
}
