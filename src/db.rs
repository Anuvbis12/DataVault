// db.rs — SQLite database layer
// Menyimpan metadata file terenkripsi: nama asli, path asli,
// nama vault, hash, IV, salt, waktu enkripsi.

use rusqlite::{Connection, Result, params};
use std::path::Path;

// ── Model ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AuditLog {
    pub id: i64,
    pub action_type: String,
    pub description: String,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
pub struct FolderRecord {
    pub id:             String,
    pub name:           String,
    pub icon:           String,
    pub color_hex:      String,
    pub created_at:     String,
}

#[derive(Debug, Clone)]
pub struct FileRecord {
    pub id:             String,
    pub original_name:  String,
    pub original_path:  String,
    pub vault_filename: String,
    pub sha256_hash:    String,
    pub file_size:      i64,
    pub iv_hex:         String,
    pub salt_hex:       String,
    pub encrypted_at:   String,
    pub is_deleted:     bool,
    pub deleted_at:     Option<String>,
    pub is_folder:      bool,
    pub folder_id:      Option<String>,
}

// ── Database ──────────────────────────────────────────────

pub struct VaultDb {
    conn: Connection,
}

impl VaultDb {
    /// Buka atau buat database di path yang diberikan
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db   = VaultDb { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Buat tabel jika belum ada
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS file_records (
                id              TEXT PRIMARY KEY NOT NULL,
                original_name   TEXT NOT NULL,
                original_path   TEXT NOT NULL,
                vault_filename  TEXT NOT NULL UNIQUE,
                sha256_hash     TEXT NOT NULL,
                file_size       INTEGER NOT NULL DEFAULT 0,
                iv_hex          TEXT NOT NULL,
                salt_hex        TEXT NOT NULL,
                encrypted_at    TEXT NOT NULL,
                is_deleted      BOOLEAN NOT NULL DEFAULT 0,
                deleted_at      TEXT,
                is_folder       BOOLEAN NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_encrypted_at
                ON file_records(encrypted_at DESC);

            CREATE TABLE IF NOT EXISTS vault_meta (
                key     TEXT PRIMARY KEY,
                value   TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action_type TEXT NOT NULL,
                description TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS folders (
                id              TEXT PRIMARY KEY NOT NULL,
                name            TEXT NOT NULL UNIQUE,
                icon            TEXT NOT NULL,
                color_hex       TEXT NOT NULL,
                created_at      TEXT NOT NULL
            );
        ")?;
        
        // Migrasi untuk tabel lama dengan memeriksa keberadaan kolom terlebih dahulu
        let mut has_is_deleted = false;
        let mut has_deleted_at = false;
        let mut has_is_folder = false;
        let mut has_folder_id = false;

        if let Ok(mut stmt) = self.conn.prepare("PRAGMA table_info(file_records)") {
            if let Ok(mut rows) = stmt.query([]) {
                while let Ok(Some(row)) = rows.next() {
                    if let Ok(col_name) = row.get::<_, String>(1) {
                        match col_name.as_str() {
                            "is_deleted" => has_is_deleted = true,
                            "deleted_at" => has_deleted_at = true,
                            "is_folder" => has_is_folder = true,
                            "folder_id" => has_folder_id = true,
                            _ => {}
                        }
                    }
                }
            }
        }

        if !has_is_deleted {
            let _ = self.conn.execute("ALTER TABLE file_records ADD COLUMN is_deleted BOOLEAN NOT NULL DEFAULT 0", []);
        }
        if !has_deleted_at {
            let _ = self.conn.execute("ALTER TABLE file_records ADD COLUMN deleted_at TEXT", []);
        }
        if !has_is_folder {
            let _ = self.conn.execute("ALTER TABLE file_records ADD COLUMN is_folder BOOLEAN NOT NULL DEFAULT 0", []);
        }
        if !has_folder_id {
            let _ = self.conn.execute("ALTER TABLE file_records ADD COLUMN folder_id TEXT", []);
        }

        // Seed folder bawaan jika masih kosong
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM folders", [], |row| row.get(0)).unwrap_or(0);
        if count == 0 {
            let default_folders = &[
                ("identitas", "Identitas", "💳", "#818cf8"),
                ("dokumen_kerja", "Dokumen Kerja", "💼", "#10b981"),
                ("foto_keluarga", "Foto Keluarga", "🖼️", "#f43f5e"),
                ("keuangan", "Keuangan", "📄", "#fbbf24"),
                ("kunci_akses", "Kunci & Akses", "🔑", "#38bdf8"),
            ];
            let now = "2026-05-30 00:00:00".to_string();
            for &(id, name, icon, color) in default_folders {
                let _ = self.conn.execute(
                    "INSERT INTO folders (id, name, icon, color_hex, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, name, icon, color, now],
                );
            }
        }
        
        Ok(())
    }

    // ── Vault Meta (PIN hash + salt) ──────────────────────

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT value FROM vault_meta WHERE key = ?1"
        )?;
        let result: rusqlite::Result<String> = stmt.query_row(params![key], |row| row.get(0));
        match result {
            Ok(v)                              => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e)                             => Err(e),
        }
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO vault_meta (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn is_user_set(&self) -> bool {
        self.get_meta("password_hash").unwrap_or(None).is_some()
    }

    /// Simpan data user: username, display_name, password hash + salt
    pub fn set_user(&self, username: &str, display_name: &str, password_hash: &str, salt_hex: &str) -> Result<()> {
        self.set_meta("username", username)?;
        self.set_meta("display_name", display_name)?;
        self.set_meta("password_hash", password_hash)?;
        self.set_meta("password_salt", salt_hex)?;
        Ok(())
    }

    pub fn get_username(&self) -> Result<Option<String>> {
        self.get_meta("username")
    }

    pub fn get_display_name(&self) -> Result<Option<String>> {
        self.get_meta("display_name")
    }

    pub fn get_password_hash(&self) -> Result<Option<String>> {
        self.get_meta("password_hash")
    }

    pub fn get_password_salt(&self) -> Result<Option<String>> {
        self.get_meta("password_salt")
    }

    /// Update password hash + salt
    pub fn update_password(&self, password_hash: &str, salt_hex: &str) -> Result<()> {
        self.set_meta("password_hash", password_hash)?;
        self.set_meta("password_salt", salt_hex)?;
        Ok(())
    }

    /// Reset seluruh database (Hapus semua record dan meta)
    pub fn reset_database(&self) -> Result<()> {
        self.conn.execute("DELETE FROM file_records", [])?;
        self.conn.execute("DELETE FROM vault_meta", [])?;
        // Kosongkan konfigurasi TOTP jika ada
        let _ = self.conn.execute("DELETE FROM totp_config", []);
        Ok(())
    }

    // ── File Records ──────────────────────────────────────

    pub fn insert_file(&self, record: &FileRecord) -> Result<()> {
        self.conn.execute(
            "INSERT INTO file_records
             (id, original_name, original_path, vault_filename,
              sha256_hash, file_size, iv_hex, salt_hex, encrypted_at, is_deleted, deleted_at, is_folder, folder_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                record.id,
                record.original_name,
                record.original_path,
                record.vault_filename,
                record.sha256_hash,
                record.file_size,
                record.iv_hex,
                record.salt_hex,
                record.encrypted_at,
                record.is_deleted,
                record.deleted_at,
                record.is_folder,
                record.folder_id,
            ],
        )?;
        Ok(())
    }

    pub fn get_all_files(&self) -> Result<Vec<FileRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, original_name, original_path, vault_filename,
                    sha256_hash, file_size, iv_hex, salt_hex, encrypted_at, is_deleted, deleted_at, is_folder, folder_id
             FROM file_records
             WHERE is_deleted = 0
             ORDER BY encrypted_at DESC"
        )?;

        let records = stmt.query_map([], |row| {
            Ok(FileRecord {
                id:             row.get(0)?,
                original_name:  row.get(1)?,
                original_path:  row.get(2)?,
                vault_filename: row.get(3)?,
                sha256_hash:    row.get(4)?,
                file_size:      row.get(5)?,
                iv_hex:         row.get(6)?,
                salt_hex:       row.get(7)?,
                encrypted_at:   row.get(8)?,
                is_deleted:     row.get(9)?,
                deleted_at:     row.get(10)?,
                is_folder:      row.get(11)?,
                folder_id:      row.get(12)?,
            })
        })?.collect::<Result<Vec<_>>>()?;

        Ok(records)
    }

    pub fn permanent_delete_file(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM file_records WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn rename_file(&self, id: &str, new_name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE file_records SET original_name = ?2 WHERE id = ?1",
            params![id, new_name],
        )?;
        Ok(())
    }

    pub fn soft_delete_file(&self, id: &str, deleted_at: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE file_records SET is_deleted = 1, deleted_at = ?2 WHERE id = ?1",
            params![id, deleted_at],
        )?;
        Ok(())
    }

    pub fn restore_file(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE file_records SET is_deleted = 0, deleted_at = NULL WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn update_file_folder(&self, id: &str, folder_id: Option<&str>) -> Result<()> {
        self.conn.execute(
            "UPDATE file_records SET folder_id = ?2 WHERE id = ?1",
            params![id, folder_id],
        )?;
        Ok(())
    }

    pub fn get_deleted_files(&self) -> Result<Vec<FileRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, original_name, original_path, vault_filename,
                    sha256_hash, file_size, iv_hex, salt_hex, encrypted_at, is_deleted, deleted_at, is_folder, folder_id
             FROM file_records
             WHERE is_deleted = 1
             ORDER BY deleted_at DESC"
        )?;

        let records = stmt.query_map([], |row| {
            Ok(FileRecord {
                id:             row.get(0)?,
                original_name:  row.get(1)?,
                original_path:  row.get(2)?,
                vault_filename: row.get(3)?,
                sha256_hash:    row.get(4)?,
                file_size:      row.get(5)?,
                iv_hex:         row.get(6)?,
                salt_hex:       row.get(7)?,
                encrypted_at:   row.get(8)?,
                is_deleted:     row.get(9)?,
                deleted_at:     row.get(10)?,
                is_folder:      row.get(11)?,
                folder_id:      row.get(12)?,
            })
        })?.collect::<Result<Vec<_>>>()?;

        Ok(records)
    }

    #[allow(dead_code)]
    pub fn find_by_vault_filename(&self, vault_filename: &str) -> Result<Option<FileRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, original_name, original_path, vault_filename,
                    sha256_hash, file_size, iv_hex, salt_hex, encrypted_at, is_deleted, deleted_at, is_folder, folder_id
             FROM file_records WHERE vault_filename = ?1"
        )?;

        let result = stmt.query_row(params![vault_filename], |row| {
            Ok(FileRecord {
                id:             row.get(0)?,
                original_name:  row.get(1)?,
                original_path:  row.get(2)?,
                vault_filename: row.get(3)?,
                sha256_hash:    row.get(4)?,
                file_size:      row.get(5)?,
                iv_hex:         row.get(6)?,
                salt_hex:       row.get(7)?,
                encrypted_at:   row.get(8)?,
                is_deleted:     row.get(9)?,
                deleted_at:     row.get(10)?,
                is_folder:      row.get(11)?,
                folder_id:      row.get(12)?,
            })
        });

        match result {
            Ok(r)                              => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e)                             => Err(e),
        }
    }

    /// Eksekusi SQL statement langsung (untuk operasi admin)
    pub fn conn_exec(&self, sql: &str) -> Result<()> {
        self.conn.execute_batch(sql)?;
        Ok(())
    }

    pub fn get_file(&self, vault_filename: &str) -> Result<Option<FileRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, original_name, original_path, vault_filename, sha256_hash, file_size, iv_hex, salt_hex, encrypted_at, is_deleted, deleted_at, is_folder, folder_id 
             FROM file_records WHERE vault_filename = ?1 AND is_deleted = 0"
        )?;
        let mut rows = stmt.query([vault_filename])?;
        if let Some(row) = rows.next()? {
            Ok(Some(FileRecord {
                id: row.get(0)?,
                original_name: row.get(1)?,
                original_path: row.get(2)?,
                vault_filename: row.get(3)?,
                sha256_hash: row.get(4)?,
                file_size: row.get(5)?,
                iv_hex: row.get(6)?,
                salt_hex: row.get(7)?,
                encrypted_at: row.get(8)?,
                is_deleted: row.get(9)?,
                deleted_at: row.get(10)?,
                is_folder: row.get(11)?,
                folder_id: row.get(12)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_folders(&self) -> Result<Vec<FolderRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, icon, color_hex, created_at FROM folders ORDER BY created_at ASC"
        )?;
        let records = stmt.query_map([], |row| {
            Ok(FolderRecord {
                id:             row.get(0)?,
                name:           row.get(1)?,
                icon:           row.get(2)?,
                color_hex:      row.get(3)?,
                created_at:     row.get(4)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        Ok(records)
    }

    pub fn insert_folder(&self, record: &FolderRecord) -> Result<()> {
        self.conn.execute(
            "INSERT INTO folders (id, name, icon, color_hex, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![record.id, record.name, record.icon, record.color_hex, record.created_at],
        )?;
        Ok(())
    }

    // ── Update ────────────────────────────────────────────────

    pub fn insert_audit_log(&self, action_type: &str, description: &str, timestamp: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO audit_logs (action_type, description, timestamp) VALUES (?1, ?2, ?3)",
            params![action_type, description, timestamp],
        )?;
        Ok(())
    }

    pub fn get_all_audit_logs(&self) -> Result<Vec<AuditLog>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, action_type, description, timestamp
             FROM audit_logs
             ORDER BY id DESC LIMIT 100"
        )?;

        let logs = stmt.query_map([], |row| {
            Ok(AuditLog {
                id:          row.get(0)?,
                action_type: row.get(1)?,
                description: row.get(2)?,
                timestamp:   row.get(3)?,
            })
        })?.collect::<Result<Vec<_>>>()?;

        Ok(logs)
    }
}