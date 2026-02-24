use reqwest::header::HeaderMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::time::Instant;

const DEFAULT_RETRY_AFTER_SECONDS: u64 = 5;

#[derive(Debug, Clone)]
pub struct RpcRateLimitConfig {
    pub requests_per_minute: f64,
    pub burst_size: f64,
    pub queue_size: usize,
}

impl Default for RpcRateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 90.0,
            burst_size: 10.0,
            queue_size: 100,
        }
    }
}

impl RpcRateLimitConfig {
    pub fn from_env() -> Self {
        let default = Self::default();

        let requests_per_minute = std::env::var("RPC_RATE_LIMIT_REQUESTS_PER_MINUTE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| *v > 0.0)
            .unwrap_or(default.requests_per_minute);

        let burst_size = std::env::var("RPC_RATE_LIMIT_BURST_SIZE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| *v > 0.0)
            .unwrap_or(default.burst_size);

        let queue_size = std::env::var("RPC_RATE_LIMIT_QUEUE_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(default.queue_size);

        Self {
            requests_per_minute,
            burst_size,
            queue_size,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RpcRateLimitMetrics {
    pub total_requests: u64,
    pub throttled_requests: u64,
    pub rejected_requests: u64,
    pub rate_limited_responses: u64,
}

#[derive(Debug)]
pub enum RpcRateLimitError {
    QueueFull,
}

impl fmt::Display for RpcRateLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcRateLimitError::QueueFull => write!(f, "rate limiter queue is full"),
        }
    }
}

impl std::error::Error for RpcRateLimitError {}

#[derive(Debug)]
struct TokenBucketState {
    tokens: f64,
    capacity: f64,
    refill_rate_per_second: f64,
    last_refill: Instant,
}

#[derive(Clone)]
pub struct RpcRateLimiter {
    state: Arc<Mutex<TokenBucketState>>,
    queue: Arc<Semaphore>,
    total_requests: Arc<AtomicU64>,
    throttled_requests: Arc<AtomicU64>,
    rejected_requests: Arc<AtomicU64>,
    rate_limited_responses: Arc<AtomicU64>,
}

pub struct QueuePermit {
    _permit: OwnedSemaphorePermit,
}

impl RpcRateLimiter {
    pub fn new(config: RpcRateLimitConfig) -> Self {
        let capacity = config.burst_size.max(1.0);
        let refill_rate_per_second = (config.requests_per_minute / 60.0).max(0.01);

        Self {
            state: Arc::new(Mutex::new(TokenBucketState {
                tokens: capacity,
                capacity,
                refill_rate_per_second,
                last_refill: Instant::now(),
            })),
            queue: Arc::new(Semaphore::new(config.queue_size)),
            total_requests: Arc::new(AtomicU64::new(0)),
            throttled_requests: Arc::new(AtomicU64::new(0)),
            rejected_requests: Arc::new(AtomicU64::new(0)),
            rate_limited_responses: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn acquire(&self) -> Result<QueuePermit, RpcRateLimitError> {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        let permit = self.queue.clone().try_acquire_owned().map_err(|_| {
            self.rejected_requests.fetch_add(1, Ordering::Relaxed);
            RpcRateLimitError::QueueFull
        })?;

        loop {
            let wait_time = {
                let mut state = self.state.lock().await;
                Self::refill_locked(&mut state);

                if state.tokens >= 1.0 {
                    state.tokens -= 1.0;
                    Duration::from_secs(0)
                } else {
                    self.throttled_requests.fetch_add(1, Ordering::Relaxed);
                    let seconds = ((1.0 - state.tokens) / state.refill_rate_per_second).max(0.001);
                    Duration::from_secs_f64(seconds)
                }
            };

            if wait_time.is_zero() {
                return Ok(QueuePermit { _permit: permit });
            }

            tokio::time::sleep(wait_time).await;
        }
    }

    pub async fn observe_headers(&self, headers: &HeaderMap) {
        let limit = parse_u64_header(headers, "x-ratelimit-limit");
        let remaining = parse_u64_header(headers, "x-ratelimit-remaining");

        if limit.is_none() && remaining.is_none() {
            return;
        }

        let mut state = self.state.lock().await;
        Self::refill_locked(&mut state);

        if let Some(limit) = limit {
            let limit_f = limit as f64;
            if limit_f > 0.0 {
                state.refill_rate_per_second = (limit_f / 60.0).max(0.01);
                state.capacity = state.capacity.max(limit_f.min(600.0));
                if state.tokens > state.capacity {
                    state.tokens = state.capacity;
                }
            }
        }

        if let Some(remaining) = remaining {
            state.tokens = (remaining as f64).min(state.capacity);
        }
    }

    pub async fn on_rate_limited(&self, headers: &HeaderMap) {
        self.rate_limited_responses.fetch_add(1, Ordering::Relaxed);

        let wait_seconds =
            parse_retry_after_seconds(headers).unwrap_or(DEFAULT_RETRY_AFTER_SECONDS);

        {
            let mut state = self.state.lock().await;
            state.tokens = 0.0;
            state.last_refill = Instant::now();
        }

        tokio::time::sleep(Duration::from_secs(wait_seconds)).await;
    }

    pub fn metrics(&self) -> RpcRateLimitMetrics {
        RpcRateLimitMetrics {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            throttled_requests: self.throttled_requests.load(Ordering::Relaxed),
            rejected_requests: self.rejected_requests.load(Ordering::Relaxed),
            rate_limited_responses: self.rate_limited_responses.load(Ordering::Relaxed),
        }
    }

    fn refill_locked(state: &mut TokenBucketState) {
        let elapsed = state.last_refill.elapsed().as_secs_f64();
        if elapsed <= 0.0 {
            return;
        }

        state.tokens = (state.tokens + elapsed * state.refill_rate_per_second).min(state.capacity);
        state.last_refill = Instant::now();
    }
}

fn parse_u64_header(headers: &HeaderMap, name: &str) -> Option<u64> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
}

fn parse_retry_after_seconds(headers: &HeaderMap) -> Option<u64> {
    let value = headers.get("retry-after")?.to_str().ok()?.trim();

    if let Ok(seconds) = value.parse::<u64>() {
        return Some(seconds);
    }

    let timestamp = chrono::DateTime::parse_from_rfc2822(value).ok()?;
    let now = chrono::Utc::now();
    let retry_at = timestamp.with_timezone(&chrono::Utc);
    if retry_at <= now {
        Some(0)
    } else {
        Some((retry_at - now).num_seconds() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;
    use std::time::SystemTime;

    #[tokio::test]
    async fn token_bucket_refills_and_waits() {
        let limiter = RpcRateLimiter::new(RpcRateLimitConfig {
            requests_per_minute: 60.0,
            burst_size: 1.0,
            queue_size: 10,
        });

        limiter.acquire().await.unwrap();
        let start = Instant::now();
        limiter.acquire().await.unwrap();
        assert!(start.elapsed() >= Duration::from_millis(850));
    }

    #[tokio::test]
    async fn queue_full_rejects_request() {
        let limiter = RpcRateLimiter::new(RpcRateLimitConfig {
            requests_per_minute: 60.0,
            burst_size: 0.1,
            queue_size: 1,
        });

        let limiter_clone = limiter.clone();
        let holder = tokio::spawn(async move {
            let _permit = limiter_clone.acquire().await.unwrap();
            tokio::time::sleep(Duration::from_millis(200)).await;
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        let err = limiter.acquire().await.err();
        assert!(matches!(err, Some(RpcRateLimitError::QueueFull)));

        holder.await.unwrap();
    }

    #[tokio::test]
    async fn observes_rate_limit_headers() {
        let limiter = RpcRateLimiter::new(RpcRateLimitConfig::default());

        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-limit", HeaderValue::from_static("120"));
        headers.insert("x-ratelimit-remaining", HeaderValue::from_static("3"));
        limiter.observe_headers(&headers).await;

        let _a = limiter.acquire().await.unwrap();
        let _b = limiter.acquire().await.unwrap();
        let _c = limiter.acquire().await.unwrap();
        let start = Instant::now();
        limiter.acquire().await.unwrap();

        assert!(start.elapsed() >= Duration::from_millis(450));
    }

    #[tokio::test]
    async fn rate_limited_updates_metrics_and_respects_retry_after() {
        let limiter = RpcRateLimiter::new(RpcRateLimitConfig::default());
        let mut headers = HeaderMap::new();
        headers.insert("retry-after", HeaderValue::from_static("0"));

        limiter.on_rate_limited(&headers).await;

        let metrics = limiter.metrics();
        assert_eq!(metrics.rate_limited_responses, 1);
    }

    #[tokio::test]
    async fn queue_full_increments_rejected_metric() {
        let limiter = RpcRateLimiter::new(RpcRateLimitConfig {
            requests_per_minute: 60.0,
            burst_size: 0.1,
            queue_size: 1,
        });

        let limiter_clone = limiter.clone();
        let holder = tokio::spawn(async move {
            let _permit = limiter_clone.acquire().await.unwrap();
            tokio::time::sleep(Duration::from_millis(200)).await;
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        let _ = limiter.acquire().await.err();

        let metrics = limiter.metrics();
        assert_eq!(metrics.rejected_requests, 1);

        holder.await.unwrap();
    }

    #[test]
    fn retry_after_parses_http_date_format() {
        let retry_at =
            chrono::DateTime::<chrono::Utc>::from(SystemTime::now() + Duration::from_secs(2));
        let mut headers = HeaderMap::new();
        headers.insert(
            "retry-after",
            HeaderValue::from_str(&retry_at.to_rfc2822()).unwrap(),
        );

        let parsed = parse_retry_after_seconds(&headers).unwrap();
        assert!(parsed <= 2);
    }
}
