use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SubscriptionService {
    pool: SqlitePool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TelegramSubscription {
    pub id: String,
    pub chat_id: i64,
    pub chat_type: String,
    pub chat_title: Option<String>,
    pub username: Option<String>,
    pub subscribed_at: String,
    pub is_active: i64,
    pub alert_types: String,
    pub last_alert_sent_at: Option<String>,
}

impl SubscriptionService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn subscribe(
        &self,
        chat_id: i64,
        chat_type: &str,
        chat_title: Option<&str>,
        username: Option<&str>,
    ) -> anyhow::Result<bool> {
        let existing: Option<TelegramSubscription> =
            sqlx::query_as("SELECT * FROM telegram_subscriptions WHERE chat_id = ?")
                .bind(chat_id)
                .fetch_optional(&self.pool)
                .await?;

        if let Some(sub) = existing {
            if sub.is_active == 1 {
                return Ok(false); // already subscribed
            }
            // Re-activate
            sqlx::query("UPDATE telegram_subscriptions SET is_active = 1, subscribed_at = datetime('now') WHERE chat_id = ?")
                .bind(chat_id)
                .execute(&self.pool)
                .await?;
            return Ok(true);
        }

        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO telegram_subscriptions (id, chat_id, chat_type, chat_title, username) VALUES (?, ?, ?, ?, ?)"
        )
            .bind(&id)
            .bind(chat_id)
            .bind(chat_type)
            .bind(chat_title)
            .bind(username)
            .execute(&self.pool)
            .await?;

        Ok(true)
    }

    pub async fn unsubscribe(&self, chat_id: i64) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "UPDATE telegram_subscriptions SET is_active = 0 WHERE chat_id = ? AND is_active = 1",
        )
        .bind(chat_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_active_chat_ids(&self) -> anyhow::Result<Vec<i64>> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT chat_id FROM telegram_subscriptions WHERE is_active = 1")
                .fetch_all(&self.pool)
                .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn is_subscribed(&self, chat_id: i64) -> anyhow::Result<bool> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM telegram_subscriptions WHERE chat_id = ? AND is_active = 1",
        )
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(c,)| c > 0).unwrap_or(false))
    }

    pub async fn update_last_alert_sent(&self, chat_id: i64) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE telegram_subscriptions SET last_alert_sent_at = datetime('now') WHERE chat_id = ?",
        )
        .bind(chat_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
