pub mod oauth;
/// SEP-10 authentication — canonical implementation.
///
/// `sep10_simple` is the only supported SEP-10 module in this repository.
/// A previous `sep10` module that depended on `stellar-xdr` directly has been
/// removed. All handler wiring must use `crate::auth::sep10_simple::Sep10Service`.
pub mod sep10_simple;

use anyhow::{anyhow, Result};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// Token expiry constants
const ACCESS_TOKEN_EXPIRY_HOURS: i64 = 1;
const REFRESH_TOKEN_EXPIRY_DAYS: i64 = 7;
/// How long a pending-2FA token (issued by login() when the account has 2FA
/// enabled, consumed by complete_2fa_login()) stays valid. Short on purpose:
/// it only needs to survive the gap between typing a password and typing a
/// 6-digit code, and a short window bounds how long a leaked pending token
/// (e.g. from a compromised client-side log) is useful to an attacker who
/// still needs the second factor to do anything with it.
const PENDING_2FA_TOKEN_EXPIRY_MINUTES: i64 = 5;

// WARNING: Demo credentials removed for security. Use database-backed user store.
// See SEC-001 in SECURITY_AUDIT.md

/// User model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub is_admin: bool,
}

/// Login request
#[derive(Debug, Deserialize)]
#[derive(utoipa::ToSchema)]
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

/// Result of a login attempt: either the account has no 2FA and login
/// completed immediately, or it does and the caller must now present a TOTP
/// or backup code to `AuthService::complete_2fa_login` before real tokens
/// are issued.
#[derive(Debug, Serialize)]
#[serde(tag = "status")]
pub enum LoginOutcome {
    #[serde(rename = "success")]
    Success(LoginResponse),
    #[serde(rename = "two_fa_required")]
    TwoFaRequired {
        /// Short-lived token identifying this login attempt. Not a
        /// credential on its own -- it only proves "this client already
        /// supplied a correct password", and complete_2fa_login still
        /// requires a valid TOTP/backup code before issuing real tokens.
        pending_token: String,
        expires_in: i64,
    },
}

/// Request body for completing a 2FA-gated login.
#[derive(Debug, Deserialize)]
#[derive(utoipa::ToSchema)]
pub struct VerifyTwoFaRequest {
    pub pending_token: String,
    pub code: String,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
#[derive(utoipa::ToSchema)]
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
#[derive(utoipa::ToSchema)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>, // JWT ID — present on access tokens for revocation checks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>, // Session ID for device/session tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>, // Refresh token JTI (for refresh token validation)
    /// `#[serde(default)]`: tokens issued before this field existed decode
    /// to `false` rather than failing, matching the users.is_admin
    /// column's own default.
    #[serde(default)]
    pub is_admin: bool,
}

/// Authentication service
pub struct AuthService {
    jwt_secret: String,
    redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    db_pool: SqlitePool,
    session_service: crate::session::SessionService,
    twofa: crate::twofa::TwoFAService,
}

impl AuthService {
    pub fn new(
        redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
        db_pool: SqlitePool,
    ) -> Self {
        let jwt_secret = crate::vault::SecretsService::from_env()
            .map(|s| s.jwt_secret)
            .unwrap_or_else(|_| {
                std::env::var("JWT_SECRET")
                    .expect("JWT_SECRET environment variable is required. Generate a cryptographically secure random key of at least 32 bytes.")
            });

        assert!(
            jwt_secret.len() >= 32,
            "JWT_SECRET must be at least 32 characters for adequate security"
        );
        assert!(
            !jwt_secret.starts_with("CHANGE_ME"),
            "JWT_SECRET must not use a placeholder value — generate a cryptographically secure random key"
        );

        let session_service = crate::session::SessionService::new(db_pool.clone());
        let twofa = crate::twofa::TwoFAService::new(db_pool.clone(), crate::crypto::CryptoService::from_env());

        Self {
            jwt_secret,
            redis_connection,
            db_pool,
            session_service,
            twofa,
        }
    }

    pub fn new_with_secret(
        redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
        db_pool: SqlitePool,
        jwt_secret: String,
    ) -> Self {
        assert!(
            jwt_secret.len() >= 32,
            "JWT_SECRET must be at least 32 characters for adequate security"
        );
        assert!(
            !jwt_secret.starts_with("CHANGE_ME"),
            "JWT_SECRET must not use a placeholder value — generate a cryptographically secure random key"
        );

        let session_service = crate::session::SessionService::new(db_pool.clone());
        let twofa = crate::twofa::TwoFAService::new(db_pool.clone(), crate::crypto::CryptoService::from_env());

        Self {
            jwt_secret,
            redis_connection,
            db_pool,
            session_service,
            twofa,
        }
    }

    /// The resolved JWT signing secret (from Vault or JWT_SECRET), for
    /// wiring `auth_middleware`'s `JwtSecret` extension to the same value
    /// this service uses to issue tokens.
    #[must_use]
    pub fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }

    /// Session management (list/revoke), for HTTP handlers that need to act
    /// on a specific session rather than the whole login/refresh/logout flow.
    #[must_use]
    pub const fn session_service(&self) -> &crate::session::SessionService {
        &self.session_service
    }

    /// Database pool, for wiring `auth_middleware`'s `TokenRevocationStore`
    /// extension to the same database this service uses.
    #[must_use]
    pub const fn db_pool(&self) -> &SqlitePool {
        &self.db_pool
    }

    /// Authenticate user with credentials against the database.
    /// Passwords are verified using argon2 — never stored or compared in plaintext.
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<User> {
        #[derive(sqlx::FromRow)]
        struct UserRecord {
            id: String,
            username: String,
            password_hash: String,
            is_admin: bool,
        }

        let record = sqlx::query_as::<_, UserRecord>(
            "SELECT id, username, password_hash, is_admin FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| anyhow!("Database error during authentication: {e}"))?;

        let record = record.ok_or_else(|| anyhow!("Invalid username or password"))?;

        let parsed_hash = PasswordHash::new(&record.password_hash)
            .map_err(|e| anyhow!("Failed to parse password hash: {e}"))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| anyhow!("Invalid username or password"))?;

        Ok(User {
            id: record.id,
            username: record.username,
            is_admin: record.is_admin,
        })
    }

    /// Generate access token with session tracking
    pub fn generate_access_token(&self, user: &User, session_id: Option<&str>) -> Result<String> {
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
            jti: Some(Uuid::new_v4().to_string()),
            session_id: session_id.map(|s| s.to_string()),
            sid: None,
            is_admin: user.is_admin,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to generate access token: {e}"))
    }

    /// Generate refresh token with session tracking
    pub fn generate_refresh_token(&self, user: &User, session_id: Option<&str>, refresh_token_jti: &str) -> Result<String> {
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
            jti: Some(refresh_token_jti.to_string()),
            session_id: session_id.map(|s| s.to_string()),
            sid: Some(refresh_token_jti.to_string()),
            is_admin: user.is_admin,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to generate refresh token: {e}"))
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
        .map_err(|e| anyhow!("Invalid token: {e}"))
    }

    /// Store refresh token in Redis
    pub async fn store_refresh_token(&self, token: &str, user_id: &str) -> Result<()> {
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            let key = format!("refresh_token:{user_id}");
            let expiry = REFRESH_TOKEN_EXPIRY_DAYS * 24 * 60 * 60; // seconds

            conn.set_ex::<_, _, ()>(&key, token, expiry as u64)
                .await
                .map_err(|e| anyhow!("Failed to store refresh token: {e}"))?;

            tracing::debug!(
                user_id = crate::logging::redaction::redact_user_id(user_id),
                "Stored refresh token for user"
            );
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
                .map_err(|e| anyhow!("Failed to retrieve refresh token: {e}"))?;

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
            let key = format!("refresh_token:{user_id}");

            conn.del::<_, ()>(&key)
                .await
                .map_err(|e| anyhow!("Failed to invalidate refresh token: {e}"))?;

            tracing::debug!(
                user_id = crate::logging::redaction::redact_user_id(user_id),
                "Invalidated refresh token for user"
            );
        }

        Ok(())
    }

    /// Login flow with session and device tracking. If the account has 2FA
    /// enabled, this stops short of issuing real tokens and instead returns
    /// a pending_token that must be presented to complete_2fa_login along
    /// with a TOTP/backup code.
    pub async fn login(
        &self,
        request: LoginRequest,
        device_user_agent: Option<String>,
        ip_address: &str,
    ) -> Result<LoginOutcome> {
        // Authenticate user
        let user = self
            .authenticate(&request.username, &request.password)
            .await?;

        if self.twofa.is_2fa_enabled(&user.id).await? {
            let pending_token = self.generate_pending_2fa_token(&user)?;
            return Ok(LoginOutcome::TwoFaRequired {
                pending_token,
                expires_in: PENDING_2FA_TOKEN_EXPIRY_MINUTES * 60,
            });
        }

        self.complete_login(&user, device_user_agent, ip_address)
            .await
            .map(LoginOutcome::Success)
    }

    /// Verify the code from a pending 2FA login (see login's TwoFaRequired
    /// branch) and, if valid, finish the login exactly as the no-2FA path
    /// would: create a session and issue real tokens.
    pub async fn complete_2fa_login(
        &self,
        request: VerifyTwoFaRequest,
        device_user_agent: Option<String>,
        ip_address: &str,
    ) -> Result<LoginResponse> {
        let claims = self.validate_token(&request.pending_token)?;

        if claims.token_type != "pending_2fa" {
            return Err(anyhow!("Invalid token type"));
        }

        let jti = claims
            .jti
            .clone()
            .ok_or_else(|| anyhow!("Pending 2FA token missing jti"))?;

        // Single-use: a pending token that's already been consumed (a prior
        // successful verify) must not grant a second independent session.
        let revocation_store =
            crate::auth_middleware::TokenRevocationStore(Arc::new(self.db_pool.clone()));
        if crate::auth_middleware::is_token_revoked(&revocation_store, &jti)
            .await
            .map_err(|e| anyhow!("Revocation check failed: {e}"))?
        {
            return Err(anyhow!("Pending 2FA token already used"));
        }

        if !self.twofa.verify_login_code(&claims.sub, &request.code).await? {
            return Err(anyhow!("Invalid 2FA code"));
        }

        // Consume the token now that it's done its job, so a replay of the
        // same pending_token + code (e.g. a retried request) can't mint a
        // second session.
        crate::auth_middleware::revoke_token(&revocation_store, &jti, &claims.sub, claims.exp)
            .await
            .map_err(|e| anyhow!("Failed to revoke pending 2FA token: {e}"))?;

        // Reconstructed from the pending token's own claims rather than a
        // fresh DB lookup -- same tradeoff already made in refresh() below
        // for is_admin: acceptable given the token is only 5 minutes old.
        let user = User {
            id: claims.sub,
            username: claims.username,
            is_admin: claims.is_admin,
        };

        self.complete_login(&user, device_user_agent, ip_address).await
    }

    /// Shared tail of both login paths: create a session and issue real
    /// access/refresh tokens for an already-authenticated (password, and
    /// 2FA if enabled) user.
    async fn complete_login(
        &self,
        user: &User,
        device_user_agent: Option<String>,
        ip_address: &str,
    ) -> Result<LoginResponse> {
        // Create session with device tracking
        let refresh_token_jti = Uuid::new_v4().to_string();
        let session = self
            .session_service
            .create_session(&user.id, &refresh_token_jti, device_user_agent, ip_address, None, None)
            .await?;

        // Generate tokens with session_id
        let access_token = self.generate_access_token(user, Some(&session.id))?;
        let refresh_token = self.generate_refresh_token(user, Some(&session.id), &refresh_token_jti)?;

        // Store refresh token
        self.store_refresh_token(&refresh_token, &user.id).await?;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            expires_in: ACCESS_TOKEN_EXPIRY_HOURS * 3600,
        })
    }

    /// Short-lived token identifying a password-verified, 2FA-pending login
    /// attempt. Deliberately carries no session_id (no session exists yet)
    /// and a distinct token_type so it can't be mistaken for, or used as, a
    /// real access/refresh token by auth_middleware (which only accepts
    /// token_type == "access").
    fn generate_pending_2fa_token(&self, user: &User) -> Result<String> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::minutes(PENDING_2FA_TOKEN_EXPIRY_MINUTES))
            .ok_or_else(|| anyhow!("Invalid timestamp"))?
            .timestamp();

        let claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            exp: expiration,
            iat: Utc::now().timestamp(),
            token_type: "pending_2fa".to_string(),
            jti: Some(Uuid::new_v4().to_string()),
            session_id: None,
            sid: None,
            is_admin: user.is_admin,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to generate pending 2FA token: {e}"))
    }

    /// Refresh access token and touch session
    pub async fn refresh(&self, request: RefreshTokenRequest) -> Result<RefreshTokenResponse> {
        // Validate refresh token
        let claims = self.validate_refresh_token(&request.refresh_token).await?;

        // Create user from claims. is_admin carries over from the refresh
        // token's own claims (set at login, see generate_refresh_token)
        // rather than a fresh DB lookup -- if an admin is demoted, that
        // takes effect on their next full login, not their next refresh.
        // Acceptable here: refresh tokens are already short-lived relative
        // to how often role changes should matter, and re-querying on
        // every refresh would add a DB round-trip to the cheapest path.
        let user = User {
            id: claims.sub,
            username: claims.username,
            is_admin: claims.is_admin,
        };

        // Touch session if session_id is present
        if let Some(session_id) = &claims.session_id {
            if let Err(e) = self.session_service.touch_session(session_id).await {
                tracing::warn!("Failed to touch session: {e}");
            }
        }

        // Generate new access token with session_id
        let access_token = self.generate_access_token(&user, claims.session_id.as_deref())?;

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

// SEP-10 Authentication Middleware and Types
// Consolidated from sep10_middleware.rs

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// Extract SEP-10 authenticated user from request
#[derive(Debug, Clone)]
pub struct Sep10User {
    pub account: String,
    pub client_domain: Option<String>,
}

/// SEP-10 claims for extracting authenticated user in handlers
#[derive(Debug, Clone)]
pub struct Sep10Claims {
    pub account: String,
    pub client_domain: Option<String>,
}

impl<S> FromRequestParts<S> for Sep10Claims
where
    S: Send + Sync,
{
    type Rejection = Sep10AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get Sep10User from extensions (set by middleware)
        parts
            .extensions
            .get::<Sep10User>()
            .map(|user| Self {
                account: user.account.clone(),
                client_domain: user.client_domain.clone(),
            })
            .ok_or(Sep10AuthError::MissingToken)
    }
}

/// SEP-10 authentication middleware
pub async fn sep10_auth_middleware(
    State(sep10_service): State<Arc<sep10_simple::Sep10Service>>,
    mut req: Request,
    next: Next,
) -> Result<Response, Sep10AuthError> {
    // Extract Authorization header and token before mutating req
    let token = {
        let auth_header = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(Sep10AuthError::MissingToken)?;

        // Extract Bearer token
        auth_header
            .strip_prefix("Bearer ")
            .ok_or(Sep10AuthError::InvalidToken)?
            .to_string()
    };

    // Validate session
    let session = sep10_service
        .validate_session(&token)
        .await
        .map_err(|_| Sep10AuthError::InvalidToken)?;

    // Attach user to request extensions
    let sep10_user = Sep10User {
        account: session.account,
        client_domain: session.client_domain,
    };
    req.extensions_mut().insert(sep10_user);
    req.extensions_mut().insert(token);

    Ok(next.run(req).await)
}

/// SEP-10 authentication errors
#[derive(Debug)]
pub enum Sep10AuthError {
    MissingToken,
    InvalidToken,
}

impl IntoResponse for Sep10AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authentication token"),
            Self::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
        };

        let body = json!({
            "error": message,
        });

        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod pending_2fa_tests {
    use super::*;
    use argon2::password_hash::PasswordHasher;

    async fn migrated_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    fn test_service(pool: SqlitePool) -> AuthService {
        // AuthService::new_with_secret still builds a real TwoFAService
        // internally (CryptoService::from_env), so this needs a real-shaped
        // ENCRYPTION_KEY even though the JWT secret is passed explicitly.
        std::env::set_var(
            "ENCRYPTION_KEY",
            "33333333333333333333333333333333333333333333333333333333333333ef",
        );
        AuthService::new_with_secret(
            Arc::new(RwLock::new(None)),
            pool,
            "pending_2fa_test_jwt_secret_at_least_32_bytes_long".to_string(),
        )
    }

    async fn insert_user(pool: &SqlitePool, id: &str, username: &str, password: &str) {
        // argon2 0.6's hash_password generates its own random salt
        // internally now (see models/api_key.rs's identical fix).
        let hash = Argon2::default()
            .hash_password(password.as_bytes())
            .unwrap()
            .to_string();
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, created_at, updated_at) \
             VALUES (?, ?, ?, datetime('now'), datetime('now'))",
        )
        .bind(id)
        .bind(username)
        .bind(&hash)
        .execute(pool)
        .await
        .unwrap();
    }

    /// End-to-end: a 2FA-enrolled account's login stops at TwoFaRequired, a
    /// wrong code is rejected, the correct TOTP code completes the login,
    /// and the same pending_token cannot be replayed afterward.
    #[tokio::test]
    async fn login_with_2fa_enabled_requires_and_completes_verify_2fa() {
        let pool = migrated_pool().await;
        insert_user(&pool, "user-2fa-test-1", "twofa_tester", "correct horse battery staple").await;
        let service = test_service(pool);

        // No 2FA enrolled yet: login should succeed immediately.
        let outcome = service
            .login(
                LoginRequest {
                    username: "twofa_tester".to_string(),
                    password: "correct horse battery staple".to_string(),
                },
                None,
                "127.0.0.1",
            )
            .await
            .unwrap();
        assert!(matches!(outcome, LoginOutcome::Success(_)));

        // Enroll and activate 2FA directly via the service's own (private,
        // but visible to this descendant module) twofa field -- mirrors
        // what api/twofa.rs's initiate_enrollment + confirm_enrollment do.
        let (_, secret) = service
            .twofa
            .generate_totp_secret("user-2fa-test-1", "twofa_tester")
            .unwrap();
        service
            .twofa
            .enroll_2fa("user-2fa-test-1", &secret)
            .await
            .unwrap();
        service.twofa.activate_2fa("user-2fa-test-1").await.unwrap();

        // Now login must stop short of issuing real tokens.
        let outcome = service
            .login(
                LoginRequest {
                    username: "twofa_tester".to_string(),
                    password: "correct horse battery staple".to_string(),
                },
                None,
                "127.0.0.1",
            )
            .await
            .unwrap();
        let pending_token = match outcome {
            LoginOutcome::TwoFaRequired { pending_token, .. } => pending_token,
            LoginOutcome::Success(_) => panic!("expected 2FA to be required"),
        };

        // A wrong code must not complete the login.
        let rejected = service
            .complete_2fa_login(
                VerifyTwoFaRequest {
                    pending_token: pending_token.clone(),
                    code: "000000".to_string(),
                },
                None,
                "127.0.0.1",
            )
            .await;
        assert!(rejected.is_err());

        // The correct TOTP code, computed independently the same way
        // twofa.rs's own RFC 6238 tests do, must complete the login.
        let secret_bytes =
            base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret).unwrap();
        let step = (Utc::now().timestamp().max(0) as u64) / 30;
        let code = crate::twofa::totp_code_for_step(&secret_bytes, step);

        let completed = service
            .complete_2fa_login(
                VerifyTwoFaRequest {
                    pending_token: pending_token.clone(),
                    code,
                },
                None,
                "127.0.0.1",
            )
            .await
            .unwrap();
        assert!(!completed.access_token.is_empty());
        assert!(!completed.refresh_token.is_empty());

        // The same pending_token must not be usable a second time, even
        // with a (still currently valid) correct code.
        let replay = service
            .complete_2fa_login(
                VerifyTwoFaRequest {
                    pending_token,
                    code: crate::twofa::totp_code_for_step(&secret_bytes, step),
                },
                None,
                "127.0.0.1",
            )
            .await;
        assert!(replay.is_err());
    }
}
