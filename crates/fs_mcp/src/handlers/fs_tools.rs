use std::path::PathBuf;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router,
};

use crate::{errors::ToolResult, models::requests::ReadTextFileRequest};

/// Filesystem MCP Service
///
/// Provides secure filesystem operations through the MCP protocol
#[derive(Debug)]
pub struct FileSystemService {
    allowed_directories: Vec<PathBuf>,
    tool_router: ToolRouter<FileSystemService>,
}

impl FileSystemService {
    /// Create a new FileSystemService with the given configuration
    pub fn new(allowed_directories: Vec<PathBuf>) -> Self {
        Self {
            allowed_directories,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl FileSystemService {
    #[tool(description = "Read the complete contents of a file from the file system as text")]
    async fn read_text_file(&self, Parameters(req): Parameters<ReadTextFileRequest>) -> ToolResult {
        unimplemented!("Read text file not implemented yet {req:?}")
    }
}

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
