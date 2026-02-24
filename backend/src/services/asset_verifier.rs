use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::asset_verification::{
    StellarTomlData, VerificationResult, VerificationStatus, VerifiedAsset,
};

const STELLAR_EXPERT_API: &str = "https://api.stellar.expert/explorer/public";
const REQUEST_TIMEOUT_SECS: u64 = 10;
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StellarExpertAsset {
    asset: String,
    domain: Option<String>,
    toml_info: Option<TomlInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TomlInfo {
    name: Option<String>,
    desc: Option<String>,
    org_name: Option<String>,
    org_url: Option<String>,
    image: Option<String>,
}

pub struct AssetVerifier {
    http_client: Client,
    pool: SqlitePool,
}

impl AssetVerifier {
    pub fn new(pool: SqlitePool) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .user_agent("StellarInsights/1.0")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { http_client, pool })
    }

    /// Main verification method that checks all sources
    pub async fn verify_asset(
        &self,
        asset_code: &str,
        asset_issuer: &str,
    ) -> Result<VerificationResult> {
        info!(
            "Starting verification for asset: {}:{}",
            asset_code, asset_issuer
        );

        // Check Stellar Expert
        let stellar_expert_verified = self
            .check_stellar_expert(asset_code, asset_issuer)
            .await
            .unwrap_or(false);

        // Check stellar.toml
        let (stellar_toml_verified, stellar_toml_data) =
            self.check_stellar_toml(asset_issuer).await;

        // Check anchor registry (placeholder - would integrate with actual registry)
        let anchor_registry_verified = self
            .check_anchor_registry(asset_code, asset_issuer)
            .await
            .unwrap_or(false);

        // Get on-chain metrics
        let (trustline_count, transaction_count, total_volume_usd) =
            self.get_on_chain_metrics(asset_code, asset_issuer).await;

        Ok(VerificationResult {
            stellar_expert_verified,
            stellar_toml_verified,
            stellar_toml_data,
            anchor_registry_verified,
            trustline_count,
            transaction_count,
            total_volume_usd,
        })
    }

    /// Check Stellar Expert for asset verification
    async fn check_stellar_expert(&self, asset_code: &str, asset_issuer: &str) -> Result<bool> {
        let url = format!(
            "{}/asset/{}-{}",
            STELLAR_EXPERT_API, asset_code, asset_issuer
        );

        for attempt in 1..=MAX_RETRIES {
            match self.http_client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    let asset_data: StellarExpertAsset = response.json().await?;
                    // Asset exists in Stellar Expert and has domain info
                    return Ok(asset_data.domain.is_some());
                }
                Ok(response) if response.status().as_u16() == 404 => {
                    return Ok(false);
                }
                Ok(response) => {
                    warn!(
                        "Stellar Expert returned status {}: attempt {}/{}",
                        response.status(),
                        attempt,
                        MAX_RETRIES
                    );
                }
                Err(e) => {
                    warn!(
                        "Stellar Expert request failed: {} (attempt {}/{})",
                        e, attempt, MAX_RETRIES
                    );
                }
            }

            if attempt < MAX_RETRIES {
                tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * attempt as u64)).await;
            }
        }

        Ok(false)
    }

    /// Check and parse stellar.toml file
    async fn check_stellar_toml(&self, asset_issuer: &str) -> (bool, Option<StellarTomlData>) {
        // First, try to get the home domain from the issuer account
        let home_domain = match self.get_home_domain_from_account(asset_issuer).await {
            Ok(Some(domain)) => domain,
            Ok(None) => {
                info!("No home domain found for issuer: {}", asset_issuer);
                return (false, None);
            }
            Err(e) => {
                warn!("Failed to get home domain: {}", e);
                return (false, None);
            }
        };

        // Fetch and parse stellar.toml
        let toml_url = format!("https://{}/.well-known/stellar.toml", home_domain);

        for attempt in 1..=MAX_RETRIES {
            match self.http_client.get(&toml_url).send().await {
                Ok(response) if response.status().is_success() => match response.text().await {
                    Ok(toml_content) => {
                        return self.parse_stellar_toml(&toml_content, &home_domain);
                    }
                    Err(e) => {
                        warn!("Failed to read TOML content: {}", e);
                    }
                },
                Ok(response) => {
                    warn!(
                        "TOML fetch returned status {}: attempt {}/{}",
                        response.status(),
                        attempt,
                        MAX_RETRIES
                    );
                }
                Err(e) => {
                    warn!(
                        "TOML fetch failed: {} (attempt {}/{})",
                        e, attempt, MAX_RETRIES
                    );
                }
            }

            if attempt < MAX_RETRIES {
                tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * attempt as u64)).await;
            }
        }

        (false, None)
    }

    /// Get home domain from Stellar account
    async fn get_home_domain_from_account(&self, account_id: &str) -> Result<Option<String>> {
        let url = format!("https://horizon.stellar.org/accounts/{}", account_id);

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        #[derive(Deserialize)]
        struct AccountResponse {
            home_domain: Option<String>,
        }

        let account: AccountResponse = response.json().await?;
        Ok(account.home_domain)
    }

    /// Parse stellar.toml content
    fn parse_stellar_toml(
        &self,
        toml_content: &str,
        home_domain: &str,
    ) -> (bool, Option<StellarTomlData>) {
        match toml_content.parse::<toml::Value>() {
            Ok(toml_value) => {
                let documentation = toml_value.get("DOCUMENTATION");
                let org_name = documentation
                    .and_then(|d| d.get("ORG_NAME"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let org_url = documentation
                    .and_then(|d| d.get("ORG_URL"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                // Check for currencies section
                let has_currencies = toml_value.get("CURRENCIES").is_some();

                let toml_data = StellarTomlData {
                    home_domain: home_domain.to_string(),
                    name: None, // Would extract from CURRENCIES section
                    description: None,
                    org_name,
                    org_url,
                    logo_url: None,
                };

                (has_currencies, Some(toml_data))
            }
            Err(e) => {
                warn!("Failed to parse TOML: {}", e);
                (false, None)
            }
        }
    }

    /// Check anchor registry (placeholder implementation)
    async fn check_anchor_registry(&self, _asset_code: &str, _asset_issuer: &str) -> Result<bool> {
        // Placeholder: Would integrate with actual anchor registry
        // For now, return false
        Ok(false)
    }

    /// Get on-chain metrics from database or Horizon
    async fn get_on_chain_metrics(&self, asset_code: &str, asset_issuer: &str) -> (i64, i64, f64) {
        // Try to get from database first
        if let Ok(Some(metrics)) = self.get_metrics_from_db(asset_code, asset_issuer).await {
            return metrics;
        }

        // Fallback to Horizon API
        self.get_metrics_from_horizon(asset_code, asset_issuer)
            .await
            .unwrap_or((0, 0, 0.0))
    }

    /// Get metrics from database
    async fn get_metrics_from_db(
        &self,
        asset_code: &str,
        asset_issuer: &str,
    ) -> Result<Option<(i64, i64, f64)>> {
        let result = sqlx::query_as::<_, (i64, i64, f64)>(
            r#"
            SELECT 
                COALESCE(SUM(trustline_count), 0) as trustline_count,
                COALESCE(COUNT(*), 0) as transaction_count,
                COALESCE(SUM(total_volume_usd), 0.0) as total_volume
            FROM payments
            WHERE asset_code = $1 AND asset_issuer = $2
            "#,
        )
        .bind(asset_code)
        .bind(asset_issuer)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get metrics from Horizon API
    async fn get_metrics_from_horizon(
        &self,
        asset_code: &str,
        asset_issuer: &str,
    ) -> Result<(i64, i64, f64)> {
        let url = format!(
            "https://horizon.stellar.org/assets?asset_code={}&asset_issuer={}",
            asset_code, asset_issuer
        );

        #[derive(Deserialize)]
        struct AssetRecord {
            num_accounts: i64,
            #[serde(default)]
            num_claimable_balances: i64,
        }

        #[derive(Deserialize)]
        struct AssetsResponse {
            _embedded: Embedded,
        }

        #[derive(Deserialize)]
        struct Embedded {
            records: Vec<AssetRecord>,
        }

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok((0, 0, 0.0));
        }

        let assets: AssetsResponse = response.json().await?;

        if let Some(asset) = assets._embedded.records.first() {
            Ok((asset.num_accounts, 0, 0.0))
        } else {
            Ok((0, 0, 0.0))
        }
    }

    /// Calculate reputation score based on verification results
    pub fn calculate_reputation_score(&self, result: &VerificationResult) -> f64 {
        let mut score: f64 = 0.0;

        // Stellar Expert verification (30 points)
        if result.stellar_expert_verified {
            score += 30.0;
        }

        // Stellar TOML verification (30 points)
        if result.stellar_toml_verified {
            score += 30.0;
        }

        // Anchor registry verification (20 points)
        if result.anchor_registry_verified {
            score += 20.0;
        }

        // Trustline count (up to 10 points)
        if result.trustline_count > 10000 {
            score += 10.0;
        } else if result.trustline_count > 1000 {
            score += 7.0;
        } else if result.trustline_count > 100 {
            score += 5.0;
        } else if result.trustline_count > 10 {
            score += 2.0;
        }

        // Transaction count (up to 10 points)
        if result.transaction_count > 100000 {
            score += 10.0;
        } else if result.transaction_count > 10000 {
            score += 7.0;
        } else if result.transaction_count > 1000 {
            score += 5.0;
        } else if result.transaction_count > 100 {
            score += 2.0;
        }

        score.min(100.0)
    }

    /// Determine verification status based on reputation score and other factors
    pub fn determine_status(
        &self,
        reputation_score: f64,
        suspicious_reports_count: i64,
    ) -> VerificationStatus {
        if suspicious_reports_count >= 3 {
            return VerificationStatus::Suspicious;
        }

        if reputation_score >= 60.0 {
            VerificationStatus::Verified
        } else {
            VerificationStatus::Unverified
        }
    }

    /// Save or update verification result in database
    pub async fn save_verification_result(
        &self,
        asset_code: &str,
        asset_issuer: &str,
        result: &VerificationResult,
        reputation_score: f64,
        status: VerificationStatus,
    ) -> Result<VerifiedAsset> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let toml_home_domain = result
            .stellar_toml_data
            .as_ref()
            .map(|d| d.home_domain.clone());
        let toml_name = result
            .stellar_toml_data
            .as_ref()
            .and_then(|d| d.name.clone());
        let toml_description = result
            .stellar_toml_data
            .as_ref()
            .and_then(|d| d.description.clone());
        let toml_org_name = result
            .stellar_toml_data
            .as_ref()
            .and_then(|d| d.org_name.clone());
        let toml_org_url = result
            .stellar_toml_data
            .as_ref()
            .and_then(|d| d.org_url.clone());
        let toml_logo_url = result
            .stellar_toml_data
            .as_ref()
            .and_then(|d| d.logo_url.clone());

        let verified_asset = sqlx::query_as::<_, VerifiedAsset>(
            r#"
            INSERT INTO verified_assets (
                id, asset_code, asset_issuer, verification_status, reputation_score,
                stellar_expert_verified, stellar_toml_verified, anchor_registry_verified,
                trustline_count, transaction_count, total_volume_usd,
                toml_home_domain, toml_name, toml_description, toml_org_name, toml_org_url, toml_logo_url,
                last_verified_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            ON CONFLICT (asset_code, asset_issuer) DO UPDATE SET
                verification_status = EXCLUDED.verification_status,
                reputation_score = EXCLUDED.reputation_score,
                stellar_expert_verified = EXCLUDED.stellar_expert_verified,
                stellar_toml_verified = EXCLUDED.stellar_toml_verified,
                anchor_registry_verified = EXCLUDED.anchor_registry_verified,
                trustline_count = EXCLUDED.trustline_count,
                transaction_count = EXCLUDED.transaction_count,
                total_volume_usd = EXCLUDED.total_volume_usd,
                toml_home_domain = EXCLUDED.toml_home_domain,
                toml_name = EXCLUDED.toml_name,
                toml_description = EXCLUDED.toml_description,
                toml_org_name = EXCLUDED.toml_org_name,
                toml_org_url = EXCLUDED.toml_org_url,
                toml_logo_url = EXCLUDED.toml_logo_url,
                last_verified_at = EXCLUDED.last_verified_at,
                updated_at = EXCLUDED.updated_at
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(asset_code)
        .bind(asset_issuer)
        .bind(status.as_str())
        .bind(reputation_score)
        .bind(result.stellar_expert_verified)
        .bind(result.stellar_toml_verified)
        .bind(result.anchor_registry_verified)
        .bind(result.trustline_count)
        .bind(result.transaction_count)
        .bind(result.total_volume_usd)
        .bind(toml_home_domain)
        .bind(toml_name)
        .bind(toml_description)
        .bind(toml_org_name)
        .bind(toml_org_url)
        .bind(toml_logo_url)
        .bind(now)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        // Record history
        self.record_verification_history(
            asset_code,
            asset_issuer,
            None,
            status.as_str(),
            None,
            reputation_score,
            "Automated verification",
        )
        .await?;

        info!(
            "Saved verification result for {}:{} - Status: {:?}, Score: {}",
            asset_code, asset_issuer, status, reputation_score
        );

        Ok(verified_asset)
    }

    /// Record verification history
    async fn record_verification_history(
        &self,
        asset_code: &str,
        asset_issuer: &str,
        previous_status: Option<&str>,
        new_status: &str,
        previous_score: Option<f64>,
        new_score: f64,
        reason: &str,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO asset_verification_history (
                id, asset_code, asset_issuer, previous_status, new_status,
                previous_reputation_score, new_reputation_score, change_reason, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(&id)
        .bind(asset_code)
        .bind(asset_issuer)
        .bind(previous_status)
        .bind(new_status)
        .bind(previous_score)
        .bind(new_score)
        .bind(reason)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get verified asset from database
    pub async fn get_verified_asset(
        &self,
        asset_code: &str,
        asset_issuer: &str,
    ) -> Result<Option<VerifiedAsset>> {
        let asset = sqlx::query_as::<_, VerifiedAsset>(
            r#"
            SELECT * FROM verified_assets
            WHERE asset_code = $1 AND asset_issuer = $2
            "#,
        )
        .bind(asset_code)
        .bind(asset_issuer)
        .fetch_optional(&self.pool)
        .await?;

        Ok(asset)
    }

    /// List verified assets with filters
    pub async fn list_verified_assets(
        &self,
        status: Option<VerificationStatus>,
        min_reputation: Option<f64>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<VerifiedAsset>> {
        let mut query = String::from("SELECT * FROM verified_assets WHERE 1=1");

        if let Some(status) = status {
            query.push_str(&format!(" AND verification_status = '{}'", status.as_str()));
        }

        if let Some(min_rep) = min_reputation {
            query.push_str(&format!(" AND reputation_score >= {}", min_rep));
        }

        query.push_str(" ORDER BY reputation_score DESC, updated_at DESC");
        query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let assets = sqlx::query_as::<_, VerifiedAsset>(&query)
            .fetch_all(&self.pool)
            .await?;

        Ok(assets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_reputation_score() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let verifier = AssetVerifier::new(pool).unwrap();

        let result = VerificationResult {
            stellar_expert_verified: true,
            stellar_toml_verified: true,
            stellar_toml_data: None,
            anchor_registry_verified: false,
            trustline_count: 5000,
            transaction_count: 50000,
            total_volume_usd: 1000000.0,
        };

        let score = verifier.calculate_reputation_score(&result);
        assert!(score >= 60.0);
        assert!(score <= 100.0);
    }

    #[test]
    fn test_determine_status() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let verifier = AssetVerifier::new(pool).unwrap();

        assert_eq!(
            verifier.determine_status(80.0, 0),
            VerificationStatus::Verified
        );
        assert_eq!(
            verifier.determine_status(40.0, 0),
            VerificationStatus::Unverified
        );
        assert_eq!(
            verifier.determine_status(80.0, 3),
            VerificationStatus::Suspicious
        );
    }
}
