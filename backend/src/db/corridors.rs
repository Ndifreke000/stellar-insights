use anyhow::{Context, Result};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::cache::CacheManager;
use crate::models::CorridorRecord;

pub struct CorridorDb {
    pool: SqlitePool,
}

impl CorridorDb {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates a new corridor or updates its timestamp on conflict.
    #[tracing::instrument(skip(self, req), fields(source = %req.source_asset_code, dest = %req.dest_asset_code))]
    pub async fn create_corridor(
        &self,
        req: crate::models::CreateCorridorRequest,
    ) -> Result<crate::models::corridor::Corridor> {
        let corridor = crate::models::corridor::Corridor::new(
            req.source_asset_code,
            req.source_asset_issuer,
            req.dest_asset_code,
            req.dest_asset_issuer,
        );
        sqlx::query(
            r"
            INSERT INTO corridors (
                id, source_asset_code, source_asset_issuer,
                destination_asset_code, destination_asset_issuer
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer)
            DO UPDATE SET updated_at = CURRENT_TIMESTAMP
            ",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&corridor.source_asset_code)
        .bind(&corridor.source_asset_issuer)
        .bind(&corridor.destination_asset_code)
        .bind(&corridor.destination_asset_issuer)
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to create corridor: {}:{} -> {}:{}",
                corridor.source_asset_code,
                corridor.source_asset_issuer,
                corridor.destination_asset_code,
                corridor.destination_asset_issuer
            )
        })?;
        Ok(corridor)
    }

    /// Lists all corridors with pagination, sorted by reliability score.
    #[tracing::instrument(skip(self), fields(limit = limit, offset = offset))]
    pub async fn list_corridors(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::corridor::Corridor>> {
        let records = sqlx::query_as::<_, CorridorRecord>(
            r"
            SELECT * FROM corridors ORDER BY reliability_score DESC LIMIT $1 OFFSET $2
            ",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to list corridors (limit={}, offset={})",
                limit, offset
            )
        })?;

        let corridors = records
            .into_iter()
            .map(|r| {
                crate::models::corridor::Corridor::new(
                    r.source_asset_code,
                    r.source_asset_issuer,
                    r.destination_asset_code,
                    r.destination_asset_issuer,
                )
            })
            .collect::<Vec<_>>();
        Ok(corridors)
    }

    /// Retrieves a corridor by its ID.
    #[tracing::instrument(skip(self), fields(corridor_id = %id))]
    pub async fn get_corridor_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::corridor::Corridor>> {
        let record = sqlx::query_as::<_, CorridorRecord>(
            r"
            SELECT * FROM corridors WHERE id = $1
            ",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch corridor with id: {}", id))?;

        Ok(record.map(|r| {
            crate::models::corridor::Corridor::new(
                r.source_asset_code,
                r.source_asset_issuer,
                r.destination_asset_code,
                r.destination_asset_issuer,
            )
        }))
    }

    /// Updates the metrics for a corridor and invalidates the cache.
    #[tracing::instrument(skip(self, cache))]
    pub async fn update_corridor_metrics(
        &self,
        id: Uuid,
        metrics: crate::models::corridor::CorridorMetrics,
        cache: &CacheManager,
    ) -> Result<crate::models::corridor::Corridor> {
        let record = sqlx::query_as::<_, CorridorRecord>(
            r"
            UPDATE corridors
            SET reliability_score = $1,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            RETURNING *
            ",
        )
        .bind(metrics.success_rate)
        .bind(id.to_string())
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to update corridor metrics for id: {}", id))?;

        let corridor = crate::models::corridor::Corridor::new(
            record.source_asset_code,
            record.source_asset_issuer,
            record.destination_asset_code,
            record.destination_asset_issuer,
        );

        let corridor_key = corridor.to_string_key();
        let _ = cache.invalidate_corridor(&corridor_key).await.map_err(|e| {
            tracing::warn!(
                "Failed to invalidate cache for corridor {}: {}",
                corridor_key,
                e
            );
        });

        Ok(corridor)
    }

    /// Latest `corridor_metrics` row per `corridor_key` in a single query.
    #[tracing::instrument(skip(self))]
    pub async fn fetch_latest_corridor_metrics_for_broadcast(
        &self,
    ) -> Result<Vec<crate::models::corridor::CorridorMetrics>> {
        sqlx::query_as::<_, crate::models::corridor::CorridorMetrics>(
            r"
                SELECT cm.*
                FROM corridor_metrics cm
                INNER JOIN (
                    SELECT corridor_key, MAX(date) AS max_date
                    FROM corridor_metrics
                    GROUP BY corridor_key
                ) latest
                  ON cm.corridor_key = latest.corridor_key
                 AND cm.date = latest.max_date
                ORDER BY cm.corridor_key
                ",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch latest corridor metrics for broadcast")
    }
}
