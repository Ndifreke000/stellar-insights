use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Creating database...");
    let options = SqliteConnectOptions::from_str("sqlite://payraider.db")
        .map_err(|e| anyhow::anyhow!("Invalid database URL: {}", e))?
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(options)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;
    println!("Applying migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
    println!("Database ready!");
    Ok(())
}
