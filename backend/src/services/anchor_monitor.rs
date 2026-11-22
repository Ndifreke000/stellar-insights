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

/// A single SEP capability advertised by an anchor's `/info` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SepCapability {
    /// SEP number, e.g. `6`, `24`, `31`.
    pub sep: u32,
    /// Whether the anchor advertises this SEP in its `/info` response.
    pub enabled: bool,
    /// Deposit assets advertised for this SEP (empty for SEP-31).
    pub deposit_assets: Vec<String>,
    /// Withdraw assets advertised for this SEP (empty for SEP-31).
    pub withdraw_assets: Vec<String>,
    /// Countries supported for this SEP (SEP-31 only).
    pub countries: Vec<String>,
}

/// Normalised capability matrix for a single anchor (issue #2870).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnchorCapabilityMatrix {
    pub anchor_id: String,
    pub anchor_name: String,
    /// Per-SEP capabilities, keyed by SEP number.
    pub seps: Vec<SepCapability>,
    /// Union of all assets across SEPs.
    pub assets: Vec<String>,
    /// Union of all countries across SEPs.
    pub countries: Vec<String>,
    /// Fee information from `/fee` where available.
    pub fee: Option<AnchorFeeInfo>,
    /// When this snapshot was captured.
    pub captured_at: chrono::DateTime<Utc>,
}

/// Normalised fee information from an anchor's `/fee` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnchorFeeInfo {
    pub asset: String,
    pub fee_fixed: Option<f64>,
    pub fee_percent: Option<f64>,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
}

/// Raw shape of a SEP-6/24 `/info` response (subset we care about).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SepInfoResponse {
    #[serde(default)]
    pub deposit: HashMap<String, SepAssetInfo>,
    #[serde(default)]
    pub withdraw: HashMap<String, SepAssetInfo>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SepAssetInfo {
    #[serde(default)]
    pub enabled: bool,
}

/// Raw shape of a SEP-31 `/info` response (subset we care about).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Sep31InfoResponse {
    #[serde(default)]
    pub receive: HashMap<String, Sep31AssetInfo>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Sep31AssetInfo {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub countries: HashMap<String, Sep31CountryInfo>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Sep31CountryInfo {
    #[serde(default)]
    pub enabled: bool,
}

/// Raw shape of a `/fee` response (subset we care about).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FeeResponse {
    #[serde(default)]
    pub fee: Option<FeeDetail>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FeeDetail {
    #[serde(default)]
    pub asset: Option<String>,
    #[serde(default)]
    pub fee_fixed: Option<f64>,
    #[serde(default)]
    pub fee_percent: Option<f64>,
    #[serde(default)]
    pub min_amount: Option<f64>,
    #[serde(default)]
    pub max_amount: Option<f64>,
}

/// Normalise a SEP-6/24 `/info` response into a `SepCapability`.
#[must_use]
pub fn normalise_sep_info(sep: u32, info: &SepInfoResponse) -> SepCapability {
    let deposit_assets: Vec<String> = info
        .deposit
        .iter()
        .filter(|(_, a)| a.enabled)
        .map(|(k, _)| k.clone())
        .collect();
    let withdraw_assets: Vec<String> = info
        .withdraw
        .iter()
        .filter(|(_, a)| a.enabled)
        .map(|(k, _)| k.clone())
        .collect();
    let enabled = !deposit_assets.is_empty() || !withdraw_assets.is_empty();
    SepCapability {
        sep,
        enabled,
        deposit_assets,
        withdraw_assets,
        countries: Vec::new(),
    }
}

/// Normalise a SEP-31 `/info` response into a `SepCapability`.
#[must_use]
pub fn normalise_sep31_info(info: &Sep31InfoResponse) -> SepCapability {
    let mut assets = Vec::new();
    let mut countries = Vec::new();
    for (asset, detail) in &info.receive {
        if !detail.enabled {
            continue;
        }
        assets.push(asset.clone());
        for (country, c) in &detail.countries {
            if c.enabled && !countries.contains(country) {
                countries.push(country.clone());
            }
        }
    }
    let enabled = !assets.is_empty();
    SepCapability {
        sep: 31,
        enabled,
        deposit_assets: assets,
        withdraw_assets: Vec::new(),
        countries,
    }
}

/// Normalise a `/fee` response into an `AnchorFeeInfo`.
#[must_use]
pub fn normalise_fee_info(fee: &FeeResponse) -> Option<AnchorFeeInfo> {
    let detail = fee.fee.as_ref()?;
    Some(AnchorFeeInfo {
        asset: detail.asset.clone().unwrap_or_default(),
        fee_fixed: detail.fee_fixed,
        fee_percent: detail.fee_percent,
        min_amount: detail.min_amount,
        max_amount: detail.max_amount,
    })
}

/// Build a normalised capability matrix from per-SEP capabilities and optional fee info.
#[must_use]
pub fn build_capability_matrix(
    anchor_id: &str,
    anchor_name: &str,
    seps: Vec<SepCapability>,
    fee: Option<AnchorFeeInfo>,
) -> AnchorCapabilityMatrix {
    let mut assets: Vec<String> = Vec::new();
    let mut countries: Vec<String> = Vec::new();
    for sep in &seps {
        for a in sep.deposit_assets.iter().chain(sep.withdraw_assets.iter()) {
            if !assets.contains(a) {
                assets.push(a.clone());
            }
        }
        for c in &sep.countries {
            if !countries.contains(c) {
                countries.push(c.clone());
            }
        }
    }
    AnchorCapabilityMatrix {
        anchor_id: anchor_id.to_string(),
        anchor_name: anchor_name.to_string(),
        seps,
        assets,
        countries,
        fee,
        captured_at: Utc::now(),
    }
}

/// Diff two capability snapshots, returning a human-readable list of changes.
#[must_use]
pub fn diff_capabilities(
    prev: &AnchorCapabilityMatrix,
    curr: &AnchorCapabilityMatrix,
) -> Vec<String> {
    let mut changes = Vec::new();
    for curr_sep in &curr.seps {
        let prev_sep = prev.seps.iter().find(|s| s.sep == curr_sep.sep);
        match prev_sep {
            None if curr_sep.enabled => {
                changes.push(format!("SEP-{} newly enabled", curr_sep.sep));
            }
            Some(p) if p.enabled != curr_sep.enabled => {
                changes.push(format!(
                    "SEP-{} {}",
                    curr_sep.sep,
                    if curr_sep.enabled { "enabled" } else { "disabled" }
                ));
            }
            _ => {}
        }
    }
    for a in &curr.assets {
        if !prev.assets.contains(a) {
            changes.push(format!("asset {a} added"));
        }
    }
    for a in &prev.assets {
        if !curr.assets.contains(a) {
            changes.push(format!("asset {a} removed"));
        }
    }
    for c in &curr.countries {
        if !prev.countries.contains(c) {
            changes.push(format!("country {c} added"));
        }
    }
    for c in &prev.countries {
        if !curr.countries.contains(c) {
            changes.push(format!("country {c} removed"));
        }
    }
    changes
}

pub struct AnchorMonitor {
    db: Arc<Database>,
    alert_manager: Arc<AlertManager>,
    cache: Arc<CacheManager>,
    last_metrics: Arc<tokio::sync::RwLock<HashMap<String, AnchorMetrics>>>,
    last_capabilities: Arc<tokio::sync::RwLock<HashMap<String, AnchorCapabilityMatrix>>>,
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
            last_capabilities: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Record a freshly crawled capability matrix, tracking changes over time.
    ///
    /// Returns the list of capability changes detected since the previous snapshot.
    pub async fn record_capabilities(
        &self,
        matrix: AnchorCapabilityMatrix,
    ) -> Vec<String> {
        let mut last = self.last_capabilities.write().await;
        let changes = match last.get(&matrix.anchor_id) {
            Some(prev) => diff_capabilities(prev, &matrix),
            None => Vec::new(),
        };
        if !changes.is_empty() {
            tracing::info!(
                "Anchor '{}' capability changes: {}",
                matrix.anchor_name,
                changes.join(", ")
            );
        }
        last.insert(matrix.anchor_id.clone(), matrix);
        changes
    }

    /// Return the latest capability matrix for an anchor, if one has been recorded.
    pub async fn get_capabilities(&self, anchor_id: &str) -> Option<AnchorCapabilityMatrix> {
        self.last_capabilities.read().await.get(anchor_id).cloned()
    }

    /// Return all recorded capability matrices, optionally filtered by SEP and asset.
    pub async fn list_capabilities(
        &self,
        sep: Option<u32>,
        asset: Option<&str>,
    ) -> Vec<AnchorCapabilityMatrix> {
        let guard = self.last_capabilities.read().await;
        guard
            .values()
            .filter(|m| {
                let sep_ok = sep.map_or(true, |s| {
                    m.seps.iter().any(|c| c.sep == s && c.enabled)
                });
                let asset_ok = asset.map_or(true, |a| m.assets.iter().any(|x| x == a));
                sep_ok && asset_ok
            })
            .cloned()
            .collect()
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
        let stats = self.cache.ge

/* … truncated 16497 chars — edit only what you need near the top … */
