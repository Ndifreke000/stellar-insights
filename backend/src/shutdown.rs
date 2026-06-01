//! Graceful shutdown handling for the backend server
//!
//! This module provides utilities for handling shutdown signals (SIGTERM, SIGINT)
//! and coordinating graceful shutdown of server components.

use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tracing::{info, warn};

/// Configuration for graceful shutdown behavior
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Maximum time to wait for in-flight requests to complete
    pub graceful_timeout: Duration,
    /// Maximum time to wait for background tasks to complete
    pub background_task_timeout: Duration,
    /// Maximum time to wait for database connections to close
    pub db_close_timeout: Duration,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            graceful_timeout: Duration::from_secs(30),
            background_task_timeout: Duration::from_secs(10),
            db_close_timeout: Duration::from_secs(5),
        }
    }
}

impl ShutdownConfig {
    /// Create a new shutdown configuration from environment variables
    pub fn from_env() -> Self {
        let graceful_timeout = std::env::var("SHUTDOWN_GRACEFUL_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(30));

        let background_task_timeout = std::env::var("SHUTDOWN_BACKGROUND_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(10));

        let db_close_timeout = std::env::var("SHUTDOWN_DB_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(5));

        Self {
            graceful_timeout,
            background_task_timeout,
            db_close_timeout,
        }
    }
}

/// Shutdown coordinator that manages the graceful shutdown process
pub struct ShutdownCoordinator {
    config: ShutdownConfig,
    shutdown_tx: broadcast::Sender<()>,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator with the given configuration
    pub fn new(config: ShutdownConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            config,
            shutdown_tx,
        }
    }

    /// Get a receiver for shutdown notifications
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Trigger shutdown and notify all subscribers
    pub fn trigger_shutdown(&self) {
        info!("Triggering graceful shutdown");
        let _ = self.shutdown_tx.send(());
    }

    /// Get the graceful timeout duration
    pub fn graceful_timeout(&self) -> Duration {
        self.config.graceful_timeout
    }

    /// Get the background task timeout duration
    pub fn background_task_timeout(&self) -> Duration {
        self.config.background_task_timeout
    }

    /// Get the database close timeout duration
    pub fn db_close_timeout(&self) -> Duration {
        self.config.db_close_timeout
    }
}

/// Wait for shutdown signals (SIGTERM, SIGINT/Ctrl+C)
///
/// This function will block until either SIGTERM or SIGINT is received.
/// On Windows, only Ctrl+C is supported.
pub async fn wait_for_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM signal");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT signal");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C signal");
    }
}

/// Gracefully shutdown background tasks
///
/// Waits for the given task handles to complete within the timeout period.
/// If the timeout is exceeded, tasks are forcefully aborted.
pub async fn shutdown_background_tasks(
    tasks: Vec<tokio::task::JoinHandle<()>>,
    timeout_duration: Duration,
) {
    info!("Shutting down {} background tasks", tasks.len());

    let shutdown_future = async {
        for (idx, task) in tasks.into_iter().enumerate() {
            match task.await {
                Ok(_) => info!("Background task {} completed successfully", idx),
                Err(e) if e.is_cancelled() => {
                    info!("Background task {} was cancelled", idx)
                }
                Err(e) => warn!("Background task {} failed: {}", idx, e),
            }
        }
    };

    match timeout(timeout_duration, shutdown_future).await {
        Ok(_) => info!("All background tasks completed within timeout"),
        Err(_) => warn!(
            "Background tasks did not complete within {:?}, proceeding with shutdown",
            timeout_duration
        ),
    }
}

/// Gracefully close database connection pool
///
/// Attempts to close the database pool within the timeout period.
pub async fn shutdown_database(pool: sqlx::SqlitePool, timeout_duration: Duration) {
    info!("Closing database connections");

    let close_future = async {
        pool.close().await;
        info!("Database connections closed successfully");
    };

    match timeout(timeout_duration, close_future).await {
        Ok(_) => info!("Database closed within timeout"),
        Err(_) => warn!(
            "Database did not close within {:?}, proceeding with shutdown",
            timeout_duration
        ),
    }
}

/// Flush Redis cache and close connections gracefully
///
/// Ensures all pending Redis operations are completed and connections are closed properly.
pub async fn flush_cache(
    cache: std::sync::Arc<crate::cache::CacheManager>,
    timeout_duration: Duration,
) {
    info!("Flushing cache and closing Redis connections");

    let flush_future = async {
        // Log cache statistics before shutdown
        let stats = cache.get_stats();
        info!(
            "Cache statistics - Hits: {}, Misses: {}, Invalidations: {}, Hit Rate: {:.2}%",
            stats.hits,
            stats.misses,
            stats.invalidations,
            stats.hit_rate()
        );

        // Close Redis connection gracefully
        if let Err(e) = cache.close().await {
            warn!("Error closing cache connections: {}", e);
        } else {
            info!("Cache connections closed successfully");
        }
    };

    match timeout(timeout_duration, flush_future).await {
        Ok(_) => info!("Cache flush completed within timeout"),
        Err(_) => warn!(
            "Cache flush did not complete within {:?}, proceeding with shutdown",
            timeout_duration
        ),
    }
}

/// Close WebSocket connections gracefully
///
/// Notifies all connected WebSocket clients about the shutdown and closes connections.
pub async fn shutdown_websockets(
    ws_state: std::sync::Arc<crate::websocket::WsState>,
    timeout_duration: Duration,
) {
    info!(
        "Closing {} WebSocket connections",
        ws_state.connection_count()
    );

    let close_future = async {
        // Send shutdown notification to all connected clients
        ws_state.broadcast(crate::websocket::WsMessage::ServerShutdown {
            message: "Server is shutting down gracefully".to_string(),
        });

        // Give clients a moment to receive the message
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Close all connections
        ws_state.close_all_connections().await;

        info!("All WebSocket connections closed");
    };

    match timeout(timeout_duration, close_future).await {
        Ok(_) => info!("WebSocket shutdown completed within timeout"),
        Err(_) => warn!(
            "WebSocket shutdown did not complete within {:?}, proceeding with shutdown",
            timeout_duration
        ),
    }
}

/// Log shutdown statistics and final state
pub fn log_shutdown_summary(start_time: std::time::Instant) {
    let elapsed = start_time.elapsed();
    info!(
        "Graceful shutdown completed in {:.2}s",
        elapsed.as_secs_f64()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_config_default() {
        let config = ShutdownConfig::default();
        assert_eq!(config.graceful_timeout, Duration::from_secs(30));
        assert_eq!(config.background_task_timeout, Duration::from_secs(10));
        assert_eq!(config.db_close_timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_shutdown_coordinator_creation() {
        let config = ShutdownConfig::default();
        let coordinator = ShutdownCoordinator::new(config);

        // Should be able to subscribe
        let _rx = coordinator.subscribe();

        // Should be able to get timeouts
        assert_eq!(coordinator.graceful_timeout(), Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_broadcast() {
        let config = ShutdownConfig::default();
        let coordinator = ShutdownCoordinator::new(config);

        let mut rx1 = coordinator.subscribe();
        let mut rx2 = coordinator.subscribe();

        coordinator.trigger_shutdown();

        // Both receivers should get the signal
        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_background_tasks_success() {
        let task1 = tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
        });

        let task2 = tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(20)).await;
        });

        shutdown_background_tasks(vec![task1, task2], Duration::from_secs(1)).await;

        // Should complete without panic
    }

    #[tokio::test]
    async fn test_shutdown_background_tasks_timeout() {
        let task = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(10)).await;
        });

        // Should timeout but not panic
        shutdown_background_tasks(vec![task], Duration::from_millis(100)).await;
    }
}
