use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

#[tokio::main]
async fn main() {
    println!("Creating database...");
    let options = SqliteConnectOptions::from_str("sqlite://stellar_insights.db")
        .unwrap()
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await.unwrap();
    println!("Applying migrations...");
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    println!("Database ready!");
}
