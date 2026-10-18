use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Scope granting read-only access to GET endpoints.
pub const SCOPE_READ: &str = "read";
/// Scope granting access to the preflight endpoint.
pub const SCOPE_PREFLIGHT: &str = "preflight";
/// Scope granting full read/write access.
pub const SCOPE_WRITE: &str = "write";

/// All scopes recognised by the system.
pub const ALL_SCOPES: &[&str] = &[SCOPE_READ, SCOPE_PREFLIGHT, SCOPE_WRITE];

/// Default scopes assigned when a key is created without explicit scopes.
pub const DEFAULT_SCOPES: &str = SCOPE_READ;

/// Default per-key rate limit (requests per minute) when none is configured.
pub const DEFAULT_RATE_LIMIT_PER_MINUTE: i64 = 60;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub wallet_address: String,
    pub scopes: String,
    pub rate_limit_per_minute: i64,
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
    pub rate_limit_per_minute: i64,
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
            rate_limit_per_minute: key.rate_limit_per_minute,
            status: key.status,
            created_at: key.created_at,
            last_used_at: key.last_used_at,
            expires_at: key.expires_at,
            revoked_at: key.revoked_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Validate, utoipa::ToSchema)]
pub struct CreateApiKeyRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "name must be between 1 and 100 characters"
    ))]
    pub name: String,
    pub scopes: Option<String>,
    pub rate_limit_per_minute: Option<i64>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateApiKeyResponse {
    pub key: ApiKeyInfo,
    pub plain_key: String,
}

/// Parse a comma-separated scope string into a normalised list of scopes.
///
/// Unknown scopes are dropped; if nothing valid remains the default scope is
/// returned so a key is never created with no permissions at all.
#[must_use]
pub fn parse_scopes(raw: &str) -> Vec<String> {
    let mut scopes: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter(|s| ALL_SCOPES.contains(s))
        .map(ToString::to_string)
        .collect();
    scopes.sort();
    scopes.dedup();
    if scopes.is_empty() {
        scopes.push(DEFAULT_SCOPES.to_string());
    }
    scopes
}

/// Normalise a user-supplied scope string for storage.
#[must_use]
pub fn normalise_scopes(raw: Option<&str>) -> String {
    match raw {
        Some(s) => parse_scopes(s).join(","),
        None => DEFAULT_SCOPES.to_string(),
    }
}

/// Returns true if the stored scope string grants `required`.
///
/// The `write` scope implies `read` and `preflight`.
#[must_use]
pub fn has_scope(stored_scopes: &str, required: &str) -> bool {
    let granted = parse_scopes(stored_scopes);
    if granted.iter().any(|s| s == required) {
        return true;
    }
    if granted.iter().any(|s| s == SCOPE_WRITE) {
        return matches!(required, SCOPE_READ | SCOPE_PREFLIGHT | SCOPE_WRITE);
    }
    false
}

/// Resolve the effective per-key rate limit, falling back to the default.
#[must_use]
pub fn effective_rate_limit(configured: i64) -> i64 {
    if configured > 0 {
        configured
    } else {
        DEFAULT_RATE_LIMIT_PER_MINUTE
    }
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
    // argon2 0.6's hash_password generates its own random salt internally
    // (via the getrandom feature, on by default) -- password-hash 0.6
    // dropped the separate SaltString type this used to take explicitly.
    Argon2::default()
        .hash_password(plain_key.as_bytes())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scopes_drops_unknown_and_dedups() {
        assert_eq!(parse_scopes("read, bogus, read"), vec!["read".to_string()]);
    }

    #[test]
    fn parse_scopes_falls_back_to_default() {
        assert_eq!(parse_scopes("nonsense"), vec![DEFAULT_SCOPES.to_string()]);
    }

    #[test]
    fn write_scope_implies_read_and_preflight() {
        assert!(has_scope("write", SCOPE_READ));
        assert!(has_scope("write", SCOPE_PREFLIGHT));
        assert!(has_scope("write", SCOPE_WRITE));
    }

    #[test]
    fn read_only_key_cannot_write() {
        assert!(has_scope("read", SCOPE_READ));
        assert!(!has_scope("read", SCOPE_WRITE));
        assert!(!has_scope("read", SCOPE_PREFLIGHT));
    }

    #[test]
    fn effective_rate_limit_falls_back() {
        assert_eq!(effective_rate_limit(0), DEFAULT_RATE_LIMIT_PER_MINUTE);
        assert_eq!(effective_rate_limit(120), 120);
    }
}
