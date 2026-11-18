//! Horizon REST endpoints of [`StellarRpcClient`] (ledgers, payments,
//! trades, order books, transactions, operations, effects).

use crate::observability::tracing::inject_trace_context;
use crate::rpc::error::RpcError;
use crate::rpc::metrics;
use crate::rpc::stellar::*;
use std::fmt::Write;
use tracing::info;

impl StellarRpcClient {
    /// Fetch latest ledger information
    pub async fn fetch_latest_ledger(&self) -> Result<LedgerInfo, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_ledger_info());
        }

        let result = self
            .execute_with_retry(|| self.fetch_latest_ledger_internal())
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_latest_ledger_internal(&self) -> Result<LedgerInfo, RpcError> {
        let url = format!("{}/ledgers?order=desc&limit=1", self.horizon_url);
        let response = inject_trace_context(self.client.get(&url))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<LedgerInfo> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        horizon_response
            .embedded
            .and_then(|e| e.records.into_iter().next())
            .ok_or_else(|| RpcError::ParseError("No ledger data found".to_string()))
    }

    /// Fetch a specific ledger by its sequence number and return its info.
    /// Used to verify ledger hashes during snapshot generation (issue #1631).
    pub async fn fetch_ledger_by_sequence(&self, sequence: u64) -> Result<LedgerInfo, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_ledger_info());
        }

        let result = self
            .execute_with_retry(|| self.fetch_ledger_by_sequence_internal(sequence))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_ledger_by_sequence_internal(
        &self,
        sequence: u64,
    ) -> Result<LedgerInfo, RpcError> {
        let url = format!("{}/ledgers/{}", self.horizon_url, sequence);
        let response = inject_trace_context(self.client.get(&url))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        // Horizon returns a single ledger object (not wrapped in _embedded)
        // when querying by sequence directly.
        response
            .json::<LedgerInfo>()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))
    }

    /// Fetch recent payments
    pub async fn fetch_payments(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<Payment>, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_payments(limit));
        }

        info!("Fetching {} payments from Horizon API", limit);

        let result = self
            .execute_with_retry(|| self.fetch_payments_internal(limit, cursor))
            .await;
        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_payments_internal(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<Payment>, RpcError> {
        // include_failed: without it Horizon returns only successful payments,
        // and every corridor's success rate would read 100%.
        let mut url = format!(
            "{}/payments?order=desc&limit={}&include_failed=true",
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
        let horizon_response: HorizonResponse<Payment> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch recent trades
    pub async fn fetch_trades(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<Trade>, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_trades(limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_trades_internal(limit, cursor))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_trades_internal(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<Trade>, RpcError> {
        let mut url = format!("{}/trades?order=desc&limit={}", self.horizon_url, limit);
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
        let horizon_response: HorizonResponse<Trade> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch order book for a trading pair
    pub async fn fetch_order_book(
        &self,
        selling_asset: &Asset,
        buying_asset: &Asset,
        limit: u32,
    ) -> Result<OrderBook, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_order_book(
                selling_asset,
                buying_asset,
            ));
        }

        let result = self
            .execute_with_retry(|| {
                self.fetch_order_book_internal(selling_asset, buying_asset, limit)
            })
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_order_book_internal(
        &self,
        selling_asset: &Asset,
        buying_asset: &Asset,
        limit: u32,
    ) -> Result<OrderBook, RpcError> {
        let selling_params = Self::asset_to_query_params("selling", selling_asset)
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        let buying_params = Self::asset_to_query_params("buying", buying_asset)
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        let url = format!(
            "{}/order_book?{}&{}&limit={}",
            self.horizon_url, selling_params, buying_params, limit
        );
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

    pub async fn fetch_payments_for_ledger(&self, sequence: u64) -> Result<Vec<Payment>, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_payments(5));
        }

        let result = self
            .execute_with_retry(|| self.fetch_payments_for_ledger_internal(sequence))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_payments_for_ledger_internal(
        &self,
        sequence: u64,
    ) -> Result<Vec<Payment>, RpcError> {
        let url = format!(
            "{}/ledgers/{}/payments?limit=200",
            self.horizon_url, sequence
        );
        let response = inject_trace_context(self.client.get(&url))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<Payment> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch transactions for a specific ledger
    pub async fn fetch_transactions_for_ledger(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_transactions(5, sequence));
        }

        let result = self
            .execute_with_retry(|| self.fetch_transactions_for_ledger_internal(sequence))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_transactions_for_ledger_internal(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError> {
        let url = format!(
            "{}/ledgers/{}/transactions?limit=200&include_failed=true",
            self.horizon_url, sequence
        );
        let response = inject_trace_context(self.client.get(&url))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<HorizonTransaction> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch operations for a specific ledger
    pub async fn fetch_operations_for_ledger(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonOperation>, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_operations_for_ledger(sequence));
        }

        let result = self
            .execute_with_retry(|| self.fetch_operations_for_ledger_internal(sequence))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_operations_for_ledger_internal(
        &self,
        sequence: u64,
    ) -> Result<Vec<HorizonOperation>, RpcError> {
        let url = format!(
            "{}/ledgers/{}/operations?limit=200",
            self.horizon_url, sequence
        );
        let response = inject_trace_context(self.client.get(&url))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<HorizonOperation> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch effects for a specific operation
    pub async fn fetch_operation_effects(
        &self,
        operation_id: &str,
    ) -> Result<Vec<HorizonEffect>, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_effects_for_operation(
                operation_id,
            ));
        }

        let result = self
            .execute_with_retry(|| self.fetch_operation_effects_internal(operation_id))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_operation_effects_internal(
        &self,
        operation_id: &str,
    ) -> Result<Vec<HorizonEffect>, RpcError> {
        let url = format!(
            "{}/operations/{}/effects?limit=200",
            self.horizon_url, operation_id
        );
        let response = inject_trace_context(self.client.get(&url))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<HorizonEffect> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }

    /// Fetch payments for a specific account
    pub async fn fetch_account_payments(
        &self,
        account_id: &str,
        limit: u32,
    ) -> Result<Vec<Payment>, RpcError> {
        if self.mock_mode {
            return Ok(crate::rpc::mock_stellar::mock_payments(limit));
        }

        let result = self
            .execute_with_retry(|| self.fetch_account_payments_internal(account_id, limit))
            .await;

        result.inspect_err(|e| {
            metrics::record_rpc_error(e.error_type_label(), "stellar");
        })
    }

    async fn fetch_account_payments_internal(
        &self,
        account_id: &str,
        limit: u32,
    ) -> Result<Vec<Payment>, RpcError> {
        let url = format!(
            "{}/accounts/{}/payments?order=desc&limit={}",
            self.horizon_url, account_id, limit
        );
        let response = inject_trace_context(self.client.get(&url))
            .send()
            .await
            .map_err(|e| RpcError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }
        let horizon_response: HorizonResponse<Payment> = response
            .json()
            .await
            .map_err(|e| RpcError::ParseError(e.to_string()))?;
        Ok(horizon_response
            .embedded
            .map(|e| e.records)
            .unwrap_or_default())
    }
}
