use async_graphql::*;
use sqlx::SqlitePool;
use std::fmt::Write;
use std::sync::Arc;

use super::types::*;

pub struct QueryRoot {
    pub pool: Arc<SqlitePool>,
}

#[derive(Default)]
struct AnchorQueryBuilder {
    conditions: Vec<String>,
}

impl AnchorQueryBuilder {
    fn from_filter(filter: Option<&AnchorFilter>) -> Self {
        let mut builder = Self::default();

        if let Some(f) = filter {
            if let Some(status) = &f.status {
                builder.add_status_filter(status);
            }
            if let Some(min_score) = f.min_reliability_score {
                builder.add_min_score_filter(min_score);
            }
            if let Some(search) = &f.search {
                builder.add_search_filter(search);
            }
        }

        builder
    }

    fn add_status_filter(&mut self, status: &str) {
        let escaped = escape_sql_literal(status);
        self.conditions.push(format!("status = '{escaped}'"));
    }

    fn add_min_score_filter(&mut self, min_score: f64) {
        self.conditions
            .push(format!("reliability_score >= {min_score}"));
    }

    fn add_search_filter(&mut self, search: &str) {
        let escaped = escape_sql_literal(search);
        self.conditions.push(format!(
            "(name LIKE '%{escaped}%' OR stellar_account LIKE '%{escaped}%')"
        ));
    }

    fn apply_conditions(&self, query: &mut String) {
        if !self.conditions.is_empty() {
            query.push_str(" AND ");
            query.push_str(&self.conditions.join(" AND "));
        }
    }

    fn build_data_query(&self, limit: i32, offset: i32) -> String {
        let mut query = String::from(
            "SELECT id, name, stellar_account, home_domain, total_transactions, successful_transactions, failed_transactions, total_volume_usd, avg_settlement_time_ms, reliability_score, status, created_at, updated_at FROM anchors WHERE 1=1",
        );
        self.apply_conditions(&mut query);
        write!(
            query,
            " ORDER BY reliability_score DESC LIMIT {} OFFSET {}",
            limit, offset
        )
        .unwrap();
        query
    }

    fn build_count_query(&self) -> String {
        let mut query = String::from("SELECT COUNT(*) as count FROM anchors WHERE 1=1");
        self.apply_conditions(&mut query);
        query
    }
}

fn escape_sql_literal(value: &str) -> String {
    value.replace('\'', "''")
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

        let query_builder = AnchorQueryBuilder::from_filter(filter.as_ref());
        let query = query_builder.build_data_query(limit, offset);
        let count_query = query_builder.build_count_query();

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
                write!(query, " AND source_asset_code = '{}'", source).unwrap();
                write!(count_query, " AND source_asset_code = '{}'", source).unwrap();
            }
            if let Some(dest) = &f.destination_asset_code {
                write!(query, " AND destination_asset_code = '{}'", dest).unwrap();
                write!(count_query, " AND destination_asset_code = '{}'", dest).unwrap();
            }
            if let Some(status) = &f.status {
                write!(query, " AND status = '{}'", status).unwrap();
                write!(count_query, " AND status = '{}'", status).unwrap();
            }
            if let Some(min_score) = f.min_reliability_score {
                write!(query, " AND reliability_score >= {}", min_score).unwrap();
                write!(count_query, " AND reliability_score >= {}", min_score).unwrap();
            }
        }

        write!(query, " ORDER BY reliability_score DESC LIMIT {} OFFSET {}", limit, offset).unwrap();

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
            write!(query, " AND entity_id = '{}'", eid).unwrap();
        }
        if let Some(etype) = &entity_type {
            write!(query, " AND entity_type = '{}'", etype).unwrap();
        }
        if let Some(tr) = &time_range {
            write!(query, " AND timestamp >= '{}' AND timestamp <= '{}'", tr.start, tr.end).unwrap();
        }

        write!(query, " ORDER BY timestamp DESC LIMIT {} OFFSET {}", limit, offset).unwrap();

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
