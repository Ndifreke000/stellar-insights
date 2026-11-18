//! Paginated Horizon fetches with `DoS` caps.

use crate::observability::tracing::inject_trace_context;
use crate::rpc::stellar::*;
use anyhow::{Context, Result};
use std::fmt::Write;
use tracing::info;

impl StellarRpcClient {
    // ============================================================================
    // Paginated Fetch Methods
    // ============================================================================

    /// Fetch all payments with automatic pagination up to `max_total_records`
    ///
    /// # Arguments
    /// * `max_records` - Optional maximum number of records to fetch (uses config default if None)
    ///
    /// # Returns
    /// Vector of all fetched payments up to the limit
    pub async fn fetch_all_payments(&self, max_records: Option<u32>) -> Result<Vec<Payment>> {
        if self.mock_mode {
            let limit = self.resolve_max_records(max_records);
            return Ok(crate::rpc::mock_stellar::mock_payments(limit));
        }

        let max_records = self.resolve_max_records(max_records);
        let mut all_payments = Vec::new();
        let mut cursor: Option<String> = None;
        let mut fetched = 0;

        info!(
            "Starting paginated fetch of payments (max: {}, per_request: {})",
            max_records, self.max_records_per_request
        );

        while fetched < max_records {
            let limit = std::cmp::min(self.max_records_per_request, max_records - fetched);

            let payments = self
                .fetch_payments_page(limit, cursor.as_deref())
                .await
                .context("Failed to fetch payments page during pagination")?;

            if payments.is_empty() {
                info!("No more payments available, stopping pagination");
                break;
            }

            fetched += payments.len() as u32;
            cursor = Self::last_payment_cursor(&payments);

            all_payments.extend(payments);

            info!(
                "Fetched {} payments so far ({}/{})",
                all_payments.len(),
                fetched,
                max_records
            );

            // Rate limiting delay between requests
            if fetched < max_records && cursor.is_some() {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.pagination_delay_ms))
                    .await;
            } else {
                break;
            }
        }

        info!(
            "Completed pagination: fetched {} total payments",
            all_payments.len()
        );
        Ok(all_payments)
    }

    fn resolve_max_records(&self, max_records: Option<u32>) -> u32 {
        max_records
            .unwrap_or(self.max_total_records)
            .min(ABSOLUTE_MAX_TOTAL_RECORDS)
    }

    async fn fetch_payments_page(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Payment>> {
        self.fetch_payments(limit, cursor)
            .await
            .context("Failed to fetch payments page")
    }

    fn last_payment_cursor(payments: &[Payment]) -> Option<String> {
        payments.last().map(|payment| payment.paging_token.clone())
    }

    /// Fetch all trades with automatic pagination up to `max_total_records`
    ///
    /// # Arguments
    /// * `max_records` - Optional maximum number of records to fetch (uses config default if None)
    ///
    /// # Returns
    /// Vector of all fetched trades up to the limit
    pub async fn fetch_all_trades(&self, max_records: Option<u32>) -> Result<Vec<Trade>> {
        if self.mock_mode {
            let limit = max_records
                .unwrap_or(self.max_total_records)
                .min(ABSOLUTE_MAX_TOTAL_RECORDS);
            return Ok(crate::rpc::mock_stellar::mock_trades(limit));
        }

        let max_records = max_records
            .unwrap_or(self.max_total_records)
            .min(ABSOLUTE_MAX_TOTAL_RECORDS);
        let mut all_trades = Vec::new();
        let mut cursor: Option<String> = None;
        let mut fetched = 0;

        info!(
            "Starting paginated fetch of trades (max: {}, per_request: {})",
            max_records, self.max_records_per_request
        );

        while fetched < max_records {
            let limit = std::cmp::min(self.max_records_per_request, max_records - fetched);

            // Note: Trade struct doesn't have paging_token, we'll use id as cursor
            let trades = self
                .fetch_trades(limit, cursor.as_deref())
                .await
                .context("Failed to fetch trades page")?;

            if trades.is_empty() {
                info!("No more trades available, stopping pagination");
                break;
            }

            fetched += trades.len() as u32;

            // Extract cursor from last trade for next page
            // Horizon uses the id field as cursor for trades
            if let Some(last_trade) = trades.last() {
                cursor = Some(last_trade.id.clone());
            }

            all_trades.extend(trades);

            info!(
                "Fetched {} trades so far ({}/{})",
                all_trades.len(),
                fetched,
                max_records
            );

            // Rate limiting delay between requests
            if fetched < max_records && cursor.is_some() {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.pagination_delay_ms))
                    .await;
            } else {
                break;
            }
        }

        info!(
            "Completed pagination: fetched {} total trades",
            all_trades.len()
        );
        Ok(all_trades)
    }

    /// Fetch all payments for a specific account with automatic pagination
    ///
    /// # Arguments
    /// * `account_id` - The Stellar account ID
    /// * `max_records` - Optional maximum number of records to fetch (uses config default if None)
    ///
    /// # Returns
    /// Vector of all fetched payments for the account up to the limit
    pub async fn fetch_all_account_payments(
        &self,
        account_id: &str,
        max_records: Option<u32>,
    ) -> Result<Vec<Payment>> {
        if self.mock_mode {
            let limit = max_records
                .unwrap_or(self.max_total_records)
                .min(ABSOLUTE_MAX_TOTAL_RECORDS);
            return Ok(crate::rpc::mock_stellar::mock_payments(limit));
        }

        let max_records = max_records
            .unwrap_or(self.max_total_records)
            .min(ABSOLUTE_MAX_TOTAL_RECORDS);
        let mut all_payments = Vec::new();
        let mut cursor: Option<String> = None;
        let mut fetched = 0;

        info!(
            "Starting paginated fetch of payments for account {} (max: {}, per_request: {})",
            account_id, max_records, self.max_records_per_request
        );

        while fetched < max_records {
            let limit = std::cmp::min(self.max_records_per_request, max_records - fetched);

            let mut url = format!(
                "{}/accounts/{}/payments?order=desc&limit={}",
                self.horizon_url, account_id, limit
            );

            if let Some(ref cursor_val) = cursor {
                let _ = write!(url, "&cursor={cursor_val}");
            }

            let response = self
                .retry_request(|| async {
                    inject_trace_context(self.client.get(&url)).send().await
                })
                .await
                .context("Failed to fetch account payments page")?;

            let horizon_response: HorizonResponse<Payment> = response
                .json()
                .await
                .context("Failed to parse payments response")?;

            let payments = horizon_response
                .embedded
                .map(|e| e.records)
                .unwrap_or_default();

            if payments.is_empty() {
                info!("No more payments available for account, stopping pagination");
                break;
            }

            fetched += payments.len() as u32;

            // Extract cursor from last payment for next page
            if let Some(last_payment) = payments.last() {
                cursor = Some(last_payment.paging_token.clone());
            }

            all_payments.extend(payments);

            info!(
                "Fetched {} payments for account so far ({}/{})",
                all_payments.len(),
                fetched,
                max_records
            );

            // Rate limiting delay between requests
            if fetched < max_records && cursor.is_some() {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.pagination_delay_ms))
                    .await;
            } else {
                break;
            }
        }

        info!(
            "Completed pagination: fetched {} total payments for account {}",
            all_payments.len(),
            account_id
        );
        Ok(all_payments)
    }
}
