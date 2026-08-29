use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as base64_standard, Engine as _};
use rand::Rng;

/// Encrypts plaintext using AES-256-GCM.
/// Returns a base64 encoded string containing the nonce and ciphertext separated by a colon `nonce:ciphertext`.
pub fn encrypt_data(plain_text: &str, key_hex: &str) -> Result<String> {
    if plain_text.is_empty() {
        return Ok(String::new());
    }

    let key_bytes = hex::decode(key_hex).map_err(|e| anyhow!("Invalid hex key: {e}"))?;

    if key_bytes.len() != 32 {
        return Err(anyhow!(
            "Encryption key must be exactly 32 bytes (64 hex characters)"
        ));
    }

    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    // 96-bit nonce; unique per message. `AeadCore::generate_nonce` was removed
    // upstream, so the nonce is filled manually instead.
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let cipher_text = cipher
        .encrypt(&nonce, plain_text.as_bytes())
        .map_err(|e| anyhow!("Encryption failed: {e}"))?;

    let nonce_b64 = base64_standard.encode(nonce);
    let cipher_text_b64 = base64_standard.encode(cipher_text);

    Ok(format!("{nonce_b64}:{cipher_text_b64}"))
}

/// Decrypts a base64 encoded `nonce:ciphertext` string using AES-256-GCM.
pub fn decrypt_data(encrypted_data: &str, key_hex: &str) -> Result<String> {
    if encrypted_data.is_empty() {
        return Ok(String::new());
    }

    let parts: Vec<&str> = encrypted_data.split(':').collect();
    if parts.len() != 2 {
        // If the data is not in the correct format, we return as-is for backward compatibility
        // or fail. Secure approach is to fail.
        // But for a migration, maybe we just return the original if it isn't encrypted.
        // For security, if it's supposed to be encrypted, we fail.
        return Err(anyhow!(
            "Invalid encrypted data format. Expected nonce:ciphertext"
        ));
    }

    let key_bytes = hex::decode(key_hex).map_err(|e| anyhow!("Invalid hex key: {e}"))?;

    if key_bytes.len() != 32 {
        return Err(anyhow!(
            "Encryption key must be exactly 32 bytes (64 hex characters)"
        ));
    }

    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce_bytes = base64_standard
        .decode(parts[0])
        .map_err(|e| anyhow!("Invalid nonce base64: {e}"))?;
    let cipher_text_bytes = base64_standard
        .decode(parts[1])
        .map_err(|e| anyhow!("Invalid ciphertext base64: {e}"))?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    let plain_text_bytes = cipher
        .decrypt(nonce, cipher_text_bytes.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {e}"))?;

    String::from_utf8(plain_text_bytes).map_err(|e| anyhow!("Invalid UTF-8 in decrypted data: {e}"))
}

/// Helper function to check if a string appears to be encrypted
#[must_use]
pub fn is_encrypted(data: &str) -> bool {
    data.contains(':') && data.split(':').count() == 2
}

#[path = "crypto/encryption.rs"]
pub mod encryption;

pub use encryption::{EncryptedData, EncryptionService};

/// High-level crypto service used by twofa and sensitive field storage
#[derive(Clone)]
pub struct CryptoService {
    service: std::sync::Arc<EncryptionService>,
}

impl CryptoService {
    pub fn new(key_hex: &str) -> Result<Self> {
        EncryptionService::new(key_hex)
            .map(|s| Self {
                service: std::sync::Arc::new(s),
            })
            .map_err(|e| anyhow!("{e}"))
    }

    pub fn new_for_tests() -> Self {
        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        Self::new(key).expect("valid test key")
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        self.service
            .encrypt(plaintext)
            .map(|d| d.to_string())
            .map_err(|e| anyhow!("{e}"))
    }

    pub fn decrypt(&self, encrypted_str: &str) -> Result<String> {
        let data: EncryptedData = encrypted_str.parse().map_err(|e| anyhow!("{e}"))?;
        self.service.decrypt(&data).map_err(|e| anyhow!("{e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_test_key() -> String {
        let mut key = [0u8; 32];
        rand::rng().fill_bytes(&mut key);
        hex::encode(key)
    }

    #[test]
    fn test_encryption_decryption() {
        let key = generate_test_key();
        let plain_text = "super_secret_token_123!@#";

        let encrypted = encrypt_data(plain_text, &key).unwrap();
        assert_ne!(plain_text, encrypted);
        assert!(encrypted.contains(':'));
        assert!(is_encrypted(&encrypted));

        let decrypted = decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(plain_text, decrypted);
    }

    #[test]
    fn test_decryption_wrong_key() {
        let key1 = generate_test_key();
        let key2 = generate_test_key();

        let encrypted = encrypt_data("test data", &key1).unwrap();

        let result = decrypt_data(&encrypted, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_string() {
        let key = generate_test_key();
        let encrypted = encrypt_data("", &key).unwrap();
        assert_eq!(encrypted, "");

        let decrypted = decrypt_data("", &key).unwrap();
        assert_eq!(decrypted, "");
    }
}
