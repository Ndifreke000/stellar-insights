use crate::models::corridor_alerts::{
    CorridorAlertConfig, CorridorAlertEvent, CorridorPerformanceSnapshot,
};
use anyhow::Result;
use uuid::Uuid;

impl crate::database::Database {
    // ---- Performance Snapshots ----

    pub async fn insert_performance_snapshot(
        &self,
        corridor_key: &str,
        source_asset_code: &str,
        source_asset_issuer: &str,
        destination_asset_code: &str,
        destination_asset_issuer: &str,
        success_rate: f64,
        avg_settlement_latency_ms: f64,
        liquidity_depth_usd: f64,
        volume_usd: f64,
        total_transactions: i64,
        successful_transactions: i64,
        failed_transactions: i64,
    ) -> Result<CorridorPerformanceSnapshot> {
        let id = Uuid::new_v4().to_string();
        let snapshot = sqlx::query_as::<_, CorridorPerformanceSnapshot>(
            r"
            INSERT INTO corridor_performance_snapshots (
                id, corridor_key, source_asset_code, source_asset_issuer,
                destination_asset_code, destination_asset_issuer,
                success_rate, avg_settlement_latency_ms, liquidity_depth_usd,
                volume_usd, total_transactions, successful_transactions, failed_transactions
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(corridor_key)
        .bind(source_asset_code)
        .bind(source_asset_issuer)
        .bind(destination_asset_code)
        .bind(destination_asset_issuer)
        .bind(success_rate)
        .bind(avg_settlement_latency_ms)
        .bind(liquidity_depth_usd)
        .bind(volume_usd)
        .bind(total_transactions)
        .bind(successful_transactions)
        .bind(failed_transactions)
        .fetch_one(self.pool())
        .await?;

        Ok(snapshot)
    }

    pub async fn get_latest_snapshot_for_corridor(
        &self,
        corridor_key: &str,
    ) -> Result<Option<CorridorPerformanceSnapshot>> {
        let snapshot = sqlx::query_as::<_, CorridorPerformanceSnapshot>(
            r"
            SELECT * FROM corridor_performance_snapshots
            WHERE corridor_key = $1
            ORDER BY snapshot_time DESC
            LIMIT 1
            ",
        )
        .bind(corridor_key)
        .fetch_optional(self.pool())
        .await?;

        Ok(snapshot)
    }

    pub async fn get_previous_snapshot_for_corridor(
        &self,
        corridor_key: &str,
    ) -> Result<Option<CorridorPerformanceSnapshot>> {
        let snapshot = sqlx::query_as::<_, CorridorPerformanceSnapshot>(
            r"
            SELECT * FROM corridor_performance_snapshots
            WHERE corridor_key = $1
            ORDER BY snapshot_time DESC
            LIMIT 1 OFFSET 1
            ",
        )
        .bind(corridor_key)
        .fetch_optional(self.pool())
        .await?;

        Ok(snapshot)
    }

    pub async fn get_snapshots_for_corridor(
        &self,
        corridor_key: &str,
        limit: i64,
    ) -> Result<Vec<CorridorPerformanceSnapshot>> {
        let snapshots = sqlx::query_as::<_, CorridorPerformanceSnapshot>(
            r"
            SELECT * FROM corridor_performance_snapshots
            WHERE corridor_key = $1
            ORDER BY snapshot_time DESC
            LIMIT $2
            ",
        )
        .bind(corridor_key)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        Ok(snapshots)
    }

    pub async fn get_latest_snapshots_all_corridors(&self) -> Result<Vec<CorridorPerformanceSnapshot>> {
        let snapshots = sqlx::query_as::<_, CorridorPerformanceSnapshot>(
            r"
            SELECT * FROM corridor_performance_snapshots
            WHERE id IN (
                SELECT id FROM corridor_performance_snapshots
                GROUP BY corridor_key
                HAVING snapshot_time = MAX(snapshot_time)
            )
            ORDER BY corridor_key
            ",
        )
        .fetch_all(self.pool())
        .await?;

        Ok(snapshots)
    }

    // ---- Alert Configs ----

    pub async fn create_corridor_alert_config(
        &self,
        user_id: &str,
        req: crate::models::corridor_alerts::CreateCorridorAlertConfigRequest,
    ) -> Result<CorridorAlertConfig> {
        let id = Uuid::new_v4().to_string();
        let config = sqlx::query_as::<_, CorridorAlertConfig>(
            r"
            INSERT INTO corridor_alert_configs (
                id, user_id, corridor_key, name,
                success_rate_threshold, latency_threshold_ms, liquidity_threshold_usd,
                success_rate_drop_pct, latency_increase_pct, liquidity_drop_pct,
                cooldown_seconds, notify_email, notify_webhook, notify_in_app,
                notify_slack, notify_telegram
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(user_id)
        .bind(&req.corridor_key)
        .bind(&req.name)
        .bind(req.success_rate_threshold)
        .bind(req.latency_threshold_ms)
        .bind(req.liquidity_threshold_usd)
        .bind(req.success_rate_drop_pct.unwrap_or(10.0))
        .bind(req.latency_increase_pct.unwrap_or(50.0))
        .bind(req.liquidity_drop_pct.unwrap_or(30.0))
        .bind(req.cooldown_seconds.unwrap_or(300))
        .bind(req.notify_email.unwrap_or(false))
        .bind(req.notify_webhook.unwrap_or(false))
        .bind(req.notify_in_app.unwrap_or(true))
        .bind(req.notify_slack.unwrap_or(false))
        .bind(req.notify_telegram.unwrap_or(false))
        .fetch_one(self.pool())
        .await?;

        Ok(config)
    }

    pub async fn get_corridor_alert_configs_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<CorridorAlertConfig>> {
        let configs = sqlx::query_as::<_, CorridorAlertConfig>(
            r"
            SELECT * FROM corridor_alert_configs
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id)
        .fetch_all(self.pool())
        .await?;

        Ok(configs)
    }

    pub async fn get_all_active_corridor_alert_configs(&self) -> Result<Vec<CorridorAlertConfig>> {
        let configs = sqlx::query_as::<_, CorridorAlertConfig>(
            r"
            SELECT * FROM corridor_alert_configs
            WHERE is_active = 1
            ",
        )
        .fetch_all(self.pool())
        .await?;

        Ok(configs)
    }

    pub async fn get_corridor_alert_config_by_id(
        &self,
        id: &str,
    ) -> Result<Option<CorridorAlertConfig>> {
        let config = sqlx::query_as::<_, CorridorAlertConfig>(
            r"
            SELECT * FROM corridor_alert_configs
            WHERE id = $1
            ",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(config)
    }

    pub async fn update_corridor_alert_config(
        &self,
        id: &str,
        user_id: &str,
        req: crate::models::corridor_alerts::UpdateCorridorAlertConfigRequest,
    ) -> Result<CorridorAlertConfig> {
        let mut sets = Vec::new();
        sets.push("updated_at = CURRENT_TIMESTAMP".to_string());

        if req.name.is_some() {
            sets.push(format!("name = ${}", sets.len() + 2));
        }
        if req.success_rate_threshold.is_some() {
            sets.push(format!("success_rate_threshold = ${}", sets.len() + 2));
        }
        if req.latency_threshold_ms.is_some() {
            sets.push(format!("latency_threshold_ms = ${}", sets.len() + 2));
        }
        if req.liquidity_threshold_usd.is_some() {
            sets.push(format!("liquidity_threshold_usd = ${}", sets.len() + 2));
        }
        if req.success_rate_drop_pct.is_some() {
            sets.push(format!("success_rate_drop_pct = ${}", sets.len() + 2));
        }
        if req.latency_increase_pct.is_some() {
            sets.push(format!("latency_increase_pct = ${}", sets.len() + 2));
        }
        if req.liquidity_drop_pct.is_some() {
            sets.push(format!("liquidity_drop_pct = ${}", sets.len() + 2));
        }
        if req.cooldown_seconds.is_some() {
            sets.push(format!("cooldown_seconds = ${}", sets.len() + 2));
        }
        if req.notify_email.is_some() {
            sets.push(format!("notify_email = ${}", sets.len() + 2));
        }
        if req.notify_webhook.is_some() {
            sets.push(format!("notify_webhook = ${}", sets.len() + 2));
        }
        if req.notify_in_app.is_some() {
            sets.push(format!("notify_in_app = ${}", sets.len() + 2));
        }
        if req.notify_slack.is_some() {
            sets.push(format!("notify_slack = ${}", sets.len() + 2));
        }
        if req.notify_telegram.is_some() {
            sets.push(format!("notify_telegram = ${}", sets.len() + 2));
        }
        if req.is_active.is_some() {
            sets.push(format!("is_active = ${}", sets.len() + 2));
        }

        let set_clause = sets.join(", ");
        let query = format!(
            "UPDATE corridor_alert_configs SET {} WHERE id = $1 AND user_id = $2 RETURNING *",
            set_clause
        );

        let mut q = sqlx::query_as::<_, CorridorAlertConfig>(&query)
            .bind(id)
            .bind(user_id);

        if let Some(v) = &req.name {
            q = q.bind(v);
        }
        if let Some(v) = req.success_rate_threshold {
            q = q.bind(v);
        }
        if let Some(v) = req.latency_threshold_ms {
            q = q.bind(v);
        }
        if let Some(v) = req.liquidity_threshold_usd {
            q = q.bind(v);
        }
        if let Some(v) = req.success_rate_drop_pct {
            q = q.bind(v);
        }
        if let Some(v) = req.latency_increase_pct {
            q = q.bind(v);
        }
        if let Some(v) = req.liquidity_drop_pct {
            q = q.bind(v);
        }
        if let Some(v) = req.cooldown_seconds {
            q = q.bind(v);
        }
        if let Some(v) = req.notify_email {
            q = q.bind(v);
        }
        if let Some(v) = req.notify_webhook {
            q = q.bind(v);
        }
        if let Some(v) = req.notify_in_app {
            q = q.bind(v);
        }
        if let Some(v) = req.notify_slack {
            q = q.bind(v);
        }
        if let Some(v) = req.notify_telegram {
            q = q.bind(v);
        }
        if let Some(v) = req.is_active {
            q = q.bind(v);
        }

        let config = q.fetch_one(self.pool()).await?;
        Ok(config)
    }

    pub async fn delete_corridor_alert_config(&self, id: &str, user_id: &str) -> Result<()> {
        sqlx::query(
            r"
            DELETE FROM corridor_alert_configs WHERE id = $1 AND user_id = $2
            ",
        )
        .bind(id)
        .bind(user_id)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn update_last_triggered(&self, config_id: &str) -> Result<()> {
        sqlx::query(
            r"
            UPDATE corridor_alert_configs
            SET last_triggered_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            ",
        )
        .bind(config_id)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    // ---- Alert Events ----

    pub async fn insert_corridor_alert_event(
        &self,
        config_id: &str,
        user_id: &str,
        corridor_key: &str,
        alert_type: &str,
        severity: &str,
        message: &str,
        old_value: Option<f64>,
        new_value: Option<f64>,
        threshold_value: Option<f64>,
    ) -> Result<CorridorAlertEvent> {
        let id = Uuid::new_v4().to_string();
        let event = sqlx::query_as::<_, CorridorAlertEvent>(
            r"
            INSERT INTO corridor_alert_events (
                id, config_id, user_id, corridor_key, alert_type, severity,
                message, old_value, new_value, threshold_value
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(config_id)
        .bind(user_id)
        .bind(corridor_key)
        .bind(alert_type)
        .bind(severity)
        .bind(message)
        .bind(old_value)
        .bind(new_value)
        .bind(threshold_value)
        .fetch_one(self.pool())
        .await?;

        Ok(event)
    }

    pub async fn get_corridor_alert_events_for_user(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CorridorAlertEvent>> {
        let events = sqlx::query_as::<_, CorridorAlertEvent>(
            r"
            SELECT * FROM corridor_alert_events
            WHERE user_id = $1
            ORDER BY triggered_at DESC
            LIMIT $2
            ",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        Ok(events)
    }

    pub async fn get_corridor_alert_events_for_corridor(
        &self,
        corridor_key: &str,
        limit: i64,
    ) -> Result<Vec<CorridorAlertEvent>> {
        let events = sqlx::query_as::<_, CorridorAlertEvent>(
            r"
            SELECT * FROM corridor_alert_events
            WHERE corridor_key = $1
            ORDER BY triggered_at DESC
            LIMIT $2
            ",
        )
        .bind(corridor_key)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        Ok(events)
    }

    pub async fn get_alert_events_24h_for_user(&self, user_id: &str) -> Result<Vec<CorridorAlertEvent>> {
        let events = sqlx::query_as::<_, CorridorAlertEvent>(
            r"
            SELECT * FROM corridor_alert_events
            WHERE user_id = $1
              AND triggered_at >= datetime('now', '-1 day')
            ORDER BY triggered_at DESC
            ",
        )
        .bind(user_id)
        .fetch_all(self.pool())
        .await?;

        Ok(events)
    }

    pub async fn acknowledge_corridor_alert_event(
        &self,
        id: &str,
        user_id: &str,
    ) -> Result<()> {
        sqlx::query(
            r"
            UPDATE corridor_alert_events
            SET acknowledged = 1, acknowledged_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND user_id = $2
            ",
        )
        .bind(id)
        .bind(user_id)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn get_unacknowledged_count_for_user(&self, user_id: &str) -> Result<i64> {
        let result: (i64,) = sqlx::query_as(
            r"
            SELECT COUNT(*) FROM corridor_alert_events
            WHERE user_id = $1 AND acknowledged = 0
            ",
        )
        .bind(user_id)
        .fetch_one(self.pool())
        .await?;

        Ok(result.0)
    }
}
