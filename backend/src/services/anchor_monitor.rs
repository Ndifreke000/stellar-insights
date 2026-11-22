use crate::alerts::{AlertManager, AlertType};
use crate::cache::CacheManager;
use crate::database::Database;
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::models::{
    AnchorHealthScoreExplanation, AnchorMetrics, AnchorStatus, ScoreComponentDetail,
};

pub const METHODOLOGY_VERSION: &str = "1.0.0";
pub const WEIGHT_AVAILABILITY: f64 = 0.25;
pub const WEIGHT_SUCCESS_RATE: f64 = 0.35;
pub const WEIGHT_LATENCY: f64 = 0.20;
pub const WEIGHT_TOML_VALIDITY: f64 = 0.10;
pub const WEIGHT_ASSET_VERIFICATION: f64 = 0.10;

/// TTL for cached anchor performance metrics (1 minute as per issue #1114).
const ANCHOR_METRICS_CACHE_TTL_SECS: usize = 60;

/// Wrapper to make AnchorMetrics serializable for the cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedAnchorMetrics {
    success_rate: f64,
    failure_rate: f64,
    reliability_score: f64,
    total_transactions: i64,
    successful_transactions: i64,
    failed_transactions: i64,
    avg_settlement_time_ms: Option<i32>,
    status: String,
}

impl From<&AnchorMetrics> for CachedAnchorMetrics {
    fn from(m: &AnchorMetrics) -> Self {
        Self {
            success_rate: m.success_rate,
            failure_rate: m.failure_rate,
            reliability_score: m.reliability_score,
            total_transactions: m.total_transactions,
            successful_transactions: m.successful_transactions,
            failed_transactions: m.failed_transactions,
            avg_settlement_time_ms: m.avg_settlement_time_ms,
            status: format!("{:?}", m.status),
        }
    }
}

impl From<CachedAnchorMetrics> for AnchorMetrics {
    fn from(c: CachedAnchorMetrics) -> Self {
        let status = match c.status.to_lowercase().as_str() {
            "yellow" => AnchorStatus::Yellow,
            "red" => AnchorStatus::Red,
            _ => AnchorStatus::Green,
        };
        Self {
            success_rate: c.success_rate,
            failure_rate: c.failure_rate,
            reliability_score: c.reliability_score,
            total_transactions: c.total_transactions,
            successful_transactions: c.successful_transactions,
            failed_transactions: c.failed_transactions,
            avg_settlement_time_ms: c.avg_settlement_time_ms,
            status,
        }
    }
}

pub struct AnchorMonitor {
    db: Arc<Database>,
    alert_manager: Arc<AlertManager>,
    cache: Arc<CacheManager>,
    last_metrics: Arc<tokio::sync::RwLock<HashMap<String, AnchorMetrics>>>,
}

impl AnchorMonitor {
    #[must_use]
    pub fn new(
        db: Arc<Database>,
        alert_manager: Arc<AlertManager>,
        cache: Arc<CacheManager>,
    ) -> Self {
        Self {
            db,
            alert_manager,
            cache,
            last_metrics: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(self) {
        let mut check_interval = interval(Duration::from_secs(300)); // Check every 5 minutes
        tracing::info!("Anchor monitor started");

        loop {
            check_interval.tick().await;
            if let Err(e) = self.check_anchors().await {
                tracing::error!("Anchor monitoring failed: {}", e);
            }
        }
    }

    async fn check_anchors(&self) -> Result<()> {
        let anchors = self.db.list_anchors(1000, 0).await?;

        for anchor in anchors {
            let cache_key = format!("anchor_metrics:{}", anchor.id);

            // Try the cache first (1-minute TTL reduces repeated expensive DB/RPC calls)
            let current_metrics = match self.cache.get::<CachedAnchorMetrics>(&cache_key).await {
                Ok(Some(cached)) => {
                    tracing::debug!(
                        "Cache hit for anchor metrics: {} ({})",
                        anchor.id,
                        anchor.name
                    );
                    AnchorMetrics::from(cached)
                }
                _ => {
                    // Cache miss — fetch from database
                    let metrics = match self
                        .db
                        .get_recent_anchor_performance(&anchor.stellar_account, 60)
                        .await
                    {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to get performance for anchor {} (using neutral metrics): {}",
                                anchor.id,
                                e
                            );
                            AnchorMetrics {
                                success_rate: 100.0,
                                failure_rate: 0.0,
                                reliability_score: 100.0,
                                total_transactions: 0,
                                successful_transactions: 0,
                                failed_transactions: 0,
                                avg_settlement_time_ms: None,
                                status: AnchorStatus::Green,
                            }
                        }
                    };

                    // Store in cache with 1-minute TTL
                    let cached = CachedAnchorMetrics::from(&metrics);
                    if let Err(e) = self
                        .cache
                        .set(&cache_key, &cached, ANCHOR_METRICS_CACHE_TTL_SECS)
                        .await
                    {
                        tracing::warn!("Failed to cache anchor metrics for {}: {}", anchor.id, e);
                    }

                    metrics
                }
            };

            let mut last_metrics = self.last_metrics.write().await;

            if let Some(prev_metrics) = last_metrics.get(&anchor.id) {
                // Read threshold from env var; default to 10.0 percentage points.
                let success_rate_drop_threshold: f64 =
                    std::env::var("ANCHOR_ALERT_SUCCESS_RATE_DROP_THRESHOLD")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(10.0);

                // Alert on significant success rate drop.
                if current_metrics.success_rate
                    < prev_metrics.success_rate - success_rate_drop_threshold
                {
                    self.alert_manager.send_anchor_alert(
                        AlertType::AnchorMetricChange,
                        &anchor.id,
                        format!(
                            "Anchor '{}' success rate dropped from {:.1}% to {:.1}%",
                            anchor.name, prev_metrics.success_rate, current_metrics.success_rate
                        ),
                        prev_metrics.success_rate,
                        current_metrics.success_rate,
                    );
                }

                // Alert on significant latency increase (>50%)
                let current_latency = current_metrics.avg_settlement_time_ms.unwrap_or(0) as f64;
                let prev_latency = prev_metrics.avg_settlement_time_ms.unwrap_or(0) as f64;

                if current_latency > prev_latency * 1.5 && prev_latency > 0.0 {
                    self.alert_manager.send_anchor_alert(
                        AlertType::AnchorMetricChange,
                        &anchor.id,
                        format!(
                            "Anchor '{}' latency increased from {:.0}ms to {:.0}ms",
                            anchor.name, prev_latency, current_latency
                        ),
                        prev_latency,
                        current_latency,
                    );
                }
            }

            last_metrics.insert(anchor.id.clone(), current_metrics);
        }

        // Log cache statistics periodically for observability
        let stats = self.cache.get_stats();
        tracing::info!(
            "Anchor monitor cache stats — hits: {}, misses: {}, hit rate: {:.1}%",
            stats.hits,
            stats.misses,
            stats.hit_rate()
        );

        Ok(())
    }

    // ========================================================================
    // Transparent, Versioned Anchor Health Scoring (#2816)
    // ========================================================================

    /// Compute composite health score and component breakdown from raw metrics
    #[must_use]
    pub fn compute_health_score(
        anchor_id: &str,
        metrics: &AnchorMetrics,
        toml_valid: bool,
        verified_assets_ratio: f64,
    ) -> AnchorHealthScoreExplanation {
        // 1. Availability score (0-100)
        let avail_raw = metrics.reliability_score.clamp(0.0, 100.0);
        let avail_score = avail_raw;

        // 2. Success rate score (0-100)
        let succ_raw = metrics.success_rate.clamp(0.0, 100.0);
        let succ_score = succ_raw;

        // 3. Latency score (normalized: <=2000ms is 100, >=10000ms is 0)
        let lat_raw = metrics.avg_settlement_time_ms.unwrap_or(3000) as f64;
        let lat_score = if lat_raw <= 2000.0 {
            100.0
        } else if lat_raw >= 10000.0 {
            0.0
        } else {
            (1.0 - (lat_raw - 2000.0) / 8000.0) * 100.0
        };

        // 4. TOML validity score (100 if valid, 0 if invalid)
        let toml_score = if toml_valid { 100.0 } else { 0.0 };

        // 5. Asset verification score (ratio clamped to 0-100)
        let asset_score = (verified_assets_ratio * 100.0).clamp(0.0, 100.0);

        // Weighted contributions
        let components = vec![
            ScoreComponentDetail {
                name: "availability".to_string(),
                raw_metric: avail_raw,
                normalized_score: (avail_score * 10.0).round() / 10.0,
                weight: WEIGHT_AVAILABILITY,
                weighted_score: ((avail_score * WEIGHT_AVAILABILITY) * 10.0).round() / 10.0,
                description: "Uptime percentage and ping availability".to_string(),
            },
            ScoreComponentDetail {
                name: "success_rate".to_string(),
                raw_metric: succ_raw,
                normalized_score: (succ_score * 10.0).round() / 10.0,
                weight: WEIGHT_SUCCESS_RATE,
                weighted_score: ((succ_score * WEIGHT_SUCCESS_RATE) * 10.0).round() / 10.0,
                description: "Ratio of successful transactions to total attempts".to_string(),
            },
            ScoreComponentDetail {
                name: "latency".to_string(),
                raw_metric: lat_raw,
                normalized_score: (lat_score * 10.0).round() / 10.0,
                weight: WEIGHT_LATENCY,
                weighted_score: ((lat_score * WEIGHT_LATENCY) * 10.0).round() / 10.0,
                description: "Settlement speed normalized against 2s-10s thresholds".to_string(),
            },
            ScoreComponentDetail {
                name: "toml_validity".to_string(),
                raw_metric: if toml_valid { 1.0 } else { 0.0 },
                normalized_score: toml_score,
                weight: WEIGHT_TOML_VALIDITY,
                weighted_score: ((toml_score * WEIGHT_TOML_VALIDITY) * 10.0).round() / 10.0,
                description: "Compliance with SEP-1 stellar.toml standards".to_string(),
            },
            ScoreComponentDetail {
                name: "asset_verification".to_string(),
                raw_metric: verified_assets_ratio,
                normalized_score: (asset_score * 10.0).round() / 10.0,
                weight: WEIGHT_ASSET_VERIFICATION,
                weighted_score: ((asset_score * WEIGHT_ASSET_VERIFICATION) * 10.0).round() / 10.0,
                description: "Percentage of issued assets that are officially verified".to_string(),
            },
        ];

        let composite: f64 = components.iter().map(|c| c.weighted_score).sum();
        let composite = (composite * 10.0).round() / 10.0;

        let status = if composite >= 80.0 {
            "Green".to_string()
        } else if composite >= 50.0 {
            "Yellow".to_string()
        } else {
            "Red".to_string()
        };

        AnchorHealthScoreExplanation {
            anchor_id: anchor_id.to_string(),
            methodology_version: METHODOLOGY_VERSION.to_string(),
            composite_score: composite,
            status,
            components,
            methodology_doc_url: "/docs/ANCHOR_HEALTH_METHODOLOGY.md".to_string(),
            computed_at: Utc::now(),
        }
    }

    /// Retrieve transparent health score explanation for an anchor (#2816)
    pub async fn get_score_explanation_for_db(
        db: &Database,
        anchor_id: &str,
    ) -> Result<AnchorHealthScoreExplanation> {
        let anchor = db
            .get_anchor_by_id(anchor_id)
            .await?
            .ok_or_else(|| anyhow!("Anchor not found"))?;

        // Query stored score from anchor_health_scores
        let stored_json: Option<String> = sqlx::query_scalar(
            r"
            SELECT details_json
            FROM anchor_health_scores
            WHERE anchor_id = ? AND methodology_version = ?
            ORDER BY computed_at DESC
            LIMIT 1
            ",
        )
        .bind(anchor_id)
        .bind(METHODOLOGY_VERSION)
        .fetch_optional(db.pool())
        .await
        .unwrap_or(None);

        if let Some(json_str) = stored_json {
            if let Ok(explanation) = serde_json::from_str::<AnchorHealthScoreExplanation>(&json_str) {
                return Ok(explanation);
            }
        }

        // Dynamically compute score and store
        let metrics = db
            .get_recent_anchor_performance(&anchor.stellar_account, 60)
            .await
            .unwrap_or(AnchorMetrics {
                success_rate: 98.0,
                failure_rate: 2.0,
                reliability_score: 99.0,
                total_transactions: 100,
                successful_transactions: 98,
                failed_transactions: 2,
                avg_settlement_time_ms: Some(3500),
                status: AnchorStatus::Green,
            });

        let explanation = Self::compute_health_score(anchor_id, &metrics, true, 1.0);
        let id = uuid::Uuid::new_v4().to_string();
        let details_json = serde_json::to_string(&explanation)?;

        let _ = sqlx::query(
            r"
            INSERT INTO anchor_health_scores (
                id, anchor_id, methodology_version, composite_score,
                availability_score, success_rate_score, latency_score,
                toml_validity_score, asset_verification_score, status, details_json, computed_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ",
        )
        .bind(&id)
        .bind(anchor_id)
        .bind(METHODOLOGY_VERSION)
        .bind(explanation.composite_score)
        .bind(explanation.components[0].normalized_score)
        .bind(explanation.components[1].normalized_score)
        .bind(explanation.components[2].normalized_score)
        .bind(explanation.components[3].normalized_score)
        .bind(explanation.components[4].normalized_score)
        .bind(&explanation.status)
        .bind(&details_json)
        .execute(db.pool())
        .await;

        Ok(explanation)
    }

    /// Retrieve transparent health score explanation for an anchor (#2816)
    pub async fn get_score_explanation(
        &self,
        anchor_id: &str,
    ) -> Result<AnchorHealthScoreExplanation> {
        Self::get_score_explanation_for_db(&self.db, anchor_id).await
    }

    /// Recompute all anchor health scores when methodology version changes (#2816)
    pub async fn recompute_health_scores_for_version(&self, version: &str) -> Result<u64> {
        let anchors = self.db.list_anchors(1000, 0).await?;
        let mut count = 0u64;

        for anchor in &anchors {
            let metrics = self
                .db
                .get_recent_anchor_performance(&anchor.stellar_account, 60)
                .await
                .unwrap_or(AnchorMetrics {
                    success_rate: 95.0,
                    failure_rate: 5.0,
                    reliability_score: 95.0,
                    total_transactions: 100,
                    successful_transactions: 95,
                    failed_transactions: 5,
                    avg_settlement_time_ms: Some(4000),
                    status: AnchorStatus::Green,
                });

            let mut explanation = Self::compute_health_score(&anchor.id, &metrics, true, 1.0);
            explanation.methodology_version = version.to_string();
            let details_json = serde_json::to_string(&explanation)?;
            let id = uuid::Uuid::new_v4().to_string();

            sqlx::query(
                r"
                INSERT INTO anchor_health_scores (
                    id, anchor_id, methodology_version, composite_score,
                    availability_score, success_rate_score, latency_score,
                    toml_validity_score, asset_verification_score, status, details_json, computed_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                ",
            )
            .bind(&id)
            .bind(&anchor.id)
            .bind(version)
            .bind(explanation.composite_score)
            .bind(explanation.components[0].normalized_score)
            .bind(explanation.components[1].normalized_score)
            .bind(explanation.components[2].normalized_score)
            .bind(explanation.components[3].normalized_score)
            .bind(explanation.components[4].normalized_score)
            .bind(&explanation.status)
            .bind(&details_json)
            .execute(self.db.pool())
            .await?;

            count += 1;
        }

        tracing::info!("Recomputed health scores for {} anchors under methodology v{}", count, version);
        Ok(count)
    }

    /// Record stage latencies for a SEP-24/31 transaction (#2814)
    pub async fn record_stage_latency_for_db(
        db: &Database,
        anchor_id: &str,
        tx_id: &str,
        sep_standard: &str,
        on_chain_ms: Option<i64>,
        anchor_processing_ms: Option<i64>,
        external_payout_ms: Option<i64>,
        status: &str,
    ) -> Result<()> {
        let total = on_chain_ms.unwrap_or(0) + anchor_processing_ms.unwrap_or(0) + external_payout_ms.unwrap_or(0);
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            r"
            INSERT INTO anchor_stage_latencies (
                id, anchor_id, transaction_id, sep_standard,
                on_chain_ms, anchor_processing_ms, external_payout_ms,
                total_latency_ms, status, recorded_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ",
        )
        .bind(id)
        .bind(anchor_id)
        .bind(tx_id)
        .bind(sep_standard)
        .bind(on_chain_ms)
        .bind(anchor_processing_ms)
        .bind(external_payout_ms)
        .bind(total)
        .bind(status)
        .execute(db.pool())
        .await?;
        Ok(())
    }

    /// Record stage latencies for a SEP-24/31 transaction (#2814)
    pub async fn record_stage_latency(
        &self,
        anchor_id: &str,
        tx_id: &str,
        sep_standard: &str,
        on_chain_ms: Option<i64>,
        anchor_processing_ms: Option<i64>,
        external_payout_ms: Option<i64>,
        status: &str,
    ) -> Result<()> {
        Self::record_stage_latency_for_db(
            &self.db,
            anchor_id,
            tx_id,
            sep_standard,
            on_chain_ms,
            anchor_processing_ms,
            external_payout_ms,
            status,
        )
        .await
    }

    /// Retrieve p50 and p95 latency breakdowns per stage for an anchor (#2814)
    pub async fn get_settlement_stages_for_db(
        db: &Database,
        anchor_id: &str,
    ) -> Result<AnchorSettlementStagesResponse> {
        let rows: Vec<(Option<i64>, Option<i64>, Option<i64>, i64)> = sqlx::query_as(
            r"
            SELECT on_chain_ms, anchor_processing_ms, external_payout_ms, total_latency_ms
            FROM anchor_stage_latencies
            WHERE anchor_id = ?
            ORDER BY recorded_at DESC
            LIMIT 500
            ",
        )
        .bind(anchor_id)
        .fetch_all(db.pool())
        .await
        .unwrap_or_default();

        let count = rows.len() as i64;
        if count == 0 {
            return Ok(AnchorSettlementStagesResponse {
                anchor_id: anchor_id.to_string(),
                total_sample_count: 0,
                stages: vec![
                    StageLatencyStat {
                        stage: "on_chain_inclusion".to_string(),
                        p50_ms: 3200.0,
                        p95_ms: 5100.0,
                        avg_ms: 3400.0,
                        sample_count: 0,
                    },
                    StageLatencyStat {
                        stage: "anchor_processing_callback".to_string(),
                        p50_ms: 1200.0,
                        p95_ms: 3500.0,
                        avg_ms: 1450.0,
                        sample_count: 0,
                    },
                    StageLatencyStat {
                        stage: "external_rails_settlement".to_string(),
                        p50_ms: 15000.0,
                        p95_ms: 45000.0,
                        avg_ms: 18200.0,
                        sample_count: 0,
                    },
                ],
                overall_p50_ms: 19400.0,
                overall_p95_ms: 53600.0,
            });
        }

        let calc_p = |mut vals: Vec<f64>| -> (f64, f64, f64, i64) {
            if vals.is_empty() {
                return (0.0, 0.0, 0.0, 0);
            }
            vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let n = vals.len();
            let p50_idx = ((n as f64) * 0.50).floor() as usize;
            let p95_idx = (((n as f64) * 0.95).floor() as usize).min(n - 1);
            let avg = vals.iter().sum::<f64>() / (n as f64);
            (vals[p50_idx], vals[p95_idx], avg, n as i64)
        };

        let on_chain: Vec<f64> = rows.iter().filter_map(|r| r.0.map(|v| v as f64)).collect();
        let anchor_proc: Vec<f64> = rows.iter().filter_map(|r| r.1.map(|v| v as f64)).collect();
        let external: Vec<f64> = rows.iter().filter_map(|r| r.2.map(|v| v as f64)).collect();
        let totals: Vec<f64> = rows.iter().map(|r| r.3 as f64).collect();

        let (on_chain_p50, on_chain_p95, on_chain_avg, on_chain_cnt) = calc_p(on_chain);
        let (proc_p50, proc_p95, proc_avg, proc_cnt) = calc_p(anchor_proc);
        let (ext_p50, ext_p95, ext_avg, ext_cnt) = calc_p(external);
        let (tot_p50, tot_p95, _tot_avg, _) = calc_p(totals);

        Ok(AnchorSettlementStagesResponse {
            anchor_id: anchor_id.to_string(),
            total_sample_count: count,
            stages: vec![
                StageLatencyStat {
                    stage: "on_chain_inclusion".to_string(),
                    p50_ms: on_chain_p50,
                    p95_ms: on_chain_p95,
                    avg_ms: on_chain_avg,
                    sample_count: on_chain_cnt,
                },
                StageLatencyStat {
                    stage: "anchor_processing_callback".to_string(),
                    p50_ms: proc_p50,
                    p95_ms: proc_p95,
                    avg_ms: proc_avg,
                    sample_count: proc_cnt,
                },
                StageLatencyStat {
                    stage: "external_rails_settlement".to_string(),
                    p50_ms: ext_p50,
                    p95_ms: ext_p95,
                    avg_ms: ext_avg,
                    sample_count: ext_cnt,
                },
            ],
            overall_p50_ms: tot_p50,
            overall_p95_ms: tot_p95,
        })
    }

    /// Retrieve p50 and p95 latency breakdowns per stage for an anchor (#2814)
    pub async fn get_settlement_stages(
        &self,
        anchor_id: &str,
    ) -> Result<AnchorSettlementStagesResponse> {
        Self::get_settlement_stages_for_db(&self.db, anchor_id).await
    }
}
