// anti_tamper.rs — Pendeteksi Root & Emulator (Anti-Tampering)
// Menyediakan pemeriksaan keamanan di level sistem untuk mendeteksi
// apakah perangkat telah dimodifikasi (Rooted) atau dijalankan di dalam emulator.

pub fn check_security_violation() -> Option<String> {
    // 0. Cek mock environment terlebih dahulu (sangat berguna untuk pengujian di Windows/Desktop)
    #[cfg(not(target_os = "android"))]
    {
        if let Some(reason) = check_mock_env() {
            return Some(reason);
        }
    }

    // 1. Cek keberadaan file biner 'su' yang biasa digunakan oleh superuser/Magisk
    const SU_PATHS: &[&str] = &[
        "/system/app/Superuser.apk",
        "/sbin/su",
        "/system/bin/su",
        "/system/xbin/su",
        "/data/local/xbin/su",
        "/data/local/bin/su",
        "/system/sd/xbin/su",
        "/system/bin/failsafe/su",
        "/data/local/su",
        "/su/bin/su",
        "/system/usr/we-need-sys/su-backup",
        "/system/xbin/mu",
    ];

    for path in SU_PATHS {
        if std::path::Path::new(path).exists() {
            return Some(format!(
                "ROOT terdeteksi: berkas '{}' ditemukan di sistem.",
                path
            ));
        }
    }

    // 2. Cek keberadaan file-file virtualisasi atau driver yang khas pada Emulator
    const EMU_FILES: &[&str] = &[
        "/dev/socket/qemud",
        "/dev/qemu_pipe",
        "/system/lib/libc_malloc_debug_qemu.so",
        "/sys/qemu_trace",
        "/system/bin/qemu-props",
        "/system/lib/libdroid4x.so",
        "/system/bin/windroyed",
        "/system/bin/nox-prop",
        "/system/lib/libnoxspeedup.so",
    ];

    for file in EMU_FILES {
        if std::path::Path::new(file).exists() {
            return Some(format!(
                "EMULATOR terdeteksi: berkas emulasi '{}' ditemukan di sistem.",
                file
            ));
        }
    }

    // 3. Cek properti sistem (Android System Properties via getprop)
    // Pengecekan ini paling akurat untuk mendeteksi emulator Android resmi maupun pihak ketiga (Genymotion, BlueStacks, Nox, dll.)

    // ro.hardware
    if let Some(hardware) = get_prop("ro.hardware") {
        let hw_lower = hardware.to_lowercase();
        if hw_lower.contains("goldfish")
            || hw_lower.contains("ranchu")
            || hw_lower.contains("vbox86")
            || hw_lower.contains("sdk")
            || hw_lower.contains("nox")
        {
            return Some(format!(
                "EMULATOR terdeteksi: ro.hardware = '{}'.",
                hardware
            ));
        }
    }

    // ro.kernel.qemu
    if let Some(qemu) = get_prop("ro.kernel.qemu") {
        if qemu == "1" {
            return Some("EMULATOR terdeteksi: ro.kernel.qemu bernilai '1'.".to_string());
        }
    }

    // ro.product.model
    if let Some(model) = get_prop("ro.product.model") {
        let model_lower = model.to_lowercase();
        if model_lower.contains("sdk")
            || model_lower.contains("emulator")
            || model_lower.contains("android sdk")
            || model_lower.contains("genymotion")
        {
            return Some(format!(
                "EMULATOR terdeteksi: ro.product.model = '{}'.",
                model
            ));
        }
    }

    // ro.product.device
    if let Some(device) = get_prop("ro.product.device") {
        let device_lower = device.to_lowercase();
        if device_lower.contains("generic")
            || device_lower.contains("vbox")
            || device_lower.contains("emulator")
            || device_lower.contains("nox")
        {
            return Some(format!(
                "EMULATOR terdeteksi: ro.product.device = '{}'.",
                device
            ));
        }
    }

    // ro.product.brand
    if let Some(brand) = get_prop("ro.product.brand") {
        let brand_lower = brand.to_lowercase();
        if brand_lower.contains("generic")
            || brand_lower.contains("google")
                && get_prop("ro.product.name")
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains("sdk")
        {
            return Some(format!(
                "EMULATOR terdeteksi: ro.product.brand = '{}'.",
                brand
            ));
        }
    }

    // ro.product.name
    if let Some(name) = get_prop("ro.product.name") {
        let name_lower = name.to_lowercase();
        if name_lower.contains("sdk")
            || name_lower.contains("emulator")
            || name_lower.contains("nox")
            || name_lower.contains("vbox86")
        {
            return Some(format!(
                "EMULATOR terdeteksi: ro.product.name = '{}'.",
                name
            ));
        }
    }

    // ro.build.tags (Mendeteksi custom build ROM / test-keys yang sering digunakan di lingkungan rooted/hacking)
    if let Some(tags) = get_prop("ro.build.tags") {
        if tags.to_lowercase().contains("test-keys") {
            return Some(format!(
                "ROOT terdeteksi: build tag menggunakan '{}' (test-keys).",
                tags
            ));
        }
    };
    None
}

/// Helper untuk mengambil nilai system property via perintah 'getprop'
#[allow(unused_variables)]
fn get_prop(prop_name: &str) -> Option<String> {
    #[cfg(target_os = "android")]
    {
        // Menghindari eksekusi subprocess (Command::new) di Android karena
        // akan langsung memicu SIGKILL / SIGSYS (Security Violation) di Android 10+.
        // Untuk saat ini kita bypass pengecekan via getprop.
        None
    }
    #[cfg(not(target_os = "android"))]
    {
        use std::process::Command;
        let output = Command::new("getprop").arg(prop_name).output().ok()?;

        if output.status.success() {
            let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !val.is_empty() {
                return Some(val);
            }
        }
        None
    }
}

/// Helper untuk mock pengujian di lingkungan non-Android (Windows / Linux / macOS)
#[cfg(not(target_os = "android"))]
fn check_mock_env() -> Option<String> {
    if std::env::var("MOCK_ROOT").is_ok() {
        return Some("ROOT (Magisk / su) terdeteksi (Simulasi MOCK_ROOT).".to_string());
    }
    if std::env::var("MOCK_EMULATOR").is_ok() {
        return Some("EMULATOR (Virtual Device) terdeteksi (Simulasi MOCK_EMULATOR).".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_root() {
        std::env::set_var("MOCK_ROOT", "1");
        std::env::remove_var("MOCK_EMULATOR");
        let res = check_security_violation();
        assert!(res.is_some());
        assert!(res.unwrap().contains("ROOT"));
        std::env::remove_var("MOCK_ROOT");
    }

    #[test]
    fn test_mock_emulator() {
        std::env::remove_var("MOCK_ROOT");
        std::env::set_var("MOCK_EMULATOR", "1");
        let res = check_security_violation();
        assert!(res.is_some());
        assert!(res.unwrap().contains("EMULATOR"));
        std::env::remove_var("MOCK_EMULATOR");
    }

    #[test]
    fn test_normal_non_android() {
        #[cfg(not(target_os = "android"))]
        {
            std::env::remove_var("MOCK_ROOT");
            std::env::remove_var("MOCK_EMULATOR");
            let res = check_security_violation();
            assert!(res.is_none());
        }
    }
}
