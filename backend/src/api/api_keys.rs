use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

/// Prefix that identifies a live API key.
const KEY_PREFIX: &str = "pr_live_";
/// Number of hex chars kept in the clear for lookup.
const PREFIX_LEN: usize = 8;
/// Number of random bytes in the secret (hex-encoded => 32 chars).
const SECRET_BYTES: usize = 16;

#[derive(Debug, FromRow, Serialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: String,
    pub name: String,
    pub prefix: String,
    /// Full key, returned exactly once at creation time.
    pub key: String,
    pub created_at: DateTime<Utc>,
}

/// Generate a new key: `pr_live_<prefix8>_<secret32>`.
/// Returns (full_key, prefix, sha256_hex_of_secret).
fn generate_key() -> (String, String, String) {
    let prefix = Uuid::new_v4().simple().to_string()[..PREFIX_LEN].to_string();
    let secret = Uuid::new_v4().simple().to_string()[..SECRET_BYTES * 2].to_string();
    let full = format!("{}{}_{}", KEY_PREFIX, prefix, secret);
    let hash = hash_secret(&secret);
    (full, prefix, hash)
}

fn hash_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Split a presented key into (prefix, secret). Rejects malformed input.
fn parse_key(key: &str) -> Option<(String, String)> {
    let rest = key.strip_prefix(KEY_PREFIX)?;
    let (prefix, secret) = rest.split_once('_')?;
    if prefix.len() != PREFIX_LEN || secret.len() != SECRET_BYTES * 2 {
        return None;
    }
    if !prefix.chars().all(|c| c.is_ascii_hexdigit())
        || !secret.chars().all(|c| c.is_ascii_hexdigit())
    {
        return None;
    }
    Some((prefix.to_string(), secret.to_string()))
}

/// Constant-time comparison of two equal-length byte slices.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Verify a presented key against the stored prefix + hash.
/// Returns the matching key id when valid and not revoked.
pub async fn verify_key(state: &Arc<AppState>, presented: &str) -> Option<String> {
    let (prefix, secret) = parse_key(presented)?;
    let row: Option<(String, String, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, key_hash, revoked_at FROM api_keys WHERE prefix = ?",
    )
    .bind(&prefix)
    .fetch_optional(&state.db)
    .await
    .ok()?;

    let (id, stored_hash, revoked_at) = row?;
    if revoked_at.is_some() {
        return None;
    }
    let presented_hash = hash_secret(&secret);
    if constant_time_eq(presented_hash.as_bytes(), stored_hash.as_bytes()) {
        Some(id)
    } else {
        None
    }
}

pub async fn list_keys(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows: Result<Vec<ApiKey>, _> = sqlx::query_as(
        "SELECT id, name, prefix, created_at, last_used_at, revoked_at FROM api_keys ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(keys) => (StatusCode::OK, Json(keys)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to list api keys" })),
        )
            .into_response(),
    }
}

pub async fn create_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateApiKeyRequest>,
) -> impl IntoResponse {
    let name = req.name.trim();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "name is required" })),
        )
            .into_response();
    }

    let (full_key, prefix, key_hash) = generate_key();
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let result = sqlx::query(
        "INSERT INTO api_keys (id, name, prefix, key_hash, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(&prefix)
    .bind(&key_hash)
    .bind(now)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => (
            StatusCode::CREATED,
            Json(CreateApiKeyResponse {
                id,
                name: name.to_string(),
                prefix,
                key: full_key,
                created_at: now,
            }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create api key" })),
        )
            .into_response(),
    }
}

pub async fn revoke_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query("UPDATE api_keys SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
        .bind(Utc::now())
        .bind(&id)
        .execute(&state.db)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "api key not found" })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to revoke api key" })),
        )
            .into_response(),
    }
}

/// One-time migration: hash any legacy plaintext keys in place.
/// Legacy rows are expected to have a `key` column holding the full key and
/// no `key_hash`. After this runs, no plaintext remains in the table.
pub async fn migrate_plaintext_keys(state: &Arc<AppState>) -> Result<(), sqlx::Error> {
    let legacy: Vec<(String, String)> =
        sqlx::query_as("SELECT id, key FROM api_keys WHERE key_hash IS NULL AND key IS NOT NULL")
            .fetch_all(&state.db)
            .await?;

    for (id, plaintext) in legacy {
        let (prefix, secret) = match parse_key(&plaintext) {
            Some(parts) => parts,
            None => continue,
        };
        let key_hash = hash_secret(&secret);
        sqlx::query("UPDATE api_keys SET prefix = ?, key_hash = ?, key = NULL WHERE id = ?")
            .bind(&prefix)
            .bind(&key_hash)
            .bind(&id)
            .execute(&state.db)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_key_round_trips() {
        let (full, prefix, hash) = generate_key();
        let (parsed_prefix, secret) = parse_key(&full).expect("valid key");
        assert_eq!(parsed_prefix, prefix);
        assert_eq!(hash_secret(&secret), hash);
        assert!(full.starts_with(KEY_PREFIX));
    }

    #[test]
    fn rejects_malformed_keys() {
        assert!(parse_key("not-a-key").is_none());
        assert!(parse_key("pr_live_short_secret").is_none());
        assert!(parse_key("pr_live_zzzzzzzz_00000000000000000000000000000000").is_none());
    }

    #[test]
    fn constant_time_eq_matches() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
