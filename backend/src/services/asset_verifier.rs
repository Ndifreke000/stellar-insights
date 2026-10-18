use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::time::Duration;
use tracing::{info, warn};
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

/// SEP support declared by an anchor's `stellar.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SepSupport {
    /// SEP-24 hosted deposit/withdrawal endpoint, if declared.
    pub sep24_endpoint: Option<String>,
    /// SEP-31 cross-border payments endpoint, if declared.
    pub sep31_endpoint: Option<String>,
    /// Asset codes declared in the `CURRENCIES` section.
    pub currencies: Vec<String>,
}

impl SepSupport {
    /// An anchor is considered verified when it publishes at least one valid
    /// SEP-24/SEP-31 endpoint and declares at least one currency.
    pub fn is_verified(&self) -> bool {
        (self.sep24_endpoint.is_some() || self.sep31_endpoint.is_some())
            && !self.currencies.is_empty()
    }
}

pub struct AssetVerifier {
    http_client: Client,
    pool: SqlitePool,
}

impl AssetVerifier {
    pub fn new(pool: SqlitePool) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .user_agent("PayRaider/1.0")
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

    /// Verify an anchor by fetching its `stellar.toml` and validating SEP-24/SEP-31
    /// endpoint declarations and currencies. Returns the parsed SEP support.
    pub async fn verify_anchor_sep(&self, asset_issuer: &str) -> Result<SepSupport> {
        let home_domain = match self.get_home_domain_from_account(asset_issuer).await? {
            Some(domain) => domain,
            None => {
                info!("No home domain found for issuer: {}", asset_issuer);
                return Ok(SepSupport::default());
            }
        };

        let toml_url = format!("https://{home_domain}/.well-known/stellar.toml");

        for attempt in 1..=MAX_RETRIES {
            match self.http_client.get(&toml_url).send().await {
                Ok(response) if response.status().is_success() => {
                    if let Ok(toml_content) = response.text().await {
                        return Ok(Self::parse_sep_support(&toml_content));
                    }
                }
                Ok(response) => {
                    warn!(
                        "Anchor TOML fetch returned status {}: attempt {}/{}",
                        response.status(),
                        attempt,
                        MAX_RETRIES
                    );
                }
                Err(e) => {
                    warn!(
                        "Anchor TOML fetch failed: {} (attempt {}/{})",
                        e, attempt, MAX_RETRIES
                    );
                }
            }

            if attempt < MAX_RETRIES {
                tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * u64::from(attempt)))
                    .await;
            }
        }

        Ok(SepSupport::default())
    }

    /// Parse SEP-24/SEP-31 endpoints and currencies from `stellar.toml` content.
    pub fn parse_sep_support(toml_content: &str) -> SepSupport {
        let toml_value = match toml_content.parse::<toml::Value>() {
            Ok(value) => value,
            Err(e) => {
                warn!("Failed to parse anchor TOML: {}", e);
                return SepSupport::default();
            }
        };

        let sep24_endpoint = toml_value
            .get("TRANSFER_SERVER_SEP0024")
            .and_then(|v| v.as_str())
            .filter(|s| Self::is_valid_endpoint(s))
            .map(String::from);

        let sep31_endpoint = toml_value
            .get("DIRECT_PAYMENT_SERVER")
            .and_then(|v| v.as_str())
            .filter(|s| Self::is_valid_endpoint(s))
            .map(String::from);

        let currencies = toml_value
            .get("CURRENCIES")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.get("code").and_then(|v| v.as_str()))
                    .map(String::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        SepSupport {
            sep24_endpoint,
            sep31_endpoint,
            currencies,
        }
    }

    /// Validate that a declared endpoint is an absolute http(s) URL.
    fn is_valid_endpoint(endpoint: &str) -> bool {
        let trimmed = endpoint.trim();
        (trimmed.starts_with("https://") || trimmed.starts_with("http://"))
            && trimmed.len() > "https://".len()
    }

    /// Periodic verification job: refreshes SEP verification status for the
    /// given anchors and stores the results so the anchor API can return them.
    pub async fn run_anchor_verification_job(&self, issuers: &[String]) -> Result<()> {
        for issuer in issuers {
            match self.verify_anchor_sep(issuer).await {
                Ok(support) => {
                    let verified = support.is_verified();
                    self.store_anchor_verification(issuer, verified, &support)
                        .await?;
                    info!(
                        "Anchor {} verification refreshed: verified={}",
                        issuer, verified
                    );
                }
                Err(e) => {
                    warn!("Anchor verification failed for {}: {}", issuer, e);
                }
            }
        }

        Ok(())
    }

    /// Persist the latest anchor verification result.
    async fn store_anchor_verification(
        &self,
        asset_issuer: &str,
        verified: bool,
        support: &SepSupport,
    ) -> Result<()> {
        let sep_support_json = serde_json::to_string(support)
            .context("Failed to serialize SEP support")?;

        sqlx::query(
            r#"
            INSERT INTO anchor_verifications
                (asset_issuer, verified, sep_support, verified_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(asset_issuer) DO UPDATE SET
                verified = excluded.verified,
                sep_support = excluded.sep_support,
                verified_at = excluded.verified_at
            "#,
        )
        .bind(asset_issuer)
        .bind(verified)
        .bind(sep_support_json)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .context("Failed to store anchor verification")?;

        Ok(())
    }

    /// Check Stellar Expert for asset verification
    async fn check_stellar_expert(&self, asset_code: &str, asset_issuer: &str) -> Result<bool> {
        let url = format!("{STELLAR_EXPERT_API}/asset/{asset_code}-{asset_issuer}");

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
                tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * u64::from(attempt)))
                    .await;
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
        let toml_url = format!("https://{home_domain}/.well-known/stellar.toml");

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
                tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * u64::from(attempt)))
                    .await;
            }
        }

        (false, None)
    }

    /// Get home domain from Stellar account
    async fn get_home_domain_from_account(&self, account_id: &str) -> Result<Option<String>> {
        let url = format!("https://horizon.stellar.org/accounts/{account_id}");

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

    /// Check anchor registry (placeholder)
    fn check_anchor_registry(&self, _asset_code: &str, _asset_issuer: &str) -> Result<bool> {
        Ok(false)
    }

    /// Get on-chain metrics for an asset
    async fn get_on_chain_metrics(
        &self,
        _asset_code: &str,
        _asset_issuer: &str,
    ) -> (i64, i64, f64) {
        (0, 0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sep24_and_currencies() {
        let toml = r#"
            TRANSFER_SERVER_SEP0024 = "https://anchor.example/sep24"
            [[CURRENCIES]]
            code = "USDC"
            issuer = "GABC"
            [[CURRENCIES]]
            code = "NGN"
            issuer = "GDEF"
        "#;

        let support = AssetVerifier::parse_sep_support(toml);
        assert_eq!(
            support.sep24_endpoint.as_deref(),
            Some("https://anchor.example/sep24")
        );
        assert_eq!(support.sep31_endpoint, None);
        assert_eq!(support.currencies, vec!["USDC", "NGN"]);
        assert!(support.is_verified());
    }

    #[test]
    fn parses_sep31_endpoint() {
        let toml = r#"
            DIRECT_PAYMENT_SERVER = "https://anchor.example/sep31"
            [[CURRENCIES]]
            code = "USDC"
        "#;

        let support = AssetVerifier::parse_sep_support(toml);
        assert_eq!(
            support.sep31_endpoint.as_deref(),
            Some("https://anchor.example/sep31")
        );
        assert!(support.is_verified());
    }

    #[test]
    fn rejects_invalid_endpoint_and_missing_currencies() {
        let toml = r#"
            TRANSFER_SERVER_SEP0024 = "not-a-url"
        "#;

        let support = AssetVerifier::parse_sep_support(toml);
        assert_eq!(support.sep24_endpoint, None);
        assert!(support.currencies.is_empty());
        assert!(!support.is_verified());
    }

    #[test]
    fn invalid_toml_yields_empty_support() {
        let support = AssetVerifier::parse_sep_support("this is not toml = = =");
        assert_eq!(support, SepSupport::default());
        assert!(!support.is_verified());
    }
}
