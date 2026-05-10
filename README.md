<div align="center">
  
# 🛡️ DataVault (Aegis Vault)

**Akses Aman & Privasi Penuh untuk Data Anda**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![egui](https://img.shields.io/badge/GUI-egui-blue.svg?style=for-the-badge)](https://github.com/emilk/egui)
[![Security](https://img.shields.io/badge/Security-AES--256-green.svg?style=for-the-badge&logo=security)](#)
[![License](https://img.shields.io/badge/License-MIT-lightgray.svg?style=for-the-badge)](#)

*Brankas digital modern yang memadukan performa tangguh Rust dengan antarmuka memukau.*

**[📥 Unduh Rilis Terbaru](#) • [🚀 Cara Penggunaan](#-cara-penggunaan) • [🐛 Laporkan Bug](#)**

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

[⬆ Kembali ke Atas](#-datavault-aegis-vault)

---

## ✨ Fitur Unggulan

| Fitur | Deskripsi |
| :--- | :--- |
| 🛡️ **Keamanan Kelas Militer** | Menggunakan standar enkripsi **AES-256** dan **PBKDF2-HMAC-SHA256** untuk mencegah serangan *brute-force*. |
| 🔑 **Sistem PIN Pintar** | Akses cepat dengan PIN 6-digit. PIN di-hash dengan aman dan tidak pernah disimpan dalam *plaintext*. |
| ✅ **Validasi Integritas** | Hash **SHA-256** memastikan data Anda tidak pernah rusak atau dimodifikasi oleh pihak ketiga. |
| 🗄️ **Database Lokal Terpusat**| Manajemen metadata pintar menggunakan **SQLite**, memastikan sinkronisasi data yang cepat dan aman. |
| 🎨 **Desain Modern** | Antarmuka pengguna *glassmorphism* yang elegan, responsif, dan memanjakan mata. |

[⬆ Kembali ke Atas](#-datavault-aegis-vault)

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

### 3. Jalankan Aplikasi
> [!IMPORTANT]
> **Selalu jalankan dalam mode `--release`!** Operasi kriptografi membutuhkan komputasi berat. Mode release membuat aplikasi berjalan ratusan kali lebih cepat dibandingkan mode debug.

```bash
cargo run --release
```

[⬆ Kembali ke Atas](#-datavault-aegis-vault)

---

## 📖 Cara Penggunaan

1. **Setup Awal 🔐**
   Saat pertama kali dijalankan, buat **PIN 6-digit** Anda. Ini adalah kunci utama ke brankas Anda. *Jangan sampai lupa!*
2. **Amankan File ➕**
   Klik ikon tambah (+), lalu pilih dokumen, foto, atau video yang ingin disembunyikan. File akan dienkripsi secara otomatis.
3. **Pulihkan Data 🔓**
   Klik ikon gembok pada file di dalam brankas, pilih lokasi penyimpanan tujuan, dan file Anda akan kembali ke format aslinya.

[⬆ Kembali ke Atas](#-datavault-aegis-vault)

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

[⬆ Kembali ke Atas](#-datavault-aegis-vault)

---

## 🛠️ Teknologi yang Digunakan

Aplikasi ini didukung oleh ekosistem Rust yang luar biasa:

* **[egui](https://github.com/emilk/egui)**: Framework GUI *Immediate Mode* yang cepat.
* **[RustCrypto](https://github.com/RustCrypto)**: Implementasi murni Rust untuk algoritma kriptografi (`aes`, `pbkdf2`, `sha2`).
* **[rusqlite](https://github.com/rusqlite/rusqlite)**: Binding aman untuk SQLite.
* **[rfd](https://github.com/PolyMeilex/rfd)**: Dialog file *native* lintas platform.

[⬆ Kembali ke Atas](#-datavault-aegis-vault)

---

## 👥 Tim Pengembang

Proyek ini dikembangkan dengan dedikasi oleh:

<details>
<summary><b>👑 Lihat Anggota Tim</b></summary>
<br>

* 👨‍💻 **Rizma Indra Pramudya** (25051204370)
* 👩‍💻 **Izora Elverda Narulita Putri** (25051204287)
* 👨‍💻 **Putera Al Khalidi** (25051204362)
* 👨‍💻 **Muhammad Abdullah Ro'in** (25051204270)

</details>

---

<div align="center">
  <p>Dibuat dengan ❤️ dan 🦀 (Rust) | Open Source</p>
</div>
