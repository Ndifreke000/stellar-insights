//! Database query performance monitoring.
//!
//! - Per-operation Prometheus metrics (duration histogram, slow query and full-scan counters)
//! - Slow query capture with `EXPLAIN QUERY PLAN` output
//! - Slow query log file (JSON lines, `SLOW_QUERY_LOG_PATH`, default `logs/slow_queries.log`)
//! - Index usage report that flags full table scans as missing-index candidates

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Write;
use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry, HistogramVec,
    IntCounterVec,
};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

use super::metrics::REGISTRY;

const MAX_RECORDED_SLOW_QUERIES: usize = 200;

lazy_static! {
    pub static ref DB_QUERY_DURATION_BY_OPERATION: HistogramVec =
        register_histogram_vec_with_registry!(
            "db_query_operation_duration_seconds",
            "Database query duration in seconds by operation and status",
            &["operation", "status"],
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5],
            REGISTRY
        )
        .unwrap();
    pub static ref DB_SLOW_QUERIES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
        "db_slow_queries_total",
        "Number of queries exceeding the slow query threshold, by operation",
        &["operation"],
        REGISTRY
    )
    .unwrap();
    pub static ref DB_FULL_TABLE_SCANS_TOTAL: IntCounterVec =
        register_int_counter_vec_with_registry!(
            "db_full_table_scans_total",
            "Full table scans observed in slow query plans, by table",
            &["table"],
            REGISTRY
        )
        .unwrap();
}

/// A captured slow query with its execution plan.
#[derive(Debug, Clone, Serialize)]
pub struct SlowQueryRecord {
    pub operation: String,
    pub sql: Option<String>,
    pub duration_ms: u64,
    pub threshold_ms: u64,
    pub status: String,
    pub timestamp: DateTime<Utc>,
    /// `EXPLAIN QUERY PLAN` detail lines
    pub plan: Vec<String>,
    /// Tables scanned without an index
    pub full_scan_tables: Vec<String>,
}

/// Aggregated per-operation slow query statistics.
#[derive(Debug, Clone, Serialize)]
pub struct SlowQuerySummary {
    pub operation: String,
    pub count: usize,
    pub max_ms: u64,
    pub avg_ms: u64,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlowQueryReport {
    pub threshold_ms: u64,
    pub summary: Vec<SlowQuerySummary>,
    pub recent: Vec<SlowQueryRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexReport {
    /// Existing indexes per table
    pub indexes: BTreeMap<String, Vec<String>>,
    /// Tables that were fully scanned in slow queries, with the operations responsible
    pub full_scans: BTreeMap<String, BTreeSet<String>>,
    /// Human-readable index suggestions
    pub suggestions: Vec<String>,
}

fn store() -> &'static Mutex<VecDeque<SlowQueryRecord>> {
    static STORE: OnceLock<Mutex<VecDeque<SlowQueryRecord>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_RECORDED_SLOW_QUERIES)))
}

fn log_path() -> String {
    std::env::var("SLOW_QUERY_LOG_PATH").unwrap_or_else(|_| "logs/slow_queries.log".to_string())
}

/// Record the duration of a query in the per-operation histogram.
pub fn observe(operation: &str, status: &str, duration_seconds: f64) {
    DB_QUERY_DURATION_BY_OPERATION
        .with_label_values(&[operation, status])
        .observe(duration_seconds);
}

/// Replace positional parameters (`$1`, `?1`, `?`) with NULL so the statement
/// can be passed to `EXPLAIN QUERY PLAN` without bindings.
fn strip_bind_params(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' || c == '?' {
            while chars.peek().is_some_and(char::is_ascii_digit) {
                chars.next();
            }
            out.push_str("NULL");
        } else {
            out.push(c);
        }
    }
    out
}

/// Run `EXPLAIN QUERY PLAN` for a statement and return the plan detail lines.
pub async fn explain_query_plan(pool: &SqlitePool, sql: &str) -> anyhow::Result<Vec<String>> {
    let stmt = format!("EXPLAIN QUERY PLAN {}", strip_bind_params(sql.trim()));
    let rows = sqlx::query(&stmt).fetch_all(pool).await?;
    Ok(rows
        .iter()
        .filter_map(|r| r.try_get::<String, _>("detail").ok())
        .collect())
}

/// Extract tables that are scanned without an index from plan detail lines.
#[must_use]
pub fn full_scan_tables(plan: &[String]) -> Vec<String> {
    plan.iter()
        .filter(|line| line.starts_with("SCAN ") && !line.contains("USING"))
        .filter_map(|line| line.split_whitespace().nth(1))
        .filter(|table| !table.starts_with("CONSTANT") && !table.starts_with("SUBQUERY"))
        .map(str::to_string)
        .collect()
}

/// Capture a slow query: run EXPLAIN (if SQL is known), update metrics, append to
/// the in-memory buffer and the slow query log file.
pub async fn record_slow_query(
    pool: &SqlitePool,
    operation: &str,
    sql: Option<&str>,
    duration_ms: u64,
    threshold_ms: u64,
    status: &str,
) {
    let plan = match sql {
        Some(sql) => explain_query_plan(pool, sql).await.unwrap_or_else(|e| {
            tracing::debug!(operation, error = %e, "EXPLAIN QUERY PLAN failed");
            Vec::new()
        }),
        None => Vec::new(),
    };
    let full_scans = full_scan_tables(&plan);

    DB_SLOW_QUERIES_TOTAL.with_label_values(&[operation]).inc();
    for table in &full_scans {
        DB_FULL_TABLE_SCANS_TOTAL.with_label_values(&[table.as_str()]).inc();
    }

    let record = SlowQueryRecord {
        operation: operation.to_string(),
        sql: sql.map(|s| s.split_whitespace().collect::<Vec<_>>().join(" ")),
        duration_ms,
        threshold_ms,
        status: status.to_string(),
        timestamp: Utc::now(),
        plan,
        full_scan_tables: full_scans,
    };

    tracing::warn!(
        operation = %record.operation,
        duration_ms,
        threshold_ms,
        sql = record.sql.as_deref().unwrap_or("<unknown>"),
        plan = ?record.plan,
        full_scans = ?record.full_scan_tables,
        "Slow query detected"
    );

    append_to_log_file(&record);

    if let Ok(mut buf) = store().lock() {
        if buf.len() >= MAX_RECORDED_SLOW_QUERIES {
            buf.pop_front();
        }
        buf.push_back(record);
    }
}

fn append_to_log_file(record: &SlowQueryRecord) {
    let path = log_path();
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(line) = serde_json::to_string(record) else {
        return;
    };
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut file) => {
            let _ = writeln!(file, "{line}");
        }
        Err(e) => tracing::debug!(path, error = %e, "Failed to write slow query log"),
    }
}

/// Snapshot of recently captured slow queries with per-operation aggregates.
#[must_use]
pub fn slow_query_report(threshold_ms: u64) -> SlowQueryReport {
    let recent: Vec<SlowQueryRecord> = store()
        .lock()
        .map(|buf| buf.iter().rev().cloned().collect())
        .unwrap_or_default();

    let mut by_op: BTreeMap<&str, (usize, u64, u64, DateTime<Utc>)> = BTreeMap::new();
    for r in &recent {
        let entry = by_op
            .entry(r.operation.as_str())
            .or_insert((0, 0, 0, r.timestamp));
        entry.0 += 1;
        entry.1 = entry.1.max(r.duration_ms);
        entry.2 += r.duration_ms;
        entry.3 = entry.3.max(r.timestamp);
    }
    let mut summary: Vec<SlowQuerySummary> = by_op
        .into_iter()
        .map(|(op, (count, max_ms, total, last_seen))| SlowQuerySummary {
            operation: op.to_string(),
            count,
            max_ms,
            avg_ms: total / count as u64,
            last_seen,
        })
        .collect();
    summary.sort_by(|a, b| b.max_ms.cmp(&a.max_ms));

    SlowQueryReport {
        threshold_ms,
        summary,
        recent,
    }
}

/// Build an index usage report: existing indexes plus tables fully scanned by slow queries.
pub async fn index_report(pool: &SqlitePool) -> anyhow::Result<IndexReport> {
    let rows = sqlx::query(
        "SELECT tbl_name, name FROM sqlite_master \
         WHERE type = 'index' AND tbl_name NOT LIKE 'sqlite_%' AND tbl_name NOT LIKE '_sqlx%' \
         ORDER BY tbl_name, name",
    )
    .fetch_all(pool)
    .await?;

    let mut indexes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in rows {
        let table: String = row.try_get("tbl_name")?;
        let name: String = row.try_get("name")?;
        indexes.entry(table).or_default().push(name);
    }

    let mut full_scans: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Ok(buf) = store().lock() {
        for r in buf.iter() {
            for table in &r.full_scan_tables {
                full_scans
                    .entry(table.clone())
                    .or_default()
                    .insert(r.operation.clone());
            }
        }
    }

    let suggestions = full_scans
        .iter()
        .map(|(table, ops)| {
            let existing = indexes.get(table).map_or(0, Vec::len);
            format!(
                "Table '{table}' is fully scanned by slow operations [{}] ({existing} existing index(es)); \
                 consider an index on the columns used in their WHERE/ORDER BY clauses",
                ops.iter().cloned().collect::<Vec<_>>().join(", ")
            )
        })
        .collect();

    Ok(IndexReport {
        indexes,
        full_scans,
        suggestions,
    })
}
