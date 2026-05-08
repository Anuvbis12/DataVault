use aes::Aes256;
use aes::cipher::{BlockEncrypt, KeyInit, generic_array::{GenericArray, typenum::U16}};
use rand::{thread_rng, RngCore};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const BUFFER_SIZE: usize = 64 * 1024;
const CUSTOM_EXTENSION: &str = ".vlt";

#[derive(Debug)]
pub struct EncryptionResult {
    pub encrypted_filename: String,
    pub file_hash: String,
}

pub fn secure_encrypt_file(
    source_path: &Path,
    dest_dir: &Path,
    key: &[u8; 32],
) -> Result<EncryptionResult, std::io::Error> {
    let mut iv = [0u8; 16];
    thread_rng().fill_bytes(&mut iv);

    let mut random_bytes = [0u8; 16];
    thread_rng().fill_bytes(&mut random_bytes);
    let random_name: String = random_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    let encrypted_filename = format!("{}{}", random_name, CUSTOM_EXTENSION);
    let dest_path = dest_dir.join(&encrypted_filename);

    let mut source_file = File::open(source_path)?;
    let mut dest_file = File::create(&dest_path)?;
    let mut hasher = Sha256::new();

    dest_file.write_all(&iv)?;
    hasher.update(&iv);

    let encryptor = Aes256::new(key.into());
    let mut prev_iv: GenericArray<u8, U16> = GenericArray::clone_from_slice(&iv);
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let bytes_read = source_file.read(&mut buffer)?;
        if bytes_read == 0 {
            // Write standard PKCS7 padding for an empty block
            let mut pad_block: GenericArray<u8, U16> = GenericArray::clone_from_slice(&[16u8; 16]);
            for i in 0..16 {
                pad_block[i] ^= prev_iv[i];
            }
            encryptor.encrypt_block(&mut pad_block);
            dest_file.write_all(&pad_block)?;
            hasher.update(&pad_block);
            break;
        }

        if bytes_read < BUFFER_SIZE {
            let full_blocks = bytes_read / 16;
            let rem = bytes_read % 16;

            // Process all full blocks
            for b in 0..full_blocks {
                let offset = b * 16;
                for i in 0..16 {
                    buffer[offset + i] ^= prev_iv[i];
                }
                let mut block: GenericArray<u8, U16> = GenericArray::clone_from_slice(&buffer[offset..offset + 16]);
                encryptor.encrypt_block(&mut block);
                buffer[offset..offset + 16].copy_from_slice(&block);
                prev_iv.copy_from_slice(&block);
            }
            dest_file.write_all(&buffer[..full_blocks * 16])?;
            hasher.update(&buffer[..full_blocks * 16]);

            // PKCS7 Pad the remaining bytes (or add a full padded block if rem == 0)
            let pad_len = 16 - rem;
            let mut pad_block = [pad_len as u8; 16];
            pad_block[..rem].copy_from_slice(&buffer[full_blocks * 16..bytes_read]);

            for i in 0..16 {
                pad_block[i] ^= prev_iv[i];
            }
            let mut block: GenericArray<u8, U16> = GenericArray::clone_from_slice(&pad_block);
            encryptor.encrypt_block(&mut block);

            dest_file.write_all(&block)?;
            hasher.update(&block);
            break;
        } else {
            // Process the full 64KB buffer
            for b in 0..(BUFFER_SIZE / 16) {
                let offset = b * 16;
                for i in 0..16 {
                    buffer[offset + i] ^= prev_iv[i];
                }
                let mut block: GenericArray<u8, U16> = GenericArray::clone_from_slice(&buffer[offset..offset + 16]);
                encryptor.encrypt_block(&mut block);
                buffer[offset..offset + 16].copy_from_slice(&block);
                prev_iv.copy_from_slice(&block);
            }
            dest_file.write_all(&buffer)?;
            hasher.update(&buffer);
        }
    }

    dest_file.flush()?;

    let hash_result = hasher.finalize();
    let file_hash = hex::encode(hash_result);

    secure_delete(source_path)?;

    Ok(EncryptionResult {
        encrypted_filename,
        file_hash,
    })
}

fn secure_delete(path: &Path) -> Result<(), std::io::Error> {
    if let Ok(mut file) = OpenOptions::new().write(true).open(path) {
        let file_size = file.metadata()?.len();
        let zeros = vec![0u8; BUFFER_SIZE];

        let mut written: u64 = 0;

        file.seek(SeekFrom::Start(0))?;

        while written < file_size {
            let to_write = std::cmp::min((file_size - written) as usize, BUFFER_SIZE);
            file.write_all(&zeros[..to_write])?;
            written += to_write as u64;
        }
        file.sync_all()?;
    }
    std::fs::remove_file(path)?;
    Ok(())
}
