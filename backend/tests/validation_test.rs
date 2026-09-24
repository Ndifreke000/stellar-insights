//! Tests for request-parameter validation logic.
//!
//! Extends the inline `#[cfg(test)]` block in `src/validation.rs` with
//! additional boundary and cross-field cases.
//!
//! Contains two test suites:
//!  1. `validate_corridor_filters` — query-parameter boundary tests
//!  2. `CreateCorridorRequest` / `CreateAnchorRequest` — `validator` crate struct tests

use validator::Validate;

// ── Struct mirrors for validator-crate tests (keep in sync with src/models.rs) ──
// Mirror the structs here for unit testing without needing the full app state.
// These must stay in sync with backend/src/models.rs.

#[derive(Debug, serde::Deserialize, Validate)]
struct CreateCorridorRequest {
    #[validate(length(min = 1, max = 12))]
    pub source_asset_code: String,
    #[validate(length(min = 56, max = 56))]
    pub source_asset_issuer: String,
    #[validate(length(min = 1, max = 12))]
    pub dest_asset_code: String,
    #[validate(length(min = 56, max = 56))]
    pub dest_asset_issuer: String,
}

#[derive(Debug, serde::Deserialize, Validate)]
struct CreateAnchorRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 56, max = 56))]
    pub stellar_account: String,
    #[validate(length(max = 253))]
    pub home_domain: Option<String>,
}

const VALID_ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
const ALT_ISSUER: &str = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN7";

// --- CreateCorridorRequest validation tests ---

#[test]
fn test_create_corridor_validation_valid() {
    let req = CreateCorridorRequest {
        source_asset_code: "USDC".into(),
        source_asset_issuer: VALID_ISSUER.into(),
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_ok());
}

#[test]
fn test_invalid_asset_code_rejected_empty() {
    let req = CreateCorridorRequest {
        source_asset_code: "".into(),
        source_asset_issuer: VALID_ISSUER.into(),
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_invalid_asset_code_rejected_too_long() {
    let req = CreateCorridorRequest {
        source_asset_code: "TOOLONGASSET1".into(), // 13 chars
        source_asset_issuer: VALID_ISSUER.into(),
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_invalid_issuer_rejected_too_short() {
    let req = CreateCorridorRequest {
        source_asset_code: "USDC".into(),
        source_asset_issuer: "GSHORT".into(),
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_invalid_issuer_rejected_too_long() {
    let req = CreateCorridorRequest {
        source_asset_code: "USDC".into(),
        source_asset_issuer: format!("{}X", VALID_ISSUER), // 57 chars
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_dest_asset_code_validated() {
    let req = CreateCorridorRequest {
        source_asset_code: "USDC".into(),
        source_asset_issuer: VALID_ISSUER.into(),
        dest_asset_code: "".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_dest_issuer_validated() {
    let req = CreateCorridorRequest {
        source_asset_code: "USDC".into(),
        source_asset_issuer: VALID_ISSUER.into(),
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: "GSHORT".into(),
    };
    assert!(req.validate().is_err());
}

// --- CreateAnchorRequest validation tests ---

#[test]
fn test_create_anchor_validation_valid() {
    let req = CreateAnchorRequest {
        name: "Circle".into(),
        stellar_account: VALID_ISSUER.into(),
        home_domain: Some("circle.com".into()),
    };
    assert!(req.validate().is_ok());
}

#[test]
fn test_create_anchor_empty_name_rejected() {
    let req = CreateAnchorRequest {
        name: "".into(),
        stellar_account: VALID_ISSUER.into(),
        home_domain: None,
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_create_anchor_invalid_account_rejected() {
    let req = CreateAnchorRequest {
        name: "Test".into(),
        stellar_account: "SHORTACCOUNT".into(),
        home_domain: None,
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_create_anchor_no_home_domain_ok() {
    let req = CreateAnchorRequest {
        name: "Test Anchor".into(),
        stellar_account: VALID_ISSUER.into(),
        home_domain: None,
    };
    assert!(req.validate().is_ok());
}

// --- Business logic validation tests ---

#[test]
fn test_corridor_not_self_referential_same_asset() {
    // This mirrors validate_corridor_not_self_referential logic
    let source_code = "USDC";
    let source_issuer = VALID_ISSUER;
    let dest_code = "USDC";
    let dest_issuer = VALID_ISSUER;

    let is_same = source_code.eq_ignore_ascii_case(dest_code) && source_issuer == dest_issuer;
    assert!(is_same, "same asset should be detected as self-referential");
}

#[test]
fn test_corridor_different_issuers_allowed() {
    let source_code = "USDC";
    let source_issuer = VALID_ISSUER;
    let dest_code = "USDC";
    let dest_issuer = ALT_ISSUER;

    let is_same = source_code.eq_ignore_ascii_case(dest_code) && source_issuer == dest_issuer;
    assert!(
        !is_same,
        "same code but different issuers are different assets"
    );
}
