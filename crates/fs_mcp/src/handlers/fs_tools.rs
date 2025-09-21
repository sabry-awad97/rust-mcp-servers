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
    application::{FileReaderService, FileWriterService},
    domain::{FileReader, FileWriter},
    errors::{FileSystemMcpError, ToolResult},
    models::requests::{
        CreateDirectoryRequest, DirectoryTreeRequest, EditFileRequest, GetFileInfoRequest,
        ListDirectoryRequest, ListDirectoryWithSizesRequest, MoveFileRequest, ReadMediaFileRequest,
        ReadMultipleFilesRequest, ReadTextFileRequest, SearchFilesRequest, WriteFileRequest,
    },
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
    file_writer: Arc<dyn FileWriter>,
    tool_router: ToolRouter<FileSystemService>,
}

impl FileSystemService {
    /// Create a new FileSystemService with the given configuration and file reader
    pub fn new(allowed_directories: Vec<PathBuf>) -> Self {
        Self {
            allowed_directories,
            file_reader: Arc::new(FileReaderService::new()),
            file_writer: Arc::new(FileWriterService::new()),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl FileSystemService {
    #[tool(
        description = "Read the complete contents of a file from the file system as text. Handles various text encodings and provides detailed error messages if the file cannot be read. Use this tool when you need to examine the contents of a single file. Use the 'head' parameter to read only the first N lines of a file, or the 'tail' parameter to read only the last N lines of a file. Operates on the file as text regardless of extension. Only works within allowed directories."
    )]
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

        Ok(CallToolResult::success(vec![content.into()]))
    }

    #[tool(
        description = "Read an image or audio file and return base64 encoded data and MIME type. Only works within allowed directories."
    )]
    async fn read_media_file(
        &self,
        Parameters(req): Parameters<ReadMediaFileRequest>,
    ) -> ToolResult {
        req.validate()?;
        let path = validate_path(req.path(), &self.allowed_directories).await?;

        let content = self.file_reader.read_media_file(&path).await?;

        Ok(CallToolResult::success(vec![content.into()]))
    }

    #[tool(
        description = "Read the contents of multiple files simultaneously. This is more efficient than reading files one by one when you need to analyze or compare multiple files. Each file's content is returned with its path as a reference. Failed reads for individual files won't stop the entire operation. Only works within allowed directories."
    )]
    async fn read_multiple_files(
        &self,
        Parameters(req): Parameters<ReadMultipleFilesRequest>,
    ) -> ToolResult {
        req.validate()?;

        // Validate all paths first
        let mut validated_paths = Vec::new();
        for path_str in req.paths() {
            let path = validate_path(path_str, &self.allowed_directories).await?;
            validated_paths.push(path);
        }

        // Read files concurrently
        let results = self.file_reader.read_files(&validated_paths).await;

        // Collect successful results and handle errors
        let mut contents = Vec::new();
        let mut errors = Vec::new();

        for (result, path) in results.into_iter().zip(validated_paths.iter()) {
            match result {
                Ok(content) => contents.push(format!("{}:\n{}\n", path.display(), content)),
                Err(e) => errors.push(format!("Error reading {}: {}", path.display(), e)),
            }
        }

        // Add error summary if there were any errors
        if !errors.is_empty() {
            contents.push(format!("Errors encountered:\n{}", errors.join("\n")));
        }

        Ok(CallToolResult::success(vec![Content::text(
            contents.join("\n---\n"),
        )]))
    }

    #[tool(
        description = "Create a new file or completely overwrite an existing file with new content. Use with caution as it will overwrite existing files without warning. Handles text content with proper encoding. Only works within allowed directories."
    )]
    async fn write_file(&self, Parameters(req): Parameters<WriteFileRequest>) -> ToolResult {
        req.validate()?;
        let valid_path = validate_path(req.path(), &self.allowed_directories).await?;
        let result = self
            .file_writer
            .write_file(&valid_path, req.content())
            .await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(
        description = "Make line-based edits to a text file. Each edit replaces exact line sequences with new content. Returns a git-style diff showing the changes made. Only works within allowed directories."
    )]
    async fn edit_file(&self, Parameters(req): Parameters<EditFileRequest>) -> ToolResult {
        req.validate()?;
        let valid_path = validate_path(req.path(), &self.allowed_directories).await?;
        let result = self
            .file_writer
            .apply_file_edits(&valid_path, req.edits(), req.dry_run())
            .await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(
        description = "Create a new directory or ensure a directory exists. Can create multiple nested directories in one operation. If the directory already exists, this operation will succeed silently. Perfect for setting up directory structures for projects or ensuring required paths exist. Only works within allowed directories."
    )]
    async fn create_directory(
        &self,
        Parameters(req): Parameters<CreateDirectoryRequest>,
    ) -> ToolResult {
        req.validate()?;
        let valid_path = validate_path(req.path(), &self.allowed_directories).await?;
        let result = self.file_writer.create_directory(&valid_path).await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(description = "Get a detailed listing of all files and directories in a specified path")]
    async fn list_directory(
        &self,
        Parameters(req): Parameters<ListDirectoryRequest>,
    ) -> ToolResult {
        req.validate()?;
        let valid_path = validate_path(req.path(), &self.allowed_directories).await?;
        let result = self.file_writer.list_directory(&valid_path).await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(description = "Get a detailed listing with file sizes")]
    async fn list_directory_with_sizes(
        &self,
        Parameters(req): Parameters<ListDirectoryWithSizesRequest>,
    ) -> ToolResult {
        req.validate()?;
        let valid_path = validate_path(req.path(), &self.allowed_directories).await?;
        let result = self
            .file_writer
            .list_directory_with_sizes(&valid_path, req.sort_by())
            .await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(description = "Get a recursive tree view of files and directories as JSON")]
    async fn directory_tree(
        &self,
        Parameters(req): Parameters<DirectoryTreeRequest>,
    ) -> ToolResult {
        req.validate()?;
        let valid_path = validate_path(req.path(), &self.allowed_directories).await?;
        let result = self
            .file_writer
            .directory_tree(&valid_path, req.exclude_patterns())
            .await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(description = "Move or rename files and directories")]
    async fn move_file(&self, Parameters(req): Parameters<MoveFileRequest>) -> ToolResult {
        req.validate()?;
        let valid_from = validate_path(req.source(), &self.allowed_directories).await?;
        let valid_to = validate_path(req.destination(), &self.allowed_directories).await?;
        let result = self.file_writer.move_file(&valid_from, &valid_to).await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(description = "Search for files and directories matching a pattern")]
    async fn search_files(&self, Parameters(req): Parameters<SearchFilesRequest>) -> ToolResult {
        req.validate()?;
        let valid_path = validate_path(req.path(), &self.allowed_directories).await?;
        let result = self
            .file_writer
            .search_files(
                &valid_path,
                req.pattern(),
                &self.allowed_directories,
                req.exclude_patterns(),
            )
            .await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(description = "Retrieve detailed metadata about a file or directory")]
    async fn get_file_info(&self, Parameters(req): Parameters<GetFileInfoRequest>) -> ToolResult {
        req.validate()?;
        let valid_path = validate_path(req.path(), &self.allowed_directories).await?;
        let result = self.file_writer.get_file_info(&valid_path).await?;
        Ok(CallToolResult::success(vec![result.into()]))
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

impl fmt::Debug for FileSystemService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileSystemService")
            .field("allowed_directories", &self.allowed_directories)
            .finish()
    }
}
