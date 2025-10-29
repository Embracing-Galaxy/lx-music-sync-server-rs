use serde::{de::DeserializeOwned, Serialize};
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
        let bytes = fs::read(path).expect(format!("Failed to read {:?}", path).as_str());
        serde_json::from_slice(&bytes).expect(format!("Failed to deserialize {:?}", path).as_str())
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect(format!("Failed to create {:?}", parent).as_str());
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
use std::time::{Duration, Instant};

pub struct RwCounter<T: Eq + Hash> {
    map: DashMap<T, (usize, Instant)>,
}

impl<T: Eq + Hash> RwCounter<T> {
    const TTL: Duration = Duration::from_secs(60 * 60 * 24 * 2); // 2 days
    pub(crate) fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    pub(crate) fn increase(&self, key: T) {
        match self.map.entry(key) {
            Entry::Occupied(mut e) => {
                let (count, time) = e.get_mut();
                *count += 1;
                *time = Instant::now();
            }
            Entry::Vacant(e) => {
                e.insert((1, Instant::now()));
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
        let now = Instant::now();
        let Some(deadline) = now.checked_sub(Self::TTL) else {
            // TODO "Make `last_used` persistent"
            return; // The program did not run long enough
        };
        self.map.retain(|_, (_, last_used)| *last_used >= deadline);
    }
}

// ----------------------------------------------------------------------------------

use base64::prelude::{Engine, BASE64_STANDARD};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
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
    use base64::prelude::{Engine, BASE64_STANDARD};
    use openssl::{
        hash::{hash, MessageDigest},
        rsa::{Padding, Rsa},
        symm::{decrypt, encrypt, Cipher},
    };
    use rand::Rng;

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
