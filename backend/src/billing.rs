//! Paid API key tier, settled in USDC on Stellar.
//!
//! Flow:
//! 1. The key's owner (signed in over SEP-10) asks for an invoice. It names
//!    the treasury account, the USDC asset, the amount and a unique text memo.
//! 2. They send that payment from any wallet.
//! 3. They submit the transaction hash. The transaction and its operations are
//!    read from Horizon and checked against the invoice; if they match, the
//!    invoice is marked paid and the key's subscription is extended.
//!
//! Nothing here holds keys or moves funds; it only reads the public ledger.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::str::FromStr;
use uuid::Uuid;

/// Circle's USDC issuer on the public network.
pub const MAINNET_USDC_ISSUER: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
/// Circle's USDC issuer on testnet.
pub const TESTNET_USDC_ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

const PLAN_NAME: &str = "pro";
const DEFAULT_PRICE_USDC: &str = "50";
const DEFAULT_PERIOD_DAYS: i64 = 30;
const DEFAULT_LIMIT_PER_MINUTE: i64 = 1_000;
/// How long an unpaid invoice stays payable.
const INVOICE_TTL_HOURS: i64 = 24;

/// Pricing and payment destination, from the environment.
#[derive(Debug, Clone, Serialize)]
pub struct BillingConfig {
    /// Account that receives payments.
    pub treasury: String,
    pub asset_code: String,
    pub asset_issuer: String,
    /// Price per period, in USDC.
    pub price_usdc: Decimal,
    pub period_days: i64,
    pub limit_per_minute: i64,
    #[serde(skip)]
    pub horizon_url: String,
}

impl BillingConfig {
    /// `None` when `PAYRAIDER_TREASURY_ACCOUNT` is unset, which disables the
    /// paid tier. Panics are avoided: an invalid value is logged and also
    /// disables billing rather than accepting payments to a bad address.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let treasury = std::env::var("PAYRAIDER_TREASURY_ACCOUNT").ok()?;
        let treasury = treasury.trim().to_string();
        if stellar_strkey::ed25519::PublicKey::from_string(&treasury).is_err() {
            tracing::error!(
                "PAYRAIDER_TREASURY_ACCOUNT is not a valid Stellar account; billing disabled"
            );
            return None;
        }

        let network = std::env::var("STELLAR_NETWORK").unwrap_or_else(|_| "mainnet".to_string());
        let is_mainnet = !network.eq_ignore_ascii_case("testnet");
        let default_issuer = if is_mainnet {
            MAINNET_USDC_ISSUER
        } else {
            TESTNET_USDC_ISSUER
        };
        let default_horizon = if is_mainnet {
            "https://horizon.stellar.org"
        } else {
            "https://horizon-testnet.stellar.org"
        };
        let horizon_env = if is_mainnet {
            "STELLAR_HORIZON_URL_MAINNET"
        } else {
            "STELLAR_HORIZON_URL_TESTNET"
        };

        let price = std::env::var("PAYRAIDER_PRO_PRICE_USDC")
            .unwrap_or_else(|_| DEFAULT_PRICE_USDC.to_string());
        let Ok(price_usdc) = Decimal::from_str(price.trim()) else {
            tracing::error!("PAYRAIDER_PRO_PRICE_USDC is not a number; billing disabled");
            return None;
        };
        if price_usdc <= Decimal::ZERO {
            tracing::error!("PAYRAIDER_PRO_PRICE_USDC must be positive; billing disabled");
            return None;
        }

        let int_env = |name: &str, default: i64| {
            std::env::var(name)
                .ok()
                .and_then(|v| v.trim().parse::<i64>().ok())
                .filter(|v| *v > 0)
                .unwrap_or(default)
        };

        Some(Self {
            treasury,
            asset_code: "USDC".to_string(),
            asset_issuer: std::env::var("PAYRAIDER_USDC_ISSUER")
                .unwrap_or_else(|_| default_issuer.to_string()),
            price_usdc,
            period_days: int_env("PAYRAIDER_PRO_PERIOD_DAYS", DEFAULT_PERIOD_DAYS),
            limit_per_minute: int_env("PAYRAIDER_PRO_LIMIT_PER_MINUTE", DEFAULT_LIMIT_PER_MINUTE),
            horizon_url: std::env::var(horizon_env)
                .unwrap_or_else(|_| default_horizon.to_string())
                .trim_end_matches('/')
                .to_string(),
        })
    }
}

/// What a payer needs, and the invoice's state.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Invoice {
    pub id: String,
    pub api_key_id: String,
    pub wallet_address: String,
    pub plan: String,
    /// Amount to send, in USDC.
    pub amount_usdc: String,
    pub asset_code: String,
    pub asset_issuer: String,
    /// Account to send the payment to.
    pub destination: String,
    /// Text memo the payment transaction must carry.
    pub memo: String,
    pub limit_per_minute: i64,
    pub period_days: i64,
    /// `pending`, `paid` or `expired`.
    pub status: String,
    pub transaction_hash: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub paid_at: Option<String>,
}

/// A key's paid tier.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Subscription {
    pub api_key_id: String,
    pub plan: String,
    pub limit_per_minute: i64,
    pub paid_until: String,
    pub updated_at: String,
}

/// Why an invoice could not be created or confirmed. The variants map to
/// HTTP statuses in the API layer.
#[derive(Debug, thiserror::Error)]
pub enum BillingError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
    #[error("could not reach Horizon: {0}")]
    Upstream(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

fn timestamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("invalid timestamp {value}"))?
        .with_timezone(&Utc))
}

/// Short, unambiguous text memo (Stellar text memos are at most 28 bytes).
fn new_memo() -> String {
    let id = Uuid::new_v4().simple().to_string();
    format!("payraider-{}", &id[..12])
}

/// Create a payable invoice for a key the wallet owns.
pub async fn create_invoice(
    pool: &SqlitePool,
    config: &BillingConfig,
    api_key_id: &str,
    wallet_address: &str,
) -> Result<Invoice, BillingError> {
    let owned: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM api_keys WHERE id = ? AND wallet_address = ? AND status = 'active'",
    )
    .bind(api_key_id)
    .bind(wallet_address)
    .fetch_optional(pool)
    .await
    .context("look up API key")?;
    if owned.is_none() {
        return Err(BillingError::NotFound("API key not found".to_string()));
    }

    let now = Utc::now();
    let invoice = Invoice {
        id: Uuid::new_v4().to_string(),
        api_key_id: api_key_id.to_string(),
        wallet_address: wallet_address.to_string(),
        plan: PLAN_NAME.to_string(),
        amount_usdc: config.price_usdc.normalize().to_string(),
        asset_code: config.asset_code.clone(),
        asset_issuer: config.asset_issuer.clone(),
        destination: config.treasury.clone(),
        memo: new_memo(),
        limit_per_minute: config.limit_per_minute,
        period_days: config.period_days,
        status: "pending".to_string(),
        transaction_hash: None,
        created_at: timestamp(now),
        expires_at: timestamp(now + Duration::hours(INVOICE_TTL_HOURS)),
        paid_at: None,
    };

    sqlx::query(
        "INSERT INTO billing_invoices (id, api_key_id, wallet_address, plan, amount_usdc, asset_code,
            asset_issuer, destination, memo, limit_per_minute, period_days, status, created_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&invoice.id)
    .bind(&invoice.api_key_id)
    .bind(&invoice.wallet_address)
    .bind(&invoice.plan)
    .bind(&invoice.amount_usdc)
    .bind(&invoice.asset_code)
    .bind(&invoice.asset_issuer)
    .bind(&invoice.destination)
    .bind(&invoice.memo)
    .bind(invoice.limit_per_minute)
    .bind(invoice.period_days)
    .bind(&invoice.status)
    .bind(&invoice.created_at)
    .bind(&invoice.expires_at)
    .execute(pool)
    .await
    .context("insert invoice")?;

    Ok(invoice)
}

/// A single payment operation as reported by Horizon.
#[derive(Debug, Clone, Deserialize)]
pub struct HorizonPayment {
    #[serde(rename = "type")]
    pub op_type: String,
    #[serde(default)]
    pub asset_code: Option<String>,
    #[serde(default)]
    pub asset_issuer: Option<String>,
    #[serde(default)]
    pub amount: Option<String>,
    #[serde(default)]
    pub destination: Option<String>,
    #[serde(default)]
    pub source_account: Option<String>,
    /// Present on path payments: the asset actually delivered.
    #[serde(default)]
    pub destination_asset_code: Option<String>,
    #[serde(default)]
    pub destination_asset_issuer: Option<String>,
    #[serde(default)]
    pub destination_amount: Option<String>,
}

/// A transaction as reported by Horizon, with its operations embedded.
#[derive(Debug, Clone, Deserialize)]
pub struct HorizonTransaction {
    pub hash: String,
    pub successful: bool,
    #[serde(default)]
    pub memo_type: Option<String>,
    #[serde(default)]
    pub memo: Option<String>,
    #[serde(default)]
    pub operations: Vec<HorizonPayment>,
}

/// The amount a payment operation actually delivers to the destination, in
/// the asset it delivers. Returns `None` when the op is not a payment to the
/// billing account in the configured asset.
fn delivered_amount(
    op: &HorizonPayment,
    config: &BillingConfig,
) -> Option<Decimal> {
    let (code, issuer, amount) = match op.op_type.as_str() {
        "payment" => (
            op.asset_code.as_deref()?,
            op.asset_issuer.as_deref()?,
            op.amount.as_deref()?,
        ),
        "path_payment_strict_send" | "path_payment_strict_receive" => (
            op.destination_asset_code.as_deref()?,
            op.destination_asset_issuer.as_deref()?,
            op.destination_amount.as_deref()?,
        ),
        _ => return None,
    };

    if op.destination.as_deref() != Some(config.treasury.as_str()) {
        return None;
    }
    if code != config.asset_code || issuer != config.asset_issuer {
        return None;
    }
    Decimal::from_str(amount.trim()).ok()
}

/// Verify a payment on the ledger and, if it pays the invoice, mark the
/// invoice paid and extend the key's subscription.
pub async fn confirm_invoice(
    pool: &SqlitePool,
    config: &BillingConfig,
    http: &reqwest::Client,
    invoice_id: &str,
    wallet_address: &str,
    transaction_hash: &str,
) -> Result<(Invoice, Subscription), BillingError> {
    let invoice: Option<Invoice> = sqlx::query_as(
        "SELECT id, api_key_id, wallet_address, plan, amount_usdc, asset_code, asset_issuer,
            destination, memo, limit_per_minute, period_days, status, transaction_hash,
            created_at, expires_at, paid_at
         FROM billing_invoices WHERE id = ?",
    )
    .bind(invoice_id)
    .fetch_optional(pool)
    .await
    .context("load invoice")?;
    let Some(invoice) = invoice else {
        return Err(BillingError::NotFound("invoice not found".to_string()));
    };

    if invoice.wallet_address != wallet_address {
        return Err(BillingError::NotFound("invoice not found".to_string()));
    }
    if invoice.status == "paid" {
        return Err(BillingError::Conflict("invoice already paid".to_string()));
    }
    if invoice.status == "expired" {
        return Err(BillingError::Invalid("invoice expired".to_string()));
    }
    if Utc::now() > parse_timestamp(&invoice.expires_at).context("parse expiry")? {
        return Err(BillingError::Invalid("invoice expired".to_string()));
    }

    // Deny-by-default: the transaction hash must not already be bound to any
    // invoice. A reused hash is a replay attempt.
    let reused: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM billing_invoices WHERE transaction_hash = ? AND id != ?",
    )
    .bind(transaction_hash)
    .bind(&invoice.id)
    .fetch_optional(pool)
    .await
    .context("check transaction hash reuse")?;
    if reused.is_some() {
        return Err(BillingError::Conflict(
            "transaction already used for another invoice".to_string(),
        ));
    }

    let url = format!("{}/transactions/{}", config.horizon_url, transaction_hash);
    let response = http
        .get(&url)
        .send()
        .await
        .map_err(|e| BillingError::Upstream(e.to_string()))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(BillingError::Invalid("transaction not found".to_string()));
    }
    if !response.status().is_success() {
        return Err(BillingError::Upstream(format!(
            "Horizon returned {}",
            response.status()
        )));
    }
    let tx: HorizonTransaction = response
        .json()
        .await
        .map_err(|e| BillingError::Upstream(e.to_string()))?;

    if tx.hash != transaction_hash {
        return Err(BillingError::Invalid("transaction hash mismatch".to_string()));
    }
    if !tx.successful {
        return Err(BillingError::Invalid("transaction failed".to_string()));
    }

    // The memo must be a text memo equal to the invoice's memo.
    let memo_matches = tx.memo_type.as_deref() == Some("text")
        && tx.memo.as_deref() == Some(invoice.memo.as_str());
    if !memo_matches {
        return Err(BillingError::Invalid(
            "transaction memo does not match invoice".to_string(),
        ));
    }

    let required = Decimal::from_str(&invoice.amount_usdc).context("parse invoice amount")?;
    let mut delivered = Decimal::ZERO;
    for op in &tx.operations {
        if let Some(amount) = delivered_amount(op, config) {
            delivered += amount;
        }
    }
    if delivered < required {
        return Err(BillingError::Invalid(format!(
            "payment amount {delivered} is less than invoice amount {required}"
        )));
    }

    // Bind the hash to the invoice atomically; a concurrent confirm loses.
    let now = Utc::now();
    let paid_at = timestamp(now);
    let updated = sqlx::query(
        "UPDATE billing_invoices SET status = 'paid', transaction_hash = ?, paid_at = ?
         WHERE id = ? AND status = 'pending'",
    )
    .bind(transaction_hash)
    .bind(&paid_at)
    .bind(&invoice.id)
    .execute(pool)
    .await
    .context("mark invoice paid")?;
    if updated.rows_affected() == 0 {
        return Err(BillingError::Conflict("invoice already paid".to_string()));
    }

    let paid_until = timestamp(now + Duration::days(invoice.period_days));
    sqlx::query(
        "INSERT INTO subscriptions (api_key_id, plan, limit_per_minute, paid_until, updated_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(api_key_id) DO UPDATE SET
            plan = excluded.plan,
            limit_per_minute = excluded.limit_per_minute,
            paid_until = excluded.paid_until,
            updated_at = excluded.updated_at",
    )
    .bind(&invoice.api_key_id)
    .bind(&invoice.plan)
    .bind(invoice.limit_per_minute)
    .bind(&paid_until)
    .bind(&paid_at)
    .execute(pool)
    .await
    .context("extend subscription")?;

    let subscription: Subscription = sqlx::query_as(
        "SELECT api_key_id, plan, limit_per_minute, paid_until, updated_at
         FROM subscriptions WHERE api_key_id = ?",
    )
    .bind(&invoice.api_key_id)
    .fetch_one(pool)
    .await
    .context("load subscription")?;

    let mut paid_invoice = invoice;
    paid_invoice.status = "paid".to_string();
    paid_invoice.transaction_hash = Some(transaction_hash.to_string());
    paid_invoice.paid_at = Some(paid_at);

    Ok((paid_invoice, subscription))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> BillingConfig {
        BillingConfig {
            treasury: "GTREASURYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            asset_code: "USDC".to_string(),
            asset_issuer: MAINNET_USDC_ISSUER.to_string(),
            price_usdc: Decimal::from_str("50").unwrap(),
            period_days: 30,
            limit_per_minute: 1_000,
            horizon_url: "https://horizon.stellar.org".to_string(),
        }
    }

    fn payment_op() -> HorizonPayment {
        HorizonPayment {
            op_type: "payment".to_string(),
            asset_code: Some("USDC".to_string()),
            asset_issuer: Some(MAINNET_USDC_ISSUER.to_string()),
            amount: Some("50".to_string()),
            destination: Some("GTREASURYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string()),
            source_account: None,
            destination_asset_code: None,
            destination_asset_issuer: None,
            destination_amount: None,
        }
    }

    #[test]
    fn accepts_matching_payment() {
        let cfg = config();
        let op = payment_op();
        assert_eq!(delivered_amount(&op, &cfg), Some(Decimal::from_str("50").unwrap()));
    }

    #[test]
    fn rejects_wrong_issuer() {
        let cfg = config();
        let mut op = payment_op();
        op.asset_issuer = Some(TESTNET_USDC_ISSUER.to_string());
        assert_eq!(delivered_amount(&op, &cfg), None);
    }

    #[test]
    fn rejects_fake_usdc_code() {
        let cfg = config();
        let mut op = payment_op();
        op.asset_code = Some("FAKEUSDC".to_string());
        assert_eq!(delivered_amount(&op, &cfg), None);
    }

    #[test]
    fn rejects_wrong_destination() {
        let cfg = config();
        let mut op = payment_op();
        op.destination = Some("GOTHERAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string());
        assert_eq!(delivered_amount(&op, &cfg), None);
    }

    #[test]
    fn rejects_non_payment_op() {
        let cfg = config();
        let mut op = payment_op();
        op.op_type = "create_account".to_string();
        assert_eq!(delivered_amount(&op, &cfg), None);
    }

    #[test]
    fn accepts_path_payment_delivering_usdc() {
        let cfg = config();
        let op = HorizonPayment {
            op_type: "path_payment_strict_send".to_string(),
            asset_code: Some("XLM".to_string()),
            asset_issuer: None,
            amount: Some("100".to_string()),
            destination: Some("GTREASURYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string()),
            source_account: None,
            destination_asset_code: Some("USDC".to_string()),
            destination_asset_issuer: Some(MAINNET_USDC_ISSUER.to_string()),
            destination_amount: Some("50".to_string()),
        };
        assert_eq!(delivered_amount(&op, &cfg), Some(Decimal::from_str("50").unwrap()));
    }

    #[test]
    fn rejects_path_payment_with_wrong_destination_asset() {
        let cfg = config();
        let op = HorizonPayment {
            op_type: "path_payment_strict_send".to_string(),
            asset_code: Some("XLM".to_string()),
            asset_issuer: None,
            amount: Some("100".to_string()),
            destination: Some("GTREASURYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string()),
            source_account: None,
            destination_asset_code: Some("USDC".to_string()),
            destination_asset_issuer: Some(TESTNET_USDC_ISSUER.to_string()),
            destination_amount: Some("50".to_string()),
        };
        assert_eq!(delivered_amount(&op, &cfg), None);
    }
}
