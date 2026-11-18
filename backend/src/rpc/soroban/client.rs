//! Soroban / Stellar JSON-RPC endpoints of [`StellarRpcClient`].

use crate::observability::tracing::inject_trace_context;
use crate::rpc::error::RpcError;
use crate::rpc::metrics;
use crate::rpc::stellar::*;
use serde_json::json;
use tracing::info;

impl StellarRpcClient {
    /// Check the health of the RPC endpoint
    pub async fn check_health(&self) -> Result<HealthResponse, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_health_response());
        }

        info!("Checking RPC health at {}", self.rpc_url);

        let result = self
            .execute_with_retry(|| self.check_health_internal())
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn check_health_internal(&self) -> Result<HealthResponse, RpcError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "getHealth",
            "id": 1
        });

        let response = inject_trace_context(self.client.post(&self.rpc_url).json(&payload))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }

        let json_response: JsonRpcResponse<HealthResponse> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;

        if let Some(error) = json_response.error {
            return Err(RpcError::ServerError {
                status: 500,
                message: format!("RPC error: {} (code: {})", error.message, error.code),
            });
        }

        json_response
            .result
            .ok_or_else(|| RpcError::ParseError("No result in health response".to_string()))
    }

    /// I'm fetching ledgers via RPC getLedgers for sequential ingestion (issue #2)
    pub async fn fetch_ledgers(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError> {
        if self.mock_mode {
            let start = if let Some(c) = cursor {
                c.parse::<u64>().ok().map_or_else(
                    || start_ledger.unwrap_or(crate::rpc::mock_stellar::MOCK_OLDEST_LEDGER),
                    |v| v.saturating_add(1),
                )
            } else {
                start_ledger.unwrap_or(crate::rpc::mock_stellar::MOCK_OLDEST_LEDGER)
            };
            return Ok(crate::rpc::mock_stellar::mock_get_ledgers(start, limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_ledgers_internal(start_ledger, limit, cursor))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_ledgers_internal(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError> {
        let mut params = serde_json::Map::new();
        params.insert("pagination".to_string(), json!({ "limit": limit }));
        if let Some(c) = cursor {
            params
                .get_mut("pagination")
                .expect("pagination field should exist")
                .as_object_mut()
                .expect("pagination should be an object")
                .insert("cursor".to_string(), json!(c));
        } else if let Some(start) = start_ledger {
            params.insert("startLedger".to_string(), json!(start));
        }
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "getLedgers",
            "id": 1,
            "params": params
        });
        let response = inject_trace_context(self.client.post(&self.rpc_url).json(&payload))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let json_response: JsonRpcResponse<GetLedgersResult> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        if let Some(error) = json_response.error {
            return Err(RpcError::ServerError {
                status: 500,
                message: format!("RPC error: {} (code: {})", error.message, error.code),
            });
        }
        json_response
            .result
            .ok_or_else(|| RpcError::ParseError("No result in getLedgers response".to_string()))
    }
}
