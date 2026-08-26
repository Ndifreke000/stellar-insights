#[cfg(test)]
mod session_tests {
    use chrono::Utc;
    use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
    use stellar_insights_backend::session::SessionService;
    use std::time::Duration;

    async fn setup_test_db() -> SqlitePool {
        let db_url = "sqlite::memory:";
        let pool = SqlitePoolOptions::new()
            .connect(db_url)
            .await
            .expect("Failed to create pool");

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                refresh_token_jti TEXT NOT NULL UNIQUE,
                device_user_agent TEXT,
                ip_address TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL,
                last_activity_at TIMESTAMP NOT NULL,
                expires_at TIMESTAMP NOT NULL,
                idle_timeout_seconds INTEGER NOT NULL,
                max_lifetime_seconds INTEGER NOT NULL,
                revoked_at TIMESTAMP
            )
            ",
        )
        .execute(&pool)
        .await
        .expect("Failed to create sessions table");

        pool
    }

    #[tokio::test]
    async fn test_session_creation() {
        let pool = setup_test_db().await;
        let service = SessionService::new(pool);

        let session = service
            .create_session(
                "user123",
                "jti-abc123",
                Some("Mozilla/5.0".to_string()),
                "192.168.1.1",
                Some(3600),
                Some(604800),
            )
            .await;

        assert!(session.is_ok());
        let session = session.unwrap();
        assert_eq!(session.user_id, "user123");
        assert_eq!(session.ip_address, "192.168.1.1");
        assert_eq!(session.device_user_agent, Some("Mozilla/5.0".to_string()));
        assert!(session.revoked_at.is_none());
    }

    #[tokio::test]
    async fn test_get_active_session() {
        let pool = setup_test_db().await;
        let service = SessionService::new(pool);

        let created_session = service
            .create_session(
                "user123",
                "jti-abc123",
                Some("Mozilla/5.0".to_string()),
                "192.168.1.1",
                Some(3600),
                Some(604800),
            )
            .await
            .unwrap();

        let retrieved = service.get_active_session(&created_session.id).await;
        assert!(retrieved.is_ok());
        let retrieved = retrieved.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().user_id, "user123");
    }

    #[tokio::test]
    async fn test_session_revocation() {
        let pool = setup_test_db().await;
        let service = SessionService::new(pool);

        let session = service
            .create_session(
                "user123",
                "jti-abc123",
                None,
                "192.168.1.1",
                Some(3600),
                Some(604800),
            )
            .await
            .unwrap();

        // Revoke the session
        let _ = service.revoke_session(&session.id).await;

        // Try to get it again - should return None
        let retrieved = service.get_active_session(&session.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_session_expiry() {
        let pool = setup_test_db().await;
        let service = SessionService::new(pool);

        // Create session with very short max_lifetime (1 second)
        let session = service
            .create_session(
                "user123",
                "jti-abc123",
                None,
                "192.168.1.1",
                Some(3600),
                Some(1), // 1 second
            )
            .await
            .unwrap();

        // Session should be active immediately
        let active = service.get_active_session(&session.id).await.unwrap();
        assert!(active.is_some());

        // Wait for expiry
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Session should now be expired
        let expired = service.get_active_session(&session.id).await.unwrap();
        assert!(expired.is_none());
    }

    #[tokio::test]
    async fn test_list_active_sessions() {
        let pool = setup_test_db().await;
        let service = SessionService::new(pool);

        // Create multiple sessions for same user
        let _session1 = service
            .create_session(
                "user123",
                "jti-1",
                Some("Chrome".to_string()),
                "192.168.1.1",
                Some(3600),
                Some(604800),
            )
            .await
            .unwrap();

        let _session2 = service
            .create_session(
                "user123",
                "jti-2",
                Some("Firefox".to_string()),
                "192.168.1.2",
                Some(3600),
                Some(604800),
            )
            .await
            .unwrap();

        let sessions = service.list_active_sessions("user123").await.unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions
            .iter()
            .any(|s| s.device_user_agent == Some("Chrome".to_string())));
        assert!(sessions
            .iter()
            .any(|s| s.device_user_agent == Some("Firefox".to_string())));
    }

    #[tokio::test]
    async fn test_revoke_all_other_sessions() {
        let pool = setup_test_db().await;
        let service = SessionService::new(pool);

        let session1 = service
            .create_session(
                "user123",
                "jti-1",
                None,
                "192.168.1.1",
                Some(3600),
                Some(604800),
            )
            .await
            .unwrap();

        let session2 = service
            .create_session(
                "user123",
                "jti-2",
                None,
                "192.168.1.2",
                Some(3600),
                Some(604800),
            )
            .await
            .unwrap();

        // Revoke all except session1
        let _ = service
            .revoke_all_other_sessions("user123", &session1.id)
            .await;

        // session1 should still be active
        let active1 = service.get_active_session(&session1.id).await.unwrap();
        assert!(active1.is_some());

        // session2 should be revoked
        let active2 = service.get_active_session(&session2.id).await.unwrap();
        assert!(active2.is_none());
    }

    #[tokio::test]
    async fn test_touch_session() {
        let pool = setup_test_db().await;
        let service = SessionService::new(pool);

        let session = service
            .create_session(
                "user123",
                "jti-abc123",
                None,
                "192.168.1.1",
                Some(3600),
                Some(604800),
            )
            .await
            .unwrap();

        let original_activity = session.last_activity_at;

        // Wait and touch session
        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = service.touch_session(&session.id).await;

        // Get session again and check last_activity_at updated
        let updated = service.get_active_session(&session.id).await.unwrap();
        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert!(updated.last_activity_at > original_activity);
    }
}
