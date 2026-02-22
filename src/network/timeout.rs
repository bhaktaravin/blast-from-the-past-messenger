//! Timeout utilities for network operations
use std::future::Future;
use std::time::Duration;
use tokio::time::timeout as tokio_timeout;

/// Wrapper for futures with timeout functionality
pub struct TimeoutWrapper;

impl TimeoutWrapper {
    /// Execute a future with a timeout
    ///
    /// # Arguments
    ///
    /// * `future` - The future to execute
    /// * `duration` - The timeout duration
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - If the future completes successfully within the timeout
    /// * `Err(TimeoutError)` - If the future times out
    pub async fn with_timeout<T>(
        future: impl Future<Output = T>,
        duration: Duration,
    ) -> Result<T, TimeoutError> {
        match tokio_timeout(duration, future).await {
            Ok(result) => Ok(result),
            Err(_) => Err(TimeoutError::Timeout),
        }
    }

    /// Execute a future with a default timeout (30 seconds)
    pub async fn with_default_timeout<T>(
        future: impl Future<Output = T>,
    ) -> Result<T, TimeoutError> {
        Self::with_timeout(future, Duration::from_secs(30)).await
    }
}

/// Timeout error types
#[derive(Debug, thiserror::Error)]
pub enum TimeoutError {
    #[error("Operation timed out")]
    Timeout,
}

/// Configuration for timeout behavior
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Default timeout duration
    pub default_timeout: Duration,
    /// Connection timeout duration
    pub connection_timeout: Duration,
    /// Read timeout duration
    pub read_timeout: Duration,
    /// Write timeout duration
    pub write_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
        }
    }
}

impl TimeoutConfig {
    /// Create a new timeout configuration
    pub fn new(
        default_timeout: Duration,
        connection_timeout: Duration,
        read_timeout: Duration,
        write_timeout: Duration,
    ) -> Self {
        Self {
            default_timeout,
            connection_timeout,
            read_timeout,
            write_timeout,
        }
    }
}
