use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::pagination::PaginatedResponse;
use crate::rpc::{Asset, StellarRpcClient};

/// Horizon's maximum page size.
const MAX_HORIZON_LIMIT: u32 = 200;

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
    pub cursor: Option<String>,
    /// Optional asset code filter (e.g. `USDC` or `XLM`).
    pub asset_code: Option<String>,
    /// Optional status filter (`successful` or `failed`).
    pub status: Option<String>,
    /// Optional free-text search across transaction hash, source, and destination.
    pub search: Option<String>,
    /// Optional sort direction (`desc` default, or `asc`).
    pub sort_order: Option<String>,
}

fn bad_request(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.to_string(),
        }),
    )
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
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
        ("limit" = Option<u32>, Query, description = "Maximum number of payments to return (default 20, max 200)"),
        ("cursor" = Option<String>, Query, description = "Opaque cursor from `pagination.next_cursor`")
    ),
    responses(
        (status = 200, description = "Paginated list of recent payments, newest first (`PaginatedResponse<Payment>`; `total` is null)"),
        (status = 400, description = "Invalid pagination cursor", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "RPC"
)]
#[tracing::instrument(skip(client, uri))]
pub async fn get_payments(
    State(client): State<Arc<StellarRpcClient>>,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
    Query(params): Query<PaginationQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.clamp(1, MAX_HORIZON_LIMIT);
    // Horizon paging tokens are numeric; rejecting anything else also stops the
    // cursor from smuggling extra query parameters into the upstream URL.
    let cursor = params.cursor.as_deref().filter(|c| !c.is_empty());
    if cursor.is_some_and(|c| !c.bytes().all(|b| b.is_ascii_digit())) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "The pagination cursor is invalid".to_string(),
            }),
        ));
    }
    match client.fetch_payments(limit, cursor).await {
        Ok(mut payments) => {
            // Results are newest-first; a full page means older records may remain.
            // Derive next_cursor from the unfiltered Horizon page so cursor
            // advancement stays monotonic even when client filters narrow rows.
            let next_cursor = (payments.len() as u32 >= limit)
                .then(|| payments.last().map(|p| p.paging_token.clone()))
                .flatten();

            if let Some(asset_code) = params
                .asset_code
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let wanted = asset_code.to_ascii_lowercase();
                payments.retain(|p| {
                    let code = if p.asset_type == "native" {
                        "xlm".to_string()
                    } else {
                        p.get_asset_code().unwrap_or_default().to_ascii_lowercase()
                    };
                    code.contains(&wanted)
                });
            }

            if let Some(status) = params
                .status
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                if status.eq_ignore_ascii_case("successful") {
                    payments.retain(crate::rpc::Payment::succeeded);
                } else if status.eq_ignore_ascii_case("failed") {
                    payments.retain(|p| !p.succeeded());
                }
            }

            if let Some(search) = params
                .search
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let q = search.to_ascii_lowercase();
                payments.retain(|p| {
                    p.transaction_hash.to_ascii_lowercase().contains(&q)
                        || p.from
                            .as_deref()
                            .is_some_and(|s| s.to_ascii_lowercase().contains(&q))
                        || p.to
                            .as_deref()
                            .is_some_and(|s| s.to_ascii_lowercase().contains(&q))
                        || p.id.to_ascii_lowercase().contains(&q)
                });
            }

            if params
                .sort_order
                .as_deref()
                .is_some_and(|dir| dir.eq_ignore_ascii_case("asc"))
            {
                payments.reverse();
            }

            Ok(Json(
                PaginatedResponse::from_tokens(payments, i64::from(limit), next_cursor, None)
                    .with_links(&uri),
            ))
        }
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
    // account_id is interpolated into the upstream Horizon URL path, so it
    // must be a well-formed account address (no '/', '?', '..' etc.).
    if crate::validation::validate_stellar_address(&account_id).is_err() {
        return Err(bad_request("The account id is not a valid Stellar address"));
    }
    let limit = params.limit.clamp(1, MAX_HORIZON_LIMIT);
    match client.fetch_account_payments(&account_id, limit).await {
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
    // Same guards as get_payments: Horizon rejects limit > 200 (which we
    // would surface as a 500), and a non-numeric cursor could smuggle extra
    // query parameters into the upstream URL.
    let limit = params.limit.clamp(1, MAX_HORIZON_LIMIT);
    let cursor = params.cursor.as_deref().filter(|c| !c.is_empty());
    if cursor.is_some_and(|c| !c.bytes().all(|b| b.is_ascii_digit() || b == b'-')) {
        return Err(bad_request("The pagination cursor is invalid"));
    }
    match client.fetch_trades(limit, cursor).await {
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
        .fetch_order_book(
            &selling_asset,
            &buying_asset,
            params.limit.clamp(1, MAX_HORIZON_LIMIT),
        )
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
