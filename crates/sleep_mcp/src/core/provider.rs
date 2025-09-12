use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time;

use crate::core::{
    error::{SleepServerError, SleepServerResult},
    models::{SleepResult, SleepStatus},
    utils::{calculate_progress, format_duration, parse_duration, parse_iso8601},
};

/// Sleep operation state
#[derive(Debug, Clone)]
struct SleepOperation {
    start_time: DateTime<Utc>,
    duration: Duration,
    start_instant: Instant,
    message: Option<String>,
}

/// Sleep server implementation
#[derive(Clone)]
pub struct SleepServer {
    current_operation: Arc<Mutex<Option<SleepOperation>>>,
}

impl SleepServer {
    pub fn new() -> Self {
        Self {
            current_operation: Arc::new(Mutex::new(None)),
        }
    }

    /// Sleep for a specified duration
    pub async fn sleep_for(
        &self,
        duration_str: &str,
        message: Option<String>,
    ) -> SleepServerResult<SleepResult> {
        let duration = parse_duration(duration_str)?;

        let start_time = Utc::now();
        let start_instant = Instant::now();

        // Set current operation
        {
            let mut current_op = self.current_operation.lock().unwrap();
            *current_op = Some(SleepOperation {
                start_time,
                duration,
                start_instant,
                message: message.clone(),
            });
        }

        tracing::info!("Starting sleep for {}", format_duration(duration));

        // Perform the sleep
        time::sleep(duration).await;

        let end_time = Utc::now();

        // Clear current operation
        {
            let mut current_op = self.current_operation.lock().unwrap();
            *current_op = None;
        }

        tracing::info!("Sleep completed after {}", format_duration(duration));

        Ok(SleepResult::new(
            duration, start_time, end_time, true, message,
        ))
    }

    /// Sleep until a specific time
    pub async fn sleep_until(
        &self,
        target_time_str: &str,
        message: Option<String>,
    ) -> SleepServerResult<SleepResult> {
        let target_time = parse_iso8601(target_time_str)?;
        let now = Utc::now();

        if target_time <= now {
            return Err(SleepServerError::InvalidDuration {
                duration: format!("Target time {} is in the past", target_time_str),
            });
        }

        let duration_chrono = target_time - now;
        let duration = Duration::from_millis(duration_chrono.num_milliseconds().max(0) as u64);

        // Check if duration exceeds maximum
        if duration > crate::core::utils::MAX_SLEEP_DURATION {
            return Err(SleepServerError::DurationTooLong {
                duration: format_duration(duration),
                max_duration: format_duration(crate::core::utils::MAX_SLEEP_DURATION),
            });
        }

        let start_time = now;
        let start_instant = Instant::now();

        // Set current operation
        {
            let mut current_op = self.current_operation.lock().unwrap();
            *current_op = Some(SleepOperation {
                start_time,
                duration,
                start_instant,
                message: message.clone(),
            });
        }

        tracing::info!(
            "Starting sleep until {} (duration: {})",
            target_time_str,
            format_duration(duration)
        );

        // Perform the sleep
        time::sleep(duration).await;

        let end_time = Utc::now();

        // Clear current operation
        {
            let mut current_op = self.current_operation.lock().unwrap();
            *current_op = None;
        }

        tracing::info!("Sleep completed at target time {}", target_time_str);

        Ok(SleepResult::new(
            duration, start_time, end_time, true, message,
        ))
    }

    /// Get current sleep status
    pub fn get_status(&self, detailed: bool) -> SleepStatus {
        let current_op = self.current_operation.lock().unwrap();

        match current_op.as_ref() {
            Some(op) => {
                let elapsed = op.start_instant.elapsed();
                let progress = calculate_progress(elapsed, op.duration);
                let remaining = if elapsed < op.duration {
                    op.duration - elapsed
                } else {
                    Duration::ZERO
                };

                SleepStatus {
                    is_sleeping: true,
                    current_duration_ms: Some(op.duration.as_millis() as u64),
                    start_time: if detailed {
                        Some(op.start_time.to_rfc3339())
                    } else {
                        None
                    },
                    expected_end_time: if detailed {
                        Some(
                            (op.start_time + chrono::Duration::from_std(op.duration).unwrap())
                                .to_rfc3339(),
                        )
                    } else {
                        None
                    },
                    progress_percent: Some(progress),
                    remaining_ms: Some(remaining.as_millis() as u64),
                    message: op.message.clone(),
                }
            }
            None => SleepStatus {
                is_sleeping: false,
                current_duration_ms: None,
                start_time: None,
                expected_end_time: None,
                progress_percent: None,
                remaining_ms: None,
                message: None,
            },
        }
    }

    /// Cancel current sleep operation (if any)
    pub async fn cancel_sleep(&self) -> SleepServerResult<bool> {
        let mut current_op = self.current_operation.lock().unwrap();

        match current_op.take() {
            Some(_) => {
                tracing::info!("Sleep operation cancelled");
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

impl Default for SleepServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration as TokioDuration};

    #[tokio::test]
    async fn test_sleep_server_creation() {
        let server = SleepServer::new();
        let status = server.get_status(false);
        assert!(!status.is_sleeping);
    }

    #[tokio::test]
    async fn test_short_sleep() {
        let server = SleepServer::new();

        let result = timeout(
            TokioDuration::from_millis(200),
            server.sleep_for("100ms", Some("Test sleep".to_string())),
        )
        .await;

        assert!(result.is_ok());
        let sleep_result = result.unwrap().unwrap();
        assert_eq!(sleep_result.duration_ms, 100);
        assert!(sleep_result.completed);
        assert_eq!(sleep_result.message, Some("Test sleep".to_string()));
    }

    #[tokio::test]
    async fn test_sleep_status_tracking() {
        let server = SleepServer::new();

        // Start a sleep operation in the background
        let server_clone = server.clone();
        let sleep_handle = tokio::spawn(async move { server_clone.sleep_for("200ms", None).await });

        // Give it a moment to start
        tokio::time::sleep(TokioDuration::from_millis(50)).await;

        // Check status
        let status = server.get_status(true);
        assert!(status.is_sleeping);
        assert!(status.progress_percent.is_some());
        assert!(status.remaining_ms.is_some());

        // Wait for completion
        let result = sleep_handle.await.unwrap();
        assert!(result.is_ok());

        // Check status after completion
        let status = server.get_status(false);
        assert!(!status.is_sleeping);
    }

    #[tokio::test]
    async fn test_invalid_duration() {
        let server = SleepServer::new();

        let result = server.sleep_for("invalid", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_duration_too_long() {
        let server = SleepServer::new();

        let result = server.sleep_for("1h", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sleep_until_past_time() {
        let server = SleepServer::new();

        let past_time = "2020-01-01T00:00:00Z";
        let result = server.sleep_until(past_time, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_sleep() {
        let server = SleepServer::new();

        // No active sleep to cancel
        let result = server.cancel_sleep().await;
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Start a sleep and cancel it
        let server_clone = server.clone();
        let _sleep_handle = tokio::spawn(async move { server_clone.sleep_for("1s", None).await });

        // Give it a moment to start
        tokio::time::sleep(TokioDuration::from_millis(10)).await;

        // Cancel the sleep
        let result = server.cancel_sleep().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
