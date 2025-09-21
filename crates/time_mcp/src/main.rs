use std::env;
use tracing_subscriber::{self, EnvFilter};

mod core;
mod server;

/// Time MCP Server
///
/// A comprehensive example MCP server demonstrating:
/// - Tools: Timezone operations and time conversion
/// - Resources: Server status and help documentation
///
/// Usage: npx @modelcontextprotocol/inspector cargo run --bin mcp_server_time
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging only if LOG_LEVEL environment variable is set
    if let Ok(log_level) = env::var("LOG_LEVEL") {
        // Initialize the tracing subscriber with file and stdout logging
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new(&log_level))
            )
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .init();

        tracing::info!("Starting Time MCP server with log level: {}", log_level);
    }

    if let Err(e) = server::run().await {
        // Only log error if logging is initialized
        if env::var("LOG_LEVEL").is_ok() {
            tracing::error!("Error running Time MCP server: {}", e);
        }
        return Err(e);
    }

    Ok(())
}
