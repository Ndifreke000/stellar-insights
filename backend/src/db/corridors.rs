use anyhow::{Context, Result};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::cache::CacheManager;
use crate::models::{corridor::Corridor, corridor::CorridorMetrics, CorridorRecord, CreateCorridorRequest};

pub struct CorridorDb {
    pool: SqlitePool,
}

impl CorridorDb {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, req: CreateCorridorRequest) -> Result<Corridor> {
        let corridor = Corridor::new(
            req.source_asset_code,
            req.source_asset_issuer,
            req.dest_asset_code,
            req.dest_asset_issuer,
        );
        sqlx::query(
            "INSERT INTO corridors (
                id, source_asset_code, source_asset_issuer,
                destination_asset_code, destination_asset_issuer
             ) VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer)
             DO UPDATE SET updated_at = CURRENT_TIMESTAMP",
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

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Corridor>> {
        let records = sqlx::query_as::<_, CorridorRecord>(
            "SELECT * FROM corridors ORDER BY reliability_score DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to list corridors (limit={limit}, offset={offset})"))?;

        Ok(records
            .into_iter()
            .map(|r| {
                Corridor::new(
                    r.source_asset_code,
                    r.source_asset_issuer,
                    r.destination_asset_code,
                    r.destination_asset_issuer,
                )
            })
            .collect())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Corridor>> {
        let record = sqlx::query_as::<_, CorridorRecord>(
            "SELECT * FROM corridors WHERE id = $1",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch corridor: {id}"))?;

        Ok(record.map(|r| {
            Corridor::new(
                r.source_asset_code,
                r.source_asset_issuer,
                r.destination_asset_code,
                r.destination_asset_issuer,
            )
        }))
    }

    pub async fn update_metrics(
        &self,
        id: Uuid,
        metrics: CorridorMetrics,
        cache: &CacheManager,
    ) -> Result<Corridor> {
        let record = sqlx::query_as::<_, CorridorRecord>(
            "UPDATE corridors SET reliability_score = $1, updated_at = CURRENT_TIMESTAMP
             WHERE id = $2 RETURNING *",
        )
        .bind(metrics.success_rate)
        .bind(id.to_string())
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to update corridor metrics: {id}"))?;

        let corridor = Corridor::new(
            record.source_asset_code,
            record.source_asset_issuer,
            record.destination_asset_code,
            record.destination_asset_issuer,
        );

        let key = corridor.to_string_key();
        let _ = cache.invalidate_corridor(&key).await.map_err(|e| {
            tracing::warn!("Failed to invalidate cache for corridor {key}: {e}");
        });

        Ok(corridor)
    }
}
