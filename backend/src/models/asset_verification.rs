use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VerificationStatus {
    Verified,
    Unverified,
    Suspicious,
}

impl VerificationStatus {
    pub fn as_str(&self) -> &str {
        match self {
            VerificationStatus::Verified => "verified",
            VerificationStatus::Unverified => "unverified",
            VerificationStatus::Suspicious => "suspicious",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "verified" => VerificationStatus::Verified,
            "suspicious" => VerificationStatus::Suspicious,
            _ => VerificationStatus::Unverified,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VerifiedAsset {
    pub id: String,
    pub asset_code: String,
    pub asset_issuer: String,
    pub verification_status: String,
    pub reputation_score: f64,

    // Verification sources
    pub stellar_expert_verified: bool,
    pub stellar_toml_verified: bool,
    pub anchor_registry_verified: bool,

    // Metrics
    pub trustline_count: i64,
    pub transaction_count: i64,
    pub total_volume_usd: f64,

    // TOML data
    pub toml_home_domain: Option<String>,
    pub toml_name: Option<String>,
    pub toml_description: Option<String>,
    pub toml_org_name: Option<String>,
    pub toml_org_url: Option<String>,
    pub toml_logo_url: Option<String>,

    // Community reports
    pub suspicious_reports_count: i64,
    pub last_suspicious_report_at: Option<DateTime<Utc>>,

    // Verification metadata
    pub last_verified_at: Option<DateTime<Utc>>,
    pub verification_notes: Option<String>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl VerifiedAsset {
    pub fn get_status(&self) -> VerificationStatus {
        VerificationStatus::from_str(&self.verification_status)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedAssetResponse {
    pub asset_code: String,
    pub asset_issuer: String,
    pub verification_status: VerificationStatus,
    pub reputation_score: f64,
    pub trust_indicators: TrustIndicators,
    pub toml_info: Option<TomlInfo>,
    pub metrics: AssetMetrics,
    pub last_verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustIndicators {
    pub stellar_expert_verified: bool,
    pub stellar_toml_verified: bool,
    pub anchor_registry_verified: bool,
    pub has_suspicious_reports: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlInfo {
    pub home_domain: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub org_name: Option<String>,
    pub org_url: Option<String>,
    pub logo_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetrics {
    pub trustline_count: i64,
    pub transaction_count: i64,
    pub total_volume_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReportType {
    Suspicious,
    Scam,
    Impersonation,
    Other,
}

impl ReportType {
    pub fn as_str(&self) -> &str {
        match self {
            ReportType::Suspicious => "suspicious",
            ReportType::Scam => "scam",
            ReportType::Impersonation => "impersonation",
            ReportType::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReportStatus {
    Pending,
    Reviewed,
    Resolved,
    Dismissed,
}

impl ReportStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ReportStatus::Pending => "pending",
            ReportStatus::Reviewed => "reviewed",
            ReportStatus::Resolved => "resolved",
            ReportStatus::Dismissed => "dismissed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AssetVerificationReport {
    pub id: String,
    pub asset_code: String,
    pub asset_issuer: String,
    pub reporter_account: Option<String>,
    pub report_type: String,
    pub description: String,
    pub evidence_url: Option<String>,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AssetVerificationHistory {
    pub id: String,
    pub asset_code: String,
    pub asset_issuer: String,
    pub previous_status: Option<String>,
    pub new_status: String,
    pub previous_reputation_score: Option<f64>,
    pub new_reputation_score: f64,
    pub change_reason: Option<String>,
    pub changed_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Request/Response DTOs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyAssetRequest {
    pub asset_code: String,
    pub asset_issuer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportAssetRequest {
    pub asset_code: String,
    pub asset_issuer: String,
    pub report_type: ReportType,
    pub description: String,
    pub evidence_url: Option<String>,
    pub reporter_account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVerifiedAssetsQuery {
    pub status: Option<VerificationStatus>,
    pub min_reputation: Option<f64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// Verification result from external sources
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub stellar_expert_verified: bool,
    pub stellar_toml_verified: bool,
    pub stellar_toml_data: Option<StellarTomlData>,
    pub anchor_registry_verified: bool,
    pub trustline_count: i64,
    pub transaction_count: i64,
    pub total_volume_usd: f64,
}

#[derive(Debug, Clone)]
pub struct StellarTomlData {
    pub home_domain: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub org_name: Option<String>,
    pub org_url: Option<String>,
    pub logo_url: Option<String>,
}

impl From<VerifiedAsset> for VerifiedAssetResponse {
    fn from(asset: VerifiedAsset) -> Self {
        VerifiedAssetResponse {
            asset_code: asset.asset_code.clone(),
            asset_issuer: asset.asset_issuer.clone(),
            verification_status: asset.get_status(),
            reputation_score: asset.reputation_score,
            trust_indicators: TrustIndicators {
                stellar_expert_verified: asset.stellar_expert_verified,
                stellar_toml_verified: asset.stellar_toml_verified,
                anchor_registry_verified: asset.anchor_registry_verified,
                has_suspicious_reports: asset.suspicious_reports_count > 0,
            },
            toml_info: asset.toml_home_domain.map(|home_domain| TomlInfo {
                home_domain,
                name: asset.toml_name,
                description: asset.toml_description,
                org_name: asset.toml_org_name,
                org_url: asset.toml_org_url,
                logo_url: asset.toml_logo_url,
            }),
            metrics: AssetMetrics {
                trustline_count: asset.trustline_count,
                transaction_count: asset.transaction_count,
                total_volume_usd: asset.total_volume_usd,
            },
            last_verified_at: asset.last_verified_at,
        }
    }
}
