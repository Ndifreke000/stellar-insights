use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use stellar_insights_backend::models::asset_verification::{
    ReportAssetRequest, ReportType, VerificationStatus,
};
use stellar_insights_backend::services::asset_verifier::AssetVerifier;
use uuid::Uuid;

/// Helper function to create a test database
async fn create_test_db() -> Result<SqlitePool> {
    let pool = SqlitePool::connect(":memory:").await?;

    // Run migrations
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS verified_assets (
            id TEXT PRIMARY KEY,
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            verification_status TEXT NOT NULL CHECK (verification_status IN ('verified', 'unverified', 'suspicious')),
            reputation_score REAL NOT NULL DEFAULT 0.0,
            stellar_expert_verified BOOLEAN DEFAULT FALSE,
            stellar_toml_verified BOOLEAN DEFAULT FALSE,
            anchor_registry_verified BOOLEAN DEFAULT FALSE,
            trustline_count INTEGER DEFAULT 0,
            transaction_count INTEGER DEFAULT 0,
            total_volume_usd REAL DEFAULT 0.0,
            toml_home_domain TEXT,
            toml_name TEXT,
            toml_description TEXT,
            toml_org_name TEXT,
            toml_org_url TEXT,
            toml_logo_url TEXT,
            suspicious_reports_count INTEGER DEFAULT 0,
            last_suspicious_report_at TIMESTAMP,
            last_verified_at TIMESTAMP,
            verification_notes TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(asset_code, asset_issuer)
        );

        CREATE TABLE IF NOT EXISTS asset_verification_reports (
            id TEXT PRIMARY KEY,
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            reporter_account TEXT,
            report_type TEXT NOT NULL CHECK (report_type IN ('suspicious', 'scam', 'impersonation', 'other')),
            description TEXT NOT NULL,
            evidence_url TEXT,
            status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'reviewed', 'resolved', 'dismissed')),
            reviewed_by TEXT,
            reviewed_at TIMESTAMP,
            resolution_notes TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (asset_code, asset_issuer) REFERENCES verified_assets(asset_code, asset_issuer) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS asset_verification_history (
            id TEXT PRIMARY KEY,
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            previous_status TEXT,
            new_status TEXT NOT NULL,
            previous_reputation_score REAL,
            new_reputation_score REAL NOT NULL,
            change_reason TEXT,
            changed_by TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (asset_code, asset_issuer) REFERENCES verified_assets(asset_code, asset_issuer) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

#[tokio::test]
async fn test_reputation_score_calculation() -> Result<()> {
    let pool = create_test_db().await?;
    let verifier = AssetVerifier::new(pool)?;

    // Test case 1: All verifications passed
    let result = stellar_insights_backend::services::asset_verifier::VerificationResult {
        stellar_expert_verified: true,
        stellar_toml_verified: true,
        stellar_toml_data: None,
        anchor_registry_verified: true,
        trustline_count: 15000,
        transaction_count: 150000,
        total_volume_usd: 1000000.0,
    };

    let score = verifier.calculate_reputation_score(&result);
    assert_eq!(score, 100.0); // 30 + 30 + 20 + 10 + 10

    // Test case 2: Only Stellar Expert verified
    let result = stellar_insights_backend::services::asset_verifier::VerificationResult {
        stellar_expert_verified: true,
        stellar_toml_verified: false,
        stellar_toml_data: None,
        anchor_registry_verified: false,
        trustline_count: 0,
        transaction_count: 0,
        total_volume_usd: 0.0,
    };

    let score = verifier.calculate_reputation_score(&result);
    assert_eq!(score, 30.0);

    // Test case 3: Medium metrics
    let result = stellar_insights_backend::services::asset_verifier::VerificationResult {
        stellar_expert_verified: true,
        stellar_toml_verified: true,
        stellar_toml_data: None,
        anchor_registry_verified: false,
        trustline_count: 1500,
        transaction_count: 15000,
        total_volume_usd: 100000.0,
    };

    let score = verifier.calculate_reputation_score(&result);
    assert_eq!(score, 74.0); // 30 + 30 + 0 + 7 + 7

    Ok(())
}

#[tokio::test]
async fn test_status_determination() -> Result<()> {
    let pool = create_test_db().await?;
    let verifier = AssetVerifier::new(pool)?;

    // Test case 1: Verified status
    let status = verifier.determine_status(80.0, 0);
    assert_eq!(status, VerificationStatus::Verified);

    // Test case 2: Unverified status (low score)
    let status = verifier.determine_status(40.0, 0);
    assert_eq!(status, VerificationStatus::Unverified);

    // Test case 3: Suspicious status (high reports)
    let status = verifier.determine_status(80.0, 5);
    assert_eq!(status, VerificationStatus::Suspicious);

    // Test case 4: Boundary case (exactly 60 score)
    let status = verifier.determine_status(60.0, 0);
    assert_eq!(status, VerificationStatus::Verified);

    // Test case 5: Boundary case (exactly 3 reports)
    let status = verifier.determine_status(80.0, 3);
    assert_eq!(status, VerificationStatus::Suspicious);

    Ok(())
}

#[tokio::test]
async fn test_save_and_retrieve_verification() -> Result<()> {
    let pool = create_test_db().await?;
    let verifier = AssetVerifier::new(pool.clone())?;

    let asset_code = "USDC";
    let asset_issuer = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

    let result = stellar_insights_backend::services::asset_verifier::VerificationResult {
        stellar_expert_verified: true,
        stellar_toml_verified: true,
        stellar_toml_data: Some(
            stellar_insights_backend::services::asset_verifier::StellarTomlData {
                home_domain: "centre.io".to_string(),
                name: Some("USD Coin".to_string()),
                description: Some("Stablecoin".to_string()),
                org_name: Some("Centre".to_string()),
                org_url: Some("https://centre.io".to_string()),
                logo_url: None,
            },
        ),
        anchor_registry_verified: false,
        trustline_count: 50000,
        transaction_count: 1000000,
        total_volume_usd: 50000000.0,
    };

    // Save verification result
    let saved_asset = verifier
        .save_verification_result(asset_code, asset_issuer, &result)
        .await?;

    assert_eq!(saved_asset.asset_code, asset_code);
    assert_eq!(saved_asset.asset_issuer, asset_issuer);
    assert_eq!(saved_asset.get_status(), VerificationStatus::Verified);
    assert!(saved_asset.reputation_score >= 80.0);
    assert!(saved_asset.stellar_expert_verified);
    assert!(saved_asset.stellar_toml_verified);

    // Retrieve verification result
    let retrieved_asset = verifier
        .get_verified_asset(asset_code, asset_issuer)
        .await?;

    assert!(retrieved_asset.is_some());
    let retrieved_asset = retrieved_asset.unwrap();
    assert_eq!(retrieved_asset.asset_code, asset_code);
    assert_eq!(retrieved_asset.toml_home_domain, Some("centre.io".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_list_verified_assets() -> Result<()> {
    let pool = create_test_db().await?;
    let verifier = AssetVerifier::new(pool.clone())?;

    // Insert test assets
    for i in 0..5 {
        let asset_code = format!("TEST{}", i);
        let asset_issuer = format!("G{}SEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN", i);

        let result = stellar_insights_backend::services::asset_verifier::VerificationResult {
            stellar_expert_verified: i % 2 == 0,
            stellar_toml_verified: i % 2 == 0,
            stellar_toml_data: None,
            anchor_registry_verified: false,
            trustline_count: (i as i64) * 1000,
            transaction_count: (i as i64) * 10000,
            total_volume_usd: (i as f64) * 100000.0,
        };

        verifier
            .save_verification_result(&asset_code, &asset_issuer, &result)
            .await?;
    }

    // List all assets
    let assets = verifier.list_verified_assets(None, None, 10, 0).await?;
    assert_eq!(assets.len(), 5);

    // List only verified assets
    let assets = verifier
        .list_verified_assets(Some(&VerificationStatus::Verified), None, 10, 0)
        .await?;
    assert!(assets.len() >= 2); // At least assets 0, 2, 4

    // List with minimum reputation
    let assets = verifier.list_verified_assets(None, Some(50.0), 10, 0).await?;
    assert!(!assets.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_unique_constraint() -> Result<()> {
    let pool = create_test_db().await?;
    let verifier = AssetVerifier::new(pool.clone())?;

    let asset_code = "USDC";
    let asset_issuer = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

    let result = stellar_insights_backend::services::asset_verifier::VerificationResult {
        stellar_expert_verified: true,
        stellar_toml_verified: false,
        stellar_toml_data: None,
        anchor_registry_verified: false,
        trustline_count: 1000,
        transaction_count: 10000,
        total_volume_usd: 100000.0,
    };

    // First save should succeed
    let first_save = verifier
        .save_verification_result(asset_code, asset_issuer, &result)
        .await?;
    assert_eq!(first_save.reputation_score, 37.0); // 30 + 7

    // Second save should update (not create duplicate)
    let updated_result = stellar_insights_backend::services::asset_verifier::VerificationResult {
        stellar_expert_verified: true,
        stellar_toml_verified: true,
        stellar_toml_data: None,
        anchor_registry_verified: false,
        trustline_count: 2000,
        transaction_count: 20000,
        total_volume_usd: 200000.0,
    };

    let second_save = verifier
        .save_verification_result(asset_code, asset_issuer, &updated_result)
        .await?;
    assert_eq!(second_save.reputation_score, 74.0); // 30 + 30 + 7 + 7

    // Verify only one record exists
    let assets = verifier.list_verified_assets(None, None, 10, 0).await?;
    assert_eq!(assets.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_verification() -> Result<()> {
    let pool = create_test_db().await?;

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                let verifier = AssetVerifier::new(pool_clone).unwrap();
                let asset_code = format!("CONCURRENT{}", i);
                let asset_issuer =
                    format!("G{}SEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN", i);

                let result =
                    stellar_insights_backend::services::asset_verifier::VerificationResult {
                        stellar_expert_verified: true,
                        stellar_toml_verified: false,
                        stellar_toml_data: None,
                        anchor_registry_verified: false,
                        trustline_count: 100,
                        transaction_count: 1000,
                        total_volume_usd: 10000.0,
                    };

                verifier
                    .save_verification_result(&asset_code, &asset_issuer, &result)
                    .await
            })
        })
        .collect();

    // Wait for all tasks to complete
    for handle in handles {
        handle.await??;
    }

    // Verify all assets were saved
    let verifier = AssetVerifier::new(pool)?;
    let assets = verifier.list_verified_assets(None, None, 10, 0).await?;
    assert_eq!(assets.len(), 5);

    Ok(())
}

#[tokio::test]
async fn test_similar_asset_codes() -> Result<()> {
    let pool = create_test_db().await?;
    let verifier = AssetVerifier::new(pool.clone())?;

    // Create assets with similar codes but different issuers
    let assets = vec![
        ("USDC", "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
        ("USDC", "GB5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
        ("USD", "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
    ];

    for (code, issuer) in &assets {
        let result = stellar_insights_backend::services::asset_verifier::VerificationResult {
            stellar_expert_verified: true,
            stellar_toml_verified: false,
            stellar_toml_data: None,
            anchor_registry_verified: false,
            trustline_count: 1000,
            transaction_count: 10000,
            total_volume_usd: 100000.0,
        };

        verifier
            .save_verification_result(code, issuer, &result)
            .await?;
    }

    // Verify each asset is stored separately
    let all_assets = verifier.list_verified_assets(None, None, 10, 0).await?;
    assert_eq!(all_assets.len(), 3);

    // Verify we can retrieve each one individually
    for (code, issuer) in &assets {
        let asset = verifier.get_verified_asset(code, issuer).await?;
        assert!(asset.is_some());
        let asset = asset.unwrap();
        assert_eq!(&asset.asset_code, code);
        assert_eq!(&asset.asset_issuer, issuer);
    }

    Ok(())
}
