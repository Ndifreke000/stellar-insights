//! Pre-payment check for off-ramp and payout applications.
//!
//! An off-ramp calls this before it moves money: it names the corridor and
//! the amount, and gets back a single decision (`proceed`, `caution`, `hold`
//! or `unknown`) with the individual checks that produced it and healthier
//! alternative corridors to the same destination asset.
//!
//! The endpoint is public and read-only so an integrator can evaluate it
//! without an API key; the anonymous rate-limit tier applies.

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::corridors::{compute_live_corridors, CorridorResponse};
use crate::cache::helpers::cached_query;
use crate::cache::CacheManager;
use crate::database::Database;
use crate::error::{ApiError, ApiResult};
use crate::rpc::StellarRpcClient;
use crate::services::price_feed::PriceFeedClient;

/// Cache key for the corridor table the check scores against.
const CORRIDOR_TABLE_CACHE_KEY: &str = "preflight:corridors";

/// Success rate (percent) below which a corridor is not recommended.
pub const DEFAULT_MIN_SUCCESS_RATE: f64 = 95.0;
/// Payments observed below which the metrics are treated as low-confidence.
pub const MIN_SAMPLE_SIZE: i64 = 20;
/// Share of observed corridor liquidity a single payment can use comfortably.
pub const COMFORTABLE_LIQUIDITY_SHARE: f64 = 0.10;
/// Share of observed corridor liquidity beyond which a payment should wait.
pub const MAX_LIQUIDITY_SHARE: f64 = 0.50;
/// How far below a threshold a metric may fall and still only warn.
const WARN_MARGIN_SUCCESS_RATE: f64 = 10.0;
/// Number of alternative corridors returned.
const MAX_ALTERNATIVES: usize = 3;
/// Most payments one batch request may check.
pub const MAX_BATCH_ITEMS: usize = 100;

/// Pre-payment check request.
#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PreflightRequest {
    /// Asset the payment is sent in: a code (`USDC`) or `CODE:ISSUER`.
    #[schema(example = "USDC")]
    #[param(example = "USDC")]
    pub source_asset: String,
    /// Asset the recipient receives: a code (`NGN`) or `CODE:ISSUER`.
    #[schema(example = "NGN")]
    #[param(example = "NGN")]
    pub destination_asset: String,
    /// Payment size in USD. Omit to skip the liquidity check.
    #[schema(example = 2500.0)]
    #[param(example = 2500.0)]
    pub amount_usd: Option<f64>,
    /// Minimum acceptable success rate in percent (default 95).
    #[schema(example = 95.0)]
    pub min_success_rate: Option<f64>,
}

/// Overall recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PreflightDecision {
    /// Every check passed.
    Proceed,
    /// At least one check is marginal; pay with care or pick an alternative.
    Caution,
    /// At least one check failed; do not pay on this corridor now.
    Hold,
    /// No recent data for this corridor, so no recommendation can be made.
    Unknown,
}

/// Outcome of one check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

/// One check that contributed to the decision.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PreflightCheck {
    /// Stable identifier: `success_rate`, `liquidity`, `sample_size` or
    /// `health_score`.
    #[schema(example = "success_rate")]
    pub name: String,
    pub status: CheckStatus,
    /// Human-readable explanation with the observed value and threshold.
    pub detail: String,
}

/// Pre-payment check result.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PreflightResponse {
    pub decision: PreflightDecision,
    /// One-line summary suitable for showing to an operator.
    pub summary: String,
    /// Corridor health score (0-100) when the corridor is known.
    pub score: Option<f64>,
    /// The corridor that was evaluated, if recent data exists for it.
    pub corridor: Option<CorridorResponse>,
    pub checks: Vec<PreflightCheck>,
    /// Healthier corridors to the same destination asset, best first.
    pub alternatives: Vec<CorridorResponse>,
    /// RFC 3339 timestamp of the evaluation.
    pub evaluated_at: String,
}

/// Thresholds a caller may override per request.
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    pub min_success_rate: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            min_success_rate: DEFAULT_MIN_SUCCESS_RATE,
        }
    }
}

/// Split `CODE` or `CODE:ISSUER` into an upper-cased code and optional issuer.
fn parse_asset(value: &str) -> (String, Option<String>) {
    let trimmed = value.trim();
    match trimmed.split_once(':') {
        Some((code, issuer)) if !issuer.is_empty() => {
            (code.trim().to_uppercase(), Some(issuer.trim().to_string()))
        }
        Some((code, _)) => (code.trim().to_uppercase(), None),
        None => (trimmed.to_uppercase(), None),
    }
}

/// Does one side of a corridor id (`CODE:ISSUER`) match the requested asset?
fn side_matches(side: &str, code: &str, issuer: Option<&str>) -> bool {
    let (side_code, side_issuer) = side.split_once(':').unwrap_or((side, ""));
    if !side_code.eq_ignore_ascii_case(code) {
        return false;
    }
    issuer.is_none_or(|wanted| side_issuer.eq_ignore_ascii_case(wanted))
}

/// Does a corridor carry `source` to `destination`?
fn corridor_matches(
    corridor: &CorridorResponse,
    source: &(String, Option<String>),
    destination: &(String, Option<String>),
) -> bool {
    let Some((src_side, dst_side)) = corridor.id.split_once("->") else {
        return false;
    };
    side_matches(src_side, &source.0, source.1.as_deref())
        && side_matches(dst_side, &destination.0, destination.1.as_deref())
}

fn check(name: &str, status: CheckStatus, detail: String) -> PreflightCheck {
    PreflightCheck {
        name: name.to_string(),
        status,
        detail,
    }
}

/// Score one corridor against the thresholds. Pure, so it is unit-tested
/// without a database or RPC client.
#[must_use]
pub fn run_checks(
    corridor: &CorridorResponse,
    amount_usd: Option<f64>,
    thresholds: Thresholds,
) -> Vec<PreflightCheck> {
    let mut checks = Vec::with_capacity(5);

    let rate = corridor.success_rate;
    let min_rate = thresholds.min_success_rate;
    let status = if rate >= min_rate {
        CheckStatus::Pass
    } else if rate >= min_rate - WARN_MARGIN_SUCCESS_RATE {
        CheckStatus::Warn
    } else {
        CheckStatus::Fail
    };
    checks.push(check(
        "success_rate",
        status,
        format!("{rate:.1}% of recent payments succeeded (minimum {min_rate:.1}%)"),
    ));

    if let Some(amount) = amount_usd {
        let depth = corridor.liquidity_depth_usd;
        let (status, detail) = if depth <= 0.0 {
            (
                CheckStatus::Fail,
                "no liquidity observed on this corridor".to_string(),
            )
        } else {
            let share = amount / depth;
            let status = if share <= COMFORTABLE_LIQUIDITY_SHARE {
                CheckStatus::Pass
            } else if share <= MAX_LIQUIDITY_SHARE {
                CheckStatus::Warn
            } else {
                CheckStatus::Fail
            };
            (
                status,
                format!(
                    "payment is {:.1}% of ${depth:.0} observed liquidity",
                    share * 100.0
                ),
            )
        };
        checks.push(check("liquidity", status, detail));
    }

    // No latency check: corridor latency figures are not measured from
    // settlement data, so a decision must not rest on them.

    let attempts = corridor.total_attempts;
    let status = if attempts >= MIN_SAMPLE_SIZE {
        CheckStatus::Pass
    } else {
        CheckStatus::Warn
    };
    checks.push(check(
        "sample_size",
        status,
        format!("{attempts} recent payments observed (at least {MIN_SAMPLE_SIZE} for confidence)"),
    ));

    let health = corridor.health_score;
    let status = if health >= 80.0 {
        CheckStatus::Pass
    } else if health >= 60.0 {
        CheckStatus::Warn
    } else {
        CheckStatus::Fail
    };
    checks.push(check(
        "health_score",
        status,
        format!("corridor health score {health:.1} of 100"),
    ));

    checks
}

/// Collapse individual checks into one decision: any failure holds the
/// payment, any warning asks for caution.
#[must_use]
pub fn decide(checks: &[PreflightCheck]) -> PreflightDecision {
    if checks.iter().any(|c| c.status == CheckStatus::Fail) {
        PreflightDecision::Hold
    } else if checks.iter().any(|c| c.status == CheckStatus::Warn) {
        PreflightDecision::Caution
    } else {
        PreflightDecision::Proceed
    }
}

fn summarize(decision: PreflightDecision, checks: &[PreflightCheck]) -> String {
    let flagged = |status: CheckStatus| -> String {
        checks
            .iter()
            .filter(|c| c.status == status)
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    match decision {
        PreflightDecision::Proceed => {
            "All checks passed; safe to pay on this corridor.".to_string()
        }
        PreflightDecision::Caution => format!(
            "Marginal on: {}. Pay with care or use an alternative corridor.",
            flagged(CheckStatus::Warn)
        ),
        PreflightDecision::Hold => format!(
            "Failed on: {}. Do not pay on this corridor right now.",
            flagged(CheckStatus::Fail)
        ),
        PreflightDecision::Unknown => {
            "No recent payments observed on this corridor; no recommendation.".to_string()
        }
    }
}

/// Rank alternative corridors by success rate (primary) then estimated cost
/// at the requested payment size (secondary), with corridor id as a final
/// deterministic tie-break (#2668).
///
/// Alternatives were previously ordered by `health_score` alone, which
/// blends success rate with a volume/sample-size term and has no cost
/// dimension at all — two corridors with identical success rates but wildly
/// different liquidity could rank in either order depending on unrelated
/// traffic volume. "Cost" here is the price-impact share a payment of
/// `amount_usd` would take out of a corridor's `liquidity_depth_usd` (now
/// combining order book and AMM pool depth — see
/// `compute_live_corridors`/#2667): a corridor with 10x the liquidity for
/// the same success rate is objectively cheaper to route a given payment
/// through. When no amount is given there's nothing to price, so corridors
/// are compared on raw liquidity depth instead (deeper = lower baseline
/// cost).
#[must_use]
pub fn rank_alternatives(
    mut alternatives: Vec<CorridorResponse>,
    amount_usd: Option<f64>,
) -> Vec<CorridorResponse> {
    alternatives.sort_by(|a, b| {
        b.success_rate
            .total_cmp(&a.success_rate)
            .then_with(|| cost_score(a, amount_usd).total_cmp(&cost_score(b, amount_usd)))
            .then_with(|| a.id.cmp(&b.id))
    });
    alternatives
}

/// Lower is cheaper. The price-impact share of liquidity a payment of
/// `amount_usd` would consume, or — with no amount to price — the inverse
/// of liquidity depth so deeper corridors still sort ahead of shallower
/// ones. A corridor with no observed liquidity is treated as the most
/// expensive (not a division-by-zero panic, and not a false "free").
fn cost_score(corridor: &CorridorResponse, amount_usd: Option<f64>) -> f64 {
    let depth = corridor.liquidity_depth_usd;
    match amount_usd {
        Some(amount) if depth > 0.0 => amount / depth,
        Some(_) => f64::INFINITY,
        None if depth > 0.0 => 1.0 / depth,
        None => f64::INFINITY,
    }
}

/// Evaluate a request against a corridor table. Pure: the caller supplies the
/// corridors and the timestamp.
#[must_use]
pub fn evaluate(
    corridors: &[CorridorResponse],
    request: &PreflightRequest,
    evaluated_at: String,
) -> PreflightResponse {
    let source = parse_asset(&request.source_asset);
    let destination = parse_asset(&request.destination_asset);
    let thresholds = Thresholds {
        min_success_rate: request.min_success_rate.unwrap_or(DEFAULT_MIN_SUCCESS_RATE),
    };

    // Several issuers can serve the same code pair; evaluate the healthiest.
    let chosen = corridors
        .iter()
        .filter(|c| corridor_matches(c, &source, &destination))
        .max_by(|a, b| a.health_score.total_cmp(&b.health_score));

    let alternatives: Vec<CorridorResponse> = corridors
        .iter()
        .filter(|c| {
            c.id.split_once("->")
                .is_some_and(|(_, dst)| side_matches(dst, &destination.0, destination.1.as_deref()))
        })
        .filter(|c| chosen.is_none_or(|picked| picked.id != c.id))
        .filter(|c| chosen.is_none_or(|picked| c.health_score > picked.health_score))
        .cloned()
        .collect();
    let mut alternatives = rank_alternatives(alternatives, request.amount_usd);
    alternatives.truncate(MAX_ALTERNATIVES);

    let Some(corridor) = chosen else {
        return PreflightResponse {
            decision: PreflightDecision::Unknown,
            summary: summarize(PreflightDecision::Unknown, &[]),
            score: None,
            corridor: None,
            checks: Vec::new(),
            alternatives,
            evaluated_at,
        };
    };

    let checks = run_checks(corridor, request.amount_usd, thresholds);
    let decision = decide(&checks);
    PreflightResponse {
        decision,
        summary: summarize(decision, &checks),
        score: Some(corridor.health_score),
        corridor: Some(corridor.clone()),
        checks,
        alternatives,
        evaluated_at,
    }
}

/// Reject a malformed request with a stable error code and message.
fn check_request(request: &PreflightRequest) -> Result<(), (&'static str, &'static str)> {
    if request.source_asset.trim().is_empty() || request.destination_asset.trim().is_empty() {
        return Err((
            "INVALID_ASSET",
            "source_asset and destination_asset are required",
        ));
    }
    if let Some(amount) = request.amount_usd {
        if !amount.is_finite() || amount <= 0.0 {
            return Err(("INVALID_AMOUNT", "amount_usd must be a positive number"));
        }
    }
    if let Some(rate) = request.min_success_rate {
        if !(0.0..=100.0).contains(&rate) {
            return Err((
                "INVALID_THRESHOLD",
                "min_success_rate must be between 0 and 100",
            ));
        }
    }
    Ok(())
}

fn validate(request: &PreflightRequest) -> ApiResult<()> {
    check_request(request).map_err(|(code, message)| ApiError::bad_request(code, message))
}

/// Several pre-payment checks in one request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PreflightBatchRequest {
    /// Payments to check, 1 to 100.
    pub items: Vec<PreflightRequest>,
}

/// Why one item in a batch could not be evaluated.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PreflightItemError {
    /// Same codes as the single check: `INVALID_ASSET`, `INVALID_AMOUNT`,
    /// `INVALID_THRESHOLD`.
    #[schema(example = "INVALID_AMOUNT")]
    pub code: String,
    pub message: String,
}

/// Outcome for one item, in request order. Exactly one of `result` and
/// `error` is set.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PreflightBatchItem {
    /// Position of the item in the request.
    pub index: usize,
    pub result: Option<PreflightResponse>,
    pub error: Option<PreflightItemError>,
}

/// Batch pre-payment check result.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PreflightBatchResponse {
    pub results: Vec<PreflightBatchItem>,
    /// RFC 3339 timestamp shared by every item: all are scored against the
    /// same corridor snapshot.
    pub evaluated_at: String,
}

/// Evaluate every item of a batch against one corridor table. An invalid item
/// is reported in place and does not fail the others. Pure, like `evaluate`.
#[must_use]
pub fn evaluate_batch(
    corridors: &[CorridorResponse],
    items: &[PreflightRequest],
    evaluated_at: String,
) -> PreflightBatchResponse {
    let results = items
        .iter()
        .enumerate()
        .map(|(index, item)| match check_request(item) {
            Ok(()) => PreflightBatchItem {
                index,
                result: Some(evaluate(corridors, item, evaluated_at.clone())),
                error: None,
            },
            Err((code, message)) => PreflightBatchItem {
                index,
                result: None,
                error: Some(PreflightItemError {
                    code: code.to_string(),
                    message: message.to_string(),
                }),
            },
        })
        .collect();
    PreflightBatchResponse {
        results,
        evaluated_at,
    }
}

type PreflightState = (
    Arc<Database>,
    Arc<CacheManager>,
    Arc<StellarRpcClient>,
    Arc<PriceFeedClient>,
);

async fn load_corridors(
    cache: &Arc<CacheManager>,
    rpc_client: &Arc<StellarRpcClient>,
    price_feed: &Arc<PriceFeedClient>,
) -> ApiResult<Vec<CorridorResponse>> {
    Ok(cached_query(
        cache,
        CORRIDOR_TABLE_CACHE_KEY,
        cache.config.get_ttl("corridor"),
        || async { compute_live_corridors(rpc_client, price_feed).await },
    )
    .await?)
}

async fn run(
    cache: &Arc<CacheManager>,
    rpc_client: &Arc<StellarRpcClient>,
    price_feed: &Arc<PriceFeedClient>,
    request: &PreflightRequest,
) -> ApiResult<PreflightResponse> {
    validate(request)?;

    let corridors = load_corridors(cache, rpc_client, price_feed).await?;

    Ok(evaluate(
        &corridors,
        request,
        chrono::Utc::now().to_rfc3339(),
    ))
}

/// Check a corridor before paying (JSON body).
#[utoipa::path(
    post,
    path = "/api/v1/preflight",
    request_body = PreflightRequest,
    responses(
        (status = 200, description = "Decision with the checks behind it", body = PreflightResponse),
        (status = 400, description = "Missing asset or invalid amount/threshold"),
        (status = 500, description = "Corridor data could not be loaded")
    ),
    tag = "Preflight"
)]
pub async fn preflight_post(
    State((_db, cache, rpc_client, price_feed)): State<PreflightState>,
    Json(request): Json<PreflightRequest>,
) -> ApiResult<Json<PreflightResponse>> {
    Ok(Json(run(&cache, &rpc_client, &price_feed, &request).await?))
}

/// Check a corridor before paying (query string).
#[utoipa::path(
    get,
    path = "/api/v1/preflight",
    params(PreflightRequest),
    responses(
        (status = 200, description = "Decision with the checks behind it", body = PreflightResponse),
        (status = 400, description = "Missing asset or invalid amount/threshold"),
        (status = 500, description = "Corridor data could not be loaded")
    ),
    tag = "Preflight"
)]
pub async fn preflight_get(
    State((_db, cache, rpc_client, price_feed)): State<PreflightState>,
    Query(request): Query<PreflightRequest>,
) -> ApiResult<Json<PreflightResponse>> {
    Ok(Json(run(&cache, &rpc_client, &price_feed, &request).await?))
}

/// Check several payments in one request (JSON body).
///
/// The corridor table is loaded once and every item is scored against it.
/// Items are validated individually: a bad item comes back with an `error`
/// and the rest are still evaluated. Only an empty or oversized batch fails
/// the whole request.
#[utoipa::path(
    post,
    path = "/api/v1/preflight/batch",
    request_body = PreflightBatchRequest,
    responses(
        (status = 200, description = "One result or error per item, in request order", body = PreflightBatchResponse),
        (status = 400, description = "Batch is empty or has more than 100 items"),
        (status = 500, description = "Corridor data could not be loaded")
    ),
    tag = "Preflight"
)]
pub async fn preflight_batch(
    State((_db, cache, rpc_client, price_feed)): State<PreflightState>,
    Json(request): Json<PreflightBatchRequest>,
) -> ApiResult<Json<PreflightBatchResponse>> {
    if request.items.is_empty() {
        return Err(ApiError::bad_request(
            "EMPTY_BATCH",
            "items must contain at least one payment",
        ));
    }
    if request.items.len() > MAX_BATCH_ITEMS {
        return Err(ApiError::bad_request(
            "BATCH_TOO_LARGE",
            format!("items may contain at most {MAX_BATCH_ITEMS} payments"),
        ));
    }

    let corridors = load_corridors(&cache, &rpc_client, &price_feed).await?;
    Ok(Json(evaluate_batch(
        &corridors,
        &request.items,
        chrono::Utc::now().to_rfc3339(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn corridor(
        id: &str,
        success_rate: f64,
        depth: f64,
        attempts: i64,
        health: f64,
    ) -> CorridorResponse {
        let (src, dst) = id.split_once("->").unwrap_or((id, id));
        CorridorResponse {
            id: id.to_string(),
            source_asset: src.split(':').next().unwrap_or(src).to_string(),
            destination_asset: dst.split(':').next().unwrap_or(dst).to_string(),
            success_rate,
            total_attempts: attempts,
            successful_payments: attempts,
            failed_payments: 0,
            average_latency_ms: 600.0,
            median_latency_ms: 450.0,
            p95_latency_ms: 1_500.0,
            p99_latency_ms: 2_400.0,
            liquidity_depth_usd: depth,
            liquidity_volume_24h_usd: depth * 0.1,
            liquidity_trend: "stable".to_string(),
            health_score: health,
            last_updated: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn request(source: &str, destination: &str, amount: Option<f64>) -> PreflightRequest {
        PreflightRequest {
            source_asset: source.to_string(),
            destination_asset: destination.to_string(),
            amount_usd: amount,
            min_success_rate: None,
        }
    }

    fn now() -> String {
        "2026-01-01T00:00:00Z".to_string()
    }

    fn status_of(response: &PreflightResponse, name: &str) -> Option<CheckStatus> {
        response
            .checks
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.status)
    }

    #[test]
    fn healthy_corridor_proceeds() {
        let table = vec![corridor("USDC:GA->NGN:GB", 99.5, 500_000.0, 400, 92.0)];
        let result = evaluate(&table, &request("USDC", "NGN", Some(2_500.0)), now());

        assert_eq!(result.decision, PreflightDecision::Proceed);
        assert_eq!(result.score, Some(92.0));
        assert!(result.checks.iter().all(|c| c.status == CheckStatus::Pass));
    }

    #[test]
    fn unknown_corridor_gives_no_recommendation() {
        let table = vec![corridor("USDC:GA->NGN:GB", 99.5, 500_000.0, 400, 92.0)];
        let result = evaluate(&table, &request("USDC", "KES", Some(100.0)), now());

        assert_eq!(result.decision, PreflightDecision::Unknown);
        assert!(result.corridor.is_none());
        assert!(result.checks.is_empty());
        assert!(result.score.is_none());
    }

    #[test]
    fn low_success_rate_holds_the_payment() {
        let table = vec![corridor("USDC:GA->NGN:GB", 70.0, 500_000.0, 400, 85.0)];
        let result = evaluate(&table, &request("USDC", "NGN", Some(100.0)), now());

        assert_eq!(result.decision, PreflightDecision::Hold);
        assert_eq!(status_of(&result, "success_rate"), Some(CheckStatus::Fail));
        assert!(result.summary.contains("success_rate"));
    }

    #[test]
    fn marginal_success_rate_is_a_caution() {
        let table = vec![corridor("USDC:GA->NGN:GB", 90.0, 500_000.0, 400, 85.0)];
        let result = evaluate(&table, &request("USDC", "NGN", Some(100.0)), now());

        assert_eq!(result.decision, PreflightDecision::Caution);
        assert_eq!(status_of(&result, "success_rate"), Some(CheckStatus::Warn));
    }

    #[test]
    fn payment_larger_than_half_the_liquidity_holds() {
        let table = vec![corridor("USDC:GA->NGN:GB", 99.5, 10_000.0, 400, 92.0)];
        let result = evaluate(&table, &request("USDC", "NGN", Some(6_000.0)), now());

        assert_eq!(result.decision, PreflightDecision::Hold);
        assert_eq!(status_of(&result, "liquidity"), Some(CheckStatus::Fail));
    }

    #[test]
    fn payment_between_ten_and_fifty_percent_of_liquidity_warns() {
        let table = vec![corridor("USDC:GA->NGN:GB", 99.5, 10_000.0, 400, 92.0)];
        let result = evaluate(&table, &request("USDC", "NGN", Some(3_000.0)), now());

        assert_eq!(result.decision, PreflightDecision::Caution);
        assert_eq!(status_of(&result, "liquidity"), Some(CheckStatus::Warn));
    }

    #[test]
    fn liquidity_check_is_skipped_without_an_amount() {
        let table = vec![corridor("USDC:GA->NGN:GB", 99.5, 10_000.0, 400, 92.0)];
        let result = evaluate(&table, &request("USDC", "NGN", None), now());

        assert_eq!(status_of(&result, "liquidity"), None);
        assert_eq!(result.decision, PreflightDecision::Proceed);
    }

    #[test]
    fn thin_sample_is_a_caution_not_a_hold() {
        let table = vec![corridor("USDC:GA->NGN:GB", 100.0, 500_000.0, 3, 92.0)];
        let result = evaluate(&table, &request("USDC", "NGN", Some(100.0)), now());

        assert_eq!(result.decision, PreflightDecision::Caution);
        assert_eq!(status_of(&result, "sample_size"), Some(CheckStatus::Warn));
    }

    #[test]
    fn caller_thresholds_override_the_defaults() {
        let table = vec![corridor("USDC:GA->NGN:GB", 97.0, 500_000.0, 400, 92.0)];
        let mut strict = request("USDC", "NGN", Some(100.0));
        strict.min_success_rate = Some(99.0);
        let result = evaluate(&table, &strict, now());

        assert_eq!(status_of(&result, "success_rate"), Some(CheckStatus::Warn));
    }

    #[test]
    fn asset_matching_ignores_case_and_honours_the_issuer() {
        let table = vec![
            corridor("USDC:GA->NGN:GB", 99.5, 500_000.0, 400, 92.0),
            corridor("USDC:GA->NGN:GC", 99.5, 500_000.0, 400, 70.0),
        ];

        let by_code = evaluate(&table, &request("usdc", "ngn", None), now());
        assert_eq!(
            by_code.corridor.map(|c| c.id),
            Some("USDC:GA->NGN:GB".to_string())
        );

        let by_issuer = evaluate(&table, &request("USDC", "NGN:GC", None), now());
        assert_eq!(
            by_issuer.corridor.map(|c| c.id),
            Some("USDC:GA->NGN:GC".to_string())
        );
    }

    #[test]
    fn alternatives_are_healthier_corridors_to_the_same_destination() {
        let table = vec![
            corridor("USDC:GA->NGN:GB", 80.0, 500_000.0, 400, 55.0),
            corridor("XLM:native->NGN:GB", 99.0, 900_000.0, 800, 95.0),
            corridor("EURC:GD->NGN:GB", 98.0, 400_000.0, 300, 88.0),
            corridor("USDC:GA->KES:GE", 99.0, 900_000.0, 800, 99.0),
        ];
        let result = evaluate(&table, &request("USDC", "NGN", Some(100.0)), now());

        let ids: Vec<&str> = result.alternatives.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["XLM:native->NGN:GB", "EURC:GD->NGN:GB"]);
    }

    #[test]
    fn validation_rejects_bad_input() {
        assert!(validate(&request("", "NGN", None)).is_err());
        assert!(validate(&request("USDC", "NGN", Some(0.0))).is_err());
        assert!(validate(&request("USDC", "NGN", Some(f64::NAN))).is_err());

        let mut bad_rate = request("USDC", "NGN", None);
        bad_rate.min_success_rate = Some(140.0);
        assert!(validate(&bad_rate).is_err());

        assert!(validate(&request("USDC", "NGN", Some(10.0))).is_ok());
    }

    // ── #2668: alternatives ranked by success rate then cost ───────────────

    #[test]
    fn alternatives_rank_by_success_rate_first() {
        // B has a lower success rate than A, so A must rank first even
        // though B has far deeper liquidity (cheaper) — success rate is the
        // primary key, not a tie-break.
        let table = vec![
            corridor("USDC:GA->NGN:GB", 50.0, 500_000.0, 400, 10.0), // chosen: low health, not filtered out of alternatives
            corridor("XLM:native->NGN:GB", 99.0, 10_000.0, 400, 50.0), // A: higher success rate, thin liquidity
            corridor("EURC:GD->NGN:GB", 90.0, 1_000_000.0, 400, 50.0), // B: lower success rate, deep liquidity
        ];
        let result = evaluate(&table, &request("USDC", "NGN", Some(50_000.0)), now());

        let ids: Vec<&str> = result.alternatives.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["XLM:native->NGN:GB", "EURC:GD->NGN:GB"]);
    }

    #[test]
    fn alternatives_break_ties_on_cost_when_success_rate_matches() {
        // Equal success rate: the corridor with deeper liquidity (lower
        // price-impact cost for this payment) must rank first.
        let table = vec![
            corridor("USDC:GA->NGN:GB", 50.0, 500_000.0, 400, 10.0), // chosen
            corridor("XLM:native->NGN:GB", 97.0, 50_000.0, 400, 50.0), // thinner liquidity
            corridor("EURC:GD->NGN:GB", 97.0, 500_000.0, 400, 50.0), // deeper liquidity, same success rate
        ];
        let result = evaluate(&table, &request("USDC", "NGN", Some(50_000.0)), now());

        let ids: Vec<&str> = result.alternatives.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["EURC:GD->NGN:GB", "XLM:native->NGN:GB"]);
    }

    #[test]
    fn alternatives_without_an_amount_rank_by_liquidity_depth() {
        let table = vec![
            corridor("USDC:GA->NGN:GB", 50.0, 100.0, 10, 10.0), // chosen
            corridor("XLM:native->NGN:GB", 90.0, 1_000.0, 10, 50.0), // shallower
            corridor("EURC:GD->NGN:GB", 90.0, 10_000.0, 10, 50.0), // deeper
        ];
        let result = evaluate(&table, &request("USDC", "NGN", None), now());

        let ids: Vec<&str> = result.alternatives.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["EURC:GD->NGN:GB", "XLM:native->NGN:GB"]);
    }

    #[test]
    fn alternatives_ranking_is_deterministic_on_a_full_tie() {
        let table = vec![
            corridor("AAA:GA->NGN:GB", 50.0, 100.0, 10, 10.0), // chosen
            corridor("ZZZ:GB->NGN:GB", 90.0, 100.0, 10, 50.0),
            corridor("BBB:GC->NGN:GB", 90.0, 100.0, 10, 50.0),
        ];
        let result = evaluate(&table, &request("AAA", "NGN", None), now());

        let ids: Vec<&str> = result.alternatives.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["BBB:GC->NGN:GB", "ZZZ:GB->NGN:GB"]);
    }

    #[test]
    fn rank_alternatives_is_a_pure_deterministic_sort() {
        let alts = vec![
            corridor("A->X", 90.0, 1_000.0, 10, 0.0),
            corridor("B->X", 95.0, 1_000.0, 10, 0.0),
            corridor("C->X", 95.0, 2_000.0, 10, 0.0),
        ];
        let ranked = rank_alternatives(alts.clone(), Some(100.0));
        let ids: Vec<&str> = ranked.iter().map(|c| c.id.as_str()).collect();
        // 95% success rate corridors (B, C) rank above the 90% one (A); of
        // the two, C's deeper liquidity makes it cheaper than B.
        assert_eq!(ids, vec!["C->X", "B->X", "A->X"]);

        // Same input, run again: identical output.
        let ranked_again = rank_alternatives(alts, Some(100.0));
        let ids_again: Vec<&str> = ranked_again.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ids_again);
    }
}

/// Property tests for the decision invariants (#2645): adding a failing check
/// never improves the decision, worse inputs never improve it, and thin data
/// is never a clean `Proceed`.
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// Higher is worse. `Unknown` is not produced by `decide`.
    fn severity(decision: PreflightDecision) -> u8 {
        match decision {
            PreflightDecision::Proceed => 0,
            PreflightDecision::Caution => 1,
            PreflightDecision::Hold => 2,
            PreflightDecision::Unknown => 3,
        }
    }

    fn status() -> impl Strategy<Value = CheckStatus> {
        prop_oneof![
            Just(CheckStatus::Pass),
            Just(CheckStatus::Warn),
            Just(CheckStatus::Fail)
        ]
    }

    fn checks() -> impl Strategy<Value = Vec<PreflightCheck>> {
        prop::collection::vec(status(), 0..8).prop_map(|statuses| {
            statuses
                .into_iter()
                .map(|s| check("generated", s, String::new()))
                .collect()
        })
    }

    fn corridor(rate: f64, depth: f64, attempts: i64, health: f64) -> CorridorResponse {
        super::tests::corridor("USDC:GA->EURC:GB", rate, depth, attempts, health)
    }

    proptest! {
        #[test]
        fn adding_a_failing_check_always_holds(mut cs in checks()) {
            cs.push(check("failing", CheckStatus::Fail, String::new()));
            prop_assert_eq!(decide(&cs), PreflightDecision::Hold);
        }

        #[test]
        fn adding_any_check_never_improves_the_decision(cs in checks(), extra in status()) {
            let before = severity(decide(&cs));
            let mut more = cs.clone();
            more.push(check("extra", extra, String::new()));
            prop_assert!(severity(decide(&more)) >= before);
        }

        #[test]
        fn small_sample_is_never_proceed(
            rate in 0.0f64..=100.0,
            depth in 0.0f64..1e9,
            attempts in 0i64..MIN_SAMPLE_SIZE,
            health in 0.0f64..=100.0,
            amount in prop::option::of(0.0f64..1e7),
        ) {
            let checks = run_checks(&corridor(rate, depth, attempts, health), amount, Thresholds::default());
            prop_assert_ne!(decide(&checks), PreflightDecision::Proceed);
        }

        #[test]
        fn lower_success_rate_never_improves_the_decision(
            a in 0.0f64..=100.0,
            b in 0.0f64..=100.0,
            attempts in 0i64..1_000,
            health in 0.0f64..=100.0,
        ) {
            let (low, high) = if a <= b { (a, b) } else { (b, a) };
            let decision = |rate| decide(&run_checks(&corridor(rate, 1e6, attempts, health), None, Thresholds::default()));
            prop_assert!(severity(decision(low)) >= severity(decision(high)));
        }

        #[test]
        fn larger_payment_never_improves_the_decision(
            a in 0.0f64..1e7,
            b in 0.0f64..1e7,
            depth in 0.0f64..1e7,
            rate in 0.0f64..=100.0,
        ) {
            let (small, large) = if a <= b { (a, b) } else { (b, a) };
            let decision = |amount| decide(&run_checks(&corridor(rate, depth, 500, 90.0), Some(amount), Thresholds::default()));
            prop_assert!(severity(decision(large)) >= severity(decision(small)));
        }

        #[test]
        fn no_matching_corridor_is_unknown(rate in 0.0f64..=100.0) {
            let request = PreflightRequest {
                source_asset: "XLM".to_string(),
                destination_asset: "BRL".to_string(),
                amount_usd: None,
                min_success_rate: None,
            };
            let response = evaluate(&[corridor(rate, 1e6, 500, 90.0)], &request, String::new());
            prop_assert_eq!(response.decision, PreflightDecision::Unknown);
            prop_assert!(response.checks.is_empty());
        }
    }
}
