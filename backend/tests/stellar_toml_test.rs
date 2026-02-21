use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use stellar_insights_backend::services::stellar_toml::StellarTomlClient;

#[tokio::test]
async fn test_domain_validation() {
    let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

    // Valid domains
    assert!(client.validate_domain("stellar.org").is_ok());
    assert!(client.validate_domain("example.com").is_ok());
    assert!(client.validate_domain("sub.example.com").is_ok());
    assert!(client.validate_domain("test-anchor.stellar.org").is_ok());

    // Invalid domains - empty
    assert!(client.validate_domain("").is_err());

    // Invalid domains - path traversal attempts
    assert!(client.validate_domain("..").is_err());
    assert!(client.validate_domain("example..com").is_err());
    assert!(client.validate_domain("example.com//test").is_err());

    // Invalid domains - IP addresses
    assert!(client.validate_domain("127.0.0.1").is_err());
    assert!(client.validate_domain("192.168.1.1").is_err());
    assert!(client.validate_domain("10.0.0.1").is_err());
    assert!(client.validate_domain("172.16.0.1").is_err());

    // Invalid domains - localhost
    assert!(client.validate_domain("localhost").is_err());
    assert!(client.validate_domain("LOCALHOST").is_err());
    assert!(client.validate_domain("localhost.localdomain").is_err());

    // Invalid domains - private networks
    assert!(client.validate_domain("0.0.0.0").is_err());

    // Invalid domains - too long
    let long_domain = "a".repeat(254);
    assert!(client.validate_domain(&long_domain).is_err());
}

#[tokio::test]
async fn test_parse_basic_toml() {
    let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

    let toml_content = r#"
ORGANIZATION_NAME = "Example Anchor"
ORGANIZATION_DBA = "Example DBA"
ORGANIZATION_URL = "https://example.com"
ORGANIZATION_LOGO = "https://example.com/logo.png"
ORGANIZATION_DESCRIPTION = "An example anchor for testing"
ORGANIZATION_SUPPORT_EMAIL = "support@example.com"
ORGANIZATION_OFFICIAL_EMAIL = "info@example.com"
NETWORK_PASSPHRASE = "Public Global Stellar Network ; September 2015"
    "#;

    let result = client.parse_toml(toml_content, "example.com");
    assert!(result.is_ok());

    let toml = result.unwrap();
    assert_eq!(toml.organization_name, Some("Example Anchor".to_string()));
    assert_eq!(toml.organization_dba, Some("Example DBA".to_string()));
    assert_eq!(
        toml.organization_url,
        Some("https://example.com".to_string())
    );
    assert_eq!(
        toml.organization_logo,
        Some("https://example.com/logo.png".to_string())
    );
    assert_eq!(
        toml.organization_description,
        Some("An example anchor for testing".to_string())
    );
    assert_eq!(
        toml.organization_support_email,
        Some("support@example.com".to_string())
    );
    assert_eq!(
        toml.organization_official_email,
        Some("info@example.com".to_string())
    );
    assert_eq!(
        toml.network_passphrase,
        Some("Public Global Stellar Network ; September 2015".to_string())
    );
    assert_eq!(toml.domain, "example.com");
}

#[tokio::test]
async fn test_parse_toml_with_currencies() {
    let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

    let toml_content = r#"
ORGANIZATION_NAME = "Multi-Currency Anchor"

[[CURRENCIES]]
code = "USD"
issuer = "GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX"
display_decimals = 2
name = "US Dollar"
desc = "US Dollar stablecoin"
is_asset_anchored = true
anchor_asset_type = "fiat"
anchor_asset = "USD"
status = "live"

[[CURRENCIES]]
code = "EUR"
issuer = "GDTNXRLOJD2YEBPKK7KCMR7J33AAG5VZXHAJTHIG736D6LVEFLLLKPDL"
display_decimals = 2
name = "Euro"
desc = "Euro stablecoin"
is_asset_anchored = true
anchor_asset_type = "fiat"
anchor_asset = "EUR"
status = "live"

[[CURRENCIES]]
code = "BTC"
issuer = "GAUTUYY2THLF7SGITDFMXJVYH3LHDSMGEAKSBU267M2K7A3W543CKUEF"
display_decimals = 7
name = "Bitcoin"
desc = "Bitcoin token"
is_asset_anchored = true
anchor_asset_type = "crypto"
anchor_asset = "BTC"
status = "test"
    "#;

    let result = client.parse_toml(toml_content, "example.com");
    assert!(result.is_ok());

    let toml = result.unwrap();
    assert!(toml.currencies.is_some());

    let currencies = toml.currencies.unwrap();
    assert_eq!(currencies.len(), 3);

    // Check USD
    assert_eq!(currencies[0].code, "USD");
    assert_eq!(currencies[0].name, Some("US Dollar".to_string()));
    assert_eq!(currencies[0].display_decimals, Some(2));
    assert_eq!(currencies[0].is_asset_anchored, Some(true));
    assert_eq!(currencies[0].anchor_asset_type, Some("fiat".to_string()));
    assert_eq!(currencies[0].status, Some("live".to_string()));

    // Check EUR
    assert_eq!(currencies[1].code, "EUR");
    assert_eq!(currencies[1].name, Some("Euro".to_string()));

    // Check BTC
    assert_eq!(currencies[2].code, "BTC");
    assert_eq!(currencies[2].display_decimals, Some(7));
    assert_eq!(currencies[2].anchor_asset_type, Some("crypto".to_string()));
    assert_eq!(currencies[2].status, Some("test".to_string()));
}

#[tokio::test]
async fn test_parse_toml_with_principals() {
    let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

    let toml_content = r#"
ORGANIZATION_NAME = "Test Anchor"

[[PRINCIPALS]]
name = "Jane Doe"
email = "jane@example.com"
keybase = "janedoe"
twitter = "janedoe"
github = "janedoe"

[[PRINCIPALS]]
name = "John Smith"
email = "john@example.com"
keybase = "johnsmith"
    "#;

    let result = client.parse_toml(toml_content, "example.com");
    assert!(result.is_ok());

    let toml = result.unwrap();
    assert!(toml.principals.is_some());

    let principals = toml.principals.unwrap();
    assert_eq!(principals.len(), 2);

    assert_eq!(principals[0].name, Some("Jane Doe".to_string()));
    assert_eq!(principals[0].email, Some("jane@example.com".to_string()));
    assert_eq!(principals[0].keybase, Some("janedoe".to_string()));
    assert_eq!(principals[0].twitter, Some("janedoe".to_string()));
    assert_eq!(principals[0].github, Some("janedoe".to_string()));

    assert_eq!(principals[1].name, Some("John Smith".to_string()));
    assert_eq!(principals[1].email, Some("john@example.com".to_string()));
}

#[tokio::test]
async fn test_parse_toml_with_documentation() {
    let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

    let toml_content = r#"
ORGANIZATION_NAME = "Test Anchor"

[DOCUMENTATION]
ORG_NAME = "Test Organization"
ORG_DBA = "Test DBA"
ORG_URL = "https://test.org"
ORG_LOGO = "https://test.org/logo.png"
ORG_DESCRIPTION = "Test organization description"
    "#;

    let result = client.parse_toml(toml_content, "example.com");
    assert!(result.is_ok());

    let toml = result.unwrap();
    assert!(toml.documentation.is_some());

    let doc = toml.documentation.unwrap();
    assert_eq!(doc.org_name, Some("Test Organization".to_string()));
    assert_eq!(doc.org_dba, Some("Test DBA".to_string()));
    assert_eq!(doc.org_url, Some("https://test.org".to_string()));
    assert_eq!(doc.org_logo, Some("https://test.org/logo.png".to_string()));
    assert_eq!(
        doc.org_description,
        Some("Test organization description".to_string())
    );
}

#[tokio::test]
async fn test_parse_invalid_toml() {
    let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

    // Invalid TOML syntax
    let invalid_toml = r#"
ORGANIZATION_NAME = "Test
INVALID [[[
    "#;

    let result = client.parse_toml(invalid_toml, "example.com");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_parse_empty_toml() {
    let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

    let empty_toml = "";

    let result = client.parse_toml(empty_toml, "example.com");
    assert!(result.is_ok());

    let toml = result.unwrap();
    assert_eq!(toml.organization_name, None);
    assert_eq!(toml.currencies, None);
    assert_eq!(toml.principals, None);
}

#[tokio::test]
async fn test_network_passphrase_validation() {
    let expected_passphrase = "Public Global Stellar Network ; September 2015";
    let client = StellarTomlClient::new(
        Arc::new(RwLock::new(None)),
        Some(expected_passphrase.to_string()),
    )
    .unwrap();

    // Matching passphrase - should succeed with warning in logs
    let toml_with_matching = r#"
ORGANIZATION_NAME = "Test"
NETWORK_PASSPHRASE = "Public Global Stellar Network ; September 2015"
    "#;

    let result = client.parse_toml(toml_with_matching, "example.com");
    assert!(result.is_ok());

    // Mismatching passphrase - should succeed but log warning
    let toml_with_mismatch = r#"
ORGANIZATION_NAME = "Test"
NETWORK_PASSPHRASE = "Test SDF Network ; September 2015"
    "#;

    let result = client.parse_toml(toml_with_mismatch, "example.com");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_parse_currency_without_code() {
    let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

    let toml_content = r#"
ORGANIZATION_NAME = "Test"

[[CURRENCIES]]
issuer = "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
name = "Missing Code"
    "#;

    let result = client.parse_toml(toml_content, "example.com");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_parse_minimal_currency() {
    let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

    let toml_content = r#"
ORGANIZATION_NAME = "Test"

[[CURRENCIES]]
code = "XLM"
    "#;

    let result = client.parse_toml(toml_content, "example.com");
    assert!(result.is_ok());

    let toml = result.unwrap();
    assert!(toml.currencies.is_some());

    let currencies = toml.currencies.unwrap();
    assert_eq!(currencies.len(), 1);
    assert_eq!(currencies[0].code, "XLM");
    assert_eq!(currencies[0].issuer, None);
}

#[tokio::test]
async fn test_cache_key_format() {
    // Test that cache keys are properly formatted
    let domain = "example.com";
    let expected_key = format!("stellar_toml:{}", domain);
    assert_eq!(expected_key, "stellar_toml:example.com");
}

#[tokio::test]
async fn test_url_construction() {
    let domain = "stellar.org";
    let https_url = format!("https://{}/.well-known/stellar.toml", domain);
    let http_url = format!("http://{}/.well-known/stellar.toml", domain);

    assert_eq!(https_url, "https://stellar.org/.well-known/stellar.toml");
    assert_eq!(http_url, "http://stellar.org/.well-known/stellar.toml");
}

#[tokio::test]
async fn test_parse_all_organization_fields() {
    let client = StellarTomlClient::new(Arc::new(RwLock::new(None)), None).unwrap();

    let toml_content = r#"
ORGANIZATION_NAME = "Full Org"
ORGANIZATION_DBA = "Full DBA"
ORGANIZATION_URL = "https://full.org"
ORGANIZATION_LOGO = "https://full.org/logo.png"
ORGANIZATION_DESCRIPTION = "Full description"
ORGANIZATION_PHYSICAL_ADDRESS = "123 Main St, City, Country"
ORGANIZATION_PHONE_NUMBER = "+1-555-0123"
ORGANIZATION_KEYBASE = "fullorg"
ORGANIZATION_TWITTER = "fullorg"
ORGANIZATION_GITHUB = "fullorg"
ORGANIZATION_OFFICIAL_EMAIL = "info@full.org"
ORGANIZATION_SUPPORT_EMAIL = "support@full.org"
    "#;

    let result = client.parse_toml(toml_content, "full.org");
    assert!(result.is_ok());

    let toml = result.unwrap();
    assert_eq!(toml.organization_name, Some("Full Org".to_string()));
    assert_eq!(toml.organization_dba, Some("Full DBA".to_string()));
    assert_eq!(toml.organization_url, Some("https://full.org".to_string()));
    assert_eq!(
        toml.organization_logo,
        Some("https://full.org/logo.png".to_string())
    );
    assert_eq!(
        toml.organization_description,
        Some("Full description".to_string())
    );
    assert_eq!(
        toml.organization_physical_address,
        Some("123 Main St, City, Country".to_string())
    );
    assert_eq!(
        toml.organization_phone_number,
        Some("+1-555-0123".to_string())
    );
    assert_eq!(toml.organization_keybase, Some("fullorg".to_string()));
    assert_eq!(toml.organization_twitter, Some("fullorg".to_string()));
    assert_eq!(toml.organization_github, Some("fullorg".to_string()));
    assert_eq!(
        toml.organization_official_email,
        Some("info@full.org".to_string())
    );
    assert_eq!(
        toml.organization_support_email,
        Some("support@full.org".to_string())
    );
}
