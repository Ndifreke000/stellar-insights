use anyhow::{Context, Result};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::Asset;

pub struct AssetDb {
    pool: SqlitePool,
}

impl AssetDb {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        anchor_id: Uuid,
        asset_code: String,
        asset_issuer: String,
    ) -> Result<Asset> {
        sqlx::query_as::<_, Asset>(
            "INSERT INTO assets (id, anchor_id, asset_code, asset_issuer)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (asset_code, asset_issuer) DO UPDATE
             SET anchor_id = EXCLUDED.anchor_id, updated_at = CURRENT_TIMESTAMP
             RETURNING *",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(anchor_id.to_string())
        .bind(&asset_code)
        .bind(&asset_issuer)
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to create asset: {asset_code}/{asset_issuer}"))
    }

    pub async fn get_by_anchor(&self, anchor_id: Uuid) -> Result<Vec<Asset>> {
        sqlx::query_as::<_, Asset>(
            "SELECT * FROM assets WHERE anchor_id = $1 ORDER BY asset_code ASC",
        )
        .bind(anchor_id.to_string())
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to get assets for anchor: {anchor_id}"))
    }

    pub async fn get_by_anchors(
        &self,
        anchor_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<String, Vec<Asset>>> {
        if anchor_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let id_strs: Vec<String> = anchor_ids.iter().map(ToString::to_string).collect();

        let mut placeholders = String::with_capacity(id_strs.len() * 4);
        for i in 0..id_strs.len() {
            if i > 0 {
                placeholders.push_str(", ");
            }
            use std::fmt::Write as _;
            let _ = write!(placeholders, "?{}", i + 1);
        }

        let query_str = format!(
            "SELECT * FROM assets WHERE anchor_id IN ({placeholders}) ORDER BY anchor_id, asset_code ASC"
        );

        let mut query = sqlx::query_as::<_, Asset>(&query_str);
        for id in &id_strs {
            query = query.bind(id);
        }

        let assets = query
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!("Failed to get assets for {} anchors", anchor_ids.len()))?;

        let mut result: std::collections::HashMap<String, Vec<Asset>> =
            std::collections::HashMap::new();
        for asset in assets {
            result.entry(asset.anchor_id.clone()).or_default().push(asset);
        }
        Ok(result)
    }

    pub async fn count_by_anchor(&self, anchor_id: Uuid) -> Result<i64> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM assets WHERE anchor_id = $1")
                .bind(anchor_id.to_string())
                .fetch_one(&self.pool)
                .await
                .with_context(|| format!("Failed to count assets for anchor: {anchor_id}"))?;
        Ok(count.0)
    }
}
