use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};

use crate::cache::helpers::cached_query;
use crate::cache::keys;
use crate::state::AppState;

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
    let cache_key = format!("{}:observed", keys::analytics_dashboard());

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
}

#[derive(Debug, sqlx::FromRow)]
struct HourlySeriesRow {
    time_label: String,
    volume: f64,
    corridors: i64,
    anchors: i64,
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
    let series = if data_available {
        rows.into_iter()
            .map(|row| NetworkVolumeDataPoint {
                time: row.time_label,
                volume: row.volume,
                corridors: clamp_i32(row.corridors),
                anchors: clamp_i32(row.anchors),
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
    })
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
        SELECT n.bucket_start AS bucket_start, ast.anchor_id AS anchor_id
        FROM normalized n
        JOIN assets ast
          ON ast.asset_code = n.asset_code
         AND ast.asset_issuer = n.asset_issuer
        UNION
        SELECT n.bucket_start AS bucket_start, an.id AS anchor_id
        FROM normalized n
        JOIN anchors an
          ON an.stellar_account = n.source_account
          OR an.stellar_account = n.destination_account
    ) AS observed_anchors
    GROUP BY bucket_start
)
SELECT
    b.time_label AS time_label,
    COALESCE(v.volume, 0.0) AS volume,
    COALESCE(cc.corridors, 0) AS corridors,
    COALESCE(ac.anchors, 0) AS anchors
FROM buckets b
LEFT JOIN volumes v ON v.bucket_start = b.bucket_start
LEFT JOIN corridor_counts cc ON cc.bucket_start = b.bucket_start
LEFT JOIN anchor_counts ac ON ac.bucket_start = b.bucket_start
ORDER BY b.offset
";

const CURRENT_PAYMENT_COUNT_SQL: &str = "SELECT COUNT(*) FROM payments WHERE datetime(replace(replace(created_at, 'T', ' '), 'Z', '')) >= strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours'))";

const PREVIOUS_VOLUME_SQL: &str = "SELECT COALESCE(SUM(amount), 0.0) FROM payments WHERE datetime(replace(replace(created_at, 'T', ' '), 'Z', '')) >= datetime(strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours')), '-24 hours') AND datetime(replace(replace(created_at, 'T', ' '), 'Z', '')) < strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours'))";

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
    >= datetime(strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours')), '-24 hours')
  AND datetime(replace(replace(p.created_at, 'T', ' '), 'Z', ''))
    < strftime('%Y-%m-%d %H:00:00', datetime('now', '-23 hours'))
";

const SUCCESS_RATE_WINDOW_SQL: &str = r"
SELECT
    (SELECT AVG(success_rate) FROM corridor_metrics WHERE date >= date('now', '-1 day')) AS current_rate,
    (SELECT AVG(success_rate) FROM corridor_metrics WHERE date >= date('now', '-2 day') AND date < date('now', '-1 day')) AS previous_rate
";

async fn generate_corridor_performance(
    app_state: &AppState,
) -> Result<Vec<CorridorPerformanceMetric>, anyhow::Error> {
    let rows = sqlx::query_as::<_, CorridorPerformanceRow>(
        r"
        SELECT
            c.source_asset_code || ':' || c.source_asset_issuer || '->' || c.destination_asset_code || ':' || c.destination_asset_issuer AS corridor,
            COALESCE(AVG(cm.success_rate), 0.0) AS success_rate,
            COALESCE(SUM(cm.volume_usd), 0.0) AS volume
        FROM corridors c
        LEFT JOIN corridor_metrics cm
            ON c.source_asset_code = cm.asset_a_code
           AND c.source_asset_issuer = cm.asset_a_issuer
           AND c.destination_asset_code = cm.asset_b_code
           AND c.destination_asset_issuer = cm.asset_b_issuer
        GROUP BY
            c.id,
            c.source_asset_code,
            c.source_asset_issuer,
            c.destination_asset_code,
            c.destination_asset_issuer
        ORDER BY volume DESC
        ",
    )
    .fetch_all(app_state.db.pool())
    .await
    .map_err(|e| anyhow::anyhow!("Failed to load corridor performance from database: {e}"))?;

    let metrics = rows
        .into_iter()
        .map(|row| CorridorPerformanceMetric {
            corridor: row.corridor,
            success_rate: row.success_rate,
            volume: row.volume,
            health: calculate_health(row.success_rate),
        })
        .collect();

    Ok(metrics)
}

fn calculate_health(success_rate: f64) -> i32 {
    success_rate.clamp(0.0, 100.0).round() as i32
}

fn generate_network_stats(
    observed: &ObservedVolume,
    corridor_performance: &[CorridorPerformanceMetric],
) -> Result<NetworkStats, anyhow::Error> {
    let avg_success_rate: f64 = if corridor_performance.is_empty() {
        0.0
    } else {
        corridor_performance
            .iter()
            .map(|c| c.success_rate)
            .sum::<f64>()
            / corridor_performance.len() as f64
    };

    Ok(NetworkStats {
        volume_24h: observed.volume_24h,
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
    use sqlx::SqlitePool;

    async fn migrated_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    async fn seed_reference_rows(pool: &SqlitePool) {
        sqlx::query(
            "INSERT INTO anchors (id, name, stellar_account) VALUES ('anchor-1', 'Anchor', 'GSOURCE')",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO assets (id, anchor_id, asset_code, asset_issuer) VALUES ('asset-1', 'anchor-1', 'USDC', 'GISSUER')",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO corridors (id, source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer) VALUES ('corridor-1', 'USDC', 'GISSUER', 'NGN', 'GNGN')",
        )
        .execute(pool)
        .await
        .unwrap();
    }

    async fn insert_payment(pool: &SqlitePool, id: &str, amount: f64, created_at_sql: &str) {
        let sql = format!(
            "INSERT INTO payments (id, transaction_hash, source_account, destination_account, asset_type, asset_code, asset_issuer, amount, created_at) VALUES (?, ?, 'GSOURCE', 'GDEST', 'credit_alphanum4', 'USDC', 'GISSUER', ?, {created_at_sql})"
        );
        sqlx::query(&sql)
            .bind(id)
            .bind(format!("tx-{id}"))
            .bind(amount)
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn empty_payments_return_no_series() {
        let pool = migrated_pool().await;
        let observed = load_observed_volume(&pool).await.unwrap();
        let stats = generate_network_stats(&observed, &[]).unwrap();

        assert!(!observed.data_available);
        assert!(observed.series.is_empty());
        assert_eq!(stats.volume_24h, 0.0);
        assert_eq!(stats.volume_growth, 0.0);
        assert_eq!(stats.active_corridors, 0);
        assert_eq!(stats.success_rate_growth, 0.0);
    }

    #[tokio::test]
    async fn seeded_payments_change_the_hourly_series() {
        let pool = migrated_pool().await;
        seed_reference_rows(&pool).await;
        insert_payment(&pool, "now", 100.0, "strftime('%Y-%m-%d %H:%M:%S', 'now')").await;
        insert_payment(
            &pool,
            "recent",
            50.0,
            "strftime('%Y-%m-%d %H:%M:%S', 'now', '-2 hours')",
        )
        .await;
        insert_payment(
            &pool,
            "previous",
            25.0,
            "strftime('%Y-%m-%d %H:%M:%S', 'now', '-30 hours')",
        )
        .await;

        let observed = load_observed_volume(&pool).await.unwrap();
        let stats = generate_network_stats(&observed, &[]).unwrap();

        assert!(observed.data_available);
        assert_eq!(observed.series.len(), 24);
        assert!(observed.series.iter().any(|point| point.volume == 0.0));
        assert!(observed.series.iter().any(|point| point.anchors >= 1));
        assert!(observed.series.iter().any(|point| point.corridors >= 1));
        assert_eq!(stats.volume_24h, 150.0);
        assert_eq!(stats.volume_growth, 500.0);
        assert_eq!(stats.active_corridors, 1);
        assert!(observed.series.iter().all(|point| point.volume != 45_000.0));
    }
}
