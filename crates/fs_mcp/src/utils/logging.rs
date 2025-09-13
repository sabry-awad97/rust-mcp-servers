use tracing_subscriber::{EnvFilter, prelude::*};

use crate::errors::{FileSystemMcpError, FileSystemMcpResult};

/// Initialize logging based on environment configuration
///
/// This function follows the Open/Closed Principle by being open for extension
/// but closed for modification of core logging logic.
///
/// # Environment Variables
/// - `RUST_LOG`: Controls logging verbosity (trace, debug, info, warn, error)
///
/// # Returns
/// - `Ok(())` if logging is successfully initialized or skipped
/// - `Err(FileSystemMcpError::LoggingInitialization)` if initialization fails
pub fn init_logging() -> FileSystemMcpResult<()> {
    // Check if RUST_LOG is set, skip logging if not
    if std::env::var("RUST_LOG").is_err() {
        return Ok(());
    }

    // Use EnvFilter to automatically parse RUST_LOG environment variable
    let env_filter = EnvFilter::from_default_env();

    // Use pretty format with colors enabled by default
    let fmt_layer = tracing_subscriber::fmt::layer().with_ansi(true).pretty();

    let subscriber = tracing_subscriber::registry()
        .with(fmt_layer)
        .with(env_filter);

    subscriber
        .try_init()
        .map_err(|e| FileSystemMcpError::LoggingInitialization(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test environment variable logging setup
    #[test]
    fn test_env_logging_setup() {
        // Test without RUST_LOG - should succeed (no logging)
        let result = init_logging();
        assert!(result.is_ok());
    }
}
