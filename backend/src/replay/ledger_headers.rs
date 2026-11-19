//! Per-ledger protocol versions for replay (#2768).
//!
//! `ContractEvent` carries no protocol version: that lives in the ledger
//! header. Ingestion now records each fetched ledger's header
//! (`ledger_version` decoded from the RPC `headerXdr`) in `ledger_headers`,
//! and the replay engine looks the version up by the event's ledger so that
//! replays spanning a protocol upgrade decode each side with the right rules.

use std::collections::HashMap;

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::SqlitePool;
use stellar_xdr::curr::{LedgerHeaderHistoryEntry, Limits, ReadXdr};

use crate::network::StellarNetwork;
use crate::rpc::stellar::RpcLedger;
use crate::rpc::StellarRpcClient;

/// Source of the protocol version that was active for a given ledger.
#[async_trait]
pub trait ProtocolVersionSource: Send + Sync {
    /// `Ok(None)` when no header is stored for that ledger.
    async fn protocol_version(&self, network: &str, ledger: u64) -> Result<Option<u32>>;
}

/// Decode `ledger_version` from a base64 `LedgerHeaderHistoryEntry` (the
/// `headerXdr` field returned by RPC `getLedgers`).
pub fn protocol_version_from_header_xdr(header_xdr: &str) -> Result<u32> {
    let entry = LedgerHeaderHistoryEntry::from_xdr_base64(header_xdr, Limits::none())
        .context("invalid LedgerHeaderHistoryEntry XDR")?;
    Ok(entry.header.ledger_version)
}

/// SQLite-backed store for `ledger_headers` (migration 058).
#[derive(Clone)]
pub struct LedgerHeaderStore {
    pool: SqlitePool,
}

impl LedgerHeaderStore {
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert or update one ledger header.
    pub async fn upsert(
        &self,
        network: StellarNetwork,
        sequence: u64,
        protocol_version: u32,
        hash: &str,
        close_time: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO ledger_headers (network, sequence, protocol_version, hash, close_time) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(network, sequence) DO UPDATE SET \
               protocol_version = excluded.protocol_version, \
               hash = excluded.hash, close_time = excluded.close_time",
        )
        .bind(network.to_string())
        .bind(sequence as i64)
        .bind(i64::from(protocol_version))
        .bind(hash)
        .bind(close_time)
        .execute(&self.pool)
        .await
        .context("Failed to upsert ledger header")?;
        Ok(())
    }

    /// Record the headers of ledgers fetched from RPC. Ledgers without a
    /// decodable `headerXdr` are skipped (and left for the backfill).
    /// Returns how many headers were stored.
    pub async fn record_rpc_ledgers(
        &self,
        network: StellarNetwork,
        ledgers: &[RpcLedger],
    ) -> Result<u64> {
        let mut stored = 0;
        for ledger in ledgers {
            let Some(xdr) = ledger.header_xdr.as_deref() else {
                continue;
            };
            match protocol_version_from_header_xdr(xdr) {
                Ok(version) => {
                    self.upsert(
                        network,
                        ledger.sequence,
                        version,
                        &ledger.hash,
                        &ledger.ledger_close_time,
                    )
                    .await?;
                    stored += 1;
                }
                Err(e) => {
                    tracing::warn!(ledger = ledger.sequence, error = %e, "Skipping undecodable ledger header");
                }
            }
        }
        Ok(stored)
    }

    /// Ledgers in `[from, to]` that have contract events or ingested ledger
    /// rows but no stored header, ascending, at most `limit`.
    pub async fn missing_header_ledgers(
        &self,
        network: StellarNetwork,
        from: u64,
        to: u64,
        limit: u32,
    ) -> Result<Vec<u64>> {
        let rows: Vec<(i64,)> = sqlx::query_as(
            "SELECT DISTINCT ledger FROM ( \
                SELECT ledger FROM contract_events WHERE ledger BETWEEN ?1 AND ?2 \
                UNION SELECT sequence AS ledger FROM ledgers \
                    WHERE network = ?3 AND sequence BETWEEN ?1 AND ?2 \
             ) l \
             WHERE NOT EXISTS (SELECT 1 FROM ledger_headers h \
                               WHERE h.network = ?3 AND h.sequence = l.ledger) \
             ORDER BY ledger ASC LIMIT ?4",
        )
        .bind(from as i64)
        .bind(to as i64)
        .bind(network.to_string())
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .context("Failed to query ledgers missing headers")?;
        Ok(rows.into_iter().map(|(l,)| l as u64).collect())
    }

    /// Backfill protocol versions for already-ingested ledgers in
    /// `[from, to]` by re-fetching their headers from RPC. Returns how many
    /// headers were stored. Ledgers outside RPC retention stay missing and
    /// replay over them fails explicitly instead of guessing.
    pub async fn backfill_protocol_versions(
        &self,
        rpc: &StellarRpcClient,
        network: StellarNetwork,
        from: u64,
        to: u64,
    ) -> Result<u64> {
        let mut stored = 0;
        loop {
            let missing = self.missing_header_ledgers(network, from, to, 200).await?;
            let Some(&first) = missing.first() else {
                break;
            };
            let page = rpc
                .fetch_ledgers(Some(first), 200, None)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
                .context("RPC getLedgers failed during header backfill")?;
            let wanted: std::collections::HashSet<u64> = missing.into_iter().collect();
            let ledgers: Vec<RpcLedger> = page
                .ledgers
                .into_iter()
                .filter(|l| wanted.contains(&l.sequence))
                .collect();
            let n = self.record_rpc_ledgers(network, &ledgers).await?;
            if n == 0 {
                // Nothing recoverable (out of retention or no header XDR).
                break;
            }
            stored += n;
        }
        Ok(stored)
    }
}

#[async_trait]
impl ProtocolVersionSource for LedgerHeaderStore {
    async fn protocol_version(&self, network: &str, ledger: u64) -> Result<Option<u32>> {
        let v: Option<i64> = sqlx::query_scalar(
            "SELECT protocol_version FROM ledger_headers WHERE network = ? AND sequence = ?",
        )
        .bind(network)
        .bind(ledger as i64)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to read ledger header")?;
        Ok(v.map(|v| v as u32))
    }
}

/// In-memory source, keyed by `(network, ledger)`. Used in tests and for
/// callers that already hold the headers.
#[derive(Debug, Default, Clone)]
pub struct InMemoryProtocolVersions {
    versions: HashMap<(String, u64), u32>,
}

impl InMemoryProtocolVersions {
    #[must_use]
    pub fn with(mut self, network: &str, ledger: u64, version: u32) -> Self {
        self.versions.insert((network.to_string(), ledger), version);
        self
    }
}

#[async_trait]
impl ProtocolVersionSource for InMemoryProtocolVersions {
    async fn protocol_version(&self, network: &str, ledger: u64) -> Result<Option<u32>> {
        Ok(self.versions.get(&(network.to_string(), ledger)).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::curr::{LedgerHeader, WriteXdr};

    fn header_xdr(version: u32) -> String {
        let mut entry = LedgerHeaderHistoryEntry::default();
        entry.header = LedgerHeader {
            ledger_version: version,
            ..LedgerHeader::default()
        };
        entry.to_xdr_base64(Limits::none()).unwrap()
    }

    async fn store() -> LedgerHeaderStore {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::query(crate::db::schema::Schema::CREATE_LEDGER_HEADERS)
            .execute(&pool)
            .await
            .unwrap();
        LedgerHeaderStore::new(pool)
    }

    #[test]
    fn decodes_protocol_version_from_header_xdr() {
        assert_eq!(
            protocol_version_from_header_xdr(&header_xdr(23)).unwrap(),
            23
        );
        assert!(protocol_version_from_header_xdr("garbage").is_err());
    }

    #[tokio::test]
    async fn records_rpc_ledgers_per_network() {
        let store = store().await;
        let ledgers = vec![
            RpcLedger {
                hash: "a".into(),
                sequence: 10,
                ledger_close_time: "t".into(),
                header_xdr: Some(header_xdr(22)),
                metadata_xdr: None,
            },
            RpcLedger {
                hash: "b".into(),
                sequence: 11,
                ledger_close_time: "t".into(),
                header_xdr: None,
                metadata_xdr: None,
            },
        ];
        assert_eq!(
            store
                .record_rpc_ledgers(StellarNetwork::Mainnet, &ledgers)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            store.protocol_version("mainnet", 10).await.unwrap(),
            Some(22)
        );
        assert_eq!(store.protocol_version("mainnet", 11).await.unwrap(), None);
        assert_eq!(store.protocol_version("testnet", 10).await.unwrap(), None);
    }
}
