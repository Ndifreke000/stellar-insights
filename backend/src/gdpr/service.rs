// GDPR Service - Business logic for GDPR compliance

use crate::error::ApiError;
use crate::gdpr::models::*;
use chrono::{Duration, Utc};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use uuid::Uuid;

/// GDPR Service for handling data export, deletion, and consent management
pub struct GdprService {
    db: Pool<Sqlite>,
}

impl GdprService {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    /// Get all consents for a user
    pub async fn get_user_consents(&self, user_id: &str) -> Result<Vec<ConsentResponse>, AppError> {
        let consents = sqlx::query_as::<_, UserConsent>(
            "SELECT * FROM user_consents WHERE user_id = ? ORDER BY consent_type",
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await
        .map_err(AppError::Database)?;

        let mut responses = Vec::new();
        for consent in consents {
            responses.push(ConsentResponse {
                consent_type: consent.consent_type,
                consent_given: consent.consent_given,
                consent_version: consent.consent_version,
                granted_at: consent.granted_at,
                revoked_at: consent.revoked_at,
            });
        }

        // Include all consent types even if not set (default false)
        let existing_types: Vec<&str> = responses.iter().map(|c| c.consent_type.as_str()).collect();
        for consent_type in ConsentType::all() {
            if !existing_types.contains(&consent_type) {
                responses.push(ConsentResponse {
                    consent_type: consent_type.to_string(),
                    consent_given: false,
                    consent_version: "1.0".to_string(),
                    granted_at: None,
                    revoked_at: None,
                });
            }
        }

        Ok(responses)
    }

    /// Update a single consent for a user
    pub async fn update_consent(
        &self,
        user_id: &str,
        request: UpdateConsentRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<ConsentResponse, AppError> {
        let consent_type = request.consent_type.clone();
        let old_consent_given = sqlx::query_as::<_, UserConsent>(
            "SELECT * FROM user_consents WHERE user_id = ? AND consent_type = ?",
        )
        .bind(user_id)
        .bind(&request.consent_type)
        .fetch_optional(&self.db)
        .await
        .map_err(AppError::Database)?
        .map(|c| c.consent_given);

        let now = Utc::now().to_rfc3339();
        let consent_version = request.consent_version.unwrap_or_else(|| "1.0".to_string());

        // Upsert the consent
        sqlx::query(
            "INSERT INTO user_consents (id, user_id, consent_type, consent_given, consent_version, ip_address, user_agent, granted_at, revoked_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id, consent_type) DO UPDATE SET
                consent_given = excluded.consent_given,
                consent_version = excluded.consent_version,
                granted_at = CASE WHEN excluded.consent_given = 1 AND user_consents.consent_given = 0 THEN ? ELSE user_consents.granted_at END,
                revoked_at = CASE WHEN excluded.consent_given = 0 AND user_consents.consent_given = 1 THEN ? ELSE user_consents.revoked_at END,
                updated_at = ?"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(&request.consent_type)
        .bind(request.consent_given)
        .bind(&consent_version)
        .bind(&ip_address)
        .bind(&user_agent)
        .bind(if request.consent_given { Some(now.clone()) } else { None })
        .bind(if !request.consent_given { Some(now.clone()) } else { None })
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .execute(&self.db)
        .await
        .map_err(AppError::Database)?;

        // Log the consent change in audit log
        sqlx::query(
            "INSERT INTO consent_audit_log (id, user_id, consent_type, action, old_value, new_value, ip_address, user_agent, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(&consent_type)
        .bind(if request.consent_given { "granted" } else { "revoked" })
        .bind(old_consent_given)
        .bind(Some(request.consent_given))
        .bind(&ip_address)
        .bind(&user_agent)
        .bind(&now)
        .execute(&self.db)
        .await
        .map_err(AppError::Database)?;

        Ok(ConsentResponse {
            consent_type,
            consent_given: request.consent_given,
            consent_version,
            granted_at: if request.consent_given {
                Some(now.clone())
            } else {
                None
            },
            revoked_at: if !request.consent_given {
                Some(now)
            } else {
                None
            },
        })
    }

    /// Batch update consents
    pub async fn batch_update_consents(
        &self,
        user_id: &str,
        requests: Vec<UpdateConsentRequest>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<Vec<ConsentResponse>, AppError> {
        let mut responses = Vec::new();
        for request in requests {
            let response = self
                .update_consent(user_id, request, ip_address.clone(), user_agent.clone())
                .await?;
            responses.push(response);
        }
        Ok(responses)
    }

    /// Create a data export request
    pub async fn create_export_request(
        &self,
        user_id: &str,
        request: CreateExportRequest,
    ) -> Result<ExportRequestResponse, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let data_types = request.data_types.join(",");
        let export_format = request.export_format.unwrap_or_else(|| "json".to_string());

        // Export links expire after 7 days
        let expires_at = Utc::now()
            .checked_add_signed(Duration::days(7))
            .unwrap()
            .to_rfc3339();

        // Generate a secure download token
        let download_token = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO data_export_requests (id, user_id, status, requested_data_types, export_format, requested_at, expires_at, download_token)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(user_id)
        .bind("pending")
        .bind(&data_types)
        .bind(&export_format)
        .bind(&now)
        .bind(&expires_at)
        .bind(&download_token)
        .execute(&self.db)
        .await
        .map_err(AppError::Database)?;

        Ok(ExportRequestResponse {
            id,
            status: "pending".to_string(),
            requested_at: now,
            expires_at: Some(expires_at),
            download_url: None,
        })
    }

    /// Get export request status
    pub async fn get_export_request(
        &self,
        user_id: &str,
        request_id: &str,
    ) -> Result<ExportRequestResponse, AppError> {
        let request = sqlx::query_as::<_, DataExportRequest>(
            "SELECT * FROM data_export_requests WHERE id = ? AND user_id = ?",
        )
        .bind(request_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound("Export request not found".to_string()))?;

        let download_url = if request.status == "completed" && request.download_token.is_some() {
            Some(format!(
                "/api/gdpr/download/{}",
                request.download_token.unwrap()
            ))
        } else {
            None
        };

        Ok(ExportRequestResponse {
            id: request.id,
            status: request.status,
            requested_at: request.requested_at,
            expires_at: request.expires_at,
            download_url,
        })
    }

    /// Get all export requests for a user
    pub async fn get_user_export_requests(
        &self,
        user_id: &str,
    ) -> Result<Vec<ExportRequestResponse>, AppError> {
        let requests = sqlx::query_as::<_, DataExportRequest>(
            "SELECT * FROM data_export_requests WHERE user_id = ? ORDER BY requested_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await
        .map_err(AppError::Database)?;

        let mut responses = Vec::new();
        for request in requests {
            let download_url = if request.status == "completed" && request.download_token.is_some()
            {
                Some(format!(
                    "/api/gdpr/download/{}",
                    request.download_token.unwrap()
                ))
            } else {
                None
            };

            responses.push(ExportRequestResponse {
                id: request.id,
                status: request.status,
                requested_at: request.requested_at,
                expires_at: request.expires_at,
                download_url,
            });
        }

        Ok(responses)
    }

    /// Create a data deletion request
    pub async fn create_deletion_request(
        &self,
        user_id: &str,
        request: CreateDeletionRequest,
    ) -> Result<DeletionRequestResponse, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // Generate confirmation token
        let confirmation_token = Uuid::new_v4().to_string();

        let delete_all_data = request.delete_all_data.unwrap_or(true);
        let data_types = request.data_types.map(|d| d.join(","));

        sqlx::query(
            "INSERT INTO data_deletion_requests (id, user_id, status, reason, delete_all_data, data_types_to_delete, requested_at, confirmation_token)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(user_id)
        .bind("pending")
        .bind(&request.reason)
        .bind(delete_all_data)
        .bind(&data_types)
        .bind(&now)
        .bind(&confirmation_token)
        .execute(&self.db)
        .await
        .map_err(AppError::Database)?;

        Ok(DeletionRequestResponse {
            id,
            status: "pending".to_string(),
            requested_at: now,
            scheduled_deletion_at: None,
            confirmation_required: true,
            confirmation_token: Some(confirmation_token),
        })
    }

    /// Confirm a deletion request
    pub async fn confirm_deletion(
        &self,
        confirmation_token: &str,
    ) -> Result<DeletionRequestResponse, AppError> {
        let now = Utc::now().to_rfc3339();

        // Schedule deletion for 24 hours from now
        let scheduled_deletion = Utc::now()
            .checked_add_signed(Duration::hours(24))
            .unwrap()
            .to_rfc3339();

        let result = sqlx::query(
            "UPDATE data_deletion_requests SET status = ?, scheduled_deletion_at = ? WHERE confirmation_token = ? AND status = ?"
        )
        .bind("scheduled")
        .bind(&scheduled_deletion)
        .bind(confirmation_token)
        .bind("pending")
        .execute(&self.db)
        .await
        .map_err(AppError::Database)?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Deletion request not found or already processed".to_string(),
            ));
        }

        let request = sqlx::query_as::<_, DataDeletionRequest>(
            "SELECT * FROM data_deletion_requests WHERE confirmation_token = ?",
        )
        .bind(confirmation_token)
        .fetch_one(&self.db)
        .await
        .map_err(AppError::Database)?;

        Ok(DeletionRequestResponse {
            id: request.id,
            status: request.status,
            requested_at: request.requested_at,
            scheduled_deletion_at: request.scheduled_deletion_at,
            confirmation_required: false,
            confirmation_token: None,
        })
    }

    /// Cancel a deletion request
    pub async fn cancel_deletion(
        &self,
        user_id: &str,
        request_id: &str,
    ) -> Result<DeletionRequestResponse, AppError> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE data_deletion_requests SET status = ?, cancelled_at = ? WHERE id = ? AND user_id = ? AND status IN (?, ?)"
        )
        .bind("cancelled")
        .bind(&now)
        .bind(request_id)
        .bind(user_id)
        .bind("pending")
        .bind("scheduled")
        .execute(&self.db)
        .await
        .map_err(AppError::Database)?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Deletion request not found or cannot be cancelled".to_string(),
            ));
        }

        let request = sqlx::query_as::<_, DataDeletionRequest>(
            "SELECT * FROM data_deletion_requests WHERE id = ?",
        )
        .bind(request_id)
        .fetch_one(&self.db)
        .await
        .map_err(AppError::Database)?;

        Ok(DeletionRequestResponse {
            id: request.id,
            status: request.status,
            requested_at: request.requested_at,
            scheduled_deletion_at: request.scheduled_deletion_at,
            confirmation_required: false,
            confirmation_token: None,
        })
    }

    /// Get deletion request status
    pub async fn get_deletion_request(
        &self,
        user_id: &str,
        request_id: &str,
    ) -> Result<DeletionRequestResponse, AppError> {
        let request = sqlx::query_as::<_, DataDeletionRequest>(
            "SELECT * FROM data_deletion_requests WHERE id = ? AND user_id = ?",
        )
        .bind(request_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound("Deletion request not found".to_string()))?;

        Ok(DeletionRequestResponse {
            id: request.id,
            status: request.status,
            requested_at: request.requested_at,
            scheduled_deletion_at: request.scheduled_deletion_at,
            confirmation_required: false,
            confirmation_token: None,
        })
    }

    /// Get all deletion requests for a user
    pub async fn get_user_deletion_requests(
        &self,
        user_id: &str,
    ) -> Result<Vec<DeletionRequestResponse>, AppError> {
        let requests = sqlx::query_as::<_, DataDeletionRequest>(
            "SELECT * FROM data_deletion_requests WHERE user_id = ? ORDER BY requested_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await
        .map_err(AppError::Database)?;

        let mut responses = Vec::new();
        for request in requests {
            responses.push(DeletionRequestResponse {
                id: request.id,
                status: request.status,
                requested_at: request.requested_at,
                scheduled_deletion_at: request.scheduled_deletion_at,
                confirmation_required: false,
                confirmation_token: None,
            });
        }

        Ok(responses)
    }

    /// Get GDPR summary for a user
    pub async fn get_gdpr_summary(&self, user_id: &str) -> Result<GdprSummary, AppError> {
        let consents = self.get_user_consents(user_id).await?;

        let pending_exports: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM data_export_requests WHERE user_id = ? AND status IN (?, ?)",
        )
        .bind(user_id)
        .bind("pending")
        .bind("processing")
        .fetch_one(&self.db)
        .await
        .map_err(AppError::Database)?;

        let pending_deletions: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM data_deletion_requests WHERE user_id = ? AND status IN (?, ?)",
        )
        .bind(user_id)
        .bind("pending")
        .bind("scheduled")
        .fetch_one(&self.db)
        .await
        .map_err(AppError::Database)?;

        let processing_count: i32 =
            sqlx::query_scalar("SELECT COUNT(*) FROM data_processing_log WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.db)
                .await
                .map_err(AppError::Database)?;

        Ok(GdprSummary {
            user_id: user_id.to_string(),
            consents,
            pending_export_requests: pending_exports,
            pending_deletion_requests: pending_deletions,
            data_processing_activities_count: processing_count,
        })
    }

    /// Get available exportable data types
    pub fn get_exportable_data_types() -> ExportableDataTypes {
        ExportableDataTypes {
            types: vec![
                DataTypeInfo {
                    id: "profile".to_string(),
                    name: "Profile Information".to_string(),
                    description: "Your account profile data including username and preferences"
                        .to_string(),
                    category: "account".to_string(),
                },
                DataTypeInfo {
                    id: "activity".to_string(),
                    name: "Activity History".to_string(),
                    description: "Your activity logs and transaction history".to_string(),
                    category: "activity".to_string(),
                },
                DataTypeInfo {
                    id: "api_keys".to_string(),
                    name: "API Keys".to_string(),
                    description: "Your generated API keys and their metadata".to_string(),
                    category: "security".to_string(),
                },
                DataTypeInfo {
                    id: "consents".to_string(),
                    name: "Consent Records".to_string(),
                    description: "Your consent preferences and history".to_string(),
                    category: "privacy".to_string(),
                },
                DataTypeInfo {
                    id: "notifications".to_string(),
                    name: "Notification Settings".to_string(),
                    description: "Your notification preferences".to_string(),
                    category: "preferences".to_string(),
                },
                DataTypeInfo {
                    id: "analytics".to_string(),
                    name: "Analytics Data".to_string(),
                    description: "Analytics data associated with your account".to_string(),
                    category: "analytics".to_string(),
                },
            ],
        }
    }

    /// Log a data processing activity
    pub async fn log_data_processing(
        &self,
        user_id: &str,
        activity_type: &str,
        data_category: &str,
        purpose: Option<String>,
        legal_basis: Option<String>,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO data_processing_log (id, user_id, activity_type, data_category, purpose, legal_basis, processed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(activity_type)
        .bind(data_category)
        .bind(&purpose)
        .bind(&legal_basis)
        .bind(&now)
        .execute(&self.db)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }
}
