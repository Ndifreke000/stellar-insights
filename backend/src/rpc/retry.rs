//! Smart retry logic with exponential backoff for RPC calls.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tracing::warn;

use crate::rpc::error::RpcError;

/// Retries the given async operation with exponential backoff. Only retries on retryable errors.
/// Respects `retry_after` from rate limit errors when present.
pub async fn retry_with_backoff<F, Fut, T>(
    mut f: F,
    max_retries: u32,
    initial_backoff: Duration,
    max_backoff: Duration,
) -> Result<T, RpcError>
where
    F: FnMut() -> Pin<Box<Fut>>,
    Fut: Future<Output = Result<T, RpcError>>,
{
    let mut attempt = 0;
    let mut backoff = initial_backoff;

    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && attempt < max_retries => {
                attempt += 1;

                let sleep_duration = if let Some(retry_after) = e.retry_after() {
                    retry_after
                } else {
                    backoff
                };

                warn!(
                    "Retrying request (attempt {}/{}): {}",
                    attempt, max_retries, e
                );

                tokio::time::sleep(sleep_duration).await;
                backoff = std::cmp::min(backoff * 2, max_backoff);
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_retry_succeeds_on_first_try() {
        let result = retry_with_backoff(
            || Box::pin(async { Ok::<i32, crate::rpc::RpcError>(1) }),
            3,
            Duration::from_millis(1),
            Duration::from_secs(1),
        )
        .await;
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_no_retry_on_non_retryable_error() {
        let result = retry_with_backoff(
            || {
                Box::pin(async {
                    Err::<i32, crate::rpc::RpcError>(crate::rpc::RpcError::ParseError(
                        "bad".to_string(),
                    ))
                })
            },
            3,
            Duration::from_millis(1),
            Duration::from_secs(1),
        )
        .await;
        assert!(result.is_err());
        assert!(!result.unwrap_err().is_retryable());
    }

    #[tokio::test]
    async fn test_retry_then_succeed() {
        let attempts = AtomicU32::new(0);
        let result = retry_with_backoff(
            || {
                let attempts = &attempts;
                Box::pin(async move {
                    let n = attempts.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        Err(crate::rpc::RpcError::TimeoutError(Duration::from_secs(1)))
                    } else {
                        Ok(100)
                    }
                })
            },
            5,
            Duration::from_millis(1),
            Duration::from_secs(1),
        )
        .await;
        assert_eq!(result.unwrap(), 100);
        assert!(attempts.load(Ordering::SeqCst) >= 3);
    }
}
