//! Executed path-payment routes and per-corridor route statistics (#2774).
//!
//! Horizon path-payment operations carry a `path` array of intermediate
//! assets that we previously discarded. This module:
//!
//! 1. Decodes the executed route (source asset, intermediates, destination
//!    asset, amounts and the quoted bound) from a Horizon operation record,
//!    plus per-hop amounts from the operation's `trade` effects when present.
//! 2. Persists routes to `executed_routes` (migration 054), idempotently keyed
//!    on the Horizon operation id.
//! 3. Aggregates a [`RouteStats`] view per corridor: hop-count distribution,
//!    most-used intermediate assets, success rate and slippage vs. quoted.
//! 4. Exposes [`historical_route_adjustment`], which preflight applies only
//!    when the `PREFLIGHT_HISTORICAL_ROUTES` feature flag is on.
//!
//! Asset strings use the corridor format: `CODE:ISSUER` or `native`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::{BTreeMap, HashMap};

use crate::services::preflight::{wilson_lower_bound, PreflightStatus};

/// Env var that turns on historical-route scoring in preflight.
pub const HISTORICAL_ROUTES_FLAG: &str = "PREFLIGHT_HISTORICAL_ROUTES";

/// Whether preflight should use historical routes (feature flag, default off).
#[must_use]
pub fn historical_routes_enabled() -> bool {
    std::env::var(HISTORICAL_ROUTES_FLAG)
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes"))
        .unwrap_or(false)
}

/// One executed path payment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutedRoute {
    /// Horizon operation id (idempotency key).
    pub operation_id: String,
    pub transaction_hash: String,
    pub ledger: Option<u64>,
    /// `path_payment_strict_send` or `path_payment_strict_receive`.
    pub operation_type: String,
    pub source_asset: String,
    pub destination_asset: String,
    /// Intermediate assets, in order. Empty for a direct (1-hop) conversion.
    pub path: Vec<String>,
    pub source_amount: Option<f64>,
    pub destination_amount: Option<f64>,
    /// Quoted bound: `destination_min` (strict-send) or `source_max`
    /// (strict-receive).
    pub quoted_amount: Option<f64>,
    /// Per-hop amounts bought, from `trade` effects, when available.
    pub hop_amounts: Vec<f64>,
    pub successful: bool,
}

impl ExecutedRoute {
    /// Number of conversion hops: `path.len() + 1`.
    #[must_use]
    pub fn hop_count(&self) -> usize {
        self.path.len() + 1
    }

    #[must_use]
    pub fn corridor_key(&self) -> String {
        format!("{}->{}", self.source_asset, self.destination_asset)
    }

    /// Slippage vs. the quoted bound as a fraction (positive = better than
    /// quoted). Strict-send: `(dest - dest_min) / dest_min`. Strict-receive:
    /// `(source_max - source) / source_max`.
    #[must_use]
    pub fn slippage_vs_quoted(&self) -> Option<f64> {
        let quoted = self.quoted_amount.filter(|q| *q > 0.0)?;
        if self.operation_type.ends_with("strict_receive") {
            Some((quoted - self.source_amount?) / quoted)
        } else {
            Some((self.destination_amount? - quoted) / quoted)
        }
    }

    /// Decode a Horizon operation (or payment) JSON record. Returns `None`
    /// for anything that is not a path payment.
    #[must_use]
    pub fn from_horizon_operation(op: &Value) -> Option<Self> {
        let op_type = op.get("type")?.as_str()?;
        if !op_type.starts_with("path_payment") {
            return None;
        }
        let path = op
            .get("path")
            .and_then(Value::as_array)
            .map(|hops| hops.iter().filter_map(|h| asset_from_json(h, "")).collect())
            .unwrap_or_default();
        let quoted_key = if op_type.ends_with("strict_receive") {
            "source_max"
        } else {
            "destination_min"
        };
        Some(Self {
            operation_id: str_field(op, "id")?,
            transaction_hash: str_field(op, "transaction_hash").unwrap_or_default(),
            ledger: op.get("ledger").and_then(Value::as_u64),
            operation_type: op_type.to_string(),
            source_asset: asset_from_json(op, "source_")?,
            destination_asset: asset_from_json(op, "")?,
            path,
            source_amount: num_field(op, "source_amount"),
            destination_amount: num_field(op, "amount"),
            quoted_amount: num_field(op, quoted_key),
            hop_amounts: Vec::new(),
            successful: op
                .get("transaction_successful")
                .and_then(Value::as_bool)
                .unwrap_or(true),
        })
    }

    /// Attach per-hop amounts from the operation's effects (`trade` effects
    /// in execution order; `bought_amount` per hop).
    #[must_use]
    pub fn with_effects(mut self, effects: &[Value]) -> Self {
        self.hop_amounts = effects
            .iter()
            .filter(|e| e.get("type").and_then(Value::as_str) == Some("trade"))
            .filter_map(|e| num_field(e, "bought_amount"))
            .collect();
        self
    }
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

fn num_field(v: &Value, key: &str) -> Option<f64> {
    match v.get(key)? {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

/// Read `<prefix>asset_type` / `<prefix>asset_code` / `<prefix>asset_issuer`.
fn asset_from_json(v: &Value, prefix: &str) -> Option<String> {
    let ty = v.get(format!("{prefix}asset_type"))?.as_str()?;
    if ty == "native" {
        return Some("native".to_string());
    }
    let code = v.get(format!("{prefix}asset_code"))?.as_str()?;
    let issuer = v
        .get(format!("{prefix}asset_issuer"))
        .and_then(Value::as_str)
        .unwrap_or("");
    Some(format!("{code}:{issuer}"))
}

/// Count of routes using an intermediate asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntermediateUsage {
    pub asset: String,
    pub count: u64,
}

/// Aggregate route statistics for one corridor.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RouteStats {
    pub corridor_key: String,
    pub total_routes: u64,
    pub successful_routes: u64,
    /// Hop count -> number of routes.
    pub hop_count_distribution: BTreeMap<usize, u64>,
    /// Most-used intermediate assets, descending.
    pub top_intermediates: Vec<IntermediateUsage>,
    /// Mean slippage vs. quoted over successful routes with a quote.
    pub avg_slippage_vs_quoted: Option<f64>,
    /// Successful / total, if any routes.
    pub success_rate: Option<f64>,
    /// Wilson lower bound on the success rate.
    pub success_rate_lower_bound: Option<f64>,
}

/// Build [`RouteStats`] for `corridor_key` from executed routes.
#[must_use]
pub fn aggregate_route_stats(corridor_key: &str, routes: &[ExecutedRoute]) -> RouteStats {
    let mut hops: BTreeMap<usize, u64> = BTreeMap::new();
    let mut intermediates: HashMap<&str, u64> = HashMap::new();
    let mut slippage_sum = 0.0;
    let mut slippage_n = 0u64;
    let mut ok = 0u64;

    for r in routes {
        *hops.entry(r.hop_count()).or_default() += 1;
        for a in &r.path {
            *intermediates.entry(a.as_str()).or_default() += 1;
        }
        if r.successful {
            ok += 1;
            if let Some(s) = r.slippage_vs_quoted() {
                slippage_sum += s;
                slippage_n += 1;
            }
        }
    }

    let mut top: Vec<IntermediateUsage> = intermediates
        .into_iter()
        .map(|(asset, count)| IntermediateUsage {
            asset: asset.to_string(),
            count,
        })
        .collect();
    top.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.asset.cmp(&b.asset)));
    top.truncate(10);

    let total = routes.len() as u64;
    RouteStats {
        corridor_key: corridor_key.to_string(),
        total_routes: total,
        successful_routes: ok,
        hop_count_distribution: hops,
        top_intermediates: top,
        avg_slippage_vs_quoted: (slippage_n > 0).then(|| slippage_sum / slippage_n as f64),
        success_rate: (total > 0).then(|| ok as f64 / total as f64),
        success_rate_lower_bound: wilson_lower_bound(ok as usize, total as usize),
    }
}

/// Minimum routes before historical routes influence preflight.
pub const MIN_HISTORICAL_ROUTES: u64 = 20;

/// Preflight status suggested by historical routes: `Hold` below 50%
/// (Wilson lower bound), `Caution` below 80%, otherwise `Proceed`. Returns
/// `Proceed` when there are too few samples to judge.
#[must_use]
pub fn historical_route_status(stats: &RouteStats) -> PreflightStatus {
    if stats.total_routes < MIN_HISTORICAL_ROUTES {
        return PreflightStatus::Proceed;
    }
    match stats.success_rate_lower_bound {
        Some(lb) if lb < 0.5 => PreflightStatus::Hold,
        Some(lb) if lb < 0.8 => PreflightStatus::Caution,
        _ => PreflightStatus::Proceed,
    }
}

/// Apply historical routes to a baseline preflight status. When `enabled`
/// is false (flag off) the baseline is returned unchanged. Never upgrades.
#[must_use]
pub fn historical_route_adjustment(
    baseline: PreflightStatus,
    stats: &RouteStats,
    enabled: bool,
) -> PreflightStatus {
    if !enabled {
        return baseline;
    }
    baseline.max(historical_route_status(stats))
}

/// A/B harness row: old vs new scoring for one recorded preflight.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoringComparison {
    pub corridor_key: String,
    pub old_status: PreflightStatus,
    pub new_status: PreflightStatus,
}

/// Compare old (baseline) vs new (historical-routes) scoring on recorded
/// `(corridor_key, baseline_status)` samples.
#[must_use]
pub fn compare_scoring(
    samples: &[(String, PreflightStatus)],
    stats_by_corridor: &HashMap<String, RouteStats>,
) -> Vec<ScoringComparison> {
    samples
        .iter()
        .map(|(key, baseline)| {
            let new_status = stats_by_corridor
                .get(key)
                .map_or(*baseline, |s| historical_route_adjustment(*baseline, s, true));
            ScoringComparison {
                corridor_key: key.clone(),
                old_status: *baseline,
                new_status,
            }
        })
        .collect()
}

// ── Persistence ──────────────────────────────────────────────────────────────

/// Persist an executed route. Idempotent on `operation_id`. Returns `true`
/// when a new row was written.
pub async fn record_route(pool: &SqlitePool, route: &ExecutedRoute) -> Result<bool> {
    let res = sqlx::query(
        "INSERT INTO executed_routes (operation_id, transaction_hash, ledger, operation_type, \
         corridor_key, source_asset, destination_asset, path, hop_count, source_amount, \
         destination_amount, quoted_amount, hop_amounts, successful) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(operation_id) DO NOTHING",
    )
    .bind(&route.operation_id)
    .bind(&route.transaction_hash)
    .bind(route.ledger.map(|l| l as i64))
    .bind(&route.operation_type)
    .bind(route.corridor_key())
    .bind(&route.source_asset)
    .bind(&route.destination_asset)
    .bind(serde_json::to_string(&route.path)?)
    .bind(route.hop_count() as i64)
    .bind(route.source_amount)
    .bind(route.destination_amount)
    .bind(route.quoted_amount)
    .bind(serde_json::to_string(&route.hop_amounts)?)
    .bind(route.successful)
    .execute(pool)
    .await
    .context("Failed to record executed route")?;
    Ok(res.rows_affected() > 0)
}

type RouteRow = (
    String,
    String,
    Option<i64>,
    String,
    String,
    String,
    String,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    String,
    bool,
);

/// Load the most recent `limit` routes for a corridor.
pub async fn load_routes(
    pool: &SqlitePool,
    corridor_key: &str,
    limit: i64,
) -> Result<Vec<ExecutedRoute>> {
    let rows: Vec<RouteRow> = sqlx::query_as(
        "SELECT operation_id, transaction_hash, ledger, operation_type, source_asset, \
         destination_asset, path, source_amount, destination_amount, quoted_amount, \
         hop_amounts, successful \
         FROM executed_routes WHERE corridor_key = ? ORDER BY id DESC LIMIT ?",
    )
    .bind(corridor_key)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("Failed to load executed routes")?;

    Ok(rows
        .into_iter()
        .map(|r| ExecutedRoute {
            operation_id: r.0,
            transaction_hash: r.1,
            ledger: r.2.and_then(|l| u64::try_from(l).ok()),
            operation_type: r.3,
            source_asset: r.4,
            destination_asset: r.5,
            path: serde_json::from_str(&r.6).unwrap_or_default(),
            source_amount: r.7,
            destination_amount: r.8,
            quoted_amount: r.9,
            hop_amounts: serde_json::from_str(&r.10).unwrap_or_default(),
            successful: r.11,
        })
        .collect())
}

/// Route stats for a corridor over its most recent 5 000 routes.
pub async fn corridor_route_stats(pool: &SqlitePool, corridor_key: &str) -> Result<RouteStats> {
    let routes = load_routes(pool, corridor_key, 5_000).await?;
    Ok(aggregate_route_stats(corridor_key, &routes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const USDC: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

    fn op(id: &str, path: Value, successful: bool) -> Value {
        json!({
            "id": id,
            "transaction_hash": format!("tx-{id}"),
            "type": "path_payment_strict_send",
            "source_asset_type": "credit_alphanum4",
            "source_asset_code": "USDC",
            "source_asset_issuer": USDC,
            "source_amount": "100.0000000",
            "asset_type": "credit_alphanum4",
            "asset_code": "NGN",
            "asset_issuer": "GNGN",
            "amount": "150000.0000000",
            "destination_min": "148500.0000000",
            "path": path,
            "transaction_successful": successful,
        })
    }

    #[test]
    fn decodes_one_hop_route() {
        let r = ExecutedRoute::from_horizon_operation(&op("1", json!([]), true)).unwrap();
        assert_eq!(r.hop_count(), 1);
        assert!(r.path.is_empty());
        assert_eq!(r.source_asset, format!("USDC:{USDC}"));
        assert_eq!(r.destination_asset, "NGN:GNGN");
        assert_eq!(r.corridor_key(), format!("USDC:{USDC}->NGN:GNGN"));
    }

    #[test]
    fn decodes_two_hop_route_via_native() {
        let r = ExecutedRoute::from_horizon_operation(&op(
            "2",
            json!([{ "asset_type": "native" }]),
            true,
        ))
        .unwrap();
        assert_eq!(r.hop_count(), 2);
        assert_eq!(r.path, vec!["native".to_string()]);
    }

    #[test]
    fn decodes_three_hop_route_with_effects() {
        let r = ExecutedRoute::from_horizon_operation(&op(
            "3",
            json!([
                { "asset_type": "native" },
                { "asset_type": "credit_alphanum4", "asset_code": "EURC", "asset_issuer": "GEUR" }
            ]),
            true,
        ))
        .unwrap()
        .with_effects(&[
            json!({ "type": "account_debited" }),
            json!({ "type": "trade", "bought_amount": "900.0" }),
            json!({ "type": "trade", "bought_amount": "92.5" }),
            json!({ "type": "trade", "bought_amount": "150000.0" }),
        ]);
        assert_eq!(r.hop_count(), 3);
        assert_eq!(r.path, vec!["native".to_string(), "EURC:GEUR".to_string()]);
        assert_eq!(r.hop_amounts, vec![900.0, 92.5, 150_000.0]);
    }

    #[test]
    fn ignores_non_path_payments_and_reads_strict_receive_quote() {
        assert!(ExecutedRoute::from_horizon_operation(&json!({"id": "x", "type": "payment"})).is_none());

        let mut v = op("4", json!([]), true);
        v["type"] = json!("path_payment_strict_receive");
        v["source_max"] = json!("110.0");
        let r = ExecutedRoute::from_horizon_operation(&v).unwrap();
        assert_eq!(r.quoted_amount, Some(110.0));
        let s = r.slippage_vs_quoted().unwrap();
        assert!((s - (10.0 / 110.0)).abs() < 1e-9);
    }

    #[test]
    fn aggregates_hop_distribution_and_intermediates() {
        let routes = vec![
            ExecutedRoute::from_horizon_operation(&op("1", json!([]), true)).unwrap(),
            ExecutedRoute::from_horizon_operation(&op("2", json!([{ "asset_type": "native" }]), true)).unwrap(),
            ExecutedRoute::from_horizon_operation(&op("3", json!([{ "asset_type": "native" }]), false)).unwrap(),
        ];
        let stats = aggregate_route_stats("k", &routes);
        assert_eq!(stats.total_routes, 3);
        assert_eq!(stats.successful_routes, 2);
        assert_eq!(stats.hop_count_distribution.get(&1), Some(&1));
        assert_eq!(stats.hop_count_distribution.get(&2), Some(&2));
        assert_eq!(stats.top_intermediates[0].asset, "native");
        assert_eq!(stats.top_intermediates[0].count, 2);
        let slip = stats.avg_slippage_vs_quoted.unwrap();
        assert!((slip - (1500.0 / 148_500.0)).abs() < 1e-9);
    }

    #[test]
    fn empty_stats_are_neutral() {
        let stats = aggregate_route_stats("k", &[]);
        assert_eq!(stats.success_rate, None);
        assert_eq!(historical_route_status(&stats), PreflightStatus::Proceed);
    }

    fn stats(ok: u64, total: u64) -> RouteStats {
        RouteStats {
            corridor_key: "k".into(),
            total_routes: total,
            successful_routes: ok,
            success_rate: Some(ok as f64 / total as f64),
            success_rate_lower_bound: wilson_lower_bound(ok as usize, total as usize),
            ..Default::default()
        }
    }

    #[test]
    fn flag_off_leaves_preflight_unchanged() {
        let bad = stats(5, 100);
        assert_eq!(
            historical_route_adjustment(PreflightStatus::Proceed, &bad, false),
            PreflightStatus::Proceed
        );
        assert_eq!(
            historical_route_adjustment(PreflightStatus::Proceed, &bad, true),
            PreflightStatus::Hold
        );
        // Never upgrades a worse baseline.
        assert_eq!(
            historical_route_adjustment(PreflightStatus::Hold, &stats(100, 100), true),
            PreflightStatus::Hold
        );
    }

    #[test]
    fn ab_harness_compares_old_and_new_scoring() {
        let mut by = HashMap::new();
        by.insert("bad".to_string(), stats(30, 100));
        by.insert("good".to_string(), stats(99, 100));
        let samples = vec![
            ("bad".to_string(), PreflightStatus::Proceed),
            ("good".to_string(), PreflightStatus::Proceed),
            ("unknown".to_string(), PreflightStatus::Caution),
        ];
        let out = compare_scoring(&samples, &by);
        assert_eq!(out[0].new_status, PreflightStatus::Hold);
        assert_eq!(out[1].new_status, PreflightStatus::Proceed);
        assert_eq!(out[2].new_status, PreflightStatus::Caution);
        let changed = out.iter().filter(|c| c.old_status != c.new_status).count();
        assert_eq!(changed, 1);
    }

    #[tokio::test]
    async fn persists_routes_idempotently_and_aggregates() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(include_str!("../../migrations/054_create_executed_routes.sql"))
            .execute(&pool)
            .await
            .unwrap();
        let r = ExecutedRoute::from_horizon_operation(&op("9", json!([{ "asset_type": "native" }]), true))
            .unwrap();
        assert!(record_route(&pool, &r).await.unwrap());
        assert!(!record_route(&pool, &r).await.unwrap());
        let loaded = load_routes(&pool, &r.corridor_key(), 10).await.unwrap();
        assert_eq!(loaded, vec![r.clone()]);
        let s = corridor_route_stats(&pool, &r.corridor_key()).await.unwrap();
        assert_eq!(s.total_routes, 1);
        assert_eq!(s.hop_count_distribution.get(&2), Some(&1));
    }
}
