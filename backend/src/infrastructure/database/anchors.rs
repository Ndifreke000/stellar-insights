// Anchor database operations - extracted from database.rs

use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::Utc;

use crate::features::anchors::models::{Anchor, CreateAnchorRequest};
use crate::infrastructure::database::Database;

impl Database {
    pub async fn create_anchor(&self, req: CreateAnchorRequest) -> Result<Anchor> {
        self.execute_with_timing("create_anchor", async {
            let id = Uuid::new_v4().to_string();
            sqlx::query_as::<_, Anchor>(
                r#"
                INSERT INTO anchors (id, name, stellar_account, home_domain)
                VALUES ($1, $2, $3, $4)
                RETURNING *
                "#,
            )
            .bind(id)
            .bind(&req.name)
            .bind(&req.stellar_account)
            .bind(&req.home_domain)
            .fetch_one(&self.pool)
            .await
        })
        .await
    }

    pub async fn get_anchor_by_id(&self, id: Uuid) -> Result<Option<Anchor>> {
        self.execute_with_timing("get_anchor_by_id", async {
            sqlx::query_as::<_, Anchor>("SELECT * FROM anchors WHERE id = $1")
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await
        })
        .await
    }

    pub async fn get_anchor_by_stellar_account(&self, stellar_account: &str) -> Result<Option<Anchor>> {
        self.execute_with_timing("get_anchor_by_stellar_account", async {
            sqlx::query_as::<_, Anchor>("SELECT * FROM anchors WHERE stellar_account = $1")
                .bind(stellar_account)
                .fetch_optional(&self.pool)
                .await
        })
        .await
    }

    pub async fn list_anchors(&self, limit: i64, offset: i64) -> Result<Vec<Anchor>> {
        self.execute_with_timing("list_anchors", async {
            sqlx::query_as::<_, Anchor>(
                r#"
                SELECT * FROM anchors
                ORDER BY reliability_score DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        })
        .await
    }
}

