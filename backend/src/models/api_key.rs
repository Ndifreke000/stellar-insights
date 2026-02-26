use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

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

#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateApiKeyResponse {
    pub key: ApiKeyInfo,
    pub plain_key: String,
}

pub fn generate_api_key() -> (String, String, String) {
    let raw = Uuid::new_v4().to_string().replace('-', "");
    let plain_key = format!("si_live_{}", raw);
    let prefix = format!("si_live_{}...", &raw[..8.min(raw.len())]);
    let hash = hash_api_key(&plain_key);
    (plain_key, prefix, hash)
}

pub fn hash_api_key(plain_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(plain_key.as_bytes());
    hex::encode(hasher.finalize())
}
