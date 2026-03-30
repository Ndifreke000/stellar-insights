use async_graphql::*;
use sqlx::{QueryBuilder, SqlitePool};
use std::sync::Arc;

use super::types::*;

pub struct QueryRoot {
    pub pool: Arc<SqlitePool>,
}

#[Object]
impl QueryRoot {
    /// Get a single anchor by ID
    async fn anchor(&self, _ctx: &Context<'_>, id: String) -> Result<Option<AnchorType>> {
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
        _ctx: &Context<'_>,
        filter: Option<AnchorFilter>,
        pagination: Option<PaginationInput>,
    ) -> Result<AnchorsConnection> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(10)
            .min(100);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        // Build parameterized query for data
        let mut query_builder = QueryBuilder::new(
            "SELECT id, name, stellar_account, home_domain, total_transactions, successful_transactions, failed_transactions, total_volume_usd, avg_settlement_time_ms, reliability_score, status, created_at, updated_at FROM anchors WHERE 1=1"
        );

        if let Some(f) = &filter {
            if let Some(status) = &f.status {
                query_builder.push(" AND status = ");
                query_builder.push_bind(status);
            }
            if let Some(min_score) = f.min_reliability_score {
                query_builder.push(" AND reliability_score >= ");
                query_builder.push_bind(min_score);
            }
            if let Some(search) = &f.search {
                query_builder.push(" AND (name LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(" OR stellar_account LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(")");
            }
        }

        query_builder.push(" ORDER BY reliability_score DESC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let anchors = query_builder
            .build_query_as::<AnchorType>()
            .fetch_all(pool.as_ref())
            .await?;

        // Build parameterized query for count
        let mut count_builder = QueryBuilder::new("SELECT COUNT(*) as count FROM anchors WHERE 1=1");

        if let Some(f) = &filter {
            if let Some(status) = &f.status {
                count_builder.push(" AND status = ");
                count_builder.push_bind(status);
            }
            if let Some(min_score) = f.min_reliability_score {
                count_builder.push(" AND reliability_score >= ");
                count_builder.push_bind(min_score);
            }
            if let Some(search) = &f.search {
                count_builder.push(" AND (name LIKE ");
                count_builder.push_bind(format!("%{}%", search));
                count_builder.push(" OR stellar_account LIKE ");
                count_builder.push_bind(format!("%{}%", search));
                count_builder.push(")");
            }
        }

        let (total,): (i64,) = count_builder
            .build_query_as()
            .fetch_one(pool.as_ref())
            .await?;

        Ok(AnchorsConnection {
            nodes: anchors,
            total_count: total,
            has_next_page: (offset + limit) < total,
        })
    }

    /// Get a single corridor by ID
    async fn corridor(
        &self,
        _ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<CorridorType>> {
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
        _ctx: &Context<'_>,
        filter: Option<CorridorFilter>,
        pagination: Option<PaginationInput>,
    ) -> Result<CorridorsConnection> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(10)
            .min(100);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        // Build parameterized query for data
        let mut query_builder = QueryBuilder::new(
            "SELECT id, source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer, reliability_score, status, created_at, updated_at FROM corridors WHERE 1=1"
        );

        if let Some(f) = &filter {
            if let Some(source) = &f.source_asset_code {
                query_builder.push(" AND source_asset_code = ");
                query_builder.push_bind(source);
            }
            if let Some(dest) = &f.destination_asset_code {
                query_builder.push(" AND destination_asset_code = ");
                query_builder.push_bind(dest);
            }
            if let Some(status) = &f.status {
                query_builder.push(" AND status = ");
                query_builder.push_bind(status);
            }
            if let Some(min_score) = f.min_reliability_score {
                query_builder.push(" AND reliability_score >= ");
                query_builder.push_bind(min_score);
            }
        }

        query_builder.push(" ORDER BY reliability_score DESC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let corridors = query_builder
            .build_query_as::<CorridorType>()
            .fetch_all(pool.as_ref())
            .await?;

        // Build parameterized query for count
        let mut count_builder = QueryBuilder::new("SELECT COUNT(*) as count FROM corridors WHERE 1=1");

        if let Some(f) = &filter {
            if let Some(source) = &f.source_asset_code {
                count_builder.push(" AND source_asset_code = ");
                count_builder.push_bind(source);
            }
            if let Some(dest) = &f.destination_asset_code {
                count_builder.push(" AND destination_asset_code = ");
                count_builder.push_bind(dest);
            }
            if let Some(status) = &f.status {
                count_builder.push(" AND status = ");
                count_builder.push_bind(status);
            }
            if let Some(min_score) = f.min_reliability_score {
                count_builder.push(" AND reliability_score >= ");
                count_builder.push_bind(min_score);
            }
        }

        let (total,): (i64,) = count_builder
            .build_query_as()
            .fetch_one(pool.as_ref())
            .await?;

        Ok(CorridorsConnection {
            nodes: corridors,
            total_count: total,
            has_next_page: (offset + limit) < total,
        })
    }

    /// Get assets for a specific anchor
    async fn assets_by_anchor(
        &self,
        _ctx: &Context<'_>,
        anchor_id: String,
    ) -> Result<Vec<AssetType>> {
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
        _ctx: &Context<'_>,
        entity_id: Option<String>,
        entity_type: Option<String>,
        time_range: Option<TimeRangeInput>,
        pagination: Option<PaginationInput>,
    ) -> Result<Vec<MetricType>> {
        let pool = &self.pool;
        let limit = pagination
            .as_ref()
            .and_then(|p| p.limit)
            .unwrap_or(100)
            .min(1000);
        let offset = pagination.as_ref().and_then(|p| p.offset).unwrap_or(0);

        // Build parameterized query
        let mut query_builder = QueryBuilder::new(
            "SELECT id, name, value, entity_id, entity_type, timestamp, created_at FROM metrics WHERE 1=1"
        );

        if let Some(eid) = &entity_id {
            query_builder.push(" AND entity_id = ");
            query_builder.push_bind(eid);
        }
        if let Some(etype) = &entity_type {
            query_builder.push(" AND entity_type = ");
            query_builder.push_bind(etype);
        }
        if let Some(tr) = &time_range {
            query_builder.push(" AND timestamp >= ");
            query_builder.push_bind(&tr.start);
            query_builder.push(" AND timestamp <= ");
            query_builder.push_bind(&tr.end);
        }

        query_builder.push(" ORDER BY timestamp DESC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let metrics = query_builder
            .build_query_as::<MetricType>()
            .fetch_all(pool.as_ref())
            .await?;
        Ok(metrics)
    }

    /// Get latest snapshot for an entity
    async fn latest_snapshot(
        &self,
        _ctx: &Context<'_>,
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
        _ctx: &Context<'_>,
        query: String,
        limit: Option<i32>,
    ) -> Result<SearchResults> {
        let pool = &self.pool;
        let search_limit = limit.unwrap_or(10).min(50);
        let search_pattern = format!("%{}%", query);

        // Build parameterized query for anchors
        let mut anchor_builder = QueryBuilder::new(
            "SELECT id, name, stellar_account, home_domain, total_transactions, successful_transactions, failed_transactions, total_volume_usd, avg_settlement_time_ms, reliability_score, status, created_at, updated_at FROM anchors WHERE name LIKE "
        );
        anchor_builder.push_bind(&search_pattern);
        anchor_builder.push(" OR stellar_account LIKE ");
        anchor_builder.push_bind(&search_pattern);
        anchor_builder.push(" LIMIT ");
        anchor_builder.push_bind(search_limit);

        let anchors = anchor_builder
            .build_query_as::<AnchorType>()
            .fetch_all(pool.as_ref())
            .await?;

        // Build parameterized query for corridors
        let mut corridor_builder = QueryBuilder::new(
            "SELECT id, source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer, reliability_score, status, created_at, updated_at FROM corridors WHERE source_asset_code LIKE "
        );
        corridor_builder.push_bind(&search_pattern);
        corridor_builder.push(" OR destination_asset_code LIKE ");
        corridor_builder.push_bind(&search_pattern);
        corridor_builder.push(" LIMIT ");
        corridor_builder.push_bind(search_limit);

        let corridors = corridor_builder
            .build_query_as::<CorridorType>()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that SQL injection payloads are treated as literal strings,
    /// not executable SQL, by sqlx parameterized binding.
    #[tokio::test]
    async fn test_sql_injection_prevention() {
        let pool = Arc::new(SqlitePool::connect(":memory:").await.unwrap());

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS anchors (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL DEFAULT '',
                stellar_account TEXT NOT NULL DEFAULT '',
                home_domain TEXT,
                total_transactions INTEGER NOT NULL DEFAULT 0,
                successful_transactions INTEGER NOT NULL DEFAULT 0,
                failed_transactions INTEGER NOT NULL DEFAULT 0,
                total_volume_usd REAL NOT NULL DEFAULT 0.0,
                avg_settlement_time_ms INTEGER NOT NULL DEFAULT 0,
                reliability_score REAL NOT NULL DEFAULT 0.0,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT ''
            )",
        )
        .execute(pool.as_ref())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO anchors (id, name, stellar_account, status, reliability_score)
             VALUES ('1', 'TestAnchor', 'GTEST', 'active', 90.0)",
        )
        .execute(pool.as_ref())
        .await
        .unwrap();

        // Classic tautology injection — bypasses string-built SQL but not parameterized
        let malicious_status = "' OR '1'='1".to_string();
        let sql = "SELECT COUNT(*) as count FROM anchors WHERE 1=1 AND status = ?";

        let (count,): (i32,) = sqlx::query_as::<_, (i32,)>(sql)
            .bind(malicious_status)
            .fetch_one(pool.as_ref())
            .await
            .unwrap();

        assert_eq!(count, 0, "Injection payload must not match any rows");

        // Sanity: legitimate query still works
        let (legit_count,): (i32,) =
            sqlx::query_as::<_, (i32,)>(sql)
                .bind("active")
                .fetch_one(pool.as_ref())
                .await
                .unwrap();

        assert_eq!(legit_count, 1, "Legitimate query must return correct results");
    }
}
