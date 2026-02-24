use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use chrono::{DateTime, Utc, NaiveDate, Duration};
use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: String, // "csv", "json", "excel"
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

/// Helper method to define start and end dates when not fully provided
fn get_date_range(query: &ExportQuery) -> (NaiveDate, NaiveDate) {
    let today = Utc::now().date_naive();
    let end = query.end_date.unwrap_or(today);
    // default to 30 days if no start date provided
    let start = query.start_date.unwrap_or(end - Duration::days(30));
    (start, end)
}

/// Generate response helper
fn export_response(
    data_bytes: Vec<u8>,
    format: &str,
    filename_prefix: &str,
) -> impl IntoResponse {
    let content_type = match format {
        "csv" => "text/csv",
        "excel" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "json" => "application/json",
        _ => "application/octet-stream",
    };

    let extension = match format {
        "csv" => "csv",
        "excel" => "xlsx",
        "json" => "json",
        _ => "bin",
    };

    let filename = format!("{}_{}.{}", filename_prefix, Utc::now().format("%Y%m%d_%H%M%S"), extension);

    let mut resp = data_bytes.into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_str(content_type).unwrap(),
    );
    resp.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename)).unwrap(),
    );
    resp
}

#[derive(Serialize)]
pub struct ExportCorridorMetrics {
    pub corridor_key: String,
    pub asset_a_code: String,
    pub asset_a_issuer: String,
    pub asset_b_code: String,
    pub asset_b_issuer: String,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_success_rate: f64,
    pub total_volume_usd: f64,
}

/// GET /api/export/corridors
pub async fn export_corridors(
    State(app_state): State<AppState>,
    Query(params): Query<ExportQuery>,
) -> ApiResult<impl IntoResponse> {
    let (start_date, end_date) = get_date_range(&params);

    // Fetch aggregate metrics for corridors within date range
    let metrics = app_state
        .db
        .corridor_aggregates()
        .get_aggregated_corridor_metrics(start_date, end_date)
        .await
        .map_err(|e| {
            ApiError::internal(
                "DATABASE_ERROR",
                format!("Failed to fetch corridors for export: {}", e),
            )
        })?;

    // Map to ExportCorridorMetrics safely
    let mapped_metrics: Vec<ExportCorridorMetrics> = metrics
        .into_iter()
        .map(|m| ExportCorridorMetrics {
            corridor_key: m.corridor_key,
            asset_a_code: m.asset_a_code,
            asset_a_issuer: m.asset_a_issuer,
            asset_b_code: m.asset_b_code,
            asset_b_issuer: m.asset_b_issuer,
            total_transactions: m.total_transactions,
            successful_transactions: m.successful_transactions,
            failed_transactions: m.failed_transactions,
            avg_success_rate: m.avg_success_rate,
            total_volume_usd: m.total_volume_usd,
        })
        .collect();

    let format = params.format.to_lowercase();
    let format_str = format.as_str();

    let bytes = match format_str {
        "json" => {
            serde_json::to_vec_pretty(&mapped_metrics).map_err(|e| {
                ApiError::internal("EXPORT_ERROR", format!("Failed to serialize JSON: {}", e))
            })?
        }
        "csv" => {
            let mut wtr = csv::Writer::from_writer(vec![]);
            for metric in &mapped_metrics {
                wtr.serialize(metric).map_err(|e| {
                    ApiError::internal("EXPORT_ERROR", format!("Failed to write CSV: {}", e))
                })?;
            }
            wtr.into_inner().map_err(|e| {
                ApiError::internal("EXPORT_ERROR", format!("Failed to finalize CSV: {}", e))
            })?
        }
        "excel" => {
            let mut workbook = Workbook::new();
            let worksheet = workbook.add_worksheet();

            // Write headers
            worksheet.write_string(0, 0, "Corridor Key").unwrap();
            worksheet.write_string(0, 1, "Asset A Code").unwrap();
            worksheet.write_string(0, 2, "Asset B Code").unwrap();
            worksheet.write_string(0, 3, "Total Transactions").unwrap();
            worksheet.write_string(0, 4, "Successful Transactions").unwrap();
            worksheet.write_string(0, 5, "Failed Transactions").unwrap();
            worksheet.write_string(0, 6, "Average Success Rate").unwrap();
            worksheet.write_string(0, 7, "Total Volume USD").unwrap();

            // Write data
            for (i, m) in mapped_metrics.iter().enumerate() {
                let row = (i + 1) as u32;
                worksheet.write_string(row, 0, &m.corridor_key).unwrap();
                worksheet.write_string(row, 1, &m.asset_a_code).unwrap();
                worksheet.write_string(row, 2, &m.asset_b_code).unwrap();
                worksheet.write_number(row, 3, m.total_transactions as f64).unwrap();
                worksheet.write_number(row, 4, m.successful_transactions as f64).unwrap();
                worksheet.write_number(row, 5, m.failed_transactions as f64).unwrap();
                worksheet.write_number(row, 6, m.avg_success_rate).unwrap();
                worksheet.write_number(row, 7, m.total_volume_usd).unwrap();
            }

            workbook.save_to_buffer().map_err(|e| {
                ApiError::internal("EXPORT_ERROR", format!("Failed to generate Excel: {}", e))
            })?
        }
        _ => {
            return Err(ApiError::bad_request(
                "INVALID_FORMAT",
                "Supported formats are: csv, json, excel",
            ));
        }
    };

    Ok(export_response(bytes, format_str, "corridors_export"))
}

/// GET /api/export/anchors
pub async fn export_anchors(
    State(app_state): State<AppState>,
    Query(params): Query<ExportQuery>,
) -> ApiResult<impl IntoResponse> {
    // For anchors, we can just export the current active anchors and their metrics
    let anchors = app_state.db.list_anchors(2000, 0).await.map_err(|e| {
        ApiError::internal(
            "DATABASE_ERROR",
            format!("Failed to fetch anchors for export: {}", e),
        )
    })?;

    let format = params.format.to_lowercase();
    let format_str = format.as_str();

    let bytes = match format_str {
        "json" => {
            serde_json::to_vec_pretty(&anchors).map_err(|e| {
                ApiError::internal("EXPORT_ERROR", format!("Failed to serialize JSON: {}", e))
            })?
        }
        "csv" => {
            let mut wtr = csv::Writer::from_writer(vec![]);
            for anchor in &anchors {
                wtr.serialize(anchor).map_err(|e| {
                    ApiError::internal("EXPORT_ERROR", format!("Failed to write CSV: {}", e))
                })?;
            }
            wtr.into_inner().map_err(|e| {
                ApiError::internal("EXPORT_ERROR", format!("Failed to finalize CSV: {}", e))
            })?
        }
        "excel" => {
            let mut workbook = Workbook::new();
            let worksheet = workbook.add_worksheet();

            worksheet.write_string(0, 0, "ID").unwrap();
            worksheet.write_string(0, 1, "Name").unwrap();
            worksheet.write_string(0, 2, "Home Domain").unwrap();
            worksheet.write_string(0, 3, "Account").unwrap();
            worksheet.write_string(0, 4, "Status").unwrap();
            worksheet.write_string(0, 5, "Total Volume USD").unwrap();

            for (i, a) in anchors.iter().enumerate() {
                let row = (i + 1) as u32;
                worksheet.write_string(row, 0, &a.id).unwrap();
                worksheet.write_string(row, 1, &a.name).unwrap();
                worksheet.write_string(row, 2, a.home_domain.as_deref().unwrap_or("")).unwrap();
                worksheet.write_string(row, 3, &a.stellar_account).unwrap();
                worksheet.write_string(row, 4, &a.status).unwrap();
                worksheet.write_number(row, 5, a.total_volume_usd).unwrap();
            }

            workbook.save_to_buffer().map_err(|e| {
                ApiError::internal("EXPORT_ERROR", format!("Failed to generate Excel: {}", e))
            })?
        }
        _ => {
            return Err(ApiError::bad_request(
                "INVALID_FORMAT",
                "Supported formats are: csv, json, excel",
            ));
        }
    };

    Ok(export_response(bytes, format_str, "anchors_export"))
}

pub fn routes(app_state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/corridors", axum::routing::get(export_corridors))
        .route("/anchors", axum::routing::get(export_anchors))
        .with_state(app_state)
}
