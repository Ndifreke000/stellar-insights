use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub wallet_address: String,
    pub scopes: String,
    pub status: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub wallet_address: String,
    pub scopes: String,
    pub status: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

impl From<ApiKey> for ApiKeyInfo {
    fn from(key: ApiKey) -> Self {
        Self {
            id: key.id,
            name: key.name,
            key_prefix: key.key_prefix,
            wallet_address: key.wallet_address,
            scopes: key.scopes,
            status: key.status,
            created_at: key.created_at,
            last_used_at: key.last_used_at,
            expires_at: key.expires_at,
            revoked_at: key.revoked_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[derive(utoipa::ToSchema)]
pub struct CreateApiKeyRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "name must be between 1 and 100 characters"
    ))]
    pub name: String,
    pub scopes: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateApiKeyResponse {
    pub key: ApiKeyInfo,
    pub plain_key: String,
}

#[must_use]
pub fn generate_api_key() -> (String, String, String) {
    let raw = Uuid::new_v4().to_string().replace('-', "");
    let plain_key = format!("si_live_{raw}");
    let prefix = extract_key_prefix(&plain_key);
    let hash = hash_api_key(&plain_key);
    (plain_key, prefix, hash)
}

/// Returns the stored prefix used for efficient DB lookup before Argon2 verify.
#[must_use]
pub fn extract_key_prefix(plain_key: &str) -> String {
    // plain_key format: "si_live_<32-char uuid without hyphens>"
    // We store the first 16 chars + "..." as the prefix.
    let end = plain_key.len().min(16);
    format!("{}...", &plain_key[..end])
}

/// Hash an API key with Argon2id. The PHC-format hash embeds the salt so
/// it can be verified later with `verify_api_key`.
#[must_use]
pub fn hash_api_key(plain_key: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plain_key.as_bytes(), &salt)
        .expect("argon2 hash failed")
        .to_string()
}

/// Verify a plaintext key against a stored Argon2id PHC hash.
#[must_use]
pub fn verify_api_key(plain_key: &str, stored_hash: &str) -> bool {
    let parsed = match PasswordHash::new(stored_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(plain_key.as_bytes(), &parsed)
        .is_ok()
}
