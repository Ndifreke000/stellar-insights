use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::network_context_middleware::NetworkContext;

#[derive(Clone, Debug)]
pub struct TokenInfo {
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub refresh_token: String,
}

#[derive(Clone)]
pub struct JWTTokenRefresh {
    tokens: Arc<RwLock<std::collections::HashMap<String, TokenInfo>>>,
    expiry_duration_secs: i64,
}

impl JWTTokenRefresh {
    pub fn new(expiry_duration_secs: i64) -> Self {
        Self {
            tokens: Arc::new(RwLock::new(std::collections::HashMap::new())),
            expiry_duration_secs,
        }
    }

    pub async fn issue_token(
        &self,
        client_id: &str,
        _context: &NetworkContext,
    ) -> Result<TokenInfo> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(self.expiry_duration_secs);

        let token = TokenInfo {
            token: format!(
                "token_{}_{}_{}",
                client_id,
                now.timestamp(),
                uuid::Uuid::new_v4()
            ),
            expires_at,
            refresh_token: format!(
                "refresh_{}_{}_{}",
                client_id,
                now.timestamp(),
                uuid::Uuid::new_v4()
            ),
        };

        let mut tokens = self.tokens.write().await;
        tokens.insert(client_id.to_string(), token.clone());

        tracing::info!(
            client_id = client_id,
            expires_at = ?expires_at,
            "Token issued"
        );

        Ok(token)
    }

    pub async fn refresh_token(
        &self,
        client_id: &str,
        refresh_token: &str,
        _context: &NetworkContext,
    ) -> Result<TokenInfo> {
        let mut tokens = self.tokens.write().await;

        let existing = tokens
            .get(client_id)
            .ok_or_else(|| anyhow!("Client token not found"))?;

        if existing.refresh_token != refresh_token {
            return Err(anyhow!("Invalid refresh token"));
        }

        let now = Utc::now();
        let expires_at = now + Duration::seconds(self.expiry_duration_secs);

        let new_token = TokenInfo {
            token: format!(
                "token_{}_{}_{}",
                client_id,
                now.timestamp(),
                uuid::Uuid::new_v4()
            ),
            expires_at,
            refresh_token: format!(
                "refresh_{}_{}_{}",
                client_id,
                now.timestamp(),
                uuid::Uuid::new_v4()
            ),
        };

        tokens.insert(client_id.to_string(), new_token.clone());

        tracing::info!(
            client_id = client_id,
            expires_at = ?expires_at,
            "Token refreshed"
        );

        Ok(new_token)
    }

    pub async fn validate_token(&self, client_id: &str, token: &str) -> Result<bool> {
        let tokens = self.tokens.read().await;

        let token_info = tokens
            .get(client_id)
            .ok_or_else(|| anyhow!("Token not found"))?;

        if token_info.token != token {
            return Err(anyhow!("Invalid token"));
        }

        if Utc::now() > token_info.expires_at {
            return Ok(false);
        }

        tracing::debug!(client_id = client_id, "Token validated successfully");
        Ok(true)
    }

    pub async fn revoke_token(&self, client_id: &str) -> Result<()> {
        let mut tokens = self.tokens.write().await;
        tokens.remove(client_id);

        tracing::info!(client_id = client_id, "Token revoked");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_issue_token() {
        let jwt = JWTTokenRefresh::new(3600);
        let ctx = NetworkContext::testnet();

        let result = jwt.issue_token("client1", &ctx).await;
        assert!(result.is_ok());

        let token_info = result.unwrap();
        assert!(!token_info.token.is_empty());
        assert!(!token_info.refresh_token.is_empty());
    }

    #[tokio::test]
    async fn test_refresh_token_success() {
        let jwt = JWTTokenRefresh::new(3600);
        let ctx = NetworkContext::testnet();

        let issued = jwt.issue_token("client2", &ctx).await.unwrap();
        let refreshed = jwt
            .refresh_token("client2", &issued.refresh_token, &ctx)
            .await;

        assert!(refreshed.is_ok());
        let new_token = refreshed.unwrap();
        assert_ne!(new_token.token, issued.token);
    }

    #[tokio::test]
    async fn test_refresh_token_invalid_refresh_token() {
        let jwt = JWTTokenRefresh::new(3600);
        let ctx = NetworkContext::testnet();

        let _ = jwt.issue_token("client3", &ctx).await;
        let result = jwt
            .refresh_token("client3", "invalid_refresh_token", &ctx)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token_success() {
        let jwt = JWTTokenRefresh::new(3600);
        let ctx = NetworkContext::testnet();

        let token_info = jwt.issue_token("client4", &ctx).await.unwrap();
        let result = jwt.validate_token("client4", &token_info.token).await;

        assert!(result.is_ok() && result.unwrap());
    }

    #[tokio::test]
    async fn test_validate_token_invalid() {
        let jwt = JWTTokenRefresh::new(3600);

        let result = jwt.validate_token("client5", "invalid_token").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let jwt = JWTTokenRefresh::new(3600);
        let ctx = NetworkContext::testnet();

        let token_info = jwt.issue_token("client6", &ctx).await.unwrap();
        let _ = jwt.revoke_token("client6").await;

        let result = jwt.validate_token("client6", &token_info.token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mainnet_context() {
        let jwt = JWTTokenRefresh::new(3600);
        let ctx = NetworkContext::mainnet();

        let result = jwt.issue_token("client7", &ctx).await;
        assert!(result.is_ok());
    }
}
