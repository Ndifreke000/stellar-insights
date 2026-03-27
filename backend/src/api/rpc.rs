use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::rpc::{Asset, StellarRpcClient};

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
    pub cursor: Option<String>,
}

const fn default_limit() -> u32 {
    20
}

#[derive(Debug, Deserialize)]
pub struct OrderBookQuery {
    pub selling_asset_type: String,
    pub selling_asset_code: Option<String>,
    pub selling_asset_issuer: Option<String>,
    pub buying_asset_type: String,
    pub buying_asset_code: Option<String>,
    pub buying_asset_issuer: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Health check for Stellar RPC
#[utoipa::path(
    get,
    path = "/api/rpc/health",
    responses(
        (status = 200, description = "RPC health status"),
        (status = 503, description = "RPC service unavailable", body = ErrorResponse)
    ),
    tag = "RPC"
)]
#[tracing::instrument(skip(client))]
pub async fn rpc_health_check(
    State(client): State<Arc<StellarRpcClient>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match client.check_health().await {
        Ok(health) => Ok(Json(health)),
        Err(e) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: format!("RPC health check failed: {e}"),
            }),
        )),
    }
}

/// Get latest ledger information
#[utoipa::path(
    get,
    path = "/api/rpc/ledger",
    responses(
        (status = 200, description = "Latest ledger information"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "RPC"
)]
#[tracing::instrument(skip(client))]
pub async fn get_latest_ledger(
    State(client): State<Arc<StellarRpcClient>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match client.fetch_latest_ledger().await {
        Ok(ledger) => Ok(Json(ledger)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to fetch ledger: {e}"),
            }),
        )),
    }
}

/// Get recent payments
#[utoipa::path(
    get,
    path = "/api/rpc/payments",
    params(
        ("limit" = Option<u32>, Query, description = "Maximum number of payments to return (default 20)"),
        ("cursor" = Option<String>, Query, description = "Pagination cursor for next page")
    ),
    responses(
        (status = 200, description = "List of recent payments"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "RPC"
)]
#[tracing::instrument(skip(client))]
pub async fn get_payments(
    State(client): State<Arc<StellarRpcClient>>,
    Query(params): Query<PaginationQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let cursor = params.cursor.as_deref();
    match client.fetch_payments(params.limit, cursor).await {
        Ok(payments) => Ok(Json(payments)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to fetch payments: {e}"),
            }),
        )),
    }
}

/// Get payments for a specific account
#[utoipa::path(
    get,
    path = "/api/rpc/accounts/{account_id}/payments",
    params(
        ("account_id" = String, Path, description = "Stellar account ID"),
        ("limit" = Option<u32>, Query, description = "Maximum number of payments to return (default 20)")
    ),
    responses(
        (status = 200, description = "List of account payments"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "RPC"
)]
#[tracing::instrument(skip(client))]
pub async fn get_account_payments(
    State(client): State<Arc<StellarRpcClient>>,
    Path(account_id): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match client
        .fetch_account_payments(&account_id, params.limit)
        .await
    {
        Ok(payments) => Ok(Json(payments)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to fetch account payments: {e}"),
            }),
        )),
    }
}

/// Get recent trades
#[utoipa::path(
    get,
    path = "/api/rpc/trades",
    params(
        ("limit" = Option<u32>, Query, description = "Maximum number of trades to return (default 20)"),
        ("cursor" = Option<String>, Query, description = "Pagination cursor for next page")
    ),
    responses(
        (status = 200, description = "List of recent trades"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "RPC"
)]
#[tracing::instrument(skip(client))]
pub async fn get_trades(
    State(client): State<Arc<StellarRpcClient>>,
    Query(params): Query<PaginationQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let cursor = params.cursor.as_deref();
    match client.fetch_trades(params.limit, cursor).await {
        Ok(trades) => Ok(Json(trades)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to fetch trades: {e}"),
            }),
        )),
    }
}

/// Get order book for a trading pair
#[utoipa::path(
    get,
    path = "/api/rpc/orderbook",
    params(
        ("selling_asset_type" = String, Query, description = "Selling asset type (e.g., 'native', 'credit_alphanum4')"),
        ("selling_asset_code" = Option<String>, Query, description = "Selling asset code (e.g., 'USDC')"),
        ("selling_asset_issuer" = Option<String>, Query, description = "Selling asset issuer"),
        ("buying_asset_type" = String, Query, description = "Buying asset type"),
        ("buying_asset_code" = Option<String>, Query, description = "Buying asset code"),
        ("buying_asset_issuer" = Option<String>, Query, description = "Buying asset issuer"),
        ("limit" = Option<u32>, Query, description = "Maximum number of price levels to return (default 20)")
    ),
    responses(
        (status = 200, description = "Order book for trading pair"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "RPC"
)]
#[tracing::instrument(skip(client))]
pub async fn get_order_book(
    State(client): State<Arc<StellarRpcClient>>,
    Query(params): Query<OrderBookQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let selling_asset = Asset {
        asset_type: params.selling_asset_type,
        asset_code: params.selling_asset_code,
        asset_issuer: params.selling_asset_issuer,
    };

    let buying_asset = Asset {
        asset_type: params.buying_asset_type,
        asset_code: params.buying_asset_code,
        asset_issuer: params.buying_asset_issuer,
    };

    match client
        .fetch_order_book(&selling_asset, &buying_asset, params.limit)
        .await
    {
        Ok(order_book) => Ok(Json(order_book)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to fetch order book: {e}"),
            }),
        )),
    }
}
