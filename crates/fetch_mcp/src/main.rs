use clap::Parser;
use tracing_subscriber::EnvFilter;

mod errors;
mod models;
mod server;
mod services;
mod utils;

#[derive(Parser, Debug)]
#[command(name = "fetch-server")]
#[command(about = "MCP Fetch Server for web content retrieval")]
struct Args {
    /// Custom User-Agent string to use for requests
    #[arg(long)]
    user_agent: Option<String>,

    /// Ignore robots.txt restrictions
    #[arg(long)]
    ignore_robots_txt: bool,

    /// Proxy URL to use for requests (e.g., http://proxy:8080)
    #[arg(long)]
    proxy_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging only if LOG_LEVEL environment variable is set
    if let Ok(log_level) = std::env::var("LOG_LEVEL") {
        // Initialize the tracing subscriber with file and stdout logging
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&log_level)),
            )
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .init();

        tracing::info!("Starting Fetch MCP server with log level: {}", log_level);
    }

    let args = Args::parse();

    if let Some(ref user_agent) = args.user_agent {
        tracing::info!("Using custom user agent: {}", user_agent);
    }

    if args.ignore_robots_txt {
        tracing::info!("Ignoring robots.txt restrictions");
    }

    if let Some(ref proxy) = args.proxy_url {
        tracing::info!("Using proxy: {}", proxy);
    }

    // Run the MCP server
    if let Err(e) = server::run(args.user_agent, args.ignore_robots_txt, args.proxy_url).await {
        tracing::error!("Failed to run MCP server: {}", e);
        return Err(e);
    }

    Ok(())
}
