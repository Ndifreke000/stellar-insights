//! A presented API key must resolve to its stored key so the caller gets the
//! key's own rate-limit bucket. Keys are stored as salted Argon2 hashes, so
//! this goes through prefix lookup plus verification.

use payraider_backend::database::Database;
use payraider_backend::models::api_key::CreateApiKeyRequest;
use payraider_backend::rate_limit::RateLimiter;
use sqlx::SqlitePool;

const WALLET: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

async fn setup() -> (Database, SqlitePool) {
    let pool = SqlitePool::connect("sqlite::memory:").await.expect("pool");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    (Database::new(pool.clone()), pool)
}

fn request(name: &str) -> CreateApiKeyRequest {
    CreateApiKeyRequest {
        name: name.to_string(),
        scopes: None,
        expires_at: None,
    }
}

#[tokio::test]
async fn a_created_key_resolves_to_its_id() {
    let (db, pool) = setup().await;
    let created = db.create_api_key(WALLET, request("offramp")).await.unwrap();
    let limiter = RateLimiter::new_memory_only(Some(pool));

    let resolved = limiter.resolve_api_key_id(&created.plain_key).await;

    assert_eq!(resolved.as_deref(), Some(created.key.id.as_str()));
}

#[tokio::test]
async fn resolution_is_stable_across_requests() {
    let (db, pool) = setup().await;
    let created = db.create_api_key(WALLET, request("offramp")).await.unwrap();
    let limiter = RateLimiter::new_memory_only(Some(pool));

    let first = limiter.resolve_api_key_id(&created.plain_key).await;
    let second = limiter.resolve_api_key_id(&created.plain_key).await;

    assert!(first.is_some());
    assert_eq!(first, second);
}

#[tokio::test]
async fn an_unknown_or_altered_key_does_not_resolve() {
    let (db, pool) = setup().await;
    let created = db.create_api_key(WALLET, request("offramp")).await.unwrap();
    let limiter = RateLimiter::new_memory_only(Some(pool));

    // Same prefix, different secret.
    let mut altered = created.plain_key.clone();
    let last = altered.pop().unwrap();
    altered.push(if last == '0' { '1' } else { '0' });

    assert_eq!(limiter.resolve_api_key_id(&altered).await, None);
    assert_eq!(
        limiter.resolve_api_key_id("si_live_doesnotexist").await,
        None
    );
    assert_eq!(limiter.resolve_api_key_id("not-an-api-key").await, None);
}

#[tokio::test]
async fn a_revoked_key_does_not_resolve() {
    let (db, pool) = setup().await;
    let created = db.create_api_key(WALLET, request("offramp")).await.unwrap();
    db.revoke_api_key(&created.key.id, WALLET).await.unwrap();
    let limiter = RateLimiter::new_memory_only(Some(pool));

    assert_eq!(limiter.resolve_api_key_id(&created.plain_key).await, None);
}
