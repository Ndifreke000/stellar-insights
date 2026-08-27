/// Cryptographic operations for data encryption at rest
///
/// Provides AES-256-GCM encryption for sensitive fields with:
/// - Authenticated encryption (AEAD mode)
/// - Unique IV per operation (no key reuse vulnerability)
/// - Support for key versioning and rotation
pub mod encryption;

pub use encryption::{EncryptionService, EncryptedData};
