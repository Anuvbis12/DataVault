<div align="center">
  <h1>🛡️ Aegis Vault 🛡️</h1>
  <p><strong>Akses Aman ke Data Anda</strong></p>
  
  [![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
  [![GUI](https://img.shields.io/badge/GUI-egui-blue.svg)](https://github.com/emilk/egui)
  [![Security](https://img.shields.io/badge/Security-AES--256-green.svg)](#)
  [![License](https://img.shields.io/badge/License-MIT-lightgray.svg)](#)
</div>

---

**Aegis Vault** adalah aplikasi desktop modern dan aman yang dirancang untuk menjadi "brankas" digital bagi file-file rahasia Anda. Dibangun menggunakan performa tangguh bahasa **Rust** dan dibalut antarmuka cantik *glassmorphism* dari **`egui`**, aplikasi ini menjaga kerahasiaan data Anda tanpa mengorbankan pengalaman pengguna.

![Aegis Vault Demo](https://via.placeholder.com/800x450.png?text=Aegis+Vault+UI+Screenshot) <!-- Ganti URL ini dengan screenshot UI aplikasimu nanti -->

---

## ✨ Fitur Unggulan

🛡️ **Keamanan Kelas Militer**
Setiap file dienkripsi menggunakan standar **AES-256**. Kunci enkripsi didapatkan menggunakan metode **PBKDF2-HMAC-SHA256** dan *salting* acak, mencegah serangan *brute-force*.

🔑 **Akses Berbasis PIN yang Simpel**
Lupakan password yang panjang dan rumit untuk akses cepat! Cukup gunakan PIN Anda untuk membuka vault. PIN Anda tidak pernah disimpan dalam bentuk *plaintext* (selalu di-hash).

✅ **Validasi Integritas Data**
Aegis Vault memvalidasi file menggunakan hash **SHA-256** sebelum proses dekripsi. Ini memastikan tidak ada satupun *byte* yang rusak atau dimodifikasi oleh pihak tak bertanggung jawab.

🗄️ **Manajemen File Cerdas**
Semua rekaman dan metadata file disimpan dengan rapi di dalam database lokal **SQLite**, memudahkan pencarian dan pengelolaan tanpa membahayakan data aslinya.

🔥 **Penghapusan Ekstra Aman (Secure Delete)** *(Coming Soon / Built-in)*
Saat Anda memasukkan file ke dalam vault, file sumber akan melalui proses *3-pass wipe* agar jejak digital aslinya tidak bisa dipulihkan menggunakan *recovery tools*.

---

## 🚀 Cara Menjalankan Aplikasi

Ikuti panduan mudah ini untuk mulai mengamankan file Anda!

### 1. Persiapan Sistem (Prerequisites)
Pastikan Anda sudah menginstal **Rust** dan **Cargo**. Jika belum, instal melalui [rustup.rs](https://rustup.rs/):

```bash
# Untuk Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
*(Untuk Windows, cukup unduh installer `rustup-init.exe` dari website resminya).*

### 2. Kloning Repositori & Instalasi
Buka terminal/CMD/PowerShell, lalu jalankan:

```bash
# 1. Kloning repositori (atau masuk ke folder jika sudah diunduh)
git clone https://github.com/username_kamu/AegisVault.git
cd AegisVault

# 2. Build proyek untuk mengunduh semua dependency (Opsional, tapi disarankan)
cargo build --release
```

### 3. Menjalankan Aegis Vault
Sangat disarankan untuk selalu menjalankan aplikasi ini dalam mode `--release`. Operasi kriptografi (seperti derivasi kunci PBKDF2) membutuhkan proses komputasi berat; mode *release* membuat aplikasi berjalan **ratusan kali lebih cepat** dibandingkan mode *debug*.

```bash
# Jalankan aplikasi!
cargo run --release
```

---

## 🔒 Panduan Penggunaan (Quick Start)

1. **Setup Awal**: Saat pertama kali aplikasi terbuka, Anda akan diminta untuk membuat **PIN** (minimal 4 digit angka). *Jangan sampai lupa! PIN ini adalah kunci utama Anda.*
2. **Dashboard**: Setelah masuk, klik tombol melayang **➕** di sudut kanan bawah.
3. **Pilih File**: Pilih file apapun (Dokumen, Foto, Video) yang ingin Anda sembunyikan. File tersebut akan terenkripsi dan disimpan di folder `vault_storage/`.
4. **Pulihkan (Decrypt)**: Untuk mengambil file Anda kembali, cukup klik tombol gembok terbuka (🔓) di sebelah nama file pada daftar. Pilih direktori tujuan, dan file akan kembali ke bentuk aslinya!

---

## 📂 Struktur Direktori & Perhatian Penting

Saat aplikasi pertama kali mendeteksi file, ia akan membuat direktori lokal:
```text
📦 AegisVault
 ┣ 📂 src/
 ┣ 📂 vault_storage/      <-- TEMPAT FILE TERENKRIPSI DISIMPAN
 ┃  ┣ 📜 vault.db         <-- Database metadata (Jangan dihapus!)
 ┃  ┗ 📜 <uuid_file>      <-- File Anda yang sudah diamankan
 ┗ 📜 Cargo.toml
```

> ⚠️ **PERINGATAN KRITIS**: **DILARANG KERAS** menghapus, memindahkan, atau mengubah nama file apapun di dalam folder `vault_storage/` secara manual melalui *File Explorer*. Hal ini dapat merusak struktur database dan mengakibatkan data Anda hilang secara **permanen**.

---

## 🔧 Troubleshooting Git (Khusus Developer)

Jika Anda berkolaborasi menggunakan Git dan menemui error saat `git pull` atau `git merge` seperti:
`error: Your local changes to the following files would be overwritten by merge: target/...` atau `vault_storage/vault.db`

**Penyebabnya:** File *build* (folder `target/`) atau *database lokal* Anda (`vault_storage/`) bertabrakan dengan repositori.

**Solusinya:**
Pastikan folder `target/` dan `vault_storage/` sudah ada di dalam file `.gitignore`. Jika error sudah terlanjur terjadi, simpan perubahan lokal Anda sementara menggunakan stash sebelum pull:
```bash
git stash
git pull
git stash pop
```

---

## 🛠️ Dibangun Dengan

- [**eframe** & **egui**](https://github.com/emilk/egui) - GUI Framework yang *Fast & Immediate Mode*
- [**RustCrypto**](https://github.com/RustCrypto) - *Crates* kriptografi murni (`aes`, `pbkdf2`, `sha2`, dll.)
- [**rusqlite**](https://github.com/rusqlite/rusqlite) - SQLite binding untuk Rust
- [**rfd**](https://github.com/PolyMeilex/rfd) - Dialog file *native* lintas platform

---

<div align="center">
  Dibuat dengan ❤️ dan 🦀 (Rust) | Open Source
</div>
