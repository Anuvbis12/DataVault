// recycle_bin.rs — Windows Recycle Bin Scanner
// Membaca file yang dihapus dari Recycle Bin Windows
// dan memungkinkan user untuk melihat dan memulihkan file tersebut.

use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;
use crate::app_state::RecycleBinItem;

/// Mendapatkan SID user saat ini dari environment
fn get_user_sid() -> Option<String> {
    // Coba dapatkan SID via command: whoami /user
    let output = std::process::Command::new("cmd")
        .args(["/C", "whoami /user /fo csv /nh"])
        .output()
        .ok()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Format: "DOMAIN\User","S-1-5-21-..."
    let parts: Vec<&str> = stdout.trim().split(',').collect();
    if parts.len() >= 2 {
        let sid = parts[1].trim().trim_matches('"').to_string();
        if sid.starts_with("S-1-") {
            return Some(sid);
        }
    }
    None
}

/// Parse $I file dari Recycle Bin Windows (format Windows 10+)
/// Format:
///   Bytes 0-7:   Header/Version (u64 LE, biasanya 2)
///   Bytes 8-15:  File size (u64 LE)
///   Bytes 16-23: Deletion timestamp (FILETIME - u64 LE)
///   Bytes 24-27: Path length in characters (u32 LE)
///   Bytes 28+:   Original path (UTF-16LE)
fn parse_i_file(i_path: &Path) -> Option<(String, u64, String)> {
    let mut file = fs::File::open(i_path).ok()?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).ok()?;
    
    if data.len() < 28 {
        return None;
    }
    
    // Version check
    let version = u64::from_le_bytes(data[0..8].try_into().ok()?);
    
    // File size
    let file_size = u64::from_le_bytes(data[8..16].try_into().ok()?);
    
    // Deletion timestamp (FILETIME → epoch)
    let filetime = u64::from_le_bytes(data[16..24].try_into().ok()?);
    let deleted_at = filetime_to_string(filetime);
    
    // Original path
    let original_path = if version == 2 {
        // Windows 10+ format: u32 path length + UTF-16LE string
        let path_len = u32::from_le_bytes(data[24..28].try_into().ok()?) as usize;
        let path_bytes = &data[28..];
        
        // Decode UTF-16LE
        let utf16_chars: Vec<u16> = path_bytes.chunks(2)
            .take(path_len)
            .filter_map(|chunk| {
                if chunk.len() == 2 {
                    Some(u16::from_le_bytes([chunk[0], chunk[1]]))
                } else {
                    None
                }
            })
            .collect();
        
        String::from_utf16_lossy(&utf16_chars).trim_end_matches('\0').to_string()
    } else if version == 1 {
        // Older format: langsung UTF-16LE dari offset 24
        let path_bytes = &data[24..];
        let utf16_chars: Vec<u16> = path_bytes.chunks(2)
            .filter_map(|chunk| {
                if chunk.len() == 2 {
                    let val = u16::from_le_bytes([chunk[0], chunk[1]]);
                    if val == 0 { None } else { Some(val) }
                } else {
                    None
                }
            })
            .collect();
        String::from_utf16_lossy(&utf16_chars)
    } else {
        return None;
    };
    
    if original_path.is_empty() {
        return None;
    }
    
    Some((original_path, file_size, deleted_at))
}

/// Convert Windows FILETIME (100-nanosecond intervals since 1601-01-01) to readable string
fn filetime_to_string(filetime: u64) -> String {
    if filetime == 0 {
        return "Unknown".to_string();
    }
    
    // FILETIME epoch: 1601-01-01 00:00:00 UTC
    // Unix epoch:     1970-01-01 00:00:00 UTC
    // Difference: 11644473600 seconds = 116444736000000000 100-ns intervals
    const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
    
    if filetime < EPOCH_DIFF {
        return "Unknown".to_string();
    }
    
    let unix_100ns = filetime - EPOCH_DIFF;
    let unix_secs = (unix_100ns / 10_000_000) as i64;
    
    if let Some(dt) = chrono::DateTime::from_timestamp(unix_secs, 0) {
        let local: chrono::DateTime<chrono::Local> = dt.into();
        local.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Scan Windows Recycle Bin dan kembalikan daftar file yang dihapus
pub fn scan_recycle_bin() -> Vec<RecycleBinItem> {
    let mut items = Vec::new();
    
    let sid = match get_user_sid() {
        Some(s) => s,
        None => return items,
    };
    
    // Cari semua drive yang mungkin punya Recycle Bin
    let drives = ["C:", "D:", "E:", "F:", "G:", "H:"];
    
    for drive in &drives {
        let recycle_path = PathBuf::from(format!("{}\\$Recycle.Bin\\{}", drive, sid));
        
        if !recycle_path.exists() {
            continue;
        }
        
        let entries = match fs::read_dir(&recycle_path) {
            Ok(e) => e,
            Err(_) => continue,
        };
        
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            // $I files contain metadata, $R files contain actual data
            if !file_name.starts_with("$I") {
                continue;
            }
            
            let i_path = entry.path();
            
            // Corresponding $R file
            let r_filename = file_name.replacen("$I", "$R", 1);
            let r_path = recycle_path.join(&r_filename);
            
            if !r_path.exists() {
                continue;
            }
            
            if let Some((original_path, file_size, deleted_at)) = parse_i_file(&i_path) {
                let display_name = Path::new(&original_path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                
                let is_directory = r_path.is_dir();
                
                items.push(RecycleBinItem {
                    original_path,
                    file_name: display_name,
                    file_size,
                    deleted_at,
                    recycle_path: r_path.to_string_lossy().to_string(),
                    is_directory,
                });
            }
        }
    }
    
    // Sort by deletion date (newest first)
    items.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));
    
    items
}

/// Restore file dari Recycle Bin ke lokasi semula
pub fn restore_to_original(item: &RecycleBinItem) -> Result<(), String> {
    let source = Path::new(&item.recycle_path);
    let dest = Path::new(&item.original_path);
    
    if !source.exists() {
        return Err("File tidak ditemukan di Recycle Bin.".to_string());
    }
    
    // Buat direktori tujuan jika belum ada
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Gagal membuat direktori tujuan: {}", e))?;
    }
    
    // Copy file dari $R ke lokasi asli
    if item.is_directory {
        copy_dir_recursive(source, dest)
            .map_err(|e| format!("Gagal memulihkan folder: {}", e))?;
    } else {
        fs::copy(source, dest)
            .map_err(|e| format!("Gagal memulihkan file: {}", e))?;
    }
    
    // Hapus file dari Recycle Bin ($R dan $I)
    let i_filename = source.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .replacen("$R", "$I", 1);
    let i_path = source.parent().unwrap_or(Path::new("")).join(i_filename);
    
    let _ = if item.is_directory { fs::remove_dir_all(source) } else { fs::remove_file(source) };
    let _ = fs::remove_file(i_path);
    
    Ok(())
}

/// Restore file dari Recycle Bin ke lokasi pilihan user
pub fn restore_to_custom(item: &RecycleBinItem, dest_dir: &Path) -> Result<PathBuf, String> {
    let source = Path::new(&item.recycle_path);
    
    if !source.exists() {
        return Err("File tidak ditemukan di Recycle Bin.".to_string());
    }
    
    let dest = dest_dir.join(&item.file_name);
    
    if item.is_directory {
        copy_dir_recursive(source, &dest)
            .map_err(|e| format!("Gagal memulihkan folder: {}", e))?;
    } else {
        fs::copy(source, &dest)
            .map_err(|e| format!("Gagal memulihkan file: {}", e))?;
    }
    
    // Hapus dari Recycle Bin
    let i_filename = source.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .replacen("$R", "$I", 1);
    let i_path = source.parent().unwrap_or(Path::new("")).join(i_filename);
    
    let _ = if item.is_directory { fs::remove_dir_all(source) } else { fs::remove_file(source) };
    let _ = fs::remove_file(i_path);
    
    Ok(dest)
}

/// Copy directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}
