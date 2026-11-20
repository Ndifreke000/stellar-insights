#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Duration, Utc};
use csv::Writer;
use futures::StreamExt;
use rust_xlsxwriter::{Color, Format, Workbook};
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};
use crate::middleware::cost_rate_limiter::{CostRateLimiter, extract_client_key, get_client_tier};
use crate::models::cost::{CostConfig, CostHeaders};
use crate::models::PaymentRow;
use crate::state::AppState;

/// Prefix any cell that begins with a formula-trigger character so spreadsheet
/// applications treat it as literal text instead of executing it as a formula.
fn sanitize_csv_field(value: String) -> String {
    if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value
    }
}

/// Largest date range an export may request (#2669). Both export endpoints
/// previously accepted any `start_date`/`end_date` pair with no upper bound
/// on the span, so a multi-year request could pull an unbounded number of
/// rows into memory before any format-specific limit (like `export_payments`'
/// row `LIMIT`) even applied.
const MAX_EXPORT_RANGE_DAYS: i64 = 90;

/// Hard cap on rows materialized for a single export response, independent
/// of the date-range limit above — belt-and-suspenders against a dense
/// window (e.g. a corridor with very high transaction volume) still
/// producing an unbounded result set.
const MAX_EXPORT_ROWS: usize = 5_000;

/// Reject a date range wider than `MAX_EXPORT_RANGE_DAYS`, or one where
/// `end` precedes `start`.
fn validate_export_range(start: chrono::NaiveDate, end: chrono::NaiveDate) -> ApiResult<()> {
    let span_days = end.signed_duration_since(start).num_days();
    if span_days < 0 {
        return Err(ApiError::bad_request(
            "INVALID_RANGE",
            "end_date must not be before start_date",
        ));
    }
    if span_days > MAX_EXPORT_RANGE_DAYS {
        return Err(ApiError::bad_request(
            "RANGE_TOO_WIDE",
            format!(
                "Requested range is {span_days} days; exports are limited to {MAX_EXPORT_RANGE_DAYS} days to avoid unbounded result sets"
            ),
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ExportQuery {
    /// Output format: `csv`, `json` or `excel`
    pub format: String, // "csv", "json", "excel"
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub corridor_id: Option<String>,
}

/// Calculate estimated cost for a corridor export request.
fn calculate_corridor_export_cost(
    config: &CostConfig,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
    corridor_id: Option<&str>,
) -> f64 {
    let span_days = end_date.signed_duration_since(start_date).num_days();
    
    if let Some(_corridor_id) = corridor_id {
        // Single corridor - use single corridor cost
        config.export_corridor_single_cost(span_days)
    } else {
        // All corridors - estimate worst case
        config.export_corridor_all_cost(span_days)
    }
}

/// Add cost estimate headers to a response.
fn add_cost_headers(mut headers: HeaderMap, cost_headers: &CostHeaders) -> HeaderMap {
    headers.insert(
        "X-Cost-Estimate",
        HeaderValue::from_str(&cost_headers.cost_estimate.to_string()).unwrap_or_default(),
    );
    headers.insert(
        "X-Remaining-Budget",
        HeaderValue::from_str(&cost_headers.remaining_budget.to_string()).unwrap_or_default(),
    );
    headers.insert(
        "X-Total-Budget",
        HeaderValue::from_str(&cost_headers.total_budget.to_string()).unwrap_or_default(),
    );
    headers.insert(
        "X-Tier",
        HeaderValue::from_str(&cost_headers.tier).unwrap_or_default(),
    );
    headers.insert(
        "X-Budget-Refresh-At",
        HeaderValue::from_str(&cost_headers.budget_refresh_at.to_string()).unwrap_or_default(),
    );
    headers
}

/// Export corridors as CSV, JSON or Excel
#[utoipa::path(
    get,
    path = "/api/v1/export/corridors",
    params(ExportQuery),
    responses(
        (status = 200, description = "Exported corridors file (content type depends on `format`)"),
        (status = 400, description = "Unsupported export format"),
        (status = 429, description = "Rate limited - insufficient budget"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Export"
)]
pub async fn export_corridors(
    State(app_state): State<AppState>,
    Query(params): Query<ExportQuery>,
) -> ApiResult<Response> {
    let today = Utc::now().date_naive();
    let start_date = params
        .start_date
        .map_or(today - Duration::days(30), |d| d.date_naive());
    let end_date = params.end_date.map_or(today, |d| d.date_naive());

    validate_export_range(start_date, end_date)?;

    // Calculate cost estimate before fetching data
    let config = CostConfig::default();
    let cost_estimate = calculate_corridor_export_cost(
        &config,
        start_date,
        end_date,
        params.corridor_id.as_deref(),
    );

    // Extract client information from headers
    let client_key = extract_client_key(None, None, None);
    let tier = get_client_tier(None, None);

    // Get the cost rate limiter from AppState
    let cost_limiter = app_state.cost_rate_limiter.clone();

    // Check if client has sufficient budget
    if !cost_limiter.has_sufficient_budget(&client_key, tier, cost_estimate).await {
        let retry_after = 24 * 3600; // 24 hours until budget refresh
        
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "Retry-After",
            HeaderValue::from_str(&retry_after.to_string()).unwrap_or_default(),
        );
        
        let cost_headers = CostHeaders::from_state(
            &crate::models::cost::ClientCostState::new(0.0, tier),
            cost_estimate,
        );
        headers = add_cost_headers(headers, &cost_headers);
        
        let body = serde_json::json!({
            "error": "RATE_LIMITED",
            "code": "INSUFFICIENT_BUDGET",
            "message": format!(
                "Insufficient budget for this export. Estimated cost: {:.2}, Remaining budget: {:.2}",
                cost_estimate,
                cost_headers.remaining_budget
            ),
            "retry_after_seconds": retry_after,
        });
        
        return Err(ApiError::new(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            format!(
                "Insufficient budget for this export. Estimated cost: {:.2}, Remaining budget: {:.2}",
                cost_estimate,
                cost_headers.remaining_budget
            ),
            Some(body),
        ));
    }

    // Debit the cost from client's budget
    if let Err(e) = cost_limiter.debit(&client_key, tier, cost_estimate).await {
        return Err(ApiError::internal(
            "COST_ERROR",
            format!("Failed to debit cost: {}", e),
        ));
    }

    let corridors = app_state
        .db
        .corridor_aggregates()
        .get_aggregated_corridor_metrics(start_date, end_date)
        .await
        .map_err(|e| {
            ApiError::internal(
                "DATABASE_ERROR",
                format!("Failed to fetch corridors for export: {e}"),
            )
        })?;

    let mut filtered_corridors: Vec<_> = if let Some(corridor_id) = &params.corridor_id {
        corridors
            .into_iter()
            .filter(|c| &c.corridor_key == corridor_id)
            .collect()
    } else {
        corridors
    };
    // `get_aggregated_corridor_metrics` has no row limit of its own — this
    // is the backstop against a wide, high-cardinality window still
    // producing an unbounded result set (#2669).
    filtered_corridors.truncate(MAX_EXPORT_ROWS);

    let mut headers = HeaderMap::new();
    let cost_headers = CostHeaders::from_state(
        &crate::models::cost::ClientCostState::new(0.0, tier),
        cost_estimate,
    );
    headers = add_cost_headers(headers, &cost_headers);

    match params.format.to_lowercase().as_str() {
        "csv" => {
            let result = export_corridors_csv(&filtered_corridors)?;
            let (mut resp_headers, data) = result.into_response().into_parts();
            resp_headers.extend(headers);
            Ok(Response::from_parts(resp_headers, data))
        }
        "json" => {
            let result = export_corridors_json(&filtered_corridors)?;
            let (mut resp_headers, data) = result.into_response().into_parts();
            resp_headers.extend(headers);
            Ok(Response::from_parts(resp_headers, data))
        }
        "excel" | "xlsx" => {
            let result = export_corridors_excel(&filtered_corridors)?;
            let (mut resp_headers, data) = result.into_response().into_parts();
            resp_headers.extend(headers);
            Ok(Response::from_parts(resp_headers, data))
        }
        _ => Err(ApiError::bad_request(
            "INVALID_FORMAT",
            format!("Format {} is not supported", params.format),
        )),
    }
}

/// Helper to export corridors as CSV
fn export_corridors_csv(
    corridors: &[crate::db::aggregates::AggregatedCorridorMetrics],
) -> ApiResult<impl IntoResponse> {
    let mut wtr = Writer::from_writer(vec![]);
    wtr.write_record([
        "Corridor ID",
        "Source Asset",
        "Source Issuer",
        "Destination Asset",
        "Destination Issuer",
        "Success Rate (%)",
        "Total Transactions",
        "Successful Transactions",
        "Failed Transactions",
        "Volume (USD)",
        "Latest Date",
    ])
    .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;

    for m in corridors {
        wtr.write_record(&[
            sanitize_csv_field(m.corridor_key.clone()),
            sanitize_csv_field(m.source_asset_code.clone()),
            sanitize_csv_field(m.source_asset_issuer.clone()),
            sanitize_csv_field(m.destination_asset_code.clone()),
            sanitize_csv_field(m.destination_asset_issuer.clone()),
            format!("{:.2}", m.avg_success_rate),
            m.total_transactions.to_string(),
            m.successful_transactions.to_string(),
            m.failed_transactions.to_string(),
            format!("{:.2}", m.total_volume_usd),
            m.latest_date.to_string(),
        ])
        .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
    }

    let data = wtr
        .into_inner()
        .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/csv"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"corridors_export.csv\""),
    );

    Ok((headers, data))
}

/// Helper to export corridors as JSON
fn export_corridors_json(
    corridors: &[crate::db::aggregates::AggregatedCorridorMetrics],
) -> ApiResult<impl IntoResponse> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"corridors_export.json\""),
    );

    let data = serde_json::to_vec(corridors)
        .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
    Ok((headers, data))
}

/// Helper to export corridors as Excel
fn export_corridors_excel(
    corridors: &[crate::db::aggregates::AggregatedCorridorMetrics],
) -> ApiResult<impl IntoResponse> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x00D9_EAD3));

    let headers = [
        "Corridor ID",
        "Source Asset",
        "Source Issuer",
        "Destination Asset",
        "Destination Issuer",
        "Success Rate (%)",
        "Total Transactions",
        "Successful Transactions",
        "Failed Transactions",
        "Volume (USD)",
        "Latest Date",
    ];

    for (i, h) in headers.iter().enumerate() {
        worksheet
            .write_with_format(0, i as u16, *h, &header_format)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
    }

    for (row, m) in corridors.iter().enumerate() {
        let row = (row + 1) as u32;
        worksheet
            .write(row, 0, &m.corridor_key)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 1, &m.source_asset_code)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 2, &m.source_asset_issuer)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 3, &m.destination_asset_code)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 4, &m.destination_asset_issuer)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 5, m.avg_success_rate)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 6, m.total_transactions as f64)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 7, m.successful_transactions as f64)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 8, m.failed_transactions as f64)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 9, m.total_volume_usd)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 10, m.latest_date.to_string())
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
    }

    let data = workbook
        .save_to_buffer()
        .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"corridors_export.xlsx\""),
    );

    Ok((headers, data))
}

/// Export anchors as CSV, JSON or Excel
#[utoipa::path(
    get,
    path = "/api/v1/export/anchors",
    params(ExportQuery),
    responses(
        (status = 200, description = "Exported anchors file (content type depends on `format`)"),
        (status = 400, description = "Unsupported export format"),
        (status = 429, description = "Rate limited - insufficient budget"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Export"
)]
pub async fn export_anchors(
    State(app_state): State<AppState>,
    Query(params): Query<ExportQuery>,
) -> ApiResult<Response> {
    // Calculate cost estimate (anchors are simple, use per-request cost)
    let config = CostConfig::default();
    let cost_estimate = config.per_analytics_request * 0.5; // Lower cost for anchors

    // Extract client information from headers
    let client_key = extract_client_key(None, None, None);
    let tier = get_client_tier(None, None);

    // Get the cost rate limiter from AppState
    let cost_limiter = app_state.cost_rate_limiter.clone();

    // Check if client has sufficient budget
    if !cost_limiter.has_sufficient_budget(&client_key, tier, cost_estimate).await {
        let retry_after = 24 * 3600;
        
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "Retry-After",
            HeaderValue::from_str(&retry_after.to_string()).unwrap_or_default(),
        );
        
        let cost_headers = CostHeaders::from_state(
            &crate::models::cost::ClientCostState::new(0.0, tier),
            cost_estimate,
        );
        headers = add_cost_headers(headers, &cost_headers);
        
        let body = serde_json::json!({
            "error": "RATE_LIMITED",
            "code": "INSUFFICIENT_BUDGET",
            "message": format!(
                "Insufficient budget for this export. Estimated cost: {:.2}, Remaining budget: {:.2}",
                cost_estimate,
                cost_headers.remaining_budget
            ),
            "retry_after_seconds": retry_after,
        });
        
        return Err(ApiError::new(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            format!(
                "Insufficient budget for this export. Estimated cost: {:.2}, Remaining budget: {:.2}",
                cost_estimate,
                cost_headers.remaining_budget
            ),
            Some(body),
        ));
    }

    // Debit the cost from client's budget
    if let Err(e) = cost_limiter.debit(&client_key, tier, cost_estimate).await {
        return Err(ApiError::internal(
            "COST_ERROR",
            format!("Failed to debit cost: {}", e),
        ));
    }

    let anchors = app_state.db.list_anchors(1000, 0).await.map_err(|e| {
        ApiError::internal(
            "DATABASE_ERROR",
            format!("Failed to fetch anchors for export: {e}"),
        )
    })?;

    let mut headers = HeaderMap::new();
    let cost_headers = CostHeaders::from_state(
        &crate::models::cost::ClientCostState::new(0.0, tier),
        cost_estimate,
    );
    headers = add_cost_headers(headers, &cost_headers);

    match params.format.to_lowercase().as_str() {
        "csv" => {
            let result = export_anchors_csv(&anchors)?;
            let (mut resp_headers, data) = result.into_response().into_parts();
            resp_headers.extend(headers);
            Ok(Response::from_parts(resp_headers, data))
        }
        "json" => {
            let result = export_anchors_json(&anchors)?;
            let (mut resp_headers, data) = result.into_response().into_parts();
            resp_headers.extend(headers);
            Ok(Response::from_parts(resp_headers, data))
        }
        "excel" | "xlsx" => {
            let result = export_anchors_excel(&anchors)?;
            let (mut resp_headers, data) = result.into_response().into_parts();
            resp_headers.extend(headers);
            Ok(Response::from_parts(resp_headers, data))
        }
        _ => Err(ApiError::bad_request(
            "INVALID_FORMAT",
            format!("Format {} is not supported", params.format),
        )),
    }
}

/// Helper to export anchors as CSV
fn export_anchors_csv(anchors: &[crate::models::Anchor]) -> ApiResult<impl IntoResponse> {