//! Public snapshot verification (#2597).
//!
//! A snapshot is *verified* when three hashes agree:
//!
//! 1. `stored_hash` — the `snapshots.hash` column written at generation time
//! 2. `recomputed_hash` — `SHA-256(snapshots.data)`, recomputed on every request
//!    over the exact canonical JSON bytes that were originally hashed
//! 3. the hash recorded on-chain by the `payraider` contract for that epoch
//!
//! Editing either the stored JSON or the stored hash breaks (1) == (2), and an
//! off-chain rewrite of both still breaks (2) == (3). Everything here is pure so
//! it can be tested without HTTP, a database or a Soroban RPC node; the I/O
//! lives in [`load_stored_snapshot`] and `ContractService::get_snapshot_record`.

use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use utoipa::ToSchema;

/// Longest identifier accepted on the path: a hyphenated UUID is 36 chars and
/// a `u64` epoch at most 20, so anything longer is rejected before touching SQL.
const MAX_ID_LEN: usize = 64;

/// How the caller addressed the snapshot on `/snapshots/{id}/...`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotRef {
    /// Row id (UUID) from the `snapshots` table
    Id(String),
    /// Epoch number, which is also the on-chain key
    Epoch(u64),
}

/// Error from [`SnapshotRef::parse`]; maps to `400 INVALID_SNAPSHOT_ID`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidSnapshotRef(pub String);

impl SnapshotRef {
    /// Accepts either a positive epoch (`42`) or a snapshot UUID.
    pub fn parse(raw: &str) -> Result<Self, InvalidSnapshotRef> {
        let raw = raw.trim();
        if raw.is_empty() || raw.len() > MAX_ID_LEN {
            return Err(InvalidSnapshotRef(
                "snapshot id must be an epoch number or a snapshot UUID".to_string(),
            ));
        }
        if raw.bytes().all(|b| b.is_ascii_digit()) {
            return match raw.parse::<u64>() {
                Ok(0) => Err(InvalidSnapshotRef("epoch must be greater than 0".to_string())),
                Ok(epoch) => Ok(Self::Epoch(epoch)),
                Err(_) => Err(InvalidSnapshotRef("epoch is out of range".to_string())),
            };
        }
        uuid::Uuid::parse_str(raw)
            .map(|id| Self::Id(id.to_string()))
            .map_err(|_| {
                InvalidSnapshotRef(
                    "snapshot id must be an epoch number or a snapshot UUID".to_string(),
                )
            })
    }
}

/// Snapshot row as persisted by `SnapshotService::store_snapshot_in_database`.
#[derive(Debug, Clone)]
pub struct StoredSnapshot {
    pub id: String,
    pub epoch: u64,
    pub hash: String,
    /// Canonical JSON exactly as it was hashed
    pub data: String,
}

/// Snapshot record read from the `payraider` contract's persistent storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct OnChainRecord {
    pub contract_id: String,
    pub epoch: u64,
    /// Hex-encoded 32-byte hash stored by `submit_snapshot`
    pub hash: String,
    /// Ledger close time (unix seconds) when the snapshot was recorded
    pub timestamp: u64,
    /// Last ledger that modified the snapshots map entry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified_ledger: Option<u32>,
    /// Ledger until which the entry is live before it must be restored
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_until_ledger: Option<u32>,
}

/// Outcome of looking the epoch up on-chain.
#[derive(Debug, Clone)]
pub enum OnChainLookup {
    Found(OnChainRecord),
    /// The contract has no entry for this epoch
    NotAnchored,
    /// No contract is configured on this deployment
    NotConfigured,
    /// The RPC call failed; carries the reason for the response
    Unavailable(String),
}

/// Machine-readable verdict. Only `Verified` sets `verified: true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    /// Stored, recomputed and on-chain hashes all match
    Verified,
    /// Stored data no longer hashes to the stored hash
    Tampered,
    /// Stored data is intact but differs from what was anchored on-chain
    OnChainMismatch,
    /// Stored data is intact but the epoch was never anchored
    NotAnchored,
    /// Integrity could not be checked against the chain (RPC down / unconfigured)
    ChainUnavailable,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct VerificationChecks {
    /// `SHA-256(data) == stored hash`
    pub stored_hash_matches_data: bool,
    /// `SHA-256(data) == on-chain hash`; `null` when no on-chain record was read
    pub on_chain_hash_matches_data: Option<bool>,
}

/// Body of `GET /api/v1/snapshots/{id}/verify`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct VerificationReport {
    pub snapshot_id: String,
    pub epoch: u64,
    pub verified: bool,
    pub status: VerificationStatus,
    pub stored_hash: String,
    pub recomputed_hash: String,
    pub on_chain: Option<OnChainRecord>,
    pub checks: VerificationChecks,
    pub hash_algorithm: &'static str,
    /// Human-readable explanation of `status`
    pub reason: String,
    pub checked_at: String,
}

/// Lower-case hex SHA-256 of the canonical JSON bytes.
#[must_use]
pub fn recompute_hash(data: &str) -> String {
    hex::encode(Sha256::digest(data.as_bytes()))
}

/// Hash comparison that tolerates case and a `0x` prefix.
fn hashes_equal(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.trim().trim_start_matches("0x").to_ascii_lowercase();
    norm(a) == norm(b)
}

/// Combine a stored snapshot with the on-chain lookup into a report.
#[must_use]
pub fn build_report(stored: &StoredSnapshot, lookup: OnChainLookup) -> VerificationReport {
    let recomputed = recompute_hash(&stored.data);
    let stored_ok = hashes_equal(&stored.hash, &recomputed);

    let (on_chain, on_chain_ok, chain_note) = match lookup {
        OnChainLookup::Found(record) => {
            let ok = hashes_equal(&record.hash, &recomputed);
            (Some(record), Some(ok), None)
        }
        OnChainLookup::NotAnchored => (
            None,
            None,
            Some((
                VerificationStatus::NotAnchored,
                format!("epoch {} has no on-chain record", stored.epoch),
            )),
        ),
        OnChainLookup::NotConfigured => (
            None,
            None,
            Some((
                VerificationStatus::ChainUnavailable,
                "no snapshot contract is configured on this deployment".to_string(),
            )),
        ),
        OnChainLookup::Unavailable(err) => (
            None,
            None,
            Some((
                VerificationStatus::ChainUnavailable,
                format!("on-chain record could not be read: {err}"),
            )),
        ),
    };

    // Tampering is reported first: it is decidable without the chain.
    let (status, reason) = if !stored_ok {
        (
            VerificationStatus::Tampered,
            "stored snapshot data does not hash to the stored hash".to_string(),
        )
    } else if let Some(note) = chain_note {
        note
    } else if on_chain_ok == Some(true) {
        (
            VerificationStatus::Verified,
            "stored, recomputed and on-chain hashes match".to_string(),
        )
    } else {
        (
            VerificationStatus::OnChainMismatch,
            "stored data does not match the hash anchored on-chain".to_string(),
        )
    };

    VerificationReport {
        snapshot_id: stored.id.clone(),
        epoch: stored.epoch,
        verified: status == VerificationStatus::Verified,
        status,
        stored_hash: stored.hash.clone(),
        recomputed_hash: recomputed,
        on_chain,
        checks: VerificationChecks {
            stored_hash_matches_data: stored_ok,
            on_chain_hash_matches_data: on_chain_ok,
        },
        hash_algorithm: "sha256",
        reason,
        checked_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// Load the latest analytics snapshot matching `snapshot_ref`.
pub async fn load_stored_snapshot(
    pool: &SqlitePool,
    snapshot_ref: &SnapshotRef,
) -> Result<Option<StoredSnapshot>, sqlx::Error> {
    let base = "SELECT id, epoch, hash, data FROM snapshots \
                WHERE entity_type = 'analytics_snapshot' AND ";
    let row = match snapshot_ref {
        SnapshotRef::Id(id) => {
            sqlx::query(&format!("{base} id = ? LIMIT 1"))
                .bind(id)
                .fetch_optional(pool)
                .await?
        }
        SnapshotRef::Epoch(epoch) => {
            sqlx::query(&format!("{base} epoch = ? ORDER BY created_at DESC LIMIT 1"))
                .bind(*epoch as i64)
                .fetch_optional(pool)
                .await?
        }
    };

    Ok(row.map(|row| StoredSnapshot {
        id: row.get("id"),
        epoch: row.get::<i64, _>("epoch") as u64,
        hash: row.get::<Option<String>, _>("hash").unwrap_or_default(),
        data: row.get::<Option<String>, _>("data").unwrap_or_default(),
    }))
}
