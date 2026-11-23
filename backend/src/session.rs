use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

const DEFAULT_IDLE_TIMEOUT_SECONDS: i64 = 3600; // 1 hour
const DEFAULT_MAX_LIFETIME_SECONDS: i64 = 604800; // 7 days

/// Default number of concurrent active sessions allowed per user when no
/// plan-specific limit is configured.
const DEFAULT_MAX_CONCURRENT_SESSIONS: i64 = 5;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub refresh_token_jti: String,
    pub device_user_agent: Option<String>,
    pub ip_address: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub idle_timeout_seconds: i64,
    pub max_lifetime_seconds: i64,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug)]
pub struct SessionInfo {
    pub id: String,
    pub device_user_agent: Option<String>,
    pub ip_address: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub is_current: bool,
}

/// Result of validating a presented session token against its bound device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceBinding {
    /// The presented device key matches the one bound at session creation.
    Match,
    /// The presented device key does not match the bound device.
    Mismatch,
}

/// Anomaly signal raised when a session is used from a new IP or country.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAnomaly {
    pub ip_changed: bool,
    pub country_changed: bool,
}

impl SessionAnomaly {
    pub fn is_anomalous(&self) -> bool {
        self.ip_changed || self.country_changed
    }
}

use crate::services::user_keys::UserKeyService;

pub struct SessionService {
    pool: SqlitePool,
    user_keys: UserKeyService,
}

impl SessionService {
    pub fn new(pool: SqlitePool) -> Self {
        let user_keys = UserKeyService::from_env(pool.clone());
        Self { pool, user_keys }
    }

    #[cfg(test)]
    pub fn with_user_keys(pool: SqlitePool, user_keys: UserKeyService) -> Self {
        Self { pool, user_keys }
    }

    /// Resolve the maximum number of concurrent sessions allowed for a plan.
    /// Unknown or missing plans fall back to the default limit.
    pub fn max_concurrent_sessions_for_plan(plan: Option<&str>) -> i64 {
        match plan.map(|p| p.trim().to_ascii_lowercase()).as_deref() {
            Some("free") | None | Some("") => DEFAULT_MAX_CONCURRENT_SESSIONS,
            Some("pro") => 10,
            Some("team") => 25,
            Some("enterprise") => 100,
            Some(_) => DEFAULT_MAX_CONCURRENT_SESSIONS,
        }
    }

    /// Create a new session with device tracking.
    ///
    /// The session is bound to `device_key` (a per-device secret derived by the
    /// client). Tokens presented from a different device key are rejected by
    /// [`Self::verify_device_binding`]. Concurrent sessions are capped per plan;
    /// when the cap is reached the oldest active session is revoked.
    pub async fn create_session(
        &self,
        user_id: &str,
        refresh_token_jti: &str,
        device_user_agent: Option<String>,
        ip_address: &str,
        idle_timeout_seconds: Option<i64>,
        max_lifetime_seconds: Option<i64>,
    ) -> Result<Session> {
        self.create_session_bound(
            user_id,
            refresh_token_jti,
            device_user_agent,
            ip_address,
            idle_timeout_seconds,
            max_lifetime_seconds,
            None,
            None,
        )
        .await
    }

    /// Create a session bound to a device key and constrained by a plan limit.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_session_bound(
        &self,
        user_id: &str,
        refresh_token_jti: &str,
        device_user_agent: Option<String>,
        ip_address: &str,
        idle_timeout_seconds: Option<i64>,
        max_lifetime_seconds: Option<i64>,
        device_key: Option<&str>,
        plan: Option<&str>,
    ) -> Result<Session> {
        let session_id = Uuid::new_v4().to_string();
        let idle_timeout = idle_timeout_seconds.unwrap_or(DEFAULT_IDLE_TIMEOUT_SECONDS);
        let max_lifetime = max_lifetime_seconds.unwrap_or(DEFAULT_MAX_LIFETIME_SECONDS);

        let now = Utc::now();
        let expires_at = now
            .checked_add_signed(Duration::seconds(max_lifetime))
            .ok_or_else(|| anyhow!("Invalid timestamp"))?;

        // PII fields are encrypted under the user's DEK so that backup
        // snapshots only ever contain ciphertext (crypto-shredding).
        let stored_ua = match &device_user_agent {
            Some(ua) => Some(self.user_keys.encrypt_field(user_id, ua).await?),
            None => None,
        };
        let stored_ip = self.user_keys.encrypt_field(user_id, ip_address).await?;
        let stored_device_key = match device_key {
            Some(key) => Some(self.user_keys.encrypt_field(user_id, key).await?),
            None => None,
        };

        // Enforce the per-plan concurrent session limit before inserting.
        let limit = Self::max_concurrent_sessions_for_plan(plan);
        self.enforce_concurrent_limit(user_id, limit).await?;

        sqlx::query(
            r"
            INSERT INTO sessions (
                id, user_id, refresh_token_jti, device_user_agent, ip_address,
                created_at, last_activity_at, expires_at, idle_timeout_seconds,
                max_lifetime_seconds, device_key
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&session_id)
        .bind(user_id)
        .bind(refresh_token_jti)
        .bind(&stored_ua)
        .bind(&stored_ip)
        .bind(now)
        .bind(now)
        .bind(expires_at)
        .bind(idle_timeout)
        .bind(max_lifetime)
        .bind(&stored_device_key)
        .execute(&self.pool)
        .await?;

        Ok(Session {
            id: session_id,
            user_id: user_id.to_string(),
            refresh_token_jti: refresh_token_jti.to_string(),
            device_user_agent,
            ip_address: ip_address.to_string(),
            created_at: now,
            last_activity_at: now,
            expires_at,
            idle_timeout_seconds: idle_timeout,
            max_lifetime_seconds: max_lifetime,
            revoked_at: None,
        })
    }

    /// Revoke the oldest active sessions until the user is under `limit`.
    async fn enforce_concurrent_limit(&self, user_id: &str, limit: i64) -> Result<()> {
        if limit <= 0 {
            return Ok(());
        }
        let now = Utc::now();
        let active: Vec<(String,)> = sqlx::query_as(
            r"
            SELECT id FROM sessions
            WHERE user_id = ? AND revoked_at IS NULL AND expires_at > ?
            ORDER BY last_activity_at ASC
            ",
        )
        .bind(user_id)
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        // We are about to insert one more session, so keep `limit - 1`.
        let allowed_existing = (limit - 1).max(0) as usize;
        if active.len() > allowed_existing {
            let to_revoke = active.len() - allowed_existing;
            for (id,) in active.into_iter().take(to_revoke) {
                self.revoke_session(&id).await?;
            }
        }
        Ok(())
    }

    /// Verify that a presented device key matches the one bound to the session.
    /// Returns `Mismatch` when the session is bound but the presented key is
    /// absent or different, which callers must treat as a stolen token.
    pub async fn verify_device_binding(
        &self,
        session_id: &str,
        presented_device_key: Option<&str>,
    ) -> Result<DeviceBinding> {
        let row: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT user_id, device_key FROM sessions WHERE id = ?")
                .bind(session_id)
                .fetch_optional(&self.pool)
                .await?;

        let (user_id, stored) = match row {
            Some(r) => r,
            None => return Ok(DeviceBinding::Mismatch),
        };

        let bound = match stored {
            Some(enc) => self.user_keys.decrypt_field(&user_id, &enc).await?,
            None => return Ok(DeviceBinding::Match),
        };

        match presented_device_key {
            Some(key) if key == bound => Ok(DeviceBinding::Match),
            _ => Ok(DeviceBinding::Mismatch),
        }
    }

    /// Detect whether a session is being used from a new IP or country.
    pub async fn detect_anomaly(
        &self,
        session_id: &str,
        current_ip: &str,
        current_country: Option<&str>,
    ) -> Result<Option<SessionAnomaly>> {
        let row: Option<(String, String, Option<String>)> =
            sqlx::query_as("SELECT user_id, ip_address, country FROM sessions WHERE id = ?")
                .bind(session_id)
                .fetch_optional(&self.pool)
                .await?;

        let (user_id, stored_ip, stored_country) = match row {
            Some(r) => r,
            None => return Ok(None),
        };

        let bound_ip = self.user_keys.decrypt_field(&user_id, &stored_ip).await?;
        let ip_changed = bound_ip != current_ip;
        let country_changed = match (stored_country.as_deref(), current_country) {
            (Some(a), Some(b)) => a != b,
            _ => false,
        };

        let anomaly = SessionAnomaly {
            ip_changed,
            country_changed,
        };
        Ok(if anomaly.is_anomalous() { Some(anomaly) } else { None })
    }

    /// Get active session (not expired, not revoked, not idle)
    pub async fn get_active_session(&self, session_id: &str) -> Result<Option<Session>> {
        let now = Utc::now();

        let session = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<String>,
                String,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
                i64,
                i64,
                Option<chrono::DateTime<chrono::Utc>>,
            ),
        >(
            r"
            SELECT id, user_id, refresh_token_jti, device_user_agent, ip_address,
                   created_at, last_activity_at, expires_at, idle_timeout_seconds,
                   max_lifetime_seconds, revoked_at
            FROM sessions
            WHERE id = ?
            ",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((
            id,
            user_id,
            jti,
            device,
            ip,
            created,
            last_activity,
            expires,
            idle,
            max_lifetime,
            revoked,
        )) = session
        {
            // Check if revoked
            if revoked.is_some() {
                return Ok(None);
            }

            // Check if absolute lifetime expired
            if now > expires {
                return Ok(None);
            }

            // Check if idle timeout exceeded
            let idle_limit = last_activity
                .checked_add_signed(Duration::seconds(idle))
                .ok_or_else(|| anyhow!("Invalid timestamp"))?;
            if now > idle_limit {
                return Ok(None);
            }

            let ip_address = self.user_keys.decrypt_field(&user_id, &ip).await?;
            let device_user_agent =
                self.user_keys.decrypt_opt(&user_id, device).await?;

            return Ok(Some(Session {
                id,
                user_id,
                refresh_token_jti: jti,
                device_user_agent,
                ip_address,
                created_at: created,
                last_activity_at: last_activity,
                expires_at: expires,
                idle_timeout_seconds: idle,
                max_lifetime_seconds: max_lifetime,
                revoked_at: revoked,
            }));
        }

        Ok(None)
    }

    /// Update session's last activity timestamp
    pub async fn touch_session(&self, session_id: &str) -> Result<()> {
        let now = Utc::now();

        sqlx::query("UPDATE sessions SET last_activity_at = ? WHERE id = ?")
            .bind(now)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Revoke a specific session
    pub async fn revoke_session(&self, session_id: &str) -> Result<()> {
        let now = Utc::now();

        sqlx::query("UPDATE sessions SET revoked_at = ? WHERE id = ?")
            .bind(now)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// List all active sessions for a user (for device management)
    pub async fn list_active_sessions(&self, user_id: &str) -> Result<Vec<SessionInfo>> {
        let now = Utc::now();

        let sessions = sqlx::query_as::<
            _,
            (
                String,
                Option<String>,
                String,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            r"
            SELECT id, device_user_agent, ip_address, created_at, last_activity_at, expires_at
            FROM sessions
            WHERE user_id = ? AND revoked_at IS NULL AND expires_at > ?
            ORDER BY last_activity_at DESC
            ",
        )
        .bind(user_id)
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        let result: Vec<SessionInfo> = sessions
            .into_iter()
            .map(
                |(id, device, ip, created, last_activity, expires)| SessionInfo {
                    id,
                    device_user_agent: device,
                    ip_address: ip,
                    created_at: created,
                    last_activity_at: last_activity,
                    expires_at: expires,
                    is_current: false,
                },
            )
            .collect();

        // Decrypt PII fields for display. Done after the query so a single
        // bad row cannot fail the whole listing.
        let mut out = Vec::with_capacity(result.len());
        for info in result {
            let ip_address = self
                .user_keys
                .decrypt_field(user_id, &info.ip_address)
                .await
                .unwrap_or_else(|_| info.ip_address.clone());
            let device_user_agent = match &info.device_user_agent {
                Some(ua) => self
                    .user_keys
                    .decrypt_field(user_id, ua)
                    .await
                    .ok()
                    .or_else(|| info.device_user_agent.clone()),
                None => None,
            };
            out.push(SessionInfo {
                id: info.id,
                device_user_agent,
                ip_address,
                created_at: info.created_at,
                last_activity_at: info.last_activity_at,
                expires_at: info.expires_at,
                is_current: info.is_current,
            });
        }

        Ok(out)
    }

    /// Revoke all sessions except the current one
    pub async fn revoke_all_other_sessions(
        &self,
        user_id: &str,
        current_session_id: &str,
    ) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            "UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND id != ? AND revoked_at IS NULL",
        )
        .bind(now)
        .bind(user_id)
        .bind(current_session_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Revoke all sessions for a user (logout everywhere)
    pub async fn revoke_all_sessions(&self, user_id: &str) -> Result<()> {
        let now = Utc::now();

        sqlx::query("UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL")
            .bind(now)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get session by refresh token JTI
    pub async fn get_session_by_jti(&self, jti: &str) -> Result<Option<Session>> {
        let session = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<String>,
                String,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
                i64,
                i64,
                Option<chrono::DateTime<chrono::Utc>>,
            ),
        >(
            r"
            SELECT id, user_id, refresh_token_jti, device_user_agent, ip_address,
                   created_at, last_activity_at, expires_at, idle_timeout_seconds,
                   max_lifetime_seconds, revoked_at
            FROM sessions
            WHERE refresh_token_jti = ?
            ",
        )
        .bind(jti)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((
            id,
            user_id,
            jti,
            device,
            ip,
            created,
            last_activity,
            expires,
            idle,
            max_lifetime,
            revoked,
        )) = session
        {
            let ip_address = self.user_keys.decrypt_field(&user_id, &ip).await?;
            let device_user_agent = self.user_keys.decrypt_opt(&user_id, device).await?;

            return Ok(Some(Session {
                id,
                user_id,
                refresh_token_jti: jti,
                device_user_agent,
                ip_address,
                created_at: created,
                last_activity_at: last_activity,
                expires_at: expires,
                idle_timeout_seconds: idle,
                max_lifetime_seconds: max_lifetime,
                revoked_at: revoked,
            }));
        }

        Ok(None)
    }
}
