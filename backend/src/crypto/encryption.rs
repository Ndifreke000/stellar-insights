/// AES-256-GCM encryption service for sensitive data
///
/// Implements authenticated encryption (AEAD) with:
/// - Per-operation random IV/nonce
/// - HMAC authentication tag
/// - Hex encoding for database storage
/// - Support for key versioning
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::{Rng, RngExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Errors returned by the encryption service.
///
/// Decryption operates on potentially attacker-controlled input, so every
/// failure mode is surfaced as a typed error instead of a panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncryptionError {
    /// The provided key was not 64 hex characters / not valid hex.
    InvalidKey(String),
    /// The nonce was not valid hex.
    InvalidNonceHex(String),
    /// The nonce did not decode to the expected 12 bytes.
    InvalidNonceLength(usize),
    /// The ciphertext was not valid hex.
    InvalidCiphertextHex(String),
    /// The key version is unknown to this service.
    KeyVersionMismatch {
        required: u32,
        available: u32,
    },
    /// AEAD authentication failed (tampered ciphertext or wrong key).
    DecryptionFailed(String),
    /// The decrypted bytes were not valid UTF-8.
    InvalidUtf8(String),
    /// Encryption failed.
    EncryptionFailed(String),
}

impl fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EncryptionError::InvalidKey(msg) => write!(f, "Invalid encryption key: {}", msg),
            EncryptionError::InvalidNonceHex(msg) => write!(f, "Invalid nonce hex: {}", msg),
            EncryptionError::InvalidNonceLength(len) => {
                write!(f, "Invalid nonce length: {}", len)
            }
            EncryptionError::InvalidCiphertextHex(msg) => {
                write!(f, "Invalid ciphertext hex: {}", msg)
            }
            EncryptionError::KeyVersionMismatch {
                required,
                available,
            } => write!(
                f,
                "Key version mismatch: data requires v{}, service has v{}",
                required, available
            ),
            EncryptionError::DecryptionFailed(msg) => write!(f, "Decryption failed: {}", msg),
            EncryptionError::InvalidUtf8(msg) => {
                write!(f, "Decrypted data is not valid UTF-8: {}", msg)
            }
            EncryptionError::EncryptionFailed(msg) => write!(f, "Encryption failed: {}", msg),
        }
    }
}

impl std::error::Error for EncryptionError {}

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
        write!(f, "{}:{}:{}", self.key_version, self.nonce, self.ciphertext)
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
            key_version: parts[0].parse().map_err(|_| "Invalid key version")?,
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
    /// Result<Self, EncryptionError> - Service instance or error if key format is invalid
    pub fn new(key_hex: &str) -> Result<Self, EncryptionError> {
        Self::new_with_version(key_hex, 1)
    }

    /// Create a new encryption service with an explicit key version
    pub fn new_with_version(key_hex: &str, version: u32) -> Result<Self, EncryptionError> {
        if key_hex.len() != 64 {
            return Err(EncryptionError::InvalidKey(format!(
                "Encryption key must be 64 hex characters (32 bytes), got {}",
                key_hex.len()
            )));
        }

        let mut key = [0u8; 32];
        hex::decode_to_slice(key_hex, &mut key)
            .map_err(|e| EncryptionError::InvalidKey(format!("Invalid hex key: {}", e)))?;

        Ok(Self {
            primary_key: key,
            key_version: version,
            previous_keys: HashMap::new(),
        })
    }

    /// Add a historical key version to support decrypting data encrypted prior to key rotation.
    pub fn add_previous_key(&mut self, version: u32, key_hex: &str) -> Result<(), EncryptionError> {
        if key_hex.len() != 64 {
            return Err(EncryptionError::InvalidKey(format!(
                "Encryption key must be 64 hex characters (32 bytes), got {}",
                key_hex.len()
            )));
        }

        let mut key = [0u8; 32];
        hex::decode_to_slice(key_hex, &mut key)
            .map_err(|e| EncryptionError::InvalidKey(format!("Invalid hex key: {}", e)))?;

        self.previous_keys.insert(version, key);
        Ok(())
    }

    /// Builder method to register a historical key version for key rotation.
    pub fn with_previous_key(mut self, version: u32, key_hex: &str) -> Result<Self, EncryptionError> {
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
    pub fn encrypt(&self, plaintext: &str) -> Result<EncryptedData, EncryptionError> {
        let cipher = Aes256Gcm::new((&self.primary_key).into());

        // Generate random 96-bit nonce (standard for GCM)
        let mut rng = rand::rng();
        let nonce_bytes: [u8; 12] = rng.random();
        let nonce = Nonce::from(nonce_bytes);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

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
    ///
    /// All malformed input (bad hex, wrong nonce length, tampered ciphertext,
    /// unknown key version) is returned as an [`EncryptionError`] — never a panic.
    pub fn decrypt(&self, encrypted: &EncryptedData) -> Result<String, EncryptionError> {
        let key_bytes: &[u8; 32] = if encrypted.key_version == self.key_version {
            &self.primary_key
        } else if let Some(old_key) = self.previous_keys.get(&encrypted.key_version) {
            old_key
        } else {
            return Err(EncryptionError::KeyVersionMismatch {
                required: encrypted.key_version,
                available: self.key_version,
            });
        };

        let cipher = Aes256Gcm::new(key_bytes.into());

        let nonce_bytes = hex::decode(&encrypted.nonce)
            .map_err(|e| EncryptionError::InvalidNonceHex(e.to_string()))?;
        if nonce_bytes.len() != 12 {
            return Err(EncryptionError::InvalidNonceLength(nonce_bytes.len()));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext_bytes = hex::decode(&encrypted.ciphertext)
            .map_err(|e| EncryptionError::InvalidCiphertextHex(e.to_string()))?;

        let plaintext_bytes = cipher
            .decrypt(nonce, ciphertext_bytes.as_slice())
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        String::from_utf8(plaintext_bytes)
            .map_err(|e| EncryptionError::InvalidUtf8(e.to_string()))
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
        // because of rand
        assert_ne!(encrypted1.nonce, encrypted2.nonce);
        assert_ne!(encrypted1.ciphertext, encrypted2.ciphertext);
    }

    #[test]
    fn decrypt_rejects_invalid_nonce_hex() {
        let service = EncryptionService::new(&test_key()).unwrap();
        let encrypted = EncryptedData {
            ciphertext: "00".to_string(),
            nonce: "not-hex".to_string(),
            key_version: 1,
        };
        assert!(matches!(
            service.decrypt(&encrypted),
            Err(EncryptionError::InvalidNonceHex(_))
        ));
    }

    #[test]
    fn decrypt_rejects_wrong_nonce_length() {
        let service = EncryptionService::new(&test_key()).unwrap();
        let encrypted = EncryptedData {
            ciphertext: "00".to_string(),
            nonce: "00112233".to_string(),
            key_version: 1,
        };
        assert!(matches!(
            service.decrypt(&encrypted),
            Err(EncryptionError::InvalidNonceLength(4))
        ));
    }

    #[test]
    fn decrypt_rejects_invalid_ciphertext_hex() {
        let service = EncryptionService::new(&test_key()).unwrap();
        let encrypted = EncryptedData {
            ciphertext: "zzzz".to_string(),
            nonce: "00112233445566778899aabb".to_string(),
            key_version: 1,
        };
        assert!(matches!(
            service.decrypt(&encrypted),
            Err(EncryptionError::InvalidCiphertextHex(_))
        ));
    }

    #[test]
    fn decrypt_rejects_tampered_ciphertext() {
        let service = EncryptionService::new(&test_key()).unwrap();
        let mut encrypted = service.encrypt("secret").unwrap();
        // Flip a byte in the ciphertext to break the auth tag.
        let mut bytes = hex::decode(&encrypted.ciphertext).unwrap();
        bytes[0] ^= 0xff;
        encrypted.ciphertext = hex::encode(&bytes);
        assert!(matches!(
            service.decrypt(&encrypted),
            Err(EncryptionError::DecryptionFailed(_))
        ));
    }

    #[test]
    fn decrypt_rejects_unknown_key_version() {
        let service = EncryptionService::new(&test_key()).unwrap();
        let encrypted = EncryptedData {
            ciphertext: "00".to_string(),
            nonce: "00112233445566778899aabb".to_string(),
            key_version: 99,
        };
        assert!(matches!(
            service.decrypt(&encrypted),
            Err(EncryptionError::KeyVersionMismatch { required: 99, .. })
        ));
    }

    #[test]
    fn decrypt_random_bytes_never_panics() {
        let service = EncryptionService::new(&test_key()).unwrap();
        let mut rng = rand::rng();
        for _ in 0..1000 {
            let len = rng.random_range(0..64);
            let mut bytes = vec![0u8; len];
            rng.fill(&mut bytes[..]);
            let encrypted = EncryptedData {
                ciphertext: hex::encode(&bytes),
                nonce: hex::encode(&bytes),
                key_version: rng.random_range(0..3),
            };
            // Must never panic; any Result is acceptable.
            let _ = service.decrypt(&encrypted);
        }
    }
}
