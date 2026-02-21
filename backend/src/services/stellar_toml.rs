use anyhow::{anyhow, Result};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use url::Url;

/// Cache TTL for successful stellar.toml fetches (24 hours)
const SUCCESS_CACHE_TTL: u64 = 24 * 60 * 60;

/// Cache TTL for failed fetches (1 hour)
const FAILURE_CACHE_TTL: u64 = 60 * 60;

/// Request timeout for stellar.toml fetches
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum response size (1MB)
const MAX_RESPONSE_SIZE: usize = 1024 * 1024;

/// Stellar.toml metadata according to SEP-1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StellarToml {
    // Organization Information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_dba: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_logo: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_physical_address: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_phone_number: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_keybase: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_twitter: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_github: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_official_email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_support_email: Option<String>,

    // Network Information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_passphrase: Option<String>,

    // Currencies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currencies: Option<Vec<CurrencyInfo>>,

    // Principals
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principals: Option<Vec<Principal>>,

    // Documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<Documentation>,

    // Metadata
    pub domain: String,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyInfo {
    pub code: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_decimals: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_number: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_number: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_unlimited: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_asset_anchored: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_asset_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_asset: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub redemption_instructions: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub keybase: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub github: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Documentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_dba: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_logo: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_description: Option<String>,
}

/// Cached result for stellar.toml fetch
#[derive(Debug, Clone, Serialize, Deserialize)]
enum CachedResult {
    Success(StellarToml),
    Failure(String),
}

/// Stellar.toml client for fetching and parsing anchor metadata
pub struct StellarTomlClient {
    http_client: Client,
    redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    network_passphrase: Option<String>,
}

impl StellarTomlClient {
    /// Create a new StellarTomlClient
    pub fn new(
        redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
        network_passphrase: Option<String>,
    ) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent("StellarInsights/1.0")
            .redirect(reqwest::redirect::Policy::limited(3))
            .build()?;

        Ok(Self {
            http_client,
            redis_connection,
            network_passphrase,
        })
    }

    /// Fetch stellar.toml for a domain with caching
    pub async fn fetch_toml(&self, domain: &str) -> Result<StellarToml> {
        // Validate domain
        self.validate_domain(domain)?;

        // Check cache first
        if let Some(cached) = self.get_from_cache(domain).await? {
            return match cached {
                CachedResult::Success(toml) => Ok(toml),
                CachedResult::Failure(err) => Err(anyhow!("Cached failure: {}", err)),
            };
        }

        // Fetch from network
        match self.fetch_toml_from_network(domain).await {
            Ok(toml) => {
                // Cache success
                self.cache_success(domain, &toml).await?;
                Ok(toml)
            }
            Err(e) => {
                // Cache failure
                self.cache_failure(domain, &e.to_string()).await?;
                Err(e)
            }
        }
    }

    /// Fetch stellar.toml without caching (for background refresh)
    pub async fn fetch_toml_no_cache(&self, domain: &str) -> Result<StellarToml> {
        self.validate_domain(domain)?;
        self.fetch_toml_from_network(domain).await
    }

    /// Invalidate cache for a domain
    pub async fn invalidate_cache(&self, domain: &str) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("stellar_toml:{}", domain);
            conn.del::<_, ()>(&key)
                .await
                .map_err(|e| anyhow!("Failed to invalidate cache: {}", e))?;
        }
        Ok(())
    }

    /// Background refresh for popular anchors
    pub async fn background_refresh(&self, domain: &str) -> Result<()> {
        match self.fetch_toml_no_cache(domain).await {
            Ok(toml) => {
                self.cache_success(domain, &toml).await?;
                tracing::info!("Background refresh successful for domain: {}", domain);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Background refresh failed for domain {}: {}", domain, e);
                Err(e)
            }
        }
    }

    // Private methods

    /// Validate domain to prevent SSRF
    fn validate_domain(&self, domain: &str) -> Result<()> {
        // Check for empty domain
        if domain.is_empty() {
            return Err(anyhow!("Domain cannot be empty"));
        }

        // Check for invalid characters
        if domain.contains("..") || domain.contains("//") {
            return Err(anyhow!("Invalid domain format"));
        }

        // Check for IP addresses (prevent direct IP access)
        if domain.parse::<std::net::IpAddr>().is_ok() {
            return Err(anyhow!("IP addresses not allowed"));
        }

        // Check for localhost/private networks
        let lowercase = domain.to_lowercase();
        if lowercase.contains("localhost")
            || lowercase.contains("127.0.0.1")
            || lowercase.contains("0.0.0.0")
            || lowercase.starts_with("10.")
            || lowercase.starts_with("192.168.")
            || lowercase.starts_with("172.")
        {
            return Err(anyhow!("Private network domains not allowed"));
        }

        // Check length
        if domain.len() > 253 {
            return Err(anyhow!("Domain too long"));
        }

        Ok(())
    }

    /// Fetch stellar.toml from network
    async fn fetch_toml_from_network(&self, domain: &str) -> Result<StellarToml> {
        // Try HTTPS first
        let https_url = format!("https://{}/.well-known/stellar.toml", domain);

        match self.fetch_url(&https_url).await {
            Ok(content) => return self.parse_toml(&content, domain),
            Err(e) => {
                tracing::debug!("HTTPS fetch failed for {}: {}", domain, e);
            }
        }

        // Fallback to HTTP
        let http_url = format!("http://{}/.well-known/stellar.toml", domain);

        match self.fetch_url(&http_url).await {
            Ok(content) => self.parse_toml(&content, domain),
            Err(e) => {
                tracing::warn!("HTTP fetch also failed for {}: {}", domain, e);
                Err(anyhow!(
                    "Failed to fetch stellar.toml from both HTTPS and HTTP"
                ))
            }
        }
    }

    /// Fetch URL content
    async fn fetch_url(&self, url: &str) -> Result<String> {
        // Validate URL
        let parsed_url = Url::parse(url).map_err(|e| anyhow!("Invalid URL: {}", e))?;

        // Additional security checks
        if parsed_url.scheme() != "https" && parsed_url.scheme() != "http" {
            return Err(anyhow!("Only HTTP(S) schemes allowed"));
        }

        // Fetch content
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow!("Request failed: {}", e))?;

        // Check status
        if !response.status().is_success() {
            return Err(anyhow!("HTTP error: {}", response.status()));
        }

        // Check content length
        if let Some(content_length) = response.content_length() {
            if content_length > MAX_RESPONSE_SIZE as u64 {
                return Err(anyhow!("Response too large"));
            }
        }

        // Read body with size limit
        let bytes = response
            .bytes()
            .await
            .map_err(|e| anyhow!("Failed to read response: {}", e))?;

        if bytes.len() > MAX_RESPONSE_SIZE {
            return Err(anyhow!("Response exceeds size limit"));
        }

        String::from_utf8(bytes.to_vec()).map_err(|e| anyhow!("Invalid UTF-8: {}", e))
    }

    /// Parse TOML content
    fn parse_toml(&self, content: &str, domain: &str) -> Result<StellarToml> {
        // Parse TOML
        let parsed: toml::Value =
            toml::from_str(content).map_err(|e| anyhow!("Failed to parse TOML: {}", e))?;

        // Extract organization information
        let organization_name = parsed
            .get("ORGANIZATION_NAME")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let organization_dba = parsed
            .get("ORGANIZATION_DBA")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let organization_url = parsed
            .get("ORGANIZATION_URL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let organization_logo = parsed
            .get("ORGANIZATION_LOGO")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let organization_description = parsed
            .get("ORGANIZATION_DESCRIPTION")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let organization_physical_address = parsed
            .get("ORGANIZATION_PHYSICAL_ADDRESS")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let organization_phone_number = parsed
            .get("ORGANIZATION_PHONE_NUMBER")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let organization_keybase = parsed
            .get("ORGANIZATION_KEYBASE")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let organization_twitter = parsed
            .get("ORGANIZATION_TWITTER")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let organization_github = parsed
            .get("ORGANIZATION_GITHUB")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let organization_official_email = parsed
            .get("ORGANIZATION_OFFICIAL_EMAIL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let organization_support_email = parsed
            .get("ORGANIZATION_SUPPORT_EMAIL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract network passphrase
        let network_passphrase = parsed
            .get("NETWORK_PASSPHRASE")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Validate network passphrase if configured
        if let Some(ref expected) = self.network_passphrase {
            if let Some(ref actual) = network_passphrase {
                if actual != expected {
                    tracing::warn!(
                        "Network passphrase mismatch for {}: expected {}, got {}",
                        domain,
                        expected,
                        actual
                    );
                }
            }
        }

        // Parse currencies
        let currencies = self.parse_currencies(&parsed)?;

        // Parse principals
        let principals = self.parse_principals(&parsed)?;

        // Parse documentation
        let documentation = self.parse_documentation(&parsed)?;

        Ok(StellarToml {
            organization_name,
            organization_dba,
            organization_url,
            organization_logo,
            organization_description,
            organization_physical_address,
            organization_phone_number,
            organization_keybase,
            organization_twitter,
            organization_github,
            organization_official_email,
            organization_support_email,
            network_passphrase,
            currencies,
            principals,
            documentation,
            domain: domain.to_string(),
            fetched_at: chrono::Utc::now().timestamp(),
        })
    }

    /// Parse currencies from TOML
    fn parse_currencies(&self, parsed: &toml::Value) -> Result<Option<Vec<CurrencyInfo>>> {
        let currencies_array = match parsed.get("CURRENCIES") {
            Some(toml::Value::Array(arr)) => arr,
            _ => return Ok(None),
        };

        let mut currencies = Vec::new();

        for currency in currencies_array {
            if let toml::Value::Table(table) = currency {
                let code = table
                    .get("code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Currency missing code"))?
                    .to_string();

                currencies.push(CurrencyInfo {
                    code,
                    issuer: table
                        .get("issuer")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    display_decimals: table
                        .get("display_decimals")
                        .and_then(|v| v.as_integer())
                        .map(|i| i as i32),
                    name: table
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    desc: table
                        .get("desc")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    conditions: table
                        .get("conditions")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    image: table
                        .get("image")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    fixed_number: table.get("fixed_number").and_then(|v| v.as_integer()),
                    max_number: table.get("max_number").and_then(|v| v.as_integer()),
                    is_unlimited: table.get("is_unlimited").and_then(|v| v.as_bool()),
                    is_asset_anchored: table.get("is_asset_anchored").and_then(|v| v.as_bool()),
                    anchor_asset_type: table
                        .get("anchor_asset_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    anchor_asset: table
                        .get("anchor_asset")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    redemption_instructions: table
                        .get("redemption_instructions")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    status: table
                        .get("status")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }

        Ok(if currencies.is_empty() {
            None
        } else {
            Some(currencies)
        })
    }

    /// Parse principals from TOML
    fn parse_principals(&self, parsed: &toml::Value) -> Result<Option<Vec<Principal>>> {
        let principals_array = match parsed.get("PRINCIPALS") {
            Some(toml::Value::Array(arr)) => arr,
            _ => return Ok(None),
        };

        let mut principals = Vec::new();

        for principal in principals_array {
            if let toml::Value::Table(table) = principal {
                principals.push(Principal {
                    name: table
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    email: table
                        .get("email")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    keybase: table
                        .get("keybase")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    twitter: table
                        .get("twitter")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    github: table
                        .get("github")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }

        Ok(if principals.is_empty() {
            None
        } else {
            Some(principals)
        })
    }

    /// Parse documentation from TOML
    fn parse_documentation(&self, parsed: &toml::Value) -> Result<Option<Documentation>> {
        let doc_table = match parsed.get("DOCUMENTATION") {
            Some(toml::Value::Table(table)) => table,
            _ => return Ok(None),
        };

        Ok(Some(Documentation {
            org_name: doc_table
                .get("ORG_NAME")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            org_dba: doc_table
                .get("ORG_DBA")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            org_url: doc_table
                .get("ORG_URL")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            org_logo: doc_table
                .get("ORG_LOGO")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            org_description: doc_table
                .get("ORG_DESCRIPTION")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }))
    }

    /// Get from cache
    async fn get_from_cache(&self, domain: &str) -> Result<Option<CachedResult>> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("stellar_toml:{}", domain);

            let cached: Option<String> = conn
                .get(&key)
                .await
                .map_err(|e| anyhow!("Failed to get from cache: {}", e))?;

            if let Some(json) = cached {
                let result: CachedResult = serde_json::from_str(&json)?;
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    /// Cache success
    async fn cache_success(&self, domain: &str, toml: &StellarToml) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("stellar_toml:{}", domain);
            let cached = CachedResult::Success(toml.clone());
            let json = serde_json::to_string(&cached)?;

            conn.set_ex::<_, _, ()>(&key, json, SUCCESS_CACHE_TTL)
                .await
                .map_err(|e| anyhow!("Failed to cache success: {}", e))?;
        }
        Ok(())
    }

    /// Cache failure
    async fn cache_failure(&self, domain: &str, error: &str) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("stellar_toml:{}", domain);
            let cached = CachedResult::Failure(error.to_string());
            let json = serde_json::to_string(&cached)?;

            conn.set_ex::<_, _, ()>(&key, json, FAILURE_CACHE_TTL)
                .await
                .map_err(|e| anyhow!("Failed to cache failure: {}", e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_domain() {
        let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

        // Valid domains
        assert!(client.validate_domain("example.com").is_ok());
        assert!(client.validate_domain("sub.example.com").is_ok());
        assert!(client.validate_domain("stellar.org").is_ok());

        // Invalid domains
        assert!(client.validate_domain("").is_err());
        assert!(client.validate_domain("..").is_err());
        assert!(client.validate_domain("example..com").is_err());
        assert!(client.validate_domain("127.0.0.1").is_err());
        assert!(client.validate_domain("localhost").is_err());
        assert!(client.validate_domain("10.0.0.1").is_err());
        assert!(client.validate_domain("192.168.1.1").is_err());
    }

    #[test]
    fn test_parse_toml_basic() {
        let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

        let toml_content = r#"
ORGANIZATION_NAME = "Test Anchor"
ORGANIZATION_DBA = "Test DBA"
ORGANIZATION_URL = "https://test.com"
ORGANIZATION_LOGO = "https://test.com/logo.png"
ORGANIZATION_DESCRIPTION = "A test anchor"
ORGANIZATION_SUPPORT_EMAIL = "support@test.com"
NETWORK_PASSPHRASE = "Test SDF Network ; September 2015"
        "#;

        let result = client.parse_toml(toml_content, "test.com");
        assert!(result.is_ok());

        let toml = result.unwrap();
        assert_eq!(toml.organization_name, Some("Test Anchor".to_string()));
        assert_eq!(toml.organization_dba, Some("Test DBA".to_string()));
        assert_eq!(toml.organization_url, Some("https://test.com".to_string()));
        assert_eq!(
            toml.organization_logo,
            Some("https://test.com/logo.png".to_string())
        );
        assert_eq!(
            toml.organization_support_email,
            Some("support@test.com".to_string())
        );
        assert_eq!(toml.domain, "test.com");
    }

    #[test]
    fn test_parse_toml_with_currencies() {
        let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

        let toml_content = r#"
ORGANIZATION_NAME = "Test Anchor"

[[CURRENCIES]]
code = "USD"
issuer = "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
display_decimals = 2
name = "US Dollar"
desc = "US Dollar token"
is_asset_anchored = true
anchor_asset_type = "fiat"
anchor_asset = "USD"

[[CURRENCIES]]
code = "EUR"
issuer = "GYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY"
display_decimals = 2
name = "Euro"
        "#;

        let result = client.parse_toml(toml_content, "test.com");
        assert!(result.is_ok());

        let toml = result.unwrap();
        assert!(toml.currencies.is_some());

        let currencies = toml.currencies.unwrap();
        assert_eq!(currencies.len(), 2);
        assert_eq!(currencies[0].code, "USD");
        assert_eq!(currencies[0].name, Some("US Dollar".to_string()));
        assert_eq!(currencies[1].code, "EUR");
    }

    #[test]
    fn test_parse_invalid_toml() {
        let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

        let invalid_toml = "INVALID TOML [[[";
        let result = client.parse_toml(invalid_toml, "test.com");
        assert!(result.is_err());
    }
}
