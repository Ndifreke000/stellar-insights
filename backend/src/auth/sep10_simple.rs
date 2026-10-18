use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// SEP-10 challenge transaction validity duration (default: 5 minutes)
fn default_challenge_expiry_seconds() -> i64 {
    300
}

/// SEP-10 session expiry (default: 7 days)
fn default_session_expiry_days() -> i64 {
    7
}

fn challenge_expiry_seconds() -> i64 {
    std::env::var("SEP10_CHALLENGE_EXPIRY_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(default_challenge_expiry_seconds)
}

fn session_expiry_days() -> i64 {
    std::env::var("SEP10_SESSION_EXPIRY_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(default_session_expiry_days)
}

/// SEP-10 Challenge Request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
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
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct VerificationRequest {
    /// The challenge exactly as `generate_challenge` returned it.
    pub transaction: String,
    /// Base64 Ed25519 signature, by the challenged account's key, over the
    /// UTF-8 bytes of `transaction`. Proves the caller holds that account.
    #[serde(default)]
    pub signature: Option<String>,
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

/// SEP-10 Authentication Service (canonical implementation used by API)
///
/// This is the preferred, simplified SEP-10 workflow for this repository.
/// It avoids direct stellar-xdr dependency issues and is the recommended entry point.
///
/// Use `crate::auth::sep10_simple::Sep10Service` in handler wiring.
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

        // Reject placeholder keys (e.g. GXXX...XXX) where all non-G chars are identical
        if server_public_key
            .chars()
            .skip(1)
            .all(|c| c == server_public_key.chars().nth(1).unwrap_or('X'))
        {
            return Err(anyhow!(
                "SEP10_SERVER_PUBLIC_KEY is a placeholder. Set a real Stellar public key."
            ));
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
    /// TODO #2326: Implement full SEP-10 Stellar transaction generation
    /// Currently simplified: creates JSON challenge structure instead of proper Stellar transaction envelope
    /// Required for production: Build actual Stellar transaction using stellar-sdk, sign with server key
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
            "expires_at": Utc::now().timestamp() + challenge_expiry_seconds(),
            "network_passphrase": self.network_passphrase,
        });

        // Encode challenge as base64
        let challenge_json = serde_json::to_string(&challenge)?;
        let transaction_xdr = BASE64.encode(challenge_json.as_bytes());

        // Store challenge in Redis for validation
        self.store_challenge(&request.account, &nonce, challenge_expiry_seconds())
            .await?;

        Ok(ChallengeResponse {
            transaction: transaction_xdr,
            network_passphrase: self.network_passphrase.clone(),
        })
    }

    /// Verify a signed challenge and open a session for its account.
    ///
    /// The caller must sign the challenge with the account's Ed25519 key
    /// (#2326). Before this check existed, returning the challenge unsigned
    /// produced a session for any account the caller chose to name.
    pub async fn verify_challenge(
        &self,
        request: VerificationRequest,
    ) -> Result<VerificationResponse> {
        // Decode transaction
        let challenge_bytes = BASE64
            .decode(&request.transaction)
            .map_err(|e| anyhow!("Invalid base64 encoding: {e}"))?;

        let challenge_json =
            String::from_utf8(challenge_bytes).map_err(|e| anyhow!("Invalid UTF-8: {e}"))?;

        let challenge: serde_json::Value =
            serde_json::from_str(&challenge_json).map_err(|e| anyhow!("Invalid JSON: {e}"))?;

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

        verify_account_signature(
            &client_account,
            request.transaction.as_bytes(),
            request.signature.as_deref(),
        )?;

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
        let client_domain = challenge["client_domain"]
            .as_str()
            .map(std::string::ToString::to_string);
        let session = Sep10Session {
            account: client_account,
            client_domain,
            created_at: Utc::now().timestamp(),
            expires_at: Utc::now().timestamp() + (session_expiry_days() * 24 * 60 * 60),
        };

        self.store_session(&token, &session).await?;

        Ok(VerificationResponse {
            token,
            expires_in: session_expiry_days() * 24 * 60 * 60,
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
            let key = format!("sep10:session:{token}");
            conn.del::<_, ()>(&key)
                .await
                .map_err(|e| anyhow!("Failed to invalidate session: {e}"))?;
        }
        Ok(())
    }

    // Private helper methods

    fn generate_nonce(&self) -> String {
        use rand::RngExt;
        let mut rng = rand::rng();
        let nonce: [u8; 32] = rng.random();
        BASE64.encode(nonce)
    }

    fn generate_session_token(&self, account: &str) -> Result<String> {
        use rand::RngExt;
        let mut rng = rand::rng();
        let random_bytes: [u8; 32] = rng.random();
        let token = format!("{}:{}", account, BASE64.encode(random_bytes));
        Ok(BASE64.encode(token.as_bytes()))
    }

    async fn store_challenge(&self, account: &str, nonce: &str, expiry: i64) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("sep10:challenge:{account}:{nonce}");
            conn.set_ex::<_, _, ()>(&key, "1", expiry as u64)
                .await
                .map_err(|e| anyhow!("Failed to store challenge: {e}"))?;
        }
        Ok(())
    }

    async fn validate_and_consume_challenge(&self, account: &str, nonce: &str) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("sep10:challenge:{account}:{nonce}");

            // Check if challenge exists
            let exists: bool = conn
                .exists(&key)
                .await
                .map_err(|e| anyhow!("Failed to check challenge: {e}"))?;

            if !exists {
                return Err(anyhow!("Challenge not found or already used"));
            }

            // Delete challenge (consume it)
            conn.del::<_, ()>(&key)
                .await
                .map_err(|e| anyhow!("Failed to consume challenge: {e}"))?;
        } else {
            // Fail closed: refuse to validate without Redis (SEC-007)
            tracing::error!(
                "Redis unavailable - refusing SEP-10 challenge validation (fail closed)"
            );
            return Err(anyhow!("Challenge validation service unavailable"));
        }
        Ok(())
    }

    async fn store_session(&self, token: &str, session: &Sep10Session) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("sep10:session:{token}");
            let session_json = serde_json::to_string(session)?;
            let expiry = session_expiry_days() * 24 * 60 * 60;

            conn.set_ex::<_, _, ()>(&key, session_json, expiry as u64)
                .await
                .map_err(|e| anyhow!("Failed to store session: {e}"))?;
        }
        Ok(())
    }

    async fn get_session(&self, token: &str) -> Result<Sep10Session> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("sep10:session:{token}");

            let session_json: Option<String> = conn
                .get(&key)
                .await
                .map_err(|e| anyhow!("Failed to get session: {e}"))?;

            if let Some(json) = session_json {
                let session: Sep10Session = serde_json::from_str(&json)?;
                return Ok(session);
            }
        }
        Err(anyhow!("Session not found"))
    }
}

/// Check that `signature` (base64) is the Ed25519 signature of `message` by
/// the Stellar account `account` (a `G...` public key).
pub fn verify_account_signature(
    account: &str,
    message: &[u8],
    signature: Option<&str>,
) -> Result<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let signature = signature.ok_or_else(|| anyhow!("Missing challenge signature"))?;
    let signature_bytes: [u8; 64] = BASE64
        .decode(signature)
        .map_err(|_| anyhow!("Invalid signature encoding"))?
        .try_into()
        .map_err(|_| anyhow!("Invalid signature length"))?;

    let public_key = stellar_strkey::ed25519::PublicKey::from_string(account)
        .map_err(|_| anyhow!("Invalid account address"))?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key.0).map_err(|_| anyhow!("Invalid account key"))?;

    let signature = Signature::from_bytes(&signature_bytes);

    // Wallets that implement SEP-53 (Freighter's signMessage) sign
    // SHA-256("Stellar Signed Message:\n" + message) rather than the raw
    // bytes. Accept either form; both prove control of the account key.
    let sep53_digest = {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(b"Stellar Signed Message:\n");
        hasher.update(message);
        hasher.finalize()
    };

    if verifying_key.verify(message, &signature).is_ok()
        || verifying_key.verify(&sep53_digest, &signature).is_ok()
    {
        Ok(())
    } else {
        Err(anyhow!("Challenge signature does not match the account"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account_and_key(seed: u8) -> (String, ed25519_dalek::SigningKey) {
        let key = ed25519_dalek::SigningKey::from_bytes(&[seed; 32]);
        let account = stellar_strkey::ed25519::PublicKey(key.verifying_key().to_bytes())
            .to_string()
            .to_string();
        (account, key)
    }

    fn sign(key: &ed25519_dalek::SigningKey, message: &[u8]) -> String {
        use ed25519_dalek::Signer;
        BASE64.encode(key.sign(message).to_bytes())
    }

    #[test]
    fn accepts_the_accounts_own_signature() {
        let (account, key) = account_and_key(7);
        let challenge = b"challenge-bytes";
        let signature = sign(&key, challenge);

        assert!(verify_account_signature(&account, challenge, Some(&signature)).is_ok());
    }

    #[test]
    fn accepts_a_sep53_signed_message() {
        use sha2::Digest;
        let (account, key) = account_and_key(7);
        let challenge = b"challenge-bytes";
        let mut hasher = sha2::Sha256::new();
        hasher.update(b"Stellar Signed Message:\n");
        hasher.update(challenge);
        let signature = sign(&key, &hasher.finalize());

        assert!(verify_account_signature(&account, challenge, Some(&signature)).is_ok());
    }

    #[test]
    fn rejects_a_missing_signature() {
        let (account, _) = account_and_key(7);
        assert!(verify_account_signature(&account, b"challenge", None).is_err());
    }

    #[test]
    fn rejects_a_signature_from_another_account() {
        let (account, _) = account_and_key(7);
        let (_, attacker) = account_and_key(9);
        let challenge = b"challenge-bytes";
        let forged = sign(&attacker, challenge);

        assert!(verify_account_signature(&account, challenge, Some(&forged)).is_err());
    }

    #[test]
    fn rejects_a_signature_over_a_different_challenge() {
        let (account, key) = account_and_key(7);
        let signature = sign(&key, b"an older challenge");

        assert!(verify_account_signature(&account, b"this challenge", Some(&signature)).is_err());
    }

    #[test]
    fn rejects_malformed_input() {
        let (account, _) = account_and_key(7);
        assert!(verify_account_signature(&account, b"c", Some("not base64!")).is_err());
        assert!(verify_account_signature(&account, b"c", Some(&BASE64.encode([0u8; 10]))).is_err());
        assert!(
            verify_account_signature("GNOTAKEY", b"c", Some(&BASE64.encode([0u8; 64]))).is_err()
        );
    }

    #[tokio::test]
    async fn test_generate_challenge() {
        let redis_conn = Arc::new(RwLock::new(None));
        let service = Sep10Service::new(
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
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
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
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
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
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
