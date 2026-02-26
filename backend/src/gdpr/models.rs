// GDPR Models - Data structures for GDPR compliance

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Consent types that can be tracked
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConsentType {
    #[serde(rename = "terms_of_service")]
    TermsOfService,
    #[serde(rename = "privacy_policy")]
    PrivacyPolicy,
    #[serde(rename = "marketing_emails")]
    MarketingEmails,
    #[serde(rename = "analytics")]
    Analytics,
    #[serde(rename = "personalization")]
    Personalization,
    #[serde(rename = "data_sharing")]
    DataSharing,
    #[serde(rename = "cookies")]
    Cookies,
}

impl ConsentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConsentType::TermsOfService => "terms_of_service",
            ConsentType::PrivacyPolicy => "privacy_policy",
            ConsentType::MarketingEmails => "marketing_emails",
            ConsentType::Analytics => "analytics",
            ConsentType::Personalization => "personalization",
            ConsentType::DataSharing => "data_sharing",
            ConsentType::Cookies => "cookies",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "terms_of_service" => Some(ConsentType::TermsOfService),
            "privacy_policy" => Some(ConsentType::PrivacyPolicy),
            "marketing_emails" => Some(ConsentType::MarketingEmails),
            "analytics" => Some(ConsentType::Analytics),
            "personalization" => Some(ConsentType::Personalization),
            "data_sharing" => Some(ConsentType::DataSharing),
            "cookies" => Some(ConsentType::Cookies),
            _ => None,
        }
    }

    pub fn all() -> Vec<&'static str> {
        vec![
            "terms_of_service",
            "privacy_policy",
            "marketing_emails",
            "analytics",
            "personalization",
            "data_sharing",
            "cookies",
        ]
    }
}

// User consent record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserConsent {
    pub id: String,
    pub user_id: String,
    pub consent_type: String,
    pub consent_given: bool,
    pub consent_version: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub granted_at: Option<String>,
    pub revoked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// Request to update consent
#[derive(Debug, Deserialize)]
pub struct UpdateConsentRequest {
    pub consent_type: String,
    pub consent_given: bool,
    pub consent_version: Option<String>,
}

// Batch consent update request
#[derive(Debug, Deserialize)]
pub struct BatchUpdateConsentsRequest {
    pub consents: Vec<UpdateConsentRequest>,
}

// Consent response with metadata
#[derive(Debug, Serialize)]
pub struct ConsentResponse {
    pub consent_type: String,
    pub consent_given: bool,
    pub consent_version: String,
    pub granted_at: Option<String>,
    pub revoked_at: Option<String>,
}

// Export request status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    Pending,
    Processing,
    Completed,
    Expired,
    Failed,
}

impl ExportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportStatus::Pending => "pending",
            ExportStatus::Processing => "processing",
            ExportStatus::Completed => "completed",
            ExportStatus::Expired => "expired",
            ExportStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "processing" => ExportStatus::Processing,
            "completed" => ExportStatus::Completed,
            "expired" => ExportStatus::Expired,
            "failed" => ExportStatus::Failed,
            _ => ExportStatus::Pending,
        }
    }
}

// Data export request
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataExportRequest {
    pub id: String,
    pub user_id: String,
    pub status: String,
    pub requested_data_types: String,
    pub export_format: String,
    pub requested_at: String,
    pub completed_at: Option<String>,
    pub expires_at: Option<String>,
    pub download_token: Option<String>,
    pub file_path: Option<String>,
    pub error_message: Option<String>,
}

// Request to export user data
#[derive(Debug, Deserialize)]
pub struct CreateExportRequest {
    pub data_types: Vec<String>,
    pub export_format: Option<String>,
}

// Data export response
#[derive(Debug, Serialize)]
pub struct ExportRequestResponse {
    pub id: String,
    pub status: String,
    pub requested_at: String,
    pub expires_at: Option<String>,
    pub download_url: Option<String>,
}

// Deletion request status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeletionStatus {
    Pending,
    Scheduled,
    Processing,
    Completed,
    Cancelled,
    Failed,
}

impl DeletionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeletionStatus::Pending => "pending",
            DeletionStatus::Scheduled => "scheduled",
            DeletionStatus::Processing => "processing",
            DeletionStatus::Completed => "completed",
            DeletionStatus::Cancelled => "cancelled",
            DeletionStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "scheduled" => DeletionStatus::Scheduled,
            "processing" => DeletionStatus::Processing,
            "completed" => DeletionStatus::Completed,
            "cancelled" => DeletionStatus::Cancelled,
            "failed" => DeletionStatus::Failed,
            _ => DeletionStatus::Pending,
        }
    }
}

// Data deletion request
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataDeletionRequest {
    pub id: String,
    pub user_id: String,
    pub status: String,
    pub reason: Option<String>,
    pub delete_all_data: bool,
    pub data_types_to_delete: Option<String>,
    pub requested_at: String,
    pub scheduled_deletion_at: Option<String>,
    pub completed_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub error_message: Option<String>,
    pub confirmation_token: Option<String>,
}

// Request to delete user data
#[derive(Debug, Deserialize)]
pub struct CreateDeletionRequest {
    pub reason: Option<String>,
    pub delete_all_data: Option<bool>,
    pub data_types: Option<Vec<String>>,
}

// Deletion request response
#[derive(Debug, Serialize)]
pub struct DeletionRequestResponse {
    pub id: String,
    pub status: String,
    pub requested_at: String,
    pub scheduled_deletion_at: Option<String>,
    pub confirmation_required: bool,
    pub confirmation_token: Option<String>,
}

// Confirm deletion request
#[derive(Debug, Deserialize)]
pub struct ConfirmDeletionRequest {
    pub confirmation_token: String,
}

// Consent audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConsentAuditLog {
    pub id: String,
    pub user_id: String,
    pub consent_type: String,
    pub action: String,
    pub old_value: Option<bool>,
    pub new_value: Option<bool>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

// Data processing log entry
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataProcessingLog {
    pub id: String,
    pub user_id: String,
    pub activity_type: String,
    pub data_category: String,
    pub purpose: Option<String>,
    pub legal_basis: Option<String>,
    pub processed_at: String,
}

// GDPR summary for a user
#[derive(Debug, Serialize)]
pub struct GdprSummary {
    pub user_id: String,
    pub consents: Vec<ConsentResponse>,
    pub pending_export_requests: i32,
    pub pending_deletion_requests: i32,
    pub data_processing_activities_count: i32,
}

// Exportable data types
#[derive(Debug, Clone, Serialize)]
pub struct ExportableDataTypes {
    pub types: Vec<DataTypeInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataTypeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
}
