use anyhow::{Context, Result};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::Asset;

pub struct AssetDb {
    pool: SqlitePool,
}

impl AssetDb {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates a new asset or updates existing asset's anchor association (upsert).
    #[tracing::instrument(skip(self), fields(anchor_id = %anchor_id, asset_code = %asset_code))]
    pub async fn create_asset(
        &self,
        anchor_id: Uuid,
        asset_code: String,
        asset_issuer: String,
    ) -> Result<Asset> {
        let id = Uuid::new_v4().to_string();
        let asset = sqlx::query_as::<_, Asset>(
            r"
            INSERT INTO assets (id, anchor_id, asset_code, asset_issuer)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (asset_code, asset_issuer) DO UPDATE
            SET anchor_id = EXCLUDED.anchor_id,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            ",
        )
        .bind(id)
        .bind(anchor_id.to_string())
        .bind(&asset_code)
        .bind(&asset_issuer)
        .fetch_one(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to create asset: code={}, issuer={}, anchor_id={}",
                asset_code, asset_issuer, anchor_id
            )
        })?;
        Ok(asset)
    }

    /// Retrieves all assets issued by a specific anchor.
    #[tracing::instrument(skip(self), fields(anchor_id = %anchor_id))]
    pub async fn get_assets_by_anchor(&self, anchor_id: Uuid) -> Result<Vec<Asset>> {
        let assets = sqlx::query_as::<_, Asset>(
            r"
            SELECT * FROM assets WHERE anchor_id = $1
            ORDER BY asset_code ASC
            ",
        )
        .bind(anchor_id.to_string())
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Failed to get assets for anchor_id: {}", anchor_id))?;
        Ok(assets)
    }

    /// Retrieves assets for multiple anchors in a single query.
    ///
    /// Returns a `HashMap` mapping `anchor_id` to their list of assets.
    #[tracing::instrument(skip(self), fields(anchor_count = anchor_ids.len()))]
    pub async fn get_assets_by_anchors(
        &self,
        anchor_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<String, Vec<Asset>>> {
        if anchor_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let anchor_id_strs: Vec<String> = anchor_ids
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        let mut placeholders = String::with_capacity(anchor_id_strs.len() * 4);
        for i in 0..anchor_id_strs.len() {
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
        for id in &anchor_id_strs {
            query = query.bind(id);
        }

        let assets = query
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!("Failed to get assets for {} anchor ids", anchor_ids.len()))?;

        let mut result: std::collections::HashMap<String, Vec<Asset>> =
            std::collections::HashMap::new();
        for asset in assets {
            result
                .entry(asset.anchor_id.clone())
                .or_default()
                .push(asset);
        }

        Ok(result)
    }

    /// Counts the number of assets associated with a given anchor.
    #[tracing::instrument(skip(self), fields(anchor_id = %anchor_id))]
    pub async fn count_assets_by_anchor(&self, anchor_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r"
            SELECT COUNT(*) FROM assets WHERE anchor_id = $1
            ",
        )
        .bind(anchor_id.to_string())
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to count assets for anchor_id: {}", anchor_id))?;
        Ok(count.0)
    }
}
