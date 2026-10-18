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

/// Fetch an invoice belonging to the wallet.
pub async fn get_invoice(
    pool: &SqlitePool,
    invoice_id: &str,
    wallet_address: &str,
) -> Result<Invoice, BillingError> {
    sqlx::query_as::<_, Invoice>(
        "SELECT * FROM billing_invoices WHERE id = ? AND wallet_address = ?",
    )
    .bind(invoice_id)
    .bind(wallet_address)
    .fetch_optional(pool)
    .await
    .context("look up invoice")?
    .ok_or_else(|| BillingError::NotFound("Invoice not found".to_string()))
}

/// The key's subscription, if the wallet owns the key and one exists.
pub async fn get_subscription(
    pool: &SqlitePool,
    api_key_id: &str,
    wallet_address: &str,
) -> Result<Option<Subscription>, BillingError> {
    Ok(sqlx::query_as::<_, Subscription>(
        "SELECT s.* FROM api_key_subscriptions s
         JOIN api_keys k ON k.id = s.api_key_id
         WHERE s.api_key_id = ? AND k.wallet_address = ?",
    )
    .bind(api_key_id)
    .bind(wallet_address)
    .fetch_optional(pool)
    .await
    .context("look up subscription")?)
}

/// The per-minute limit a key's active subscription grants, if any.
pub async fn active_limit(pool: &SqlitePool, api_key_id: &str) -> Result<Option<u32>> {
    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT limit_per_minute, paid_until FROM api_key_subscriptions WHERE api_key_id = ?",
    )
    .bind(api_key_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|(limit, paid_until)| {
        let active = parse_timestamp(&paid_until).is_ok_and(|until| until > Utc::now());
        active.then(|| u32::try_from(limit).unwrap_or(u32::MAX))
    }))
}

/// Horizon transaction fields the check needs.
#[derive(Debug, Deserialize)]
pub struct HorizonTransaction {
    pub hash: String,
    pub successful: bool,
    #[serde(default)]
    pub memo_type: String,
    #[serde(default)]
    pub memo: Option<String>,
    pub created_at: String,
}

/// Horizon operation fields the check needs.
#[derive(Debug, Deserialize)]
pub struct HorizonOperation {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub asset_type: Option<String>,
    #[serde(default)]
    pub asset_code: Option<String>,
    #[serde(default)]
    pub asset_issuer: Option<String>,
    #[serde(default)]
    pub amount: Option<String>,
}

/// Does this transaction pay the invoice? Pure, so it is tested without
/// Horizon.
pub fn check_payment(
    invoice: &Invoice,
    transaction: &HorizonTransaction,
    operations: &[HorizonOperation],
) -> Result<(), String> {
    if !transaction.successful {
        return Err("The transaction failed on the ledger".to_string());
    }
    if transaction.memo_type != "text" || transaction.memo.as_deref() != Some(invoice.memo.as_str())
    {
        return Err(format!(
            "The transaction memo must be the text memo \"{}\"",
            invoice.memo
        ));
    }

    // A payment made before the invoice existed cannot be for it.
    let issued = parse_timestamp(&invoice.created_at).map_err(|e| e.to_string())?;
    let made = parse_timestamp(&transaction.created_at).map_err(|e| e.to_string())?;
    if made < issued - Duration::minutes(1) {
        return Err("The transaction predates the invoice".to_string());
    }

    let due = Decimal::from_str(&invoice.amount_usdc).map_err(|e| e.to_string())?;
    let paid: Decimal = operations
        .iter()
        .filter(|op| {
            matches!(
                op.kind.as_str(),
                "payment" | "path_payment_strict_receive" | "path_payment_strict_send"
            )
        })
        .filter(|op| op.to.as_deref() == Some(invoice.destination.as_str()))
        .filter(|op| op.asset_type.as_deref() != Some("native"))
        .filter(|op| {
            op.asset_code.as_deref() == Some(invoice.asset_code.as_str())
                && op.asset_issuer.as_deref() == Some(invoice.asset_issuer.as_str())
        })
        .filter_map(|op| op.amount.as_deref().and_then(|a| Decimal::from_str(a).ok()))
        .sum();

    if paid < due {
        return Err(format!(
            "The transaction sends {paid} {} to {}; {due} is due",
            invoice.asset_code, invoice.destination
        ));
    }
    Ok(())
}

async fn fetch_json<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    url: &str,
) -> Result<T, BillingError> {
    let response = http
        .get(url)
        .send()
        .await
        .map_err(|e| BillingError::Upstream(e.to_string()))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(BillingError::Invalid(
            "Transaction not found on the ledger yet; wait a few seconds and retry".to_string(),
        ));
    }
    if !response.status().is_success() {
        return Err(BillingError::Upstream(format!(
            "HTTP {}",
            response.status()
        )));
    }
    response
        .json::<T>()
        .await
        .map_err(|e| BillingError::Upstream(e.to_string()))
}

#[derive(Deserialize)]
struct Records<T> {
    #[serde(rename = "_embedded")]
    embedded: Embedded<T>,
}

#[derive(Deserialize)]
struct Embedded<T> {
    records: Vec<T>,
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
    let hash = transaction_hash.trim().to_ascii_lowercase();
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(BillingError::Invalid(
            "transaction_hash must be a 64-character hex transaction hash".to_string(),
        ));
    }

    let invoice = get_invoice(pool, invoice_id, wallet_address).await?;
    match invoice.status.as_str() {
        "pending" => {}
        "paid" => {
            return Err(BillingError::Conflict(
                "Invoice is already paid".to_string(),
            ))
        }
        _ => {
            return Err(BillingError::Conflict(
                "Invoice is no longer payable".to_string(),
            ))
        }
    }
    let expires = parse_timestamp(&invoice.expires_at)?;

    let transaction: HorizonTransaction =
        fetch_json(http, &format!("{}/transactions/{hash}", config.horizon_url)).await?;
    let operations: Records<HorizonOperation> = fetch_json(
        http,
        &format!(
            "{}/transactions/{hash}/operations?limit=200",
            config.horizon_url
        ),
    )
    .await?;

    // Payments made before expiry still count if confirmed a little late.
    if parse_timestamp(&transaction.created_at)? > expires {
        return Err(BillingError::Conflict(
            "The payment was made after the invoice expired; request a new invoice".to_string(),
        ));
    }
    check_payment(&invoice, &transaction, &operations.embedded.records)
        .map_err(BillingError::Invalid)?;

    let now = Utc::now();
    let mut tx = pool.begin().await.context("begin")?;

    let settled = sqlx::query(
        "UPDATE billing_invoices SET status = 'paid', transaction_hash = ?, paid_at = ?
         WHERE id = ? AND status = 'pending'",
    )
    .bind(&transaction.hash)
    .bind(timestamp(now))
    .bind(&invoice.id)
    .execute(&mut *tx)
    .await;
    match settled {
        Ok(result) if result.rows_affected() == 1 => {}
        Ok(_) => {
            return Err(BillingError::Conflict(
                "Invoice is already paid".to_string(),
            ))
        }
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(BillingError::Conflict(
                "This transaction has already paid another invoice".to_string(),
            ))
        }
        Err(e) => return Err(BillingError::Internal(e.into())),
    }

    // Extend from the later of now and the current expiry, so renewing early
    // does not lose paid time.
    let current: Option<(String,)> =
        sqlx::query_as("SELECT paid_until FROM api_key_subscriptions WHERE api_key_id = ?")
            .bind(&invoice.api_key_id)
            .fetch_optional(&mut *tx)
            .await
            .context("look up subscription")?;
    let start = current
        .and_then(|(until,)| parse_timestamp(&until).ok())
        .filter(|until| *until > now)
        .unwrap_or(now);
    let subscription = Subscription {
        api_key_id: invoice.api_key_id.clone(),
        plan: invoice.plan.clone(),
        limit_per_minute: invoice.limit_per_minute,
        paid_until: timestamp(start + Duration::days(invoice.period_days)),
        updated_at: timestamp(now),
    };
    sqlx::query(
        "INSERT INTO api_key_subscriptions (api_key_id, plan, limit_per_minute, paid_until, updated_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(api_key_id) DO UPDATE SET plan = excluded.plan,
            limit_per_minute = excluded.limit_per_minute,
            paid_until = excluded.paid_until, updated_at = excluded.updated_at",
    )
    .bind(&subscription.api_key_id)
    .bind(&subscription.plan)
    .bind(subscription.limit_per_minute)
    .bind(&subscription.paid_until)
    .bind(&subscription.updated_at)
    .execute(&mut *tx)
    .await
    .context("upsert subscription")?;

    tx.commit().await.context("commit")?;

    let invoice = get_invoice(pool, invoice_id, wallet_address).await?;
    Ok((invoice, subscription))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TREASURY: &str = "GCJROW3RGUJNOUJZTFSKLWS4YVEROS5A7YBH5JABGW4HK4QN3A37HIKX";

    fn invoice() -> Invoice {
        Invoice {
            id: "inv".into(),
            api_key_id: "key".into(),
            wallet_address: "GWALLET".into(),
            plan: "pro".into(),
            amount_usdc: "50".into(),
            asset_code: "USDC".into(),
            asset_issuer: MAINNET_USDC_ISSUER.into(),
            destination: TREASURY.into(),
            memo: "payraider-abc123def456".into(),
            limit_per_minute: 1000,
            period_days: 30,
            status: "pending".into(),
            transaction_hash: None,
            created_at: "2026-10-01T00:00:00Z".into(),
            expires_at: "2026-10-02T00:00:00Z".into(),
            paid_at: None,
        }
    }

    fn transaction() -> HorizonTransaction {
        HorizonTransaction {
            hash: "a".repeat(64),
            successful: true,
            memo_type: "text".into(),
            memo: Some("payraider-abc123def456".into()),
            created_at: "2026-10-01T00:10:00Z".into(),
        }
    }

    fn usdc_payment(to: &str, amount: &str) -> HorizonOperation {
        HorizonOperation {
            kind: "payment".into(),
            to: Some(to.into()),
            asset_type: Some("credit_alphanum4".into()),
            asset_code: Some("USDC".into()),
            asset_issuer: Some(MAINNET_USDC_ISSUER.into()),
            amount: Some(amount.into()),
        }
    }

    #[test]
    fn accepts_an_exact_payment() {
        assert!(check_payment(
            &invoice(),
            &transaction(),
            &[usdc_payment(TREASURY, "50.0000000")]
        )
        .is_ok());
    }

    #[test]
    fn accepts_an_overpayment_split_across_operations() {
        let ops = [usdc_payment(TREASURY, "30"), usdc_payment(TREASURY, "25")];
        assert!(check_payment(&invoice(), &transaction(), &ops).is_ok());
    }

    #[test]
    fn rejects_an_underpayment() {
        let err = check_payment(
            &invoice(),
            &transaction(),
            &[usdc_payment(TREASURY, "49.9999999")],
        );
        assert!(err.unwrap_err().contains("is due"));
    }

    #[test]
    fn rejects_payment_to_another_account() {
        let other = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        assert!(check_payment(&invoice(), &transaction(), &[usdc_payment(other, "50")]).is_err());
    }

    #[test]
    fn rejects_a_counterfeit_usdc_issuer() {
        let mut op = usdc_payment(TREASURY, "50");
        op.asset_issuer = Some("GFAKEISSUERXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".into());
        assert!(check_payment(&invoice(), &transaction(), &[op]).is_err());
    }

    #[test]
    fn rejects_a_native_xlm_payment() {
        let mut op = usdc_payment(TREASURY, "50");
        op.asset_type = Some("native".into());
        op.asset_code = None;
        op.asset_issuer = None;
        assert!(check_payment(&invoice(), &transaction(), &[op]).is_err());
    }

    #[test]
    fn rejects_a_wrong_or_missing_memo() {
        let mut tx = transaction();
        tx.memo = Some("payraider-somethingelse".into());
        assert!(check_payment(&invoice(), &tx, &[usdc_payment(TREASURY, "50")]).is_err());

        let mut tx = transaction();
        tx.memo_type = "none".into();
        tx.memo = None;
        assert!(check_payment(&invoice(), &tx, &[usdc_payment(TREASURY, "50")]).is_err());
    }

    #[test]
    fn rejects_a_failed_transaction() {
        let mut tx = transaction();
        tx.successful = false;
        assert!(check_payment(&invoice(), &tx, &[usdc_payment(TREASURY, "50")]).is_err());
    }

    #[test]
    fn rejects_a_transaction_older_than_the_invoice() {
        let mut tx = transaction();
        tx.created_at = "2026-09-30T23:00:00Z".into();
        assert!(check_payment(&invoice(), &tx, &[usdc_payment(TREASURY, "50")]).is_err());
    }

    #[test]
    fn memos_fit_the_stellar_text_memo_limit() {
        for _ in 0..50 {
            assert!(new_memo().len() <= 28);
        }
    }
}
