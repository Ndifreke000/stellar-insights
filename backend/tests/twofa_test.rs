#[cfg(test)]
mod twofa_tests {
    use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
    use stellar_insights_backend::twofa::TwoFAService;
    use stellar_insights_backend::crypto::CryptoService;

    async fn setup_test_db() -> SqlitePool {
        let db_url = "sqlite::memory:";
        let pool = SqlitePoolOptions::new()
            .connect(db_url)
            .await
            .expect("Failed to create pool");

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_2fa_secrets (
                user_id TEXT PRIMARY KEY,
                encrypted_secret TEXT NOT NULL,
                is_enabled BOOLEAN NOT NULL,
                enrolled_at TIMESTAMP,
                backup_codes_generated_at TIMESTAMP
            )
            ",
        )
        .execute(&pool)
        .await
        .expect("Failed to create user_2fa_secrets table");

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_2fa_backup_codes (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                hashed_code TEXT NOT NULL UNIQUE,
                used_at TIMESTAMP,
                created_at TIMESTAMP NOT NULL
            )
            ",
        )
        .execute(&pool)
        .await
        .expect("Failed to create user_2fa_backup_codes table");

        pool
    }

    #[tokio::test]
    async fn test_generate_totp_secret() {
        let _pool = setup_test_db().await;
        let crypto = CryptoService::new_for_tests();
        let _twofa = TwoFAService::new(_pool, crypto);

        let (otpauth_uri, secret) = _twofa
            .generate_totp_secret("user123", "user@example.com")
            .expect("Failed to generate secret");

        assert!(otpauth_uri.contains("otpauth://totp"));
        assert!(otpauth_uri.contains("user@example.com"));
        assert!(otpauth_uri.contains("stellar-insights"));
        assert!(!secret.is_empty());
        assert_eq!(secret.len(), 32); // base32 encoded 20 bytes
    }

    #[tokio::test]
    async fn test_2fa_enrollment() {
        let pool = setup_test_db().await;
        let crypto = CryptoService::new_for_tests();
        let twofa = TwoFAService::new(pool, crypto);

        let (_uri, secret) = twofa
            .generate_totp_secret("user123", "user@example.com")
            .expect("Failed to generate secret");

        let result = twofa.enroll_2fa("user123", &secret).await;
        assert!(result.is_ok());

        // Check 2FA is not enabled yet
        let enabled = twofa.is_2fa_enabled("user123").await.unwrap();
        assert!(!enabled);
    }

    #[tokio::test]
    async fn test_activate_2fa() {
        let pool = setup_test_db().await;
        let crypto = CryptoService::new_for_tests();
        let twofa = TwoFAService::new(pool, crypto);

        let (_uri, secret) = twofa
            .generate_totp_secret("user123", "user@example.com")
            .expect("Failed to generate secret");

        twofa.enroll_2fa("user123", &secret).await.unwrap();
        twofa.activate_2fa("user123").await.unwrap();

        let enabled = twofa.is_2fa_enabled("user123").await.unwrap();
        assert!(enabled);
    }

    #[tokio::test]
    async fn test_backup_codes_generation() {
        let pool = setup_test_db().await;
        let crypto = CryptoService::new_for_tests();
        let twofa = TwoFAService::new(pool, crypto);

        let (_uri, secret) = twofa
            .generate_totp_secret("user123", "user@example.com")
            .expect("Failed to generate secret");

        twofa.enroll_2fa("user123", &secret).await.unwrap();
        let codes = twofa.generate_backup_codes("user123").await.unwrap();

        assert_eq!(codes.len(), 10);
        for code in &codes {
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|c| c.is_numeric()));
        }
    }

    #[tokio::test]
    async fn test_backup_code_one_time_use() {
        let pool = setup_test_db().await;
        let crypto = CryptoService::new_for_tests();
        let twofa = TwoFAService::new(pool, crypto);

        let (_uri, secret) = twofa
            .generate_totp_secret("user123", "user@example.com")
            .expect("Failed to generate secret");

        twofa.enroll_2fa("user123", &secret).await.unwrap();
        let codes = twofa.generate_backup_codes("user123").await.unwrap();

        let test_code = &codes[0];

        // First use should succeed
        let first_use = twofa.verify_backup_code("user123", test_code).await.unwrap();
        assert!(first_use);

        // Second use should fail (already consumed)
        let second_use = twofa.verify_backup_code("user123", test_code).await.unwrap();
        assert!(!second_use);
    }

    #[tokio::test]
    async fn test_unused_backup_codes_count() {
        let pool = setup_test_db().await;
        let crypto = CryptoService::new_for_tests();
        let twofa = TwoFAService::new(pool, crypto);

        let (_uri, secret) = twofa
            .generate_totp_secret("user123", "user@example.com")
            .expect("Failed to generate secret");

        twofa.enroll_2fa("user123", &secret).await.unwrap();
        let codes = twofa.generate_backup_codes("user123").await.unwrap();

        let count = twofa
            .get_unused_backup_codes_count("user123")
            .await
            .unwrap();
        assert_eq!(count, 10);

        // Use one code
        twofa.verify_backup_code("user123", &codes[0]).await.unwrap();

        let count = twofa
            .get_unused_backup_codes_count("user123")
            .await
            .unwrap();
        assert_eq!(count, 9);
    }

    #[tokio::test]
    async fn test_disable_2fa() {
        let pool = setup_test_db().await;
        let crypto = CryptoService::new_for_tests();
        let twofa = TwoFAService::new(pool, crypto);

        let (_uri, secret) = twofa
            .generate_totp_secret("user123", "user@example.com")
            .expect("Failed to generate secret");

        twofa.enroll_2fa("user123", &secret).await.unwrap();
        twofa.activate_2fa("user123").await.unwrap();

        let enabled = twofa.is_2fa_enabled("user123").await.unwrap();
        assert!(enabled);

        twofa.disable_2fa("user123").await.unwrap();

        let enabled = twofa.is_2fa_enabled("user123").await.unwrap();
        assert!(!enabled);
    }

    #[tokio::test]
    async fn test_invalid_backup_code() {
        let pool = setup_test_db().await;
        let crypto = CryptoService::new_for_tests();
        let twofa = TwoFAService::new(pool, crypto);

        let (_uri, secret) = twofa
            .generate_totp_secret("user123", "user@example.com")
            .expect("Failed to generate secret");

        twofa.enroll_2fa("user123", &secret).await.unwrap();
        twofa.generate_backup_codes("user123").await.unwrap();

        // Try to use an invalid code
        let result = twofa
            .verify_backup_code("user123", "000000")
            .await
            .unwrap();
        assert!(!result);
    }
}
