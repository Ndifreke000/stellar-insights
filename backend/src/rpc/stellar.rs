//! Stellar RPC client core.
//!
//! The concrete [`StellarRpcClient`] is defined here together with its
//! constructors and the shared retry / rate-limit plumbing. Endpoint methods
//! live next to the protocol they speak:
//!
//! * [`crate::rpc::soroban`] - JSON-RPC (`getHealth`, `getLedgers`) client + models
//! * [`crate::rpc::horizon`] - Horizon REST client, pagination, pools + models
//!
//! Model types are re-exported from this module so existing
//! `crate::rpc::stellar::*` imports keep compiling for one release.

use crate::network::{NetworkConfig, StellarNetwork};
use crate::rpc::circuit_breaker::{new_rpc_circuit_breaker, rpc_circuit_breaker, CircuitBreaker};
use crate::rpc::config::{initial_backoff_from_env, max_backoff_from_env, max_retries_from_env};
use crate::rpc::error::{with_retry, RetryConfig, RpcError};
use crate::rpc::rate_limiter::{RpcRateLimitConfig, RpcRateLimitMetrics, RpcRateLimiter};
use anyhow::{anyhow, Result};
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

pub use super::horizon::models::*;
pub use super::soroban::models::*;

// ============================================================================
// RPC Pagination Security Limits
// ============================================================================
/// Maximum records that can be returned in a single RPC request (hard cap)
pub(in crate::rpc) const ABSOLUTE_MAX_RECORDS_PER_REQUEST: u32 = 500;
/// Default records per RPC request
pub(in crate::rpc) const DEFAULT_MAX_RECORDS_PER_REQUEST: u32 = 200;
/// Absolute maximum total records across all paginated requests (`DoS` protection)
pub(in crate::rpc) const ABSOLUTE_MAX_TOTAL_RECORDS: u32 = 5000;
/// Default total records limit for pagination
pub(in crate::rpc) const DEFAULT_MAX_TOTAL_RECORDS: u32 = 1000;
/// Minimum delay between pagination requests (`DoS` protection)
pub(in crate::rpc) const MIN_PAGINATION_DELAY_MS: u64 = 50;
/// Default delay between pagination requests
pub(in crate::rpc) const DEFAULT_PAGINATION_DELAY_MS: u64 = 100;

#[derive(Clone)]
pub struct StellarRpcClient {
    pub(in crate::rpc) client: Client,
    pub(in crate::rpc) rpc_url: String,
    pub(in crate::rpc) horizon_url: String,
    pub(in crate::rpc) network_config: NetworkConfig,
    pub(in crate::rpc) mock_mode: bool,
    pub(in crate::rpc) rate_limiter: RpcRateLimiter,
    pub(in crate::rpc) circuit_breaker: Arc<CircuitBreaker>,
    /// Maximum records per single request (default: 200)
    pub(in crate::rpc) max_records_per_request: u32,
    /// Maximum total records across all paginated requests (default: 10_000)
    pub max_total_records: u32,
    /// Delay between pagination requests in milliseconds (default: 100)
    pub(in crate::rpc) pagination_delay_ms: u64,
    /// Maximum retries for RPC calls
    pub(in crate::rpc) max_retries: u32,
    /// Initial backoff duration
    pub(in crate::rpc) initial_backoff: Duration,
    /// Maximum backoff duration
    pub(in crate::rpc) max_backoff: Duration,
}

// ============================================================================
// Helpers: map HTTP response to RpcError
// ============================================================================

pub(in crate::rpc) fn status_to_rpc_error(
    status: reqwest::StatusCode,
    body: String,
    retry_after_secs: Option<u64>,
) -> RpcError {
    if status.as_u16() == 429 {
        return RpcError::RateLimitError {
            retry_after: retry_after_secs.map(Duration::from_secs),
        };
    }
    if (500..=599).contains(&status.as_u16()) {
        return RpcError::ServerError {
            status: status.as_u16(),
            message: body,
        };
    }
    RpcError::ServerError {
        status: status.as_u16(),
        message: body,
    }
}

pub(in crate::rpc) async fn map_response_error(response: reqwest::Response) -> RpcError {
    let status = response.status();
    let retry_after = response
        .headers()
        .get("Retry-After")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    let body = read_error_text(response).await;
    status_to_rpc_error(status, body, retry_after)
}

// ============================================================================
// Implementation
// ============================================================================

impl StellarRpcClient {
    /// Create a new Stellar RPC client
    ///
    /// # Arguments
    /// * `rpc_url` - The Stellar RPC endpoint URL (e.g., `OnFinality`)
    /// * `horizon_url` - The Horizon API endpoint URL
    /// * `mock_mode` - If true, returns mock data instead of making real API calls
    ///
    /// # Panics
    /// Panics if the underlying HTTP client cannot be built (TLS backend
    /// initialisation failure). Use [`Self::try_new`] to handle that error.
    #[must_use]
    pub fn new(rpc_url: String, horizon_url: String, mock_mode: bool) -> Self {
        Self::try_new(rpc_url, horizon_url, mock_mode).expect("Failed to build HTTP client")
    }

    /// Fallible version of [`Self::new`]: returns an error instead of panicking
    /// when the HTTP client cannot be built.
    pub fn try_new(
        rpc_url: String,
        horizon_url: String,
        mock_mode: bool,
    ) -> Result<Self, reqwest::Error> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
        let rate_limiter = RpcRateLimiter::new(RpcRateLimitConfig::from_env());

        // Determine network based on URLs
        let network = if horizon_url.contains("testnet") {
            StellarNetwork::Testnet
        } else {
            StellarNetwork::Mainnet
        };

        let network_config = NetworkConfig::for_network(network);
        let circuit_breaker = if mock_mode {
            new_rpc_circuit_breaker()
        } else {
            rpc_circuit_breaker()
        };

        // Load pagination config from environment or use defaults with security limits
        let max_records_per_request = std::env::var("RPC_MAX_RECORDS_PER_REQUEST")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(DEFAULT_MAX_RECORDS_PER_REQUEST)
            .min(ABSOLUTE_MAX_RECORDS_PER_REQUEST);

        if max_records_per_request > ABSOLUTE_MAX_RECORDS_PER_REQUEST {
            warn!(
                "RPC_MAX_RECORDS_PER_REQUEST ({}) exceeds maximum ({}), capping to {}",
                std::env::var("RPC_MAX_RECORDS_PER_REQUEST").unwrap_or_default(),
                ABSOLUTE_MAX_RECORDS_PER_REQUEST,
                ABSOLUTE_MAX_RECORDS_PER_REQUEST
            );
        }

        let max_total_records = std::env::var("RPC_MAX_TOTAL_RECORDS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(DEFAULT_MAX_TOTAL_RECORDS)
            .min(ABSOLUTE_MAX_TOTAL_RECORDS);

        if std::env::var("RPC_MAX_TOTAL_RECORDS").is_ok()
            && max_total_records > ABSOLUTE_MAX_TOTAL_RECORDS
        {
            warn!(
                "RPC_MAX_TOTAL_RECORDS ({}) exceeds maximum ({}), capping to {} (DoS protection)",
                std::env::var("RPC_MAX_TOTAL_RECORDS").unwrap_or_default(),
                ABSOLUTE_MAX_TOTAL_RECORDS,
                ABSOLUTE_MAX_TOTAL_RECORDS
            );
        }

        let pagination_delay_ms = std::env::var("RPC_PAGINATION_DELAY_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_PAGINATION_DELAY_MS)
            .max(MIN_PAGINATION_DELAY_MS);

        if std::env::var("RPC_PAGINATION_DELAY_MS").is_ok()
            && pagination_delay_ms < MIN_PAGINATION_DELAY_MS
        {
            warn!(
                "RPC_PAGINATION_DELAY_MS ({}) is below minimum ({}), using minimum",
                std::env::var("RPC_PAGINATION_DELAY_MS").unwrap_or_default(),
                MIN_PAGINATION_DELAY_MS
            );
        }

        info!(
            "RPC pagination config: max_per_request={}, max_total={}, delay_ms={}",
            max_records_per_request, max_total_records, pagination_delay_ms
        );

        Ok(Self {
            client,
            rpc_url,
            horizon_url,
            network_config,
            mock_mode,
            rate_limiter,
            circuit_breaker,
            max_records_per_request,
            max_total_records,
            pagination_delay_ms,
            max_retries: max_retries_from_env(),
            initial_backoff: initial_backoff_from_env(),
            max_backoff: max_backoff_from_env(),
        })
    }

    /// Create a new client with network configuration
    ///
    /// # Panics
    /// Panics if the underlying HTTP client cannot be built (TLS backend
    /// initialisation failure). Use [`Self::try_new_with_network`] to handle
    /// that error.
    #[must_use]
    pub fn new_with_network(network: StellarNetwork, mock_mode: bool) -> Self {
        Self::try_new_with_network(network, mock_mode).expect("Failed to build HTTP client")
    }

    /// Fallible version of [`Self::new_with_network`]: returns an error instead
    /// of panicking when the HTTP client cannot be built.
    pub fn try_new_with_network(
        network: StellarNetwork,
        mock_mode: bool,
    ) -> Result<Self, reqwest::Error> {
        let network_config = NetworkConfig::for_network(network);

        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
        let rate_limiter = RpcRateLimiter::new(RpcRateLimitConfig::from_env());
        let circuit_breaker = if mock_mode {
            new_rpc_circuit_breaker()
        } else {
            rpc_circuit_breaker()
        };

        // Load pagination config from environment or use defaults with security limits
        let max_records_per_request = std::env::var("RPC_MAX_RECORDS_PER_REQUEST")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(DEFAULT_MAX_RECORDS_PER_REQUEST)
            .min(ABSOLUTE_MAX_RECORDS_PER_REQUEST);

        let max_total_records = std::env::var("RPC_MAX_TOTAL_RECORDS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(DEFAULT_MAX_TOTAL_RECORDS)
            .min(ABSOLUTE_MAX_TOTAL_RECORDS);

        let pagination_delay_ms = std::env::var("RPC_PAGINATION_DELAY_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_PAGINATION_DELAY_MS)
            .max(MIN_PAGINATION_DELAY_MS);

        Ok(Self {
            client,
            rpc_url: network_config.rpc_url.clone(),
            horizon_url: network_config.horizon_url.clone(),
            network_config,
            mock_mode,
            rate_limiter,
            circuit_breaker,
            max_records_per_request,
            max_total_records,
            pagination_delay_ms,
            max_retries: max_retries_from_env(),
            initial_backoff: initial_backoff_from_env(),
            max_backoff: max_backoff_from_env(),
        })
    }

    /// Create a new client with default `OnFinality` RPC and Horizon URLs (mainnet).
    ///
    /// When `mock_mode` is true the client never makes a real network call, so it
    /// defaults to testnet instead of mainnet — that avoids requiring the
    /// `STELLAR_RPC_URL_MAINNET`/`STELLAR_HORIZON_URL_MAINNET` production secrets
    /// just to construct a mock client in tests.
    #[must_use]
    pub fn new_with_defaults(mock_mode: bool) -> Self {
        let network = if mock_mode {
            StellarNetwork::Testnet
        } else {
            StellarNetwork::Mainnet
        };
        Self::new_with_network(network, mock_mode)
    }

    /// Replace the circuit breaker (e.g. one breaker per provider in a
    /// [`crate::rpc::provider_pool::FailoverRpcClient`]).
    #[must_use]
    pub fn with_circuit_breaker(mut self, breaker: Arc<CircuitBreaker>) -> Self {
        self.circuit_breaker = breaker;
        self
    }

    /// The circuit breaker guarding this client's requests.
    pub fn circuit_breaker(&self) -> Arc<CircuitBreaker> {
        self.circuit_breaker.clone()
    }

    /// Get the current network configuration
    #[must_use]
    pub const fn network_config(&self) -> &NetworkConfig {
        &self.network_config
    }

    /// Get the current network
    #[must_use]
    pub const fn network(&self) -> StellarNetwork {
        self.network_config.network
    }

    /// Check if this client is connected to mainnet
    #[must_use]
    pub fn is_mainnet(&self) -> bool {
        self.network_config.is_mainnet()
    }

    /// Check if this client is connected to testnet
    #[must_use]
    pub fn is_testnet(&self) -> bool {
        self.network_config.is_testnet()
    }

    /// Snapshot current outbound RPC/Horizon rate limiter metrics.
    #[must_use]
    pub fn rate_limit_metrics(&self) -> RpcRateLimitMetrics {
        self.rate_limiter.metrics()
    }

    pub(in crate::rpc) async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T, RpcError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, RpcError>>,
    {
        let retry_config = RetryConfig {
            max_attempts: self.max_retries + 1,
            base_delay_ms: self.initial_backoff.as_millis() as u64,
            max_delay_ms: self.max_backoff.as_millis() as u64,
        };

        with_retry(operation, retry_config, self.circuit_breaker.clone()).await
    }

    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Convert asset to query parameters for Horizon API
    pub(in crate::rpc) fn asset_to_query_params(prefix: &str, asset: &Asset) -> Result<String> {
        if asset.asset_type == "native" {
            Ok(format!("{prefix}_asset_type=native"))
        } else {
            let asset_code = asset.asset_code.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "Asset code missing for non-native asset type: {}",
                    asset.asset_type
                )
            })?;
            let asset_issuer = asset.asset_issuer.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "Asset issuer missing for non-native asset type: {}",
                    asset.asset_type
                )
            })?;
            Ok(format!(
                "{}_asset_type={}&{}_asset_code={}&{}_asset_issuer={}",
                prefix, asset.asset_type, prefix, asset_code, prefix, asset_issuer
            ))
        }
    }

    /// Retry a request with exponential backoff
    pub(in crate::rpc) async fn retry_request<F, Fut>(&self, request_fn: F) -> Result<reqwest::Response>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    {
        let retry_config = RetryConfig {
            max_attempts: self.max_retries + 1,
            base_delay_ms: self.initial_backoff.as_millis() as u64,
            max_delay_ms: self.max_backoff.as_millis() as u64,
        };

        with_retry(
            || async {
                let queue_permit = self
                    .rate_limiter
                    .acquire()
                    .await
                    .map_err(|_| RpcError::RateLimitError { retry_after: None })?;

                let start_time = Instant::now();
                let response = request_fn()
                    .await
                    .map_err(|e| RpcError::categorize(&e.to_string()))?;
                let elapsed = start_time.elapsed().as_millis();
                let status = response.status();
                let headers = response.headers().clone();

                drop(queue_permit);
                self.rate_limiter.observe_headers(&headers).await;

                if status.is_success() {
                    debug!("Request succeeded in {} ms", elapsed);
                    return Ok(response);
                }

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    self.rate_limiter.on_rate_limited(&headers).await;
                }

                let error_text = read_error_text(response).await;
                warn!(
                    "Request failed with status {} in {} ms: {}",
                    status, elapsed, error_text
                );

                let msg = format!("HTTP {status}: {error_text}");
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    let retry_after = headers
                        .get("Retry-After")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .map(Duration::from_secs);
                    Err(RpcError::RateLimitError { retry_after })
                } else if status == reqwest::StatusCode::REQUEST_TIMEOUT
                    || status == reqwest::StatusCode::GATEWAY_TIMEOUT
                {
                    Err(RpcError::TimeoutError(msg))
                } else if status.as_u16() >= 500 {
                    Err(RpcError::NetworkError(msg))
                } else {
                    Err(RpcError::ServerError {
                        status: status.as_u16(),
                        message: msg,
                    })
                }
            },
            retry_config,
            self.circuit_breaker.clone(),
        )
        .await
        .map_err(|e| {
            info!("Request failed after retry/circuit-breaker checks: {}", e);
            anyhow!("Request failed: {e}")
        })
    }
}

#[cfg(test)]
#[path = "stellar_tests.rs"]
mod tests;
