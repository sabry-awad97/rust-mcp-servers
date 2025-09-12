use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time;

use crate::core::{
    error::{SleepServerError, SleepServerResult},
    models::{
        EnhancedSleepStatus, OperationStatus, SleepOperation, SleepResult, SleepStartResponse,
        SleepStatus,
    },
    task_manager::TaskManager,
    utils::{calculate_progress, format_duration, parse_duration, parse_iso8601},
};

/// Legacy sleep operation state (for backward compatibility)
#[derive(Debug, Clone)]
struct LegacySleepOperation {
    start_time: DateTime<Utc>,
    duration: Duration,
    start_instant: Instant,
    message: Option<String>,
}

/// Enhanced sleep server implementation with background task management
#[derive(Clone)]
pub struct SleepServer {
    /// Legacy current operation for backward compatibility
    current_operation: Arc<Mutex<Option<LegacySleepOperation>>>,
    /// Background task manager for non-blocking operations
    task_manager: TaskManager,
}

impl SleepServer {
    pub fn new() -> Self {
        Self {
            current_operation: Arc::new(Mutex::new(None)),
            task_manager: TaskManager::new(),
        }
    }

    /// Create a new sleep server with custom task manager limits
    pub fn with_limits(max_concurrent: usize, max_history: usize) -> Self {
        Self {
            current_operation: Arc::new(Mutex::new(None)),
            task_manager: TaskManager::with_limits(max_concurrent, max_history),
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
            *current_op = Some(LegacySleepOperation {
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
            *current_op = Some(LegacySleepOperation {
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

    /// Start a non-blocking sleep operation (returns immediately with operation ID)
    pub async fn start_sleep_operation(
        &self,
        duration_str: &str,
        message: Option<String>,
    ) -> SleepServerResult<SleepStartResponse> {
        let duration = parse_duration(duration_str)?;

        let operation_id = self
            .task_manager
            .start_sleep_operation(duration, message.clone())
            .await
            .map_err(|e| SleepServerError::InvalidDuration { duration: e })?;

        let expected_completion =
            (Utc::now() + chrono::Duration::from_std(duration).unwrap()).to_rfc3339();

        Ok(SleepStartResponse {
            operation_id,
            message: format!("Sleep operation started for {}", format_duration(duration)),
            duration_ms: duration.as_millis() as u64,
            duration_str: format_duration(duration),
            expected_completion,
        })
    }

    /// Start a non-blocking sleep until operation (returns immediately with operation ID)
    pub async fn start_sleep_until_operation(
        &self,
        target_time_str: &str,
        message: Option<String>,
    ) -> SleepServerResult<SleepStartResponse> {
        let target_time = parse_iso8601(target_time_str)?;
        let now = Utc::now();

        if target_time <= now {
            return Err(SleepServerError::InvalidDuration {
                duration: format!("Target time {} is in the past", target_time_str),
            });
        }

        let duration_chrono = target_time - now;
        let duration = Duration::from_millis(duration_chrono.num_milliseconds().max(0) as u64);

        if duration > crate::core::utils::MAX_SLEEP_DURATION {
            return Err(SleepServerError::DurationTooLong {
                duration: format_duration(duration),
                max_duration: format_duration(crate::core::utils::MAX_SLEEP_DURATION),
            });
        }

        let operation_id = self
            .task_manager
            .start_sleep_operation(duration, message.clone())
            .await
            .map_err(|e| SleepServerError::InvalidDuration { duration: e })?;

        Ok(SleepStartResponse {
            operation_id,
            message: format!("Sleep operation started until {}", target_time_str),
            duration_ms: duration.as_millis() as u64,
            duration_str: format_duration(duration),
            expected_completion: target_time.to_rfc3339(),
        })
    }

    /// Get enhanced sleep status with background operation tracking
    pub async fn get_enhanced_status(&self, operation_id: Option<String>) -> EnhancedSleepStatus {
        if let Some(op_id) = operation_id {
            // Get specific operation status
            if let Some(operation) = self.task_manager.get_operation(&op_id).await {
                EnhancedSleepStatus {
                    is_sleeping: operation.status == OperationStatus::Running,
                    active_operations: if operation.status == OperationStatus::Running {
                        1
                    } else {
                        0
                    },
                    operations: vec![operation.clone()],
                    current_operation: Some(operation),
                }
            } else {
                EnhancedSleepStatus {
                    is_sleeping: false,
                    active_operations: 0,
                    operations: vec![],
                    current_operation: None,
                }
            }
        } else {
            // Get all operations status
            let all_operations = self.task_manager.get_all_operations().await;
            let active_operations = self.task_manager.get_active_operations().await;
            let current_operation = active_operations.first().cloned();

            EnhancedSleepStatus {
                is_sleeping: !active_operations.is_empty(),
                active_operations: active_operations.len(),
                operations: all_operations,
                current_operation,
            }
        }
    }

    /// Cancel a specific operation
    pub async fn cancel_operation(&self, operation_id: &str) -> SleepServerResult<bool> {
        self.task_manager
            .cancel_operation(operation_id)
            .await
            .map_err(|e| SleepServerError::InvalidDuration { duration: e })
    }

    /// Cancel all active operations
    pub async fn cancel_all_operations(&self) -> usize {
        self.task_manager.cancel_all_operations().await
    }

    /// Clean up old completed operations
    pub async fn cleanup_old_operations(&self) {
        self.task_manager.cleanup_old_operations().await;
    }

    /// Get current sleep status (legacy method for backward compatibility)
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
