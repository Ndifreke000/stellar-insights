use async_graphql::*;
use sqlx::SqlitePool;
use std::sync::Arc;

use super::types::*;

pub struct QueryRoot {
    pub pool: Arc<SqlitePool>,
}

#[Object]
impl QueryRoot {
    /// Get a single anchor by ID
    async fn anchor(&self, ctx: &Context<'_>, id: String) -> Result<Option<AnchorType>> {
        let pool = &self.pool;
        
        let anchor = sqlx::query_as!(
            AnchorType,
            r#"
            SELECT 
                id, name, stellar_account, home_domain,
                total_transactions, successful_transactions, failed_transactions,
                total_volume_usd, avg_settlement_time_ms, reliability_score,
                status, created_at as "created_at: _", updated_at as "updated_at: _"
            FROM anchors
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool.as_ref())
        .await?;

        Ok(anchor)
    }

    /// Get all anchors with optional filtering and pagination
    async fn anchors(
        &self,
        ctx: &Context<'_>,
        filter: Option<AnchorFilter>,
        pagination: Option<PaginationInput>,
    ) -> Result<AnchorsConnection> {
        let pool = &self.pool;
        let limit = pagination.as_ref().and_then(|p| p.limit).unwrap_or(10).min(100);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        let mut query = String::from("SELECT id, name, stellar_account, home_domain, total_transactions, successful_transactions, failed_transactions, total_volume_usd, avg_settlement_time_ms, reliability_score, status, created_at, updated_at FROM anchors WHERE 1=1");
        let mut count_query = String::from("SELECT COUNT(*) as count FROM anchors WHERE 1=1");

        if let Some(f) = &filter {
            if let Some(status) = &f.status {
                query.push_str(&format!(" AND status = '{}'", status));
                count_query.push_str(&format!(" AND status = '{}'", status));
            }
            if let Some(min_score) = f.min_reliability_score {
                query.push_str(&format!(" AND reliability_score >= {}", min_score));
                count_query.push_str(&format!(" AND reliability_score >= {}", min_score));
            }
            if let Some(search) = &f.search {
                query.push_str(&format!(" AND (name LIKE '%{}%' OR stellar_account LIKE '%{}%')", search, search));
                count_query.push_str(&format!(" AND (name LIKE '%{}%' OR stellar_account LIKE '%{}%')", search, search));
            }
        }

        query.push_str(&format!(" ORDER BY reliability_score DESC LIMIT {} OFFSET {}", limit, offset));

        let anchors = sqlx::query_as::<_, AnchorType>(&query)
            .fetch_all(pool.as_ref())
            .await?;

        let total: (i32,) = sqlx::query_as(&count_query)
            .fetch_one(pool.as_ref())
            .await?;

        Ok(AnchorsConnection {
            nodes: anchors,
            total_count: total.0,
            has_next_page: (offset + limit) < total.0,
        })
    }

    /// Get a single corridor by ID
    async fn corridor(&self, ctx: &Context<'_>, id: String) -> Result<Option<CorridorType>> {
        let pool = &self.pool;
        
        let corridor = sqlx::query_as!(
            CorridorType,
            r#"
            SELECT 
                id, source_asset_code, source_asset_issuer,
                destination_asset_code, destination_asset_issuer,
                reliability_score, status,
                created_at as "created_at: _", updated_at as "updated_at: _"
            FROM corridors
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool.as_ref())
        .await?;

        Ok(corridor)
    }

    /// Get all corridors with optional filtering and pagination
    async fn corridors(
        &self,
        ctx: &Context<'_>,
        filter: Option<CorridorFilter>,
        pagination: Option<PaginationInput>,
    ) -> Result<CorridorsConnection> {
        let pool = &self.pool;
        let limit = pagination.as_ref().and_then(|p| p.limit).unwrap_or(10).min(100);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        let mut query = String::from("SELECT id, source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer, reliability_score, status, created_at, updated_at FROM corridors WHERE 1=1");
        let mut count_query = String::from("SELECT COUNT(*) as count FROM corridors WHERE 1=1");

        if let Some(f) = &filter {
            if let Some(source) = &f.source_asset_code {
                query.push_str(&format!(" AND source_asset_code = '{}'", source));
                count_query.push_str(&format!(" AND source_asset_code = '{}'", source));
            }
            if let Some(dest) = &f.destination_asset_code {
                query.push_str(&format!(" AND destination_asset_code = '{}'", dest));
                count_query.push_str(&format!(" AND destination_asset_code = '{}'", dest));
            }
            if let Some(status) = &f.status {
                query.push_str(&format!(" AND status = '{}'", status));
                count_query.push_str(&format!(" AND status = '{}'", status));
            }
            if let Some(min_score) = f.min_reliability_score {
                query.push_str(&format!(" AND reliability_score >= {}", min_score));
                count_query.push_str(&format!(" AND reliability_score >= {}", min_score));
            }
        }

        query.push_str(&format!(" ORDER BY reliability_score DESC LIMIT {} OFFSET {}", limit, offset));

        let corridors = sqlx::query_as::<_, CorridorType>(&query)
            .fetch_all(pool.as_ref())
            .await?;

        let total: (i32,) = sqlx::query_as(&count_query)
            .fetch_one(pool.as_ref())
            .await?;

        Ok(CorridorsConnection {
            nodes: corridors,
            total_count: total.0,
            has_next_page: (offset + limit) < total.0,
        })
    }

    /// Get assets for a specific anchor
    async fn assets_by_anchor(&self, ctx: &Context<'_>, anchor_id: String) -> Result<Vec<AssetType>> {
        let pool = &self.pool;
        
        let assets = sqlx::query_as!(
            AssetType,
            r#"
            SELECT 
                id, anchor_id, asset_code, asset_issuer,
                total_supply, num_holders,
                created_at as "created_at: _", updated_at as "updated_at: _"
            FROM assets
            WHERE anchor_id = ?
            ORDER BY num_holders DESC
            "#,
            anchor_id
        )
        .fetch_all(pool.as_ref())
        .await?;

        Ok(assets)
    }

    /// Get metrics for an entity within a time range
    async fn metrics(
        &self,
        ctx: &Context<'_>,
        entity_id: Option<String>,
        entity_type: Option<String>,
        time_range: Option<TimeRangeInput>,
        pagination: Option<PaginationInput>,
    ) -> Result<Vec<MetricType>> {
        let pool = &self.pool;
        let limit = pagination.as_ref().and_then(|p| p.limit).unwrap_or(100).min(1000);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        let mut query = String::from("SELECT id, name, value, entity_id, entity_type, timestamp, created_at FROM metrics WHERE 1=1");

        if let Some(eid) = &entity_id {
            query.push_str(&format!(" AND entity_id = '{}'", eid));
        }
        if let Some(etype) = &entity_type {
            query.push_str(&format!(" AND entity_type = '{}'", etype));
        }
        if let Some(tr) = &time_range {
            query.push_str(&format!(" AND timestamp >= '{}' AND timestamp <= '{}'", tr.start, tr.end));
        }

        query.push_str(&format!(" ORDER BY timestamp DESC LIMIT {} OFFSET {}", limit, offset));

        let metrics = sqlx::query_as::<_, MetricType>(&query)
            .fetch_all(pool.as_ref())
            .await?;

        Ok(metrics)
    }

    /// Get latest snapshot for an entity
    async fn latest_snapshot(
        &self,
        ctx: &Context<'_>,
        entity_id: String,
        entity_type: String,
    ) -> Result<Option<SnapshotType>> {
        let pool = &self.pool;
        
        let snapshot = sqlx::query_as!(
            SnapshotType,
            r#"
            SELECT 
                id, entity_id, entity_type, data, hash, epoch,
                timestamp as "timestamp: _", created_at as "created_at: _"
            FROM snapshots
            WHERE entity_id = ? AND entity_type = ?
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            entity_id,
            entity_type
        )
        .fetch_optional(pool.as_ref())
        .await?;

        Ok(snapshot)
    }

    /// Search across anchors and corridors
    async fn search(
        &self,
        ctx: &Context<'_>,
        query: String,
        limit: Option<i32>,
    ) -> Result<SearchResults> {
        let pool = &self.pool;
        let search_limit = limit.unwrap_or(10).min(50);

        let anchors = sqlx::query_as::<_, AnchorType>(&format!(
            "SELECT id, name, stellar_account, home_domain, total_transactions, successful_transactions, failed_transactions, total_volume_usd, avg_settlement_time_ms, reliability_score, status, created_at, updated_at FROM anchors WHERE name LIKE '%{}%' OR stellar_account LIKE '%{}%' LIMIT {}",
            query, query, search_limit
        ))
        .fetch_all(pool.as_ref())
        .await?;

        let corridors = sqlx::query_as::<_, CorridorType>(&format!(
            "SELECT id, source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer, reliability_score, status, created_at, updated_at FROM corridors WHERE source_asset_code LIKE '%{}%' OR destination_asset_code LIKE '%{}%' LIMIT {}",
            query, query, search_limit
        ))
        .fetch_all(pool.as_ref())
        .await?;

        Ok(SearchResults { anchors, corridors })
    }
}

/// Search results combining multiple entity types
#[derive(Debug, Clone, SimpleObject)]
pub struct SearchResults {
    pub anchors: Vec<AnchorType>,
    pub corridors: Vec<CorridorType>,
}

pub struct MutationRoot {
    pub pool: Arc<SqlitePool>,
}

#[Object]
impl MutationRoot {
    /// Placeholder for future mutations
    async fn placeholder(&self) -> Result<bool> {
        Ok(true)
    }
}
