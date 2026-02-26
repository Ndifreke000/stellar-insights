use sqlx::SqlitePool;

pub async fn init(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Create sponsorships table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sponsorships (
            id TEXT PRIMARY KEY,
            sponsor TEXT NOT NULL,
            sponsored_account TEXT NOT NULL,
            sponsored_reserves INTEGER NOT NULL,
            total_amount TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(sponsor, sponsored_account)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create sponsorship_history table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sponsorship_history (
            id TEXT PRIMARY KEY,
            sponsorship_id TEXT NOT NULL,
            change_type TEXT NOT NULL,
            previous_amount TEXT,
            new_amount TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(sponsorship_id) REFERENCES sponsorships(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create alerts table for sponsorship changes
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sponsorship_alerts (
            id TEXT PRIMARY KEY,
            sponsorship_id TEXT NOT NULL,
            change_type TEXT NOT NULL,
            previous_value TEXT,
            new_value TEXT NOT NULL,
            created_at TEXT NOT NULL,
            acknowledged BOOLEAN DEFAULT FALSE,
            FOREIGN KEY(sponsorship_id) REFERENCES sponsorships(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indices for performance
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sponsor ON sponsorships(sponsor)")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_sponsored_account ON sponsorships(sponsored_account)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_sponsorship_history_sponsorship_id ON sponsorship_history(sponsorship_id)",
    )
    .execute(pool)
    .await?;

    Ok(())
}
