use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use anyhow::{bail, Context, Result};
use rand::RngCore;
use std::fs;
use std::path::Path;

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

const SALT_LEN: usize = 8;
const KEY_LEN: usize = 32;
const IV_LEN: usize = 16;
const PBKDF2_ITERATIONS: u32 = 10000;
const OPENSSL_MAGIC: &[u8; 8] = b"Salted__";

/// Derive key and IV from password + salt using PBKDF2-HMAC-SHA256.
/// OpenSSL uses a single PBKDF2 call with output length = key_len + iv_len.
fn derive_key_iv(password: &[u8], salt: &[u8]) -> ([u8; KEY_LEN], [u8; IV_LEN]) {
    let mut derived = [0u8; KEY_LEN + IV_LEN];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password, salt, PBKDF2_ITERATIONS, &mut derived);
    let mut key = [0u8; KEY_LEN];
    let mut iv = [0u8; IV_LEN];
    key.copy_from_slice(&derived[..KEY_LEN]);
    iv.copy_from_slice(&derived[KEY_LEN..]);
    (key, iv)
}

/// Encrypt plaintext using AES-256-CBC with OpenSSL-compatible Salted__ format.
pub fn encrypt(plaintext: &[u8], password: &str) -> Result<Vec<u8>> {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);

    let (key, iv) = derive_key_iv(password.as_bytes(), &salt);

    let cipher = Aes256CbcEnc::new(&key.into(), &iv.into());
    let ciphertext = cipher.encrypt_padded_vec_mut::<Pkcs7>(plaintext);

    let mut output = Vec::with_capacity(OPENSSL_MAGIC.len() + SALT_LEN + ciphertext.len());
    output.extend_from_slice(OPENSSL_MAGIC);
    output.extend_from_slice(&salt);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// Decrypt data in OpenSSL-compatible Salted__ format using AES-256-CBC.
pub fn decrypt(data: &[u8], password: &str) -> Result<Vec<u8>> {
    if data.len() < OPENSSL_MAGIC.len() + SALT_LEN {
        bail!("encrypted data too short");
    }
    if &data[..OPENSSL_MAGIC.len()] != OPENSSL_MAGIC {
        bail!("missing OpenSSL Salted__ header");
    }
    let salt = &data[OPENSSL_MAGIC.len()..OPENSSL_MAGIC.len() + SALT_LEN];
    let ciphertext = &data[OPENSSL_MAGIC.len() + SALT_LEN..];

    let (key, iv) = derive_key_iv(password.as_bytes(), salt);

    let cipher = Aes256CbcDec::new(&key.into(), &iv.into());
    let plaintext = cipher
        .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
        .map_err(|_| anyhow::anyhow!("decryption failed (wrong password or corrupted data)"))?;
    Ok(plaintext)
}

/// Encrypt plaintext and write to file atomically.
pub fn encrypt_to_file(plaintext: &[u8], outfile: &Path, password: &str) -> Result<()> {
    let encrypted = encrypt(plaintext, password)?;
    crate::config::atomic_write(outfile, &encrypted, 0o600)
}

/// Read encrypted file and decrypt.
pub fn decrypt_from_file(infile: &Path, password: &str) -> Result<Vec<u8>> {
    let data =
        fs::read(infile).with_context(|| format!("reading encrypted file {}", infile.display()))?;
    decrypt(&data, password)
}

/// Generate a random hex key (replaces `openssl rand -hex 32`).
pub fn generate_hex_key(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    hex::encode(&buf)
}

/// We need hex encoding but don't want another dependency.
mod hex {
    pub fn encode(data: &[u8]) -> String {
        let mut s = String::with_capacity(data.len() * 2);
        for byte in data {
            s.push_str(&format!("{:02x}", byte));
        }
        s
    }
}

/// Read the local key from file, or initialize it if missing.
pub fn ensure_local_key(key_file: &Path) -> Result<String> {
    if key_file.exists() {
        let key = fs::read_to_string(key_file)
            .with_context(|| format!("reading local key {}", key_file.display()))?;
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Ok(key);
        }
    }
    let key = generate_hex_key(32);
    crate::config::atomic_write(key_file, key.as_bytes(), 0o600)?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = b"ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
        let password = "test-password-12345";

        let encrypted = encrypt(plaintext, password).unwrap();
        assert!(encrypted.starts_with(OPENSSL_MAGIC));

        let decrypted = decrypt(&encrypted, password).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_password() {
        let plaintext = b"secret-token";
        let encrypted = encrypt(plaintext, "right-password").unwrap();
        let result = decrypt(&encrypted, "wrong-password");
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_produces_salted_header() {
        let encrypted = encrypt(b"test", "pass").unwrap();
        assert_eq!(&encrypted[..8], b"Salted__");
        // 8 bytes magic + 8 bytes salt + at least 16 bytes ciphertext (one AES block)
        assert!(encrypted.len() >= 32);
    }

    #[test]
    fn test_generate_hex_key() {
        let key = generate_hex_key(32);
        assert_eq!(key.len(), 64); // 32 bytes = 64 hex chars
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
