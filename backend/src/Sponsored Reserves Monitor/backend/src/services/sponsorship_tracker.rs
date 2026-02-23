use crate::models::{
    Sponsorship, SponsorshipAnalytics, SponsorshipChangeAlert, SponsorshipHistory,
    SponsorLeaderboard,
};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Clone)]
pub struct SponsorshipTrackerService {
    db: SqlitePool,
}

impl SponsorshipTrackerService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Track a new sponsorship relationship
    pub async fn track_sponsorship(
        &self,
        sponsor: String,
        sponsored_account: String,
        sponsored_reserves: i64,
        total_amount: String,
    ) -> Result<Sponsorship, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO sponsorships (id, sponsor, sponsored_account, sponsored_reserves, total_amount, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&sponsor)
        .bind(&sponsored_account)
        .bind(sponsored_reserves)
        .bind(&total_amount)
        .bind(&now)
        .bind(&now)
        .execute(&self.db)
        .await?;

        // Record initial history entry
        self.record_history(
            &id,
            "CREATED",
            None,
            &total_amount,
        )
        .await?;

        Ok(Sponsorship {
            id,
            sponsor,
            sponsored_account,
            sponsored_reserves,
            total_amount,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Update an existing sponsorship
    pub async fn update_sponsorship(
        &self,
        sponsorship_id: String,
        sponsored_reserves: i64,
        new_amount: String,
    ) -> Result<Sponsorship, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        // Get previous amount for history
        let previous: (String,) = sqlx::query_as(
            "SELECT total_amount FROM sponsorships WHERE id = ?"
        )
        .bind(&sponsorship_id)
        .fetch_one(&self.db)
        .await?;

        sqlx::query(
            "UPDATE sponsorships SET sponsored_reserves = ?, total_amount = ?, updated_at = ? WHERE id = ?",
        )
        .bind(sponsored_reserves)
        .bind(&new_amount)
        .bind(&now)
        .bind(&sponsorship_id)
        .execute(&self.db)
        .await?;

        // Record history entry
        self.record_history(
            &sponsorship_id,
            "UPDATED",
            Some(&previous.0),
            &new_amount,
        )
        .await?;

        // Fetch and return updated sponsorship
        let sponsorship: Sponsorship = sqlx::query_as(
            "SELECT * FROM sponsorships WHERE id = ?"
        )
        .bind(&sponsorship_id)
        .fetch_one(&self.db)
        .await?;

        Ok(sponsorship)
    }

    /// Get all sponsorships with pagination
    pub async fn get_all_sponsorships(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Sponsorship>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM sponsorships ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }

    /// Get sponsorships by a specific sponsor
    pub async fn get_sponsorships_by_sponsor(
        &self,
        sponsor: String,
    ) -> Result<Vec<Sponsorship>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM sponsorships WHERE sponsor = ? ORDER BY created_at DESC"
        )
        .bind(sponsor)
        .fetch_all(&self.db)
        .await
    }

    /// Get sponsorships for a specific account
    pub async fn get_sponsorships_for_account(
        &self,
        account: String,
    ) -> Result<Vec<Sponsorship>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM sponsorships WHERE sponsored_account = ? ORDER BY created_at DESC"
        )
        .bind(account)
        .fetch_all(&self.db)
        .await
    }

    /// Get sponsor leaderboard (top sponsors by total amount)
    pub async fn get_sponsor_leaderboard(
        &self,
        limit: i64,
    ) -> Result<Vec<SponsorLeaderboard>, sqlx::Error> {
        sqlx::query_as::<_, SponsorLeaderboard>(
            r#"
            SELECT 
                sponsor,
                total_amount as total_sponsored_amount,
                COUNT(DISTINCT sponsored_account) as sponsored_accounts_count,
                ROW_NUMBER() OVER (ORDER BY CAST(total_amount AS REAL) DESC) as rank
            FROM (
                SELECT sponsor, sponsored_account, CAST(
                    REPLACE(REPLACE(REPLACE(total_amount, ',', ''), ' XLM', ''), '$', '') 
                    AS REAL) + 0 AS total_amount
                FROM sponsorships
            )
            GROUP BY sponsor
            ORDER BY total_amount DESC
            LIMIT ?
            "#
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await
    }

    /// Get sponsorship history for tracking changes
    pub async fn get_sponsorship_history(
        &self,
        sponsorship_id: String,
    ) -> Result<Vec<SponsorshipHistory>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM sponsorship_history WHERE sponsorship_id = ? ORDER BY created_at DESC"
        )
        .bind(sponsorship_id)
        .fetch_all(&self.db)
        .await
    }

    /// Calculate analytics summary
    pub async fn get_analytics(&self) -> Result<SponsorshipAnalytics, sqlx::Error> {
        let total_sponsorships: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sponsorships"
        )
        .fetch_one(&self.db)
        .await?;

        let unique_sponsors: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT sponsor) FROM sponsorships"
        )
        .fetch_one(&self.db)
        .await?;

        let unique_accounts: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT sponsored_account) FROM sponsorships"
        )
        .fetch_one(&self.db)
        .await?;

        // Note: For a production system, you'd properly parse the amount strings
        // This is a simplified version
        let amounts: Vec<(String,)> = sqlx::query_as(
            "SELECT total_amount FROM sponsorships"
        )
        .fetch_all(&self.db)
        .await?;

        let total_amount = calculate_total_amount(&amounts);
        let (avg, max, min) = calculate_amount_stats(&amounts);

        Ok(SponsorshipAnalytics {
            total_sponsorships: total_sponsorships.0,
            total_amount_sponsored: total_amount,
            unique_sponsors: unique_sponsors.0,
            unique_sponsored_accounts: unique_accounts.0,
            average_sponsorship: avg,
            largest_sponsorship: max,
            smallest_sponsorship: min,
        })
    }

    /// Record a sponsorship history entry
    async fn record_history(
        &self,
        sponsorship_id: &str,
        change_type: &str,
        previous_amount: Option<&str>,
        new_amount: &str,
    ) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO sponsorship_history (id, sponsorship_id, change_type, previous_amount, new_amount, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(sponsorship_id)
        .bind(change_type)
        .bind(previous_amount)
        .bind(new_amount)
        .bind(&now)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Create a sponsorship change alert
    pub async fn create_alert(
        &self,
        sponsorship_id: String,
        sponsor: String,
        sponsored_account: String,
        change_type: String,
        previous_value: Option<String>,
        new_value: String,
    ) -> Result<SponsorshipChangeAlert, sqlx::Error> {
        let alert_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO sponsorship_alerts (id, sponsorship_id, change_type, previous_value, new_value, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&alert_id)
        .bind(&sponsorship_id)
        .bind(&change_type)
        .bind(&previous_value)
        .bind(&new_value)
        .bind(&now)
        .execute(&self.db)
        .await?;

        Ok(SponsorshipChangeAlert {
            sponsorship_id,
            sponsor,
            sponsored_account,
            change_type,
            previous_value,
            new_value,
            timestamp: now,
        })
    }
}

fn calculate_total_amount(amounts: &[(String,)]) -> String {
    amounts.len().to_string()
}

fn calculate_amount_stats(amounts: &[(String,)]) -> (String, String, String) {
    if amounts.is_empty() {
        return ("0".to_string(), "0".to_string(), "0".to_string());
    }

    let avg = (amounts.len() as f64 / 1.0).to_string();
    let max = amounts.iter().next().map(|a| a.0.clone()).unwrap_or_default();
    let min = amounts.iter().last().map(|a| a.0.clone()).unwrap_or_default();

    (avg, max, min)
}
