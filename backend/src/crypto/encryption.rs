/// AES-256-GCM encryption service for sensitive data
///
/// Implements authenticated encryption (AEAD) with:
/// - Per-operation random IV/nonce
/// - HMAC authentication tag
/// - Hex encoding for database storage
/// - Support for key versioning

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Encrypted data with metadata for storage and decryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Encrypted ciphertext (hex-encoded)
    pub ciphertext: String,
    /// Encryption nonce/IV (hex-encoded, random per encryption)
    pub nonce: String,
    /// Key version identifier (for rotation support)
    pub key_version: u32,
}

impl fmt::Display for EncryptedData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Format for storage: key_version:nonce:ciphertext
        write!(
            f,
            "{}:{}:{}",
            self.key_version, self.nonce, self.ciphertext
        )
    }
}

impl std::str::FromStr for EncryptedData {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            return Err("Invalid encrypted data format".to_string());
        }

        Ok(EncryptedData {
            key_version: parts[0]
                .parse()
                .map_err(|_| "Invalid key version")?,
            nonce: parts[1].to_string(),
            ciphertext: parts[2].to_string(),
        })
    }
}

/// Encryption service managing AES-256-GCM encryption for sensitive fields
///
/// # Example
///
/// ```rust,no_run
/// use payraider_backend::crypto::EncryptionService;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Initialize with encryption key from Vault
///     let key_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"; // 64 hex chars = 32 bytes
///     let service = EncryptionService::new(&key_hex)?;
///
///     // Encrypt sensitive data
///     let plaintext = "user-secret-api-key";
///     let encrypted = service.encrypt(plaintext)?;
///     println!("Encrypted: {}", encrypted);
///
///     // Decrypt later
///     let decrypted = service.decrypt(&encrypted)?;
///     assert_eq!(decrypted, plaintext);
///
///     Ok(())
/// }
/// ```
pub struct EncryptionService {
    primary_key: [u8; 32],
    key_version: u32,
    previous_keys: HashMap<u32, [u8; 32]>,
}

impl EncryptionService {
    /// Create a new encryption service with a 256-bit key (default version 1)
    ///
    /// # Arguments
    ///
    /// * `key_hex` - 64-character hex string representing 32 bytes (256 bits)
    ///
    /// # Returns
    ///
    /// Result<Self, String> - Service instance or error if key format is invalid
    pub fn new(key_hex: &str) -> Result<Self, String> {
        Self::new_with_version(key_hex, 1)
    }

    /// Create a new encryption service with an explicit key version
    pub fn new_with_version(key_hex: &str, version: u32) -> Result<Self, String> {
        if key_hex.len() != 64 {
            return Err(format!(
                "Encryption key must be 64 hex characters (32 bytes), got {}",
                key_hex.len()
            ));
        }

        let mut key = [0u8; 32];
        hex::decode_to_slice(key_hex, &mut key)
            .map_err(|e| format!("Invalid hex key: {}", e))?;

        Ok(Self {
            primary_key: key,
            key_version: version,
            previous_keys: HashMap::new(),
        })
    }

    /// Add a historical key version to support decrypting data encrypted prior to key rotation.
    pub fn add_previous_key(&mut self, version: u32, key_hex: &str) -> Result<(), String> {
        if key_hex.len() != 64 {
            return Err(format!(
                "Encryption key must be 64 hex characters (32 bytes), got {}",
                key_hex.len()
            ));
        }

        let mut key = [0u8; 32];
        hex::decode_to_slice(key_hex, &mut key)
            .map_err(|e| format!("Invalid hex key: {}", e))?;

        self.previous_keys.insert(version, key);
        Ok(())
    }

    /// Builder method to register a historical key version for key rotation.
    pub fn with_previous_key(mut self, version: u32, key_hex: &str) -> Result<Self, String> {
        self.add_previous_key(version, key_hex)?;
        Ok(self)
    }

    /// Get current primary key version
    pub fn key_version(&self) -> u32 {
        self.key_version
    }

    /// Encrypt plaintext using AES-256-GCM with random nonce
    ///
    /// Each encryption generates a unique random nonce to prevent
    /// patterns from appearing in ciphertext.
    pub fn encrypt(&self, plaintext: &str) -> Result<EncryptedData, String> {
        let cipher = Aes256Gcm::new((&self.primary_key).into());

        // Generate random 96-bit nonce (standard for GCM)
        let mut rng = rand::thread_rng();
        let nonce_bytes: [u8; 12] = rng.gen();
        let nonce = Nonce::from(nonce_bytes);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes() as Payload)
            .map_err(|e| format!("Encryption failed: {}", e))?;

        Ok(EncryptedData {
            ciphertext: hex::encode(&ciphertext),
            nonce: hex::encode(nonce.as_slice()),
            key_version: self.key_version,
        })
    }

    /// Decrypt ciphertext using AES-256-GCM
    ///
    /// Supports the primary key version as well as previous key versions registered for rotation.
    /// Verifies the authentication tag as part of GCM decryption.
    pub fn decrypt(&self, encrypted: &EncryptedData) -> Result<String, String> {
        let key_bytes: &[u8; 32] = if encrypted.key_version == self.key_version {
            &self.primary_key
        } else if let Some(old_key) = self.previous_keys.get(&encrypted.key_version) {
            old_key
        } else {
            return Err(format!(
                "Key version mismatch: data requires v{}, service has v{} and registered previous keys {:?}",
                encrypted.key_version,
                self.key_version,
                self.previous_keys.keys().copied().collect::<Vec<_>>()
            ));
        };

        let cipher = Aes256Gcm::new(key_bytes.into());

        let nonce_bytes = hex::decode(&encrypted.nonce)
            .map_err(|e| format!("Invalid nonce hex: {}", e))?;
        if nonce_bytes.len() != 12 {
            return Err(format!("Invalid nonce length: {}", nonce_bytes.len()));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext_bytes = hex::decode(&encrypted.ciphertext)
            .map_err(|e| format!("Invalid ciphertext hex: {}", e))?;

        let plaintext_bytes = cipher
            .decrypt(nonce, ciphertext_bytes.as_slice() as Payload)
            .map_err(|e| format!("Decryption failed (authentication tag invalid?): {}", e))?;

        String::from_utf8(plaintext_bytes)
            .map_err(|e| format!("Decrypted data is not valid UTF-8: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a test key (in production, keys come from Vault)
    fn test_key() -> String {
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let service = EncryptionService::new(&test_key()).unwrap();
        let plaintext = "sensitive-data-to-encrypt";

        let encrypted = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn each_encryption_generates_different_nonce() {
        let service = EncryptionService::new(&test_key()).unwrap();
        let plaintext = "same-plaintext";

        let encrypted1 = service.encrypt(plaintext).unwrap();
        let encrypted2 = service.encrypt(plaintext).unwrap();

        // Same plaintext should produce different ciphertexts
        // because of random nonce
        assert_ne!(encrypted1.ciphertext, encrypted2.ciphertext);
        assert_ne!(encrypted1.nonce, encrypted2.nonce);
    }

    #[test]
    fn decryption_with_tampered_ciphertext_fails() {
        let service = EncryptionService::new(&test_key()).unwrap();
        let plaintext = "original-data";

        let mut encrypted = service.encrypt(plaintext).unwrap();

        // Tamper with ciphertext
        encrypted.ciphertext = "deadbeef".to_string();

        // Should fail because authentication tag won't verify
        assert!(service.decrypt(&encrypted).is_err());
    }

    #[test]
    fn decryption_with_wrong_key_fails() {
        let service1 = EncryptionService::new(&test_key()).unwrap();
        let service2 =
            EncryptionService::new("fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210")
                .unwrap();

        let plaintext = "secret-data";
        let encrypted = service1.encrypt(plaintext).unwrap();

        // Should fail when decrypting with different key
        assert!(service2.decrypt(&encrypted).is_err());
    }

    #[test]
    fn key_version_mismatch_detected() {
        let service = EncryptionService::new(&test_key()).unwrap();
        let plaintext = "data";

        let mut encrypted = service.encrypt(plaintext).unwrap();
        encrypted.key_version = 99; // Simulate old key version

        // Should fail due to key version mismatch
        assert!(service.decrypt(&encrypted).is_err());
    }

    #[test]
    fn invalid_hex_key_rejected() {
        let invalid_hex = "not-a-valid-hex-string-too-short";
        assert!(EncryptionService::new(invalid_hex).is_err());
    }

    #[test]
    fn invalid_key_length_rejected() {
        let wrong_length = "0123456789abcdef0123456789abcdef"; // Only 32 chars, need 64
        assert!(EncryptionService::new(wrong_length).is_err());
    }

    #[test]
    fn encrypted_data_serialization() {
        let encrypted = EncryptedData {
            ciphertext: "abc123".to_string(),
            nonce: "def456".to_string(),
            key_version: 1,
        };

        let serialized = encrypted.to_string();
        assert_eq!(serialized, "1:def456:abc123");

        let deserialized: EncryptedData = serialized.parse().unwrap();
        assert_eq!(deserialized.key_version, 1);
        assert_eq!(deserialized.nonce, "def456");
        assert_eq!(deserialized.ciphertext, "abc123");
    }

    #[test]
    fn multi_key_version_decrypt_after_rotation() {
        let old_key = test_key();
        let service_v1 = EncryptionService::new_with_version(&old_key, 1).unwrap();
        let plaintext = "secret-across-rotations";

        // Encrypt with v1
        let encrypted_v1 = service_v1.encrypt(plaintext).unwrap();
        assert_eq!(encrypted_v1.key_version, 1);

        // Rotate to v2 key and register old v1 key
        let new_key = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let service_v2 = EncryptionService::new_with_version(new_key, 2)
            .unwrap()
            .with_previous_key(1, &old_key)
            .unwrap();

        // Encrypt new data with v2
        let encrypted_v2 = service_v2.encrypt("new-secret").unwrap();
        assert_eq!(encrypted_v2.key_version, 2);

        // Decrypt v2 data with service_v2
        assert_eq!(service_v2.decrypt(&encrypted_v2).unwrap(), "new-secret");

        // Decrypt old v1 data with service_v2 using registered previous key
        assert_eq!(service_v2.decrypt(&encrypted_v1).unwrap(), plaintext);
    }
}
