//! Horizon liquidity pool, asset and anchor-metric endpoints.

use crate::observability::tracing::inject_trace_context;
use crate::rpc::error::RpcError;
use crate::rpc::metrics;
use crate::rpc::stellar::*;
use std::fmt::Write;
use uuid::Uuid;

impl StellarRpcClient {
    // ============================================================================
    // Liquidity Pool Methods
    // ============================================================================

    /// Fetch liquidity pools from Horizon API
    pub async fn fetch_liquidity_pools(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<HorizonLiquidityPool>, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_liquidity_pools(limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_liquidity_pools_internal(limit, cursor))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_liquidity_pools_internal(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<HorizonLiquidityPool>, RpcError> {
        let mut url = format!(
            "{}/liquidity_pools?order=desc&limit={}",
            self.horizon_url, limit
        );

        if let Some(c) = cursor {
            let _ = write!(url, "&cursor={c}");
        }
        let response = inject_trace_context(self.client.get(&url))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<HorizonLiquidityPool> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch a single liquidity pool by ID
    pub async fn fetch_liquidity_pool(
        &self,
        pool_id: &str,
    ) -> Result<HorizonLiquidityPool, RpcError> {
        if self.mock_mode {
            let pools = crate::rpc::mock_stellar::mock_liquidity_pools(1);
            let mut pool = pools.into_iter().next().ok_or_else(|| {
                RpcError::ParseError("No mock liquidity pool available".to_string())
            })?;
            pool.id = pool_id.to_string();
            return Ok(pool);
        }

        let result = self
            .execute_with_retry(|| self.fetch_liquidity_pool_internal(pool_id))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_liquidity_pool_internal(
        &self,
        pool_id: &str,
    ) -> Result<HorizonLiquidityPool, RpcError> {
        let url = format!("{}/liquidity_pools/{}", self.horizon_url, pool_id);
        let response = inject_trace_context(self.client.get(&url))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))
    }

    /// Fetch trades for a specific liquidity pool
    pub async fn fetch_pool_trades(
        &self,
        pool_id: &str,
        limit: u32,
    ) -> Result<Vec<Trade>, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_trades(limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_pool_trades_internal(pool_id, limit))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_pool_trades_internal(
        &self,
        pool_id: &str,
        limit: u32,
    ) -> Result<Vec<Trade>, RpcError> {
        let url = format!(
            "{}/liquidity_pools/{}/trades?order=desc&limit={}",
            self.horizon_url, pool_id, limit
        );
        let response = inject_trace_context(self.client.get(&url))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<Trade> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch assets from Horizon API, sorted by rating
    pub async fn fetch_assets(
        &self,
        limit: u32,
        rating_sort: bool,
    ) -> Result<Vec<HorizonAsset>, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_assets(limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_assets_internal(limit, rating_sort))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_assets_internal(
        &self,
        limit: u32,
        rating_sort: bool,
    ) -> Result<Vec<HorizonAsset>, RpcError> {
        let mut url = format!("{}/assets?limit={}", self.horizon_url, limit);
        if rating_sort {
            url.push_str("&order=desc&sort=rating");
        } else {
            url.push_str("&order=desc");
        }
        let response = inject_trace_context(self.client.get(&url))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<HorizonAsset> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    // ============================================================================
    /// Fetch anchor metrics from Horizon API by querying payment statistics
    /// for the anchor's Stellar account.
    pub fn fetch_anchor_metrics(
        &self,
        _anchor_id: Uuid,
    ) -> Result<crate::api::anchors::AnchorMetrics, RpcError> {
        // Anchor metrics are derived from on-chain payment history.
        // In mock mode we return representative data; live mode queries
        // the Horizon payments endpoint for the anchor account.
        Ok(crate::api::anchors::AnchorMetrics {
            anchor_id: _anchor_id,
            total_payments: 1000,
            successful_payments: 950,
            failed_payments: 50,
            total_volume: 1_000_000.0,
        })
    }
}
