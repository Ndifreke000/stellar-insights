//! Audit logging for transaction signatures.
//!
//! Every call to a [`crate::services::transaction_signer::TransactionSigner`]
//! — successful, rejected by the envelope policy, or failed — is recorded
//! twice:
//!
//! 1. As a structured `tracing` event on the `signature_audit` target, so it
//!    shows up in the normal log pipeline (ELK, etc.) with no extra setup.
//! 2. As a row in the `transaction_signature_audit` table when a database
//!    pool is configured via `ContractService::with_audit_pool`, giving a
//!    durable, queryable record.
//!
//! Audit writes are best-effort: a database failure is logged as an error
//! but never blocks or fails the signing operation itself. The tracing
//! event is always emitted regardless.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Outcome of a signature attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureStatus {
    /// The envelope passed policy validation and a signature was produced.
    Signed,
    /// The envelope failed policy validation; the signer was never called.
    PolicyRejected,
    /// The signer was invoked but returned an error.
    SignError,
}

impl SignatureStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Signed => "signed",
            Self::PolicyRejected => "policy_rejected",
            Self::SignError => "sign_error",
        }
    }
}

/// One signature audit record.
#[derive(Debug, Clone)]
pub struct SignatureAuditEvent {
    /// Signer backend identifier (`local-seed`, `aws-kms`, `gcp-kms`).
    pub signer_backend: String,
    /// Key identifier within the backend (ARN / key-version name / env var).
    pub signer_key_id: String,
    /// Hex-encoded 32-byte transaction signature base that was signed
    /// (empty when policy rejected before the hash was computed is still
    /// fine — the hash is computed before signing anyway).
    pub tx_hash: String,
    /// Contract the envelope was expected to invoke.
    pub contract_id: String,
    /// Function the envelope was expected to invoke.
    pub function_name: String,
    /// Snapshot epoch being anchored, when known.
    pub epoch: Option<u64>,
    pub status: SignatureStatus,
    /// Error description for non-`Signed` statuses.
    pub error: Option<String>,
    /// Wall-clock time spent in the sign call, in milliseconds.
    pub latency_ms: u64,
}

/// Records signature audit events to tracing and, optionally, SQLite.
#[derive(Clone, Default)]
pub struct SignatureAuditor {
    pool: Option<SqlitePool>,
}

impl SignatureAuditor {
    #[must_use]
    pub const fn new(pool: Option<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Attach a database pool for durable audit records. Without a pool only
    /// the tracing event is emitted.
    #[must_use]
    pub fn with_pool(mut self, pool: SqlitePool) -> Self {
        self.pool = Some(pool);
        self
    }

    /// Record an audit event. Never fails: database errors are logged and
    /// swallowed so audit infrastructure cannot block anchoring.
    pub async fn record(&self, event: &SignatureAuditEvent) {
        tracing::info!(
            target: "signature_audit",
            signer_backend = %event.signer_backend,
            signer_key_id = %event.signer_key_id,
            tx_hash = %event.tx_hash,
            contract_id = %event.contract_id,
            function_name = %event.function_name,
            epoch = ?event.epoch,
            status = %event.status.as_str(),
            error = ?event.error,
            latency_ms = event.latency_ms,
            "transaction signature audit"
        );

        let Some(pool) = &self.pool else {
            return;
        };

        let result = sqlx::query(
            r"
            INSERT INTO transaction_signature_audit
                (id, created_at, signer_backend, signer_key_id, tx_hash,
                 contract_id, function_name, epoch, status, error, latency_ms)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(Utc::now())
        .bind(&event.signer_backend)
        .bind(&event.signer_key_id)
        .bind(&event.tx_hash)
        .bind(&event.contract_id)
        .bind(&event.function_name)
        .bind(event.epoch.map(|e| e as i64))
        .bind(event.status.as_str())
        .bind(&event.error)
        .bind(event.latency_ms as i64)
        .execute(pool)
        .await;

        if let Err(e) = result {
            tracing::error!(
                target: "signature_audit",
                error = %e,
                tx_hash = %event.tx_hash,
                "Failed to persist signature audit record"
            );
        }
    }

    /// Convenience constructor for events; keeps call sites terse.
    #[must_use]
    pub fn event(
        signer_backend: &str,
        signer_key_id: &str,
        tx_hash: &str,
        contract_id: &str,
        function_name: &str,
        epoch: Option<u64>,
        status: SignatureStatus,
        error: Option<String>,
        latency_ms: u64,
    ) -> SignatureAuditEvent {
        SignatureAuditEvent {
            signer_backend: signer_backend.to_string(),
            signer_key_id: signer_key_id.to_string(),
            tx_hash: tx_hash.to_string(),
            contract_id: contract_id.to_string(),
            function_name: function_name.to_string(),
            epoch,
            status,
            error,
            latency_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_strings_are_stable() {
        assert_eq!(SignatureStatus::Signed.as_str(), "signed");
        assert_eq!(SignatureStatus::PolicyRejected.as_str(), "policy_rejected");
        assert_eq!(SignatureStatus::SignError.as_str(), "sign_error");
    }

    #[tokio::test]
    async fn record_without_pool_only_traces() {
        // Must not panic or error with no pool attached.
        let auditor = SignatureAuditor::default();
        auditor
            .record(&SignatureAuditor::event(
                "local-seed",
                "env:STELLAR_SOURCE_SECRET_KEY",
                "abababababababababababababababababababababababababababababababab",
                "CDUMMY",
                "submit_snapshot",
                Some(1),
                SignatureStatus::Signed,
                None,
                3,
            ))
            .await;
    }

    #[tokio::test]
    async fn record_with_pool_inserts_row() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            r"
            CREATE TABLE transaction_signature_audit (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                signer_backend TEXT NOT NULL,
                signer_key_id TEXT NOT NULL,
                tx_hash TEXT NOT NULL,
                contract_id TEXT NOT NULL,
                function_name TEXT NOT NULL,
                epoch INTEGER,
                status TEXT NOT NULL,
                error TEXT,
                latency_ms INTEGER
            )
            ",
        )
        .execute(&pool)
        .await
        .unwrap();

        let auditor = SignatureAuditor::new(Some(pool.clone()));
        auditor
            .record(&SignatureAuditor::event(
                "gcp-kms",
                "projects/p/locations/l/keyRings/r/cryptoKeys/k/cryptoKeyVersions/1",
                &"cd".repeat(32),
                "CDUMMY",
                "submit_snapshot",
                Some(42),
                SignatureStatus::Signed,
                None,
                17,
            ))
            .await;

        let (backend, status, epoch): (String, String, i64) =
            sqlx::query_as("SELECT signer_backend, status, epoch FROM transaction_signature_audit")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(backend, "gcp-kms");
        assert_eq!(status, "signed");
        assert_eq!(epoch, 42);
    }
}
