//! Account lineage: follow `account_merge` history to the surviving account (#2773).
//!
//! Anchors rotate and merge operational accounts. Each merge is stored as one
//! edge `merged_from -> merged_into` in `account_lineage`. To attribute an
//! account's activity, follow its edges to the account that survives the merges
//! (e.g. A -> B -> C gives C) and look up C's anchor.
//!
//! Resolution is bounded ([`MAX_LINEAGE_DEPTH`] hops) and cycle-safe. Stellar
//! merges cannot normally form a cycle, but a recreated account that is merged
//! again can. If a cycle is found, the account resolves to itself and
//! `cycle_detected` is set, so nothing is attributed to the wrong anchor.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::{Pool, Sqlite};

/// Maximum number of merge hops followed when resolving an account.
pub const MAX_LINEAGE_DEPTH: u32 = 16;

/// One merge edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct LineageEdge {
    pub merged_from: String,
    pub merged_into: String,
    pub ledger_sequence: i64,
}

/// Result of resolving an account through its merge history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LineageResolution {
    /// The account the input was asked about.
    pub account: String,
    /// The account that survives the merges (the input itself if there are
    /// none, or if resolution failed).
    pub surviving_account: String,
    /// Accounts visited, starting with `account`.
    pub chain: Vec<String>,
    pub cycle_detected: bool,
    pub depth_exceeded: bool,
}

/// Resolve `account` against an in-memory edge map (`merged_from -> merged_into`).
/// Used by tests and batch callers that already loaded the lineage.
#[must_use]
pub fn resolve_in_memory(
    edges: &HashMap<String, String>,
    account: &str,
    max_depth: u32,
) -> LineageResolution {
    let mut chain = vec![account.to_string()];
    let mut seen: HashSet<&str> = HashSet::from([account]);
    let mut current = account;
    let mut depth = 0;
    while let Some(next) = edges.get(current) {
        if seen.contains(next.as_str()) {
            return LineageResolution {
                account: account.to_string(),
                surviving_account: account.to_string(),
                chain,
                cycle_detected: true,
                depth_exceeded: false,
            };
        }
        if depth == max_depth {
            return LineageResolution {
                account: account.to_string(),
                surviving_account: account.to_string(),
                chain,
                cycle_detected: false,
                depth_exceeded: true,
            };
        }
        seen.insert(next.as_str());
        chain.push(next.clone());
        current = next.as_str();
        depth += 1;
    }
    LineageResolution {
        account: account.to_string(),
        surviving_account: current.to_string(),
        chain,
        cycle_detected: false,
        depth_exceeded: false,
    }
}

/// Map every account to the anchor that owns its surviving account.
/// `anchor_accounts` maps an anchor's Stellar account to its anchor id.
#[must_use]
pub fn attribute_to_anchor<'a>(
    edges: &HashMap<String, String>,
    anchor_accounts: &'a HashMap<String, String>,
    account: &str,
) -> Option<&'a String> {
    if let Some(anchor) = anchor_accounts.get(account) {
        return Some(anchor);
    }
    let resolution = resolve_in_memory(edges, account, MAX_LINEAGE_DEPTH);
    if resolution.cycle_detected || resolution.depth_exceeded {
        return None;
    }
    anchor_accounts.get(&resolution.surviving_account)
}

/// Database access for `account_lineage` and `lineage_recompute_queue`.
#[derive(Clone)]
pub struct AccountLineage {
    pool: Pool<Sqlite>,
}

impl AccountLineage {
    #[must_use]
    pub const fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Record a merge edge and queue a corridor-metrics recompute. Returns
    /// `false` (nothing recorded) for a self-merge or an edge older than the
    /// one already stored for `merged_from`.
    pub async fn record_merge(
        &self,
        merged_from: &str,
        merged_into: &str,
        ledger_sequence: i64,
        operation_id: Option<&str>,
    ) -> Result<bool> {
        if merged_from.is_empty() || merged_into.is_empty() || merged_from == merged_into {
            return Ok(false);
        }
        let mut tx = self.pool.begin().await.context("begin lineage tx")?;
        let result = sqlx::query(
            r"
            INSERT INTO account_lineage (merged_from, merged_into, ledger_sequence, operation_id)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (merged_from) DO UPDATE SET
                merged_into = excluded.merged_into,
                ledger_sequence = excluded.ledger_sequence,
                operation_id = excluded.operation_id
            WHERE excluded.ledger_sequence > account_lineage.ledger_sequence
            ",
        )
        .bind(merged_from)
        .bind(merged_into)
        .bind(ledger_sequence)
        .bind(operation_id)
        .execute(&mut *tx)
        .await
        .context("Failed to record account lineage")?;

        let changed = result.rows_affected() > 0;
        if changed {
            sqlx::query(
                "INSERT INTO lineage_recompute_queue (account, ledger_sequence) VALUES ($1, $2)",
            )
            .bind(merged_from)
            .bind(ledger_sequence)
            .execute(&mut *tx)
            .await
            .context("Failed to queue lineage recompute")?;
        }
        tx.commit().await.context("commit lineage tx")?;
        Ok(changed)
    }

    /// Resolve `account` to its surviving account with a bounded recursive CTE.
    pub async fn resolve(&self, account: &str) -> Result<LineageResolution> {
        // `path` is a '|'-delimited list of visited accounts; the instr()
        // guard stops the recursion from revisiting an account (cycle).
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r"
            WITH RECURSIVE chain(account, depth, path) AS (
                SELECT $1, 0, '|' || $1 || '|'
                UNION ALL
                SELECT l.merged_into, c.depth + 1, c.path || l.merged_into || '|'
                FROM account_lineage l
                JOIN chain c ON l.merged_from = c.account
                WHERE c.depth < $2
                  AND instr(c.path, '|' || l.merged_into || '|') = 0
            )
            SELECT account, depth FROM chain ORDER BY depth ASC
            ",
        )
        .bind(account)
        .bind(i64::from(MAX_LINEAGE_DEPTH))
        .fetch_all(&self.pool)
        .await
        .context("Failed to resolve account lineage")?;

        let chain: Vec<String> = rows.iter().map(|(a, _)| a.clone()).collect();
        let last = chain.last().cloned().unwrap_or_else(|| account.to_string());
        let last_depth = rows.last().map_or(0, |(_, d)| *d);

        // The CTE stopped either because `last` has no outgoing edge (resolved),
        // because the next hop was already visited (cycle), or because the
        // depth bound was hit.
        let next: Option<(String,)> =
            sqlx::query_as("SELECT merged_into FROM account_lineage WHERE merged_from = $1")
                .bind(&last)
                .fetch_optional(&self.pool)
                .await
                .context("Failed to read lineage edge")?;

        let (cycle_detected, depth_exceeded) = match next {
            None => (false, false),
            Some((n,)) if chain.contains(&n) => (true, false),
            Some(_) => (false, last_depth >= i64::from(MAX_LINEAGE_DEPTH)),
        };
        let surviving_account = if cycle_detected || depth_exceeded {
            account.to_string()
        } else {
            last
        };
        Ok(LineageResolution {
            account: account.to_string(),
            surviving_account,
            chain,
            cycle_detected,
            depth_exceeded,
        })
    }

    /// Every account whose lineage ends at `surviving` (direct and indirect
    /// merges), nearest first. Used by the anchor detail API.
    pub async fn merged_into(&self, surviving: &str) -> Result<Vec<LineageEdge>> {
        let rows = sqlx::query_as::<_, LineageEdge>(
            r"
            WITH RECURSIVE ancestors(merged_from, merged_into, ledger_sequence, depth, path) AS (
                SELECT merged_from, merged_into, ledger_sequence, 1,
                       '|' || $1 || '|' || merged_from || '|'
                FROM account_lineage WHERE merged_into = $1 AND merged_from <> $1
                UNION ALL
                SELECT l.merged_from, l.merged_into, l.ledger_sequence, a.depth + 1,
                       a.path || l.merged_from || '|'
                FROM account_lineage l
                JOIN ancestors a ON l.merged_into = a.merged_from
                WHERE a.depth < $2
                  AND instr(a.path, '|' || l.merged_from || '|') = 0
            )
            SELECT merged_from, merged_into, ledger_sequence
            FROM ancestors
            ORDER BY depth ASC, ledger_sequence DESC
            ",
        )
        .bind(surviving)
        .bind(i64::from(MAX_LINEAGE_DEPTH))
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch merged-account lineage")?;
        Ok(rows)
    }

    /// Load all edges (`merged_from -> merged_into`) for batch resolution.
    pub async fn load_edges(&self) -> Result<HashMap<String, String>> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT merged_from, merged_into FROM account_lineage")
                .fetch_all(&self.pool)
                .await
                .context("Failed to load account lineage")?;
        Ok(rows.into_iter().collect())
    }

    /// Ids of pending recompute requests.
    pub async fn pending_recomputes(&self) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> = sqlx::query_as(
            "SELECT id FROM lineage_recompute_queue WHERE processed_at IS NULL ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to read lineage recompute queue")?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Mark recompute requests up to and including `max_id` as processed.
    pub async fn mark_recomputed(&self, max_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE lineage_recompute_queue SET processed_at = CURRENT_TIMESTAMP \
             WHERE processed_at IS NULL AND id <= $1",
        )
        .bind(max_id)
        .execute(&self.pool)
        .await
        .context("Failed to mark lineage recompute processed")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    fn edges(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(a, b)| ((*a).to_string(), (*b).to_string()))
            .collect()
    }

    async fn pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE account_merges (operation_id TEXT PRIMARY KEY, transaction_hash TEXT, \
             ledger_sequence INTEGER, source_account TEXT, destination_account TEXT, \
             merged_balance REAL, created_at DATETIME)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!("../../migrations/053_create_account_lineage.sql"))
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[test]
    fn chain_a_b_c_resolves_to_c() {
        let e = edges(&[("A", "B"), ("B", "C")]);
        let r = resolve_in_memory(&e, "A", MAX_LINEAGE_DEPTH);
        assert_eq!(r.surviving_account, "C");
        assert_eq!(r.chain, vec!["A", "B", "C"]);
        assert!(!r.cycle_detected);
    }

    #[test]
    fn unmerged_account_resolves_to_itself() {
        let r = resolve_in_memory(&HashMap::new(), "X", MAX_LINEAGE_DEPTH);
        assert_eq!(r.surviving_account, "X");
        assert_eq!(r.chain, vec!["X"]);
    }

    #[test]
    fn cycle_is_detected_and_not_followed() {
        let e = edges(&[("A", "B"), ("B", "C"), ("C", "A")]);
        let r = resolve_in_memory(&e, "A", MAX_LINEAGE_DEPTH);
        assert!(r.cycle_detected);
        assert_eq!(r.surviving_account, "A");
    }

    #[test]
    fn depth_bound_is_enforced() {
        let pairs: Vec<(String, String)> =
            (0..40).map(|i| (format!("N{i}"), format!("N{}", i + 1))).collect();
        let e: HashMap<String, String> = pairs.into_iter().collect();
        let r = resolve_in_memory(&e, "N0", 16);
        assert!(r.depth_exceeded);
        assert_eq!(r.surviving_account, "N0");
        assert_eq!(r.chain.len(), 17);
    }

    #[test]
    fn payments_from_a_are_attributed_to_cs_anchor() {
        let e = edges(&[("A", "B"), ("B", "C")]);
        let anchors: HashMap<String, String> =
            [("C".to_string(), "anchor-c".to_string())].into_iter().collect();
        assert_eq!(
            attribute_to_anchor(&e, &anchors, "A").map(String::as_str),
            Some("anchor-c")
        );
        assert_eq!(attribute_to_anchor(&e, &anchors, "Z"), None);
        let cyclic = edges(&[("A", "B"), ("B", "A")]);
        assert_eq!(attribute_to_anchor(&cyclic, &anchors, "A"), None);
    }

    #[tokio::test]
    async fn sql_chain_resolves_and_queues_recompute() {
        let lineage = AccountLineage::new(pool().await);
        assert!(lineage.record_merge("A", "B", 10, Some("op1")).await.unwrap());
        assert!(lineage.record_merge("B", "C", 20, Some("op2")).await.unwrap());

        let r = lineage.resolve("A").await.unwrap();
        assert_eq!(r.surviving_account, "C");
        assert_eq!(r.chain, vec!["A", "B", "C"]);
        assert!(!r.cycle_detected && !r.depth_exceeded);

        let merged = lineage.merged_into("C").await.unwrap();
        let froms: Vec<&str> = merged.iter().map(|e| e.merged_from.as_str()).collect();
        assert_eq!(froms, vec!["B", "A"]);

        let pending = lineage.pending_recomputes().await.unwrap();
        assert_eq!(pending.len(), 2);
        lineage.mark_recomputed(*pending.last().unwrap()).await.unwrap();
        assert!(lineage.pending_recomputes().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn sql_cycle_is_safe() {
        let lineage = AccountLineage::new(pool().await);
        lineage.record_merge("A", "B", 10, None).await.unwrap();
        lineage.record_merge("B", "A", 20, None).await.unwrap();
        let r = lineage.resolve("A").await.unwrap();
        assert!(r.cycle_detected);
        assert_eq!(r.surviving_account, "A");
        // Reverse walk terminates too.
        let merged = lineage.merged_into("A").await.unwrap();
        assert_eq!(merged.len(), 1);
    }

    #[tokio::test]
    async fn self_merge_and_stale_edges_are_ignored() {
        let lineage = AccountLineage::new(pool().await);
        assert!(!lineage.record_merge("A", "A", 5, None).await.unwrap());
        assert!(lineage.record_merge("A", "B", 10, None).await.unwrap());
        // Older ledger for the same source does not overwrite.
        assert!(!lineage.record_merge("A", "X", 9, None).await.unwrap());
        assert_eq!(lineage.resolve("A").await.unwrap().surviving_account, "B");
        // Newer ledger (recreated + merged again) wins.
        assert!(lineage.record_merge("A", "Y", 11, None).await.unwrap());
        assert_eq!(lineage.resolve("A").await.unwrap().surviving_account, "Y");
    }
}
