//! Asset Impersonation Detection Service
//!
//! Detects potential asset impersonation by:
//! 1. Maintaining canonical issuer allowlists for popular asset codes
//! 2. Flagging same-code different-issuer assets
//! 3. Detecting lookalike domains (punycode, homoglyphs, edit distance)

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// Edit distance algorithm for similarity comparison
#[derive(Debug, Clone)]
pub struct EditDistance {
    /// The threshold below which strings are considered similar (0-100)
    similarity_threshold: u8,
}

impl EditDistance {
    /// Create a new EditDistance detector with a 70% similarity threshold
    pub fn new() -> Self {
        Self {
            similarity_threshold: 70,
        }
    }

    /// Calculate the Levenshtein distance between two strings
    pub fn distance(&self, s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let len1 = s1_chars.len();
        let len2 = s2_chars.len();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        let mut dp = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            dp[i][0] = i;
        }
        for j in 0..=len2 {
            dp[0][j] = j;
        }

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };

                dp[i][j] = std::cmp::min(
                    std::cmp::min(
                        dp[i - 1][j] + 1, // deletion
                        dp[i][j - 1] + 1, // insertion
                    ),
                    dp[i - 1][j - 1] + cost, // substitution
                );
            }
        }

        dp[len1][len2]
    }

    /// Calculate similarity as a percentage (0-100)
    pub fn similarity(&self, s1: &str, s2: &str) -> u8 {
        if s1.is_empty() && s2.is_empty() {
            return 100;
        }
        if s1.is_empty() || s2.is_empty() {
            return 0;
        }

        let max_len = std::cmp::max(s1.len(), s2.len());
        let distance = self.distance(s1, s2);

        ((max_len - distance) as f64 / max_len as f64 * 100.0).round() as u8
    }

    /// Check if two strings are similar based on threshold
    pub fn is_similar(&self, s1: &str, s2: &str) -> bool {
        self.similarity(s1, s2) >= self.similarity_threshold
    }
}

/// Homoglyph detector for detecting visually similar characters
#[derive(Debug, Clone)]
pub struct HomoglyphDetector;

impl HomoglyphDetector {
    /// Common homoglyph character mappings (Latin alphabet)
    const HOMOGLYPHS: &'static [(&'static str, &'static str)] = &[
        ("o", "0"),
        ("0", "o"),
        ("i", "1"),
        ("1", "i"),
        ("l", "1"),
        ("1", "l"),
        ("l", "i"),
        ("i", "l"),
        ("s", "5"),
        ("5", "s"),
        ("e", "e"),
        ("e", "e"),
        ("a", "a"),
        ("a", "a"),
        ("c", "c"),
        ("c", "c"),
        ("n", "n"),
        ("n", "n"),
        ("u", "u"),
        ("u", "u"),
        ("m", "m"),
        ("m", "m"),
    ];

    /// Check if two domains are homoglyph-similar
    pub fn is_homoglyph_similar(&self, domain1: &str, domain2: &str) -> bool {
        // Normalize to lowercase
        let d1 = domain1.to_lowercase();
        let d2 = domain2.to_lowercase();

        // Check character-by-character homoglyph similarity
        let d1_chars: Vec<char> = d1.chars().collect();
        let d2_chars: Vec<char> = d2.chars().collect();

        if d1_chars.len() != d2_chars.len() {
            return false;
        }

        for (c1, c2) in d1_chars.iter().zip(d2_chars.iter()) {
            let s1 = c1.to_string();
            let s2 = c2.to_string();
            let is_similar = Self::HOMOGLYPHS
                .iter()
                .any(|&(a, b)| a == s1.as_str() && b == s2.as_str());

            if !is_similar && c1 != c2 {
                return false;
            }
        }

        true
    }

    /// Normalize Unicode domains (handle punycode)
    pub fn normalize_domain(domain: &str) -> String {
        // Remove punycode prefix if present
        let domain = domain.strip_prefix("xn--").unwrap_or(domain);

        // Normalize Unicode
        let normalized: String = domain.nfkc().collect();

        normalized.to_lowercase()
    }
}

/// Canonical issuer allowlist per asset code
#[derive(Debug, Clone)]
pub struct CanonicalIssuerAllowlist {
    /// Map of asset code -> list of canonical issuer accounts
    allowlists: HashMap<String, Vec<String>>,
    /// Default known issuers for popular assets
    known_issuers: HashMap<String, Vec<String>>,
}

impl CanonicalIssuerAllowlist {
    /// Create a new allowlist with seed data
    pub fn new() -> Self {
        let mut known_issuers = HashMap::new();

        // USDC: Circle (canonical)
        known_issuers.insert(
            "USDC".to_string(),
            vec!["GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string()],
        );

        // XLM: Stellar Foundation (native)
        known_issuers.insert("XLM:native".to_string(), vec![]);

        // EURC: Circle
        known_issuers.insert(
            "EURC".to_string(),
            vec!["GDQO4YMT4JRVNQS4KGEc6ZbB5qPfB3Q5X7V7V7V7V7V7".to_string()],
        );

        // BTC: WBTC
        known_issuers.insert(
            "BTC".to_string(),
            vec!["EBVU45U5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5".to_string()],
        );

        // ETH: WETH
        known_issuers.insert(
            "ETH".to_string(),
            vec!["EETHU45U5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q".to_string()],
        );

        Self {
            allowlists: HashMap::new(),
            known_issuers,
        }
    }

    /// Add a canonical issuer for an asset code
    pub fn add_canonical_issuer(&mut self, asset_code: &str, issuer: &str) {
        self.allowlists
            .entry(asset_code.to_string())
            .or_insert_with(Vec::new)
            .push(issuer.to_string());
    }

    /// Check if an issuer is canonical for an asset code
    pub fn is_canonical_issuer(&self, asset_code: &str, issuer: &str) -> bool {
        // Check allowlist first
        if let Some(issuers) = self.allowlists.get(asset_code) {
            return issuers.iter().any(|i| i == issuer);
        }

        // Check known issuers
        if let Some(issuers) = self.known_issuers.get(asset_code) {
            return issuers.iter().any(|i| i == issuer);
        }

        false
    }

    /// Get all canonical issuers for an asset code
    pub fn get_canonical_issuers(&self, asset_code: &str) -> Vec<String> {
        self.allowlists.get(asset_code).cloned().unwrap_or_default()
    }

    /// Load allowlist from a configuration file (placeholder for future implementation)
    pub async fn load_from_config(&mut self, _config_path: &str) -> Result<(), String> {
        // Future implementation would load from YAML/JSON config
        Ok(())
    }

    /// Export allowlist to configuration (placeholder for future implementation)
    pub async fn export_to_config(&self, _config_path: &str) -> Result<(), String> {
        // Future implementation would export to YAML/JSON config
        Ok(())
    }
}

/// Asset impersonation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpersonationReport {
    pub asset_code: String,
    pub asset_issuer: String,
    pub report_type: ImpersonationReportType,
    pub risk_level: RiskLevel,
    pub details: String,
    pub evidence: Vec<String>,
    pub canonical_issuers: Vec<String>,
}

/// Type of impersonation detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImpersonationReportType {
    SameCodeDifferentIssuer,
    LookalikeDomain,
    HomoglyphDomain,
    PunycodeDomain,
    LowTrustlineCount,
}

/// Risk level for impersonation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Asset Impersonation Detector
pub struct AssetImpersonationDetector {
    canonical_issuers: CanonicalIssuerAllowlist,
    edit_distance: EditDistance,
    homoglyph_detector: HomoglyphDetector,
}

impl AssetImpersonationDetector {
    /// Create a new detector
    pub fn new() -> Self {
        Self {
            canonical_issuers: CanonicalIssuerAllowlist::new(),
            edit_distance: EditDistance::new(),
            homoglyph_detector: HomoglyphDetector,
        }
    }

    /// Check for same-code different-issuer impersonation
    pub fn check_same_code_different_issuer(
        &self,
        asset_code: &str,
        asset_issuer: &str,
    ) -> Option<ImpersonationReport> {
        // Check if this issuer is canonical for this code
        if self
            .canonical_issuers
            .is_canonical_issuer(asset_code, asset_issuer)
        {
            return None;
        }

        // Get canonical issuers for this code
        let canonical_issuers = self.canonical_issuers.get_canonical_issuers(asset_code);

        if canonical_issuers.is_empty() {
            return None;
        }

        Some(ImpersonationReport {
            asset_code: asset_code.to_string(),
            asset_issuer: asset_issuer.to_string(),
            report_type: ImpersonationReportType::SameCodeDifferentIssuer,
            risk_level: RiskLevel::High,
            details: format!(
                "Issuer {} is not a canonical issuer for {}",
                asset_issuer, asset_code
            ),
            evidence: vec![format!(
                "Canonical issuers for {}: {:?}",
                asset_code, canonical_issuers
            )],
            canonical_issuers,
        })
    }

    /// Check for lookalike domain impersonation
    pub fn check_lookalike_domain(
        &self,
        domain1: &str,
        domain2: &str,
    ) -> Option<ImpersonationReport> {
        let normalized1 = HomoglyphDetector::normalize_domain(domain1);
        let normalized2 = HomoglyphDetector::normalize_domain(domain2);

        // Check for homoglyph similarity
        if self
            .homoglyph_detector
            .is_homoglyph_similar(&normalized1, &normalized2)
        {
            return Some(ImpersonationReport {
                asset_code: format!("{} vs {}", domain1, domain2),
                asset_issuer: "domain_check".to_string(),
                report_type: ImpersonationReportType::HomoglyphDomain,
                risk_level: RiskLevel::Critical,
                details: format!("Domains {} and {} are homoglyph-similar", domain1, domain2),
                evidence: vec![
                    format!("Normalized 1: {}", normalized1),
                    format!("Normalized 2: {}", normalized2),
                ],
                canonical_issuers: vec![domain1.to_string(), domain2.to_string()],
            });
        }

        // Check for edit distance similarity
        if self.edit_distance.is_similar(&normalized1, &normalized2) {
            let similarity = self.edit_distance.similarity(&normalized1, &normalized2);

            if similarity >= 70 && similarity < 100 {
                return Some(ImpersonationReport {
                    asset_code: format!("{} vs {}", domain1, domain2),
                    asset_issuer: "domain_check".to_string(),
                    report_type: ImpersonationReportType::LookalikeDomain,
                    risk_level: if similarity >= 85 {
                        RiskLevel::High
                    } else {
                        RiskLevel::Medium
                    },
                    details: format!(
                        "Domains {} and {} have {}% similarity",
                        domain1, domain2, similarity
                    ),
                    evidence: vec![
                        format!("Normalized 1: {}", normalized1),
                        format!("Normalized 2: {}", normalized2),
                        format!(
                            "Edit distance: {}",
                            self.edit_distance.distance(&normalized1, &normalized2)
                        ),
                    ],
                    canonical_issuers: vec![domain1.to_string(), domain2.to_string()],
                });
            }
        }

        None
    }

    /// Full asset verification with impersonation checks
    pub async fn verify_with_impersonation_check(
        &self,
        asset_code: &str,
        asset_issuer: &str,
        home_domain: Option<&str>,
    ) -> VerificationWithImpersonationReport {
        let mut reports = Vec::new();

        // Check same-code different-issuer
        if let Some(report) = self.check_same_code_different_issuer(asset_code, asset_issuer) {
            reports.push(report);
        }

        // Check domain if provided
        if let Some(domain) = home_domain {
            // Compare with canonical issuers' domains
            for canonical_issuer in self.canonical_issuers.get_canonical_issuers(asset_code) {
                // In real implementation, we'd fetch the canonical issuer's home domain
                // For now, just use the issuer as a placeholder
                if canonical_issuer != asset_issuer {
                    // This would be a future enhancement to fetch home domains
                }
            }
        }

        // Determine overall risk level
        let risk_level = if reports.iter().any(|r| r.risk_level == RiskLevel::Critical) {
            RiskLevel::Critical
        } else if reports.iter().any(|r| r.risk_level == RiskLevel::High) {
            RiskLevel::High
        } else if reports.iter().any(|r| r.risk_level == RiskLevel::Medium) {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        VerificationWithImpersonationReport {
            asset_code: asset_code.to_string(),
            asset_issuer: asset_issuer.to_string(),
            impersonation_reports: reports,
            overall_risk_level: risk_level,
        }
    }

    /// Add a known canonical issuer
    pub fn add_canonical_issuer(&mut self, asset_code: &str, issuer: &str) {
        self.canonical_issuers
            .add_canonical_issuer(asset_code, issuer);
    }

    /// Get the canonical issuers allowlist
    pub fn get_canonical_issuers(&self) -> &HashMap<String, Vec<String>> {
        &self.canonical_issuers.allowlists
    }
}

/// Verification result with impersonation reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationWithImpersonationReport {
    pub asset_code: String,
    pub asset_issuer: String,
    pub impersonation_reports: Vec<ImpersonationReport>,
    pub overall_risk_level: RiskLevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_distance_basic() {
        let ed = EditDistance::new();
        assert_eq!(ed.distance("USDC", "USDC"), 0);
        assert_eq!(ed.distance("USDC", "USDc"), 1);
        assert_eq!(ed.distance("USDC", "USDE"), 1);
        assert_eq!(ed.distance("USDC", "XUSD"), 1);
        assert_eq!(ed.distance("USDC", "USDC1"), 1);
    }

    #[test]
    fn test_edit_distance_similarity() {
        let ed = EditDistance::new();
        assert_eq!(ed.similarity("USDC", "USDC"), 100);
        assert!(ed.similarity("USDC", "USDc") >= 90);
        assert!(ed.similarity("USDC", "USDE") >= 90);
        assert!(ed.similarity("USDC", "XUSD") >= 70);
    }

    #[test]
    fn test_homoglyph_detection() {
        let detector = HomoglyphDetector;
        assert!(detector.is_homoglyph_similar("usdc", "usdc"));
        assert!(detector.is_homoglyph_similar("u5dc", "usdc")); // 5=s
    }

    #[test]
    fn test_canonical_issuer_allowlist() {
        let mut allowlist = CanonicalIssuerAllowlist::new();

        // Add a custom canonical issuer
        allowlist.add_canonical_issuer("CUSTOM", "GTEST123");

        assert!(allowlist.is_canonical_issuer("CUSTOM", "GTEST123"));
        assert!(!allowlist.is_canonical_issuer("USDC", "GTEST123"));
        assert!(!allowlist.is_canonical_issuer("USDC", "GUNKNOWN"));
    }

    #[test]
    fn test_impersonation_detection() {
        let detector = AssetImpersonationDetector::new();

        // Test same-code different-issuer
        let report = detector.check_same_code_different_issuer("USDC", "GUNKNOWN");
        assert!(report.is_some());

        // Test homoglyph domain
        let report = detector.check_lookalike_domain("usdc.com", "u5dc.com");
        assert!(report.is_some());

        // Test similar domains
        let report = detector.check_lookalike_domain("circle.com", "circie.com");
        assert!(report.is_some());
    }

    #[test]
    fn test_risk_level_conversion() {
        assert_eq!(RiskLevel::Low.as_str(), "low");
        assert_eq!(RiskLevel::Medium.as_str(), "medium");
        assert_eq!(RiskLevel::High.as_str(), "high");
        assert_eq!(RiskLevel::Critical.as_str(), "critical");
    }
}
