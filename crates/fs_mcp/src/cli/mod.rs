use std::path::PathBuf;

use clap::Parser;

use crate::config::Config;
use crate::errors::FileSystemMcpResult;
use crate::utils::fs::{resolve_directories, validate_directories};

/// Filesystem MCP Server
///
/// A secure Model Context Protocol server providing comprehensive filesystem operations
/// with built-in security validation and path traversal protection.
///
/// ## Features
/// - **Tools**: File operations, directory listing, and search capabilities
/// - **Security**: Path traversal protection and directory allowlist enforcement
/// - **Validation**: Canonical path validation and permission checks
///
/// ## Development
/// ```bash
/// npx @modelcontextprotocol/inspector cargo run --bin mcp-server-filesystem
/// ```
///
/// ## Configuration
/// Add to your MCP client configuration:
/// ```json
/// {
///   "mcpServers": {
///     "filesystem": {
///       "command": "mcp-server-filesystem",
///       "args": ["/allowed/directory"],
///       "env": {
///         "RUST_LOG": "info"
///       }
///     }
///   }
/// }
/// ```
///
/// ## Environment Variables
/// - `RUST_LOG`: Controls logging verbosity (trace, debug, info, warn, error)
#[derive(Parser, Debug, Clone)]
#[command(name = "mcp-server-filesystem")]
#[command(about = "A secure filesystem MCP server with comprehensive directory operations")]
#[command(version)]
#[command(
    long_about = "A Model Context Protocol (MCP) server that provides secure filesystem operations. \nSupports file reading, writing, directory listing, and search operations with built-in security validation."
)]
pub struct Cli {
    /// Allowed directories for filesystem operations.
    ///
    /// If not specified, uses current directory as default.
    /// Only operations within these directories (and subdirectories) are permitted.
    /// Supports both absolute and relative paths.
    #[arg(
        help = "Directories to allow filesystem operations in",
        value_name = "DIRECTORY",
        long_help = "Specify one or more directories where filesystem operations are allowed. \nAll operations are restricted to these directories and their subdirectories for security."
    )]
    pub directories: Vec<PathBuf>,
}

impl Cli {
    /// Parse CLI arguments and convert to configuration
    ///
    /// This method implements the Single Responsibility Principle by focusing
    /// solely on parsing and configuration creation.
    pub async fn parse_config() -> FileSystemMcpResult<Config> {
        let cli = Self::parse();
        let allowed_directories = resolve_directories(cli.directories).await?;
        validate_directories(&allowed_directories).await?;
        Ok(Config {
            allowed_directories,
        })
    }
}
