use crate::core::models::{OperationStatus, SleepOperation};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Handle for managing a background sleep task
#[derive(Debug)]
pub struct TaskHandle {
    /// Task join handle
    pub handle: JoinHandle<()>,
    /// Cancellation token
    pub cancel_token: tokio_util::sync::CancellationToken,
}

/// Background task manager for sleep operations
#[derive(Debug, Clone)]
pub struct TaskManager {
    /// Active operations indexed by operation ID
    operations: Arc<RwLock<HashMap<String, SleepOperation>>>,
    /// Active task handles indexed by operation ID
    task_handles: Arc<Mutex<HashMap<String, TaskHandle>>>,
    /// Maximum number of concurrent operations
    max_concurrent_operations: usize,
    /// Maximum operation history to keep
    max_history_size: usize,
}

impl TaskManager {
    /// Create a new task manager
    pub fn new() -> Self {
        Self {
            operations: Arc::new(RwLock::new(HashMap::new())),
            task_handles: Arc::new(Mutex::new(HashMap::new())),
            max_concurrent_operations: 10,
            max_history_size: 100,
        }
    }

    /// Create a new task manager with custom limits
    pub fn with_limits(max_concurrent: usize, max_history: usize) -> Self {
        Self {
            operations: Arc::new(RwLock::new(HashMap::new())),
            task_handles: Arc::new(Mutex::new(HashMap::new())),
            max_concurrent_operations: max_concurrent,
            max_history_size: max_history,
        }
    }

    /// Start a new background sleep operation
    pub async fn start_sleep_operation(
        &self,
        duration: Duration,
        message: Option<String>,
    ) -> Result<String, String> {
        // Check concurrent operation limit
        let active_count = self.get_active_operation_count().await;
        if active_count >= self.max_concurrent_operations {
            return Err(format!(
                "Maximum concurrent operations ({}) exceeded",
                self.max_concurrent_operations
            ));
        }

        let operation_id = Uuid::new_v4().to_string();
        let start_time = Utc::now();
        let expected_end_time = start_time + chrono::Duration::from_std(duration).unwrap();

        // Create operation record
        let operation = SleepOperation {
            operation_id: operation_id.clone(),
            status: OperationStatus::Running,
            duration_ms: duration.as_millis() as u64,
            duration_str: crate::core::utils::format_duration(duration),
            start_time: start_time.to_rfc3339(),
            expected_end_time: expected_end_time.to_rfc3339(),
            actual_end_time: None,
            progress_percent: 0.0,
            remaining_ms: duration.as_millis() as u64,
            message: message.clone(),
            error: None,
        };

        // Store operation
        {
            let mut operations = self.operations.write().await;
            operations.insert(operation_id.clone(), operation);
        }

        // Create cancellation token
        let cancel_token = tokio_util::sync::CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();

        // Clone necessary data for the background task
        let operations_clone = Arc::clone(&self.operations);
        let task_handles_clone = Arc::clone(&self.task_handles);
        let op_id_clone = operation_id.clone();

        // Spawn background task
        let handle = tokio::spawn(async move {
            Self::execute_sleep_task(
                operations_clone,
                task_handles_clone,
                op_id_clone,
                duration,
                start_time,
                cancel_token_clone,
                message,
            )
            .await;
        });

        // Store task handle
        {
            let mut handles = self.task_handles.lock().await;
            handles.insert(
                operation_id.clone(),
                TaskHandle {
                    handle,
                    cancel_token,
                },
            );
        }

        info!(
            operation_id = %operation_id,
            duration_ms = duration.as_millis(),
            "Started background sleep operation"
        );

        Ok(operation_id)
    }

    /// Execute a sleep task in the background
    async fn execute_sleep_task(
        operations: Arc<RwLock<HashMap<String, SleepOperation>>>,
        task_handles: Arc<Mutex<HashMap<String, TaskHandle>>>,
        operation_id: String,
        duration: Duration,
        start_time: DateTime<Utc>,
        cancel_token: tokio_util::sync::CancellationToken,
        message: Option<String>,
    ) {
        let update_interval = Duration::from_millis(100); // Update progress every 100ms
        let total_ms = duration.as_millis() as u64;
        let mut elapsed_ms = 0u64;

        debug!(
            operation_id = %operation_id,
            "Starting sleep task execution"
        );

        // Sleep with periodic progress updates
        while elapsed_ms < total_ms {
            // Check for cancellation
            if cancel_token.is_cancelled() {
                Self::update_operation_status(
                    &operations,
                    &operation_id,
                    OperationStatus::Cancelled,
                    Some("Operation was cancelled".to_string()),
                )
                .await;

                debug!(
                    operation_id = %operation_id,
                    "Sleep task cancelled"
                );
                break;
            }

            // Sleep for update interval or remaining time
            let sleep_duration =
                std::cmp::min(update_interval.as_millis() as u64, total_ms - elapsed_ms);

            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(sleep_duration)) => {
                    elapsed_ms += sleep_duration;

                    // Update progress
                    let progress = (elapsed_ms as f64 / total_ms as f64) * 100.0;
                    let remaining = total_ms - elapsed_ms;

                    Self::update_operation_progress(&operations, &operation_id, progress, remaining).await;
                }
                _ = cancel_token.cancelled() => {
                    Self::update_operation_status(
                        &operations,
                        &operation_id,
                        OperationStatus::Cancelled,
                        Some("Operation was cancelled".to_string()),
                    ).await;

                    debug!(
                        operation_id = %operation_id,
                        "Sleep task cancelled during execution"
                    );
                    break;
                }
            }
        }

        // Mark as completed if not cancelled
        if !cancel_token.is_cancelled() {
            let end_time = Utc::now();
            Self::complete_operation(&operations, &operation_id, end_time).await;

            info!(
                operation_id = %operation_id,
                duration_ms = total_ms,
                "Sleep operation completed successfully"
            );
        }

        // Clean up task handle
        {
            let mut handles = task_handles.lock().await;
            handles.remove(&operation_id);
        }
    }

    /// Update operation progress
    async fn update_operation_progress(
        operations: &Arc<RwLock<HashMap<String, SleepOperation>>>,
        operation_id: &str,
        progress_percent: f64,
        remaining_ms: u64,
    ) {
        let mut ops = operations.write().await;
        if let Some(operation) = ops.get_mut(operation_id) {
            operation.progress_percent = progress_percent;
            operation.remaining_ms = remaining_ms;
        }
    }

    /// Update operation status
    async fn update_operation_status(
        operations: &Arc<RwLock<HashMap<String, SleepOperation>>>,
        operation_id: &str,
        status: OperationStatus,
        error: Option<String>,
    ) {
        let mut ops = operations.write().await;
        if let Some(operation) = ops.get_mut(operation_id) {
            operation.status = status;
            operation.error = error;
            if matches!(
                operation.status,
                OperationStatus::Cancelled | OperationStatus::Failed
            ) {
                operation.actual_end_time = Some(Utc::now().to_rfc3339());
            }
        }
    }

    /// Complete an operation
    async fn complete_operation(
        operations: &Arc<RwLock<HashMap<String, SleepOperation>>>,
        operation_id: &str,
        end_time: DateTime<Utc>,
    ) {
        let mut ops = operations.write().await;
        if let Some(operation) = ops.get_mut(operation_id) {
            operation.status = OperationStatus::Completed;
            operation.progress_percent = 100.0;
            operation.remaining_ms = 0;
            operation.actual_end_time = Some(end_time.to_rfc3339());
        }
    }

    /// Cancel a specific operation
    pub async fn cancel_operation(&self, operation_id: &str) -> Result<bool, String> {
        let mut handles = self.task_handles.lock().await;

        if let Some(task_handle) = handles.get(operation_id) {
            task_handle.cancel_token.cancel();
            info!(
                operation_id = %operation_id,
                "Cancelled sleep operation"
            );
            Ok(true)
        } else {
            warn!(
                operation_id = %operation_id,
                "Attempted to cancel non-existent operation"
            );
            Ok(false)
        }
    }

    /// Cancel all active operations
    pub async fn cancel_all_operations(&self) -> usize {
        let mut handles = self.task_handles.lock().await;
        let count = handles.len();

        for (operation_id, task_handle) in handles.iter() {
            task_handle.cancel_token.cancel();
            debug!(
                operation_id = %operation_id,
                "Cancelled operation during shutdown"
            );
        }

        handles.clear();

        if count > 0 {
            info!(cancelled_count = count, "Cancelled all active operations");
        }

        count
    }

    /// Get operation by ID
    pub async fn get_operation(&self, operation_id: &str) -> Option<SleepOperation> {
        let operations = self.operations.read().await;
        operations.get(operation_id).cloned()
    }

    /// Get all operations
    pub async fn get_all_operations(&self) -> Vec<SleepOperation> {
        let operations = self.operations.read().await;
        operations.values().cloned().collect()
    }

    /// Get active operations only
    pub async fn get_active_operations(&self) -> Vec<SleepOperation> {
        let operations = self.operations.read().await;
        operations
            .values()
            .filter(|op| op.status == OperationStatus::Running)
            .cloned()
            .collect()
    }

    /// Get count of active operations
    pub async fn get_active_operation_count(&self) -> usize {
        let operations = self.operations.read().await;
        operations
            .values()
            .filter(|op| op.status == OperationStatus::Running)
            .count()
    }

    /// Clean up old completed operations
    pub async fn cleanup_old_operations(&self) {
        let mut operations = self.operations.write().await;

        if operations.len() <= self.max_history_size {
            return;
        }

        // Keep active operations and recent completed ones
        let mut to_remove = Vec::new();
        let mut completed_ops: Vec<_> = operations
            .iter()
            .filter(|(_, op)| {
                matches!(
                    op.status,
                    OperationStatus::Completed
                        | OperationStatus::Cancelled
                        | OperationStatus::Failed
                )
            })
            .map(|(id, op)| (id.clone(), op.clone()))
            .collect();

        // Sort by completion time (most recent first)
        completed_ops.sort_by(|a, b| {
            let time_a = a.1.actual_end_time.as_ref().unwrap_or(&a.1.start_time);
            let time_b = b.1.actual_end_time.as_ref().unwrap_or(&b.1.start_time);
            time_b.cmp(time_a)
        });

        // Keep only the most recent completed operations
        let keep_completed = self.max_history_size / 2; // Keep half for completed operations
        if completed_ops.len() > keep_completed {
            for (id, _) in completed_ops.iter().skip(keep_completed) {
                to_remove.push(id.clone());
            }
        }

        // Remove old operations
        for id in to_remove {
            operations.remove(&id);
        }

        debug!(
            total_operations = operations.len(),
            max_history = self.max_history_size,
            "Cleaned up old operations"
        );
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_task_manager_creation() {
        let manager = TaskManager::new();
        assert_eq!(manager.get_active_operation_count().await, 0);
    }

    #[tokio::test]
    async fn test_start_sleep_operation() {
        let manager = TaskManager::new();
        let duration = Duration::from_millis(100);

        let operation_id = manager
            .start_sleep_operation(duration, Some("Test operation".to_string()))
            .await
            .unwrap();

        assert!(!operation_id.is_empty());
        assert_eq!(manager.get_active_operation_count().await, 1);

        // Wait for completion
        timeout(Duration::from_secs(1), async {
            loop {
                if manager.get_active_operation_count().await == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("Operation should complete");

        let operation = manager.get_operation(&operation_id).await.unwrap();
        assert_eq!(operation.status, OperationStatus::Completed);
    }

    #[tokio::test]
    async fn test_cancel_operation() {
        let manager = TaskManager::new();
        let duration = Duration::from_secs(10); // Long duration

        let operation_id = manager.start_sleep_operation(duration, None).await.unwrap();

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        let cancelled = manager.cancel_operation(&operation_id).await.unwrap();
        assert!(cancelled);

        // Wait for cancellation to take effect
        timeout(Duration::from_secs(1), async {
            loop {
                if let Some(op) = manager.get_operation(&operation_id).await {
                    if op.status == OperationStatus::Cancelled {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("Operation should be cancelled");
    }

    #[tokio::test]
    async fn test_concurrent_operations_limit() {
        let manager = TaskManager::with_limits(2, 10);
        let duration = Duration::from_millis(200);

        // Start two operations (should succeed)
        let _op1 = manager.start_sleep_operation(duration, None).await.unwrap();
        let _op2 = manager.start_sleep_operation(duration, None).await.unwrap();

        // Third operation should fail
        let result = manager.start_sleep_operation(duration, None).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Maximum concurrent operations"));
    }

    #[tokio::test]
    async fn test_cancel_all_operations() {
        let manager = TaskManager::new();
        let duration = Duration::from_secs(10);

        // Start multiple operations
        let _op1 = manager.start_sleep_operation(duration, None).await.unwrap();
        let _op2 = manager.start_sleep_operation(duration, None).await.unwrap();

        assert_eq!(manager.get_active_operation_count().await, 2);

        let cancelled_count = manager.cancel_all_operations().await;
        assert_eq!(cancelled_count, 2);

        // Wait for cancellations to take effect
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(manager.get_active_operation_count().await, 0);
    }
}
