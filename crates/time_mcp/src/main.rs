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
    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting Time MCP server");

    if let Err(e) = server::run().await {
        tracing::error!("Error running Time MCP server: {}", e);
        return Err(e);
    }

    Ok(())
}
