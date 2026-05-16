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
                deleted_at      TEXT
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
        ")?;
        
        // Migrasi untuk tabel lama
        let _ = self.conn.execute("ALTER TABLE file_records ADD COLUMN is_deleted BOOLEAN NOT NULL DEFAULT 0", []);
        let _ = self.conn.execute("ALTER TABLE file_records ADD COLUMN deleted_at TEXT", []);
        
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

    pub fn is_pin_set(&self) -> bool {
        self.get_meta("pin_hash").unwrap_or(None).is_some()
    }

    /// Simpan hash PIN + salt untuk verifikasi login
    pub fn set_pin(&self, pin_hash: &str, salt_hex: &str) -> Result<()> {
        self.set_meta("pin_hash",  pin_hash)?;
        self.set_meta("pin_salt",  salt_hex)?;
        Ok(())
    }

    pub fn get_pin_hash(&self) -> Result<Option<String>> {
        self.get_meta("pin_hash")
    }

    pub fn get_pin_salt(&self) -> Result<Option<String>> {
        self.get_meta("pin_salt")
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
              sha256_hash, file_size, iv_hex, salt_hex, encrypted_at, is_deleted, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
            ],
        )?;
        Ok(())
    }

    pub fn get_all_files(&self) -> Result<Vec<FileRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, original_name, original_path, vault_filename,
                    sha256_hash, file_size, iv_hex, salt_hex, encrypted_at, is_deleted, deleted_at
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

    pub fn get_deleted_files(&self) -> Result<Vec<FileRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, original_name, original_path, vault_filename,
                    sha256_hash, file_size, iv_hex, salt_hex, encrypted_at, is_deleted, deleted_at
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
            })
        })?.collect::<Result<Vec<_>>>()?;

        Ok(records)
    }

    #[allow(dead_code)]
    pub fn find_by_vault_filename(&self, vault_filename: &str) -> Result<Option<FileRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, original_name, original_path, vault_filename,
                    sha256_hash, file_size, iv_hex, salt_hex, encrypted_at, is_deleted, deleted_at
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

    // ── Audit Logs ────────────────────────────────────────

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