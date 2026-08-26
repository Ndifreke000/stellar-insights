use anyhow::{anyhow, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

// TOTP parameters: 30-second time step, 6-digit codes
const TOTP_TIME_STEP_SECONDS: u64 = 30;
const TOTP_DIGIT_COUNT: usize = 6;
const BACKUP_CODES_COUNT: usize = 10;

#[derive(Debug, Clone)]
pub struct TwoFASecret {
    pub user_id: String,
    pub encrypted_secret: String,
    pub is_enabled: bool,
    pub enrolled_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct BackupCode {
    pub code: String,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct TwoFAService {
    pool: SqlitePool,
    crypto: crate::crypto::CryptoService,
}

impl TwoFAService {
    pub fn new(pool: SqlitePool, crypto: crate::crypto::CryptoService) -> Self {
        Self { pool, crypto }
    }

    /// Generate TOTP secret and return otpauth URI for QR code + raw secret
    pub fn generate_totp_secret(&self, user_id: &str, username: &str) -> Result<(String, String)> {
        use base32::{Alphabet, encode};

        // Generate 20-byte random secret (standard for TOTP)
        let secret_bytes = uuid::Uuid::new_v4().as_bytes();
        let secret_base32 = encode(Alphabet::RFC4648 { padding: false }, secret_bytes);

        // Generate otpauth URI for QR code
        let otpauth_uri = format!(
            "otpauth://totp/stellar-insights:{}?secret={}&issuer=stellar-insights&digits={}",
            username, secret_base32, TOTP_DIGIT_COUNT
        );

        Ok((otpauth_uri, secret_base32))
    }

    /// Enroll user in 2FA by storing encrypted TOTP secret
    pub async fn enroll_2fa(&self, user_id: &str, totp_secret: &str) -> Result<()> {
        // Encrypt the secret before storing
        let encrypted_secret = self.crypto.encrypt(totp_secret)?;

        sqlx::query(
            r"
            INSERT OR REPLACE INTO user_2fa_secrets (user_id, encrypted_secret, is_enabled, enrolled_at)
            VALUES (?, ?, FALSE, CURRENT_TIMESTAMP)
            ",
        )
        .bind(user_id)
        .bind(&encrypted_secret)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Activate 2FA after successful TOTP code verification
    pub async fn activate_2fa(&self, user_id: &str) -> Result<()> {
        sqlx::query("UPDATE user_2fa_secrets SET is_enabled = TRUE WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Generate and store backup codes
    pub async fn generate_backup_codes(&self, user_id: &str) -> Result<Vec<String>> {
        // Delete existing codes
        sqlx::query("DELETE FROM user_2fa_backup_codes WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        let mut codes = Vec::with_capacity(BACKUP_CODES_COUNT);

        for _ in 0..BACKUP_CODES_COUNT {
            let code = format!("{:06}", uuid::Uuid::new_v4().as_u64_pair().0 % 1000000);
            let code_id = Uuid::new_v4().to_string();
            let hashed_code = format!("{:x}", sha2::Sha256::digest(code.as_bytes()));

            sqlx::query(
                r"
                INSERT INTO user_2fa_backup_codes (id, user_id, hashed_code, created_at)
                VALUES (?, ?, ?, CURRENT_TIMESTAMP)
                ",
            )
            .bind(&code_id)
            .bind(user_id)
            .bind(&hashed_code)
            .execute(&self.pool)
            .await?;

            codes.push(code);
        }

        // Update backup_codes_generated_at timestamp
        sqlx::query(
            "UPDATE user_2fa_secrets SET backup_codes_generated_at = CURRENT_TIMESTAMP WHERE user_id = ?",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(codes)
    }

    /// Check if user has 2FA enabled
    pub async fn is_2fa_enabled(&self, user_id: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT is_enabled FROM user_2fa_secrets WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.unwrap_or(false))
    }

    /// Get TOTP secret (decrypted)
    async fn get_totp_secret(&self, user_id: &str) -> Result<Option<String>> {
        let result = sqlx::query_scalar::<_, String>(
            "SELECT encrypted_secret FROM user_2fa_secrets WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(encrypted) = result {
            let decrypted = self.crypto.decrypt(&encrypted)?;
            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }

    /// Verify TOTP code (placeholder - requires totp library implementation)
    pub async fn verify_totp_code(&self, user_id: &str, code: &str) -> Result<bool> {
        if code.len() != TOTP_DIGIT_COUNT {
            return Ok(false);
        }

        // TODO: Implement TOTP verification using time-based algorithm
        // This requires adding a TOTP library (e.g., totp-lite or totp-rs) to Cargo.toml
        // For now, return false as placeholder
        let _ = (self.get_totp_secret(user_id).await, code);
        Ok(false)
    }

    /// Verify and consume a backup code
    pub async fn verify_backup_code(&self, user_id: &str, code: &str) -> Result<bool> {
        let hashed_code = format!("{:x}", sha2::Sha256::digest(code.as_bytes()));

        let result = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
            "SELECT used_at FROM user_2fa_backup_codes WHERE user_id = ? AND hashed_code = ?",
        )
        .bind(user_id)
        .bind(&hashed_code)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(Some(Some(_))) => {
                // Code already used
                Ok(false)
            }
            Some(Some(None)) => {
                // Valid unused code - mark as used
                let now = Utc::now();
                sqlx::query(
                    "UPDATE user_2fa_backup_codes SET used_at = ? WHERE user_id = ? AND hashed_code = ?",
                )
                .bind(now)
                .bind(user_id)
                .bind(&hashed_code)
                .execute(&self.pool)
                .await?;

                Ok(true)
            }
            _ => {
                // Code not found
                Ok(false)
            }
        }
    }

    /// Disable 2FA for user
    pub async fn disable_2fa(&self, user_id: &str) -> Result<()> {
        sqlx::query("UPDATE user_2fa_secrets SET is_enabled = FALSE WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get unused backup codes count
    pub async fn get_unused_backup_codes_count(&self, user_id: &str) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_2fa_backup_codes WHERE user_id = ? AND used_at IS NULL",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}
