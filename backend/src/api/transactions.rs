use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::Sep10User,
    models::{PendingTransaction, PendingTransactionWithSignatures, TransactionResult},
    multisig,
    network::NetworkConfig,
    state::AppState,
};

type HandlerError = (StatusCode, String);

/// The caller's wallet, proven by the SEP-10 session the routes require.
fn wallet(user: Option<Extension<Sep10User>>) -> Result<String, HandlerError> {
    user.map(|Extension(u)| u.account).ok_or((
        StatusCode::UNAUTHORIZED,
        "A SEP-10 wallet session is required".to_string(),
    ))
}

fn bad_request(e: impl std::fmt::Display) -> HandlerError {
    (StatusCode::BAD_REQUEST, e.to_string())
}

const DEFAULT_PAGE_LIMIT: i64 = 20;
const MAX_PAGE_LIMIT: i64 = 100;

// Request/Response DTOs
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateTransactionRequest {
    pub source_account: String,
    pub xdr: String,
    pub required_signatures: i32,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AddSignatureRequest {
    pub signer: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct ListTransactionsQuery {
    /// Optional source_account filter.
    pub account: Option<String>,
    /// Opaque cursor returned by a previous page response.
    pub cursor: Option<String>,
    /// Maximum number of results (1–100, default 20).
    pub limit: Option<i64>,
}

/// Internal structure encoded inside the opaque cursor token.
///
/// Encoding the account filter into the cursor guarantees that changing the
/// filter mid-pagination is detected and rejected with 400, preventing the
/// sparse-skip bug where `id > last_id` jumps over rows not visible to the
/// new filter.
#[derive(Debug, Serialize, Deserialize)]
struct TransactionCursor {
    /// The account filter that was active when this cursor was issued.
    account: Option<String>,
    /// The `id` of the last row returned on the previous page.
    last_id: String,
}

impl TransactionCursor {
    fn encode(&self) -> String {
        let json = serde_json::to_vec(self).expect("TransactionCursor is always serialisable");
        BASE64.encode(json)
    }

    fn decode(token: &str) -> Result<Self, &'static str> {
        let bytes = BASE64
            .decode(token)
            .map_err(|_| "cursor is not valid base64")?;
        serde_json::from_slice(&bytes).map_err(|_| "cursor payload is not valid JSON")
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListTransactionsResponse {
    pub data: Vec<PendingTransaction>,
    /// Opaque token to pass as `cursor` to retrieve the next page.
    /// `null` when there are no more results.
    pub next_cursor: Option<String>,
}

// Routes
/// Multi-signature coordination. Every route needs a SEP-10 wallet session:
/// wallets create and submit their own transactions and add only their own,
/// verified, signatures.
pub fn routes(
    state: AppState,
    sep10_service: std::sync::Arc<crate::auth::sep10_simple::Sep10Service>,
) -> Router {
    Router::new()
        .route("/", get(list_transactions).post(create_transaction))
        .route("/{id}", get(get_transaction))
        .route("/{id}/signatures", post(add_signature))
        .route("/{id}/submit", post(submit_transaction))
        .layer(axum::middleware::from_fn_with_state(
            sep10_service,
            crate::auth::sep10_auth_middleware,
        ))
        .with_state(state)
}

// Handlers

/// GET /api/transactions - List pending transactions with cursor pagination
///
/// The cursor is an opaque base64-encoded JSON token that includes the active
/// account filter. Changing the `account` filter between pages will be detected
/// and rejected with 400 Bad Request, preventing the sparse-skip bug where
/// using a global `id` cursor with a filtered query skips rows.
#[utoipa::path(
    get,
    path = "/api/transactions",
    params(
        ("account" = Option<String>, Query, description = "Filter by source account"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor from a previous response"),
        ("limit" = Option<i64>, Query, description = "Maximum results (1-100, default 20)")
    ),
    responses(
        (status = 200, description = "Paginated list of pending transactions", body = ListTransactionsResponse),
        (status = 400, description = "Cursor/filter mismatch or invalid cursor"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transactions"
)]
pub async fn list_transactions(
    State(state): State<AppState>,
    user: Option<Extension<Sep10User>>,
    Query(mut query): Query<ListTransactionsQuery>,
) -> Result<Json<ListTransactionsResponse>, (StatusCode, String)> {
    // A wallet lists only the transactions it created.
    query.account = Some(wallet(user)?);
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .clamp(1, MAX_PAGE_LIMIT);

    // Decode cursor and validate that the embedded filter matches this request.
    let after_id: Option<String> = match query.cursor.as_deref() {
        None => None,
        Some(token) => {
            let decoded = TransactionCursor::decode(token)
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

            if decoded.account != query.account {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "cursor was issued for a different account filter; start a new query"
                        .to_string(),
                ));
            }
            Some(decoded.last_id)
        }
    };

    // Fetch one extra row to detect whether a next page exists.
    let mut rows = state
        .db
        .list_pending_transactions(query.account.as_deref(), after_id.as_deref(), limit + 1)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list transactions: {}", e);
            (StatusCode::BAD_REQUEST, e.to_string())
        })?;

    let next_cursor = if rows.len() as i64 > limit {
        rows.truncate(limit as usize);
        rows.last().map(|row| {
            TransactionCursor {
                account: query.account.clone(),
                last_id: row.id.clone(),
            }
            .encode()
        })
    } else {
        None
    };

    Ok(Json(ListTransactionsResponse {
        data: rows,
        next_cursor,
    }))
}

/// POST /api/transactions - Create a new pending transaction
#[utoipa::path(
    post,
    path = "/api/transactions",
    request_body = CreateTransactionRequest,
    responses(
        (status = 200, description = "Transaction created", body = PendingTransaction),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transactions"
)]
pub async fn create_transaction(
    State(state): State<AppState>,
    user: Option<Extension<Sep10User>>,
    Json(req): Json<CreateTransactionRequest>,
) -> Result<Json<PendingTransaction>, (StatusCode, String)> {
    let wallet = wallet(user)?;
    if req.source_account != wallet {
        return Err((
            StatusCode::FORBIDDEN,
            "source_account must be the signed-in wallet".to_string(),
        ));
    }
    let tx_source = multisig::source_account(&req.xdr).map_err(bad_request)?;
    if tx_source != wallet {
        return Err(bad_request(
            "the transaction's source account is not the signed-in wallet",
        ));
    }
    if !(1..=20).contains(&req.required_signatures) {
        return Err(bad_request("required_signatures must be between 1 and 20"));
    }

    let pending_transaction = state
        .db
        .create_pending_transaction(&req.source_account, &req.xdr, req.required_signatures)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create transaction: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    Ok(Json(pending_transaction))
}

/// GET /api/transactions/{id} - Get a pending transaction by ID
#[utoipa::path(
    get,
    path = "/api/transactions/{id}",
    params(
        ("id" = String, Path, description = "Transaction ID")
    ),
    responses(
        (status = 200, description = "Transaction details", body = PendingTransactionWithSignatures),
        (status = 404, description = "Transaction not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transactions"
)]
pub async fn get_transaction(
    State(state): State<AppState>,
    user: Option<Extension<Sep10User>>,
    Path(id): Path<String>,
) -> Result<Json<PendingTransactionWithSignatures>, (StatusCode, String)> {
    // Any signed-in wallet may read a transaction so co-signers can review it
    // before signing; nothing in it is secret once submitted.
    wallet(user)?;
    let pending_transaction = state.db.get_pending_transaction(&id).await.map_err(|e| {
        tracing::error!("Failed to get transaction: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    if let Some(transaction_with_signatures) = pending_transaction {
        Ok(Json(transaction_with_signatures))
    } else {
        Err((StatusCode::NOT_FOUND, "Transaction not found".to_string()))
    }
}

/// POST /api/transactions/{id}/signatures - Add a signature to a transaction
#[utoipa::path(
    post,
    path = "/api/transactions/{id}/signatures",
    params(
        ("id" = String, Path, description = "Transaction ID")
    ),
    request_body = AddSignatureRequest,
    responses(
        (status = 201, description = "Signature added"),
        (status = 400, description = "Signature already exists from this signer"),
        (status = 404, description = "Transaction not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transactions"
)]
pub async fn add_signature(
    State(state): State<AppState>,
    user: Option<Extension<Sep10User>>,
    Path(id): Path<String>,
    Json(req): Json<AddSignatureRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let wallet = wallet(user)?;
    if req.signer != wallet {
        return Err((
            StatusCode::FORBIDDEN,
            "a wallet can only add its own signature".to_string(),
        ));
    }
    // Run the duplicate-check, signature insert, and optional status update
    // inside a single transaction to prevent races between concurrent signers.
    let mut tx = state.db.pool().begin().await.map_err(|e| {
        tracing::error!("Failed to begin transaction: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    // Re-read the transaction and its signatures inside the transaction so
    // the duplicate check and the insert are serialised.
    let pending = sqlx::query_as::<_, crate::models::PendingTransaction>(
        "SELECT * FROM pending_transactions WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?
    .ok_or((StatusCode::NOT_FOUND, "Transaction not found".to_string()))?;

    let existing_sigs = sqlx::query_as::<_, crate::models::Signature>(
        "SELECT * FROM transaction_signatures WHERE transaction_id = $1",
    )
    .bind(&id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    if existing_sigs.iter().any(|s| s.signer == req.signer) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Signature already exists from this signer".to_string(),
        ));
    }

    // Store only a signature that verifies against this transaction on this
    // network. A raw signature or a wallet-signed envelope is accepted.
    let network = NetworkConfig::from_env();
    let hash = multisig::transaction_hash(&pending.xdr, &network.network_passphrase)
        .map_err(bad_request)?;
    let verified =
        multisig::extract_signature(&req.signature, &req.signer, &hash).map_err(bad_request)?;
    let signature = base64::engine::general_purpose::STANDARD.encode(verified);

    let sig_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transaction_signatures (id, transaction_id, signer, signature) VALUES ($1, $2, $3, $4)",
    )
    .bind(sig_id)
    .bind(&id)
    .bind(&req.signer)
    .bind(&signature)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to add signature: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
    })?;

    // Promote to "ready" if threshold is now met — same transaction.
    let new_sig_count = existing_sigs.len() as i32 + 1;
    if new_sig_count >= pending.required_signatures {
        sqlx::query(
            "UPDATE pending_transactions SET status = 'ready', updated_at = CURRENT_TIMESTAMP WHERE id = $1",
        )
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update transaction status: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
        })?;
    }

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit signature transaction: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    Ok(StatusCode::CREATED)
}

/// POST /api/transactions/{id}/submit - Submit a transaction to the Stellar network
#[utoipa::path(
    post,
    path = "/api/transactions/{id}/submit",
    params(
        ("id" = String, Path, description = "Transaction ID")
    ),
    responses(
        (status = 200, description = "Transaction submitted", body = TransactionResult),
        (status = 400, description = "Not enough signatures"),
        (status = 404, description = "Transaction not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transactions"
)]
pub async fn submit_transaction(
    State(state): State<AppState>,
    user: Option<Extension<Sep10User>>,
    Path(id): Path<String>,
) -> Result<Json<TransactionResult>, (StatusCode, String)> {
    let wallet = wallet(user)?;
    let tx_opt = state.db.get_pending_transaction(&id).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    let tx_with_sigs =
        tx_opt.ok_or((StatusCode::NOT_FOUND, "Transaction not found".to_string()))?;
    if tx_with_sigs.transaction.source_account != wallet {
        return Err((
            StatusCode::FORBIDDEN,
            "only the transaction's creator can submit it".to_string(),
        ));
    }
    if tx_with_sigs.transaction.status == "submitted" {
        return Err((
            StatusCode::CONFLICT,
            "Transaction was already submitted".to_string(),
        ));
    }
    if (tx_with_sigs.collected_signatures.len() as i32)
        < tx_with_sigs.transaction.required_signatures
    {
        return Err((StatusCode::BAD_REQUEST, "Not enough signatures".to_string()));
    }

    // Signatures were verified when added; attach them and submit for real.
    // (This used to return a random "success" hash without submitting.)
    let mut signatures = Vec::with_capacity(tx_with_sigs.collected_signatures.len());
    for collected in &tx_with_sigs.collected_signatures {
        let bytes: [u8; 64] = base64::engine::general_purpose::STANDARD
            .decode(&collected.signature)
            .ok()
            .and_then(|raw| raw.try_into().ok())
            .ok_or_else(|| {
                bad_request(format!(
                    "stored signature by {} is malformed",
                    collected.signer
                ))
            })?;
        signatures.push((collected.signer.clone(), bytes));
    }
    let envelope =
        multisig::assemble(&tx_with_sigs.transaction.xdr, &signatures).map_err(bad_request)?;

    let network = NetworkConfig::from_env();
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();
    let submission = multisig::submit(&http, &network.horizon_url, &envelope)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let status = if submission.successful {
        "submitted"
    } else {
        "failed"
    };
    state.db.update_transaction_status(&id, status).await.ok();

    if let Some(error) = submission.error {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("Horizon rejected the transaction: {error}"),
        ));
    }
    Ok(Json(TransactionResult {
        hash: submission.hash,
        status: if submission.successful {
            "success"
        } else {
            "failed"
        }
        .to_string(),
    }))
}
