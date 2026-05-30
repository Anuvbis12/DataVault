// recycle_bin.rs — Windows Recycle Bin Scanner & Android MediaStore Trash Detector
// Menghubungkan aplikasi ke sistem tempat sampah bawaan OS (Windows & Android)

use crate::app_state::RecycleBinItem;
use std::path::{Path, PathBuf};

#[cfg(not(target_os = "android"))]
pub use windows_impl::*;

#[cfg(target_os = "android")]
pub use android_impl::*;

// ── IMPLEMENTASI WINDOWS ─────────────────────────────────────────────────────
#[cfg(not(target_os = "android"))]
mod windows_impl {
    use super::*;
    use std::fs;
    use std::io::Read;

    /// Mendapatkan SID user saat ini dari environment
    fn get_user_sid() -> Option<String> {
        let output = std::process::Command::new("cmd")
            .args(["/C", "whoami /user /fo csv /nh"])
            .output()
            .ok()?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
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
    fn parse_i_file(i_path: &Path) -> Option<(String, u64, String)> {
        let mut file = fs::File::open(i_path).ok()?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).ok()?;
        
        if data.len() < 28 {
            return None;
        }
        
        let version = u64::from_le_bytes(data[0..8].try_into().ok()?);
        let file_size = u64::from_le_bytes(data[8..16].try_into().ok()?);
        let filetime = u64::from_le_bytes(data[16..24].try_into().ok()?);
        let deleted_at = filetime_to_string(filetime);
        
        let original_path = if version == 2 {
            let path_len = u32::from_le_bytes(data[24..28].try_into().ok()?) as usize;
            let path_bytes = &data[28..];
            
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

    /// Convert Windows FILETIME to readable string
    fn filetime_to_string(filetime: u64) -> String {
        if filetime == 0 {
            return "Unknown".to_string();
        }
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
                
                if !file_name.starts_with("$I") {
                    continue;
                }
                
                let i_path = entry.path();
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
        
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Gagal membuat direktori tujuan: {}", e))?;
        }
        
        if item.is_directory {
            copy_dir_recursive(source, dest)
                .map_err(|e| format!("Gagal memulihkan folder: {}", e))?;
        } else {
            fs::copy(source, dest)
                .map_err(|e| format!("Gagal memulihkan file: {}", e))?;
        }
        
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
        
        let i_filename = source.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .replacen("$R", "$I", 1);
        let i_path = source.parent().unwrap_or(Path::new("")).join(i_filename);
        
        let _ = if item.is_directory { fs::remove_dir_all(source) } else { fs::remove_file(source) };
        let _ = fs::remove_file(i_path);
        
        Ok(dest)
    }

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
}

// ── IMPLEMENTASI ANDROID (MEDIASTORE JNI) ────────────────────────────────────
#[cfg(target_os = "android")]
mod android_impl {
    use super::*;

    /// Scan Android MediaStore untuk mendapatkan item yang berstatus IS_TRASHED = 1
    pub fn scan_recycle_bin() -> Vec<RecycleBinItem> {
        let mut items = Vec::new();
        
        let vm = match unsafe { jni::JavaVM::from_raw(ndk_context::android_context().vm() as *mut _) } {
            Ok(v) => v,
            Err(_) => return items,
        };
        let mut env = match vm.attach_current_thread() {
            Ok(e) => e,
            Err(_) => return items,
        };
        let _ = env.exception_clear();
        
        let context_ptr = ndk_context::android_context().context();
        let activity_obj = unsafe { jni::objects::JObject::from_raw(context_ptr as jni::sys::jobject) };
        
        let content_resolver = match env.call_method(&activity_obj, "getContentResolver", "()Landroid/content/ContentResolver;", &[]) {
            Ok(res) => res.l().unwrap(),
            Err(_) => return items,
        };
        
        let media_store_files = match env.find_class("android/provider/MediaStore$Files") {
            Ok(c) => c,
            Err(_) => return items,
        };
        
        let external_string = env.new_string("external").unwrap();
        let query_uri = match env.call_static_method(
            &media_store_files,
            "getContentUri",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[jni::objects::JValue::Object(&external_string)]
        ) {
            Ok(res) => res.l().unwrap(),
            Err(_) => return items,
        };

        let bundle_class = match env.find_class("android/os/Bundle") {
            Ok(c) => c,
            Err(_) => return items,
        };
        let bundle = match env.new_object(&bundle_class, "()V", &[]) {
            Ok(o) => o,
            Err(_) => return items,
        };
        
        let match_trashed_key = env.new_string("android.provider.extra.MATCH_TRASHED").unwrap();
        let _ = env.call_method(&bundle, "putInt", "(Ljava/lang/String;I)V", &[
            jni::objects::JValue::Object(&match_trashed_key),
            jni::objects::JValue::Int(3) // MATCH_ONLY = 3 (Hanya ambil file di sampah)
        ]);

        let columns = [
            "_id",
            "_display_name",
            "_size",
            "_data",
            "date_expires",
            "media_type"
        ];
        let string_class = env.find_class("java/lang/String").unwrap();
        let array = env.new_object_array(columns.len() as jni::sys::jsize, &string_class, jni::objects::JObject::null()).unwrap();
        for (idx, &col) in columns.iter().enumerate() {
            let col_str = env.new_string(col).unwrap();
            env.set_object_array_element(&array, idx as jni::sys::jsize, &col_str).unwrap();
        }

        let cursor_obj = match env.call_method(
            &content_resolver,
            "query",
            "(Landroid/net/Uri;[Ljava/lang/String;Landroid/os/Bundle;Landroid/os/CancellationSignal;)Landroid/database/Cursor;",
            &[
                jni::objects::JValue::Object(&query_uri),
                jni::objects::JValue::Object(&array),
                jni::objects::JValue::Object(&bundle),
                jni::objects::JValue::Object(&jni::objects::JObject::null())
            ]
        ) {
            Ok(res) => res.l().unwrap(),
            Err(e) => {
                log::error!("Gagal query MediaStore: {:?}", e);
                let _ = env.exception_clear();
                return items;
            }
        };

        if !cursor_obj.is_null() {
            let id_str = env.new_string("_id").unwrap();
            let name_str = env.new_string("_display_name").unwrap();
            let size_str_col = env.new_string("_size").unwrap();
            let data_str = env.new_string("_data").unwrap();
            let expires_str = env.new_string("date_expires").unwrap();
            let media_type_str = env.new_string("media_type").unwrap();

            let id_col = env.call_method(&cursor_obj, "getColumnIndex", "(Ljava/lang/String;)I", &[jni::objects::JValue::Object(&id_str)]).unwrap().i().unwrap();
            let name_col = env.call_method(&cursor_obj, "getColumnIndex", "(Ljava/lang/String;)I", &[jni::objects::JValue::Object(&name_str)]).unwrap().i().unwrap();
            let size_col = env.call_method(&cursor_obj, "getColumnIndex", "(Ljava/lang/String;)I", &[jni::objects::JValue::Object(&size_str_col)]).unwrap().i().unwrap();
            let data_col = env.call_method(&cursor_obj, "getColumnIndex", "(Ljava/lang/String;)I", &[jni::objects::JValue::Object(&data_str)]).unwrap().i().unwrap();
            let expires_col = env.call_method(&cursor_obj, "getColumnIndex", "(Ljava/lang/String;)I", &[jni::objects::JValue::Object(&expires_str)]).unwrap().i().unwrap();
            let media_type_col = env.call_method(&cursor_obj, "getColumnIndex", "(Ljava/lang/String;)I", &[jni::objects::JValue::Object(&media_type_str)]).unwrap().i().unwrap();

            while env.call_method(&cursor_obj, "moveToNext", "()Z", &[]).unwrap().z().unwrap() {
                let id = if id_col >= 0 { env.call_method(&cursor_obj, "getLong", "(I)J", &[jni::objects::JValue::Int(id_col)]).unwrap().j().unwrap() } else { 0 };
                let name = if name_col >= 0 {
                    let jstr = env.call_method(&cursor_obj, "getString", "(I)Ljava/lang/String;", &[jni::objects::JValue::Int(name_col)]).unwrap().l().unwrap();
                    if !jstr.is_null() {
                        let s: String = env.get_string(&jstr.into()).unwrap().into();
                        s
                    } else {
                        "Unknown".to_string()
                    }
                } else {
                    "Unknown".to_string()
                };
                let size = if size_col >= 0 { env.call_method(&cursor_obj, "getLong", "(I)J", &[jni::objects::JValue::Int(size_col)]).unwrap().j().unwrap() as u64 } else { 0 };
                let data = if data_col >= 0 {
                    let jstr = env.call_method(&cursor_obj, "getString", "(I)Ljava/lang/String;", &[jni::objects::JValue::Int(data_col)]).unwrap().l().unwrap();
                    if !jstr.is_null() {
                        let s: String = env.get_string(&jstr.into()).unwrap().into();
                        s
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                };
                let expires = if expires_col >= 0 {
                    env.call_method(&cursor_obj, "getLong", "(I)J", &[jni::objects::JValue::Int(expires_col)]).unwrap().j().unwrap()
                } else {
                    0
                };
                let media_type = if media_type_col >= 0 {
                    env.call_method(&cursor_obj, "getInt", "(I)I", &[jni::objects::JValue::Int(media_type_col)]).unwrap().i().unwrap()
                } else {
                    0
                };

                let deleted_at_str = if expires > 0 {
                    if let Some(dt) = chrono::DateTime::from_timestamp(expires, 0) {
                        let local: chrono::DateTime<chrono::Local> = dt.into();
                        let deletion_time = local - chrono::Duration::days(30);
                        deletion_time.format("%Y-%m-%d %H:%M:%S").to_string()
                    } else {
                        "Unknown".to_string()
                    }
                } else {
                    "Unknown".to_string()
                };

                let lower_name = name.to_lowercase();
                let item_uri = if lower_name.ends_with(".jpg") || lower_name.ends_with(".jpeg") || lower_name.ends_with(".png") || lower_name.ends_with(".webp") || lower_name.ends_with(".gif") || media_type == 1 {
                    format!("{}/{}", "content://media/external/images/media", id)
                } else if lower_name.ends_with(".mp4") || lower_name.ends_with(".mkv") || lower_name.ends_with(".avi") || lower_name.ends_with(".webm") || lower_name.ends_with(".mov") || media_type == 3 {
                    format!("{}/{}", "content://media/external/video/media", id)
                } else if lower_name.ends_with(".mp3") || lower_name.ends_with(".wav") || lower_name.ends_with(".ogg") || lower_name.ends_with(".m4a") || lower_name.ends_with(".flac") || media_type == 2 {
                    format!("{}/{}", "content://media/external/audio/media", id)
                } else {
                    format!("{}/{}", "content://media/external/file", id)
                };

                items.push(RecycleBinItem {
                    original_path: data,
                    file_name: name,
                    file_size: size,
                    deleted_at: deleted_at_str,
                    recycle_path: item_uri,
                    is_directory: false,
                });
            }
            let _ = env.call_method(&cursor_obj, "close", "()V", &[]);
        }
        
        items
    }

    pub fn restore_to_original(item: &RecycleBinItem) -> Result<(), String> {
        android_restore_item(item)
    }

    pub fn restore_to_custom(item: &RecycleBinItem, dest_dir: &Path) -> Result<PathBuf, String> {
        let bytes = read_recycle_bin_item_bytes(item)?;
        let dest = dest_dir.join(&item.file_name);
        std::fs::write(&dest, bytes).map_err(|e| format!("Gagal menyimpan file: {}", e))?;
        // Setelah berhasil menyalin bytes, coba hapus dari MediaStore.
        // Jika file milik app lain, ini mungkin gagal — tidak dianggap error fatal.
        let _ = android_permanent_delete_owned(item);
        Ok(dest)
    }

    pub fn android_restore_item(item: &RecycleBinItem) -> Result<(), String> {
        let vm = unsafe { jni::JavaVM::from_raw(ndk_context::android_context().vm() as *mut _).unwrap() };
        let mut env = vm.attach_current_thread().unwrap();
        let _ = env.exception_clear();
        
        let context_ptr = ndk_context::android_context().context();
        let activity_obj = unsafe { jni::objects::JObject::from_raw(context_ptr as jni::sys::jobject) };
        
        let content_resolver = env.call_method(&activity_obj, "getContentResolver", "()Landroid/content/ContentResolver;", &[])
            .map_err(|e| format!("Gagal get content resolver: {:?}", e))?.l().unwrap();
            
        let uri_class = env.find_class("android/net/Uri").unwrap();
        let uri_str = env.new_string(&item.recycle_path).unwrap();
        let uri = env.call_static_method(
            &uri_class,
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[jni::objects::JValue::Object(&uri_str)]
        ).map_err(|e| format!("Gagal parse uri: {:?}", e))?.l().unwrap();

        let values_class = env.find_class("android/content/ContentValues").unwrap();
        let values = env.new_object(&values_class, "()V", &[]).unwrap();
        
        let is_trashed_key = env.new_string("is_trashed").unwrap();
        let integer_class = env.find_class("java/lang/Integer").unwrap();
        let zero_val = env.new_object(&integer_class, "(I)V", &[jni::objects::JValue::Int(0)]).unwrap();
        
        let _ = env.call_method(
            &values,
            "put",
            "(Ljava/lang/String;Ljava/lang/Integer;)V",
            &[
                jni::objects::JValue::Object(&is_trashed_key),
                jni::objects::JValue::Object(&zero_val)
            ]
        ).map_err(|e| format!("Gagal set ContentValues: {:?}", e))?;

        let rows_updated = env.call_method(
            &content_resolver,
            "update",
            "(Landroid/net/Uri;Landroid/content/ContentValues;Ljava/lang/String;[Ljava/lang/String;)I",
            &[
                jni::objects::JValue::Object(&uri),
                jni::objects::JValue::Object(&values),
                jni::objects::JValue::Object(&jni::objects::JObject::null()),
                jni::objects::JValue::Object(&jni::objects::JObject::null())
            ]
        ).map_err(|e| {
            let _ = env.exception_clear();
            format!("Gagal memulihkan dari MediaStore: {:?}", e)
        })?.i().unwrap();

        if rows_updated > 0 {
            Ok(())
        } else {
            Err("Gagal memulihkan file dari MediaStore.".into())
        }
    }

    /// Meminta penghapusan permanen item MediaStore Trash via dialog sistem Android.
    ///
    /// Menggunakan `MediaStore.createDeleteRequest(contentResolver, listOf(uri))`
    /// untuk membuat PendingIntent yang akan memicu dialog konfirmasi bawaan Android.
    /// Mengembalikan URI string dari PendingIntent tersebut agar bisa diserahkan ke Kotlin
    /// lewat `launchDeleteRequest(intentUriString)`.
    ///
    /// Flow lengkap:
    ///   1. Rust → android_request_delete(item) → dapat intent_uri String
    ///   2. AppState.request_android_delete_uris.push(intent_uri)
    ///   3. lib.rs membaca flag → panggil Kotlin launchDeleteRequest(uri)
    ///   4. Kotlin menjalankan startIntentSenderForResult → Android tampilkan dialog
    ///   5. Setelah user klik OK → Kotlin panggil onDeleteConfirmedNative()
    ///   6. AppState.android_delete_confirmed = true → UI refresh & hapus item dari list
    pub fn android_request_delete(item: &RecycleBinItem) -> Result<String, String> {
        let vm = unsafe {
            jni::JavaVM::from_raw(ndk_context::android_context().vm() as *mut _).unwrap()
        };
        let mut env = vm.attach_current_thread().unwrap();
        let _ = env.exception_clear();

        let context_ptr = ndk_context::android_context().context();
        let activity_obj = unsafe {
            jni::objects::JObject::from_raw(context_ptr as jni::sys::jobject)
        };

        // 1. Dapatkan ContentResolver
        let content_resolver = env
            .call_method(
                &activity_obj,
                "getContentResolver",
                "()Landroid/content/ContentResolver;",
                &[],
            )
            .map_err(|e| format!("Gagal get content resolver: {:?}", e))?
            .l()
            .unwrap();

        // 2. Parse URI item
        let uri_class = env
            .find_class("android/net/Uri")
            .map_err(|e| format!("Gagal find Uri class: {:?}", e))?;
        let uri_jstr = env.new_string(&item.recycle_path).unwrap();
        let item_uri = env
            .call_static_method(
                &uri_class,
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[jni::objects::JValue::Object(&uri_jstr)],
            )
            .map_err(|e| format!("Gagal parse URI: {:?}", e))?
            .l()
            .unwrap();

        // 3. Buat Collection<Uri> — gunakan ArrayList
        let arraylist_class = env
            .find_class("java/util/ArrayList")
            .map_err(|e| format!("Gagal find ArrayList: {:?}", e))?;
        let uri_list = env
            .new_object(&arraylist_class, "()V", &[])
            .map_err(|e| format!("Gagal buat ArrayList: {:?}", e))?;
        env.call_method(
            &uri_list,
            "add",
            "(Ljava/lang/Object;)Z",
            &[jni::objects::JValue::Object(&item_uri)],
        )
        .map_err(|e| format!("Gagal add URI ke list: {:?}", e))?;

        // 4. Panggil MediaStore.createDeleteRequest(contentResolver, uriCollection)
        let mediastore_class = env
            .find_class("android/provider/MediaStore")
            .map_err(|e| format!("Gagal find MediaStore: {:?}", e))?;
        let pending_intent = env
            .call_static_method(
                &mediastore_class,
                "createDeleteRequest",
                "(Landroid/content/ContentResolver;Ljava/util/Collection;)Landroid/app/PendingIntent;",
                &[
                    jni::objects::JValue::Object(&content_resolver),
                    jni::objects::JValue::Object(&uri_list),
                ],
            )
            .map_err(|e| {
                let _ = env.exception_clear();
                format!("Gagal createDeleteRequest: {:?}", e)
            })?
            .l()
            .unwrap();

        if pending_intent.is_null() {
            return Err("createDeleteRequest mengembalikan null PendingIntent".into());
        }

        // 5. Ambil IntentSender dari PendingIntent
        let intent_sender = env
            .call_method(
                &pending_intent,
                "getIntentSender",
                "()Landroid/content/IntentSender;",
                &[],
            )
            .map_err(|e| format!("Gagal getIntentSender: {:?}", e))?
            .l()
            .unwrap();

        // 6. Serialisasi IntentSender ke String via toString()
        //    Format: "IntentSender{...}" — kita pakai ini sebagai token yang
        //    dikembalikan ke AppState lalu dikirim ke Kotlin.
        //    Di sisi Kotlin, kita tidak parse string ini; kita simpan PendingIntent-nya
        //    di companion object dan cukup gunakan token sebagai sinyal.
        //
        //    Strategi yang lebih robust: simpan dulu pending_intent di static Kotlin companion
        //    via JNI call terpisah, lalu Kotlin jalankan dari sana.
        //    Kita kirim sinyal "PENDING_DELETE:<uri>" ke Kotlin.
        let intent_uri_string = format!("PENDING_DELETE:{}", item.recycle_path);

        // 7. Simpan PendingIntent ke MainActivity.pendingDeleteIntent via JNI
        //    agar Kotlin bisa langsung meluncurkannya.
        let intent_uri_jstr = env
            .new_string(&item.recycle_path)
            .map_err(|e| format!("Gagal buat JString: {:?}", e))?;
        let _ = env.call_method(
            &activity_obj,
            "storePendingDeleteIntent",
            "(Landroid/app/PendingIntent;Ljava/lang/String;)V",
            &[
                jni::objects::JValue::Object(&pending_intent),
                jni::objects::JValue::Object(&intent_uri_jstr),
            ],
        );
        // Abaikan error jika metode belum ada; Kotlin akan tetap dipanggil
        // lewat flag intent_uri_string.
        let _ = env.exception_clear();

        Ok(intent_uri_string)
    }

    /// Meminta penghapusan permanen untuk BANYAK item MediaStore Trash sekaligus via dialog sistem Android.
    pub fn android_request_delete_multiple(items: &[RecycleBinItem]) -> Result<String, String> {
        if items.is_empty() {
            return Err("Tidak ada file untuk dihapus.".into());
        }

        let vm = unsafe {
            jni::JavaVM::from_raw(ndk_context::android_context().vm() as *mut _).unwrap()
        };
        let mut env = vm.attach_current_thread().unwrap();
        let _ = env.exception_clear();

        let context_ptr = ndk_context::android_context().context();
        let activity_obj = unsafe {
            jni::objects::JObject::from_raw(context_ptr as jni::sys::jobject)
        };

        // 1. Dapatkan ContentResolver
        let content_resolver = env
            .call_method(
                &activity_obj,
                "getContentResolver",
                "()Landroid/content/ContentResolver;",
                &[],
            )
            .map_err(|e| format!("Gagal get content resolver: {:?}", e))?
            .l()
            .unwrap();

        // 2. Parse URIs dan kumpulkan ke ArrayList
        let arraylist_class = env
            .find_class("java/util/ArrayList")
            .map_err(|e| format!("Gagal find ArrayList: {:?}", e))?;
        let uri_list = env
            .new_object(&arraylist_class, "()V", &[])
            .map_err(|e| format!("Gagal buat ArrayList: {:?}", e))?;

        let uri_class = env
            .find_class("android/net/Uri")
            .map_err(|e| format!("Gagal find Uri class: {:?}", e))?;

        // Batasi maksimal 2000 URI (limit Google untuk API 35+)
        let limit = items.len().min(2000);
        for item in items.iter().take(limit) {
            let uri_jstr = env.new_string(&item.recycle_path).unwrap();
            let item_uri = env
                .call_static_method(
                    &uri_class,
                    "parse",
                    "(Ljava/lang/String;)Landroid/net/Uri;",
                    &[jni::objects::JValue::Object(&uri_jstr)],
                )
                .map_err(|e| format!("Gagal parse URI: {:?}", e))?
                .l()
                .unwrap();

            env.call_method(
                &uri_list,
                "add",
                "(Ljava/lang/Object;)Z",
                &[jni::objects::JValue::Object(&item_uri)],
            )
            .map_err(|e| format!("Gagal add URI ke list: {:?}", e))?;
        }

        // 3. Panggil MediaStore.createDeleteRequest(contentResolver, uriCollection)
        let mediastore_class = env
            .find_class("android/provider/MediaStore")
            .map_err(|e| format!("Gagal find MediaStore: {:?}", e))?;
        let pending_intent = env
            .call_static_method(
                &mediastore_class,
                "createDeleteRequest",
                "(Landroid/content/ContentResolver;Ljava/util/Collection;)Landroid/app/PendingIntent;",
                &[
                    jni::objects::JValue::Object(&content_resolver),
                    jni::objects::JValue::Object(&uri_list),
                ],
            )
            .map_err(|e| {
                let _ = env.exception_clear();
                format!("Gagal createDeleteRequest: {:?}", e)
            })?
            .l()
            .unwrap();

        if pending_intent.is_null() {
            return Err("createDeleteRequest mengembalikan null PendingIntent".into());
        }

        // 4. Token representasi
        let first_item = &items[0];
        let intent_uri_string = format!("PENDING_DELETE:{}", first_item.recycle_path);

        // 5. Simpan PendingIntent ke MainActivity
        let first_item_jstr = env
            .new_string(&first_item.recycle_path)
            .map_err(|e| format!("Gagal buat JString: {:?}", e))?;
        let _ = env.call_method(
            &activity_obj,
            "storePendingDeleteIntent",
            "(Landroid/app/PendingIntent;Ljava/lang/String;)V",
            &[
                jni::objects::JValue::Object(&pending_intent),
                jni::objects::JValue::Object(&first_item_jstr),
            ],
        );
        let _ = env.exception_clear();

        Ok(intent_uri_string)
    }

    /// Fast-path: hapus langsung tanpa dialog untuk file yang memang milik app ini.
    /// Hanya berhasil jika file di-create oleh com.aegis.vault, jika tidak akan
    /// gagal dengan SecurityException (tergantung versi Android & ROM).
    pub fn android_permanent_delete_owned(item: &RecycleBinItem) -> Result<(), String> {
        let vm = unsafe {
            jni::JavaVM::from_raw(ndk_context::android_context().vm() as *mut _).unwrap()
        };
        let mut env = vm.attach_current_thread().unwrap();
        let _ = env.exception_clear();

        let context_ptr = ndk_context::android_context().context();
        let activity_obj = unsafe {
            jni::objects::JObject::from_raw(context_ptr as jni::sys::jobject)
        };

        let content_resolver = env
            .call_method(
                &activity_obj,
                "getContentResolver",
                "()Landroid/content/ContentResolver;",
                &[],
            )
            .map_err(|e| format!("Gagal get content resolver: {:?}", e))?
            .l()
            .unwrap();

        let uri_class = env.find_class("android/net/Uri").unwrap();
        let uri_str = env.new_string(&item.recycle_path).unwrap();
        let uri = env
            .call_static_method(
                &uri_class,
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[jni::objects::JValue::Object(&uri_str)],
            )
            .map_err(|e| format!("Gagal parse uri: {:?}", e))?
            .l()
            .unwrap();

        let rows_deleted = env
            .call_method(
                &content_resolver,
                "delete",
                "(Landroid/net/Uri;Ljava/lang/String;[Ljava/lang/String;)I",
                &[
                    jni::objects::JValue::Object(&uri),
                    jni::objects::JValue::Object(&jni::objects::JObject::null()),
                    jni::objects::JValue::Object(&jni::objects::JObject::null()),
                ],
            )
            .map_err(|e| {
                let _ = env.exception_clear();
                format!("Gagal menghapus dari MediaStore: {:?}", e)
            })?
            .i()
            .unwrap();

        if rows_deleted > 0 {
            Ok(())
        } else {
            Err("File tidak berhasil dihapus. Mungkin bukan milik app ini — gunakan android_request_delete.".into())
        }
    }

    pub fn read_recycle_bin_item_bytes(item: &RecycleBinItem) -> Result<Vec<u8>, String> {
        let vm = unsafe { jni::JavaVM::from_raw(ndk_context::android_context().vm() as *mut _).unwrap() };
        let mut env = vm.attach_current_thread().unwrap();
        let _ = env.exception_clear();
        
        let context_ptr = ndk_context::android_context().context();
        let activity_obj = unsafe { jni::objects::JObject::from_raw(context_ptr as jni::sys::jobject) };
        
        let content_resolver = env.call_method(&activity_obj, "getContentResolver", "()Landroid/content/ContentResolver;", &[])
            .map_err(|e| format!("Gagal get content resolver: {:?}", e))?.l().unwrap();
            
        let uri_class = env.find_class("android/net/Uri").unwrap();
        let uri_str = env.new_string(&item.recycle_path).unwrap();
        let uri = env.call_static_method(
            &uri_class,
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[jni::objects::JValue::Object(&uri_str)]
        ).map_err(|e| format!("Gagal parse uri: {:?}", e))?.l().unwrap();

        let input_stream = env.call_method(
            &content_resolver,
            "openInputStream",
            "(Landroid/net/Uri;)Ljava/io/InputStream;",
            &[jni::objects::JValue::Object(&uri)]
        ).map_err(|e| {
            let _ = env.exception_clear();
            format!("Gagal openInputStream: {:?}", e)
        })?.l().unwrap();

        if input_stream.is_null() {
            return Err("InputStream null".into());
        }

        let mut file_bytes = Vec::new();
        let mut buffer = vec![0i8; 4096];
        let byte_array = env.new_byte_array(4096).unwrap();
        
        loop {
            let bytes_read = env.call_method(
                &input_stream,
                "read",
                "([B)I",
                &[jni::objects::JValue::Object(&byte_array)]
            ).map_err(|e| format!("Gagal membaca stream file: {:?}", e))?.i().unwrap();

            if bytes_read <= 0 {
                break;
            }

            env.get_byte_array_region(&byte_array, 0, &mut buffer[..bytes_read as usize]).unwrap();
            let u8_slice = unsafe { std::slice::from_raw_parts(buffer.as_ptr() as *const u8, bytes_read as usize) };
            file_bytes.extend_from_slice(u8_slice);
        }

        let _ = env.call_method(&input_stream, "close", "()V", &[]);

        Ok(file_bytes)
    }
}
