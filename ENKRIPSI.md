# 🔐 Dokumentasi Enkripsi DataVault (Aegis Vault)

> **Catatan:** Dokumen ini hanya menjelaskan cara kerja sistem enkripsi yang sudah ada di dalam kode.
> Tidak ada perubahan kode yang dilakukan.

---

## 📋 Daftar Isi

1. [Gambaran Umum Sistem Keamanan](#1-gambaran-umum)
2. [Metode Enkripsi yang Digunakan](#2-metode-enkripsi)
3. [Alur Lengkap: Dari Upload sampai Tersimpan](#3-alur-enkripsi-upload)
4. [Alur Lengkap: Dari Vault sampai File Dipulihkan](#4-alur-dekripsi-pemulihan)
5. [Key Derivation: Cara Password Jadi Kunci AES](#5-key-derivation)
6. [Struktur File .vlt](#6-struktur-file-vlt)
7. [Penyimpanan Metadata di Database](#7-database-sqlite)
8. [Secure Delete: Hapus File Asli dengan Aman](#8-secure-delete)
9. [Autentikasi 2 Lapis: Login + TOTP](#9-autentikasi-2fa)
10. [Anti-Tampering: Deteksi Root & Emulator](#10-anti-tampering)
11. [P2P Sharing: Berbagi File Terenkripsi](#11-p2p-sharing)
12. [Ringkasan Alat Keamanan](#12-ringkasan)

---

## 1. Gambaran Umum

DataVault (Aegis Vault) adalah aplikasi **brankas file terenkripsi** yang ditulis dalam bahasa Rust. Setiap file yang kamu masukkan ke vault akan:

1. **Dienkripsi** menggunakan AES-256-CBC secara block-by-block
2. **Dinamakan ulang** menjadi nama acak dengan ekstensi `.vlt`
3. **File aslinya dihapus** secara aman (3-pass overwrite)
4. **Metadatanya disimpan** di database SQLite lokal
5. **Tidak bisa dibuka** tanpa password yang benar

Arsitektur kode terdiri dari:

| File | Fungsi |
|------|--------|
| `crypto.rs` | Engine enkripsi/dekripsi AES-256-CBC + Argon2id |
| `controller.rs` | Logika bisnis: login, enkripsi, dekripsi |
| `db.rs` | Penyimpanan metadata di SQLite |
| `totp.rs` | Autentikasi dua faktor (2FA) berbasis waktu |
| `anti_tamper.rs` | Deteksi Root & Emulator |
| `file_handler.rs` | UI unlock ketika file .vlt dibuka langsung |
| `app_state.rs` | Penyimpanan sesi kunci enkripsi di memori |

---

## 2. Metode Enkripsi

Sistem ini menggunakan **kombinasi beberapa algoritma kriptografi**:

### 2.1 AES-256-CBC (Advanced Encryption Standard)
- **Kunci**: 256-bit (32 byte)
- **Mode**: CBC (Cipher Block Chaining)
- **Block size**: 16 byte
- **Padding**: PKCS#7
- **Library**: crate `aes` (implementasi manual block-by-block)

### 2.2 Argon2id (Key Derivation Function)
- Mengubah password teks menjadi kunci AES 256-bit
- **Rekomendasi OWASP 2024** untuk hashing password
- Tahan terhadap serangan brute-force GPU/ASIC
- Parameter: default Argon2id (time cost, memory cost, parallelism)

### 2.3 SHA-256 (Hash Integrity Check)
- Menghitung hash file terenkripsi untuk verifikasi integritas
- Hash disimpan di database
- Sebelum dekripsi, hash file `.vlt` diperiksa ulang
- Jika hash tidak cocok → file ditolak (kemungkinan dimanipulasi)

### 2.4 HMAC-SHA1 (TOTP 2FA)
- Digunakan untuk menghasilkan kode OTP 6 digit
- Standar RFC 6238 (kompatibel Google Authenticator)
- Toleransi ±4 time step (±120 detik) untuk perbedaan jam

---

## 3. Alur Enkripsi (Upload)

Berikut adalah langkah detail ketika kamu mengupload/mengamankan sebuah file:

```
[File Asli] 
    │
    ▼
[STEP 1: Validasi Sesi]
    - Controller cek session_key di AppState
    - session_key adalah kunci AES 256-bit yang sudah di-derive dari password saat login
    - Jika tidak ada sesi valid → proses dibatalkan
    │
    ▼
[STEP 2: Cek Apakah Folder/File]
    - Jika FOLDER → dikompres dulu menjadi .zip sementara (di temp dir)
    - Jika FILE biasa → langsung diproses
    │
    ▼
[STEP 3: Generate IV Acak (16 byte)]
    - thread_rng().fill_bytes(&mut iv)  ← menggunakan CSPRNG (Cryptographically Secure RNG)
    - IV (Initialization Vector) berbeda untuk setiap file
    - IV digunakan di mode CBC agar enkripsi tidak deterministik
    │
    ▼
[STEP 4: Generate Nama File Acak]
    - thread_rng().fill_bytes(&mut random_bytes)  ← 16 byte acak
    - Dikonversi ke hex string → nama file 32 karakter hex
    - Nama akhir: "[32 hex chars].vlt"
    - Contoh: "a3f91bc204e78d1a...45f.vlt"
    │
    ▼
[STEP 5: Enkripsi AES-256-CBC Block-by-Block]
    
    Persiapan:
    - AES encryptor diinisialisasi dengan session_key (32 byte)
    - prev_iv = IV awal (16 byte)
    - Buffer baca: 64 KB per iterasi (optimal untuk cache L2 ARM)
    
    Tulis header ke file .vlt:
    - 16 byte pertama = IV (disimpan plaintext di awal file)
    
    Loop enkripsi:
    ┌─────────────────────────────────────────────────────────┐
    │  Baca buffer (maks 64 KB) dari file asli               │
    │                                                         │
    │  Untuk setiap block 16 byte:                           │
    │    1. XOR plaintext_block dengan prev_iv  ← CBC step  │
    │    2. Enkripsi blok dengan AES-256         ← AES step  │
    │    3. Hasil enkripsi jadi prev_iv baru                 │
    │    4. Tulis ciphertext ke file .vlt                    │
    │                                                         │
    │  Pada block TERAKHIR (chunk tidak penuh):              │
    │    - Hitung sisa byte (rem = bytes % 16)               │
    │    - Buat padding PKCS#7:                              │
    │        pad_len = 16 - rem                              │
    │        isi byte padding = pad_len                      │
    │    - Enkripsi block terakhir dengan padding            │
    └─────────────────────────────────────────────────────────┘
    │
    ▼
[STEP 6: Hitung SHA-256 Hash File Terenkripsi]
    - hasher.update() dipanggil setiap kali data ditulis ke .vlt
    - file_hash = hex::encode(hasher.finalize())
    - Hash ini mencakup: IV header + semua ciphertext
    │
    ▼
[STEP 7: Secure Delete File Asli (3-Pass)]
    Pass 1: Timpa seluruh konten dengan 0x00 (nol)
    Pass 2: Timpa seluruh konten dengan 0xFF (satu)  
    Pass 3: Timpa seluruh konten dengan byte ACAK
    → Kemudian hapus file
    - Tujuan: mencegah recovery forensik file asli
    │
    ▼
[STEP 8: Simpan Metadata ke SQLite]
    Record yang disimpan:
    - id            : UUID unik
    - original_name : nama file asli (misal: "foto_ktp.jpg")
    - original_path : path asli sebelum dienkripsi
    - vault_filename: nama file .vlt yang dihasilkan
    - sha256_hash   : hash file .vlt untuk verifikasi integritas
    - file_size     : ukuran file asli (dalam byte)
    - iv_hex        : IV dalam format hex string (16 byte = 32 char hex)
    - salt_hex      : salt Argon2id sesi ini (16 byte = 32 char hex)
    - encrypted_at  : timestamp enkripsi
    │
    ▼
[File .vlt tersimpan di vault_storage/]
[File asli sudah terhapus secara aman]
```

---

## 4. Alur Dekripsi (Pemulihan)

Ketika kamu ingin memulihkan file dari vault:

```
[Pengguna Pilih File di Vault]
    │
    ▼
[STEP 1: Ambil Record dari Database]
    - Cari record berdasarkan vault_filename
    - Ambil: sha256_hash, iv_hex, salt_hex, original_name
    │
    ▼
[STEP 2: Validasi Integritas File .vlt SEBELUM Dekripsi]
    - Hitung ulang SHA-256 dari file .vlt yang ada di disk
    - Bandingkan dengan sha256_hash di database
    - JIKA TIDAK COCOK → tolak, tampilkan error "Integritas file gagal"
    - JIKA COCOK → lanjut
    
    Ini melindungi dari:
    → File yang dimanipulasi/dirusak orang lain
    → Bit rot / korupsi storage
    │
    ▼
[STEP 3: Baca IV dari 16 Byte Pertama File .vlt]
    - vault_file.read_exact(&mut iv)
    - IV diperlukan untuk memulai dekripsi CBC
    │
    ▼
[STEP 4: Baca Seluruh Ciphertext (setelah byte ke-16)]
    - vault_file.read_to_end(&mut ciphertext)
    - Validasi: panjang ciphertext harus kelipatan 16
    │
    ▼
[STEP 5: Dekripsi AES-256-CBC Block-by-Block]
    
    - AES decryptor diinisialisasi dengan session_key (32 byte)
    - prev_iv = IV yang dibaca dari header
    
    Untuk setiap block 16 byte:
    ┌─────────────────────────────────────────────────────────┐
    │  1. Simpan cipher_block saat ini sebagai next_prev_iv  │
    │  2. Dekripsi cipher_block dengan AES-256               │
    │  3. XOR hasil dekripsi dengan prev_iv  ← CBC step     │
    │  4. Hasil adalah plaintext_block                       │
    │  5. Update prev_iv = cipher_block                      │
    └─────────────────────────────────────────────────────────┘
    │
    ▼
[STEP 6: Hapus PKCS#7 Padding]
    - Baca byte terakhir plaintext → nilai = pad_len
    - Verifikasi semua pad_len byte terakhir bernilai pad_len
    - JIKA TIDAK VALID → error "Padding tidak valid / kunci salah"
    - Potong plaintext hingga pad_start
    │
    ▼
[STEP 7: Tulis ke File Output]
    - Buat file output di lokasi yang dipilih pengguna
    - Tulis plaintext yang sudah bersih
    - Jika file asli adalah folder → unzip .zip ke direktori tujuan
    │
    ▼
[File Original Berhasil Dipulihkan ✅]
```

---

## 5. Key Derivation: Cara Password Menjadi Kunci AES

Ini adalah proses paling penting dalam sistem keamanan ini.

```
[Password Pengguna: "contohPassword123"]
              │
              ▼
    ┌─────────────────────┐
    │   generate_salt()   │  ← 16 byte acak dari CSPRNG
    │   [0xA3, 0x7F, ...]│
    └─────────────────────┘
              │
              ▼
    ┌─────────────────────────────────────────────────────┐
    │              ARGON2id KDF                           │
    │                                                     │
    │  Input : password bytes + salt (16 byte)           │
    │  Output: 32 byte kunci (KEY_LEN = 32)              │
    │                                                     │
    │  Mengapa Argon2id?                                  │
    │  - Memory-hard: butuh banyak RAM untuk dihitung    │
    │  - Lambat secara by-design → susah brute-force     │
    │  - Standar OWASP 2024                              │
    └─────────────────────────────────────────────────────┘
              │
              ▼
    [32 byte kunci AES-256]
    [Disimpan di AppState.session_key]
    [Di-zeroize dari memori saat logout]
```

### Hash untuk Verifikasi Password (Login):

```
password + salt
      │
      ▼
  Argon2id → 32 byte kunci
      │
      ▼
  SHA-256(kunci) → hex string
      │
      ▼
  Disimpan di vault_meta DB sebagai "password_hash"
```

Saat login:
1. Ambil `password_salt` dari DB
2. Hitung `Argon2id(password_input, salt)` → kunci baru
3. Hitung `SHA-256(kunci_baru)` → hash_input
4. Bandingkan `hash_input` dengan `password_hash` di DB
5. Jika cocok → kunci langsung digunakan sebagai `session_key`

---

## 6. Struktur File .vlt

Setiap file terenkripsi memiliki format berikut:

```
┌──────────────────────────────────────────────┐
│           FILE FORMAT: .vlt                  │
├──────────────────────────────────────────────┤
│  Byte 0–15  : IV (Initialization Vector)     │
│               16 byte, plaintext             │
│               Dibutuhkan untuk CBC           │
├──────────────────────────────────────────────┤
│  Byte 16–N  : CIPHERTEXT                     │
│               Hasil enkripsi AES-256-CBC     │
│               Panjang = kelipatan 16 byte    │
│               Byte terakhir: PKCS#7 padding  │
└──────────────────────────────────────────────┘
```

**Contoh nyata:**
- File asli: `foto_ktp.jpg` (50.000 byte)
- Setelah enkripsi: `a3f91bc2...45f.vlt` (50.016 byte)
  - 16 byte IV + 50.000 byte → padded ke 50.000 byte (kelipatan 16) → total isi cipher = 50.000 byte + 1 padding block
  - File .vlt = 16 (IV) + 50.000 + 16 (padding block) = 50.032 byte

---

## 7. Database SQLite

Metadata semua file tersimpan di `vault_storage/vault.db`.

### Tabel `file_records`

| Kolom | Tipe | Keterangan |
|-------|------|------------|
| `id` | TEXT | UUID unik per file |
| `original_name` | TEXT | Nama file asli |
| `original_path` | TEXT | Path file sebelum dienkripsi |
| `vault_filename` | TEXT | Nama file .vlt (UNIQUE) |
| `sha256_hash` | TEXT | Hash SHA-256 dari file .vlt |
| `file_size` | INTEGER | Ukuran file asli (byte) |
| `iv_hex` | TEXT | IV dalam hex (32 char) |
| `salt_hex` | TEXT | Salt Argon2id dalam hex (32 char) |
| `encrypted_at` | TEXT | Timestamp enkripsi |
| `is_deleted` | BOOLEAN | Soft delete (recycle bin) |
| `folder_id` | TEXT | ID folder kategori |

### Tabel `vault_meta`

| Key | Value |
|-----|-------|
| `password_hash` | SHA-256(Argon2id(password, salt)) |
| `password_salt` | Salt Argon2id dalam hex |
| `pin_hash` | Hash PIN (untuk unlock cepat) |
| `pin_salt` | Salt PIN |
| `totp_secret` | Secret TOTP dalam Base32 |

> ⚠️ **Kunci AES TIDAK PERNAH disimpan di database.**
> Database hanya menyimpan hash (tidak bisa dibalik ke password).

---

## 8. Secure Delete (Penghapusan Aman 3-Pass)

Fungsi `secure_delete()` di `crypto.rs` memastikan file asli tidak bisa dipulihkan secara forensik.

```
[File Asli: foto_ktp.jpg]
        │
        ▼
  PASS 1: Timpa dengan 0x00 0x00 0x00 0x00...
  PASS 2: Timpa dengan 0xFF 0xFF 0xFF 0xFF...
  PASS 3: Timpa dengan byte ACAK (CSPRNG)
        │
        ▼
  std::fs::remove_file(path)
        │
        ▼
  [File tidak dapat dipulihkan]
```

Jika 3-pass gagal (karena batasan izin OS), sistem fallback ke `remove_file` biasa, lalu menandai `original_deleted = false` di hasil enkripsi. Dalam kasus ini, pengguna diberitahu dengan pesan peringatan.

---

## 9. Autentikasi 2 Lapis (Login + TOTP 2FA)

### Layer 1: Username + Password

```
Login Input (username + password)
        │
        ▼
  Cek username di DB (harus sama persis)
        │
        ▼
  hash_pin(password_input, salt_dari_db)
        │
        ▼
  Bandingkan dengan password_hash di DB
        │
  Jika cocok → derive session_key dari password
  Jika TOTP aktif → arahkan ke layar verifikasi 2FA
  Jika TOTP tidak aktif → langsung ke Dashboard
```

### Layer 2: TOTP (Time-based One-Time Password)

```
[Secret 20-byte acak] 
        │
        ▼
  Encode ke Base32 → disimpan di DB sebagai "totp_secret"
        │
        ▼
  otpauth:// URI → ditampilkan sebagai QR Code
        │
  [User scan dengan Google Authenticator / Authy]
        │
        ▼
  Saat login, user masukkan 6-digit kode
        │
        ▼
  verify(secret, kode):
    timestamp_sekarang / 30 = counter
    HMAC-SHA1(secret, counter) → truncate → 6 digit
    Cek ±4 time step (±120 detik) untuk toleransi jam
        │
  Jika cocok → akses diberikan ke Dashboard
```

**Cara kerja TOTP detail:**
```
counter = unix_timestamp / 30
HMAC = HMAC-SHA1(secret_bytes, counter.to_be_bytes())
offset = HMAC[19] & 0x0F
bin = (HMAC[offset] & 0x7F) << 24
     | HMAC[offset+1] << 16
     | HMAC[offset+2] << 8
     | HMAC[offset+3]
kode = bin % 1_000_000  → diformat jadi "006789"
```

---

## 10. Anti-Tampering: Deteksi Root & Emulator

`anti_tamper.rs` memeriksa apakah aplikasi berjalan di lingkungan yang tidak aman.

### Yang Diperiksa:

**Root Detection:**
- Keberadaan file: `/sbin/su`, `/system/bin/su`, `/system/xbin/su`, dll.
- Build tag `test-keys` di `ro.build.tags`

**Emulator Detection:**
- File QEMU/emulator: `/dev/qemu_pipe`, `/sys/qemu_trace`, dll.
- Property sistem: `ro.hardware` (goldfish/ranchu/vbox86)
- `ro.kernel.qemu = 1`
- Model device mengandung kata "sdk", "emulator", "genymotion"

Jika pelanggaran terdeteksi → `security_violation` di-set di `AppState` dan akses ditolak.

---

## 11. P2P Wi-Fi Sharing (Berbagi File Terenkripsi)

Fitur sharing memungkinkan berbagi file ke perangkat lain di jaringan lokal yang sama, **tanpa mengirim data ke internet**.

```
[User pilih file untuk dibagikan]
        │
        ▼
  Generate PIN 4-digit acak (1000–9999)
  Bind TCP listener ke port acak (0.0.0.0:0)
        │
        ▼
  Tampilkan: IP lokal + Port + PIN ke pengguna
  (misal: http://192.168.1.5:54321?pin=4782)
        │
        ▼
  [Penerima buka URL di browser]
        │
        ▼
  Server verifikasi PIN dari query string
  Jika PIN benar:
    → decrypt_to_memory() dipanggil
    → File didekripsi ke RAM (TIDAK ke disk)
    → Dikirim langsung via HTTP response
  Jika PIN salah → tampilkan halaman error
```

Penting: Dekripsi dilakukan di RAM (`decrypt_to_memory`), file tidak pernah ditulis ke disk sementara saat sharing.

---

## 12. Ringkasan Alat Keamanan

| Komponen | Algoritma | Kekuatan |
|----------|-----------|----------|
| Enkripsi data | AES-256-CBC | 256-bit key, industri standar |
| Derivasi kunci | Argon2id | Memory-hard, OWASP 2024 |
| Verifikasi password | SHA-256(Argon2id key) | Tidak reversible |
| Integritas file | SHA-256 | Deteksi manipulasi |
| 2FA | HMAC-SHA1 TOTP | RFC 6238, kompatibel GA |
| Hapus aman | 3-Pass overwrite | Mencegah forensik recovery |
| Keamanan memori | `zeroize` crate | Kunci dihapus dari RAM setelah pakai |
| Nama file | 16-byte random hex | Tidak mengungkap konten |
| IV enkripsi | 16-byte CSPRNG | Unik per file, anti-pattern attack |

### Kunci TIDAK Pernah:
- ❌ Disimpan ke disk dalam bentuk plaintext
- ❌ Dikirim ke server manapun
- ❌ Tersimpan di database
- ✅ Hanya ada di RAM selama sesi aktif (`session_key`)
- ✅ Di-zeroize dari memori saat logout

---

*Dokumentasi ini dibuat berdasarkan analisis kode sumber DataVault (Aegis Vault) — Rust implementation.*
*File: `crypto.rs`, `controller.rs`, `db.rs`, `totp.rs`, `anti_tamper.rs`, `file_handler.rs`, `app_state.rs`*
