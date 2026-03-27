//! Snapshot Hash Generation Service Demo
//!
//! This example demonstrates the complete snapshot hash generation workflow
//! that fulfills all acceptance criteria for issue #122:
//!
//! 1. Aggregate all metrics
//! 2. Serialize to deterministic JSON
//! 3. Compute SHA-256 hash
//! 4. Store hash in database
//! 5. Submit to smart contract
//! 6. Verify submission success

use std::sync::Arc;
use stellar_insights_backend::database::Database;
use stellar_insights_backend::services::contract::{ContractConfig, ContractService};
use stellar_insights_backend::services::snapshot::SnapshotService;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🚀 Starting Snapshot Hash Generation Demo");

    // Initialize database connection
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:stellar_insights.db".to_string());

    info!("Connecting to database: {}", database_url);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let db = Arc::new(Database::new(pool));

    // Initialize contract service (optional)
    let contract_service = if std::env::var("SNAPSHOT_CONTRACT_ID").is_ok() {
        info!("Contract service configured - will submit to blockchain");
        Some(Arc::new(ContractService::from_env()?))
    } else {
        info!("Contract service not configured - will only generate hash");
        None
    };

    // Initialize snapshot service
    let snapshot_service = SnapshotService::new(db.clone(), contract_service.clone(), None);

    // Generate snapshot for current epoch
    let epoch = chrono::Utc::now().timestamp() as u64 / 3600; // Hourly epochs

    info!("📊 Generating snapshot for epoch {}", epoch);

    match snapshot_service.generate_and_submit_snapshot(epoch).await {
        Ok(result) => {
            info!("✅ Snapshot generation completed successfully!");
            info!("📋 Results:");
            info!("   • Snapshot ID: {}", result.snapshot_id);
            info!("   • Epoch: {}", result.epoch);
            info!("   • Hash: {}", result.hash);
            info!("   • Timestamp: {}", result.timestamp);
            info!("   • Anchor metrics: {}", result.anchor_count);
            info!("   • Corridor metrics: {}", result.corridor_count);

            if let Some(submission) = result.submission_result {
                info!("🔗 Blockchain submission:");
                info!("   • Transaction hash: {}", submission.transaction_hash);
                info!("   • Ledger: {}", submission.ledger);
                info!("   • Contract timestamp: {}", submission.timestamp);
                info!(
                    "   • Verification: {}",
                    if result.verification_successful {
                        "✅ SUCCESS"
                    } else {
                        "❌ FAILED"
                    }
                );
            } else {
                info!("🔗 Blockchain submission: SKIPPED (no contract service)");
            }

            // Demonstrate hash determinism
            info!("🔍 Testing hash determinism...");
            let snapshot = snapshot_service.aggregate_all_metrics(epoch).await?;
            let json1 = SnapshotService::serialize_deterministically(snapshot.clone())?;
            let json2 = SnapshotService::serialize_deterministically(snapshot)?;

            if json1 == json2 {
                info!("✅ Deterministic serialization verified");
            } else {
                info!("❌ Deterministic serialization failed");
            }

            // Show JSON structure (truncated)
            let json_preview = if result.canonical_json.len() > 200 {
                format!("{}...", &result.canonical_json[..200])
            } else {
                result.canonical_json.clone()
            };
            info!("📄 Canonical JSON preview: {}", json_preview);
        }
        Err(e) => {
            eprintln!("❌ Snapshot generation failed: {}", e);
            std::process::exit(1);
        }
    }

    info!("🎉 Demo completed successfully!");
    Ok(())
}
