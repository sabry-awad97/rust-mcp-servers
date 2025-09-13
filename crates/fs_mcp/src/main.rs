#![allow(unused)]

mod cli;
mod config;
mod errors;
mod handlers;
mod models;
mod service;
mod utils;

use cli::Cli;
use handlers::run;
use utils::logging;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let cli = Cli::parse_config().await?;

    // Initialize logging based on environment
    logging::init_logging()?;

    // Run the MCP server
    if let Err(e) = run(cli).await {
        tracing::error!("Failed to run MCP server: {}", e);
        return Err(e);
    }

    Ok(())
}
