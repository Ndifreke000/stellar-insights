use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{MetricRecord, SnapshotRecord};

pub struct MetricsDb {
    pool: SqlitePool,
}

impl MetricsDb {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Records a generic system or entity metric.
    #[tracing::instrument(skip(self), fields(metric_name = %name))]
    pub async fn record_metric(
        &self,
        name: &str,
        value: f64,
        entity_id: Option<String>,
        entity_type: Option<String>,
    ) -> Result<MetricRecord> {
        let id = Uuid::new_v4().to_string();
        let metric = sqlx::query_as::<_, MetricRecord>(
            r"
            INSERT INTO metrics (id, name, value, entity_id, entity_type, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(name)
        .bind(value)
        .bind(entity_id.clone())
        .bind(entity_type)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to record metric: name={}, entity_id={:?}",
                name, entity_id
            )
        })?;
        Ok(metric)
    }

    /// Creates a snapshot of an entity's state at a specific point in time or epoch.
    #[tracing::instrument(skip(self, data), fields(entity_id = %entity_id, entity_type = %entity_type))]
    pub async fn create_snapshot(
        &self,
        entity_id: &str,
        entity_type: &str,
        data: serde_json::Value,
        hash: Option<String>,
        epoch: Option<i64>,
    ) -> Result<SnapshotRecord> {
        let id = Uuid::new_v4().to_string();
        let snapshot = sqlx::query_as::<_, SnapshotRecord>(
            r"
            INSERT INTO snapshots (id, entity_id, entity_type, data, hash, epoch, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            ",
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
        .with_context(|| {
            format!(
                "Failed to create snapshot: entity_id={}, entity_type={}",
                entity_id, entity_type
            )
        })?;
        Ok(snapshot)
    }

    /// Retrieves a snapshot by its epoch number.
    #[tracing::instrument(skip(self), fields(epoch = epoch))]
    pub async fn get_snapshot_by_epoch(&self, epoch: i64) -> Result<Option<SnapshotRecord>> {
        let snapshot = sqlx::query_as::<_, SnapshotRecord>(
            r"
            SELECT * FROM snapshots WHERE epoch = $1 LIMIT 1
            ",
        )
        .bind(epoch)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch snapshot for epoch: {}", epoch))?;
        Ok(snapshot)
    }

    /// Lists snapshots with pagination.
    #[tracing::instrument(skip(self), fields(limit = limit, offset = offset))]
    pub async fn list_snapshots(&self, limit: i64, offset: i64) -> Result<Vec<SnapshotRecord>> {
        let snapshots = sqlx::query_as::<_, SnapshotRecord>(
            r"
            SELECT * FROM snapshots
            WHERE epoch IS NOT NULL
            ORDER BY epoch DESC
            LIMIT $1 OFFSET $2
            ",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to list snapshots (limit={}, offset={})",
                limit, offset
            )
        })?;
        Ok(snapshots)
    }
}
