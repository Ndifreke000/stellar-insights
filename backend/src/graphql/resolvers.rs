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

        let mut data_qb = QueryBuilder::new(
            "SELECT id, name, stellar_account, home_domain, total_transactions, \
             successful_transactions, failed_transactions, total_volume_usd, \
             avg_settlement_time_ms, reliability_score, status, created_at, updated_at \
             FROM anchors WHERE 1=1",
        );
        let mut count_qb =
            QueryBuilder::new("SELECT COUNT(*) as count FROM anchors WHERE 1=1");

        if let Some(f) = &filter {
            if let Some(status) = &f.status {
                // Clone owned Strings so each QueryBuilder owns its bound value
                data_qb.push(" AND status = ").push_bind(status.clone());
                count_qb.push(" AND status = ").push_bind(status.clone());
            }
            if let Some(min_score) = f.min_reliability_score {
                data_qb.push(" AND reliability_score >= ").push_bind(min_score);
                count_qb.push(" AND reliability_score >= ").push_bind(min_score);
            }
            if let Some(search) = &f.search {
                let pattern = format!("%{}%", search);
                data_qb
                    .push(" AND (name LIKE ")
                    .push_bind(pattern.clone())
                    .push(" OR stellar_account LIKE ")
                    .push_bind(pattern.clone())
                    .push(")");
                count_qb
                    .push(" AND (name LIKE ")
                    .push_bind(pattern.clone())
                    .push(" OR stellar_account LIKE ")
                    .push_bind(pattern)
                    .push(")");
            }
        }

        data_qb
            .push(" ORDER BY reliability_score DESC LIMIT ")
            .push_bind(limit)
            .push(" OFFSET ")
            .push_bind(offset);

        let anchors = data_qb
            .build_query_as::<AnchorType>()
            .fetch_all(pool.as_ref())
            .await?;

        let total: (i32,) = count_qb
            .build_query_as()
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

        let mut data_qb = QueryBuilder::new(
            "SELECT id, source_asset_code, source_asset_issuer, destination_asset_code, \
             destination_asset_issuer, reliability_score, status, created_at, updated_at \
             FROM corridors WHERE 1=1",
        );
        let mut count_qb =
            QueryBuilder::new("SELECT COUNT(*) as count FROM corridors WHERE 1=1");

        if let Some(f) = &filter {
            if let Some(source) = &f.source_asset_code {
                data_qb.push(" AND source_asset_code = ").push_bind(source.clone());
                count_qb.push(" AND source_asset_code = ").push_bind(source.clone());
            }
            if let Some(dest) = &f.destination_asset_code {
                data_qb.push(" AND destination_asset_code = ").push_bind(dest.clone());
                count_qb.push(" AND destination_asset_code = ").push_bind(dest.clone());
            }
            if let Some(status) = &f.status {
                data_qb.push(" AND status = ").push_bind(status.clone());
                count_qb.push(" AND status = ").push_bind(status.clone());
            }
            if let Some(min_score) = f.min_reliability_score {
                data_qb.push(" AND reliability_score >= ").push_bind(min_score);
                count_qb.push(" AND reliability_score >= ").push_bind(min_score);
            }
        }

        data_qb
            .push(" ORDER BY reliability_score DESC LIMIT ")
            .push_bind(limit)
            .push(" OFFSET ")
            .push_bind(offset);

        let corridors = data_qb
            .build_query_as::<CorridorType>()
            .fetch_all(pool.as_ref())
            .await?;

        let total: (i32,) = count_qb
            .build_query_as()
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

        let mut qb = QueryBuilder::new(
            "SELECT id, name, value, entity_id, entity_type, timestamp, created_at \
             FROM metrics WHERE 1=1",
        );

        if let Some(eid) = entity_id {
            qb.push(" AND entity_id = ").push_bind(eid);
        }
        if let Some(etype) = entity_type {
            qb.push(" AND entity_type = ").push_bind(etype);
        }
        // Fix: DateTime<Utc> is bound directly — no .as_str()
        if let Some(tr) = time_range {
            qb.push(" AND timestamp >= ").push_bind(tr.start);
            qb.push(" AND timestamp <= ").push_bind(tr.end);
        }

        qb.push(" ORDER BY timestamp DESC LIMIT ")
            .push_bind(limit)
            .push(" OFFSET ")
            .push_bind(offset);

        let metrics = qb
            .build_query_as::<MetricType>()
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
        let pattern = format!("%{}%", query);

        // Fix: use let mut variables instead of chaining on temporaries
        let mut anchor_qb = QueryBuilder::new(
            "SELECT id, name, stellar_account, home_domain, total_transactions, \
             successful_transactions, failed_transactions, total_volume_usd, \
             avg_settlement_time_ms, reliability_score, status, created_at, updated_at \
             FROM anchors WHERE name LIKE ",
        );
        anchor_qb
            .push_bind(pattern.clone())
            .push(" OR stellar_account LIKE ")
            .push_bind(pattern.clone())
            .push(" LIMIT ")
            .push_bind(search_limit);

        let anchors = anchor_qb
            .build_query_as::<AnchorType>()
            .fetch_all(pool.as_ref())
            .await?;

        let mut corridor_qb = QueryBuilder::new(
            "SELECT id, source_asset_code, source_asset_issuer, destination_asset_code, \
             destination_asset_issuer, reliability_score, status, created_at, updated_at \
             FROM corridors WHERE source_asset_code LIKE ",
        );
        corridor_qb
            .push_bind(pattern.clone())
            .push(" OR destination_asset_code LIKE ")
            .push_bind(pattern)
            .push(" LIMIT ")
            .push_bind(search_limit);

        let corridors = corridor_qb
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
    /// not executable SQL, by the parameterized query builder.
    #[tokio::test]
    async fn test_sql_injection_prevention() {
        let pool = Arc::new(SqlitePool::connect(":memory:").await.unwrap());

        // Create minimal schema matching production types (avg_settlement_time_ms is INTEGER)
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

        // Injection attempt: classic tautology that bypasses string-built SQL
        let malicious_status = "' OR '1'='1".to_string();

        let mut qb = QueryBuilder::new("SELECT COUNT(*) as count FROM anchors WHERE 1=1");
        qb.push(" AND status = ").push_bind(malicious_status);

        let (count,): (i32,) = qb
            .build_query_as()
            .fetch_one(pool.as_ref())
            .await
            .unwrap();

        // If parameterization works correctly, the payload matches no rows
        assert_eq!(
            count, 0,
            "Injection payload must be treated as a literal value, not SQL"
        );

        // Sanity check: legitimate query does find the row
        let mut qb2 = QueryBuilder::new("SELECT COUNT(*) as count FROM anchors WHERE 1=1");
        qb2.push(" AND status = ").push_bind("active".to_string());

        let (legit_count,): (i32,) = qb2
            .build_query_as()
            .fetch_one(pool.as_ref())
            .await
            .unwrap();

        assert_eq!(legit_count, 1, "Legitimate query must still return correct results");
    }
}
