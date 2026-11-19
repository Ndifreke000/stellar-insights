//! Pluggable Ed25519 signers for Soroban transaction submission.
//!
//! The submitter key controls what gets anchored on-chain, so production
//! deployments should not keep the secret seed in the web process's memory.
//! This module defines a [`TransactionSigner`] trait and three backends:
//!
//! - [`LocalSeedSigner`] — dev/test only; holds the `S...` secret seed in
//!   process memory exactly like the pre-refactor code did.
//! - [`GcpKmsSigner`] — Cloud KMS `EC_SIGN_ED25519` over the REST API; the
//!   private key never leaves Google Cloud.
//! - [`AwsKmsSigner`] — AWS KMS `ECC_NIST_EDWARDS25519` (`ED25519_SHA_512`,
//!   `MessageType::RAW`); available only with `--features aws-kms` because
//!   `aws-sdk-kms` is a large dependency.
//!
//! Signers only ever see the 32-byte transaction signature base
//! (`SHA256(NETWORK_ID ++ XDR(TransactionSignaturePayload))`). Envelope
//! validation (contract id / function / args) happens in
//! `services::envelope_policy` *before* `sign` is called, and every attempt
//! is recorded by `services::signature_audit`.
//!
//! ## Environment variables
//!
//! | Variable | Backend | Purpose |
//! |---|---|---|
//! | `TX_SIGNER_BACKEND` | all | `local` (default), `aws-kms`, or `gcp-kms` |
//! | `STELLAR_SOURCE_SECRET_KEY` | local | `S...` secret seed |
//! | `AWS_KMS_KEY_ID` | aws-kms | KMS key id, ARN, or alias |
//! | `AWS_REGION` / `AWS_DEFAULT_REGION` | aws-kms | Region (standard AWS chain) |
//! | `AWS_ENDPOINT_URL` | aws-kms | Optional endpoint override (e.g. localstack) |
//! | `GCP_KMS_KEY_VERSION_NAME` | gcp-kms | `projects/.../cryptoKeyVersions/N` |
//! | `GCP_KMS_BASE_URL` | gcp-kms | Optional API base override (testing) |
//! | `GCP_ACCESS_TOKEN` | gcp-kms | Optional static OAuth2 token (dev only) |
//! | `GOOGLE_APPLICATION_CREDENTIALS` | gcp-kms | Service-account JSON for JWT-bearer auth |
//! | `GCP_KMS_TOKEN_URL` | gcp-kms | Optional OAuth2 token endpoint override |

use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as base64_standard, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::debug;

/// A signing backend for Stellar transaction hashes.
///
/// Implementations must be `Send + Sync` and safe to share across tasks;
/// `ContractService` holds one behind an `Arc`.
#[async_trait]
pub trait TransactionSigner: Send + Sync {
    /// Stable backend identifier used in audit records, e.g. `local-seed`.
    fn backend(&self) -> &'static str;

    /// Identifier of the key within the backend (ARN, key-version resource
    /// name, or `env:STELLAR_SOURCE_SECRET_KEY`), used in audit records.
    fn key_id(&self) -> &str;

    /// Sign `message` — the 32-byte Stellar transaction signature base —
    /// and return the raw 64-byte Ed25519 signature.
    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>>;

    /// The raw 32-byte Ed25519 public key, used to build the
    /// `DecoratedSignature` hint and to verify signatures locally.
    async fn public_key(&self) -> Result<[u8; 32]>;
}

/// Verify a raw 64-byte Ed25519 signature over `message` against a raw
/// 32-byte public key. Used as a defense-in-depth check after KMS signing.
pub fn verify_ed25519_signature(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8],
) -> Result<()> {
    let verifying_key =
        VerifyingKey::from_bytes(public_key).map_err(|e| anyhow!("Invalid public key: {e}"))?;
    let sig = Signature::from_slice(signature)
        .map_err(|e| anyhow!("Invalid signature encoding: {e}"))?;
    verifying_key
        .verify(message, &sig)
        .map_err(|e| anyhow!("Signature verification failed: {e}"))
}

/// Extract the last 4 bytes of a raw Ed25519 public key for use as a
/// Stellar `SignatureHint`.
#[must_use]
pub fn signature_hint_from_public_key(public_key: &[u8; 32]) -> [u8; 4] {
    let mut hint = [0u8; 4];
    hint.copy_from_slice(&public_key[28..32]);
    hint
}

// ---------------------------------------------------------------------------
// Local seed signer (development / test only)
// ---------------------------------------------------------------------------

/// Signs with a secret seed held in process memory.
///
/// This preserves the pre-refactor behaviour for local development and
/// tests. It must not be used in production: the seed controls what gets
/// anchored on-chain and should live in a KMS/HSM instead.
pub struct LocalSeedSigner {
    signing_key: SigningKey,
    key_id: String,
}

impl LocalSeedSigner {
    /// Decode a StrKey secret seed (`S...`) into a signer.
    pub fn from_secret_seed(seed: &str) -> Result<Self> {
        let raw = stellar_strkey::ed25519::PrivateKey::from_string(seed)
            .map_err(|e| anyhow!("Invalid source secret key: {e}"))?;
        Ok(Self {
            signing_key: SigningKey::from_bytes(&raw.0),
            key_id: "env:STELLAR_SOURCE_SECRET_KEY".to_string(),
        })
    }

    /// Build from `STELLAR_SOURCE_SECRET_KEY`.
    pub fn from_env() -> Result<Self> {
        let seed = std::env::var("STELLAR_SOURCE_SECRET_KEY")
            .context("STELLAR_SOURCE_SECRET_KEY environment variable not set")?;
        Self::from_secret_seed(&seed)
    }
}

#[async_trait]
impl TransactionSigner for LocalSeedSigner {
    fn backend(&self) -> &'static str {
        "local-seed"
    }

    fn key_id(&self) -> &str {
        &self.key_id
    }

    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        Ok(self.signing_key.sign(message).to_bytes().to_vec())
    }

    async fn public_key(&self) -> Result<[u8; 32]> {
        Ok(self.signing_key.verifying_key().to_bytes())
    }
}

// ---------------------------------------------------------------------------
// GCP Cloud KMS signer (EC_SIGN_ED25519 over REST)
// ---------------------------------------------------------------------------

const GCP_DEFAULT_BASE_URL: &str = "https://cloudkms.googleapis.com";
const GCP_DEFAULT_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GCP_KMS_SCOPE: &str = "https://www.googleapis.com/auth/cloudkms";
const TOKEN_EXPIRY_MARGIN: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
enum GcpCredentials {
    /// Static bearer token (development / testing only).
    StaticToken(String),
    /// Service-account JWT-bearer flow.
    ServiceAccount {
        client_email: String,
        private_key_pem: String,
    },
}

#[derive(Debug)]
struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

/// Signs via Cloud KMS `cryptoKeyVersions.asymmetricSign` using the
/// `EC_SIGN_ED25519` algorithm. The private key never leaves Google Cloud.
pub struct GcpKmsSigner {
    client: reqwest::Client,
    key_version_name: String,
    base_url: String,
    token_url: String,
    credentials: GcpCredentials,
    token_cache: tokio::sync::Mutex<Option<CachedToken>>,
    public_key: [u8; 32],
}

#[derive(Debug, Deserialize)]
struct ServiceAccountFile {
    client_email: String,
    private_key: String,
    #[serde(default)]
    token_uri: Option<String>,
}

#[derive(Debug, Serialize)]
struct ServiceAccountJwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: u64,
    exp: u64,
}

impl GcpKmsSigner {
    /// Build a signer for `key_version_name`
    /// (`projects/.../locations/.../keyRings/.../cryptoKeys/.../cryptoKeyVersions/N`).
    ///
    /// Fetches the public key eagerly so a misconfigured key or missing IAM
    /// permission fails at startup rather than at first submission.
    pub async fn new(
        key_version_name: &str,
        credentials: GcpCredentials,
        base_url: Option<String>,
        token_url: Option<String>,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        let mut signer = Self {
            client,
            key_version_name: key_version_name.to_string(),
            base_url: base_url.unwrap_or_else(|| GCP_DEFAULT_BASE_URL.to_string()),
            token_url: token_url.unwrap_or_else(|| GCP_DEFAULT_TOKEN_URL.to_string()),
            credentials,
            token_cache: tokio::sync::Mutex::new(None),
            // Placeholder; replaced by the eager fetch below.
            public_key: [0u8; 32],
        };

        signer.public_key = signer.fetch_public_key().await?;
        Ok(signer)
    }

    /// Build from environment variables (see module docs).
    pub async fn from_env() -> Result<Self> {
        let key_version_name = std::env::var("GCP_KMS_KEY_VERSION_NAME")
            .context("GCP_KMS_KEY_VERSION_NAME environment variable not set")?;

        let credentials = if let Ok(token) = std::env::var("GCP_ACCESS_TOKEN") {
            GcpCredentials::StaticToken(token)
        } else {
            let path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").context(
                "GCP_KMS_KEY_VERSION_NAME is set but no credentials were provided: \
                 set GCP_ACCESS_TOKEN or GOOGLE_APPLICATION_CREDENTIALS",
            )?;
            let contents = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read service account file {path}"))?;
            let sa: ServiceAccountFile = serde_json::from_str(&contents)
                .context("Service account file is not valid JSON")?;
            GcpCredentials::ServiceAccount {
                client_email: sa.client_email,
                private_key_pem: sa.private_key,
            }
        };

        Self::new(
            &key_version_name,
            credentials,
            std::env::var("GCP_KMS_BASE_URL").ok(),
            std::env::var("GCP_KMS_TOKEN_URL").ok(),
        )
        .await
    }

    async fn access_token(&self) -> Result<String> {
        match &self.credentials {
            GcpCredentials::StaticToken(token) => Ok(token.clone()),
            GcpCredentials::ServiceAccount {
                client_email,
                private_key_pem,
            } => {
                let mut cache = self.token_cache.lock().await;
                if let Some(cached) = cache.as_ref() {
                    if cached.expires_at > Instant::now() {
                        return Ok(cached.access_token.clone());
                    }
                }

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let claims = ServiceAccountJwtClaims {
                    iss: client_email.clone(),
                    scope: GCP_KMS_SCOPE.to_string(),
                    aud: self.token_url.clone(),
                    iat: now,
                    exp: now + 3600,
                };
                let jwt = jsonwebtoken::encode(
                    &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256),
                    &claims,
                    &jsonwebtoken::EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
                        .context("Invalid service account private key PEM")?,
                )
                .context("Failed to sign service account JWT")?;

                let response = self
                    .client
                    .post(&self.token_url)
                    .form(&[
                        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                        ("assertion", jwt.as_str()),
                    ])
                    .send()
                    .await
                    .context("Failed to request OAuth2 access token")?;

                let body: serde_json::Value = response
                    .json()
                    .await
                    .context("Failed to parse OAuth2 token response")?;
                let access_token = body
                    .get("access_token")
                    .and_then(|t| t.as_str())
                    .context("OAuth2 token response missing access_token")?
                    .to_string();
                let expires_in = body
                    .get("expires_in")
                    .and_then(|e| e.as_u64())
                    .unwrap_or(3600);

                *cache = Some(CachedToken {
                    access_token: access_token.clone(),
                    expires_at: Instant::now()
                        + Duration::from_secs(expires_in).saturating_sub(TOKEN_EXPIRY_MARGIN),
                });
                Ok(access_token)
            }
        }
    }

    async fn fetch_public_key(&self) -> Result<[u8; 32]> {
        let token = self.access_token().await?;
        let url = format!("{}/v1/{}/publicKey", self.base_url, self.key_version_name);
        let body: serde_json::Value = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to send getPublicKey request")?
            .json()
            .await
            .context("Failed to parse getPublicKey response")?;

        // Preferred: raw key bytes in `publicKey.data` (ChecksummedData).
        if let Some(data) = body
            .get("publicKey")
            .and_then(|p| p.get("data"))
            .and_then(|d| d.as_str())
        {
            let raw = base64_standard
                .decode(data)
                .context("Invalid publicKey.data base64")?;
            return raw_public_key(&raw);
        }

        // Fallback: PEM-encoded SubjectPublicKeyInfo; the raw Ed25519 key is
        // the last 32 bytes of the DER.
        let pem = body
            .get("pem")
            .and_then(|p| p.as_str())
            .context("getPublicKey response has neither publicKey.data nor pem")?;
        let der = pem_to_der(pem).context("Failed to decode public key PEM")?;
        raw_public_key(&der)
    }

    async fn sign_request(&self, message: &[u8]) -> Result<Vec<u8>> {
        let token = self.access_token().await?;
        let url = format!(
            "{}/v1/{}:asymmetricSign",
            self.base_url, self.key_version_name
        );
        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&serde_json::json!({ "data": base64_standard.encode(message) }))
            .send()
            .await
            .context("Failed to send asymmetricSign request")?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse asymmetricSign response")?;
        if !status.is_success() {
            bail!(
                "asymmetricSign failed with status {status}: {}",
                body.to_string()
            );
        }

        let signature_b64 = body
            .get("signature")
            .and_then(|s| s.as_str())
            .context("asymmetricSign response missing signature")?;
        let signature = base64_standard
            .decode(signature_b64)
            .context("Invalid signature base64")?;
        if signature.len() != 64 {
            bail!(
                "asymmetricSign returned {} bytes; expected a 64-byte Ed25519 signature",
                signature.len()
            );
        }
        Ok(signature)
    }
}

/// Reduce a KMS-returned public key blob to the raw 32-byte Ed25519 key.
///
/// Handles both a bare 32-byte key and DER `SubjectPublicKeyInfo` (where the
/// raw key is the trailing 32 bytes).
fn raw_public_key(bytes: &[u8]) -> Result<[u8; 32]> {
    let key_bytes: &[u8] = if bytes.len() == 32 {
        bytes
    } else if bytes.len() > 32 {
        &bytes[bytes.len() - 32..]
    } else {
        bail!(
            "KMS public key is {} bytes; expected at least 32",
            bytes.len()
        );
    };
    key_bytes
        .try_into()
        .map_err(|_| anyhow!("Failed to extract 32-byte public key"))
}

/// Extract DER bytes from a PEM `PUBLIC KEY` block.
fn pem_to_der(pem: &str) -> Result<Vec<u8>> {
    let begin = "-----BEGIN";
    let end = "-----END";
    let start = pem.find(begin).context("PEM missing BEGIN marker")?;
    let body_start = pem[start..]
        .find('\n')
        .map(|i| start + i + 1)
        .unwrap_or(pem.len());
    let end_idx = pem[body_start..]
        .find(end)
        .map(|i| body_start + i)
        .context("PEM missing END marker")?;
    let b64: String = pem[body_start..end_idx]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    base64_standard
        .decode(b64)
        .context("Failed to base64-decode PEM body")
}

#[async_trait]
impl TransactionSigner for GcpKmsSigner {
    fn backend(&self) -> &'static str {
        "gcp-kms"
    }

    fn key_id(&self) -> &str {
        &self.key_version_name
    }

    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        debug!(key = %self.key_version_name, "Signing via GCP KMS");
        self.sign_request(message).await
    }

    async fn public_key(&self) -> Result<[u8; 32]> {
        Ok(self.public_key)
    }
}

// ---------------------------------------------------------------------------
// AWS KMS signer (ECC_NIST_EDWARDS25519) — behind `aws-kms` feature
// ---------------------------------------------------------------------------

#[cfg(feature = "aws-kms")]
mod aws_kms {
    use super::{raw_public_key, TransactionSigner};
    use anyhow::{bail, Context, Result};
    use async_trait::async_trait;
    use tracing::debug;

    /// Signs via AWS KMS `Sign` with `ED25519_SHA_512` / `MessageType::RAW`.
    /// The private key never leaves AWS KMS.
    pub struct AwsKmsSigner {
        client: aws_sdk_kms::Client,
        key_id: String,
        public_key: [u8; 32],
    }

    impl AwsKmsSigner {
        /// Build a signer for `key_id` (key id, ARN, or alias) using the
        /// default AWS credential/region chain (`aws-config`).
        ///
        /// Fetches the public key eagerly so missing IAM permissions or a
        /// wrong key spec fail at startup.
        pub async fn new(key_id: &str) -> Result<Self> {
            let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .load()
                .await;
            if sdk_config.region().is_none() {
                bail!(
                    "No AWS region configured; set AWS_REGION or AWS_DEFAULT_REGION \
                     for the aws-kms signer backend"
                );
            }
            let client = aws_sdk_kms::Client::new(&sdk_config);

            let mut signer = Self {
                client,
                key_id: key_id.to_string(),
                public_key: [0u8; 32],
            };
            signer.public_key = signer.fetch_public_key().await?;
            Ok(signer)
        }

        /// Build from `AWS_KMS_KEY_ID`.
        pub async fn from_env() -> Result<Self> {
            let key_id = std::env::var("AWS_KMS_KEY_ID")
                .context("AWS_KMS_KEY_ID environment variable not set")?;
            Self::new(&key_id).await
        }

        async fn fetch_public_key(&self) -> Result<[u8; 32]> {
            let output = self
                .client
                .get_public_key()
                .key_id(&self.key_id)
                .send()
                .await
                .context("KMS GetPublicKey failed")?;
            let der = output
                .public_key
                .map(|b| b.into_inner())
                .context("KMS GetPublicKey returned no public key")?;
            raw_public_key(&der)
        }

        async fn sign_request(&self, message: &[u8]) -> Result<Vec<u8>> {
            let output = self
                .client
                .sign()
                .key_id(&self.key_id)
                .message(aws_sdk_kms::primitives::Blob::new(message.to_vec()))
                .message_type(aws_sdk_kms::types::MessageType::Raw)
                .signing_algorithm(aws_sdk_kms::types::SigningAlgorithm::Ed25519Sha512)
                .send()
                .await
                .context("KMS Sign failed")?;
            let signature = output
                .signature
                .map(|b| b.into_inner())
                .context("KMS Sign returned no signature")?;
            if signature.len() != 64 {
                bail!(
                    "KMS Sign returned {} bytes; expected a 64-byte Ed25519 signature",
                    signature.len()
                );
            }
            Ok(signature)
        }
    }

    #[async_trait]
    impl TransactionSigner for AwsKmsSigner {
        fn backend(&self) -> &'static str {
            "aws-kms"
        }

        fn key_id(&self) -> &str {
            &self.key_id
        }

        async fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
            debug!(key = %self.key_id, "Signing via AWS KMS");
            self.sign_request(message).await
        }

        async fn public_key(&self) -> Result<[u8; 32]> {
            Ok(self.public_key)
        }
    }
}

#[cfg(feature = "aws-kms")]
pub use aws_kms::AwsKmsSigner;

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Build the configured signer from `TX_SIGNER_BACKEND` (default `local`).
///
/// `local` keeps the secret seed in process memory and is intended for
/// development only. `gcp-kms` and `aws-kms` keep the key in the cloud KMS;
/// `aws-kms` additionally requires the `aws-kms` cargo feature.
pub async fn signer_from_env() -> Result<Arc<dyn TransactionSigner>> {
    let backend = std::env::var("TX_SIGNER_BACKEND").unwrap_or_else(|_| "local".to_string());
    match backend.as_str() {
        "local" | "local-seed" => Ok(Arc::new(LocalSeedSigner::from_env()?)),
        "gcp-kms" => Ok(Arc::new(GcpKmsSigner::from_env().await?)),
        "aws-kms" => {
            #[cfg(feature = "aws-kms")]
            {
                Ok(Arc::new(AwsKmsSigner::from_env().await?))
            }
            #[cfg(not(feature = "aws-kms"))]
            {
                bail!(
                    "TX_SIGNER_BACKEND=aws-kms requires building with \
                     `--features aws-kms` (aws-sdk-kms is an optional dependency)"
                )
            }
        }
        other => bail!(
            "Unknown TX_SIGNER_BACKEND '{other}'; expected 'local', 'gcp-kms', or 'aws-kms'"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_seed() -> String {
        // Deterministic test-only seed; not a real account.
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        stellar_strkey::ed25519::PrivateKey(signing.to_bytes()).to_string()
    }

    #[tokio::test]
    async fn local_signer_roundtrip_and_verify() {
        let signer = LocalSeedSigner::from_secret_seed(&test_seed()).unwrap();
        let message = [7u8; 32];
        let signature = signer.sign(&message).await.unwrap();
        assert_eq!(signature.len(), 64);

        let public_key = signer.public_key().await.unwrap();
        verify_ed25519_signature(&public_key, &message, &signature).unwrap();

        // A signature over different bytes must not verify.
        assert!(verify_ed25519_signature(&public_key, &[8u8; 32], &signature).is_err());
    }

    #[tokio::test]
    async fn local_signer_hint_matches_last_four_pubkey_bytes() {
        let signer = LocalSeedSigner::from_secret_seed(&test_seed()).unwrap();
        let public_key = signer.public_key().await.unwrap();
        let hint = signature_hint_from_public_key(&public_key);
        assert_eq!(hint, public_key[28..32]);
    }

    #[tokio::test]
    async fn local_signer_rejects_bad_seed() {
        assert!(LocalSeedSigner::from_secret_seed("not-a-seed").is_err());
    }

    #[test]
    fn raw_public_key_accepts_bare_and_der() {
        let bare = [3u8; 32];
        assert_eq!(raw_public_key(&bare).unwrap(), bare);

        let mut der = vec![0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00];
        der.extend_from_slice(&bare);
        assert_eq!(raw_public_key(&der).unwrap(), bare);

        assert!(raw_public_key(&[0u8; 10]).is_err());
    }

    #[test]
    fn pem_to_der_roundtrip() {
        use base64::Engine;
        let der = vec![1u8, 2, 3, 4, 5];
        let b64 = base64_standard.encode(&der);
        let pem = format!("-----BEGIN PUBLIC KEY-----\n{b64}\n-----END PUBLIC KEY-----\n");
        assert_eq!(pem_to_der(&pem).unwrap(), der);
    }
}
