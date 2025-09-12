use chrono::Utc;
use std::time::Duration;

use crate::core::{
    error::{SleepServerError, SleepServerResult},
    models::{EnhancedSleepStatus, OperationStatus, SleepResult, SleepStartResponse},
    task_manager::TaskManager,
    utils::{format_duration, parse_duration, parse_iso8601},
};

/// Sleep server implementation with non-blocking background task management
#[derive(Clone)]
pub struct SleepServer {
    /// Background task manager for non-blocking operations
    task_manager: TaskManager,
}

impl SleepServer {
    pub fn new() -> Self {
        Self {
            task_manager: TaskManager::new(),
        }
    }

    /// Sleep for a specified duration (non-blocking - returns immediately with operation ID)
    pub async fn sleep_for(
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

    /// Sleep until a specific time (non-blocking - returns immediately with operation ID)
    pub async fn sleep_until(
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

    /// Get sleep status with background operation tracking
    pub async fn get_status(
        &self,
        _detailed: bool,
        operation_id: Option<String>,
    ) -> EnhancedSleepStatus {
        // Periodically clean up old operations
        self.task_manager.cleanup_old_operations().await;

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

    /// Blocking sleep for a specified duration
    pub async fn sleep_blocking(
        &self,
        duration_str: &str,
        message: Option<String>,
    ) -> SleepServerResult<SleepResult> {
        let duration = parse_duration(duration_str)?;
        let start_time = Utc::now();

        // Perform the actual blocking sleep
        tokio::time::sleep(duration).await;

        let end_time = Utc::now();

        Ok(SleepResult {
            duration_ms: duration.as_millis() as u64,
            duration_str: format_duration(duration),
            start_time: start_time.to_rfc3339(),
            end_time: end_time.to_rfc3339(),
            completed: true,
            message,
        })
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
        let status = server.get_status(false, None).await;
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
        assert!(!sleep_result.operation_id.is_empty());
        assert!(sleep_result.message.contains("Sleep operation started"));
    }

    #[tokio::test]
    async fn test_sleep_status_tracking() {
        let server = SleepServer::new();

        // Start a sleep operation and get its ID
        let sleep_result = server.sleep_for("200ms", None).await.unwrap();
        let operation_id = sleep_result.operation_id.clone();

        // Give it a moment to start
        tokio::time::sleep(TokioDuration::from_millis(50)).await;

        // Check status while running
        let status = server.get_status(true, Some(operation_id.clone())).await;
        assert!(status.is_sleeping);
        assert!(status.current_operation.is_some());

        // Wait for the operation to complete
        tokio::time::sleep(TokioDuration::from_millis(300)).await;

        // Check status after completion
        let status = server.get_status(false, Some(operation_id)).await;
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
        let result = server.cancel_all_operations().await;
        assert_eq!(result, 0);

        // Start a sleep and cancel it
        let server_clone = server.clone();
        let _sleep_handle = tokio::spawn(async move { server_clone.sleep_for("1s", None).await });

        // Give it a moment to start
        tokio::time::sleep(TokioDuration::from_millis(10)).await;

        // Cancel the sleep
        let result = server.cancel_all_operations().await;
        assert!(result > 0);
    }

    #[tokio::test]
    async fn test_sleep_blocking() {
        let server = SleepServer::new();

        let start_time = std::time::Instant::now();
        let result = server
            .sleep_blocking("100ms", Some("Blocking test".to_string()))
            .await;
        let elapsed = start_time.elapsed();

        assert!(result.is_ok());
        let sleep_result = result.unwrap();
        assert_eq!(sleep_result.duration_ms, 100);
        assert!(sleep_result.completed);
        assert_eq!(sleep_result.message, Some("Blocking test".to_string()));
        assert!(elapsed.as_millis() >= 100); // Should have actually waited
    }
}
