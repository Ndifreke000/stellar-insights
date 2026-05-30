mod api;
mod db;
mod models;
mod services;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<SqlitePool>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Setup database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite::memory:".to_string());
    
    let pool = SqlitePool::connect(&database_url).await?;
    
    // Run migrations
    db::init(&pool).await?;

    let state = AppState {
        db: Arc::new(pool),
    };

    // Build our application with routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/sponsorships", get(api::sponsorships::get_all_sponsorships))
        .route("/api/sponsorships", post(api::sponsorships::create_sponsorship))
        .route(
            "/api/sponsorships/:id",
            get(api::sponsorships::get_sponsorship),
        )
        .route(
            "/api/sponsorships/:id/history",
            get(api::sponsorships::get_sponsorship_history),
        )
        .route(
            "/api/sponsors/leaderboard",
            get(api::sponsorships::get_sponsor_leaderboard),
        )
        .route(
            "/api/sponsorships/by-account/:account",
            get(api::sponsorships::get_sponsorships_by_account),
        )
        .route(
            "/api/analytics/summary",
            get(api::sponsorships::get_analytics_summary),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("listening on {:?}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
