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

        let mut data_sql = String::from(
            "SELECT id, name, stellar_account, home_domain, total_transactions, \
             successful_transactions, failed_transactions, total_volume_usd, \
             avg_settlement_time_ms, reliability_score, status, created_at, updated_at \
             FROM anchors WHERE 1=1",
        );
        let mut count_sql =
            String::from("SELECT COUNT(*) as count FROM anchors WHERE 1=1");

        let mut status_val: Option<String> = None;
        let mut min_score_val: Option<f64> = None;
        let mut search_pattern: Option<String> = None;

        if let Some(f) = &filter {
            if let Some(status) = &f.status {
                data_sql.push_str(" AND status = ?");
                count_sql.push_str(" AND status = ?");
                status_val = Some(status.clone());
            }
            if let Some(min_score) = f.min_reliability_score {
                data_sql.push_str(" AND reliability_score >= ?");
                count_sql.push_str(" AND reliability_score >= ?");
                min_score_val = Some(min_score);
            }
            if let Some(search) = &f.search {
                data_sql.push_str(" AND (name LIKE ? OR stellar_account LIKE ?)");
                count_sql.push_str(" AND (name LIKE ? OR stellar_account LIKE ?)");
                search_pattern = Some(format!("%{search}%"));
            }
        }

        data_sql.push_str(" ORDER BY reliability_score DESC LIMIT ? OFFSET ?");

        let mut data_query = sqlx::query_as::<_, AnchorType>(&data_sql);
        let mut count_query = sqlx::query_as::<_, (i32,)>(&count_sql);

        if let Some(ref s) = status_val {
            data_query = data_query.bind(s.clone());
            count_query = count_query.bind(s.clone());
        }
        if let Some(score) = min_score_val {
            data_query = data_query.bind(score);
            count_query = count_query.bind(score);
        }
        if let Some(ref p) = search_pattern {
            data_query = data_query.bind(p.clone()).bind(p.clone());
            count_query = count_query.bind(p.clone()).bind(p.clone());
        }

        data_query = data_query.bind(limit).bind(offset);

        let anchors = data_query.fetch_all(pool.as_ref()).await?;
        let total: (i32,) = count_query.fetch_one(pool.as_ref()).await?;

        Ok(AnchorsConnection {
            nodes: anchors,
            total_count: total.0,
            has_next_page: (offset + limit) < total.0,
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

        let mut data_sql = String::from(
            "SELECT id, source_asset_code, source_asset_issuer, destination_asset_code, \
             destination_asset_issuer, reliability_score, status, created_at, updated_at \
             FROM corridors WHERE 1=1",
        );
        let mut count_sql =
            String::from("SELECT COUNT(*) as count FROM corridors WHERE 1=1");

        let mut source_val: Option<String> = None;
        let mut dest_val: Option<String> = None;
        let mut status_val: Option<String> = None;
        let mut min_score_val: Option<f64> = None;

        if let Some(f) = &filter {
            if let Some(source) = &f.source_asset_code {
                data_sql.push_str(" AND source_asset_code = ?");
                count_sql.push_str(" AND source_asset_code = ?");
                source_val = Some(source.clone());
            }
            if let Some(dest) = &f.destination_asset_code {
                data_sql.push_str(" AND destination_asset_code = ?");
                count_sql.push_str(" AND destination_asset_code = ?");
                dest_val = Some(dest.clone());
            }
            if let Some(status) = &f.status {
                data_sql.push_str(" AND status = ?");
                count_sql.push_str(" AND status = ?");
                status_val = Some(status.clone());
            }
            if let Some(min_score) = f.min_reliability_score {
                data_sql.push_str(" AND reliability_score >= ?");
                count_sql.push_str(" AND reliability_score >= ?");
                min_score_val = Some(min_score);
            }
        }

        data_sql.push_str(" ORDER BY reliability_score DESC LIMIT ? OFFSET ?");

        let mut data_query = sqlx::query_as::<_, CorridorType>(&data_sql);
        let mut count_query = sqlx::query_as::<_, (i32,)>(&count_sql);

        if let Some(ref s) = source_val {
            data_query = data_query.bind(s.clone());
            count_query = count_query.bind(s.clone());
        }
        if let Some(ref d) = dest_val {
            data_query = data_query.bind(d.clone());
            count_query = count_query.bind(d.clone());
        }
        if let Some(ref s) = status_val {
            data_query = data_query.bind(s.clone());
            count_query = count_query.bind(s.clone());
        }
        if let Some(score) = min_score_val {
            data_query = data_query.bind(score);
            count_query = count_query.bind(score);
        }

        data_query = data_query.bind(limit).bind(offset);

        let corridors = data_query.fetch_all(pool.as_ref()).await?;
        let total: (i32,) = count_query.fetch_one(pool.as_ref()).await?;

        Ok(CorridorsConnection {
            nodes: corridors,
            total_count: total.0,
            has_next_page: (offset + limit) < total.0,
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

        let mut sql = String::from(
            "SELECT id, name, value, entity_id, entity_type, timestamp, created_at \
             FROM metrics WHERE 1=1",
        );

        let mut entity_id_val: Option<String> = None;
        let mut entity_type_val: Option<String> = None;
        let mut time_start = None;
        let mut time_end = None;

        if let Some(eid) = entity_id {
            sql.push_str(" AND entity_id = ?");
            entity_id_val = Some(eid);
        }
        if let Some(etype) = entity_type {
            sql.push_str(" AND entity_type = ?");
            entity_type_val = Some(etype);
        }
        if let Some(tr) = time_range {
            sql.push_str(" AND timestamp >= ? AND timestamp <= ?");
            time_start = Some(tr.start);
            time_end = Some(tr.end);
        }

        sql.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let mut query = sqlx::query_as::<_, MetricType>(&sql);

        if let Some(eid) = entity_id_val {
            query = query.bind(eid);
        }
        if let Some(etype) = entity_type_val {
            query = query.bind(etype);
        }
        if let Some(start) = time_start {
            query = query.bind(start);
        }
        if let Some(end) = time_end {
            query = query.bind(end);
        }

        query = query.bind(limit).bind(offset);

        let metrics = query.fetch_all(pool.as_ref()).await?;
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
        let pattern = format!("%{query}%");

        let anchors = sqlx::query_as::<_, AnchorType>(
            "SELECT id, name, stellar_account, home_domain, total_transactions, \
             successful_transactions, failed_transactions, total_volume_usd, \
             avg_settlement_time_ms, reliability_score, status, created_at, updated_at \
             FROM anchors WHERE name LIKE ? OR stellar_account LIKE ? LIMIT ?",
        )
        .bind(pattern.clone())
        .bind(pattern.clone())
        .bind(search_limit)
        .fetch_all(pool.as_ref())
        .await?;

        let corridors = sqlx::query_as::<_, CorridorType>(
            "SELECT id, source_asset_code, source_asset_issuer, destination_asset_code, \
             destination_asset_issuer, reliability_score, status, created_at, updated_at \
             FROM corridors WHERE source_asset_code LIKE ? OR destination_asset_code LIKE ? LIMIT ?",
        )
        .bind(pattern.clone())
        .bind(pattern)
        .bind(search_limit)
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
