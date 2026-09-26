use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

const DEFAULT_IDLE_TIMEOUT_SECONDS: i64 = 3600; // 1 hour
const DEFAULT_MAX_LIFETIME_SECONDS: i64 = 604800; // 7 days

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

pub struct SessionService {
    pool: SqlitePool,
}

impl SessionService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new session with device tracking
    pub async fn create_session(
        &self,
        user_id: &str,
        refresh_token_jti: &str,
        device_user_agent: Option<String>,
        ip_address: &str,
        idle_timeout_seconds: Option<i64>,
        max_lifetime_seconds: Option<i64>,
    ) -> Result<Session> {
        let session_id = Uuid::new_v4().to_string();
        let idle_timeout = idle_timeout_seconds.unwrap_or(DEFAULT_IDLE_TIMEOUT_SECONDS);
        let max_lifetime = max_lifetime_seconds.unwrap_or(DEFAULT_MAX_LIFETIME_SECONDS);

        let now = Utc::now();
        let expires_at = now
            .checked_add_signed(Duration::seconds(max_lifetime))
            .ok_or_else(|| anyhow!("Invalid timestamp"))?;

        sqlx::query(
            r"
            INSERT INTO sessions (
                id, user_id, refresh_token_jti, device_user_agent, ip_address,
                created_at, last_activity_at, expires_at, idle_timeout_seconds,
                max_lifetime_seconds
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&session_id)
        .bind(user_id)
        .bind(refresh_token_jti)
        .bind(&device_user_agent)
        .bind(ip_address)
        .bind(now)
        .bind(now)
        .bind(expires_at)
        .bind(idle_timeout)
        .bind(max_lifetime)
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

    /// Get active session (not expired, not revoked, not idle)
    pub async fn get_active_session(&self, session_id: &str) -> Result<Option<Session>> {
        let now = Utc::now();

        let session = sqlx::query_as::<_, (
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
        )>(
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

        if let Some((id, user_id, jti, device, ip, created, last_activity, expires, idle, max_lifetime, revoked)) =
            session
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

            return Ok(Some(Session {
                id,
                user_id,
                refresh_token_jti: jti,
                device_user_agent: device,
                ip_address: ip,
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

        let sessions = sqlx::query_as::<_, (
            String,
            Option<String>,
            String,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        )>(
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
            .map(|(id, device, ip, created, last_activity, expires)| {
                SessionInfo {
                    id,
                    device_user_agent: device,
                    ip_address: ip,
                    created_at: created,
                    last_activity_at: last_activity,
                    expires_at: expires,
                    is_current: false,
                }
            })
            .collect();

        Ok(result)
    }

    /// Revoke all sessions except the current one
    pub async fn revoke_all_other_sessions(&self, user_id: &str, current_session_id: &str) -> Result<()> {
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
        let session = sqlx::query_as::<_, (
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
        )>(
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

        if let Some((id, user_id, jti, device, ip, created, last_activity, expires, idle, max_lifetime, revoked)) =
            session
        {
            return Ok(Some(Session {
                id,
                user_id,
                refresh_token_jti: jti,
                device_user_agent: device,
                ip_address: ip,
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
