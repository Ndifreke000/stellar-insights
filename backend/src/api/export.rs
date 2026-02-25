use axum::{
    extract::{Query, State},
    http::{header, StatusCode, HeaderMap},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use csv::Writer;
use rust_xlsxwriter::{Workbook, Format, Color};

use crate::state::AppState;
use crate::error::{ApiError, ApiResult};
use crate::models::{PaymentRecord, Anchor, corridor::CorridorMetrics};

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: String, // "csv", "json", "excel"
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub corridor_id: Option<String>,
}

pub async fn export_corridors(
    State(app_state): State<AppState>,
    Query(params): Query<ExportQuery>,
) -> ApiResult<impl IntoResponse> {
    let today = Utc::now().date_naive();
    let start_date = params.start_date.map(|d| d.date_naive()).unwrap_or(today - Duration::days(30));
    let end_date = params.end_date.map(|d| d.date_naive()).unwrap_or(today);

    let corridors = app_state
        .db
        .corridor_aggregates()
        .get_aggregated_corridor_metrics(start_date, end_date)
        .await
        .map_err(|e| ApiError::internal("DATABASE_ERROR", format!("Failed to fetch corridors for export: {}", e)))?;

    match params.format.to_lowercase().as_str() {
        "csv" => {
            let mut wtr = Writer::from_writer(vec![]);
            wtr.write_record(&[
                "Corridor ID", "Source Asset", "Source Issuer", 
                "Destination Asset", "Destination Issuer", 
                "Success Rate (%)", "Total Transactions", 
                "Successful Transactions", "Failed Transactions", 
                "Volume (USD)", "Latest Date"
            ]).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;

            for m in corridors {
                wtr.write_record(&[
                    m.corridor_key,
                    m.asset_a_code,
                    m.asset_a_issuer,
                    m.asset_b_code,
                    m.asset_b_issuer,
                    format!("{:.2}", m.avg_success_rate),
                    m.total_transactions.to_string(),
                    m.successful_transactions.to_string(),
                    m.failed_transactions.to_string(),
                    format!("{:.2}", m.total_volume_usd),
                    m.latest_date.to_string(),
                ]).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            }

            let data = wtr.into_inner().map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "text/csv".parse().unwrap());
            headers.insert(header::CONTENT_DISPOSITION, "attachment; filename=\"corridors_export.csv\"".parse().unwrap());
            
            Ok((headers, data))
        }
        "json" => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            headers.insert(header::CONTENT_DISPOSITION, "attachment; filename=\"corridors_export.json\"".parse().unwrap());
            
            let data = serde_json::to_vec(&corridors).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            Ok((headers, data))
        }
        "excel" | "xlsx" => {
            let mut workbook = Workbook::new();
            let worksheet = workbook.add_worksheet();
            
            let header_format = Format::new()
                .set_bold()
                .set_background_color(Color::RGB(0xD9EAD3));

            let headers = [
                "Corridor ID", "Source Asset", "Source Issuer", 
                "Destination Asset", "Destination Issuer", 
                "Success Rate (%)", "Total Transactions", 
                "Successful Transactions", "Failed Transactions", 
                "Volume (USD)", "Latest Date"
            ];

            for (i, header_text) in headers.iter().enumerate() {
                worksheet.write_with_format(0, i as u16, *header_text, &header_format).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            }

            for (row, m) in corridors.iter().enumerate() {
                let row = (row + 1) as u32;
                worksheet.write(row, 0, &m.corridor_key).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 1, &m.asset_a_code).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 2, &m.asset_a_issuer).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 3, &m.asset_b_code).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 4, &m.asset_b_issuer).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 5, m.avg_success_rate).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 6, m.total_transactions as f64).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 7, m.successful_transactions as f64).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 8, m.failed_transactions as f64).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 9, m.total_volume_usd).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 10, m.latest_date.to_string()).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            }

            let data = workbook.save_to_buffer().map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".parse().unwrap());
            headers.insert(header::CONTENT_DISPOSITION, "attachment; filename=\"corridors_export.xlsx\"".parse().unwrap());
            
            Ok((headers, data))
        }
        _ => Err(ApiError::bad_request("INVALID_FORMAT", format!("Format {} is not supported", params.format))),
    }
}

pub async fn export_anchors(
    State(app_state): State<AppState>,
    Query(params): Query<ExportQuery>,
) -> ApiResult<impl IntoResponse> {
    let anchors = app_state
        .db
        .list_anchors(1000, 0)
        .await
        .map_err(|e| ApiError::internal("DATABASE_ERROR", format!("Failed to fetch anchors for export: {}", e)))?;

    match params.format.to_lowercase().as_str() {
        "csv" => {
            let mut wtr = Writer::from_writer(vec![]);
            wtr.write_record(&[
                "Anchor ID", "Name", "Stellar Account", "Home Domain",
                "Reliability Score (%)", "Total Transactions", 
                "Successful Transactions", "Failed Transactions", 
                "Volume (USD)", "Status", "Last Updated"
            ]).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;

            for a in anchors {
                wtr.write_record(&[
                    a.id,
                    a.name,
                    a.stellar_account,
                    a.home_domain.unwrap_or_default(),
                    format!("{:.2}", a.reliability_score),
                    a.total_transactions.to_string(),
                    a.successful_transactions.to_string(),
                    a.failed_transactions.to_string(),
                    format!("{:.2}", a.total_volume_usd),
                    a.status,
                    a.updated_at.to_rfc3339(),
                ]).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            }

            let data = wtr.into_inner().map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "text/csv".parse().unwrap());
            headers.insert(header::CONTENT_DISPOSITION, "attachment; filename=\"anchors_export.csv\"".parse().unwrap());
            
            Ok((headers, data))
        }
        "json" => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            headers.insert(header::CONTENT_DISPOSITION, "attachment; filename=\"anchors_export.json\"".parse().unwrap());
            
            let data = serde_json::to_vec(&anchors).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            Ok((headers, data))
        }
        "excel" | "xlsx" => {
            let mut workbook = Workbook::new();
            let worksheet = workbook.add_worksheet();
            
            let header_format = Format::new().set_bold().set_background_color(Color::RGB(0xD9EAD3));

            let headers = [
                "Anchor ID", "Name", "Stellar Account", "Home Domain",
                "Reliability Score (%)", "Total Transactions", 
                "Successful Transactions", "Failed Transactions", 
                "Volume (USD)", "Status", "Last Updated"
            ];

            for (i, header_text) in headers.iter().enumerate() {
                worksheet.write_with_format(0, i as u16, *header_text, &header_format).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            }

            for (row, a) in anchors.iter().enumerate() {
                let row = (row + 1) as u32;
                worksheet.write(row, 0, &a.id).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 1, &a.name).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 2, &a.stellar_account).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 3, a.home_domain.as_deref().unwrap_or("")).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 4, a.reliability_score).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 5, a.total_transactions as f64).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 6, a.successful_transactions as f64).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 7, a.failed_transactions as f64).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 8, a.total_volume_usd).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 9, &a.status).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 10, a.updated_at.to_rfc3339()).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            }

            let data = workbook.save_to_buffer().map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".parse().unwrap());
            headers.insert(header::CONTENT_DISPOSITION, "attachment; filename=\"anchors_export.xlsx\"".parse().unwrap());
            
            Ok((headers, data))
        }
        _ => Err(ApiError::bad_request("INVALID_FORMAT", format!("Format {} is not supported", params.format))),
    }
}

pub async fn export_payments(
    State(app_state): State<AppState>,
    Query(params): Query<ExportQuery>,
) -> ApiResult<impl IntoResponse> {
    // We need a way to fetch payments. Looking at models.rs, PaymentRecord exists.
    // Let's assume there's a list_payments method or we can query it directly.
    // Based on database.rs, it doesn't seem to have list_payments yet.
    // I will implement a quick query here.
    
    let start_date = params.start_date.unwrap_or(Utc::now() - Duration::days(30));
    let end_date = params.end_date.unwrap_or(Utc::now());

    let payments = sqlx::query_as::<_, PaymentRecord>(
        r#"
        SELECT * FROM payments 
        WHERE created_at BETWEEN $1 AND $2
        ORDER BY created_at DESC
        LIMIT 5000
        "#
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(app_state.db.pool())
    .await
    .map_err(|e| ApiError::internal("DATABASE_ERROR", format!("Failed to fetch payments for export: {}", e)))?;

    match params.format.to_lowercase().as_str() {
        "csv" => {
            let mut wtr = Writer::from_writer(vec![]);
            wtr.write_record(&[
                "Transaction Hash", "Source Account", "Destination Account", 
                "Source Asset", "Destination Asset", "Amount", 
                "Successful", "Timestamp"
            ]).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;

            for p in payments {
                wtr.write_record(&[
                    p.transaction_hash,
                    p.source_account,
                    p.destination_account,
                    format!("{}:{}", p.source_asset_code, p.source_asset_issuer),
                    format!("{}:{}", p.destination_asset_code, p.destination_asset_issuer),
                    p.amount.to_string(),
                    p.successful.to_string(),
                    p.created_at.to_rfc3339(),
                ]).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            }

            let data = wtr.into_inner().map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "text/csv".parse().unwrap());
            headers.insert(header::CONTENT_DISPOSITION, "attachment; filename=\"payments_export.csv\"".parse().unwrap());
            
            Ok((headers, data))
        }
        "json" => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            headers.insert(header::CONTENT_DISPOSITION, "attachment; filename=\"payments_export.json\"".parse().unwrap());
            
            let data = serde_json::to_vec(&payments).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            Ok((headers, data))
        }
        "excel" | "xlsx" => {
            let mut workbook = Workbook::new();
            let worksheet = workbook.add_worksheet();
            
            let header_format = Format::new().set_bold().set_background_color(Color::RGB(0xD9EAD3));

            let headers = [
                "Transaction Hash", "Source Account", "Destination Account", 
                "Source Asset", "Destination Asset", "Amount", 
                "Successful", "Timestamp"
            ];

            for (i, header_text) in headers.iter().enumerate() {
                worksheet.write_with_format(0, i as u16, *header_text, &header_format).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            }

            for (row, p) in payments.iter().enumerate() {
                let row = (row + 1) as u32;
                worksheet.write(row, 0, &p.transaction_hash).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 1, &p.source_account).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 2, &p.destination_account).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 3, &format!("{}:{}", p.source_asset_code, p.source_asset_issuer)).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 4, &format!("{}:{}", p.destination_asset_code, p.destination_asset_issuer)).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 5, p.amount).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 6, if p.successful { "Yes" } else { "No" }).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
                worksheet.write(row, 7, p.created_at.to_rfc3339()).map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            }

            let data = workbook.save_to_buffer().map_err(|e| ApiError::internal("EXPORT_ERROR", e.to_string()))?;
            
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".parse().unwrap());
            headers.insert(header::CONTENT_DISPOSITION, "attachment; filename=\"payments_export.xlsx\"".parse().unwrap());
            
            Ok((headers, data))
        }
        _ => Err(ApiError::bad_request("INVALID_FORMAT", format!("Format {} is not supported", params.format))),
    }
}
