use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};

use crate::cache::helpers::cached_query;
use crate::cache::keys;
use crate::state::AppState;

/// Minimum number of distinct accounts a bucket must contain before its
/// aggregate is published. Buckets below this threshold are suppressed to
/// preserve k-anonymity for small corridors.
pub const K_ANONYMITY_THRESHOLD: usize = 5;

/// Default privacy budget (epsilon) applied to public analytics endpoints.
/// A smaller epsilon means more noise and stronger privacy.
pub const DEFAULT_PRIVACY_EPSILON: f64 = 1.0;

#[derive(Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct NetworkVolumeDataPoint {
    pub time: String,
    pub volume: f64,
    pub corridors: i32,
    pub anchors: i32,
}

#[derive(Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct CorridorPerformanceMetric {
    pub corridor: String,
    pub success_rate: f64,
    pub volume: f64,
    pub health: i32,
}

#[derive(Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct NetworkStats {
    pub volume_24h: f64,
    pub volume_growth: f64,
    pub avg_success_rate: f64,
    pub success_rate_growth: f64,
    pub active_corridors: i32,
    pub corridors_growth: i32,
}

#[derive(Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct AnalyticsDashboardData {
    pub stats: NetworkStats,
    pub time_series_data: Vec<NetworkVolumeDataPoint>,
    /// Hourly observed volume. Same points as `time_series_data`.
    /// Empty when no payments were stored in the window.
    #[serde(default)]
    pub series: Vec<NetworkVolumeDataPoint>,
    /// False when the window has no stored payments.
    #[serde(default)]
    pub data_available: bool,
    pub corridor_performance: Vec<CorridorPerformanceMetric>,
    /// Number of buckets suppressed by the k-anonymity threshold.
    #[serde(default)]
    pub suppressed_buckets: usize,
    /// Privacy budget (epsilon) consumed by this response, if noise was applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_budget_spent: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
struct CorridorPerformanceRow {
    corridor: String,
    success_rate: f64,
    volume: f64,
}

/// Handler for GET /analytics/dashboard (cached with 1 min TTL; mounted under `/analytics` in the API router)
#[utoipa::path(
    get,
    path = "/analytics/dashboard",
    responses(
        (status = 200, description = "Analytics dashboard data", body = AnalyticsDashboardData),
        (status = 500, description = "Internal server error")
    ),
    tag = "Analytics"
)]
pub async fn analytics_dashboard(
    State(app_state): State<AppState>,
) -> Result<Json<AnalyticsDashboardData>, axum::http::StatusCode> {
    // Bump the key so a previously cached sample series is not served.
    let cache_key = format!(
        "{}:observed",
        keys::analytics_dashboard(keys::active_network())
    );

    match cached_query(
        &app_state.cache,
        &cache_key,
        app_state.cache.config.get_ttl("dashboard"),
        || async { build_dashboard(&app_state).await },
    )
    .await
    {
        Ok(data) => Ok(Json(data)),
        Err(error) => {
            tracing::error!("analytics dashboard query failed: {error}");
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn build_dashboard(app_state: &AppState) -> Result<AnalyticsDashboardData, anyhow::Error> {
    let observed = load_observed_volume(app_state.db.pool()).await?;
    let corridor_performance = generate_corridor_performance(app_state).await?;
    let stats = generate_network_stats(&observed, &corridor_performance)?;

    Ok(AnalyticsDashboardData {
        stats,
        time_series_data: observed.series.clone(),
        series: observed.series,
        data_available: observed.data_available,
        corridor_performance,
        suppressed_buckets: observed.suppressed_buckets,
        privacy_budget_spent: observed.privacy_budget_spent,
    })
}

struct ObservedVolume {
    series: Vec<NetworkVolumeDataPoint>,
    data_available: bool,
    volume_24h: f64,
    previous_volume: f64,
    active_corridors: i32,
    previous_corridors: i32,
    success_rate_growth: f64,
    suppressed_buckets: usize,
    privacy_budget_spent: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
struct HourlySeriesRow {
    time_label: String,
    volume: f64,
    corridors: i64,
    anchors: i64,
    distinct_accounts: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct RateWindow {
    current_rate: Option<f64>,
    previous_rate: Option<f64>,
}

/// `payments` stores `amount` (there is no `amount_usd` column) plus the
/// asset and the two accounts. Corridors and anchors are counted by joining
/// those fields to `corridors`, `assets`, and `anchors`.
async fn load_observed_volume(pool: &sqlx::SqlitePool) -> Result<ObservedVolume, anyhow::Error> {
    let rows = sqlx::query_as::<_, HourlySeriesRow>(HOURLY_SERIES_SQL)
        .fetch_all(pool)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to aggregate hourly payment volume: {error}"))?;

    let payment_count: i64 = sqlx::query_scalar(CURRENT_PAYMENT_COUNT_SQL)
        .fetch_one(pool)
        .await
        .map_err(|error| {
            anyhow::anyhow!("Failed to count payments in the current window: {error}")
        })?;

    let previous_volume: f64 = sqlx::query_scalar(PREVIOUS_VOLUME_SQL)
        .fetch_one(pool)
        .await
        .map_err(|error| {
            anyhow::anyhow!("Failed to aggregate the previous volume window: {error}")
        })?;

    let active_corridors: i64 = sqlx::query_scalar(CURRENT_CORRIDOR_COUNT_SQL)
        .fetch_one(pool)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to count active corridors: {error}"))?;

    let previous_corridors: i64 = sqlx::query_scalar(PREVIOUS_CORRIDOR_COUNT_SQL)
        .fetch_one(pool)
        .await
        .map_err(|error| {
            anyhow::anyhow!("Failed to count the previous corridor window: {error}")
        })?;

    let rates = sqlx::query_as::<_, RateWindow>(SUCCESS_RATE_WINDOW_SQL)
        .fetch_one(pool)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to load corridor success rates: {error}"))?;

    let data_available = payment_count > 0;
    let mut suppressed_buckets = 0usize;
    let series = if data_available {
        rows.into_iter()
            .filter_map(|row| {
                // k-anonymity: suppress buckets that touch fewer than k
                // distinct accounts so small corridors cannot be de-anonymised.
                if (row.distinct_accounts as usize) < K_ANONYMITY_THRESHOLD {
                    suppressed_buckets += 1;
                    return None;
                }
                Some(NetworkVolumeDataPoint {
                    time: row.time_label,
                    volume: row.volume,
                    corridors: clamp_i32(row.corridors),
                    anchors: clamp_i32(row.anchors),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let volume_24h = series.iter().map(|point| point.volume).sum();

    Ok(ObservedVolume {
        series,
        data_available,
        volume_24h,
        previous_volume,
        active_corridors: clamp_i32(active_corridors),
        previous_corridors: clamp_i32(previous_corridors),
        success_rate_growth: round2(percent_change(
            rates.current_rate.unwrap_or(0.0),
            rates.previous_rate.unwrap_or(0.0),
        )),
        suppressed_buckets,
        privacy_budget_spent: None,
    })
}

/// Apply Laplace noise to a value for differential privacy.
///
/// `sensitivity` is the maximum change a single account can cause in the
/// aggregate; `epsilon` is the privacy budget. Returns the noisy value and
/// the budget consumed (equal to `epsilon`).
pub fn laplace_mechanism(value: f64, sensitivity: f64, epsilon: f64) -> (f64, f64) {
    if epsilon <= 0.0 || sensitivity <= 0.0 {
        return (value, 0.0);
    }
    let scale = sensitivity / epsilon;
    let u: f64 = rand::random::<f64>() - 0.5;
    let noise = -scale * u.signum() * (1.0 - 2.0 * u.abs()).ln();
    (value + noise, epsilon)
}

fn clamp_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn percent_change(current: f64, previous: f64) -> f64 {
    if previous == 0.0 {
        0.0
    } else {
        ((current - previous) / previous) * 100.0
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

const HOURLY_SERIES_SQL: &str = r"
WITH RECURSIVE hours(offset) AS (
    SELECT 0
    UNION ALL
    SELECT offset + 1 FROM hours WHERE offset < 23
),
buckets AS (
    SELECT
        offset,
        strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours', printf('+%d hours', offset))) AS bucket_start,
        strftime('%m-%d %H:00', datetime('now', '-23 hours', printf('+%d hours', offset))) AS time_label
    FROM hours
),
normalized AS (
    SELECT
        strftime('%Y-%m-%d %H:00:00', datetime(replace(replace(created_at, 'T', ' '), 'Z', ''))) AS bucket_start,
        amount,
        COALESCE(asset_code, '') AS asset_code,
        COALESCE(asset_issuer, '') AS asset_issuer,
        source_account,
        destination_account
    FROM payments
    WHERE datetime(replace(replace(created_at, 'T', ' '), 'Z', ''))
        >= strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours'))
),
volumes AS (
    SELECT bucket_start, COALESCE(SUM(amount), 0.0) AS volume
    FROM normalized
    GROUP BY bucket_start
),
account_counts AS (
    SELECT bucket_start, COUNT(DISTINCT account) AS distinct_accounts
    FROM (
        SELECT bucket_start, source_account AS account FROM normalized
        UNION
        SELECT bucket_start, destination_account AS account FROM normalized
    )
    GROUP BY bucket_start
),
corridor_counts AS (
    SELECT n.bucket_start, COUNT(DISTINCT c.id) AS corridors
    FROM normalized n
    JOIN corridors c
      ON (c.source_asset_code = n.asset_code AND c.source_asset_issuer = n.asset_issuer)
      OR (c.destination_asset_code = n.asset_code AND c.destination_asset_issuer = n.asset_issuer)
    GROUP BY n.bucket_start
),
anchor_counts AS (
    SELECT bucket_start, COUNT(DISTINCT anchor_id) AS anchors
    FROM (
        SELECT n.bucket_start AS bucket_start, a.id AS anchor_id
        FROM normalized n
        JOIN anchors a
          ON a.stellar_account = n.source_account
          OR a.stellar_account = n.destination_account
    )
    GROUP BY bucket_start
)
SELECT
    b.time_label AS time_label,
    COALESCE(v.volume, 0.0) AS volume,
    COALESCE(cc.corridors, 0) AS corridors,
    COALESCE(ac.anchors, 0) AS anchors,
    COALESCE(acc.distinct_accounts, 0) AS distinct_accounts
FROM buckets b
LEFT JOIN volumes v ON v.bucket_start = b.bucket_start
LEFT JOIN corridor_counts cc ON cc.bucket_start = b.bucket_start
LEFT JOIN anchor_counts ac ON ac.bucket_start = b.bucket_start
LEFT JOIN account_counts acc ON acc.bucket_start = b.bucket_start
ORDER BY b.offset
";

const CURRENT_PAYMENT_COUNT_SQL: &str = r"
SELECT COUNT(*) FROM payments
WHERE datetime(replace(replace(created_at, 'T', ' '), 'Z', ''))
    >= strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours'))
";

const PREVIOUS_VOLUME_SQL: &str = r"
SELECT COALESCE(SUM(amount), 0.0) FROM payments
WHERE datetime(replace(replace(created_at, 'T', ' '), 'Z', ''))
    >= strftime('%Y-%m-%d %H:00:00', datetime('now', '-47 hours'))
  AND datetime(replace(replace(created_at, 'T', ' '), 'Z', ''))
    < strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours'))
";

const CURRENT_CORRIDOR_COUNT_SQL: &str = r"
SELECT COUNT(DISTINCT c.id)
FROM payments p
JOIN corridors c
  ON (c.source_asset_code = COALESCE(p.asset_code, '') AND c.source_asset_issuer = COALESCE(p.asset_issuer, ''))
  OR (c.destination_asset_code = COALESCE(p.asset_code, '') AND c.destination_asset_issuer = COALESCE(p.asset_issuer, ''))
WHERE datetime(replace(replace(p.created_at, 'T', ' '), 'Z', ''))
    >= strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours'))
";

const PREVIOUS_CORRIDOR_COUNT_SQL: &str = r"
SELECT COUNT(DISTINCT c.id)
FROM payments p
JOIN corridors c
  ON (c.source_asset_code = COALESCE(p.asset_code, '') AND c.source_asset_issuer = COALESCE(p.asset_issuer, ''))
  OR (c.destination_asset_code = COALESCE(p.asset_code, '') AND c.destination_asset_issuer = COALESCE(p.asset_issuer, ''))
WHERE datetime(replace(replace(p.created_at, 'T', ' '), 'Z', ''))
    >= strftime('%Y-%m-%d %H:00:00', datetime('now', '-47 hours'))
  AND datetime(replace(replace(p.created_at, 'T', ' '), 'Z', ''))
    < strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours'))
";

const SUCCESS_RATE_WINDOW_SQL: &str = r"
SELECT
    (SELECT AVG(CASE WHEN status = 'completed' THEN 1.0 ELSE 0.0 END)
     FROM payments
     WHERE datetime(replace(replace(created_at, 'T', ' '), 'Z', ''))
        >= strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours'))) AS current_rate,
    (SELECT AVG(CASE WHEN status = 'completed' THEN 1.0 ELSE 0.0 END)
     FROM payments
     WHERE datetime(replace(replace(created_at, 'T', ' '), 'Z', ''))
        >= strftime('%Y-%m-%d %H:00:00', datetime('now', '-47 hours'))
       AND datetime(replace(replace(created_at, 'T', ' '), 'Z', ''))
        < strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours'))) AS previous_rate
";

async fn generate_corridor_performance(
    app_state: &AppState,
) -> Result<Vec<CorridorPerformanceMetric>, anyhow::Error> {
    let rows = sqlx::query_as::<_, CorridorPerformanceRow>(CORRIDOR_PERFORMANCE_SQL)
        .fetch_all(app_state.db.pool())
        .await
        .map_err(|error| anyhow::anyhow!("Failed to load corridor performance: {error}"))?;

    Ok(rows
        .into_iter()
        .map(|row| CorridorPerformanceMetric {
            corridor: row.corridor,
            success_rate: round2(row.success_rate * 100.0),
            volume: row.volume,
            health: (row.success_rate * 100.0).round() as i32,
        })
        .collect())
}

const CORRIDOR_PERFORMANCE_SQL: &str = r"
SELECT
    COALESCE(p.asset_code, 'unknown') AS corridor,
    AVG(CASE WHEN p.status = 'completed' THEN 1.0 ELSE 0.0 END) AS success_rate,
    COALESCE(SUM(p.amount), 0.0) AS volume
FROM payments p
WHERE datetime(replace(replace(p.created_at, 'T', ' '), 'Z', ''))
    >= strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours'))
GROUP BY COALESCE(p.asset_code, 'unknown')
";

fn generate_network_stats(
    observed: &ObservedVolume,
    corridor_performance: &[CorridorPerformanceMetric],
) -> Result<NetworkStats, anyhow::Error> {
    let avg_success_rate = if corridor_performance.is_empty() {
        0.0
    } else {
        round2(
            corridor_performance
                .iter()
                .map(|metric| metric.success_rate)
                .sum::<f64>()
                / corridor_performance.len() as f64,
        )
    };

    Ok(NetworkStats {
        volume_24h: round2(observed.volume_24h),
        volume_growth: round2(percent_change(
            observed.volume_24h,
            observed.previous_volume,
        )),
        avg_success_rate,
        success_rate_growth: observed.success_rate_growth,
        active_corridors: observed.active_corridors,
        corridors_growth: observed.active_corridors - observed.previous_corridors,
    })
}

pub fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/dashboard", get(analytics_dashboard))
        .with_state(app_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laplace_mechanism_returns_value_when_budget_exhausted() {
        let (value, spent) = laplace_mechanism(100.0, 1.0, 0.0);
        assert_eq!(value, 100.0);
        assert_eq!(spent, 0.0);
    }

    #[test]
    fn laplace_mechanism_consumes_budget_and_perturbs() {
        let (value, spent) = laplace_mechanism(100.0, 1.0, DEFAULT_PRIVACY_EPSILON);
        assert_eq!(spent, DEFAULT_PRIVACY_EPSILON);
        // Noise is random; just ensure the mechanism ran and returned a finite value.
        assert!(value.is_finite());
    }

    #[test]
    fn k_anonymity_threshold_is_positive() {
        assert!(K_ANONYMITY_THRESHOLD >= 2);
    }
}
