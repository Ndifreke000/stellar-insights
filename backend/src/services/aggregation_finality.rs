//! Reorg / rollback safety for aggregation (#2771).
//!
//! # Design
//!
//! * **Finality depth** - a ledger is *final* once it is at least
//!   `AGGREGATION_FINALITY_DEPTH` ledgers (default 64, about 5 minutes) behind
//!   the newest ingested ledger of the same network.
//! * **Watermark** - per bucket kind (`hourly`, `daily`) we persist the close
//!   time of the newest final ledger in `aggregation_watermarks`. A bucket is
//!   final when it ends at or before the watermark; anything newer is
//!   *provisional*: it may still be recomputed and is flagged
//!   `provisional: true` in API responses. Snapshots (hashed and anchored
//!   on-chain) only read final buckets.
//! * **Rewrites** - [`upsert_ledger`] is the ledger write path for ingestion
//!   and backfill. When it replaces a ledger whose hash changed and whose
//!   close time is at or below the watermark, the affected hourly and daily
//!   buckets go into `aggregation_recompute_queue`. The aggregation scheduler
//!   drains the queue every run. A ledger rewritten *above* the watermark
//!   needs no queue entry: its buckets are still provisional and the regular
//!   lookback window recomputes them.
//! * **Failure modes** - if the watermark can't be computed (no ledgers yet,
//!   or the table is missing) everything is treated as provisional, so
//!   nothing unverified reaches a snapshot.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Timelike, Utc};
use sqlx::SqlitePool;

/// Default number of ledgers behind head before a ledger is final.
pub const DEFAULT_FINALITY_DEPTH: u64 = 64;

pub const BUCKET_HOURLY: &str = "hourly";
pub const BUCKET_DAILY: &str = "daily";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalityConfig {
    pub depth_ledgers: u64,
}

impl Default for FinalityConfig {
    fn default() -> Self {
        Self {
            depth_ledgers: DEFAULT_FINALITY_DEPTH,
        }
    }
}

impl FinalityConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let depth_ledgers = std::env::var("AGGREGATION_FINALITY_DEPTH")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_FINALITY_DEPTH)
            .clamp(1, 100_000);
        Self { depth_ledgers }
    }
}

/// Start of the hour containing `t`.
#[must_use]
pub fn hour_bucket(t: DateTime<Utc>) -> DateTime<Utc> {
    t.with_minute(0)
        .and_then(|d| d.with_second(0))
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(t)
}

/// Start of the UTC day containing `t`.
#[must_use]
pub fn day_bucket(t: DateTime<Utc>) -> DateTime<Utc> {
    hour_bucket(t).with_hour(0).unwrap_or(t)
}

/// Length of a bucket kind.
#[must_use]
pub fn bucket_len(kind: &str) -> Duration {
    if kind == BUCKET_DAILY {
        Duration::days(1)
    } else {
        Duration::hours(1)
    }
}

/// A bucket is final when it ends at or before the watermark.
#[must_use]
pub fn is_bucket_final(
    kind: &str,
    bucket_start: DateTime<Utc>,
    watermark: Option<DateTime<Utc>>,
) -> bool {
    watermark.is_some_and(|w| bucket_start + bucket_len(kind) <= w)
}

/// Inverse of [`is_bucket_final`], for API responses.
#[must_use]
pub fn is_provisional(
    kind: &str,
    bucket_start: DateTime<Utc>,
    watermark: Option<DateTime<Utc>>,
) -> bool {
    !is_bucket_final(kind, bucket_start, watermark)
}

/// Sequence of the newest final ledger, given the newest ingested one.
#[must_use]
pub const fn finalized_sequence(head: u64, cfg: FinalityConfig) -> Option<u64> {
    head.checked_sub(cfg.depth_ledgers)
}

/// Buckets affected by a rewrite of a ledger that closed at `close_time`.
#[must_use]
pub fn affected_buckets(close_time: DateTime<Utc>) -> Vec<(&'static str, DateTime<Utc>)> {
    vec![
        (BUCKET_HOURLY, hour_bucket(close_time)),
        (BUCKET_DAILY, day_bucket(close_time)),
    ]
}

fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|n| n.and_utc())
        })
}

/// Recompute the watermark from the `ledgers` table and persist it for both
/// bucket kinds. Returns the new watermark, or `None` when too few ledgers
/// have been ingested.
pub async fn advance_watermark(
    pool: &SqlitePool,
    network: &str,
    cfg: FinalityConfig,
) -> Result<Option<DateTime<Utc>>> {
    let head: Option<i64> = sqlx::query_scalar("SELECT MAX(sequence) FROM ledgers WHERE network = ?")
        .bind(network)
        .fetch_one(pool)
        .await
        .context("Failed to read ledger head")?;
    let Some(final_seq) = head.and_then(|h| finalized_sequence(h as u64, cfg)) else {
        return Ok(None);
    };
    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT sequence, close_time FROM ledgers
         WHERE network = ? AND sequence <= ? ORDER BY sequence DESC LIMIT 1",
    )
    .bind(network)
    .bind(final_seq as i64)
    .fetch_optional(pool)
    .await
    .context("Failed to read finalized ledger")?;
    let Some((seq, close_time)) = row else {
        return Ok(None);
    };
    let Some(watermark) = parse_ts(&close_time) else {
        return Ok(None);
    };
    for kind in [BUCKET_HOURLY, BUCKET_DAILY] {
        // Never move the watermark backwards: a late, out-of-order ledger
        // insert must not turn final buckets provisional again.
        sqlx::query(
            "INSERT INTO aggregation_watermarks
                (bucket_kind, network, watermark, finalized_ledger, finality_depth, updated_at)
             VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(bucket_kind, network) DO UPDATE SET
                watermark = excluded.watermark,
                finalized_ledger = excluded.finalized_ledger,
                finality_depth = excluded.finality_depth,
                updated_at = CURRENT_TIMESTAMP
             WHERE excluded.finalized_ledger > aggregation_watermarks.finalized_ledger",
        )
        .bind(kind)
        .bind(network)
        .bind(watermark.to_rfc3339())
        .bind(seq)
        .bind(cfg.depth_ledgers as i64)
        .execute(pool)
        .await
        .context("Failed to persist aggregation watermark")?;
    }
    load_watermark(pool, BUCKET_HOURLY, network).await
}

/// Load the persisted watermark (None if unknown or the table is missing).
pub async fn load_watermark(
    pool: &SqlitePool,
    kind: &str,
    network: &str,
) -> Result<Option<DateTime<Utc>>> {
    let row: Option<String> = sqlx::query_scalar(
        "SELECT watermark FROM aggregation_watermarks WHERE bucket_kind = ? AND network = ?",
    )
    .bind(kind)
    .bind(network)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    Ok(row.as_deref().and_then(parse_ts))
}

/// Outcome of a ledger write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerWrite {
    Inserted,
    Unchanged,
    /// Hash changed above the watermark: buckets are still provisional.
    RewrittenProvisional,
    /// Hash changed at/below the watermark: recomputation enqueued.
    RewrittenFinal,
}

/// Insert or replace a ledger row, enqueueing recomputation of final buckets
/// when an already-final ledger is rewritten with different data.
pub async fn upsert_ledger(
    pool: &SqlitePool,
    network: &str,
    sequence: u64,
    hash: &str,
    close_time: DateTime<Utc>,
) -> Result<LedgerWrite> {
    let existing: Option<(String, String)> =
        sqlx::query_as("SELECT hash, close_time FROM ledgers WHERE sequence = ? AND network = ?")
            .bind(sequence as i64)
            .bind(network)
            .fetch_optional(pool)
            .await
            .context("Failed to read existing ledger")?;

    sqlx::query(
        "INSERT INTO ledgers (sequence, hash, close_time, network) VALUES (?, ?, ?, ?)
         ON CONFLICT(sequence) DO UPDATE SET
            hash = excluded.hash, close_time = excluded.close_time, network = excluded.network",
    )
    .bind(sequence as i64)
    .bind(hash)
    .bind(close_time.to_rfc3339())
    .bind(network)
    .execute(pool)
    .await
    .context("Failed to upsert ledger")?;

    let Some((old_hash, old_close)) = existing else {
        return Ok(LedgerWrite::Inserted);
    };
    if old_hash == hash {
        return Ok(LedgerWrite::Unchanged);
    }

    let watermark = load_watermark(pool, BUCKET_HOURLY, network).await?;
    let old_close = parse_ts(&old_close).unwrap_or(close_time);
    let mut touched = affected_buckets(close_time);
    if old_close != close_time {
        touched.extend(affected_buckets(old_close));
    }
    let mut any_final = false;
    for (kind, start) in touched {
        if is_bucket_final(kind, start, watermark) {
            any_final = true;
            enqueue_recompute(
                pool,
                kind,
                start,
                &format!("ledger {sequence} rewritten ({old_hash} -> {hash})"),
            )
            .await?;
        }
    }
    Ok(if any_final {
        LedgerWrite::RewrittenFinal
    } else {
        LedgerWrite::RewrittenProvisional
    })
}

/// Enqueue a bucket for recomputation (deduplicated while pending).
pub async fn enqueue_recompute(
    pool: &SqlitePool,
    kind: &str,
    bucket_start: DateTime<Utc>,
    reason: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO aggregation_recompute_queue (bucket_kind, bucket_start, reason)
         VALUES (?, ?, ?)",
    )
    .bind(kind)
    .bind(bucket_start.to_rfc3339())
    .bind(reason)
    .execute(pool)
    .await
    .context("Failed to enqueue aggregate recomputation")?;
    Ok(())
}

/// A pending recomputation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecomputeJob {
    pub id: i64,
    pub bucket_kind: String,
    pub bucket_start: DateTime<Utc>,
}

pub async fn pending_recomputes(pool: &SqlitePool, limit: i64) -> Result<Vec<RecomputeJob>> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, bucket_kind, bucket_start FROM aggregation_recompute_queue
         WHERE processed_at IS NULL ORDER BY id LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("Failed to read recompute queue")?;
    Ok(rows
        .into_iter()
        .filter_map(|(id, bucket_kind, start)| {
            parse_ts(&start).map(|bucket_start| RecomputeJob {
                id,
                bucket_kind,
                bucket_start,
            })
        })
        .collect())
}

pub async fn mark_recomputed(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE aggregation_recompute_queue SET processed_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to mark recompute job done")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 2, h, m, 0).unwrap()
    }

    async fn pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE ledgers (sequence INTEGER PRIMARY KEY, hash TEXT NOT NULL,
             close_time TEXT NOT NULL, transaction_count INTEGER DEFAULT 0,
             operation_count INTEGER DEFAULT 0, created_at TEXT DEFAULT CURRENT_TIMESTAMP,
             network TEXT NOT NULL DEFAULT 'mainnet')",
        )
        .execute(&pool)
        .await
        .unwrap();
        for stmt in include_str!("../../migrations/053_create_aggregation_finality.sql")
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.lines().all(|l| l.trim_start().starts_with("--")))
        {
            sqlx::query(stmt).execute(&pool).await.unwrap();
        }
        pool
    }

    #[test]
    fn bucket_finality_uses_bucket_end() {
        let w = Some(ts(12, 30));
        assert!(is_bucket_final(BUCKET_HOURLY, ts(11, 0), w));
        assert!(!is_bucket_final(BUCKET_HOURLY, ts(12, 0), w));
        assert!(is_provisional(BUCKET_HOURLY, ts(12, 0), w));
        // Boundary: a bucket ending exactly at the watermark is final.
        assert!(is_bucket_final(BUCKET_HOURLY, ts(11, 0), Some(ts(12, 0))));
        // Unknown watermark => everything provisional.
        assert!(is_provisional(BUCKET_HOURLY, ts(1, 0), None));
        assert!(!is_bucket_final(BUCKET_DAILY, day_bucket(ts(1, 0)), w));
    }

    #[test]
    fn finalized_sequence_respects_depth() {
        let cfg = FinalityConfig { depth_ledgers: 10 };
        assert_eq!(finalized_sequence(100, cfg), Some(90));
        assert_eq!(finalized_sequence(5, cfg), None);
    }

    #[tokio::test]
    async fn rewrite_below_watermark_enqueues_only_affected_buckets() {
        let pool = pool().await;
        let cfg = FinalityConfig { depth_ledgers: 2 };
        // Ledgers at 10:05, 11:05, 12:05, 13:05.
        for (i, h) in [10, 11, 12, 13].iter().enumerate() {
            upsert_ledger(&pool, "mainnet", i as u64 + 1, &format!("h{i}"), ts(*h, 5))
                .await
                .unwrap();
        }
        let w = advance_watermark(&pool, "mainnet", cfg).await.unwrap();
        assert_eq!(w, Some(ts(11, 5)));

        // Rewrite ledger 1 (10:05, final hour bucket) with a different hash.
        let res = upsert_ledger(&pool, "mainnet", 1, "h0-corrected", ts(10, 5))
            .await
            .unwrap();
        assert_eq!(res, LedgerWrite::RewrittenFinal);
        let jobs = pending_recomputes(&pool, 10).await.unwrap();
        assert_eq!(jobs.len(), 1, "only the 10:00 hourly bucket is final");
        assert_eq!(jobs[0].bucket_kind, BUCKET_HOURLY);
        assert_eq!(jobs[0].bucket_start, ts(10, 0));

        // Rewriting again while pending does not duplicate the job.
        upsert_ledger(&pool, "mainnet", 1, "h0-again", ts(10, 5))
            .await
            .unwrap();
        assert_eq!(pending_recomputes(&pool, 10).await.unwrap().len(), 1);

        mark_recomputed(&pool, jobs[0].id).await.unwrap();
        assert!(pending_recomputes(&pool, 10).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn rewrite_above_watermark_is_provisional_and_not_enqueued() {
        let pool = pool().await;
        let cfg = FinalityConfig { depth_ledgers: 2 };
        for (i, h) in [10, 11, 12, 13].iter().enumerate() {
            upsert_ledger(&pool, "mainnet", i as u64 + 1, &format!("h{i}"), ts(*h, 5))
                .await
                .unwrap();
        }
        advance_watermark(&pool, "mainnet", cfg).await.unwrap();
        let res = upsert_ledger(&pool, "mainnet", 4, "h3-corrected", ts(13, 5))
            .await
            .unwrap();
        assert_eq!(res, LedgerWrite::RewrittenProvisional);
        assert!(pending_recomputes(&pool, 10).await.unwrap().is_empty());
        // Same hash => unchanged.
        let res = upsert_ledger(&pool, "mainnet", 4, "h3-corrected", ts(13, 5))
            .await
            .unwrap();
        assert_eq!(res, LedgerWrite::Unchanged);
    }

    #[tokio::test]
    async fn watermark_unknown_without_enough_ledgers() {
        let pool = pool().await;
        upsert_ledger(&pool, "mainnet", 1, "h", ts(10, 0)).await.unwrap();
        let w = advance_watermark(&pool, "mainnet", FinalityConfig { depth_ledgers: 64 })
            .await
            .unwrap();
        assert_eq!(w, None);
        assert_eq!(load_watermark(&pool, BUCKET_HOURLY, "mainnet").await.unwrap(), None);
    }
}
