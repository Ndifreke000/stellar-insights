//! Per-user data keys (DEKs) for crypto-shredding.
//!
//! Each user gets a random 32-byte DEK, stored only in wrapped form
//! (encrypted with the deployment KEK from `ENCRYPTION_KEY`). PII columns
//! that must survive in backups — currently `sessions.ip_address` and
//! `sessions.device_user_agent` — are encrypted under the user's DEK with a
//! `udk1:` prefix. Destroying the `user_data_keys` row on erasure renders
//! every copy of that ciphertext unrecoverable, including inside backup
//! snapshots, without needing to rewrite archive files.

use anyhow::{anyhow, Result};
use rand::Rng;
use sqlx::SqlitePool;

use crate::crypto::{decrypt_data, encrypt_data, CryptoService};

/// Prefix marking a value as encrypted under a per-user DEK (as opposed to
/// the deployment KEK used by `CryptoService`).
pub const USER_DEK_PREFIX: &str = "udk1:";

#[derive(Clone)]
pub struct UserKeyService {
    pool: SqlitePool,
    /// `None` when no KEK is configured (e.g. minimal test environments);
    /// encryption then degrades to plaintext passthrough.
    kek: Option<CryptoService>,
}

impl UserKeyService {
    pub fn new(pool: SqlitePool, kek: Option<CryptoService>) -> Self {
        Self { pool, kek }
    }

    /// Build from the environment. Never panics: if `ENCRYPTION_KEY` is
    /// unavailable the service runs without field encryption so that
    /// session handling keeps working in stripped-down environments.
    pub fn from_env(pool: SqlitePool) -> Self {
        let kek = std::panic::catch_unwind(CryptoService::from_env).ok();
        if kek.is_none() {
            tracing::warn!("ENCRYPTION_KEY unavailable; per-user field encryption disabled");
        }
        Self::new(pool, kek)
    }

    /// Fetch the user's DEK (hex), creating and wrapping a fresh one on first
    /// use. Returns `None` when no KEK is configured.
    async fn dek_hex(&self, user_id: &str) -> Result<Option<String>> {
        let Some(kek) = &self.kek else {
            return Ok(None);
        };

        if let Some(wrapped) = sqlx::query_scalar::<_, String>(
            "SELECT wrapped_dek FROM user_data_keys WHERE user_id = ? AND destroyed_at IS NULL",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        {
            return kek
                .decrypt(&wrapped)
                .map(Some)
                .map_err(|e| anyhow!("Failed to unwrap user data key: {e}"));
        }

        // Generate a fresh DEK and store it wrapped.
        let mut dek = [0u8; 32];
        rand::rng().fill_bytes(&mut dek);
        let dek_hex = hex::encode(dek);
        let wrapped = kek
            .encrypt(&dek_hex)
            .map_err(|e| anyhow!("Failed to wrap user data key: {e}"))?;

        sqlx::query(
            "INSERT INTO user_data_keys (user_id, wrapped_dek) VALUES (?, ?) \
             ON CONFLICT(user_id) DO UPDATE SET wrapped_dek = excluded.wrapped_dek, destroyed_at = NULL",
        )
        .bind(user_id)
        .bind(&wrapped)
        .execute(&self.pool)
        .await?;

        Ok(Some(dek_hex))
    }

    /// Encrypt a PII field for storage. Returns the plaintext unchanged when
    /// encryption is unavailable or the input is empty.
    pub async fn encrypt_field(&self, user_id: &str, plaintext: &str) -> Result<String> {
        if plaintext.is_empty() {
            return Ok(String::new());
        }
        let Some(dek) = self.dek_hex(user_id).await? else {
            return Ok(plaintext.to_string());
        };
        let ct = encrypt_data(plaintext, &dek)?;
        Ok(format!("{USER_DEK_PREFIX}{ct}"))
    }

    /// Decrypt a stored field. Values without the `udk1:` prefix are legacy
    /// plaintext and pass through; `udk1:` values require the user's DEK.
    pub async fn decrypt_field(&self, user_id: &str, stored: &str) -> Result<String> {
        let Some(rest) = stored.strip_prefix(USER_DEK_PREFIX) else {
            return Ok(stored.to_string());
        };
        let dek = self
            .dek_hex(user_id)
            .await?
            .ok_or_else(|| anyhow!("Per-user field encryption is not configured"))?;
        decrypt_data(rest, &dek)
    }

    /// Convenience wrapper for `Option<String>` fields.
    pub async fn decrypt_opt(
        &self,
        user_id: &str,
        stored: Option<String>,
    ) -> Result<Option<String>> {
        match stored {
            Some(v) => self.decrypt_field(user_id, &v).await.map(Some),
            None => Ok(None),
        }
    }

    /// Crypto-shred: destroy the user's DEK so every ciphertext copy —
    /// live rows and all backup snapshots — becomes unrecoverable.
    /// Returns true when a key row existed.
    pub async fn destroy(&self, user_id: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE user_data_keys SET destroyed_at = datetime('now') \
             WHERE user_id = ? AND destroyed_at IS NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Whether a live DEK exists for the user (used by tests/diagnostics).
    pub async fn has_key(&self, user_id: &str) -> Result<bool> {
        let found = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_data_keys WHERE user_id = ? AND destroyed_at IS NULL",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(found > 0)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    async fn test_pool() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE user_data_keys (\
                user_id TEXT PRIMARY KEY NOT NULL, \
                wrapped_dek TEXT NOT NULL, \
                created_at TEXT NOT NULL DEFAULT (datetime('now')), \
                destroyed_at TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    fn svc(pool: SqlitePool) -> UserKeyService {
        UserKeyService::new(pool, Some(CryptoService::new_for_tests()))
    }

    #[tokio::test]
    async fn encrypt_decrypt_roundtrip() {
        let pool = test_pool().await;
        let svc = svc(pool);
        let ct = svc.encrypt_field("u1", "203.0.113.7").await.unwrap();
        assert!(ct.starts_with(USER_DEK_PREFIX));
        assert_ne!(ct, "203.0.113.7");
        let pt = svc.decrypt_field("u1", &ct).await.unwrap();
        assert_eq!(pt, "203.0.113.7");
    }

    #[tokio::test]
    async fn plaintext_passthrough() {
        let pool = test_pool().await;
        let svc = svc(pool);
        assert_eq!(svc.decrypt_field("u1", "plain").await.unwrap(), "plain");
        assert_eq!(svc.decrypt_field("u1", "").await.unwrap(), "");
    }

    #[tokio::test]
    async fn destroy_makes_ciphertext_undecryptable() {
        let pool = test_pool().await;
        let svc = svc(pool);
        let ct = svc.encrypt_field("u1", "secret-ip").await.unwrap();
        assert!(svc.has_key("u1").await.unwrap());
        assert!(svc.destroy("u1").await.unwrap());
        // A fresh DEK is created on next access; the old ciphertext must not
        // decrypt under it.
        let err = svc.decrypt_field("u1", &ct).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn destroy_without_key_is_noop() {
        let pool = test_pool().await;
        let svc = svc(pool);
        assert!(!svc.destroy("ghost").await.unwrap());
    }
}
