#[cfg(test)]
mod webhook_integration_tests {
    use super::*;
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use uuid::Uuid;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        
        // Create webhook tables
        sqlx::query(
            r#"
            CREATE TABLE webhooks (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                url TEXT NOT NULL,
                event_types TEXT NOT NULL,
                filters TEXT,
                secret TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_fired_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE webhook_events (
                id TEXT PRIMARY KEY,
                webhook_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                status TEXT NOT NULL,
                retries INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_webhook_registration_and_triggering() {
        let pool = setup_test_db().await;
        let webhook_service = Arc::new(WebhookEventService::new(pool.clone()));
        
        // Register a test webhook
        let webhook_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4().to_string();
        
        sqlx::query(
            r#"
            INSERT INTO webhooks (id, user_id, url, event_types, secret, is_active, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&webhook_id)
        .bind(&user_id)
        .bind("https://example.com/webhook")
        .bind("anchor.status_changed")
        .bind("test_secret")
        .bind(true)
        .bind("2023-01-01T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        // Trigger anchor status change event
        let result = webhook_service
            .trigger_anchor_status_changed(
                &webhook_id,
                "test_anchor",
                "healthy",
                "degraded",
                85.0,
                5,
            )
            .await;

        assert!(result.is_ok());

        // Verify webhook event was created
        let events: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id, webhook_id, event_type FROM webhook_events WHERE webhook_id = ?"
        )
        .bind(&webhook_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].1, webhook_id);
        assert_eq!(events[0].2, "anchor.status_changed");
    }

    #[tokio::test]
    async fn test_corridor_health_webhook() {
        let pool = setup_test_db().await;
        let webhook_service = Arc::new(WebhookEventService::new(pool.clone()));
        
        // Register a test webhook for corridor events
        let webhook_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4().to_string();
        
        sqlx::query(
            r#"
            INSERT INTO webhooks (id, user_id, url, event_types, secret, is_active, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&webhook_id)
        .bind(&user_id)
        .bind("https://example.com/webhook")
        .bind("corridor.health_degraded")
        .bind("test_secret")
        .bind(true)
        .bind("2023-01-01T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        // Create test corridor metrics
        let old_metrics = CorridorMetrics {
            success_rate: 0.95,
            avg_latency_ms: 100.0,
            p95_latency_ms: 150.0,
            p99_latency_ms: 200.0,
            liquidity_depth_usd: 1000000.0,
            liquidity_volume_24h_usd: 500000.0,
            total_attempts: 1000,
            successful_payments: 950,
            failed_payments: 50,
        };

        let new_metrics = CorridorMetrics {
            success_rate: 0.84, // 11% drop
            avg_latency_ms: 180.0,
            p95_latency_ms: 250.0,
            p99_latency_ms: 350.0,
            liquidity_depth_usd: 800000.0,
            liquidity_volume_24h_usd: 400000.0,
            total_attempts: 1000,
            successful_payments: 840,
            failed_payments: 160,
        };

        // Trigger corridor health degradation event
        let result = webhook_service
            .trigger_corridor_health_degraded(
                "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN->XLM:native",
                &old_metrics,
                &new_metrics,
                "warning",
                vec!["success_rate_dropped: 95.0% -> 84.0%".to_string()],
            )
            .await;

        assert!(result.is_ok());

        // Verify webhook event was created
        let events: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id, webhook_id, event_type FROM webhook_events WHERE webhook_id = ?"
        )
        .bind(&webhook_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].2, "corridor.health_degraded");
    }

    #[tokio::test]
    async fn test_payment_created_webhook() {
        let pool = setup_test_db().await;
        let webhook_service = Arc::new(WebhookEventService::new(pool.clone()));
        
        // Register a test webhook for payment events
        let webhook_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4().to_string();
        
        sqlx::query(
            r#"
            INSERT INTO webhooks (id, user_id, url, event_types, secret, is_active, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&webhook_id)
        .bind(&user_id)
        .bind("https://example.com/webhook")
        .bind("payment.created")
        .bind("test_secret")
        .bind(true)
        .bind("2023-01-01T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        // Trigger payment created event
        let result = webhook_service
            .trigger_payment_created(
                "payment_123",
                "GABC...",
                "GXYZ...",
                "USDC",
                "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
                100.50,
                "2023-01-01T12:00:00Z",
            )
            .await;

        assert!(result.is_ok());

        // Verify webhook event was created
        let events: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id, webhook_id, event_type FROM webhook_events WHERE webhook_id = ?"
        )
        .bind(&webhook_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].2, "payment.created");
    }

    #[tokio::test]
    async fn test_webhook_filters() {
        let pool = setup_test_db().await;
        let webhook_service = Arc::new(WebhookEventService::new(pool.clone()));
        
        // Register a test webhook with filters
        let webhook_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4().to_string();
        
        sqlx::query(
            r#"
            INSERT INTO webhooks (id, user_id, url, event_types, filters, secret, is_active, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&webhook_id)
        .bind(&user_id)
        .bind("https://example.com/webhook")
        .bind("corridor.health_degraded")
        .bind(r#"{"severity": "critical"}"#)
        .bind("test_secret")
        .bind(true)
        .bind("2023-01-01T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        // Create test corridor metrics for warning (should be filtered out)
        let old_metrics = CorridorMetrics {
            success_rate: 0.95,
            avg_latency_ms: 100.0,
            p95_latency_ms: 150.0,
            p99_latency_ms: 200.0,
            liquidity_depth_usd: 1000000.0,
            liquidity_volume_24h_usd: 500000.0,
            total_attempts: 1000,
            successful_payments: 950,
            failed_payments: 50,
        };

        let new_metrics = CorridorMetrics {
            success_rate: 0.84, // 11% drop - warning severity
            avg_latency_ms: 180.0,
            p95_latency_ms: 250.0,
            p99_latency_ms: 350.0,
            liquidity_depth_usd: 800000.0,
            liquidity_volume_24h_usd: 400000.0,
            total_attempts: 1000,
            successful_payments: 840,
            failed_payments: 160,
        };

        // Trigger warning event (should be filtered out)
        let result = webhook_service
            .trigger_corridor_health_degraded(
                "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN->XLM:native",
                &old_metrics,
                &new_metrics,
                "warning",
                vec!["success_rate_dropped: 95.0% -> 84.0%".to_string()],
            )
            .await;

        assert!(result.is_ok());

        // Verify no webhook event was created due to filter
        let events: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id, webhook_id, event_type FROM webhook_events WHERE webhook_id = ?"
        )
        .bind(&webhook_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(events.len(), 0);
    }
}
