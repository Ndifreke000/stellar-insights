/// OAuth 2.0 module for Zapier integration
/// Handles authorization code flow, token generation, and scope validation
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

/// OAuth Claims - extended JWT with additional Zapier fields
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthClaims {
    pub sub: String,         // User ID
    pub username: String,    // Username
    pub client_id: String,   // OAuth client ID
    pub scopes: Vec<String>, // Granted scopes
    pub exp: i64,            // Expiry timestamp
    pub iat: i64,            // Issued at timestamp
    pub aud: String,         // Audience (must be "zapier")
    pub token_type: String,  // "access" or "refresh"
}

/// OAuth authorization code (short-lived, for exchanging to tokens)
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorizationCode {
    pub code: String,
    pub user_id: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub expires_at: i64,
    pub redirect_uri: String,
}

/// OAuth Token Response
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64, // seconds until expiration
    pub scope: String,   // space-separated scopes
}

/// OAuth Error Response
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthError {
    pub error: String,
    pub error_description: Option<String>,
}

/// Available OAuth scopes for Zapier
pub const AVAILABLE_SCOPES: &[&str] = &[
    "read:corridors",
    "read:anchors",
    "read:payments",
    "write:webhooks",
];

/// OAuth Service
pub struct OAuthService {
    jwt_secret: String,
    jwt_audience: String,
    token_expiry_days: i64,
    refresh_expiry_days: i64,
    encryption_key: String,
    db: SqlitePool,
}

impl OAuthService {
    /// Create new OAuth service
    pub fn new(db: SqlitePool) -> Self {
        let jwt_secret = std::env::var("JWT_SECRET")
            .expect("JWT_SECRET environment variable is required for OAuth service");

        let jwt_audience = std::env::var("JWT_AUDIENCE").unwrap_or_else(|_| "zapier".to_string());

        let token_expiry_days: i64 = std::env::var("OAUTH_TOKEN_EXPIRY_DAYS")
            .unwrap_or_else(|_| "7".to_string())
            .parse()
            .unwrap_or(7);

        let refresh_expiry_days: i64 = std::env::var("OAUTH_REFRESH_EXPIRY_DAYS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);

        let encryption_key = std::env::var("ENCRYPTION_KEY")
            .expect("ENCRYPTION_KEY environment variable is required for OAuth service");

        Self {
            jwt_secret,
            jwt_audience,
            token_expiry_days,
            refresh_expiry_days,
            encryption_key,
            db,
        }
    }

    /// Create OAuth client (app registration)
    pub async fn create_oauth_client(
        &self,
        user_id: &str,
        app_name: &str,
    ) -> Result<(String, String)> {
        let id = Uuid::new_v4().to_string();
        let client_id = Uuid::new_v4().to_string();
        let client_secret = Uuid::new_v4().to_string();

        let encrypted_secret = crate::crypto::encrypt_data(&client_secret, &self.encryption_key)
            .map_err(|e| anyhow!("Failed to encrypt client secret: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO oauth_clients (id, user_id, client_id, client_secret, app_name)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(client_id.clone())
        .bind(encrypted_secret)
        .bind(app_name)
        .execute(&self.db)
        .await?;

        Ok((client_id, client_secret))
    }

    /// Validate client credentials (for token endpoint)
    pub async fn validate_client_credentials(
        &self,
        client_id: &str,
        client_secret: &str,
    ) -> Result<String> {
        let row = sqlx::query(
            r#"
            SELECT user_id, client_secret FROM oauth_clients
            WHERE client_id = ?
            "#,
        )
        .bind(client_id)
        .fetch_optional(&self.db)
        .await?;

        let client = row.map(|r| {
            use sqlx::Row;
            (r.get::<String, _>(0), r.get::<String, _>(1))
        });

        match client {
            Some((user_id, client_secret_record)) => {
                let decrypted_secret =
                    crate::crypto::decrypt_data(&client_secret_record, &self.encryption_key)
                        .map_err(|_| anyhow!("Invalid client credentials"))?;
                if decrypted_secret == client_secret {
                    Ok(user_id)
                } else {
                    Err(anyhow!("Invalid client credentials"))
                }
            }
            None => Err(anyhow!("Invalid client credentials")),
        }
    }

    /// Validate and decode OAuth token
    pub fn validate_oauth_token(&self, token: &str) -> Result<OAuthClaims> {
        let validation = Validation::default();

        let decoded = decode::<OAuthClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| anyhow!("Failed to decode token: {}", e))?;

        // Verify audience
        if decoded.claims.aud != self.jwt_audience {
            return Err(anyhow!(
                "Invalid audience: expected '{}', got '{}'",
                self.jwt_audience,
                decoded.claims.aud
            ));
        }

        Ok(decoded.claims)
    }

    /// Generate access token
    pub fn generate_access_token(
        &self,
        user_id: &str,
        username: &str,
        client_id: &str,
        scopes: Vec<String>,
    ) -> Result<String> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::days(self.token_expiry_days))
            .ok_or_else(|| anyhow!("Invalid timestamp"))?
            .timestamp();

        let claims = OAuthClaims {
            sub: user_id.to_string(),
            username: username.to_string(),
            client_id: client_id.to_string(),
            scopes,
            exp: expiration,
            iat: Utc::now().timestamp(),
            aud: self.jwt_audience.clone(),
            token_type: "access".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to generate access token: {}", e))
    }

    /// Generate refresh token
    pub fn generate_refresh_token(
        &self,
        user_id: &str,
        username: &str,
        client_id: &str,
    ) -> Result<String> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::days(self.refresh_expiry_days))
            .ok_or_else(|| anyhow!("Invalid timestamp"))?
            .timestamp();

        let claims = OAuthClaims {
            sub: user_id.to_string(),
            username: username.to_string(),
            client_id: client_id.to_string(),
            scopes: vec![],
            exp: expiration,
            iat: Utc::now().timestamp(),
            aud: self.jwt_audience.clone(),
            token_type: "refresh".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to generate refresh token: {}", e))
    }

    /// Validate scopes (ensure requested scopes are allowed)
    pub fn validate_scopes(&self, requested_scopes: &str) -> Result<Vec<String>> {
        let scopes: Vec<&str> = requested_scopes.split_whitespace().collect();

        for scope in &scopes {
            if !AVAILABLE_SCOPES.contains(scope) {
                return Err(anyhow!("Invalid scope: {}", scope));
            }
        }

        Ok(scopes.iter().map(|s| s.to_string()).collect())
    }

    /// Store authorization for a client+user (for authorization code flow)
    pub async fn store_authorization(
        &self,
        client_id: &str,
        user_id: &str,
        scopes: &[String],
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let scopes_str = scopes.join(",");

        sqlx::query(
            r#"
            INSERT INTO oauth_authorizations (id, client_id, user_id, scopes)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(client_id)
        .bind(user_id)
        .bind(scopes_str)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get authorization for a client+user (check if they've already authorized)
    pub async fn get_authorization(
        &self,
        client_id: &str,
        user_id: &str,
    ) -> Result<Option<Vec<String>>> {
        let row = sqlx::query(
            r#"
            SELECT scopes FROM oauth_authorizations
            WHERE client_id = ? AND user_id = ?
            "#,
        )
        .bind(client_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        let auth = row.map(|r| {
            use sqlx::Row;
            r.get::<String, _>(0)
        });

        Ok(auth.map(|record| record.split(',').map(|s: &str| s.to_string()).collect()))
    }

    /// Store OAuth token in database
    pub async fn store_token(
        &self,
        user_id: &str,
        access_token: &str,
        refresh_token: &str,
        _expires_at: i64,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let expires_at_str = Utc::now()
            .checked_add_signed(Duration::days(self.token_expiry_days))
            .ok_or_else(|| anyhow!("Invalid timestamp"))?
            .to_rfc3339();

        let enc_access_token = crate::crypto::encrypt_data(access_token, &self.encryption_key)
            .map_err(|e| anyhow!("Failed to encrypt access token: {}", e))?;
        let enc_refresh_token = crate::crypto::encrypt_data(refresh_token, &self.encryption_key)
            .map_err(|e| anyhow!("Failed to encrypt refresh token: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO oauth_tokens (id, user_id, access_token, refresh_token, token_type, expires_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(enc_access_token)
        .bind(enc_refresh_token)
        .bind("Bearer")
        .bind(expires_at_str)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Revoke OAuth token by deleting it from the database
    pub async fn revoke_token(&self, access_token: &str) -> Result<()> {
        let enc_token = crate::crypto::encrypt_data(access_token, &self.encryption_key)
            .map_err(|e| anyhow!("Failed to encrypt token for lookup: {}", e))?;

        let result = sqlx::query(r#"DELETE FROM oauth_tokens WHERE access_token = ?"#)
            .bind(enc_token)
            .execute(&self.db)
            .await?;

        if result.rows_affected() == 0 {
            tracing::warn!("Token revocation requested but token not found in database");
        } else {
            tracing::info!("OAuth token revoked successfully");
        }

        Ok(())
    }
}
