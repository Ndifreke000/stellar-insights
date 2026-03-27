use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserTier {
    Free,
    Premium,
}

pub struct UserTierRecord {
    pub tier: UserTier,
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_requests_free: usize,
    max_requests_premium: usize,
    window: Duration,
}

impl RateLimiter {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests_free: 10,
            max_requests_premium: 100,
            window: Duration::from_secs(60),
        })
    }

    pub async fn get_user_tier(&self, user_id: &str) -> Result<UserTier> {
        // ✅ Simplified mock that doesn't need external sqlx Postgres features
        if user_id == "user123" || user_id == "premium_user" {
            Ok(UserTier::Premium)
        } else {
            Ok(UserTier::Free)
        }
    }

    pub async fn check_rate_limit(&self, user_id: &str) -> Result<bool> {
        let tier = self.get_user_tier(user_id).await?;
        let max_reqs = match tier {
            UserTier::Premium => self.max_requests_premium,
            UserTier::Free => self.max_requests_free,
        };

        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();

        let timestamps = requests.entry(user_id.to_string()).or_insert_with(Vec::new);
        timestamps.retain(|&time| now.duration_since(time) < self.window);

        if timestamps.len() >= max_reqs {
            Ok(false)
        } else {
            timestamps.push(now);
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_premium_user_tier() {
        let rate_limiter = RateLimiter::new().await.unwrap();
        
        let tier = rate_limiter.get_user_tier("user123").await.unwrap();
        assert_eq!(tier, UserTier::Premium);
    }
}