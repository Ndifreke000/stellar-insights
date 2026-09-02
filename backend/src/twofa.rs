use anyhow::Result;
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use sha2::Digest;
use sqlx::SqlitePool;
use uuid::Uuid;

type HmacSha1 = Hmac<Sha1>;

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

        // Generate a random secret (standard for TOTP)
        let secret_uuid = uuid::Uuid::new_v4();
        let secret_bytes = secret_uuid.as_bytes();
        let secret_base32 = encode(Alphabet::Rfc4648 { padding: false }, secret_bytes);

        // Generate otpauth URI for QR code
        let otpauth_uri = format!(
            "otpauth://totp/payraider:{}?secret={}&issuer=payraider&digits={}",
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
            let hashed_code = hex::encode(sha2::Sha256::digest(code.as_bytes()));

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

    /// Verify a TOTP code (RFC 6238) against the user's stored secret.
    ///
    /// Checks the current 30-second time step and one step on either side
    /// (±30s) to tolerate clock drift between server and authenticator app
    /// -- standard practice; without it, most real devices would fail
    /// verification intermittently even with the correct code.
    pub async fn verify_totp_code(&self, user_id: &str, code: &str) -> Result<bool> {
        if code.len() != TOTP_DIGIT_COUNT || !code.bytes().all(|b| b.is_ascii_digit()) {
            return Ok(false);
        }

        let Some(secret_base32) = self.get_totp_secret(user_id).await? else {
            return Ok(false);
        };

        let Some(secret_bytes) =
            base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret_base32)
        else {
            return Ok(false);
        };

        let current_step = (Utc::now().timestamp().max(0) as u64) / TOTP_TIME_STEP_SECONDS;

        let candidate_steps = [
            current_step.saturating_sub(1),
            current_step,
            current_step + 1,
        ];

        Ok(candidate_steps
            .iter()
            .any(|&step| totp_code_for_step(&secret_bytes, step) == code))
    }

    /// Verify and consume a backup code
    pub async fn verify_backup_code(&self, user_id: &str, code: &str) -> Result<bool> {
        let hashed_code = hex::encode(sha2::Sha256::digest(code.as_bytes()));

        let result = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
            "SELECT used_at FROM user_2fa_backup_codes WHERE user_id = ? AND hashed_code = ?",
        )
        .bind(user_id)
        .bind(&hashed_code)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(Some(_)) => {
                // Code already used
                Ok(false)
            }
            Some(None) => {
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

/// RFC 6238 TOTP code for one time step: HMAC-SHA1 the step counter (as an
/// 8-byte big-endian value) with the shared secret, then dynamically
/// truncate to a 6-digit code per RFC 4226 §5.3/5.4.
fn totp_code_for_step(secret: &[u8], step: u64) -> String {
    let mut mac =
        <HmacSha1 as KeyInit>::new_from_slice(secret).expect("HMAC accepts a key of any length");
    mac.update(&step.to_be_bytes());
    let digest = mac.finalize().into_bytes();

    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    let binary = ((u32::from(digest[offset]) & 0x7f) << 24)
        | (u32::from(digest[offset + 1]) << 16)
        | (u32::from(digest[offset + 2]) << 8)
        | u32::from(digest[offset + 3]);

    format!("{:06}", binary % 1_000_000)
}

#[cfg(test)]
mod totp_tests {
    use super::totp_code_for_step;

    /// RFC 6238 Appendix B test vector: secret "12345678901234567890"
    /// (ASCII), T=59s -> time step 1 (59 / 30 = 1), SHA1 8-digit code
    /// "94287082". Our TOTP_DIGIT_COUNT is 6, so we check the low 6 digits
    /// (binary % 10^6 == binary % 10^8 % 10^6, since 10^6 divides 10^8),
    /// i.e. "287082".
    #[test]
    fn matches_rfc6238_sha1_test_vector() {
        let secret = b"12345678901234567890";
        assert_eq!(totp_code_for_step(secret, 1), "287082");
    }

    /// RFC 6238 Appendix B, T=1111111109s -> step 37037036,
    /// 8-digit code "07081804" -> low 6 digits "081804".
    #[test]
    fn matches_rfc6238_sha1_test_vector_2() {
        let secret = b"12345678901234567890";
        assert_eq!(totp_code_for_step(secret, 37_037_036), "081804");
    }
}
