use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::api_key::{
    generate_api_key, ApiKey, ApiKeyInfo, CreateApiKeyRequest, CreateApiKeyResponse,
};

pub struct ApiKeyDb {
    pool: SqlitePool,
}

impl ApiKeyDb {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates a new API key.
    #[tracing::instrument(skip(self, req), fields(wallet_address = %wallet_address, key_name = %req.name))]
    pub async fn create_api_key(
        &self,
        wallet_address: &str,
        req: CreateApiKeyRequest,
    ) -> Result<CreateApiKeyResponse> {
        let id = Uuid::new_v4().to_string();
        let (plain_key, prefix, key_hash) = generate_api_key();
        let scopes = req.scopes.unwrap_or_else(|| "read".to_string());
        let now = Utc::now().to_rfc3339();

        let mut tx = self.pool.begin().await.with_context(|| {
            format!(
                "Failed to begin transaction for create_api_key for wallet: {}",
                wallet_address
            )
        })?;

        sqlx::query(
            r"
            INSERT INTO api_keys (id, name, key_prefix, key_hash, wallet_address, scopes, status, created_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, 'active', $7, $8)
            ",
        )
        .bind(&id)
        .bind(&req.name)
        .bind(&prefix)
        .bind(&key_hash)
        .bind(wallet_address)
        .bind(&scopes)
        .bind(&now)
        .bind(&req.expires_at)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "Failed to insert API key for wallet: {}, name: {}",
                wallet_address, req.name
            )
        })?;

        let key = sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE id = $1")
            .bind(&id)
            .fetch_one(&mut *tx)
            .await
            .with_context(|| format!("Failed to fetch newly created API key with id: {}", id))?;

        tx.commit().await.with_context(|| {
            format!(
                "Failed to commit create_api_key transaction for wallet: {}",
                wallet_address
            )
        })?;

        Ok(CreateApiKeyResponse {
            key: ApiKeyInfo::from(key),
            plain_key,
        })
    }

    /// Lists all API keys for a given wallet address.
    #[tracing::instrument(skip(self), fields(wallet_address = %wallet_address))]
    pub async fn list_api_keys(&self, wallet_address: &str) -> Result<Vec<ApiKeyInfo>> {
        let keys = sqlx::query_as::<_, ApiKey>(
            r"
            SELECT * FROM api_keys
            WHERE wallet_address = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(wallet_address)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to list API keys for wallet: {}", wallet_address))?;
        Ok(keys.into_iter().map(ApiKeyInfo::from).collect())
    }

    /// Retrieves an API key by its ID for a specific wallet address.
    #[tracing::instrument(skip(self), fields(key_id = %id, wallet_address = %wallet_address))]
    pub async fn get_api_key_by_id(
        &self,
        id: &str,
        wallet_address: &str,
    ) -> Result<Option<ApiKeyInfo>> {
        let key = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE id = $1 AND wallet_address = $2",
        )
        .bind(id)
        .bind(wallet_address)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to get API key id: {} for wallet: {}",
                id, wallet_address
            )
        })?;
        Ok(key.map(ApiKeyInfo::from))
    }

    /// Revokes an active API key.
    #[tracing::instrument(skip(self), fields(key_id = %id, wallet_address = %wallet_address))]
    pub async fn revoke_api_key(&self, id: &str, wallet_address: &str) -> Result<bool> {
        let revoked_at = Utc::now().to_rfc3339();
        let result = sqlx::query(
            r"
                UPDATE api_keys
                SET status = 'revoked', revoked_at = $1
                WHERE id = $2 AND wallet_address = $3 AND status = 'active'
                ",
        )
        .bind(&revoked_at)
        .bind(id)
        .bind(wallet_address)
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to revoke API key id: {} for wallet: {}",
                id, wallet_address
            )
        })?;

        Ok(result.rows_affected() > 0)
    }

    /// Rotates an API key: revokes the old one and creates a new one with the same settings.
    #[tracing::instrument(skip(self), fields(key_id = %id, wallet_address = %wallet_address))]
    pub async fn rotate_api_key(
        &self,
        id: &str,
        wallet_address: &str,
    ) -> Result<Option<CreateApiKeyResponse>> {
        let mut tx = self.pool.begin().await.with_context(|| {
            format!(
                "Failed to begin API key rotation transaction for id: {}",
                id
            )
        })?;

        let existing = sqlx::query_as::<_, ApiKey>(
            r"
                SELECT * FROM api_keys
                WHERE id = $1 AND wallet_address = $2 AND status = 'active'
                ",
        )
        .bind(id)
        .bind(wallet_address)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "Failed to fetch API key id: {} for wallet: {} during rotation",
                id, wallet_address
            )
        })?;

        let Some(existing) = existing else {
            return Ok(None);
        };

        let revoked_at = Utc::now().to_rfc3339();
        sqlx::query(
            r"
                UPDATE api_keys
                SET status = 'revoked', revoked_at = $1
                WHERE id = $2
                ",
        )
        .bind(&revoked_at)
        .bind(&existing.id)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "Failed to revoke existing API key during rotation for id: {}",
                existing.id
            )
        })?;

        let new_id = Uuid::new_v4().to_string();
        let (plain_key, prefix, key_hash) = generate_api_key();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r"
                INSERT INTO api_keys (
                    id, name, key_prefix, key_hash, wallet_address, scopes, status, created_at, expires_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, 'active', $7, $8)
                ",
        )
        .bind(&new_id)
        .bind(&existing.name)
        .bind(&prefix)
        .bind(&key_hash)
        .bind(wallet_address)
        .bind(&existing.scopes)
        .bind(&now)
        .bind(&existing.expires_at)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "Failed to insert rotated API key for wallet: {} from id: {}",
                wallet_address, existing.id
            )
        })?;

        let key = sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE id = $1")
            .bind(&new_id)
            .fetch_one(&mut *tx)
            .await
            .with_context(|| format!("Failed to fetch rotated API key with id: {}", new_id))?;

        tx.commit().await.with_context(|| {
            format!(
                "Failed to commit API key rotation transaction for id: {}",
                id
            )
        })?;

        Ok(Some(CreateApiKeyResponse {
            key: ApiKeyInfo::from(key),
            plain_key,
        }))
    }

    /// Validates an API key using the plaintext key.
    ///
    /// Looks up candidates by stored prefix (fast), then verifies each one
    /// with Argon2 (slow, intentional). This avoids a full-table scan while
    /// keeping Argon2's brute-force resistance.
    #[tracing::instrument(skip(self, plain_key))]
    pub async fn validate_api_key(&self, plain_key: &str) -> Result<Option<ApiKey>> {
        use crate::models::api_key::{extract_key_prefix, verify_api_key};

        let prefix = extract_key_prefix(plain_key);

        let candidates = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE key_prefix = $1 AND status = 'active'",
        )
        .bind(&prefix)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query API keys by prefix")?;

        for k in candidates {
            if let Some(ref expires_at) = k.expires_at {
                match DateTime::parse_from_rfc3339(expires_at) {
                    Ok(exp) if exp < Utc::now() => continue,
                    Err(e) => {
                        tracing::warn!(
                            "API key {} has malformed expires_at '{}': {}. Treating as expired.",
                            k.id,
                            expires_at,
                            e
                        );
                        continue;
                    }
                    _ => {}
                }
            }

            if verify_api_key(plain_key, &k.key_hash) {
                // best-effort last_used_at update
                let _ = sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE id = $2")
                    .bind(Utc::now().to_rfc3339())
                    .bind(&k.id)
                    .execute(&self.pool)
                    .await;
                return Ok(Some(k));
            }
        }

        Ok(None)
    }
}
