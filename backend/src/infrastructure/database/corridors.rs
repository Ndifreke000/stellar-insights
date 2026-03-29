// Corridor database operations - extracted from database.rs

use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::Utc;

use crate::features::corridors::models::CorridorRecord;
use crate::infrastructure::database::Database;
use crate::models::CreateCorridorRequest;
use crate::features::corridors::Corridor;

impl Database {
    pub async fn create_corridor(&self, req: CreateCorridorRequest) -> Result<Corridor> {
        self.execute_with_timing("create_corridor", async {
            let corridor = Corridor::new(
                req.source_asset_code,
                req.source_asset_issuer,
                req.dest_asset_code,
                req.dest_asset_issuer,
            );
            sqlx::query(
                r#"
                INSERT INTO corridors (
                    id, source_asset_code, source_asset_issuer,
                    destination_asset_code, destination_asset_issuer
                )
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT DO UPDATE SET updated_at = CURRENT_TIMESTAMP
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&corridor.source_asset_code)
            .bind(&corridor.source_asset_issuer)
            .bind(&corridor.destination_asset_code)
            .bind(&corridor.destination_asset_issuer)
            .execute(&self.pool)
            .await?;
            Ok(corridor)
        })
        .await
    }

    pub async fn list_corridors(&self, limit: i64, offset: i64) -> Result<Vec<Corridor>> {
        self.execute_with_timing("list_corridors", async {
            let records = sqlx::query_as::<_, CorridorRecord>(
                r#"SELECT * FROM corridors ORDER BY reliability_score DESC LIMIT $1 OFFSET $2"#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

            Ok(records.into_iter()
                .map(|r| Corridor::new(
                    r.source_asset_code,
                    r.source_asset_issuer,
                    r.destination_asset_code,
                    r.destination_asset_issuer,
                ))
                .collect())
        })
        .await
    }

    pub async fn get_corridor_by_id(&self, id: Uuid) -> Result<Option<Corridor>> {
        self.execute_with_timing("get_corridor_by_id", async {
            let record = sqlx::query_as::<_, CorridorRecord>(
                r#"SELECT * FROM corridors WHERE id = $1"#,
            )
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

            Ok(record.map(|r| Corridor::new(
                r.source_asset_code,
                r.source_asset_issuer,
                r.destination_asset_code,
                r.destination_asset_issuer,
            )))
        })
        .await
    }
}

