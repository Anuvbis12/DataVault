use pbkdf2::pbkdf2;
use hmac::Hmac;
use sha2::{Sha256, Digest};
use std::sync::mpsc;
use std::thread;

fn hash_pin(pin: &str, salt: &[u8; 16]) -> String {
    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(
        pin.as_bytes(),
        salt,
        310_000,
        &mut key,
    ).unwrap();
    
    let hash = Sha256::digest(&key);
    hex::encode(hash)
}

fn main() {
    let target_hash = "af5dbd2edbc1af02fc722df4cfc02c969aef276fb23d104e19b78a35257520b9";
    let salt_hex = "2ed2e3b4822084aef82c8d68d7b04b79";
    let salt_bytes = hex::decode(salt_hex).unwrap();
    let mut salt = [0u8; 16];
    salt.copy_from_slice(&salt_bytes);
    
    let num_threads = 8;
    let (tx, rx) = mpsc::channel();
    
    // Total possible PINs from 0 to 999999. Since it can be 4, 5, or 6 digits, we check strings.
    // Let's divide the space 0-999999 into chunks.
    let chunk_size = 1_000_000 / num_threads;
    
    println!("Starting brute force with {} threads...", num_threads);
    
    for t in 0..num_threads {
        let tx = tx.clone();
        let salt = salt.clone();
        let target_hash = target_hash.to_string();
        
        thread::spawn(move || {
            let start = t * chunk_size;
            let end = if t == num_threads - 1 { 1_000_000 } else { (t + 1) * chunk_size };
            
            for i in start..end {
                // Try 4 digit, 5 digit, 6 digit representations of this number if they match
                let s = i.to_string();
                
                let pins_to_try = vec![
                    format!("{:04}", i),
                    format!("{:05}", i),
                    format!("{:06}", i)
                ];
                
                for pin in pins_to_try {
                    // Only test valid length if the number actually fits
                    if pin.parse::<usize>().unwrap() == i {
                        let h = hash_pin(&pin, &salt);
                        if h == target_hash {
                            tx.send(Some(pin)).unwrap();
                            return;
                        }
                    }
                }
                
                if i > 0 && i % 10000 == 0 {
                    println!("Thread {} at {}", t, i);
                }
            }
            tx.send(None).unwrap();
        });
    }
    
    drop(tx);
    
    for msg in rx {
        if let Some(pin) = msg {
            println!("FOUND PIN: {}", pin);
            std::process::exit(0);
        }
    }
    
    println!("PIN not found in 4-6 digit numeric space.");
}
