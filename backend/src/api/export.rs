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

/// Export corridors as CSV, JSON or Excel
#[utoipa::path(
    get,
    path = "/api/v1/export/corridors",
    params(ExportQuery),
    responses(
        (status = 200, description = "Exported corridors file (content type depends on `format`)"),
        (status = 400, description = "Unsupported export format"),
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

    match params.format.to_lowercase().as_str() {
        "csv" => export_corridors_csv(&filtered_corridors).map(IntoResponse::into_response),
        "json" => export_corridors_json(&filtered_corridors).map(IntoResponse::into_response),
        "excel" | "xlsx" => {
            export_corridors_excel(&filtered_corridors).map(IntoResponse::into_response)
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
        (status = 500, description = "Internal server error")
    ),
    tag = "Export"
)]
pub async fn export_anchors(
    State(app_state): State<AppState>,
    Query(params): Query<ExportQuery>,
) -> ApiResult<Response> {
    let anchors = app_state.db.list_anchors(1000, 0).await.map_err(|e| {
        ApiError::internal(
            "DATABASE_ERROR",
            format!("Failed to fetch anchors for export: {e}"),
        )
    })?;

    match params.format.to_lowercase().as_str() {
        "csv" => export_anchors_csv(&anchors).map(IntoResponse::into_response),
        "json" => export_anchors_json(&anchors).map(IntoResponse::into_response),
        "excel" | "xlsx" => export_anchors_excel(&anchors).map(IntoResponse::into_response),
        _ => Err(ApiError::bad_request(
            "INVALID_FORMAT",
            format!("Format {} is not supported", params.format),
        )),
    }
}

/// Helper to export anchors as CSV
fn export_anchors_csv(anchors: &[crate::models::Anchor]) -> ApiResult<impl IntoResponse> {
    let mut wtr = Writer::from_writer(vec![]);
    wtr.write_record([
        "Anchor ID",
        "Name",
        "Stellar Account",
        "Home Domain",
        "Reliability Score (%)",
        "Total Transactions",
        "Successful Transactions",
        "Failed Transactions",
        "Volume (USD)",
        "Status",
        "Last Updated",
    ])
    .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;

    for a in anchors {
        wtr.write_record(&[
            sanitize_csv_field(a.id.clone()),
            sanitize_csv_field(a.name.clone()),
            sanitize_csv_field(a.stellar_account.clone()),
            sanitize_csv_field(a.home_domain.clone().unwrap_or_default()),
            format!("{:.2}", a.reliability_score),
            a.total_transactions.to_string(),
            a.successful_transactions.to_string(),
            a.failed_transactions.to_string(),
            format!("{:.2}", a.total_volume_usd),
            sanitize_csv_field(a.status.clone()),
            a.updated_at.to_rfc3339(),
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
        HeaderValue::from_static("attachment; filename=\"anchors_export.csv\""),
    );

    Ok((headers, data))
}

/// Helper to export anchors as JSON
fn export_anchors_json(anchors: &[crate::models::Anchor]) -> ApiResult<impl IntoResponse> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"anchors_export.json\""),
    );

    let data = serde_json::to_vec(anchors)
        .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
    Ok((headers, data))
}

/// Helper to export anchors as Excel
fn export_anchors_excel(anchors: &[crate::models::Anchor]) -> ApiResult<impl IntoResponse> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x00D9_EAD3));

    let headers = [
        "Anchor ID",
        "Name",
        "Stellar Account",
        "Home Domain",
        "Reliability Score (%)",
        "Total Transactions",
        "Successful Transactions",
        "Failed Transactions",
        "Volume (USD)",
        "Status",
        "Last Updated",
    ];

    for (i, h) in headers.iter().enumerate() {
        worksheet
            .write_with_format(0, i as u16, *h, &header_format)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
    }

    for (row, a) in anchors.iter().enumerate() {
        let row = (row + 1) as u32;
        worksheet
            .write(row, 0, &a.id)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 1, &a.name)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 2, &a.stellar_account)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 3, a.home_domain.as_deref().unwrap_or(""))
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 4, a.reliability_score)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 5, a.total_transactions as f64)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 6, a.successful_transactions as f64)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 7, a.failed_transactions as f64)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 8, a.total_volume_usd)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 9, &a.status)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 10, a.updated_at.to_rfc3339())
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
        HeaderValue::from_static("attachment; filename=\"anchors_export.xlsx\""),
    );

    Ok((headers, data))
}

/// Export payments as CSV, JSON or Excel
#[utoipa::path(
    get,
    path = "/api/v1/export/payments",
    params(ExportQuery),
    responses(
        (status = 200, description = "Exported payments file (content type depends on `format`)"),
        (status = 400, description = "Unsupported export format"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Export"
)]
pub async fn export_payments(
    State(app_state): State<AppState>,
    Query(params): Query<ExportQuery>,
) -> ApiResult<Response> {
    let start_date = params
        .start_date
        .unwrap_or_else(|| Utc::now() - Duration::days(30));
    let end_date = params.end_date.unwrap_or_else(Utc::now);

    validate_export_range(start_date.date_naive(), end_date.date_naive())?;

    // CSV is the format most exposed to row-count blow-up (JSON/Excel below
    // still fetch the bounded `LIMIT MAX_EXPORT_ROWS` set into memory, since
    // `rust_xlsxwriter` and `serde_json::to_vec` both need the whole value
    // up front) — stream it straight from the DB cursor instead of
    // materializing a `Vec<PaymentRow>` and then a second, fully-buffered
    // CSV byte vector on top of it (#2669).
    if params.format.to_lowercase() == "csv" {
        return export_payments_csv_stream(app_state.db.pool().clone(), start_date, end_date).await;
    }

    let payments = sqlx::query_as::<_, PaymentRow>(
        r"
        SELECT * FROM payments
        WHERE created_at BETWEEN $1 AND $2
        ORDER BY created_at DESC
        LIMIT $3
        ",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(MAX_EXPORT_ROWS as i64)
    .fetch_all(app_state.db.pool())
    .await
    .map_err(|e| {
        ApiError::internal(
            "DATABASE_ERROR",
            format!("Failed to fetch payments for export: {e}"),
        )
    })?;

    match params.format.to_lowercase().as_str() {
        "json" => export_payments_json(&payments).map(IntoResponse::into_response),
        "excel" | "xlsx" => export_payments_excel(&payments).map(IntoResponse::into_response),
        _ => Err(ApiError::bad_request(
            "INVALID_FORMAT",
            format!("Format {} is not supported", params.format),
        )),
    }
}

/// Encode one payment row as a single CSV record (same column order as
/// `export_payments_csv_stream`'s header), reusing the `csv` crate's writer
/// so each streamed row gets the same quoting/escaping `export_corridors_csv`
/// relies on for its (non-streamed) sibling export.
fn payment_csv_row_bytes(p: &PaymentRow) -> ApiResult<Vec<u8>> {
    let mut wtr = Writer::from_writer(vec![]);
    wtr.write_record(&[
        sanitize_csv_field(p.transaction_hash.clone()),
        sanitize_csv_field(p.source_account.clone()),
        sanitize_csv_field(p.destination_account.clone()),
        sanitize_csv_field(format!("{}:{}", p.source_asset_code, p.source_asset_issuer)),
        sanitize_csv_field(format!(
            "{}:{}",
            p.destination_asset_code, p.destination_asset_issuer
        )),
        p.amount.to_string(),
        p.successful.to_string(),
        p.created_at.to_rfc3339(),
    ])
    .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
    wtr.into_inner()
        .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))
}

/// Stream payments as CSV directly from a DB cursor (`fetch`, not
/// `fetch_all`): at most one row is held in memory at a time on the way
/// into the response body, rather than the full result set plus a second,
/// fully-buffered CSV byte vector built on top of it (#2669).
async fn export_payments_csv_stream(
    pool: sqlx::SqlitePool,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> ApiResult<Response> {
    let mut header_wtr = Writer::from_writer(vec![]);
    header_wtr
        .write_record([
            "Transaction Hash",
            "Source Account",
            "Destination Account",
            "Source Asset",
            "Destination Asset",
            "Amount",
            "Successful",
            "Timestamp",
        ])
        .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
    let header_bytes = header_wtr
        .into_inner()
        .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;

    // `async_stream::stream!` (same mechanism `graphql/subscription.rs`
    // already uses for its real-time streams) owns `pool` inside the
    // generator for the life of the stream, so the `&pool` borrow used by
    // `.fetch(&pool)` below is valid across every `.await` point without
    // tying this stream's lifetime to anything outside it — required since
    // `Body::from_stream` needs the stream to be `'static`.
    let body_stream = async_stream::stream! {
        yield Ok::<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>(header_bytes);

        let mut rows = sqlx::query_as::<_, PaymentRow>(
            r"
            SELECT * FROM payments
            WHERE created_at BETWEEN $1 AND $2
            ORDER BY created_at DESC
            LIMIT $3
            ",
        )
        .bind(start_date)
        .bind(end_date)
        .bind(MAX_EXPORT_ROWS as i64)
        .fetch(&pool);

        while let Some(row_result) = rows.next().await {
            match row_result {
                Ok(row) => match payment_csv_row_bytes(&row) {
                    Ok(bytes) => yield Ok(bytes),
                    Err(e) => {
                        yield Err(Box::new(std::io::Error::other(e.to_string()))
                            as Box<dyn std::error::Error + Send + Sync>);
                        return;
                    }
                },
                Err(e) => {
                    yield Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
                    return;
                }
            }
        }
    };

    let body = Body::from_stream(body_stream);

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/csv"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"payments_export.csv\""),
    );

    Ok((headers, body).into_response())
}

/// Helper to export payments as JSON
fn export_payments_json(payments: &[PaymentRow]) -> ApiResult<impl IntoResponse> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"payments_export.json\""),
    );

    let data = serde_json::to_vec(payments)
        .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
    Ok((headers, data))
}

/// Helper to export payments as Excel
fn export_payments_excel(payments: &[PaymentRow]) -> ApiResult<impl IntoResponse> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x00D9_EAD3));

    let headers = [
        "Transaction Hash",
        "Source Account",
        "Destination Account",
        "Source Asset",
        "Destination Asset",
        "Amount",
        "Successful",
        "Timestamp",
    ];

    for (i, h) in headers.iter().enumerate() {
        worksheet
            .write_with_format(0, i as u16, *h, &header_format)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
    }

    for (row, p) in payments.iter().enumerate() {
        let row = (row + 1) as u32;
        worksheet
            .write(row, 0, &p.transaction_hash)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 1, &p.source_account)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 2, &p.destination_account)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(
                row,
                3,
                format!("{}:{}", p.source_asset_code, p.source_asset_issuer),
            )
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(
                row,
                4,
                format!(
                    "{}:{}",
                    p.destination_asset_code, p.destination_asset_issuer
                ),
            )
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 5, p.amount)
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 6, if p.successful { "Yes" } else { "No" })
            .map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
        worksheet
            .write(row, 7, p.created_at.to_rfc3339())
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
        HeaderValue::from_static("attachment; filename=\"payments_export.xlsx\""),
    );

    Ok((headers, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_payment(hash: &str, amount: f64) -> PaymentRow {
        let now = Utc::now();
        PaymentRow {
            id: "id-1".to_string(),
            transaction_hash: hash.to_string(),
            source_account: "GALICE".to_string(),
            destination_account: "GBOB".to_string(),
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("USDC".to_string()),
            asset_issuer: Some("GISSUER".to_string()),
            source_asset_code: "USDC".to_string(),
            source_asset_issuer: "GISSUER".to_string(),
            destination_asset_code: "USDC".to_string(),
            destination_asset_issuer: "GISSUER".to_string(),
            amount,
            successful: true,
            timestamp: Some(now),
            submission_time: Some(now),
            confirmation_time: Some(now),
            created_at: now,
        }
    }

    #[test]
    fn validate_export_range_accepts_a_range_within_the_limit() {
        let start = Utc::now().date_naive() - Duration::days(10);
        let end = Utc::now().date_naive();
        assert!(validate_export_range(start, end).is_ok());
    }

    #[test]
    fn validate_export_range_rejects_a_range_wider_than_the_limit() {
        let start = Utc::now().date_naive() - Duration::days(MAX_EXPORT_RANGE_DAYS + 1);
        let end = Utc::now().date_naive();
        assert!(validate_export_range(start, end).is_err());
    }

    #[test]
    fn validate_export_range_accepts_exactly_the_limit() {
        let start = Utc::now().date_naive() - Duration::days(MAX_EXPORT_RANGE_DAYS);
        let end = Utc::now().date_naive();
        assert!(validate_export_range(start, end).is_ok());
    }

    #[test]
    fn validate_export_range_rejects_end_before_start() {
        let start = Utc::now().date_naive();
        let end = start - Duration::days(1);
        assert!(validate_export_range(start, end).is_err());
    }

    #[test]
    fn payment_csv_row_bytes_sanitizes_formula_trigger_characters() {
        // A transaction hash can't start with '=', but this exercises the
        // same `sanitize_csv_field` path real export data goes through for
        // any text field that could begin with a formula trigger.
        let row = test_payment("=cmd|'/c calc'!A0", 42.5);
        let bytes = payment_csv_row_bytes(&row).expect("row encodes");
        let text = String::from_utf8(bytes).expect("valid utf8");
        assert!(text.starts_with("'="), "formula-trigger field must be prefixed: {text}");
    }

    #[test]
    fn payment_csv_row_bytes_includes_all_columns() {
        let row = test_payment("abc123", 100.0);
        let bytes = payment_csv_row_bytes(&row).expect("row encodes");
        let text = String::from_utf8(bytes).expect("valid utf8");
        assert!(text.contains("abc123"));
        assert!(text.contains("GALICE"));
        assert!(text.contains("GBOB"));
        assert!(text.contains("USDC:GISSUER"));
        assert!(text.contains("100"));
        assert!(text.contains("true"));
    }
}
