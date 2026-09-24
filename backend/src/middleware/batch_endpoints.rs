use anyhow::{bail, Result};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::{
    batch_endpoints::{BatchConfig, BatchItemResult, BatchRequest, BatchResponse},
    network_context_middleware::NetworkContext,
};

#[derive(Clone)]
pub struct BatchEndpoints {
    config: BatchConfig,
    /// Running count of total items processed across all batches.
    state: Arc<RwLock<u64>>,
}

impl BatchEndpoints {
    pub fn new(config: BatchConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(0)),
        }
    }

    /// Process a batch of items using the provided per-item handler.
    ///
    /// Returns an error only for structural problems (e.g. batch too large).
    /// Individual item failures are captured inside [`BatchItemResult`].
    pub async fn process<T, U, F>(
        &self,
        context: &NetworkContext,
        request: BatchRequest<T>,
        mut handler: F,
    ) -> Result<BatchResponse<U>>
    where
        T: Serialize,
        U: Serialize,
        F: FnMut(usize, T) -> Result<U>,
    {
        let total = request.items.len();

        if total > self.config.max_batch_size {
            bail!(
                "Batch size {} exceeds maximum of {}",
                total,
                self.config.max_batch_size
            );
        }

        let mut results = Vec::with_capacity(total);
        let mut succeeded = 0usize;
        let mut failed = 0usize;

        for (index, item) in request.items.into_iter().enumerate() {
            match handler(index, item) {
                Ok(data) => {
                    succeeded += 1;
                    results.push(BatchItemResult {
                        index,
                        success: true,
                        data: Some(data),
                        error: None,
                    });
                }
                Err(e) => {
                    failed += 1;
                    tracing::warn!(
                        network = ?context.network,
                        index,
                        error = %e,
                        "Batch item failed"
                    );
                    results.push(BatchItemResult {
                        index,
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        let mut count = self.state.write().await;
        *count += total as u64;

        tracing::info!(
            network = ?context.network,
            total,
            succeeded,
            failed,
            total_processed = *count,
            "Batch processed"
        );

        Ok(BatchResponse {
            results,
            total,
            succeeded,
            failed,
        })
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn instance() -> BatchEndpoints {
        BatchEndpoints::new(BatchConfig::default())
    }

    #[tokio::test]
    async fn test_basic_functionality() {
        let inst = instance();
        let ctx = NetworkContext::testnet();
        let req = BatchRequest {
            items: vec![1u32, 2, 3],
        };
        let result = inst.process(&ctx, req, |_, v| Ok(v * 2)).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.total, 3);
        assert_eq!(resp.succeeded, 3);
        assert_eq!(resp.failed, 0);
    }

    #[tokio::test]
    async fn test_mainnet_supported() {
        let inst = instance();
        let ctx = NetworkContext::mainnet();
        let req = BatchRequest {
            items: vec!["a", "b"],
        };
        let result = inst.process(&ctx, req, |_, v| Ok(v.to_uppercase())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_partial_failures_captured() {
        let inst = instance();
        let ctx = NetworkContext::testnet();
        let req = BatchRequest {
            items: vec![0u32, 1, 2],
        };
        let result = inst
            .process(&ctx, req, |_, v| {
                if v == 0 {
                    bail!("zero not allowed")
                } else {
                    Ok(v)
                }
            })
            .await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.total, 3);
        assert_eq!(resp.succeeded, 2);
        assert_eq!(resp.failed, 1);
        assert!(!resp.results[0].success);
        assert_eq!(resp.results[0].error.as_deref(), Some("zero not allowed"));
    }

    #[tokio::test]
    async fn test_exceeds_max_batch_size_returns_error() {
        let inst = BatchEndpoints::new(BatchConfig { max_batch_size: 2 });
        let ctx = NetworkContext::testnet();
        let req = BatchRequest {
            items: vec![1, 2, 3],
        };
        let result = inst.process(&ctx, req, |_, v: i32| Ok(v)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    #[tokio::test]
    async fn test_empty_batch_succeeds() {
        let inst = instance();
        let ctx = NetworkContext::testnet();
        let req: BatchRequest<i32> = BatchRequest { items: vec![] };
        let result = inst.process(&ctx, req, |_, v| Ok(v)).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.total, 0);
        assert_eq!(resp.succeeded, 0);
        assert_eq!(resp.failed, 0);
    }

    #[tokio::test]
    async fn test_result_indices_are_correct() {
        let inst = instance();
        let ctx = NetworkContext::testnet();
        let req = BatchRequest {
            items: vec!["x", "y", "z"],
        };
        let resp = inst
            .process(&ctx, req, |_, v| Ok(v.to_string()))
            .await
            .unwrap();
        for (i, item) in resp.results.iter().enumerate() {
            assert_eq!(item.index, i);
        }
    }
}
