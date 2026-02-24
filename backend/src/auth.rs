// pub mod sep10;  // Commented out - uses stellar-xdr types that require stellar-base
pub mod oauth;
pub mod sep10_middleware;
pub mod sep10_simple;

use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// Token expiry constants
const ACCESS_TOKEN_EXPIRY_HOURS: i64 = 1;
const REFRESH_TOKEN_EXPIRY_DAYS: i64 = 7;

// WARNING: Demo credentials removed for security. Use database-backed user store.
// See SEC-001 in SECURITY_AUDIT.md

/// User model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Refresh token response
#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub expires_in: i64,
}

/// Logout request
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // User ID
    pub username: String,   // Username
    pub exp: i64,           // Expiry timestamp
    pub iat: i64,           // Issued at timestamp
    pub token_type: String, // "access" or "refresh"
}

/// Authentication service
pub struct AuthService {
    jwt_secret: String,
    redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
}

impl AuthService {
    pub fn new(redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>) -> Self {
        let jwt_secret = std::env::var("JWT_SECRET")
            .expect("JWT_SECRET environment variable is required. Generate a cryptographically secure random key of at least 32 bytes.");

        if jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters for adequate security");
        }

        Self {
            jwt_secret,
            redis_connection,
        }
    }

    /// Authenticate user with credentials
    /// TODO: Implement database-backed user store with bcrypt/argon2 password hashing
    pub fn authenticate(&self, _username: &str, _password: &str) -> Result<User> {
        // Hardcoded demo credentials removed for security (SEC-001).
        // This must be replaced with a proper database-backed user store
        // that uses bcrypt or argon2 for password hashing before production use.
        Err(anyhow!(
            "Authentication not configured. Implement database-backed user store."
        ))
    }

    /// Generate access token
    pub fn generate_access_token(&self, user: &User) -> Result<String> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::hours(ACCESS_TOKEN_EXPIRY_HOURS))
            .ok_or_else(|| anyhow!("Invalid timestamp"))?
            .timestamp();

        let claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            exp: expiration,
            iat: Utc::now().timestamp(),
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
    pub fn generate_refresh_token(&self, user: &User) -> Result<String> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::days(REFRESH_TOKEN_EXPIRY_DAYS))
            .ok_or_else(|| anyhow!("Invalid timestamp"))?
            .timestamp();

        let claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            exp: expiration,
            iat: Utc::now().timestamp(),
            token_type: "refresh".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to generate refresh token: {}", e))
    }

    /// Validate and decode token
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let validation = Validation::default();

        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|e| anyhow!("Invalid token: {}", e))
    }

    /// Store refresh token in Redis
    pub async fn store_refresh_token(&self, token: &str, user_id: &str) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("refresh_token:{}", user_id);
            let expiry = REFRESH_TOKEN_EXPIRY_DAYS * 24 * 60 * 60; // seconds

            conn.set_ex::<_, _, ()>(&key, token, expiry as u64)
                .await
                .map_err(|e| anyhow!("Failed to store refresh token: {}", e))?;

            tracing::debug!("Stored refresh token for user: {}", user_id);
        } else {
            tracing::warn!("Redis not available, refresh token not stored");
        }

        Ok(())
    }

    /// Validate refresh token from Redis
    pub async fn validate_refresh_token(&self, token: &str) -> Result<Claims> {
        // First validate JWT signature and expiry
        let claims = self.validate_token(token)?;

        // Verify it's a refresh token
        if claims.token_type != "refresh" {
            return Err(anyhow!("Invalid token type"));
        }

        // Check if token exists in Redis (fail closed - SEC-007)
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("refresh_token:{}", claims.sub);

            let stored_token: Option<String> = conn
                .get(&key)
                .await
                .map_err(|e| anyhow!("Failed to retrieve refresh token: {}", e))?;

            if stored_token.as_deref() != Some(token) {
                return Err(anyhow!("Refresh token not found or invalid"));
            }
        } else {
            tracing::error!(
                "Redis not available - refusing refresh token validation (fail closed)"
            );
            return Err(anyhow!("Token validation service unavailable"));
        }

        Ok(claims)
    }

    /// Invalidate refresh token (logout)
    pub async fn invalidate_refresh_token(&self, user_id: &str) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("refresh_token:{}", user_id);

            conn.del::<_, ()>(&key)
                .await
                .map_err(|e| anyhow!("Failed to invalidate refresh token: {}", e))?;

            tracing::debug!("Invalidated refresh token for user: {}", user_id);
        }

        Ok(())
    }

    /// Login flow
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse> {
        // Authenticate user
        let user = self.authenticate(&request.username, &request.password)?;

        // Generate tokens
        let access_token = self.generate_access_token(&user)?;
        let refresh_token = self.generate_refresh_token(&user)?;

        // Store refresh token
        self.store_refresh_token(&refresh_token, &user.id).await?;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            expires_in: ACCESS_TOKEN_EXPIRY_HOURS * 3600,
        })
    }

    /// Refresh access token
    pub async fn refresh(&self, request: RefreshTokenRequest) -> Result<RefreshTokenResponse> {
        // Validate refresh token
        let claims = self.validate_refresh_token(&request.refresh_token).await?;

        // Create user from claims
        let user = User {
            id: claims.sub,
            username: claims.username,
        };

        // Generate new access token
        let access_token = self.generate_access_token(&user)?;

        Ok(RefreshTokenResponse {
            access_token,
            expires_in: ACCESS_TOKEN_EXPIRY_HOURS * 3600,
        })
    }

    /// Logout flow
    pub async fn logout(&self, request: LogoutRequest) -> Result<()> {
        // Validate and get claims from refresh token
        let claims = self.validate_token(&request.refresh_token)?;

        // Invalidate refresh token
        self.invalidate_refresh_token(&claims.sub).await?;

        Ok(())
    }
}
