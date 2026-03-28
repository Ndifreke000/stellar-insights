use chrono::{Datelike, Timelike, Utc};
use std::sync::Arc;
use tokio::time::{interval, Duration};

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

use crate::cache::CacheManager;
use crate::email::report::{generate_html_report, AnchorSummary, CorridorSummary, DigestReport};
use crate::email::service::EmailService;
use crate::rpc::StellarRpcClient;

pub type EmailDigest = DigestReport;

pub struct DigestScheduler {
    email_service: Arc<EmailService>,
    cache: Arc<CacheManager>,
    rpc_client: Arc<StellarRpcClient>,
    recipients: Vec<String>,
}

#[cfg(test)]
static GENERATE_DAILY_DIGEST_CALLS: AtomicU64 = AtomicU64::new(0);

impl DigestScheduler {
    #[must_use]
    pub const fn new(
        email_service: Arc<EmailService>,
        cache: Arc<CacheManager>,
        rpc_client: Arc<StellarRpcClient>,
        recipients: Vec<String>,
    ) -> Self {
        Self {
            email_service,
            cache,
            rpc_client,
            recipients,
        }
    }

    pub async fn start(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(3600)); // Check hourly

        loop {
            ticker.tick().await;
            let now = Utc::now();

            // Weekly: Monday at 9 AM
            if now.weekday().num_days_from_monday() == 0 && now.hour() == 9 {
                if let Err(e) = self.send_digest("Weekly").await {
                    tracing::error!("Failed to send weekly digest: {}", e);
                }
            }

            // Monthly: 1st of month at 9 AM
            if now.day() == 1 && now.hour() == 9 {
                if let Err(e) = self.send_digest("Monthly").await {
                    tracing::error!("Failed to send monthly digest: {}", e);
                }
            }
        }
    }

    pub async fn send_digest(&self, period: &str) -> anyhow::Result<()> {
        let cache_key = format!("daily_digest:{}", period);

        let report = if let Some(cached) = self.cache.get::<DigestReport>(&cache_key).await? {
            cached
        } else {
            let fresh = self.generate_report(period).await?;
            let _ = self.cache.set(&cache_key, &fresh, 3600).await;
            fresh
        };

        let html = generate_html_report(&report);

        for recipient in &self.recipients {
            self.email_service.send_html(
                recipient,
                &format!("Stellar Insights - {period} Performance Report"),
                &html,
            )?;
        }

        tracing::info!(
            "Sent {} digest to {} recipients",
            period,
            self.recipients.len()
        );
        Ok(())
    }

    pub async fn generate_daily_digest(&self) -> anyhow::Result<EmailDigest> {
        let cache_key = "daily_digest:latest";

        if let Some(cached) = self.cache.get::<EmailDigest>(cache_key).await? {
            return Ok(cached);
        }

        #[cfg(test)]
        {
            GENERATE_DAILY_DIGEST_CALLS.fetch_add(1, Ordering::Relaxed);
        }

        let digest = self.generate_report("Daily").await?;
        let _ = self.cache.set(cache_key, &digest, 3600).await;
        Ok(digest)
    }

    async fn generate_report(&self, period: &str) -> anyhow::Result<DigestReport> {
        let payments = self
            .rpc_client
            .fetch_payments(500, None)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut corridor_map = std::collections::HashMap::new();
        for payment in &payments {
            let key = format!(
                "{}:{}->XLM:native",
                payment.get_asset_code().as_deref().unwrap_or("XLM"),
                payment.get_asset_issuer().as_deref().unwrap_or("native")
            );
            corridor_map
                .entry(key)
                .or_insert_with(Vec::new)
                .push(payment);
        }

        let mut corridors: Vec<CorridorSummary> = corridor_map
            .iter()
            .map(|(id, payments)| {
                let volume: f64 = payments
                    .iter()
                    .filter_map(|p| p.get_amount().parse::<f64>().ok())
                    .sum();
                CorridorSummary {
                    id: id.clone(),
                    success_rate: 100.0,
                    volume_usd: volume,
                    avg_latency_ms: 450.0,
                    change_pct: 5.2,
                }
            })
            .collect();

        corridors.sort_by(|a, b| b.volume_usd.partial_cmp(&a.volume_usd).unwrap());
        corridors.truncate(10);

        let total_volume: f64 = corridors.iter().map(|c| c.volume_usd).sum();
        let avg_success_rate =
            corridors.iter().map(|c| c.success_rate).sum::<f64>() / corridors.len() as f64;

        Ok(DigestReport {
            period: period.to_string(),
            top_corridors: corridors,
            top_anchors: vec![AnchorSummary {
                name: "Circle USDC".to_string(),
                success_rate: 99.5,
                total_transactions: 15420,
                volume_usd: 2_500_000.0,
            }],
            total_volume,
            avg_success_rate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{CacheConfig, CacheManager};

    #[tokio::test]
    async fn test_digest_caching() {
        GENERATE_DAILY_DIGEST_CALLS.store(0, Ordering::Relaxed);

        let email_service = Arc::new(EmailService::new(
            "localhost".to_string(),
            "user@example.com".to_string(),
            "pass".to_string(),
        ));
        let cache = Arc::new(CacheManager::new_in_memory_for_tests(CacheConfig::default()));
        let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));

        let scheduler = DigestScheduler::new(email_service, cache, rpc_client, vec![]);

        let _first = scheduler.generate_daily_digest().await.unwrap();
        assert_eq!(
            GENERATE_DAILY_DIGEST_CALLS.load(Ordering::Relaxed),
            1,
            "first call should build digest"
        );

        let _second = scheduler.generate_daily_digest().await.unwrap();
        assert_eq!(
            GENERATE_DAILY_DIGEST_CALLS.load(Ordering::Relaxed),
            1,
            "second call should use cached digest"
        );
    }
}
