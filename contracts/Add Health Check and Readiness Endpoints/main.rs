mod api;

use std::sync::Arc;

use axum::{routing::get, Router};
use tokio::net::TcpListener;

use crate::api::health::{health_check, readiness_check, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Initialise dependencies ───────────────────────────────────────────────
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL must be set");

    let db_pool = sqlx::PgPool::connect(&database_url).await?;
    let redis_client = redis::Client::open(redis_url)?;

    let state = Arc::new(AppState {
        db_pool,
        redis_client,
        rpc_url,
        version: env!("CARGO_PKG_VERSION").to_string(),
    });

    // ── Router ────────────────────────────────────────────────────────────────
    let app = Router::new()
        // Observability / probe routes
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        // … other application routes …
        .with_state(state);

    // ── Listen ────────────────────────────────────────────────────────────────
    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
