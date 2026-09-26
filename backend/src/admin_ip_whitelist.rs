use anyhow::{anyhow, Result};
use ipnetwork::IpNetwork;
use sqlx::SqlitePool;
use std::net::IpAddr;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct WhitelistEntry {
    pub id: String,
    pub ip_or_cidr: String,
    pub description: Option<String>,
    pub added_by_user_id: Option<String>,
    pub added_at: chrono::DateTime<chrono::Utc>,
}

pub struct IpWhitelistService {
    pool: SqlitePool,
}

impl IpWhitelistService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Parse and validate IP or CIDR notation
    fn validate_ip_or_cidr(ip_or_cidr: &str) -> Result<()> {
        // Try parsing as CIDR network first
        if IpNetwork::from_str(ip_or_cidr).is_ok() {
            return Ok(());
        }

        // Try parsing as standalone IP
        if IpAddr::from_str(ip_or_cidr).is_ok() {
            return Ok(());
        }

        Err(anyhow!("Invalid IP address or CIDR notation: {}", ip_or_cidr))
    }

    /// Add IP/CIDR to whitelist
    pub async fn add_to_whitelist(
        &self,
        ip_or_cidr: &str,
        description: Option<String>,
        added_by_user_id: Option<&str>,
    ) -> Result<WhitelistEntry> {
        Self::validate_ip_or_cidr(ip_or_cidr)?;

        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        sqlx::query(
            r"
            INSERT INTO admin_ip_whitelist (id, ip_or_cidr, description, added_by_user_id, added_at)
            VALUES (?, ?, ?, ?, ?)
            ",
        )
        .bind(&id)
        .bind(ip_or_cidr)
        .bind(&description)
        .bind(added_by_user_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(WhitelistEntry {
            id,
            ip_or_cidr: ip_or_cidr.to_string(),
            description,
            added_by_user_id: added_by_user_id.map(|s| s.to_string()),
            added_at: now,
        })
    }

    /// Remove IP/CIDR from whitelist
    pub async fn remove_from_whitelist(&self, ip_or_cidr: &str) -> Result<()> {
        sqlx::query("DELETE FROM admin_ip_whitelist WHERE ip_or_cidr = ?")
            .bind(ip_or_cidr)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Check if IP is whitelisted
    pub async fn is_whitelisted(&self, ip_address: &str) -> Result<bool> {
        // Parse incoming IP
        let incoming_ip = IpAddr::from_str(ip_address)
            .map_err(|_| anyhow!("Invalid IP address: {}", ip_address))?;

        // Get all whitelist entries
        let entries = sqlx::query_scalar::<_, String>("SELECT ip_or_cidr FROM admin_ip_whitelist")
            .fetch_all(&self.pool)
            .await?;

        // If whitelist is empty, deny access (fail-closed default)
        if entries.is_empty() {
            return Ok(false);
        }

        // Check if IP matches any entry
        for entry in entries {
            // Try parsing as CIDR first
            if let Ok(network) = IpNetwork::from_str(&entry) {
                if network.contains(incoming_ip) {
                    return Ok(true);
                }
            } else if let Ok(ip) = IpAddr::from_str(&entry) {
                // Try as standalone IP
                if ip == incoming_ip {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Get all whitelist entries
    pub async fn get_all_entries(&self) -> Result<Vec<WhitelistEntry>> {
        let entries = sqlx::query_as::<_, (
            String,
            String,
            Option<String>,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        )>(
            r"
            SELECT id, ip_or_cidr, description, added_by_user_id, added_at
            FROM admin_ip_whitelist
            ORDER BY added_at DESC
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(entries
            .into_iter()
            .map(|(id, ip_or_cidr, description, added_by_user_id, added_at)| WhitelistEntry {
                id,
                ip_or_cidr,
                description,
                added_by_user_id,
                added_at,
            })
            .collect())
    }

    /// Update whitelist entry description
    pub async fn update_entry(
        &self,
        ip_or_cidr: &str,
        description: Option<String>,
    ) -> Result<()> {
        sqlx::query("UPDATE admin_ip_whitelist SET description = ? WHERE ip_or_cidr = ?")
            .bind(description)
            .bind(ip_or_cidr)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Count whitelist entries
    pub async fn count_entries(&self) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM admin_ip_whitelist")
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }
}
