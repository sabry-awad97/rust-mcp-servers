use core::fmt;
use std::path::PathBuf;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router,
};

use crate::{
    application::FileReaderService,
    domain::FileReader,
    errors::{FileSystemMcpError, ToolResult},
    models::requests::ReadTextFileRequest,
    service::validation::{Validate, validate_path},
};
use std::sync::Arc;

/// Filesystem MCP Service
///
/// Provides secure filesystem operations through the MCP protocol
/// Uses dependency injection for file reading operations
pub struct FileSystemService {
    allowed_directories: Vec<PathBuf>,
    file_reader: Arc<dyn FileReader>,
    tool_router: ToolRouter<FileSystemService>,
}

impl FileSystemService {
    /// Create a new FileSystemService with the given configuration and file reader
    pub fn new(allowed_directories: Vec<PathBuf>) -> Self {
        Self {
            allowed_directories,
            file_reader: Arc::new(FileReaderService::new()),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl FileSystemService {
    #[tool(description = "Read the complete contents of a file from the file system as text")]
    async fn read_text_file(&self, Parameters(req): Parameters<ReadTextFileRequest>) -> ToolResult {
        // Validate request parameters
        req.validate()?;

        // Validate and resolve the file path
        let path = validate_path(req.path(), &self.allowed_directories).await?;

        // Read file content based on request parameters using injected file reader
        let content = match (req.head(), req.tail()) {
            (Some(head_lines), None) => {
                // Return the first N lines of the file
                self.file_reader.read_file_head(&path, *head_lines).await?
            }
            (None, Some(tail_lines)) => {
                // Return the last N lines of the file
                self.file_reader.read_file_tail(&path, *tail_lines).await?
            }
            (None, None) => {
                // Return the entire file
                self.file_reader.read_entire_file(&path).await?
            }
            (Some(_), Some(_)) => {
                // This should be caught by validation, but handle gracefully
                return Err(FileSystemMcpError::ValidationError {
                    message: "Cannot specify both head and tail parameters".to_string(),
                    path: path.display().to_string(),
                    operation: "read_text_file".to_string(),
                    data: serde_json::json!({"error": "Conflicting parameters"}),
                }
                .into());
            }
        };

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }
}

// File reading operations are now handled by the injected FileReader service
// This follows the Single Responsibility Principle and Dependency Inversion Principle

#[tool_handler]
impl ServerHandler for FileSystemService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("FileSystem MCP Server for secure file operations. Tools: read_text_file, read_media_file, read_multiple_files, write_file, edit_file, create_directory, list_directory, list_directory_with_sizes, directory_tree, move_file, search_files, get_file_info, list_allowed_directories. All operations are restricted to allowed directories for security.".to_string()),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        tracing::info!("FileSystem MCP Server initialized successfully");
        Ok(self.get_info())
    }
}

impl fmt::Debug for FileSystemService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileSystemService")
            .field("allowed_directories", &self.allowed_directories)
            .finish()
    }
}
