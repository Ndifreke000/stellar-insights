use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// SEP-10 challenge transaction validity duration (5 minutes)
const CHALLENGE_EXPIRY_SECONDS: i64 = 300;

/// SEP-10 session expiry (7 days)
const SESSION_EXPIRY_DAYS: i64 = 7;

/// SEP-10 Challenge Request
#[derive(Debug, Deserialize)]
pub struct ChallengeRequest {
    pub account: String,
    #[serde(default)]
    pub home_domain: Option<String>,
    #[serde(default)]
    pub client_domain: Option<String>,
    #[serde(default)]
    pub memo: Option<String>,
}

/// SEP-10 Challenge Response
#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub transaction: String, // Base64-encoded XDR
    pub network_passphrase: String,
}

/// SEP-10 Verification Request
#[derive(Debug, Deserialize)]
pub struct VerificationRequest {
    pub transaction: String, // Base64-encoded signed XDR
}

/// SEP-10 Verification Response
#[derive(Debug, Serialize)]
pub struct VerificationResponse {
    pub token: String,
    pub expires_in: i64,
}

/// SEP-10 Session Info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sep10Session {
    pub account: String,
    pub client_domain: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
}

/// SEP-10 Authentication Service
///
/// This is a simplified implementation that provides the core SEP-10 functionality.
/// For production use with actual Stellar transaction signing, integrate with stellar-sdk.
pub struct Sep10Service {
    pub server_public_key: String,
    pub network_passphrase: String,
    pub home_domain: String,
    redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
}

impl Sep10Service {
    /// Create new SEP-10 service
    pub fn new(
        server_public_key: String,
        network_passphrase: String,
        home_domain: String,
        redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    ) -> Result<Self> {
        // Validate server public key format (should start with G and be 56 chars)
        if !server_public_key.starts_with('G') || server_public_key.len() != 56 {
            return Err(anyhow!("Invalid server public key format"));
        }

        Ok(Self {
            server_public_key,
            network_passphrase,
            home_domain,
            redis_connection,
        })
    }

    /// Generate SEP-10 challenge transaction
    ///
    /// In a full implementation, this would create a proper Stellar transaction.
    /// This simplified version creates a challenge structure that can be signed.
    pub async fn generate_challenge(&self, request: ChallengeRequest) -> Result<ChallengeResponse> {
        // Validate account address format
        if !request.account.starts_with('G') || request.account.len() != 56 {
            return Err(anyhow!("Invalid account address format"));
        }

        // Validate home domain if provided
        if let Some(ref domain) = request.home_domain {
            if domain != &self.home_domain {
                return Err(anyhow!("Invalid home domain"));
            }
        }

        // Generate random nonce for replay protection
        let nonce = self.generate_nonce();

        // Create challenge structure
        let challenge = serde_json::json!({
            "type": "sep10_challenge",
            "server": self.server_public_key,
            "client": request.account,
            "nonce": nonce,
            "home_domain": self.home_domain,
            "client_domain": request.client_domain,
            "memo": request.memo,
            "timestamp": Utc::now().timestamp(),
            "expires_at": Utc::now().timestamp() + CHALLENGE_EXPIRY_SECONDS,
            "network_passphrase": self.network_passphrase,
        });

        // Encode challenge as base64
        let challenge_json = serde_json::to_string(&challenge)?;
        let transaction_xdr = BASE64.encode(challenge_json.as_bytes());

        // Store challenge in Redis for validation
        self.store_challenge(&request.account, &nonce, CHALLENGE_EXPIRY_SECONDS)
            .await?;

        Ok(ChallengeResponse {
            transaction: transaction_xdr,
            network_passphrase: self.network_passphrase.clone(),
        })
    }

    /// Verify signed challenge transaction
    ///
    /// In a full implementation, this would verify Stellar signatures.
    /// This simplified version validates the challenge structure and nonce.
    pub async fn verify_challenge(
        &self,
        request: VerificationRequest,
    ) -> Result<VerificationResponse> {
        // Decode transaction
        let challenge_bytes = BASE64
            .decode(&request.transaction)
            .map_err(|e| anyhow!("Invalid base64 encoding: {}", e))?;

        let challenge_json =
            String::from_utf8(challenge_bytes).map_err(|e| anyhow!("Invalid UTF-8: {}", e))?;

        let challenge: serde_json::Value =
            serde_json::from_str(&challenge_json).map_err(|e| anyhow!("Invalid JSON: {}", e))?;

        // Validate challenge structure
        let challenge_type = challenge["type"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing challenge type"))?;

        if challenge_type != "sep10_challenge" {
            return Err(anyhow!("Invalid challenge type"));
        }

        // Extract client account
        let client_account = challenge["client"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing client account"))?
            .to_string();

        // Validate expiration
        let expires_at = challenge["expires_at"]
            .as_i64()
            .ok_or_else(|| anyhow!("Missing expiration"))?;

        if Utc::now().timestamp() > expires_at {
            return Err(anyhow!("Challenge expired"));
        }

        // Extract and validate nonce for replay protection
        let nonce = challenge["nonce"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing nonce"))?;

        self.validate_and_consume_challenge(&client_account, nonce)
            .await?;

        // Generate session token
        let token = self.generate_session_token(&client_account)?;

        // Store session
        let client_domain = challenge["client_domain"].as_str().map(|s| s.to_string());
        let session = Sep10Session {
            account: client_account,
            client_domain,
            created_at: Utc::now().timestamp(),
            expires_at: Utc::now().timestamp() + (SESSION_EXPIRY_DAYS * 24 * 60 * 60),
        };

        self.store_session(&token, &session).await?;

        Ok(VerificationResponse {
            token,
            expires_in: SESSION_EXPIRY_DAYS * 24 * 60 * 60,
        })
    }

    /// Validate session token
    pub async fn validate_session(&self, token: &str) -> Result<Sep10Session> {
        let session = self.get_session(token).await?;

        // Check expiration
        if session.expires_at < Utc::now().timestamp() {
            self.invalidate_session(token).await?;
            return Err(anyhow!("Session expired"));
        }

        Ok(session)
    }

    /// Invalidate session (logout)
    pub async fn invalidate_session(&self, token: &str) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("sep10:session:{}", token);
            conn.del::<_, ()>(&key)
                .await
                .map_err(|e| anyhow!("Failed to invalidate session: {}", e))?;
        }
        Ok(())
    }

    // Private helper methods

    fn generate_nonce(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let nonce: [u8; 32] = rng.gen();
        BASE64.encode(&nonce)
    }

    fn generate_session_token(&self, account: &str) -> Result<String> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 32] = rng.gen();
        let token = format!("{}:{}", account, BASE64.encode(&random_bytes));
        Ok(BASE64.encode(token.as_bytes()))
    }

    async fn store_challenge(&self, account: &str, nonce: &str, expiry: i64) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("sep10:challenge:{}:{}", account, nonce);
            conn.set_ex::<_, _, ()>(&key, "1", expiry as u64)
                .await
                .map_err(|e| anyhow!("Failed to store challenge: {}", e))?;
        }
        Ok(())
    }

    async fn validate_and_consume_challenge(&self, account: &str, nonce: &str) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("sep10:challenge:{}:{}", account, nonce);

            // Check if challenge exists
            let exists: bool = conn
                .exists(&key)
                .await
                .map_err(|e| anyhow!("Failed to check challenge: {}", e))?;

            if !exists {
                return Err(anyhow!("Challenge not found or already used"));
            }

            // Delete challenge (consume it)
            conn.del::<_, ()>(&key)
                .await
                .map_err(|e| anyhow!("Failed to consume challenge: {}", e))?;
        }
        Ok(())
    }

    async fn store_session(&self, token: &str, session: &Sep10Session) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("sep10:session:{}", token);
            let session_json = serde_json::to_string(session)?;
            let expiry = SESSION_EXPIRY_DAYS * 24 * 60 * 60;

            conn.set_ex::<_, _, ()>(&key, session_json, expiry as u64)
                .await
                .map_err(|e| anyhow!("Failed to store session: {}", e))?;
        }
        Ok(())
    }

    async fn get_session(&self, token: &str) -> Result<Sep10Session> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("sep10:session:{}", token);

            let session_json: Option<String> = conn
                .get(&key)
                .await
                .map_err(|e| anyhow!("Failed to get session: {}", e))?;

            if let Some(json) = session_json {
                let session: Sep10Session = serde_json::from_str(&json)?;
                return Ok(session);
            }
        }
        Err(anyhow!("Session not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_challenge() {
        let redis_conn = Arc::new(RwLock::new(None));
        let service = Sep10Service::new(
            "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            "Test SDF Network ; September 2015".to_string(),
            "example.com".to_string(),
            redis_conn,
        )
        .unwrap();

        let request = ChallengeRequest {
            account: "GCLIENTXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            home_domain: Some("example.com".to_string()),
            client_domain: None,
            memo: None,
        };

        let result = service.generate_challenge(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.transaction.is_empty());
        assert_eq!(
            response.network_passphrase,
            "Test SDF Network ; September 2015"
        );
    }

    #[tokio::test]
    async fn test_invalid_account_format() {
        let redis_conn = Arc::new(RwLock::new(None));
        let service = Sep10Service::new(
            "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            "Test SDF Network ; September 2015".to_string(),
            "example.com".to_string(),
            redis_conn,
        )
        .unwrap();

        let request = ChallengeRequest {
            account: "INVALID".to_string(),
            home_domain: Some("example.com".to_string()),
            client_domain: None,
            memo: None,
        };

        let result = service.generate_challenge(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_home_domain() {
        let redis_conn = Arc::new(RwLock::new(None));
        let service = Sep10Service::new(
            "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            "Test SDF Network ; September 2015".to_string(),
            "example.com".to_string(),
            redis_conn,
        )
        .unwrap();

        let request = ChallengeRequest {
            account: "GCLIENTXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            home_domain: Some("wrong.com".to_string()),
            client_domain: None,
            memo: None,
        };

        let result = service.generate_challenge(request).await;
        assert!(result.is_err());
    }
}
