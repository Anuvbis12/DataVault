<div align="center">

<img src="https://capsule-render.vercel.app/api?type=waving&color=CBA6F7&height=250&section=header&text=Aegis%20Vault&fontSize=70&fontAlignY=40&desc=Keamanan%20Privasi%20Tingkat%20Tinggi&descAlignY=65&descAlign=50&animation=twinkling" />

# 🛡️ DataVault (Aegis Vault)

[![Typing SVG](https://readme-typing-svg.herokuapp.com?font=Inter&weight=800&size=28&pause=1000&color=CBA6F7&center=true&vCenter=true&width=600&lines=Keamanan+Privasi+Tingkat+Tinggi;Standar+Enkripsi+AES-256;Brankas+Digital+Teraman+Anda;Cepat,+Ringan,+dan+Responsif!;Cross-Platform+Support+🚀)](https://git.io/typing-svg)

**Akses Aman & Privasi Penuh untuk Data Anda**

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.70+-E34F26.svg?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"></a>
  <a href="https://github.com/emilk/egui"><img src="https://img.shields.io/badge/GUI-egui-00C7B7.svg?style=for-the-badge" alt="egui"></a>
  <a href="#"><img src="https://img.shields.io/badge/Security-AES--256-4CAF50.svg?style=for-the-badge&logo=security" alt="Security"></a>
  <a href="#"><img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Android%20%7C%20iOS-0078D6.svg?style=for-the-badge&logo=windows&logoColor=white" alt="Platform"></a>
  <a href="#"><img src="https://img.shields.io/badge/License-MIT-lightgray.svg?style=for-the-badge" alt="License"></a>
</p>

*Brankas digital modern yang memadukan performa tangguh Rust dengan antarmuka memukau.*

**[📥 Unduh Rilis Terbaru](#) • [🚀 Cara Penggunaan](#-cara-penggunaan) • [🐛 Laporkan Bug](#)**

<br/>

<img src="https://user-images.githubusercontent.com/74038190/212284100-561aa473-3905-4a80-b561-0d28506553ee.gif" width="600">

</div>

---

<details open>
<summary><b>📑 Daftar Isi</b> (Klik untuk menyembunyikan/menampilkan)</summary>

- [🌟 Tentang Proyek](#-tentang-proyek)
- [✨ Fitur Unggulan](#-fitur-unggulan)
- [🚀 Panduan Memulai (Quick Start)](#-panduan-memulai-quick-start)
- [📖 Cara Penggunaan](#-cara-penggunaan)
- [📂 Struktur Penyimpanan](#-struktur-penyimpanan)
- [🛠️ Teknologi yang Digunakan](#️-teknologi-yang-digunakan)
- [👥 Tim Pengembang](#-tim-pengembang)

</details>

---

## 🌟 Tentang Proyek

**DataVault (Aegis Vault)** adalah aplikasi desktop modern yang dirancang untuk menjadi "brankas" digital bagi file-file rahasia Anda. Dibangun menggunakan performa tinggi dari bahasa pemrograman **Rust** dan dibalut dengan antarmuka interaktif dari **`egui`**, aplikasi ini menjaga kerahasiaan data Anda tanpa mengorbankan pengalaman pengguna.

> 💡 **Misi Kami:** Menyediakan alat keamanan data tingkat tinggi yang mudah digunakan oleh siapa saja, dengan antarmuka yang bersih, cepat, dan responsif.

<div align="right">
  <a href="#-datavault-aegis-vault">⬆ Kembali ke Atas</a>
</div>

---

## ✨ Fitur Unggulan

| Fitur | Deskripsi |
| :---: | :--- |
| 🛡️ **Keamanan Kelas Militer** | Menggunakan standar enkripsi **AES-256** dan **PBKDF2-HMAC-SHA256** untuk mencegah serangan *brute-force*. |
| 🔑 **Sistem PIN Pintar** | Akses cepat dengan PIN 6-digit. PIN di-hash dengan aman dan tidak pernah disimpan dalam *plaintext*. |
| 🔒 **Integrasi TOTP (2FA)** | Dukungan Autentikasi Dua Faktor untuk perlindungan ganda pada brankas Anda. |
| ✅ **Validasi Integritas** | Hash **SHA-256** memastikan data Anda tidak pernah rusak atau dimodifikasi oleh pihak ketiga. |
| 🗄️ **Database Lokal Terpusat**| Manajemen metadata pintar menggunakan **SQLite**, memastikan sinkronisasi data yang cepat dan aman. |
| 🗑️ **Recycle Bin Aman** | Sistem pemulihan file cerdas dengan recycle bin internal yang terenkripsi. |
| 🎨 **Desain Modern** | Antarmuka pengguna *glassmorphism* yang elegan, responsif, dan memanjakan mata. |

<div align="right">
  <a href="#-datavault-aegis-vault">⬆ Kembali ke Atas</a>
</div>

---

## 🚀 Panduan Memulai (Quick Start)

Amankan data Anda hanya dalam beberapa langkah mudah!

<details>
<summary><b>🛠️ 1. Persiapan Sistem (Klik untuk melihat instruksi)</b></summary>
<br>
Pastikan Anda sudah menginstal <b>Rust</b> dan <b>Cargo</b>.

```bash
# Instalasi untuk Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
> *Pengguna Windows dapat mengunduh installer `rustup-init.exe` dari [rustup.rs](https://rustup.rs/).*
</details>

<details>
<summary><b>📦 2. Kloning & Instalasi (Klik untuk melihat instruksi)</b></summary>
<br>

```bash
# Kloning repositori
git clone https://github.com/username_kamu/DataVault.git
cd DataVault

# Build proyek (Opsional namun disarankan untuk setup awal)
cargo build --release
```
</details>

### 3. Jalankan Aplikasi (Lintas Platform)

> [!IMPORTANT]
> **Selalu jalankan dalam mode `--release`!** Operasi kriptografi membutuhkan komputasi berat. Mode release membuat aplikasi berjalan ratusan kali lebih cepat dibandingkan mode debug.

**🖥️ Windows / Desktop:**
```bash
cargo run --release
```

**🤖 Android:**
Pastikan `cargo-apk` sudah terinstal (`cargo install cargo-apk`).
```bash
cargo apk run --lib --release
```

**🍎 iOS (Wajib menggunakan macOS & Xcode):**
1. Salin/kloning proyek ini ke komputer Mac Anda.
2. Tambahkan arsitektur iOS:
   ```bash
   rustup target add aarch64-apple-ios aarch64-apple-ios-sim
   ```
3. Lakukan kompilasi ke *Static Library* (`.a`):
   ```bash
   cargo build --target aarch64-apple-ios --release
   ```
4. Buka **Xcode**, buat proyek *iOS App* baru dengan Swift, lalu tarik file `libaegis_vault.a` ke dalam proyek.
5. Buat *Bridging Header* (`void start_app_ios();`), lalu panggil fungsi `start_app_ios()` tersebut dari *App Delegate* atau *SwiftUI* Anda.

<div align="right">
  <a href="#-datavault-aegis-vault">⬆ Kembali ke Atas</a>
</div>

---

## 📖 Cara Penggunaan

1. **Setup Awal 🔐**
   Saat pertama kali dijalankan, buat **PIN 6-digit** Anda. Ini adalah kunci utama ke brankas Anda. *Jangan sampai lupa!*
2. **Setup TOTP (2FA) 📱**
   Scan QR Code yang muncul di layar dengan aplikasi Authenticator (Google Authenticator / Authy) Anda untuk lapisan keamanan ganda.
3. **Amankan File ➕**
   Klik ikon tambah (+), lalu pilih dokumen, foto, atau video yang ingin disembunyikan. File akan dienkripsi secara otomatis.
4. **Pulihkan Data 🔓**
   Klik ikon gembok pada file di dalam brankas, pilih lokasi penyimpanan tujuan, dan file Anda akan kembali ke format aslinya.

<div align="right">
  <a href="#-datavault-aegis-vault">⬆ Kembali ke Atas</a>
</div>

---

## 📂 Struktur Penyimpanan

DataVault mengelola file Anda dengan rapi dan aman di dalam direktori `vault_storage/`:

<details>
<summary><b>📂 Tampilkan Pohon Direktori</b></summary>
<br>

```text
📦 DataVault
 ┣ 📂 src/                  # Source code aplikasi
 ┣ 📂 vault_storage/        # ⚠️ AREA TERENKRIPSI (JANGAN DIUBAH)
 ┃  ┣ 📜 vault.db           # Database metadata SQLite
 ┃  ┗ 📜 <uuid_file>        # File terenkripsi Anda
 ┗ 📜 Cargo.toml
```

</details>

> [!CAUTION]
> **DILARANG KERAS** mengubah, menghapus, atau memindahkan file di dalam `vault_storage/` secara manual melalui File Explorer. Hal ini dapat menyebabkan kerusakan database dan kehilangan data permanen!

<div align="right">
  <a href="#-datavault-aegis-vault">⬆ Kembali ke Atas</a>
</div>

---

## 🛠️ Teknologi yang Digunakan

Aplikasi ini didukung oleh ekosistem Rust yang luar biasa:

* **[egui](https://github.com/emilk/egui)**: Framework GUI *Immediate Mode* yang cepat.
* **[RustCrypto](https://github.com/RustCrypto)**: Implementasi murni Rust untuk algoritma kriptografi (`aes`, `pbkdf2`, `sha2`).
* **[rusqlite](https://github.com/rusqlite/rusqlite)**: Binding aman untuk SQLite.
* **[rfd](https://github.com/PolyMeilex/rfd)**: Dialog file *native* lintas platform.
* **[totp](https://github.com/zantinon/totp-rs)**: Implementasi *Time-Based One-Time Password*.

<div align="right">
  <a href="#-datavault-aegis-vault">⬆ Kembali ke Atas</a>
</div>

---

## 👥 Tim Pengembang

Proyek ini dikembangkan dengan dedikasi oleh:

<div align="center">

| Foto | Nama Lengkap | NIM |
| :---: | :--- | :--- |
| <img src="https://ui-avatars.com/api/?name=Rizma+Indra+Pramudya&background=random&color=fff&rounded=true" width="40"> | **Rizma Indra Pramudya** | `25051204370` |
| <img src="https://ui-avatars.com/api/?name=Izora+Elverda+Narulita+Putri&background=random&color=fff&rounded=true" width="40"> | **Izora Elverda Narulita Putri** | `25051204287` |
| <img src="https://ui-avatars.com/api/?name=Putera+Al+Khalidi&background=random&color=fff&rounded=true" width="40"> | **Putera Al Khalidi** | `25051204362` |
| <img src="https://ui-avatars.com/api/?name=Muhammad+Abdullah+Ro'in&background=random&color=fff&rounded=true" width="40"> | **Muhammad Abdullah Ro'in** | `25051204270` |

</div>

---

<div align="center">
  <img src="https://capsule-render.vercel.app/api?type=waving&color=CBA6F7&height=100&section=footer" />
  <p><b>Dibuat dengan ❤️ dan 🦀 (Rust) | Open Source</b></p>
</div>
