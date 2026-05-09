// crypto.rs — AES-256-CBC engine
// Mempertahankan implementasi manual block-by-block milik temanmu,
// ditambah: PBKDF2 key derivation, 3-pass secure delete,
// dekripsi dengan validasi hash, IV tersimpan di file header.

use aes::Aes256;
use aes::cipher::{
    BlockDecrypt, BlockEncrypt, KeyInit,
    generic_array::{GenericArray, typenum::U16},
};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::{thread_rng, RngCore};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use zeroize::Zeroize;

// ── Konstanta ─────────────────────────────────────────────
pub const BUFFER_SIZE:      usize = 64 * 1024; // 64 KB — optimal untuk cache L2 ARM
pub const CUSTOM_EXTENSION: &str  = ".vlt";
pub const PBKDF2_ITERATIONS: u32  = 310_000;   // OWASP 2024 recommendation
pub const SALT_LEN:          usize = 16;
pub const KEY_LEN:           usize = 32;        // AES-256

// ── Struct hasil enkripsi ─────────────────────────────────
#[derive(Debug, Clone)]
pub struct EncryptionResult {
    pub encrypted_filename: String,
    pub file_hash:          String,
    pub iv:                 [u8; 16],
}

// ── PBKDF2 Key Derivation ─────────────────────────────────

/// Generate salt acak 16 byte
pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    thread_rng().fill_bytes(&mut salt);
    salt
}

/// Derivasi kunci AES-256 dari PIN + salt menggunakan PBKDF2-HMAC-SHA256
/// Kunci dikembalikan dalam Box agar mudah di-zeroize setelah pakai
pub fn derive_key(pin: &str, salt: &[u8; SALT_LEN]) -> Box<[u8; KEY_LEN]> {
    let mut key = Box::new([0u8; KEY_LEN]);
    pbkdf2::<Hmac<Sha256>>(
        pin.as_bytes(),
        salt,
        PBKDF2_ITERATIONS,
        key.as_mut(),
    ).expect("PBKDF2 tidak boleh gagal dengan parameter valid");
    key
}

/// Verifikasi PIN: derivasi kunci lalu cek hash PIN yang tersimpan
/// Hash PIN disimpan sebagai: SHA-256(PBKDF2(pin, salt))
pub fn hash_pin(pin: &str, salt: &[u8; SALT_LEN]) -> String {
    let mut key = derive_key(pin, salt);
    let hash    = Sha256::digest(key.as_ref());
    key.zeroize();
    hex::encode(hash)
}

// ── Enkripsi File ─────────────────────────────────────────

pub fn secure_encrypt_file(
    source_path: &Path,
    dest_dir:    &Path,
    key:         &[u8; KEY_LEN],
) -> Result<EncryptionResult, std::io::Error> {
    // Generate IV dan nama file acak
    let mut iv            = [0u8; 16];
    let mut random_bytes  = [0u8; 16];
    thread_rng().fill_bytes(&mut iv);
    thread_rng().fill_bytes(&mut random_bytes);

    let random_name:        String = random_bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let encrypted_filename: String = format!("{}{}", random_name, CUSTOM_EXTENSION);
    let dest_path                  = dest_dir.join(&encrypted_filename);

    let mut source_file = File::open(source_path)?;
    let mut dest_file   = File::create(&dest_path)?;
    let mut hasher      = Sha256::new();

    // Tulis IV di header file (16 byte pertama)
    dest_file.write_all(&iv)?;
    hasher.update(&iv);

    // Enkripsi block-by-block AES-256-CBC (implementasi temanmu, dipertahankan)
    let encryptor  = Aes256::new(key.into());
    let mut prev_iv: GenericArray<u8, U16> = GenericArray::clone_from_slice(&iv);
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let bytes_read = source_file.read(&mut buffer)?;

        if bytes_read == 0 {
            // File kosong: tulis full padding block (PKCS7 = 16x byte 0x10)
            let mut pad_block: GenericArray<u8, U16> = GenericArray::clone_from_slice(&[16u8; 16]);
            for i in 0..16 { pad_block[i] ^= prev_iv[i]; }
            encryptor.encrypt_block(&mut pad_block);
            dest_file.write_all(&pad_block)?;
            hasher.update(&pad_block);
            break;
        }

        if bytes_read < BUFFER_SIZE {
            // Chunk terakhir — proses full blocks dulu, lalu padding
            let full_blocks = bytes_read / 16;
            let rem         = bytes_read % 16;

            for b in 0..full_blocks {
                let offset = b * 16;
                for i in 0..16 { buffer[offset + i] ^= prev_iv[i]; }
                let mut block: GenericArray<u8, U16> =
                    GenericArray::clone_from_slice(&buffer[offset..offset + 16]);
                encryptor.encrypt_block(&mut block);
                buffer[offset..offset + 16].copy_from_slice(&block);
                prev_iv.copy_from_slice(&block);
            }
            dest_file.write_all(&buffer[..full_blocks * 16])?;
            hasher.update(&buffer[..full_blocks * 16]);

            // PKCS7 padding block
            let pad_len          = 16 - rem;
            let mut pad_block    = [pad_len as u8; 16];
            pad_block[..rem].copy_from_slice(&buffer[full_blocks * 16..bytes_read]);
            for i in 0..16 { pad_block[i] ^= prev_iv[i]; }
            let mut block: GenericArray<u8, U16> = GenericArray::clone_from_slice(&pad_block);
            encryptor.encrypt_block(&mut block);
            dest_file.write_all(&block)?;
            hasher.update(&block);
            break;
        } else {
            // Full 64 KB buffer
            for b in 0..(BUFFER_SIZE / 16) {
                let offset = b * 16;
                for i in 0..16 { buffer[offset + i] ^= prev_iv[i]; }
                let mut block: GenericArray<u8, U16> =
                    GenericArray::clone_from_slice(&buffer[offset..offset + 16]);
                encryptor.encrypt_block(&mut block);
                buffer[offset..offset + 16].copy_from_slice(&block);
                prev_iv.copy_from_slice(&block);
            }
            dest_file.write_all(&buffer)?;
            hasher.update(&buffer);
        }
    }

    dest_file.flush()?;
    let file_hash = hex::encode(hasher.finalize());

    // Secure delete file asli (3-pass)
    secure_delete(source_path)?;

    Ok(EncryptionResult {
        encrypted_filename,
        file_hash,
        iv,
    })
}

// ── Dekripsi File ─────────────────────────────────────────

#[derive(Debug)]
pub enum DecryptError {
    Io(std::io::Error),
    HashMismatch { expected: String, actual: String },
    InvalidPadding,
}

impl std::fmt::Display for DecryptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecryptError::Io(e)               => write!(f, "IO error: {}", e),
            DecryptError::HashMismatch { expected, actual }  => write!(f, "Integritas file gagal — hash tidak cocok (expected: {}, actual: {})", expected, actual),
            DecryptError::InvalidPadding       => write!(f, "Padding tidak valid — kemungkinan kunci salah"),
        }
    }
}

impl From<std::io::Error> for DecryptError {
    fn from(e: std::io::Error) -> Self { DecryptError::Io(e) }
}

/// Dekripsi file .vlt ke output_path.
/// Validasi SHA-256 hash sebelum menulis output.
pub fn secure_decrypt_file(
    vault_path:   &Path,
    output_path:  &Path,
    key:          &[u8; KEY_LEN],
    expected_hash: &str,
) -> Result<(), DecryptError> {
    // Langkah 1: validasi hash file vault sebelum dekripsi
    let actual_hash = compute_file_hash(vault_path)?;
    if actual_hash != expected_hash {
        return Err(DecryptError::HashMismatch {
            expected: expected_hash.to_string(),
            actual:   actual_hash,
        });
    }

    let mut vault_file = File::open(vault_path)?;

    // Baca IV dari 16 byte pertama
    let mut iv = [0u8; 16];
    vault_file.read_exact(&mut iv)?;

    let decryptor  = Aes256::new(key.into());
    let mut prev_iv: GenericArray<u8, U16> = GenericArray::clone_from_slice(&iv);

    // Baca seluruh ciphertext (setelah IV)
    let mut ciphertext = Vec::new();
    vault_file.read_to_end(&mut ciphertext)?;

    if ciphertext.is_empty() || ciphertext.len() % 16 != 0 {
        return Err(DecryptError::InvalidPadding);
    }

    // Dekripsi semua block
    let mut plaintext = vec![0u8; ciphertext.len()];
    let block_count   = ciphertext.len() / 16;

    for b in 0..block_count {
        let offset = b * 16;
        let cipher_block: GenericArray<u8, U16> =
            GenericArray::clone_from_slice(&ciphertext[offset..offset + 16]);
        let mut plain_block = cipher_block;
        decryptor.decrypt_block(&mut plain_block);

        // CBC XOR dengan prev_iv
        for i in 0..16 { plain_block[i] ^= prev_iv[i]; }
        prev_iv = GenericArray::clone_from_slice(&ciphertext[offset..offset + 16]);
        plaintext[offset..offset + 16].copy_from_slice(&plain_block);
    }

    // Hapus PKCS7 padding dari akhir
    let pad_len = *plaintext.last().ok_or(DecryptError::InvalidPadding)? as usize;
    if pad_len == 0 || pad_len > 16 {
        return Err(DecryptError::InvalidPadding);
    }
    // Verifikasi semua byte padding konsisten
    let pad_start = plaintext.len() - pad_len;
    if !plaintext[pad_start..].iter().all(|&b| b == pad_len as u8) {
        return Err(DecryptError::InvalidPadding);
    }
    plaintext.truncate(pad_start);

    // Tulis output
    let mut out_file = File::create(output_path)?;
    out_file.write_all(&plaintext)?;
    out_file.flush()?;

    Ok(())
}

// ── Hash File ─────────────────────────────────────────────

pub fn compute_file_hash(path: &Path) -> Result<String, std::io::Error> {
    let mut file   = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }

    Ok(hex::encode(hasher.finalize()))
}

// ── 3-Pass Secure Delete ──────────────────────────────────
// Pass 1: overwrite 0x00
// Pass 2: overwrite 0xFF
// Pass 3: overwrite random bytes
// Kemudian hapus file

pub fn secure_delete(path: &Path) -> Result<(), std::io::Error> {
    if !path.exists() { return Ok(()); }

    let file_size = path.metadata()?.len();

    for pass in 0..3u8 {
        let mut file = OpenOptions::new().write(true).open(path)?;
        file.seek(SeekFrom::Start(0))?;

        let mut written: u64 = 0;
        while written < file_size {
            let chunk = std::cmp::min((file_size - written) as usize, BUFFER_SIZE);
            let mut buf = vec![0u8; chunk];

            match pass {
                0 => buf.iter_mut().for_each(|b| *b = 0x00),
                1 => buf.iter_mut().for_each(|b| *b = 0xFF),
                _ => thread_rng().fill_bytes(&mut buf),
            }

            file.write_all(&buf)?;
            written += chunk as u64;
        }
        file.sync_all()?;
    }

    std::fs::remove_file(path)?;
    Ok(())
}