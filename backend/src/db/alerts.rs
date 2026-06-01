use crate::models::alerts::{
    AlertHistory, AlertRule, CreateAlertRuleRequest, SnoozeAlertRequest, UpdateAlertRuleRequest,
};
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

impl crate::database::Database {
    // Alert Rule Operations
    pub async fn create_alert_rule(
        &self,
        user_id: &str,
        req: CreateAlertRuleRequest,
    ) -> Result<AlertRule> {
        let id = Uuid::new_v4().to_string();
        let rule = sqlx::query_as::<_, AlertRule>(
            r#"
            INSERT INTO alert_rules (
                id, user_id, corridor_id, metric_type, condition, threshold,
                notify_email, notify_webhook, notify_in_app
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&req.corridor_id)
        .bind(&req.metric_type)
        .bind(&req.condition)
        .bind(req.threshold)
        .bind(req.notify_email)
        .bind(req.notify_webhook)
        .bind(req.notify_in_app)
        .fetch_one(self.pool())
        .await?;

        Ok(rule)
    }

    pub async fn get_alert_rules_for_user(&self, user_id: &str) -> Result<Vec<AlertRule>> {
        let rules = sqlx::query_as::<_, AlertRule>(
            r#"
            SELECT * FROM alert_rules
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(self.pool())
        .await?;

        Ok(rules)
    }

    pub async fn get_all_active_alert_rules(&self) -> Result<Vec<AlertRule>> {
        let rules = sqlx::query_as::<_, AlertRule>(
            r#"
            SELECT * FROM alert_rules
            WHERE is_active = 1
            "#,
        )
        .fetch_all(self.pool())
        .await?;

        Ok(rules)
    }

    pub async fn update_alert_rule(
        &self,
        id: &str,
        user_id: &str,
        req: UpdateAlertRuleRequest,
    ) -> Result<AlertRule> {
        // Build dynamic update query
        let mut query = String::from("UPDATE alert_rules SET updated_at = CURRENT_TIMESTAMP");

        if req.corridor_id.is_some() {
            query.push_str(", corridor_id = $3");
        }
        if req.metric_type.is_some() {
            query.push_str(", metric_type = $4");
        }
        if req.condition.is_some() {
            query.push_str(", condition = $5");
        }
        if req.threshold.is_some() {
            query.push_str(", threshold = $6");
        }
        if req.notify_email.is_some() {
            query.push_str(", notify_email = $7");
        }
        if req.notify_webhook.is_some() {
            query.push_str(", notify_webhook = $8");
        }
        if req.notify_in_app.is_some() {
            query.push_str(", notify_in_app = $9");
        }
        if req.is_active.is_some() {
            query.push_str(", is_active = $10");
        }

        query.push_str(" WHERE id = $1 AND user_id = $2 RETURNING *");

        let mut q = sqlx::query_as::<_, AlertRule>(&query)
            .bind(id)
            .bind(user_id);

        if req.corridor_id.is_some() {
            q = q.bind(&req.corridor_id);
        } else {
            q = q.bind(None::<String>);
        }
        if let Some(m) = &req.metric_type {
            q = q.bind(m);
        } else {
            q = q.bind("");
        }
        if let Some(c) = &req.condition {
            q = q.bind(c);
        } else {
            q = q.bind("");
        }
        if let Some(t) = req.threshold {
            q = q.bind(t);
        } else {
            q = q.bind(0.0);
        }
        if let Some(e) = req.notify_email {
            q = q.bind(e);
        } else {
            q = q.bind(false);
        }
        if let Some(w) = req.notify_webhook {
            q = q.bind(w);
        } else {
            q = q.bind(false);
        }
        if let Some(i) = req.notify_in_app {
            q = q.bind(i);
        } else {
            q = q.bind(false);
        }
        if let Some(a) = req.is_active {
            q = q.bind(a);
        } else {
            q = q.bind(false);
        }

        let rule = q.fetch_one(self.pool()).await?;
        Ok(rule)
    }

    pub async fn delete_alert_rule(&self, id: &str, user_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM alert_rules WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn snooze_alert_rule(
        &self,
        id: &str,
        user_id: &str,
        req: SnoozeAlertRequest,
    ) -> Result<AlertRule> {
        let rule = sqlx::query_as::<_, AlertRule>(
            r#"
            UPDATE alert_rules
            SET snoozed_until = $3, updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND user_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(req.snoozed_until)
        .fetch_one(self.pool())
        .await?;

        Ok(rule)
    }

    // Alert History Operations
    pub async fn insert_alert_history(
        &self,
        rule_id: &str,
        user_id: &str,
        corridor_id: Option<String>,
        metric_type: &str,
        trigger_value: f64,
        threshold_value: f64,
        condition: &str,
        message: &str,
    ) -> Result<AlertHistory> {
        let id = Uuid::new_v4().to_string();
        let history = sqlx::query_as::<_, AlertHistory>(
            r#"
            INSERT INTO alert_history (
                id, rule_id, user_id, corridor_id, metric_type,
                trigger_value, threshold_value, condition, message
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(rule_id)
        .bind(user_id)
        .bind(corridor_id)
        .bind(metric_type)
        .bind(trigger_value)
        .bind(threshold_value)
        .bind(condition)
        .bind(message)
        .fetch_one(self.pool())
        .await?;

        Ok(history)
    }

    pub async fn get_alert_history_for_user(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<AlertHistory>> {
        let history = sqlx::query_as::<_, AlertHistory>(
            r#"
            SELECT * FROM alert_history
            WHERE user_id = $1
            ORDER BY triggered_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        Ok(history)
    }

    pub async fn mark_alert_history_read(&self, id: &str, user_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE alert_history
            SET is_read = 1
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn dismiss_alert_history(&self, id: &str, user_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE alert_history
            SET is_dismissed = 1
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(self.pool())
        .await?;

        Ok(())
    }
}
