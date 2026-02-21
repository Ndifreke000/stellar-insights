use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// SEP-10 challenge transaction validity duration (5 minutes)
const CHALLENGE_EXPIRY_SECONDS: i64 = 300;

/// SEP-10 session expiry (7 days)
const SESSION_EXPIRY_DAYS: i64 = 7;

/// Minimum time bounds for challenge validation (5 minutes)
const MIN_TIME_BOUNDS: i64 = 300;

/// Maximum time bounds for challenge validation (15 minutes)
const MAX_TIME_BOUNDS: i64 = 900;

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
pub struct Sep10Service {
    server_keypair: KeyPair,
    network_passphrase: String,
    home_domain: String,
    redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
}

impl Sep10Service {
    /// Create new SEP-10 service
    pub fn new(
        server_secret: &str,
        network_passphrase: String,
        home_domain: String,
        redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    ) -> Result<Self> {
        let server_keypair = KeyPair::from_secret_seed(server_secret)
            .map_err(|e| anyhow!("Invalid server secret key: {}", e))?;

        Ok(Self {
            server_keypair,
            network_passphrase,
            home_domain,
            redis_connection,
        })
    }

    /// Generate SEP-10 challenge transaction
    pub async fn generate_challenge(&self, request: ChallengeRequest) -> Result<ChallengeResponse> {
        // Validate account address
        let client_account = PublicKey::from_account_id(&request.account)
            .map_err(|e| anyhow!("Invalid account address: {}", e))?;

        // Validate home domain if provided
        if let Some(ref domain) = request.home_domain {
            if domain != &self.home_domain {
                return Err(anyhow!("Invalid home domain"));
            }
        }

        // Generate random nonce for replay protection
        let nonce = self.generate_nonce();

        // Build challenge transaction
        let now = Utc::now().timestamp();
        let min_time = now;
        let max_time = now + CHALLENGE_EXPIRY_SECONDS;

        let time_bounds = TimeBounds {
            min_time: min_time as u64,
            max_time: max_time as u64,
        };

        // Create ManageData operation with random nonce
        let manage_data_op = Operation {
            source_account: Some(MuxedAccount::from_public_key(&client_account)),
            body: OperationBody::ManageData {
                data_name: format!("{} auth", self.home_domain),
                data_value: Some(nonce.as_bytes().to_vec()),
            },
        };

        // Add Web Auth Domain operation if client_domain provided
        let mut operations = vec![manage_data_op];
        if let Some(ref client_domain) = request.client_domain {
            let web_auth_domain_op = Operation {
                source_account: Some(MuxedAccount::from_public_key(
                    &self.server_keypair.public_key(),
                )),
                body: OperationBody::ManageData {
                    data_name: "web_auth_domain".to_string(),
                    data_value: Some(client_domain.as_bytes().to_vec()),
                },
            };
            operations.push(web_auth_domain_op);
        }

        // Create memo if provided
        let memo = if let Some(memo_text) = request.memo {
            Memo::Text(memo_text)
        } else {
            Memo::None
        };

        // Build transaction
        let transaction = Transaction {
            source_account: MuxedAccount::from_public_key(&self.server_keypair.public_key()),
            fee: 100 * operations.len() as u32,
            seq_num: SequenceNumber(0),
            preconditions: Preconditions {
                time_bounds: Some(time_bounds),
                ..Default::default()
            },
            memo,
            operations,
        };

        // Sign transaction with server key
        let network = Network::new(&self.network_passphrase);
        let tx_hash = transaction.hash(&network)?;
        let server_signature = self.server_keypair.sign(&tx_hash);

        let decorated_sig = DecoratedSignature {
            hint: self.server_keypair.public_key().signature_hint(),
            signature: Signature::from_bytes(&server_signature)?,
        };

        let envelope = TransactionEnvelope::V1 {
            tx: transaction,
            signatures: vec![decorated_sig],
        };

        // Encode to base64 XDR
        let xdr_bytes = envelope.to_xdr()?;
        let transaction_xdr = base64::encode(&xdr_bytes);

        // Store challenge in Redis for validation
        self.store_challenge(&request.account, &nonce, CHALLENGE_EXPIRY_SECONDS)
            .await?;

        Ok(ChallengeResponse {
            transaction: transaction_xdr,
            network_passphrase: self.network_passphrase.clone(),
        })
    }

    /// Verify signed challenge transaction
    pub async fn verify_challenge(
        &self,
        request: VerificationRequest,
    ) -> Result<VerificationResponse> {
        // Decode transaction envelope
        let xdr_bytes = base64::decode(&request.transaction)
            .map_err(|e| anyhow!("Invalid base64 encoding: {}", e))?;

        let envelope = TransactionEnvelope::from_xdr(&xdr_bytes)
            .map_err(|e| anyhow!("Invalid transaction XDR: {}", e))?;

        let (transaction, signatures) = match envelope {
            TransactionEnvelope::V1 { tx, signatures } => (tx, signatures),
            _ => return Err(anyhow!("Unsupported transaction envelope version")),
        };

        // Validate transaction structure
        self.validate_transaction_structure(&transaction)?;

        // Extract client account from first operation
        let client_account = self.extract_client_account(&transaction)?;

        // Validate time bounds
        self.validate_time_bounds(&transaction)?;

        // Validate sequence number (must be 0)
        if transaction.seq_num.0 != 0 {
            return Err(anyhow!("Invalid sequence number"));
        }

        // Verify signatures
        let network = Network::new(&self.network_passphrase);
        let tx_hash = transaction.hash(&network)?;

        // Must have server signature
        let has_server_sig = self.verify_server_signature(&tx_hash, &signatures)?;
        if !has_server_sig {
            return Err(anyhow!("Missing server signature"));
        }

        // Must have client signature
        let has_client_sig = self.verify_client_signature(&tx_hash, &signatures, &client_account)?;
        if !has_client_sig {
            return Err(anyhow!("Missing or invalid client signature"));
        }

        // Extract and validate nonce for replay protection
        let nonce = self.extract_nonce(&transaction)?;
        self.validate_and_consume_challenge(&client_account.account_id(), &nonce)
            .await?;

        // Generate session token
        let token = self.generate_session_token(&client_account.account_id())?;

        // Store session
        let session = Sep10Session {
            account: client_account.account_id(),
            client_domain: self.extract_client_domain(&transaction),
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
        base64::encode(&nonce)
    }

    fn generate_session_token(&self, account: &str) -> Result<String> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 32] = rng.gen();
        let token = format!("{}:{}", account, base64::encode(&random_bytes));
        Ok(base64::encode(token.as_bytes()))
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

    fn validate_transaction_structure(&self, transaction: &Transaction) -> Result<()> {
        // Must have at least one operation
        if transaction.operations.is_empty() {
            return Err(anyhow!("Transaction must have at least one operation"));
        }

        // First operation must be ManageData
        match &transaction.operations[0].body {
            OperationBody::ManageData { data_name, .. } => {
                if !data_name.contains("auth") {
                    return Err(anyhow!("Invalid ManageData operation"));
                }
            }
            _ => return Err(anyhow!("First operation must be ManageData")),
        }

        Ok(())
    }

    fn extract_client_account(&self, transaction: &Transaction) -> Result<PublicKey> {
        if let Some(ref source) = transaction.operations[0].source_account {
            match source {
                MuxedAccount::Ed25519(account_id) => {
                    Ok(PublicKey::from_binary(account_id)?)
                }
                MuxedAccount::MuxedEd25519 { id, .. } => {
                    Ok(PublicKey::from_binary(id)?)
                }
            }
        } else {
            Err(anyhow!("Client account not found in operation"))
        }
    }

    fn validate_time_bounds(&self, transaction: &Transaction) -> Result<()> {
        let time_bounds = transaction
            .preconditions
            .time_bounds
            .as_ref()
            .ok_or_else(|| anyhow!("Time bounds required"))?;

        let now = Utc::now().timestamp() as u64;

        // Check if current time is within bounds
        if now < time_bounds.min_time || now > time_bounds.max_time {
            return Err(anyhow!("Transaction expired or not yet valid"));
        }

        // Validate time bounds duration
        let duration = (time_bounds.max_time - time_bounds.min_time) as i64;
        if duration < MIN_TIME_BOUNDS || duration > MAX_TIME_BOUNDS {
            return Err(anyhow!("Invalid time bounds duration"));
        }

        Ok(())
    }

    fn verify_server_signature(
        &self,
        tx_hash: &[u8],
        signatures: &[DecoratedSignature],
    ) -> Result<bool> {
        let server_public_key = self.server_keypair.public_key();

        for sig in signatures {
            if sig.hint == server_public_key.signature_hint() {
                return Ok(server_public_key.verify(tx_hash, &sig.signature.to_bytes())?);
            }
        }

        Ok(false)
    }

    fn verify_client_signature(
        &self,
        tx_hash: &[u8],
        signatures: &[DecoratedSignature],
        client_account: &PublicKey,
    ) -> Result<bool> {
        for sig in signatures {
            if sig.hint == client_account.signature_hint() {
                return Ok(client_account.verify(tx_hash, &sig.signature.to_bytes())?);
            }
        }

        Ok(false)
    }

    fn extract_nonce(&self, transaction: &Transaction) -> Result<String> {
        match &transaction.operations[0].body {
            OperationBody::ManageData { data_value, .. } => {
                if let Some(value) = data_value {
                    Ok(String::from_utf8(value.clone())?)
                } else {
                    Err(anyhow!("Nonce not found in ManageData operation"))
                }
            }
            _ => Err(anyhow!("Invalid operation type")),
        }
    }

    fn extract_client_domain(&self, transaction: &Transaction) -> Option<String> {
        // Check if there's a second operation with web_auth_domain
        if transaction.operations.len() > 1 {
            if let OperationBody::ManageData { data_name, data_value } =
                &transaction.operations[1].body
            {
                if data_name == "web_auth_domain" {
                    if let Some(value) = data_value {
                        return String::from_utf8(value.clone()).ok();
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_challenge() {
        let redis_conn = Arc::new(RwLock::new(None));
        let service = Sep10Service::new(
            "SALADFINGER...", // Test secret key
            "Test SDF Network ; September 2015".to_string(),
            "example.com".to_string(),
            redis_conn,
        )
        .unwrap();

        let request = ChallengeRequest {
            account: "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            home_domain: Some("example.com".to_string()),
            client_domain: None,
            memo: None,
        };

        let result = service.generate_challenge(request).await;
        assert!(result.is_ok());
    }
}
