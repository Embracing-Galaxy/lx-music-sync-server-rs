use serde::{Serialize, de::DeserializeOwned};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub fn load_or_create<T: Default + DeserializeOwned + Serialize>(path: &Path) -> T {
    if path.exists() {
        let bytes = fs::read(path).unwrap_or_else(|_| panic!("Failed to read {:?}", path));
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| panic!("Failed to deserialize {:?}", path))
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|_| panic!("Failed to create {:?}", parent));
        }
        let default: T = T::default();
        let bytes = serde_json::to_vec_pretty(&default).expect("Failed to serialize default value");
        fs::write(path, bytes).expect("Failed to write default value to file");
        default
    }
}

// ----------------------------------------------------------------------------------

use dashmap::{DashMap, Entry};
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct RwCounter<T: Eq + Hash> {
    map: DashMap<T, (usize, u128)>,
    last_save_ms: AtomicU64,
}

impl<T: Eq + Hash> RwCounter<T> {
    const TTL_MS: u128 = 60 * 60 * 24 * 2 * 1000; // 2 days in milliseconds

    fn new() -> Self {
        Self {
            map: DashMap::new(),
            last_save_ms: AtomicU64::new(0),
        }
    }

    pub(crate) fn increase(&self, key: T) {
        match self.map.entry(key) {
            Entry::Occupied(mut e) => {
                let (count, time) = e.get_mut();
                *count += 1;
                *time = now_ms();
            }
            Entry::Vacant(e) => {
                e.insert((1, now_ms()));
            }
        }
    }

    pub(crate) fn count(&self, key: &T) -> usize {
        match self.map.get(key) {
            Some(entry) => entry.0,
            None => 0,
        }
    }

    pub(crate) fn cleanup(&self) {
        let deadline = now_ms().saturating_sub(Self::TTL_MS);
        self.map.retain(|_, (_, last_used)| *last_used >= deadline);
    }
}

impl<T: Eq + Hash + Clone + Serialize> RwCounter<T> {
    fn save_to(&self, path: &Path) {
        let entries: Vec<(T, usize, u128)> = self
            .map
            .iter()
            .map(|r| {
                let (k, v) = r.pair();
                (k.clone(), v.0, v.1)
            })
            .collect();
        // TODO async serialize & IO
        let bytes = serde_json::to_vec_pretty(&entries).expect("Failed to serialize RwCounter");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(path, bytes).expect("Failed to write RwCounter");
        self.last_save_ms.store(now_ms() as u64, Ordering::Release);
    }

    pub(crate) fn try_save(&self, path: &Path) {
        const DEBOUNCE_MS: u64 = 30_000; // 30 seconds
        let now = now_ms() as u64;
        let prev = self.last_save_ms.load(Ordering::Acquire);
        // After a restart the first try_save always bypasses the debounce and
        // writes immediately as `prev` equals to 0
        if now.saturating_sub(prev) < DEBOUNCE_MS {
            return; // Debounce: too soon since last save
        }
        // CAS: only one thread proceeds to save
        if self
            .last_save_ms
            .compare_exchange(prev, now, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
        {
            self.save_to(path);
        }
    }
}

impl<T: Eq + Hash + DeserializeOwned> RwCounter<T> {
    pub(crate) fn load_from(path: &Path) -> Self {
        let counter = Self::new();
        if path.exists() {
            let bytes = fs::read(path).unwrap_or_else(|_| panic!("Failed to read {:?}", path));
            let entries: Vec<(T, usize, u128)> = serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| panic!("Failed to deserialize {:?}", path));
            for (key, count, time) in entries {
                counter.map.insert(key, (count, time));
            }
        }
        // File missing = empty counter
        counter
    }
}

// ----------------------------------------------------------------------------------

use base64::prelude::{BASE64_STANDARD, Engine};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use std::io::{Read, Write};

pub fn gzip_base64(data: impl AsRef<[u8]>) -> String {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_ref()).unwrap();
    let compressed = encoder.finish().unwrap();
    BASE64_STANDARD.encode(compressed)
}

pub fn ungzip_base64(data: impl AsRef<[u8]>) -> Vec<u8> {
    let compressed = BASE64_STANDARD
        .decode(data)
        .expect("failed to decode base64");
    let mut buffer = Vec::new();
    GzDecoder::new(compressed.as_slice())
        .read_to_end(&mut buffer)
        .expect("failed to decompress");
    buffer
}

pub mod crypto {
    use base64::prelude::{BASE64_STANDARD, Engine};
    use openssl::{
        hash::{MessageDigest, hash},
        rsa::{Padding, Rsa},
        symm::{Cipher, decrypt, encrypt},
    };
    use rand::RngExt;

    pub type MD5 = u128;

    pub fn to_md5(data: impl AsRef<[u8]>) -> MD5 {
        let bytes: Vec<u8> = hash(MessageDigest::md5(), data.as_ref())
            .unwrap()
            .iter()
            .copied()
            .collect();
        u128::from_be_bytes(bytes.try_into().unwrap())
    }

    pub fn hex_to_md5(hex_str: &str) -> MD5 {
        debug_assert_eq!(hex_str.len(), 32);
        let bytes = (0..32)
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16))
            .collect::<Result<Vec<_>, _>>()
            .expect("hex decode error");
        u128::from_be_bytes(bytes.try_into().unwrap())
    }

    pub fn md5_to_hex(data: MD5) -> String {
        // Note that this is parsed in big-endian order,
        // so MD5 should also be parsed in big-endian order.
        format!("{:032x}", data)
    }

    pub fn rand_16bytes_as_base64() -> String {
        let mut rng = rand::rng();
        let mut buf = [0u8; 16];
        rng.fill(&mut buf);
        BASE64_STANDARD.encode(buf)
    }

    pub fn aes_encrypt_with_base64(data: &str, key: &str) -> String {
        // decode key with base64
        let key_bytes = BASE64_STANDARD.decode(key).expect("Invalid base64 key");
        debug_assert_eq!(key_bytes.len(), 16, "AES-128 key must be 16 bytes");

        let cipher = Cipher::aes_128_ecb();
        let ciphertext =
            encrypt(cipher, &key_bytes, None, data.as_bytes()).expect("Encryption failed");

        // encode with base64 and return
        BASE64_STANDARD.encode(ciphertext)
    }

    pub fn aes_decrypt_with_base64(text: &str, key: &str) -> String {
        // decode key and ciphertext with base64
        let key_bytes = BASE64_STANDARD.decode(key).expect("Invalid base64 key");
        let cipher_bytes = BASE64_STANDARD
            .decode(text)
            .expect("Invalid base64 ciphertext");

        let cipher = Cipher::aes_128_ecb();

        // decrypt(No IV, ECB mode)
        let plain = decrypt(cipher, &key_bytes, None, &cipher_bytes).expect("Decryption failed");
        String::from_utf8(plain).expect("Invalid utf-8 plaintext")
    }

    pub fn rsa_encrypt_with_base64(data: &str, public_key: &str) -> String {
        let public_key = Rsa::public_key_from_pem(public_key.as_bytes())
            .expect("Failed to parse public key from PEM");

        // encrypt with PKCS1_OAEP padding
        let mut encrypted = vec![0; public_key.size() as usize];
        let encrypted_len = public_key
            .public_encrypt(data.as_bytes(), &mut encrypted, Padding::PKCS1_OAEP)
            .expect("RSA encryption failed");

        encrypted.truncate(encrypted_len);
        BASE64_STANDARD.encode(encrypted)
    }
}
