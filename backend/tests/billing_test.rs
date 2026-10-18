//! End-to-end billing: create a key and an invoice, "pay" it on a fake
//! Horizon, confirm, and check the key's limit goes up. Also covers replaying
//! one payment for a second invoice.

use payraider_backend::billing::{
    active_limit, confirm_invoice, create_invoice, BillingConfig, BillingError, MAINNET_USDC_ISSUER,
};
use payraider_backend::database::Database;
use payraider_backend::models::api_key::CreateApiKeyRequest;
use payraider_backend::rate_limit::RateLimiter;
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::SqlitePool;
use std::str::FromStr;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const WALLET: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
const TREASURY: &str = "GCJROW3RGUJNOUJZTFSKLWS4YVEROS5A7YBH5JABGW4HK4QN3A37HIKX";

async fn setup() -> (SqlitePool, Database) {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    (pool.clone(), Database::new(pool))
}

fn config(horizon_url: String) -> BillingConfig {
    BillingConfig {
        treasury: TREASURY.to_string(),
        asset_code: "USDC".to_string(),
        asset_issuer: MAINNET_USDC_ISSUER.to_string(),
        price_usdc: Decimal::from_str("50").unwrap(),
        period_days: 30,
        limit_per_minute: 1000,
        horizon_url,
    }
}

/// Serve one transaction paying `amount` USDC to the treasury with `memo`.
async fn horizon_with_payment(hash: &str, memo: &str, amount: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/transactions/{hash}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "hash": hash,
            "successful": true,
            "memo_type": "text",
            "memo": memo,
            "created_at": chrono::Utc::now().to_rfc3339(),
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/transactions/{hash}/operations")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": { "records": [{
                "type": "payment",
                "to": TREASURY,
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": MAINNET_USDC_ISSUER,
                "amount": amount,
            }]}
        })))
        .mount(&server)
        .await;
    server
}

async fn new_key(db: &Database) -> (String, String) {
    let created = db
        .create_api_key(
            WALLET,
            CreateApiKeyRequest {
                name: "offramp".to_string(),
                scopes: None,
                expires_at: None,
            },
        )
        .await
        .unwrap();
    (created.key.id, created.plain_key)
}

#[tokio::test]
async fn paying_an_invoice_raises_the_keys_limit() {
    let (pool, db) = setup().await;
    let (key_id, _) = new_key(&db).await;
    let limiter = RateLimiter::new_memory_only(Some(pool.clone()));
    assert_eq!(limiter.get_api_key_limit_per_minute(&key_id).await, 200);

    let hash = "a".repeat(64);
    let placeholder = config(String::new());
    let invoice = create_invoice(&pool, &placeholder, &key_id, WALLET)
        .await
        .unwrap();
    assert_eq!(invoice.destination, TREASURY);
    assert_eq!(invoice.amount_usdc, "50");

    let horizon = horizon_with_payment(&hash, &invoice.memo, "50.0000000").await;
    let cfg = config(horizon.uri());
    let (paid, subscription) = confirm_invoice(
        &pool,
        &cfg,
        &reqwest::Client::new(),
        &invoice.id,
        WALLET,
        &hash,
    )
    .await
    .unwrap();

    assert_eq!(paid.status, "paid");
    assert_eq!(paid.transaction_hash.as_deref(), Some(hash.as_str()));
    assert_eq!(subscription.limit_per_minute, 1000);
    assert_eq!(active_limit(&pool, &key_id).await.unwrap(), Some(1000));
    assert_eq!(limiter.get_api_key_limit_per_minute(&key_id).await, 1000);
}

#[tokio::test]
async fn one_payment_cannot_settle_two_invoices() {
    let (pool, db) = setup().await;
    let (key_id, _) = new_key(&db).await;
    let hash = "b".repeat(64);
    let placeholder = config(String::new());

    let first = create_invoice(&pool, &placeholder, &key_id, WALLET)
        .await
        .unwrap();
    let horizon = horizon_with_payment(&hash, &first.memo, "50").await;
    let cfg = config(horizon.uri());
    confirm_invoice(
        &pool,
        &cfg,
        &reqwest::Client::new(),
        &first.id,
        WALLET,
        &hash,
    )
    .await
    .unwrap();

    // The second invoice has a different memo, so the old payment does not match it.
    let second = create_invoice(&pool, &placeholder, &key_id, WALLET)
        .await
        .unwrap();
    let err = confirm_invoice(
        &pool,
        &cfg,
        &reqwest::Client::new(),
        &second.id,
        WALLET,
        &hash,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, BillingError::Invalid(_)), "{err:?}");

    // Confirming the paid invoice again is a conflict, not a second extension.
    let err = confirm_invoice(
        &pool,
        &cfg,
        &reqwest::Client::new(),
        &first.id,
        WALLET,
        &hash,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, BillingError::Conflict(_)), "{err:?}");
}

#[tokio::test]
async fn an_underpayment_leaves_the_invoice_pending() {
    let (pool, db) = setup().await;
    let (key_id, _) = new_key(&db).await;
    let hash = "c".repeat(64);
    let placeholder = config(String::new());
    let invoice = create_invoice(&pool, &placeholder, &key_id, WALLET)
        .await
        .unwrap();

    let horizon = horizon_with_payment(&hash, &invoice.memo, "10").await;
    let cfg = config(horizon.uri());
    let err = confirm_invoice(
        &pool,
        &cfg,
        &reqwest::Client::new(),
        &invoice.id,
        WALLET,
        &hash,
    )
    .await
    .unwrap_err();

    assert!(matches!(err, BillingError::Invalid(_)), "{err:?}");
    assert_eq!(active_limit(&pool, &key_id).await.unwrap(), None);
}

#[tokio::test]
async fn another_wallet_cannot_invoice_or_confirm_for_a_key() {
    let (pool, db) = setup().await;
    let (key_id, _) = new_key(&db).await;
    let placeholder = config(String::new());
    let other = "GBOTHERXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";

    let err = create_invoice(&pool, &placeholder, &key_id, other)
        .await
        .unwrap_err();
    assert!(matches!(err, BillingError::NotFound(_)));

    let invoice = create_invoice(&pool, &placeholder, &key_id, WALLET)
        .await
        .unwrap();
    let err = confirm_invoice(
        &pool,
        &placeholder,
        &reqwest::Client::new(),
        &invoice.id,
        other,
        &"d".repeat(64),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, BillingError::NotFound(_)));
}
