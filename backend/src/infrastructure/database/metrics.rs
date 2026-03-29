// Metrics database operations - extracted from database.rs

use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::Utc;

use crate::infrastructure::database::Database;
use crate::models::{MetricRecord, SnapshotRecord, AnchorMetricsHistory};

impl Database {
    pub async fn record_metric(
        &self,
        name: &str,
        value: f64,
        entity_id: Option<String>,
        entity_type: Option<String>,
    ) -> Result<MetricRecord> {
        self.execute_with_timing("record_metric", async {
            let id = Uuid::new_v4().to_string();
            sqlx::query_as::<_, MetricRecord>(
                r#"
                INSERT INTO metrics (id, name, value, entity_id, entity_type, timestamp)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING *
                "#,
            )
            .bind(id)
            .bind(name)
            .bind(value)
            .bind(entity_id)
            .bind(entity_type)
            .bind(Utc::now())
            .fetch_one(&self.pool)
            .await
        })
        .await
    }

    pub async fn create_snapshot(
        &self,
        entity_id: &str,
        entity_type: &str,
        data: serde_json::Value,
        hash: Option<String>,
        epoch: Option<i64>,
    ) -> Result<SnapshotRecord> {
        self.execute_with_timing("create_snapshot", async {
            let id = Uuid::new_v4().to_string();
            sqlx::query_as::<_, SnapshotRecord>(
                r#"
                INSERT INTO snapshots (id, entity_id, entity_type, data, hash, epoch, timestamp)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING *
                "#,
            )
            .bind(id)
            .bind(entity_id)
            .bind(entity_type)
            .bind(data.to_string())
            .bind(hash)
            .bind(epoch)
            .bind(Utc::now())
            .fetch_one(&self.pool)
            .await
        })
        .await
    }

    pub async fn get_anchor_metrics_history(
        &self,
        anchor_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AnchorMetricsHistory>> {
        self.execute_with_timing("get_anchor_metrics_history", async {
            sqlx::query_as::<_, AnchorMetricsHistory>(
                r#"
                SELECT * FROM anchor_metrics_history
                WHERE anchor_id = $1
                ORDER BY timestamp DESC
                LIMIT $2
                "#,
            )
            .bind(anchor_id.to_string())
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        })
        .await
    }
}

