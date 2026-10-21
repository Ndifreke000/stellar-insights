//! SEP-10 web authentication.
//!
//! Implements the challenge and token steps of SEP-10
//! (<https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md>):
//!
//! * `generate_challenge` builds a real Stellar `TransactionEnvelope` — server
//!   as source, sequence `0`, time bounds, one client-sourced
//!   `ManageData "{home domain} auth"` nonce operation and one server-sourced
//!   `ManageData web_auth_domain` operation — and signs it with
//!   `SEP10_SIGNING_KEY`. The response is base64 XDR, which wallets (Freighter,
//!   xBull, LOBSTR, …) can sign with their normal transaction signer.
//! * `verify_challenge` decodes the client-signed envelope, checks the server
//!   signature, time bounds, operation structure, `web_auth_domain`, the
//!   client signatures against the account's signers and medium threshold
//!   (master key only for unfunded accounts), consumes the nonce once in
//!   Redis (fail-closed), and opens a session.
//!
//! Threat model: without this, the "challenge" was an unsigned JSON blob the
//! client could rewrite (expiry, domains) and no mainstream wallet could sign
//! it at all, so nothing proved the caller controlled the account under the
//! SEP-10 guarantees (server signature, time bounds, home domain, single use).

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use ed25519_dalek::{Signature as DalekSignature, Signer, SigningKey, Verifier, VerifyingKey};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use stellar_xdr::curr::{
    DataValue, DecoratedSignature, Hash, Limits, ManageDataOp, Memo, MuxedAccount, Operation,
    OperationBody, Preconditions, ReadXdr, SequenceNumber, Signature, String64, TimeBounds,
    TimePoint, Transaction, TransactionEnvelope, TransactionExt, TransactionSignaturePayload,
    TransactionSignaturePayloadTaggedTransaction, TransactionV1Envelope, WriteXdr,
};

/// SEP-10 challenges are never submitted to the network; the fee only has to
/// be a valid `uint32` (the Stellar base fee).
const CHALLENGE_FEE: u32 = 100;

/// SEP-10 nonce: 48 random bytes, base64-encoded into the 64-byte
/// `ManageData` value the spec requires.
const NONCE_BYTES: usize = 48;

/// Tolerated difference between the clock that issued a challenge and the one
/// verifying it when checking `min_time`. `max_time` stays strict so expired
/// challenges are always rejected.
const CLOCK_SKEW_SECONDS: i64 = 30;

/// How long Horizon is given to answer an account (signers) lookup.
const HORIZON_TIMEOUT_SECONDS: u64 = 5;

/// SEP-10 challenge validity window (default: 15 minutes, the duration the
/// spec recommends).
fn default_challenge_expiry_seconds() -> i64 {
    900
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
        .max(1)
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
    /// Client-domain verification (SEP-10 §"Client Domain") is not implemented
    /// by this server; requests carrying it are rejected.
    #[serde(default)]
    pub client_domain: Option<String>,
    /// Optional memo for accounts that share one key. Only ID memos (a
    /// uint64 string) are supported, per SEP-10.
    #[serde(default)]
    pub memo: Option<String>,
}

/// SEP-10 Challenge Response
#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    /// Base64-encoded, server-signed `TransactionEnvelope` XDR.
    pub transaction: String,
    pub network_passphrase: String,
}

/// SEP-10 Verification Request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct VerificationRequest {
    /// The challenge transaction, signed by the client's wallet, exactly as
    /// `generate_challenge` returned it. Signatures travel inside the
    /// envelope; SEP-10 takes no separate signature field.
    pub transaction: String,
}

/// SEP-10 Verification Response
#[derive(Debug, Serialize)]
pub struct VerificationResponse {
    pub token: String,
    pub expires_in: i64,
}

/// A challenge that passed every structural, server-signature and
/// client-signature check, before nonce consumption.
#[derive(Debug, Clone)]
pub struct VerifiedChallenge {
    /// The `G...` account the challenge was issued for (first operation's
    /// source account).
    pub client_account: String,
    /// Hex of the 48 random nonce bytes from the first operation's value.
    pub nonce: String,
    /// ID memo carried by the challenge, when the client requested one.
    pub memo: Option<u64>,
}

/// SEP-10 Session Info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sep10Session {
    pub account: String,
    #[serde(default)]
    pub client_domain: Option<String>,
    /// ID memo the challenge was issued with, when present.
    #[serde(default)]
    pub memo: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
}

/// Where issued nonces are recorded so each challenge can be redeemed once.
///
/// Production uses Redis `SET ... NX EX` (atomic single-use); tests use an
/// in-memory map. Verification fails closed when Redis is configured but
/// unavailable (SEC-007).
enum NonceLedger {
    Redis(Arc<RwLock<Option<MultiplexedConnection>>>),
    #[cfg(test)]
    Memory(Arc<std::sync::Mutex<std::collections::HashMap<String, i64>>>),
}

impl NonceLedger {
    /// Record `key` for `ttl_seconds`. Errors if the key was already recorded
    /// (replay) or the ledger is unavailable.
    async fn consume(&self, key: &str, ttl_seconds: i64) -> Result<()> {
        match self {
            Self::Redis(handle) => {
                let connection = handle.read().await.as_ref().map(Clone::clone);
                let Some(conn) = connection else {
                    tracing::error!(
                        "Redis unavailable - refusing SEP-10 challenge validation (fail closed)"
                    );
                    return Err(anyhow!("Challenge validation service unavailable"));
                };
                let mut conn = conn;
                let result: redis::RedisResult<Option<String>> = redis::cmd("SET")
                    .arg(key)
                    .arg("1")
                    .arg("NX")
                    .arg("EX")
                    .arg(ttl_seconds.max(1))
                    .query_async(&mut conn)
                    .await;
                match result {
                    Ok(Some(_)) => Ok(()),
                    Ok(None) => Err(anyhow!("Challenge has already been used")),
                    Err(e) => Err(anyhow!("Failed to record challenge nonce: {e}")),
                }
            }
            #[cfg(test)]
            Self::Memory(map) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64);
                let mut map = map.lock().expect("nonce ledger lock poisoned");
                if map.get(key).is_some_and(|deadline| *deadline > now) {
                    return Err(anyhow!("Challenge has already been used"));
                }
                map.insert(key.to_string(), now + ttl_seconds);
                Ok(())
            }
        }
    }
}

/// Horizon `GET /accounts/{id}` fields we need for signer/threshold checks.
#[derive(Debug, Deserialize)]
struct HorizonAccount {
    #[serde(default)]
    signers: Vec<HorizonSigner>,
    #[serde(default)]
    thresholds: HorizonThresholds,
}

#[derive(Debug, Deserialize)]
struct HorizonSigner {
    key: String,
    #[serde(default)]
    weight: u32,
}

#[derive(Debug, Deserialize, Default)]
struct HorizonThresholds {
    #[serde(default)]
    medium_threshold: u32,
}

/// SEP-10 Authentication Service (canonical implementation used by API)
///
/// Use `crate::auth::sep10_simple::Sep10Service` in handler wiring.
pub struct Sep10Service {
    pub server_public_key: String,
    pub network_passphrase: String,
    pub home_domain: String,
    /// Full `WEB_AUTH_ENDPOINT` URL published in `stellar.toml`. Its host is
    /// the `web_auth_domain` operation's value.
    pub web_auth_endpoint: String,
    /// Ed25519 secret (from `SEP10_SIGNING_KEY`) matching
    /// `server_public_key`; used to sign challenge transactions.
    signing_key: SigningKey,
    server_key_bytes: [u8; 32],
    /// Host of `web_auth_endpoint`.
    web_auth_domain: String,
    horizon_url: String,
    http: reqwest::Client,
    nonces: NonceLedger,
    redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
}

impl Sep10Service {
    /// Create new SEP-10 service.
    ///
    /// Fails when the public key is malformed/placeholder, the signing seed
    /// does not match the public key, or the home domain / web-auth endpoint
    /// cannot produce valid `ManageData` fields — misconfiguration must not
    /// yield a service that issues unverifiable challenges.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        server_public_key: String,
        network_passphrase: String,
        home_domain: String,
        web_auth_endpoint: String,
        signing_seed: String,
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

        let server_key_bytes = parse_ed25519_public_key(&server_public_key)?;
        let signing_key = signing_key_from_seed(&signing_seed)?;
        if signing_key.verifying_key().to_bytes() != server_key_bytes {
            return Err(anyhow!(
                "SEP10_SIGNING_KEY does not match SEP10_SERVER_PUBLIC_KEY. \
                 Set the secret of the published SIGNING_KEY."
            ));
        }

        if home_domain.is_empty() {
            return Err(anyhow!("SEP10_HOME_DOMAIN must not be empty"));
        }
        // The first operation's data name is "{home domain} auth" and the XDR
        // string cap is 64 bytes.
        if format!("{home_domain} auth").len() > 64 {
            return Err(anyhow!(
                "SEP10_HOME_DOMAIN is too long: \"{{home}} auth\" exceeds 64 characters"
            ));
        }

        let endpoint = url::Url::parse(&web_auth_endpoint)
            .map_err(|e| anyhow!("Invalid SEP10_WEB_AUTH_ENDPOINT: {e}"))?;
        let web_auth_domain = endpoint
            .host_str()
            .ok_or_else(|| anyhow!("SEP10_WEB_AUTH_ENDPOINT has no host"))?
            .to_string();
        if web_auth_domain.is_empty() || web_auth_domain.len() > 64 {
            return Err(anyhow!(
                "SEP10_WEB_AUTH_ENDPOINT host must be 1-64 characters (SEP-10 ManageData limit)"
            ));
        }

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(HORIZON_TIMEOUT_SECONDS))
            .build()
            .context("Failed to build SEP-10 HTTP client")?;

        Ok(Self {
            horizon_url: horizon_url_from_env(),
            server_public_key,
            network_passphrase,
            home_domain,
            web_auth_endpoint,
            signing_key,
            server_key_bytes,
            web_auth_domain,
            http,
            nonces: NonceLedger::Redis(redis_connection.clone()),
            redis_connection,
        })
    }

    /// Point the service at a specific Horizon base URL (tests, overrides).
    #[must_use]
    pub fn with_horizon_url(mut self, horizon_url: String) -> Self {
        self.horizon_url = horizon_url;
        self
    }

    /// Swap the Redis nonce ledger for an in-memory one (tests only).
    #[cfg(test)]
    #[must_use]
    pub fn with_in_memory_nonces(mut self) -> Self {
        self.nonces = NonceLedger::Memory(Arc::new(std::sync::Mutex::new(
            std::collections::HashMap::new(),
        )));
        self
    }

    /// Generate SEP-10 challenge transaction.
    ///
    /// Returns a base64 `TransactionEnvelope` signed by the server key per
    /// SEP-10: source = server account, sequence 0, time bounds now..now+expiry,
    /// first operation `ManageData "{home domain} auth"` (source = client,
    /// value = base64 of 48 random bytes), then `ManageData web_auth_domain`
    /// (source = server, value = the web-auth endpoint's host).
    pub async fn generate_challenge(&self, request: ChallengeRequest) -> Result<ChallengeResponse> {
        // Validate home domain if provided
        if let Some(ref domain) = request.home_domain {
            if domain != &self.home_domain {
                return Err(anyhow!("Invalid home domain"));
            }
        }

        // Deny-by-default: this server does not fetch and verify client-domain
        // SIGNING_KEYs, so it must not issue challenges advertising one.
        if request.client_domain.is_some() {
            return Err(anyhow!("client_domain is not supported by this server"));
        }

        // Strict strkey validation (G..., 56 chars, valid checksum).
        let client = parse_ed25519_public_key(&request.account)?;
        let memo = parse_id_memo(request.memo.as_deref())?;

        let tx = self.build_challenge_tx(&client, memo, Utc::now().timestamp())?;
        let envelope = TransactionEnvelope::Tx(self.server_sign(tx)?);
        let transaction = envelope
            .to_xdr_base64(Limits::none())
            .context("Failed to encode challenge transaction")?;

        Ok(ChallengeResponse {
            transaction,
            network_passphrase: self.network_passphrase.clone(),
        })
    }

    /// Verify a signed challenge and open a session for its account.
    ///
    /// The submitted envelope must carry exactly one signature from this
    /// server's key and client signatures that meet the account's medium
    /// threshold (or the master key, for unfunded accounts), and its nonce
    /// must never have been redeemed before.
    pub async fn verify_challenge(
        &self,
        request: VerificationRequest,
    ) -> Result<VerificationResponse> {
        let verified = self.verify_signed_challenge(&request.transaction).await?;

        // Single use: atomically claim the nonce before opening a session.
        let key = format!("sep10:nonce:{}:{}", verified.client_account, verified.nonce);
        self.nonces
            .consume(&key, challenge_expiry_seconds())
            .await?;

        let token = self
            .generate_session_token(&verified.client_account)
            .await?;
        let session = Sep10Session {
            account: verified.client_account,
            client_domain: None,
            memo: verified.memo.map(|m| m.to_string()),
            created_at: Utc::now().timestamp(),
            expires_at: Utc::now().timestamp() + (session_expiry_days() * 24 * 60 * 60),
        };
        self.store_session(&token, &session).await?;

        Ok(VerificationResponse {
            token,
            expires_in: session_expiry_days() * 24 * 60 * 60,
        })
    }

    /// Run every SEP-10 check on a submitted (signed) challenge except nonce
    /// consumption: decode, structure, time bounds, `web_auth_domain`, server
    /// signature, then client signatures against the account's signers.
    pub async fn verify_signed_challenge(&self, transaction: &str) -> Result<VerifiedChallenge> {
        let envelope = decode_challenge_envelope(transaction)?;
        let tx = &envelope.tx;

        // The transaction must be issued by this server, with the invalid
        // sequence number SEP-10 mandates so it can never be submitted.
        let source: [u8; 32] = match &tx.source_account {
            MuxedAccount::Ed25519(u) => *u.as_ref(),
            MuxedAccount::MuxedEd25519(_) => {
                return Err(anyhow!(
                    "Challenge transaction source must be a G... account, not a muxed account"
                ));
            }
        };
        if source != self.server_key_bytes {
            return Err(anyhow!(
                "Challenge transaction was not issued by this server's signing key"
            ));
        }
        if tx.seq_num.0 != 0 {
            return Err(anyhow!("Challenge transaction sequence number must be 0"));
        }

        // Time bounds: present, and "now" inside them (small grace before
        // min_time for clock skew; max_time is strict so expired challenges
        // are rejected).
        let now = Utc::now().timestamp();
        match &tx.cond {
            Preconditions::Time(bounds) => {
                let min_time = i64::try_from(bounds.min_time.0).unwrap_or(i64::MAX);
                let max_time = i64::try_from(bounds.max_time.0).unwrap_or(i64::MAX);
                if now > max_time {
                    return Err(anyhow!("Challenge transaction has expired"));
                }
                if now + CLOCK_SKEW_SECONDS < min_time {
                    return Err(anyhow!("Challenge transaction is not valid yet"));
                }
            }
            _ => return Err(anyhow!("Challenge transaction is missing time bounds")),
        }

        // Memo: only None or ID, per SEP-10.
        let memo = match tx.memo {
            Memo::None => None,
            Memo::Id(id) => Some(id),
            _ => return Err(anyhow!("SEP-10 challenges may only carry an ID memo")),
        };

        // Operations: first must be the client-sourced nonce op.
        let operations: &[Operation] = &tx.operations;
        let first = operations
            .first()
            .ok_or_else(|| anyhow!("Challenge transaction has no operations"))?;
        let client_key = operation_source(&first.source_account)?;
        let client_account = ed25519_address(&client_key);
        let nonce_op = match &first.body {
            OperationBody::ManageData(op) => op,
            _ => return Err(anyhow!("First operation must be ManageData")),
        };
        let expected_name = format!("{} auth", self.home_domain);
        if manage_data_name(nonce_op) != expected_name {
            return Err(anyhow!(
                "First operation must be ManageData \"{}\"",
                expected_name
            ));
        }
        let nonce_value = nonce_op
            .data_value
            .as_ref()
            .ok_or_else(|| anyhow!("Auth nonce operation is missing its value"))?;
        let nonce_value: &[u8] = nonce_value.0.as_ref();
        if nonce_value.len() != 64 {
            return Err(anyhow!(
                "Auth nonce must be 64 bytes (base64 of 48 random bytes)"
            ));
        }
        let nonce_raw = BASE64
            .decode(nonce_value)
            .map_err(|e| anyhow!("Auth nonce is not valid base64: {e}"))?;
        if nonce_raw.len() != NONCE_BYTES {
            return Err(anyhow!(
                "Auth nonce must decode to {NONCE_BYTES} random bytes"
            ));
        }

        // Remaining operations: the server-sourced web_auth_domain op (which
        // must match this server's endpoint host). Nothing else is issued, so
        // nothing else is accepted.
        let mut saw_web_auth_domain = false;
        for op in &operations[1..] {
            // SEP-10: the web_auth_domain operation may omit its source, in
            // which case it defaults to the transaction source — this
            // server's account. Reference SDKs (e.g. @stellar/stellar-sdk)
            // omit it when the source equals the tx source, so accept both.
            let source = match &op.source_account {
                Some(_) => operation_source(&op.source_account)?,
                None => self.server_key_bytes,
            };
            let manage_data = match &op.body {
                OperationBody::ManageData(op) => op,
                _ => return Err(anyhow!("Only ManageData operations are allowed")),
            };
            if manage_data_name(manage_data) != "web_auth_domain" {
                return Err(anyhow!(
                    "Unexpected operation \"{}\" in challenge",
                    manage_data_name(manage_data)
                ));
            }
            if source != self.server_key_bytes {
                return Err(anyhow!(
                    "web_auth_domain operation must be sourced by the server account"
                ));
            }
            let value = manage_data
                .data_value
                .as_ref()
                .ok_or_else(|| anyhow!("web_auth_domain operation is missing its value"))?;
            let value: &[u8] = value.0.as_ref();
            if value != self.web_auth_domain.as_bytes() {
                return Err(anyhow!(
                    "web_auth_domain does not match this server's WEB_AUTH_ENDPOINT"
                ));
            }
            saw_web_auth_domain = true;
        }
        if !saw_web_auth_domain {
            return Err(anyhow!(
                "Challenge transaction is missing the web_auth_domain operation"
            ));
        }

        // Server signature: exactly one, valid for the signing key, over this
        // exact transaction. Everything else is a client signature.
        let hash = transaction_hash(tx, &self.network_passphrase)?;
        let mut server_signatures = 0usize;
        let mut client_signatures: Vec<&DecoratedSignature> = Vec::new();
        for index in 0..envelope.signatures.len() {
            let signature = &envelope.signatures[index];
            if signature_verifies(signature, &hash, &self.server_key_bytes) {
                server_signatures += 1;
            } else {
                client_signatures.push(signature);
            }
        }
        if server_signatures != 1 {
            return Err(anyhow!(
                "Challenge transaction must carry exactly one signature from the server signing key"
            ));
        }
        if client_signatures.is_empty() {
            return Err(anyhow!(
                "Challenge transaction is missing the client signature"
            ));
        }

        // Client signatures against the account's signers and thresholds.
        match self.fetch_account(&client_account).await? {
            // Account does not exist on the network (unfunded): SEP-10 allows
            // exactly one signature, by the account's master key.
            None => {
                if client_signatures.len() != 1 {
                    return Err(anyhow!(
                        "Unfunded accounts must submit exactly one client signature"
                    ));
                }
                if !signature_verifies(client_signatures[0], &hash, &client_key) {
                    return Err(anyhow!(
                        "Client signature is not valid for the account's master key"
                    ));
                }
            }
            // Funded account: every client signature must come from a listed
            // signer (never the server key), duplicates are rejected, and the
            // summed weight must reach the medium threshold — the bar the
            // Stellar anchor tests use for account authentication.
            Some(account) => {
                let mut eligible: Vec<([u8; 32], u32)> = Vec::new();
                for signer in &account.signers {
                    let Ok(key) = parse_ed25519_public_key(&signer.key) else {
                        continue; // non-ed25519 signer types cannot sign this scheme
                    };
                    if key == self.server_key_bytes {
                        // SEP-10: the server's own key must not count toward
                        // the client's weight even if it is also a signer.
                        continue;
                    }
                    eligible.push((key, signer.weight));
                }

                let mut seen_signatures: std::collections::HashSet<Vec<u8>> =
                    std::collections::HashSet::new();
                let mut weight: u32 = 0;
                for signature in &client_signatures {
                    let raw: Vec<u8> = signature.signature.0.to_vec();
                    if !seen_signatures.insert(raw) {
                        return Err(anyhow!("Duplicate signatures are not allowed"));
                    }
                    let mut counted = false;
                    for (key, signer_weight) in &eligible {
                        if signature_verifies(signature, &hash, key) {
                            weight = weight.saturating_add(*signer_weight);
                            counted = true;
                            break;
                        }
                    }
                    if !counted {
                        return Err(anyhow!(
                            "Challenge carries a signature from a key that is not a signer on the account"
                        ));
                    }
                }

                // Weight alone is not enough: an account with a medium
                // threshold of 0 still has to produce one real signature.
                let required = account.thresholds.medium_threshold.max(1);
                if weight < required {
                    return Err(anyhow!(
                        "Signatures reach weight {weight}, below the account's medium threshold {required}"
                    ));
                }
            }
        }

        Ok(VerifiedChallenge {
            client_account,
            nonce: hex::encode(nonce_raw),
            memo,
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

    /// Build the unsigned challenge transaction for `client` as of `now`.
    fn build_challenge_tx(&self, client: &[u8; 32], memo: Memo, now: i64) -> Result<Transaction> {
        use rand::RngExt;
        let mut rng = rand::rng();
        let nonce: [u8; NONCE_BYTES] = rng.random();
        // 48 random bytes, base64-encoded to the 64-byte value SEP-10 mandates.
        let nonce_value = BASE64.encode(nonce);

        let operations = vec![
            manage_data_op(
                MuxedAccount::Ed25519((*client).into()),
                format!("{} auth", self.home_domain),
                nonce_value.into_bytes(),
            )?,
            manage_data_op(
                MuxedAccount::Ed25519(self.server_key_bytes.into()),
                "web_auth_domain".to_string(),
                self.web_auth_domain.clone().into_bytes(),
            )?,
        ];
        let operations = operations
            .try_into()
            .map_err(|e| anyhow!("Failed to build challenge operations: {e}"))?;

        let now = u64::try_from(now).unwrap_or(0);
        let max_time = now.saturating_add(u64::try_from(challenge_expiry_seconds()).unwrap_or(900));

        Ok(Transaction {
            source_account: MuxedAccount::Ed25519(self.server_key_bytes.into()),
            fee: CHALLENGE_FEE,
            seq_num: SequenceNumber(0),
            cond: Preconditions::Time(TimeBounds {
                min_time: TimePoint(now),
                max_time: TimePoint(max_time),
            }),
            memo,
            operations,
            ext: TransactionExt::V0,
        })
    }

    /// Sign a challenge transaction with the server key (exactly one
    /// signature) and wrap it in an envelope.
    fn server_sign(&self, tx: Transaction) -> Result<TransactionV1Envelope> {
        let hash = transaction_hash(&tx, &self.network_passphrase)?;
        let signature = sign_hash(&self.signing_key, &hash)?;
        let signatures = vec![signature]
            .try_into()
            .map_err(|e| anyhow!("Failed to attach server signature: {e}"))?;
        Ok(TransactionV1Envelope { tx, signatures })
    }

    async fn generate_session_token(&self, account: &str) -> Result<String> {
        use rand::RngExt;
        let mut rng = rand::rng();
        let random_bytes: [u8; 32] = rng.random();
        let token = format!("{}:{}", account, BASE64.encode(random_bytes));
        Ok(BASE64.encode(token.as_bytes()))
    }

    /// Look the account up on Horizon. `Ok(None)` means the account does not
    /// exist (unfunded); any other failure aborts verification (fail closed).
    async fn fetch_account(&self, account: &str) -> Result<Option<HorizonAccount>> {
        let url = format!(
            "{}/accounts/{account}",
            self.horizon_url.trim_end_matches('/')
        );
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to look up the account on Horizon: {e}"))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(anyhow!(
                "Horizon account lookup failed with status {}",
                response.status()
            ));
        }
        let account = response
            .json::<HorizonAccount>()
            .await
            .map_err(|e| anyhow!("Invalid Horizon account response: {e}"))?;
        Ok(Some(account))
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

/// Decode the base64 challenge envelope a client submits (or that
/// `generate_challenge` issued). Rejects anything that is not a `Tx`
/// envelope — including the legacy JSON blob (issue #2545).
pub fn decode_challenge_envelope(transaction: &str) -> Result<TransactionV1Envelope> {
    let bytes = BASE64
        .decode(transaction)
        .map_err(|e| anyhow!("Challenge is not valid base64: {e}"))?;
    let envelope = TransactionEnvelope::from_xdr(&bytes, Limits::none())
        .map_err(|e| anyhow!("Challenge is not a Stellar transaction envelope: {e}"))?;
    match envelope {
        TransactionEnvelope::Tx(v1) => Ok(v1),
        TransactionEnvelope::TxV0(_) => Err(anyhow!("V0 transaction envelopes are not accepted")),
        TransactionEnvelope::TxFeeBump(_) => Err(anyhow!("Fee-bump envelopes are not accepted")),
    }
}

/// `max_time` of a challenge's time bounds, for early expiry checks in the
/// API layer. `None` when the input is not a readable challenge.
pub fn challenge_max_time(transaction: &str) -> Option<i64> {
    let envelope = decode_challenge_envelope(transaction).ok()?;
    match envelope.tx.cond {
        Preconditions::Time(bounds) => i64::try_from(bounds.max_time.0).ok(),
        _ => None,
    }
}

/// The `G...` account a challenge was issued for (source of the first
/// operation), for per-account rate limiting. `None` when unreadable.
pub fn challenge_client_account(transaction: &str) -> Option<String> {
    let envelope = decode_challenge_envelope(transaction).ok()?;
    let first = envelope.tx.operations.first()?;
    let source = operation_source(&first.source_account).ok()?;
    Some(ed25519_address(&source))
}

/// Horizon URL for the configured network.
fn horizon_url_from_env() -> String {
    let network = std::env::var("STELLAR_NETWORK").unwrap_or_else(|_| "testnet".to_string());
    let var_name = if network.eq_ignore_ascii_case("mainnet") {
        "STELLAR_HORIZON_URL_MAINNET"
    } else {
        "STELLAR_HORIZON_URL_TESTNET"
    };
    std::env::var(var_name).unwrap_or_else(|_| "https://horizon.stellar.org".to_string())
}

/// Decode a `G...` strkey into raw ed25519 bytes (validates the checksum).
fn parse_ed25519_public_key(value: &str) -> Result<[u8; 32]> {
    stellar_strkey::ed25519::PublicKey::from_string(value)
        .map(|key| key.0)
        .map_err(|e| anyhow!("Invalid account address format: {e}"))
}

/// Encode raw ed25519 bytes as a `G...` strkey.
fn ed25519_address(bytes: &[u8; 32]) -> String {
    // strkey's inherent `to_string` returns a `HeaplessString`; the second
    // call renders it as a std `String`.
    stellar_strkey::ed25519::PublicKey(*bytes)
        .to_string()
        .to_string()
}

/// Decode an `S...` strkey secret seed into a signing key.
fn signing_key_from_seed(seed: &str) -> Result<SigningKey> {
    let raw = stellar_strkey::ed25519::PrivateKey::from_string(seed)
        .map_err(|e| anyhow!("Invalid SEP10_SIGNING_KEY: {e}"))?;
    Ok(SigningKey::from_bytes(&raw.0))
}

/// SEP-10 allows only ID memos (a uint64), supplied as a string.
fn parse_id_memo(memo: Option<&str>) -> Result<Memo> {
    match memo {
        None => Ok(Memo::None),
        Some(value) => {
            let id = value
                .parse::<u64>()
                .map_err(|_| anyhow!("SEP-10 supports only ID memos (a uint64)"))?;
            Ok(Memo::Id(id))
        }
    }
}

/// Extract an operation's source account as raw ed25519 bytes.
fn operation_source(source: &Option<MuxedAccount>) -> Result<[u8; 32]> {
    match source {
        Some(MuxedAccount::Ed25519(u)) => Ok(*u.as_ref()),
        Some(MuxedAccount::MuxedEd25519(_)) => {
            Err(anyhow!("Muxed (M...) operation sources are not supported"))
        }
        None => Err(anyhow!(
            "Operation must carry an explicit source account (the client)"
        )),
    }
}

/// The data name of a `ManageData` operation as UTF-8 lossy text.
fn manage_data_name(op: &ManageDataOp) -> String {
    let name: &[u8] = op.data_name.as_ref();
    String::from_utf8_lossy(name).into_owned()
}

/// Build one `ManageData` operation with an explicit source account.
fn manage_data_op(source: MuxedAccount, name: String, value: Vec<u8>) -> Result<Operation> {
    let data_name = String64::try_from(name.into_bytes())
        .map_err(|e| anyhow!("ManageData name is invalid or too long: {e}"))?;
    let data_value =
        DataValue::try_from(value).map_err(|e| anyhow!("ManageData value is invalid: {e}"))?;
    Ok(Operation {
        source_account: Some(source),
        body: OperationBody::ManageData(ManageDataOp {
            data_name,
            data_value: Some(data_value),
        }),
    })
}

/// `NETWORK_ID = SHA256(network_passphrase)`.
fn network_id(passphrase: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    hasher.finalize().into()
}

/// The Stellar transaction hash: `SHA256(XDR(TransactionSignaturePayload))`.
fn transaction_hash(tx: &Transaction, network_passphrase: &str) -> Result<[u8; 32]> {
    let payload = TransactionSignaturePayload {
        network_id: Hash(network_id(network_passphrase)),
        tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(tx.clone()),
    };
    let payload_bytes = payload
        .to_xdr(Limits::none())
        .context("Failed to encode transaction signature payload")?;
    let mut hasher = Sha256::new();
    hasher.update(payload_bytes);
    Ok(hasher.finalize().into())
}

/// Sign a transaction hash with `key`, producing a `DecoratedSignature`
/// whose hint is the tail of the public key.
fn sign_hash(key: &SigningKey, hash: &[u8; 32]) -> Result<DecoratedSignature> {
    let signature: [u8; 64] = key.sign(hash).to_bytes();
    let public = key.verifying_key().to_bytes();
    let mut hint = [0u8; 4];
    hint.copy_from_slice(&public[28..32]);
    Ok(DecoratedSignature {
        hint: stellar_xdr::curr::SignatureHint(hint),
        signature: Signature(
            signature
                .to_vec()
                .try_into()
                .map_err(|e| anyhow!("Unexpected signature length: {e}"))?,
        ),
    })
}

/// Does `signature` verify over `hash` for `public_key`?
fn signature_verifies(
    signature: &DecoratedSignature,
    hash: &[u8; 32],
    public_key: &[u8; 32],
) -> bool {
    let Ok(verifying_key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let raw: &[u8] = signature.signature.0.as_ref();
    let Ok(raw) = <[u8; 64]>::try_from(raw) else {
        return false;
    };
    verifying_key
        .verify(hash, &DalekSignature::from_bytes(&raw))
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const SERVER_SEED: u8 = 11;
    const CLIENT_SEED: u8 = 7;
    const FRIEND_SEED: u8 = 21;
    const STRANGER_SEED: u8 = 33;
    const PASSPHRASE: &str = "Test SDF Network ; September 2015";

    fn keypair(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn account(key: &SigningKey) -> String {
        ed25519_address(&key.verifying_key().to_bytes())
    }

    fn seed_str(key: &SigningKey) -> String {
        let private_key = stellar_strkey::ed25519::PrivateKey(key.to_bytes());
        // Secret keys only render through `Unredacted`.
        format!("{}", stellar_strkey::Unredacted(&private_key))
    }

    /// A service configured like production would be, with the in-memory
    /// nonce ledger (no Redis in unit tests).
    fn service(server: &SigningKey) -> Sep10Service {
        Sep10Service::new(
            account(server),
            PASSPHRASE.to_string(),
            "example.com".to_string(),
            "https://example.com/api/sep10/auth".to_string(),
            seed_str(server),
            Arc::new(RwLock::new(None)),
        )
        .expect("valid SEP-10 service")
        .with_in_memory_nonces()
    }

    fn challenge_request(account: &str) -> ChallengeRequest {
        ChallengeRequest {
            account: account.to_string(),
            home_domain: Some("example.com".to_string()),
            client_domain: None,
            memo: None,
        }
    }

    /// Attach signatures from `keys` to an envelope and return the base64
    /// XDR — the wallet side of the flow.
    fn signed(envelope: TransactionV1Envelope, keys: &[&SigningKey]) -> String {
        let hash = transaction_hash(&envelope.tx, PASSPHRASE).expect("tx hash");
        let mut signatures = envelope.signatures.to_vec();
        for key in keys {
            signatures.push(sign_hash(key, &hash).expect("signature"));
        }
        let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
            tx: envelope.tx,
            signatures: signatures.try_into().expect("signature limit"),
        });
        envelope.to_xdr_base64(Limits::none()).expect("base64 xdr")
    }

    fn client_signs(challenge: &str, key: &SigningKey) -> String {
        let envelope = decode_challenge_envelope(challenge).expect("valid challenge envelope");
        signed(envelope, &[key])
    }

    async fn challenge(service: &Sep10Service, account: &str) -> String {
        service
            .generate_challenge(challenge_request(account))
            .await
            .expect("challenge")
            .transaction
    }

    /// Horizon responding 404 (account does not exist / unfunded).
    async fn unfunded_horizon(account: &str) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/accounts/{account}")))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        server
    }

    /// Horizon account JSON with the given signers and medium threshold.
    fn account_json(
        account: &str,
        signers: &[(&str, u32)],
        medium_threshold: u32,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": account,
            "account_id": account,
            "thresholds": {
                "low_threshold": 0,
                "medium_threshold": medium_threshold,
                "high_threshold": 0,
            },
            "signers": signers
                .iter()
                .map(|(key, weight)| serde_json::json!({ "key": key, "weight": weight, "type": "ed25519" }))
                .collect::<Vec<_>>(),
        })
    }

    // ------------------------------------------------------------------
    // Challenge generation (regression: #2545 — challenge must be a signed
    // transaction envelope, not a JSON blob)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn challenge_is_a_server_signed_transaction_envelope() {
        let service = service(&keypair(SERVER_SEED));
        let client = keypair(CLIENT_SEED);
        let response = service
            .generate_challenge(challenge_request(&account(&client)))
            .await
            .expect("challenge");

        assert_eq!(response.network_passphrase, PASSPHRASE);

        // The heart of #2545: this must decode as XDR, not JSON.
        let envelope =
            decode_challenge_envelope(&response.transaction).expect("challenge must be XDR");
        let tx = &envelope.tx;

        // Server source, sequence 0, 15-minute time bounds.
        assert_eq!(
            tx.source_account,
            MuxedAccount::Ed25519(keypair(SERVER_SEED).verifying_key().to_bytes().into())
        );
        assert_eq!(tx.seq_num.0, 0);
        let Preconditions::Time(bounds) = &tx.cond else {
            panic!("time bounds required");
        };
        let now = Utc::now().timestamp() as u64;
        assert!(
            bounds.min_time.0 <= now,
            "min_time must not be in the future"
        );
        assert!(
            bounds.max_time.0 >= now + 890,
            "challenge must last ~15 minutes"
        );

        // Exactly one signature, valid for the server's signing key.
        assert_eq!(envelope.signatures.len(), 1);
        let hash = transaction_hash(tx, PASSPHRASE).unwrap();
        assert!(signature_verifies(
            &envelope.signatures[0],
            &hash,
            &keypair(SERVER_SEED).verifying_key().to_bytes()
        ));
    }

    #[tokio::test]
    async fn challenge_first_operation_is_home_domain_auth_with_48_byte_nonce() {
        let service = service(&keypair(SERVER_SEED));
        let client = keypair(CLIENT_SEED);
        let response = service
            .generate_challenge(challenge_request(&account(&client)))
            .await
            .expect("challenge");
        let envelope = decode_challenge_envelope(&response.transaction).unwrap();

        let first = &envelope.tx.operations[0];
        let OperationBody::ManageData(op) = &first.body else {
            panic!("first operation must be ManageData");
        };
        assert_eq!(manage_data_name(op), "example.com auth");
        assert_eq!(
            operation_source(&first.source_account).unwrap(),
            client.verifying_key().to_bytes()
        );

        let value: &[u8] = op.data_value.as_ref().expect("nonce").0.as_ref();
        assert_eq!(value.len(), 64, "value is base64 of 48 bytes");
        let decoded = BASE64.decode(value).unwrap();
        assert_eq!(decoded.len(), 48);
    }

    #[tokio::test]
    async fn challenge_carries_web_auth_domain_operation() {
        let service = service(&keypair(SERVER_SEED));
        let client = keypair(CLIENT_SEED);
        let response = service
            .generate_challenge(challenge_request(&account(&client)))
            .await
            .expect("challenge");
        let envelope = decode_challenge_envelope(&response.transaction).unwrap();

        assert_eq!(envelope.tx.operations.len(), 2);
        let op = &envelope.tx.operations[1];
        let OperationBody::ManageData(manage_data) = &op.body else {
            panic!("second operation must be ManageData");
        };
        assert_eq!(manage_data_name(manage_data), "web_auth_domain");
        let value: &[u8] = manage_data.data_value.as_ref().expect("value").0.as_ref();
        assert_eq!(value, b"example.com");
        // Sourced by the server account (it is also the tx source).
        assert!(op.source_account.is_some());
        assert_eq!(
            operation_source(&op.source_account).unwrap(),
            keypair(SERVER_SEED).verifying_key().to_bytes()
        );
    }

    #[tokio::test]
    async fn rejects_invalid_account_format() {
        let service = service(&keypair(SERVER_SEED));
        let result = service
            .generate_challenge(challenge_request("INVALID"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_a_valid_strkey_with_the_wrong_home_domain() {
        let service = service(&keypair(SERVER_SEED));
        let mut request = challenge_request(&account(&keypair(CLIENT_SEED)));
        request.home_domain = Some("wrong.com".to_string());
        let err = service.generate_challenge(request).await.unwrap_err();
        assert!(err.to_string().contains("home domain"));
    }

    #[tokio::test]
    async fn rejects_client_domain_requests() {
        let service = service(&keypair(SERVER_SEED));
        let mut request = challenge_request(&account(&keypair(CLIENT_SEED)));
        request.client_domain = Some("client.example".to_string());
        let err = service.generate_challenge(request).await.unwrap_err();
        assert!(err.to_string().contains("client_domain"));
    }

    #[tokio::test]
    async fn rejects_non_id_memos() {
        let service = service(&keypair(SERVER_SEED));
        let mut request = challenge_request(&account(&keypair(CLIENT_SEED)));
        request.memo = Some("invoice-42".to_string());
        let err = service.generate_challenge(request).await.unwrap_err();
        assert!(err.to_string().contains("ID memo"));
    }

    #[tokio::test]
    async fn carries_a_requested_id_memo() {
        let service = service(&keypair(SERVER_SEED));
        let mut request = challenge_request(&account(&keypair(CLIENT_SEED)));
        request.memo = Some("17509749319012223907".to_string());
        let response = service
            .generate_challenge(request)
            .await
            .expect("challenge");
        let envelope = decode_challenge_envelope(&response.transaction).unwrap();
        assert!(matches!(envelope.tx.memo, Memo::Id(17509749319012223907)));
    }

    // ------------------------------------------------------------------
    // Verification
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn verify_accepts_client_signed_challenge_for_unfunded_account() {
        let server_key = keypair(SERVER_SEED);
        let client_key = keypair(CLIENT_SEED);
        let horizon = unfunded_horizon(&account(&client_key)).await;
        let service = service(&server_key).with_horizon_url(horizon.uri());

        let issued = challenge(&service, &account(&client_key)).await;
        let signed = client_signs(&issued, &client_key);

        let response = service
            .verify_challenge(VerificationRequest {
                transaction: signed,
            })
            .await
            .expect("verification should succeed");
        assert!(!response.token.is_empty());
        assert!(response.expires_in > 0);
    }

    #[tokio::test]
    async fn rejects_the_legacy_json_challenge_blob() {
        // Regression for #2545: the old format was base64 JSON; it must no
        // longer be accepted anywhere in the pipeline.
        let service = service(&keypair(SERVER_SEED));
        let blob = serde_json::json!({
            "type": "sep10_challenge",
            "server": account(&keypair(SERVER_SEED)),
            "client": account(&keypair(CLIENT_SEED)),
            "nonce": "bm9uY2U=",
            "expires_at": Utc::now().timestamp() + 100_000,
        });
        let legacy = BASE64.encode(serde_json::to_string(&blob).unwrap().as_bytes());

        let err = service.verify_signed_challenge(&legacy).await.unwrap_err();
        assert!(
            err.to_string()
                .contains("not a Stellar transaction envelope"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_a_replayed_challenge() {
        let server_key = keypair(SERVER_SEED);
        let client_key = keypair(CLIENT_SEED);
        let horizon = unfunded_horizon(&account(&client_key)).await;
        let service = service(&server_key).with_horizon_url(horizon.uri());

        let issued = challenge(&service, &account(&client_key)).await;
        let signed = client_signs(&issued, &client_key);

        service
            .verify_challenge(VerificationRequest {
                transaction: signed.clone(),
            })
            .await
            .expect("first use succeeds");

        let err = service
            .verify_challenge(VerificationRequest {
                transaction: signed,
            })
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("already been used"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_an_expired_challenge() {
        let server_key = keypair(SERVER_SEED);
        let client_key = keypair(CLIENT_SEED);
        let service = service(&server_key);

        // A challenge whose window closed 10 minutes ago (still signed by
        // the server, so only the time bounds can reject it).
        let stale = service
            .build_challenge_tx(
                &client_key.verifying_key().to_bytes(),
                Memo::None,
                Utc::now().timestamp() - 1_500,
            )
            .expect("tx");
        let envelope = service.server_sign(stale).expect("signed");
        let issued = TransactionEnvelope::Tx(envelope)
            .to_xdr_base64(Limits::none())
            .unwrap();
        let signed = signed(decode_challenge_envelope(&issued).unwrap(), &[&client_key]);

        let err = service.verify_signed_challenge(&signed).await.unwrap_err();
        assert!(
            err.to_string().contains("expired"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_a_challenge_without_the_server_signature() {
        let server_key = keypair(SERVER_SEED);
        let client_key = keypair(CLIENT_SEED);
        let service = service(&server_key);

        let issued = challenge(&service, &account(&client_key)).await;
        let envelope = decode_challenge_envelope(&issued).unwrap();
        // Drop the server signature, keep only the client's.
        let unsigned = TransactionV1Envelope {
            tx: envelope.tx,
            signatures: Vec::new().try_into().unwrap(),
        };
        let signed = signed(unsigned, &[&client_key]);

        let err = service.verify_signed_challenge(&signed).await.unwrap_err();
        assert!(
            err.to_string().contains("exactly one signature"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_a_challenge_signed_by_the_wrong_server_key() {
        let client_key = keypair(CLIENT_SEED);
        let service = service(&keypair(SERVER_SEED));

        // Structurally perfect, but signed by someone who is not this
        // server: the server-signature check must fail.
        let impostor = keypair(STRANGER_SEED);
        let tx = service
            .build_challenge_tx(
                &client_key.verifying_key().to_bytes(),
                Memo::None,
                Utc::now().timestamp(),
            )
            .expect("tx");
        let hash = transaction_hash(&tx, PASSPHRASE).unwrap();
        let signature = sign_hash(&impostor, &hash).unwrap();
        let envelope = TransactionV1Envelope {
            tx,
            signatures: vec![signature].try_into().unwrap(),
        };
        let signed = signed(envelope, &[&client_key]);

        let err = service.verify_signed_challenge(&signed).await.unwrap_err();
        assert!(
            err.to_string().contains("exactly one signature"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_a_challenge_with_the_wrong_home_domain() {
        // Issued by the same server key but for a different home domain —
        // its first operation's name will not match this server's.
        let server_key = keypair(SERVER_SEED);
        let client_key = keypair(CLIENT_SEED);
        let other = Sep10Service::new(
            account(&server_key),
            PASSPHRASE.to_string(),
            "evil.example".to_string(),
            "https://evil.example/api/sep10/auth".to_string(),
            seed_str(&server_key),
            Arc::new(RwLock::new(None)),
        )
        .expect("service")
        .with_in_memory_nonces();

        let mut request = challenge_request(&account(&client_key));
        request.home_domain = Some("evil.example".to_string());
        let issued = other
            .generate_challenge(request)
            .await
            .expect("challenge")
            .transaction;
        let signed = client_signs(&issued, &client_key);

        let service = service(&server_key);
        let err = service.verify_signed_challenge(&signed).await.unwrap_err();
        assert!(
            err.to_string().contains("home domain") || err.to_string().contains("auth"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_a_challenge_missing_the_web_auth_domain_operation() {
        let server_key = keypair(SERVER_SEED);
        let client_key = keypair(CLIENT_SEED);
        let service = service(&server_key);

        let issued = challenge(&service, &account(&client_key)).await;
        let mut envelope = decode_challenge_envelope(&issued).unwrap();
        // Remove the web_auth_domain op and re-sign as the server: only the
        // structure check can catch this.
        let nonce_op = envelope.tx.operations[0].clone();
        envelope.tx.operations = vec![nonce_op].try_into().unwrap();
        let resigned = service.server_sign(envelope.tx).expect("signed");
        let signed = signed(resigned, &[&client_key]);

        let err = service.verify_signed_challenge(&signed).await.unwrap_err();
        assert!(
            err.to_string().contains("web_auth_domain"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_missing_client_signature() {
        let server_key = keypair(SERVER_SEED);
        let client_key = keypair(CLIENT_SEED);
        let service = service(&server_key);

        let issued = challenge(&service, &account(&client_key)).await;
        let err = service.verify_signed_challenge(&issued).await.unwrap_err();
        assert!(
            err.to_string().contains("missing the client signature"),
            "unexpected error: {err}"
        );
    }

    // Multisig: signers and medium threshold from Horizon

    /// Horizon reporting the account as funded, with the given extra
    /// signers (weights) and medium threshold.
    async fn multisig_horizon(
        master: &SigningKey,
        extra_signers: &[(&str, u32)],
        medium_threshold: u32,
    ) -> MockServer {
        let master_account = account(master);
        let mut signers: Vec<(&str, u32)> = vec![(&master_account, 1)];
        signers.extend_from_slice(extra_signers);
        let server = MockServer::start().await;
        let body = account_json(&account(master), &signers, medium_threshold);
        Mock::given(method("GET"))
            .and(path(format!("/accounts/{}", account(master))))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn accepts_signatures_that_meet_the_medium_threshold() {
        let server_key = keypair(SERVER_SEED);
        let client_key = keypair(CLIENT_SEED);
        let friend_key = keypair(FRIEND_SEED);
        // master (weight 1) + friend (weight 1) reach medium threshold 2.
        let horizon = multisig_horizon(&client_key, &[(&account(&friend_key), 1)], 2).await;
        let service = service(&server_key).with_horizon_url(horizon.uri());

        let issued = challenge(&service, &account(&client_key)).await;
        let envelope = decode_challenge_envelope(&issued).unwrap();
        let signed = signed(envelope, &[&client_key, &friend_key]);

        service
            .verify_challenge(VerificationRequest {
                transaction: signed,
            })
            .await
            .expect("threshold met");
    }

    #[tokio::test]
    async fn rejects_signatures_below_the_medium_threshold() {
        let server_key = keypair(SERVER_SEED);
        let client_key = keypair(CLIENT_SEED);
        let friend_key = keypair(FRIEND_SEED);
        // medium threshold 2, but only the master (weight 1) signs.
        let horizon = multisig_horizon(&client_key, &[(&account(&friend_key), 1)], 2).await;
        let service = service(&server_key).with_horizon_url(horizon.uri());

        let issued = challenge(&service, &account(&client_key)).await;
        let signed = client_signs(&issued, &client_key);

        let err = service
            .verify_challenge(VerificationRequest {
                transaction: signed,
            })
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("medium threshold"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_a_signature_from_a_key_that_is_not_a_signer() {
        let server_key = keypair(SERVER_SEED);
        let client_key = keypair(CLIENT_SEED);
        let stranger_key = keypair(STRANGER_SEED);
        // Account exists with only the master as signer (weight 1, medium 0).
        let horizon = multisig_horizon(&client_key, &[], 0).await;
        let service = service(&server_key).with_horizon_url(horizon.uri());

        let issued = challenge(&service, &account(&client_key)).await;
        let envelope = decode_challenge_envelope(&issued).unwrap();
        let signed = signed(envelope, &[&client_key, &stranger_key]);

        let err = service
            .verify_challenge(VerificationRequest {
                transaction: signed,
            })
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("not a signer"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_duplicate_signatures() {
        let server_key = keypair(SERVER_SEED);
        let client_key = keypair(CLIENT_SEED);
        // Funded account so the duplicate-signature check (not the unfunded
        // single-signature rule) is what rejects this.
        let horizon = multisig_horizon(&client_key, &[], 0).await;
        let service = service(&server_key).with_horizon_url(horizon.uri());

        let issued = challenge(&service, &account(&client_key)).await;
        let envelope = decode_challenge_envelope(&issued).unwrap();
        // Same signature twice (client signs, then "adds" it again).
        let signed = signed(envelope, &[&client_key, &client_key]);

        let err = service
            .verify_challenge(VerificationRequest {
                transaction: signed,
            })
            .await
            .unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("Duplicate") || message.contains("exactly one"),
            "unexpected error: {err}"
        );
    }

    // ------------------------------------------------------------------
    // Free helpers used by the API layer
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn api_helpers_read_time_bounds_and_account_from_xdr() {
        let service = service(&keypair(SERVER_SEED));
        let client_key = keypair(CLIENT_SEED);
        let issued = challenge(&service, &account(&client_key)).await;

        let max_time = challenge_max_time(&issued).expect("time bounds");
        let now = Utc::now().timestamp();
        assert!(max_time > now);
        assert!(max_time - now <= challenge_expiry_seconds());

        assert_eq!(
            challenge_client_account(&issued).as_deref(),
            Some(account(&client_key).as_str())
        );

        // Unreadable input yields None (the API falls back to IP rate keys).
        assert!(challenge_max_time("not-base64!!").is_none());
        assert!(challenge_client_account("not-base64!!").is_none());
    }
}
