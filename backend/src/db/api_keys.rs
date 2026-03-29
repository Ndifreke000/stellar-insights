use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::api_key::{
    generate_api_key, hash_api_key, ApiKey, ApiKeyInfo, CreateApiKeyRequest, CreateApiKeyResponse,
};

pub struct ApiKeyDb {
    pool: SqlitePool,
}

impl ApiKeyDb {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        wallet_address: &str,
        req: CreateApiKeyRequest,
    ) -> Result<CreateApiKeyResponse> {
        let id = Uuid::new_v4().to_string();
        let (plain_key, prefix, key_hash) = generate_api_key();
        let scopes = req.scopes.unwrap_or_else(|| "read".to_string());
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO api_keys (id, name, key_prefix, key_hash, wallet_address, scopes, status, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, 'active', $7, $8)",
        )
        .bind(&id)
        .bind(&req.name)
        .bind(&prefix)
        .bind(&key_hash)
        .bind(wallet_address)
        .bind(&scopes)
        .bind(&now)
        .bind(&req.expires_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to insert API key for wallet: {wallet_address}"))?;

        let key = sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE id = $1")
            .bind(&id)
            .fetch_one(&self.pool)
            .await
            .with_context(|| format!("Failed to fetch newly created API key: {id}"))?;

        Ok(CreateApiKeyResponse {
            key: ApiKeyInfo::from(key),
            plain_key,
        })
    }

    pub async fn list(&self, wallet_address: &str) -> Result<Vec<ApiKeyInfo>> {
        let keys = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE wallet_address = $1 ORDER BY created_at DESC",
        )
        .bind(wallet_address)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to list API keys for wallet: {wallet_address}"))?;
        Ok(keys.into_iter().map(ApiKeyInfo::from).collect())
    }

    pub async fn get_by_id(&self, id: &str, wallet_address: &str) -> Result<Option<ApiKeyInfo>> {
        let key = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE id = $1 AND wallet_address = $2",
        )
        .bind(id)
        .bind(wallet_address)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to get API key {id} for wallet: {wallet_address}"))?;
        Ok(key.map(ApiKeyInfo::from))
    }

    pub async fn validate(&self, plain_key: &str) -> Result<Option<ApiKey>> {
        let key_hash = hash_api_key(plain_key);
        let key = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE key_hash = $1 AND status = 'active'",
        )
        .bind(&key_hash)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to validate API key")?;

        let Some(ref k) = key else {
            return Ok(None);
        };

        if let Some(ref expires_at) = k.expires_at {
            match DateTime::parse_from_rfc3339(expires_at) {
                Ok(exp) if exp < Utc::now() => return Ok(None),
                Err(e) => {
                    tracing::warn!(
                        "API key {} has malformed expires_at '{}': {}. Treating as expired.",
                        k.id, expires_at, e
                    );
                    return Ok(None);
                }
                _ => {}
            }
        }

        let _ = sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE id = $2")
            .bind(Utc::now().to_rfc3339())
            .bind(&k.id)
            .execute(&self.pool)
            .await;

        Ok(key)
    }

    pub async fn revoke(&self, id: &str, wallet_address: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE api_keys SET status = 'revoked', revoked_at = $1
             WHERE id = $2 AND wallet_address = $3 AND status = 'active'",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .bind(wallet_address)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn rotate(
        &self,
        id: &str,
        wallet_address: &str,
    ) -> Result<Option<CreateApiKeyResponse>> {
        let old_key = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE id = $1 AND wallet_address = $2 AND status = 'active'",
        )
        .bind(id)
        .bind(wallet_address)
        .fetch_optional(&self.pool)
        .await?;

        let Some(old_key) = old_key else {
            return Ok(None);
        };

        self.revoke(id, wallet_address).await?;

        let new_key = self
            .create(
                wallet_address,
                CreateApiKeyRequest {
                    name: old_key.name,
                    scopes: Some(old_key.scopes),
                    expires_at: old_key.expires_at,
                },
            )
            .await?;

        Ok(Some(new_key))
    }
}
