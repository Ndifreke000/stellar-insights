use anyhow::{Context, Result};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::cache::CacheManager;
use crate::models::CorridorRecord;

/// Dimension along which a corridor is keyed.
///
/// Historically corridors were keyed only by asset pair. Issue #2861 generalises
/// the key to a dimension tuple so that anchor-pair corridors and saved custom
/// groupings can be represented alongside the legacy asset-pair corridors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorridorDimension {
    /// Legacy key: `source_asset -> destination_asset`.
    AssetPair,
    /// `anchor_a:source_asset -> anchor_b:destination_asset`.
    AnchorPair,
    /// A saved grouping of member corridors aggregated on the fly.
    CustomGrouping,
}

impl CorridorDimension {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AssetPair => "asset_pair",
            Self::AnchorPair => "anchor_pair",
            Self::CustomGrouping => "custom_grouping",
        }
    }

    #[must_use]
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "asset_pair" => Some(Self::AssetPair),
            "anchor_pair" => Some(Self::AnchorPair),
            "custom_grouping" => Some(Self::CustomGrouping),
            _ => None,
        }
    }
}

/// A saved custom grouping: a named set of member corridor keys that is
/// aggregated on the fly rather than stored as a single corridor row.
#[derive(Debug, Clone)]
pub struct CorridorGrouping {
    pub id: Uuid,
    pub name: String,
    pub dimension: CorridorDimension,
    pub members: Vec<String>,
}

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

    /// Builds the canonical dimension-tuple key for an asset-pair corridor.
    ///
    /// This is the migration target for legacy asset-pair keys: the tuple is
    /// `(dimension, source, dest)` so that anchor-pair and custom-grouping keys
    /// can never collide with asset-pair keys.
    #[must_use]
    pub fn asset_pair_key(source: &str, dest: &str) -> String {
        format!(
            "{}|{}|{}",
            CorridorDimension::AssetPair.as_str(),
            source,
            dest
        )
    }

    /// Builds the canonical dimension-tuple key for an anchor-pair corridor
    /// (e.g. `Anchor A USDC -> Anchor B NGN`).
    #[must_use]
    pub fn anchor_pair_key(
        source_anchor: &str,
        source_asset: &str,
        dest_anchor: &str,
        dest_asset: &str,
    ) -> String {
        format!(
            "{}|{}:{}|{}:{}",
            CorridorDimension::AnchorPair.as_str(),
            source_anchor,
            source_asset,
            dest_anchor,
            dest_asset
        )
    }

    /// Builds the canonical dimension-tuple key for a saved custom grouping.
    #[must_use]
    pub fn custom_grouping_key(grouping_id: Uuid) -> String {
        format!(
            "{}|{}",
            CorridorDimension::CustomGrouping.as_str(),
            grouping_id
        )
    }

    /// Parses a dimension-tuple key back into its dimension and components.
    ///
    /// Legacy keys (no `|` separator) are treated as asset-pair keys so that
    /// pre-migration rows keep resolving correctly.
    #[must_use]
    pub fn parse_key(key: &str) -> (CorridorDimension, Vec<String>) {
        match key.split_once('|') {
            Some((dim, rest)) => {
                let dimension =
                    CorridorDimension::from_str(dim).unwrap_or(CorridorDimension::AssetPair);
                let parts = rest.split('|').map(str::to_string).collect();
                (dimension, parts)
            }
            None => (
                CorridorDimension::AssetPair,
                key.split("->").map(str::trim).map(str::to_string).collect(),
            ),
        }
    }

    /// Persists a saved custom grouping and its member corridor keys.
    #[tracing::instrument(skip(self, members), fields(name = %name, members = members.len()))]
    pub async fn create_grouping(
        &self,
        name: &str,
        members: &[String],
    ) -> Result<CorridorGrouping> {
        if members.is_empty() {
            anyhow::bail!("custom grouping must have at least one member corridor");
        }
        let id = Uuid::new_v4();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start grouping tx")?;
        sqlx::query(
            r"
            INSERT INTO corridor_groupings (id, name, dimension)
            VALUES ($1, $2, $3)
            ",
        )
        .bind(id.to_string())
        .bind(name)
        .bind(CorridorDimension::CustomGrouping.as_str())
        .execute(&mut *tx)
        .await
        .with_context(|| format!("Failed to create grouping: {}", name))?;

        for member in members {
            sqlx::query(
                r"
                INSERT INTO corridor_grouping_members (grouping_id, corridor_key)
                VALUES ($1, $2)
                ",
            )
            .bind(id.to_string())
            .bind(member)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("Failed to add member {} to grouping {}", member, id))?;
        }
        tx.commit().await.context("Failed to commit grouping tx")?;

        Ok(CorridorGrouping {
            id,
            name: name.to_string(),
            dimension: CorridorDimension::CustomGrouping,
            members: members.to_vec(),
        })
    }

    /// Loads a saved custom grouping by id.
    #[tracing::instrument(skip(self), fields(grouping_id = %id))]
    pub async fn get_grouping(&self, id: Uuid) -> Result<Option<CorridorGrouping>> {
        let row: Option<(String, String)> = sqlx::query_as(
            r"
            SELECT id, name FROM corridor_groupings WHERE id = $1
            ",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch grouping: {}", id))?;

        let Some((id_str, name)) = row else {
            return Ok(None);
        };

        let members: Vec<(String,)> = sqlx::query_as(
            r"
            SELECT corridor_key FROM corridor_grouping_members WHERE grouping_id = $1
            ",
        )
        .bind(&id_str)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch members for grouping: {}", id))?;

        Ok(Some(CorridorGrouping {
            id,
            name,
            dimension: CorridorDimension::CustomGrouping,
            members: members.into_iter().map(|(k,)| k).collect(),
        }))
    }

    /// Aggregates a saved custom grouping on the fly by summing the latest
    /// metrics of each member corridor. A grouped corridor equals the sum of
    /// its members (see test plan for #2861).
    #[tracing::instrument(skip(self), fields(grouping_id = %id))]
    pub async fn aggregate_grouping(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::corridor::CorridorMetrics>> {
        let Some(grouping) = self.get_grouping(id).await? else {
            return Ok(None);
        };

        let mut total = crate::models::corridor::CorridorMetrics::default();
        for member in &grouping.members {
            let row: Option<crate::models::corridor::CorridorMetrics> = sqlx::query_as(
                r"
                SELECT cm.*
                FROM corridor_metrics cm
                INNER JOIN (
                    SELECT corridor_key, MAX(date) AS max_date
                    FROM corridor_metrics
                    WHERE corridor_key = $1
                    GROUP BY corridor_key
                ) latest
                  ON cm.corridor_key = latest.corridor_key
                 AND cm.date = latest.max_date
                ",
            )
            .bind(member)
            .fetch_optional(&self.pool)
            .await
            .with_context(|| format!("Failed to fetch metrics for member: {}", member))?;

            if let Some(metrics) = row {
                total = total.merge(&metrics);
            }
        }
        Ok(Some(total))
    }
}
