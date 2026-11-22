//! Corridor search: full-text and fuzzy search over anchors, assets and corridors.
//!
//! Historically this module wrapped an Elasticsearch client that was never wired
//! up. Search is now backed by SQLite FTS5 (with a trigram index for typo
//! tolerance) and ranked by relevance combined with popularity. The public
//! surface is a thin, HTTP-free service so it can be unit tested directly.

use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

/// The kind of entity a search hit refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchKind {
    Anchor,
    Asset,
    Corridor,
}

impl SearchKind {
    fn as_str(self) -> &'static str {
        match self {
            SearchKind::Anchor => "anchor",
            SearchKind::Asset => "asset",
            SearchKind::Corridor => "corridor",
        }
    }
}

/// A single ranked search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub kind: SearchKind,
    pub id: String,
    pub name: String,
    /// Relevance score (higher is better), already blended with popularity.
    pub score: f64,
    /// Raw popularity signal (e.g. corridor volume / anchor usage).
    pub popularity: f64,
}

/// Errors surfaced by the search service. The API layer maps these onto the
/// project's standard error shape.
#[derive(Debug)]
pub enum SearchError {
    /// The query was empty or only whitespace.
    EmptyQuery,
    /// The query exceeded the maximum supported length.
    QueryTooLong,
    /// A storage-level failure.
    Storage(String),
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::EmptyQuery => write!(f, "search query must not be empty"),
            SearchError::QueryTooLong => write!(f, "search query is too long"),
            SearchError::Storage(msg) => write!(f, "search storage error: {msg}"),
        }
    }
}

impl std::error::Error for SearchError {}

impl From<sqlx::Error> for SearchError {
    fn from(err: sqlx::Error) -> Self {
        SearchError::Storage(err.to_string())
    }
}

/// Maximum accepted query length (characters).
pub const MAX_QUERY_LEN: usize = 128;
/// Default number of results returned.
pub const DEFAULT_LIMIT: i64 = 20;
/// Hard cap on the number of results returned.
pub const MAX_LIMIT: i64 = 100;

/// Weight applied to the popularity signal when blending with relevance.
const POPULARITY_WEIGHT: f64 = 0.25;

/// Full-text + fuzzy search service over anchors, assets and corridors.
#[derive(Clone)]
pub struct SearchService {
    pool: SqlitePool,
}

impl SearchService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create the FTS5 tables and triggers backing search. Safe to call on
    /// every startup; all statements are idempotent.
    pub async fn ensure_schema(&self) -> Result<(), SearchError> {
        let statements = [
            // Main full-text index (unicode61 tokenizer, prefix queries enabled).
            "CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(\
                kind UNINDEXED, entity_id UNINDEXED, name, body, \
                tokenize = 'unicode61 remove_diacritics 2', prefix = '2 3 4'\
            )",
            // Trigram index used as a typo-tolerant fallback.
            "CREATE VIRTUAL TABLE IF NOT EXISTS search_trigram USING fts5(\
                kind UNINDEXED, entity_id UNINDEXED, name, \
                tokenize = 'trigram'\
            )",
            // Popularity signal kept alongside the index.
            "CREATE TABLE IF NOT EXISTS search_popularity (\
                kind TEXT NOT NULL, entity_id TEXT NOT NULL, popularity REAL NOT NULL DEFAULT 0, \
                PRIMARY KEY (kind, entity_id)\
            )",
        ];
        for stmt in statements {
            sqlx::query(stmt).execute(&self.pool).await?;
        }
        Ok(())
    }

    /// Index (or re-index) a single entity.
    pub async fn upsert(
        &self,
        kind: SearchKind,
        id: &str,
        name: &str,
        body: &str,
        popularity: f64,
    ) -> Result<(), SearchError> {
        let kind = kind.as_str();
        let mut tx = self.pool.begin().await?;
        for table in ["search_index", "search_trigram"] {
            sqlx::query(&format!(
                "DELETE FROM {table} WHERE kind = ?1 AND entity_id = ?2"
            ))
            .bind(kind)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO search_index (kind, entity_id, name, body) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(kind)
        .bind(id)
        .bind(name)
        .bind(body)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO search_trigram (kind, entity_id, name) VALUES (?1, ?2, ?3)")
            .bind(kind)
            .bind(id)
            .bind(name)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO search_popularity (kind, entity_id, popularity) VALUES (?1, ?2, ?3) \
             ON CONFLICT(kind, entity_id) DO UPDATE SET popularity = excluded.popularity",
        )
        .bind(kind)
        .bind(id)
        .bind(popularity)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Search anchors, assets and corridors.
    ///
    /// Runs an FTS5 prefix query first; if that yields nothing, falls back to a
    /// trigram query for typo tolerance. Results are ranked by relevance blended
    /// with popularity.
    pub async fn search(
        &self,
        query: &str,
        limit: Option<i64>,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(SearchError::EmptyQuery);
        }
        if trimmed.chars().count() > MAX_QUERY_LEN {
            return Err(SearchError::QueryTooLong);
        }
        let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

        let mut hits = self.fts_search(trimmed, limit).await?;
        if hits.is_empty() {
            hits = self.trigram_search(trimmed, limit).await?;
        }
        Ok(hits);
    }

    async fn fts_search(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<SearchHit>, SearchError> {
        // Build a prefix match expression: each token becomes `token*`.
        let match_expr = query
            .split_whitespace()
            .map(|tok| format!("\"{}\"*", tok.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" ");

        let rows = sqlx::query(
            "SELECT s.kind, s.entity_id, s.name, bm25(search_index, 0.0, 0.0, 4.0, 1.0) AS rank \
             FROM search_index s \
             WHERE search_index MATCH ?1 \
             ORDER BY rank \
             LIMIT ?2",
        )
        .bind(&match_expr)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        self.collect(rows).await
    }

    async fn trigram_search(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<SearchHit>, SearchError> {
        // Trigram tokenizer tolerates typos and partial matches.
        let match_expr = format!("\"{}\"", query.replace('"', ""));
        let rows = sqlx::query(
            "SELECT t.kind, t.entity_id, t.name, bm25(search_trigram, 0.0, 0.0, 1.0) AS rank \
             FROM search_trigram t \
             WHERE search_trigram MATCH ?1 \
             ORDER BY rank \
             LIMIT ?2",
        )
        .bind(&match_expr)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        self.collect(rows).await
    }

    /// Blend bm25 relevance with popularity and sort descending.
    async fn collect(&self, rows: Vec<sqlx::sqlite::SqliteRow>) -> Result<Vec<SearchHit>, SearchError> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut popularity: HashMap<(String, String), f64> = HashMap::new();
        let pop_rows = sqlx::query("SELECT kind, entity_id, popularity FROM search_popularity")
            .fetch_all(&self.pool)
            .await?;
        for row in pop_rows {
            let kind: String = row.try_get("kind")?;
            let id: String = row.try_get("entity_id")?;
            let pop: f64 = row.try_get("popularity")?;
            popularity.insert((kind, id), pop);
        }

        let mut hits = Vec::with_capacity(rows.len());
        for row in rows {
            let kind_str: String = row.try_get("kind")?;
            let id: String = row.try_get("entity_id")?;
            let name: String = row.try_get("name")?;
            // bm25 returns negative values where more negative is better.
            let rank: f64 = row.try_get("rank")?;
            let relevance = -rank;
            let pop = popularity
                .get(&(kind_str.clone(), id.clone()))
                .copied()
                .unwrap_or(0.0);
            let kind = match kind_str.as_str() {
                "anchor" => SearchKind::Anchor,
                "asset" => SearchKind::Asset,
                _ => SearchKind::Corridor,
            };
            hits.push(SearchHit {
                kind,
                id,
                name,
                score: relevance + POPULARITY_WEIGHT * pop,
                popularity: pop,
            });
        }

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn service() -> SearchService {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let svc = SearchService::new(pool);
        svc.ensure_schema().await.unwrap();
        svc
    }

    #[tokio::test]
    async fn rejects_empty_query() {
        let svc = service().await;
        assert!(matches!(svc.search("   ", None).await, Err(SearchError::EmptyQuery)));
    }

    #[tokio::test]
    async fn rejects_overlong_query() {
        let svc = service().await;
        let long = "a".repeat(MAX_QUERY_LEN + 1);
        assert!(matches!(svc.search(&long, None).await, Err(SearchError::QueryTooLong)));
    }

    #[tokio::test]
    async fn prefix_match_finds_anchor() {
        let svc = service().await;
        svc.upsert(SearchKind::Anchor, "a1", "MoneyGram", "moneygram.com", 10.0)
            .await
            .unwrap();
        let hits = svc.search("money", None).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "a1");
    }

    #[tokio::test]
    async fn typo_tolerance_falls_back_to_trigram() {
        let svc = service().await;
        svc.upsert(SearchKind::Asset, "USDC", "USDC", "usd coin", 5.0)
            .await
            .unwrap();
        // Transposed typo should still match via trigram fallback.
        let hits = svc.search("UDSC", None).await.unwrap();
        assert!(hits.iter().any(|h| h.id == "USDC"));
    }

    #[tokio::test]
    async fn popularity_breaks_relevance_ties() {
        let svc = service().await;
        svc.upsert(SearchKind::Corridor, "c1", "USDC-NGN", "usdc ngn", 1.0)
            .await
            .unwrap();
        svc.upsert(SearchKind::Corridor, "c2", "USDC-NGN", "usdc ngn", 100.0)
            .await
            .unwrap();
        let hits = svc.search("USDC", None).await.unwrap();
        assert_eq!(hits[0].id, "c2");
    }
}
