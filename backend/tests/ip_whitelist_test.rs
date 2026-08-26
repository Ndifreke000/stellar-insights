#[cfg(test)]
mod ip_whitelist_tests {
    use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
    use stellar_insights_backend::admin_ip_whitelist::IpWhitelistService;

    async fn setup_test_db() -> SqlitePool {
        let db_url = "sqlite::memory:";
        let pool = SqlitePoolOptions::new()
            .connect(db_url)
            .await
            .expect("Failed to create pool");

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS admin_ip_whitelist (
                id TEXT PRIMARY KEY,
                ip_or_cidr TEXT NOT NULL UNIQUE,
                description TEXT,
                added_by_user_id TEXT,
                added_at TIMESTAMP NOT NULL
            )
            ",
        )
        .execute(&pool)
        .await
        .expect("Failed to create whitelist table");

        pool
    }

    #[tokio::test]
    async fn test_add_single_ip() {
        let pool = setup_test_db().await;
        let service = IpWhitelistService::new(pool);

        let result = service
            .add_to_whitelist("192.168.1.1", Some("Office network".to_string()), None)
            .await;

        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.ip_or_cidr, "192.168.1.1");
        assert_eq!(entry.description, Some("Office network".to_string()));
    }

    #[tokio::test]
    async fn test_is_whitelisted_single_ip() {
        let pool = setup_test_db().await;
        let service = IpWhitelistService::new(pool);

        service
            .add_to_whitelist("192.168.1.100", None, None)
            .await
            .unwrap();

        let whitelisted = service
            .is_whitelisted("192.168.1.100")
            .await
            .unwrap();
        assert!(whitelisted);

        let not_whitelisted = service
            .is_whitelisted("192.168.1.99")
            .await
            .unwrap();
        assert!(!not_whitelisted);
    }

    #[tokio::test]
    async fn test_fail_closed_on_empty_list() {
        let pool = setup_test_db().await;
        let service = IpWhitelistService::new(pool);

        let denied = service
            .is_whitelisted("192.168.1.1")
            .await
            .unwrap();
        assert!(!denied);
    }

    #[tokio::test]
    async fn test_remove_from_whitelist() {
        let pool = setup_test_db().await;
        let service = IpWhitelistService::new(pool);

        service
            .add_to_whitelist("192.168.1.1", None, None)
            .await
            .unwrap();

        let whitelisted = service
            .is_whitelisted("192.168.1.1")
            .await
            .unwrap();
        assert!(whitelisted);

        service
            .remove_from_whitelist("192.168.1.1")
            .await
            .unwrap();

        let not_whitelisted = service
            .is_whitelisted("192.168.1.1")
            .await
            .unwrap();
        assert!(!not_whitelisted);
    }
}
