use anyhow::Result;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use stellar_insights_backend::database::Database;
use stellar_insights_backend::services::aggregation::{AggregationConfig, AggregationService};
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting corridor aggregation service demo");

    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./stellar_insights.db".to_string());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    info!("Connected to database: {}", database_url);

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    info!("Migrations completed");

    // Create database instance
    let db = Arc::new(Database::new(pool));

    // Configure aggregation service
    let config = AggregationConfig {
        interval_hours: 1, // Run every hour
        lookback_hours: 2, // Process last 2 hours
        batch_size: 10000, // Process 10k payments at a time
    };

    // Create aggregation service
    let aggregation_service = Arc::new(AggregationService::new(Arc::clone(&db), config));

    info!("Aggregation service configured");

    // Option 1: Run aggregation once
    info!("Running one-time aggregation...");
    aggregation_service.run_hourly_aggregation().await?;
    info!("One-time aggregation completed");

    // Option 2: Start scheduler (runs continuously)
    // Uncomment to run as a background service
    /*
    info!("Starting aggregation scheduler...");
    aggregation_service.start_scheduler().await;
    */

    // Option 3: Calculate volume trends
    info!("Calculating volume trends for last 24 hours...");
    let trends = aggregation_service.calculate_volume_trends(24).await?;

    info!("Found {} corridor trends:", trends.len());
    for trend in trends.iter().take(10) {
        info!(
            "  {} - Total: ${:.2}, Avg: ${:.2}, Trend: {:.2}%",
            trend.corridor_key, trend.total_volume, trend.avg_volume, trend.trend_percentage
        );
    }

    Ok(())
}
