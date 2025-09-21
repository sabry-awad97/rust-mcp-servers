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
    application::FileService,
    domain::FileOperations,
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
    file_operations: Arc<dyn FileOperations>,
    tool_router: ToolRouter<FileSystemService>,
}

impl FileSystemService {
    /// Create a new FileSystemService with the given configuration and file reader
    pub fn new(allowed_directories: Vec<PathBuf>) -> Self {
        Self {
            allowed_directories,
            file_operations: Arc::new(FileService::new()),
            tool_router: Self::tool_router(),
        }
    }

    fn create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    fn generate_status_content(&self) -> String {
        format!(
            r#"Filesystem MCP Server Status

Server: Running
Allowed Directories: {}
Total Allowed Paths: {}
Tools Available: 13
Resources Available: 3

Capabilities:
- Secure file reading (text and media files)
- File writing and editing with line-based operations
- Directory management and navigation
- File search with pattern matching and exclusions
- File metadata and information retrieval
- File operations (move, rename, copy)
- Directory tree visualization
- Security through directory allowlisting

Security Model:
- All operations restricted to allowed directories
- Path validation and normalization
- Symlink handling with warnings
- Input sanitization and validation"#,
            self.allowed_directories
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.allowed_directories.len()
        )
    }

    fn generate_help_content(&self) -> String {
        format!(
            r#"Filesystem MCP Server Help

TOOLS:

FILE READING:
- read_text_file: Read complete file contents or specific lines
  - path: File path (required)
  - head: Read first N lines (optional)
  - tail: Read last N lines (optional)
  - Example: {{"path": "/project/README.md", "head": 10}}

- read_media_file: Read image/audio files as base64 with MIME type
  - path: Media file path (required)
  - Example: {{"path": "/images/photo.jpg"}}

- read_multiple_files: Read multiple files simultaneously
  - paths: Array of file paths (required)
  - Example: {{"paths": ["/config.json", "/settings.yaml"]}}

FILE WRITING:
- write_file: Create or overwrite file with content
  - path: File path (required)
  - content: File content (required)
  - Example: {{"path": "/project/new_file.txt", "content": "Hello World"}}

- edit_file: Make line-based edits with git-style diff
  - path: File path (required)
  - edits: Array of edit operations (required)
  - dry_run: Preview changes without applying (optional)
  - Example: {{"path": "/config.py", "edits": [{{"old_text": "DEBUG = False", "new_text": "DEBUG = True"}}]}}

DIRECTORY OPERATIONS:
- create_directory: Create directory and parent directories
  - path: Directory path (required)
  - Example: {{"path": "/project/new_folder/subfolder"}}

- list_directory: List directory contents
  - path: Directory path (required)
  - Example: {{"path": "/project"}}

- list_directory_with_sizes: List directory with file sizes and sorting
  - path: Directory path (required)
  - sort_by: Sort criteria - "name", "size", "modified" (optional)
  - Example: {{"path": "/project", "sort_by": "size"}}

- directory_tree: Get recursive directory tree as JSON
  - path: Root directory path (required)
  - exclude_patterns: Glob patterns to exclude (optional)
  - Example: {{"path": "/project", "exclude_patterns": ["*.log", "node_modules/**"]}}

FILE MANAGEMENT:
- move_file: Move or rename files and directories
  - source: Source path (required)
  - destination: Destination path (required)
  - Example: {{"source": "/old_name.txt", "destination": "/new_name.txt"}}

- search_files: Search for files matching patterns
  - path: Search directory (required)
  - pattern: Glob pattern (required)
  - exclude_patterns: Patterns to exclude (optional)
  - Example: {{"path": "/project", "pattern": "*.rs", "exclude_patterns": ["target/**"]}}

- get_file_info: Get detailed file/directory metadata
  - path: File or directory path (required)
  - Example: {{"path": "/project/config.json"}}

UTILITY:
- list_allowed_directories: Show allowed directory paths
  - No parameters required

RESOURCES:
- fs://status: Current server status and configuration
- fs://help: This help documentation
- fs://allowed-directories: List of allowed directory paths

ALLOWED DIRECTORIES:
{}

SECURITY NOTES:
- All operations are restricted to allowed directories
- Paths are validated and normalized for security
- Symlinks are handled safely with warnings
- Error messages don't leak sensitive information

PATTERN SYNTAX:
- Use glob patterns: *.txt, **/*.rs, src/**
- Exclude patterns support: ["*.log", "target/**", ".git/**"]
- Case-sensitive matching on most systems

EXAMPLE WORKFLOWS:

1. Explore Project Structure:
   - Use directory_tree to get overview
   - Use list_directory for specific folders
   - Use search_files to find specific file types

2. Read and Edit Files:
   - Use read_text_file to examine content
   - Use edit_file for precise line-based changes
   - Use write_file for new files or complete rewrites

3. File Management:
   - Use move_file to organize files
   - Use get_file_info for metadata
   - Use create_directory for new folder structures"#,
            self.allowed_directories
                .iter()
                .map(|p| format!("- {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    fn generate_allowed_directories_content(&self) -> String {
        format!(
            r#"Allowed Directories Configuration

The Filesystem MCP Server is configured with the following allowed directories.
All file operations are restricted to these paths and their subdirectories.

ALLOWED PATHS:
{}

TOTAL DIRECTORIES: {}

SECURITY INFORMATION:
- All file paths are validated against these allowed directories
- Operations outside these paths will be rejected
- Symlinks pointing outside allowed directories trigger warnings
- Path traversal attempts (../) are blocked
- Relative paths are resolved within allowed directories

USAGE NOTES:
- Use absolute paths when possible for clarity
- All subdirectories within allowed paths are accessible
- Hidden files and directories (starting with .) are accessible
- File permissions are respected by the underlying filesystem

To modify allowed directories, restart the server with different --allowed-dir arguments."#,
            if self.allowed_directories.is_empty() {
                "  No directories currently allowed (server in restricted mode)".to_string()
            } else {
                self.allowed_directories
                    .iter()
                    .enumerate()
                    .map(|(i, p)| format!("  {}. {}", i + 1, p.display()))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            self.allowed_directories.len()
        )
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
                self.file_operations
                    .read_file_head(&path, *head_lines)
                    .await?
            }
            (None, Some(tail_lines)) => {
                // Return the last N lines of the file
                self.file_operations
                    .read_file_tail(&path, *tail_lines)
                    .await?
            }
            (None, None) => {
                // Return the entire file
                self.file_operations.read_entire_file(&path).await?
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

        let content = self.file_operations.read_media_file(&path).await?;

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
        let results = self.file_operations.read_files(&validated_paths).await;

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
            .file_operations
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
            .file_operations
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
        let result = self.file_operations.create_directory(&valid_path).await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(description = "Get a detailed listing of all files and directories in a specified path")]
    async fn list_directory(
        &self,
        Parameters(req): Parameters<ListDirectoryRequest>,
    ) -> ToolResult {
        req.validate()?;
        let valid_path = validate_path(req.path(), &self.allowed_directories).await?;
        let result = self.file_operations.list_directory(&valid_path).await?;
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
            .file_operations
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
            .file_operations
            .directory_tree(&valid_path, req.exclude_patterns())
            .await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(description = "Move or rename files and directories")]
    async fn move_file(&self, Parameters(req): Parameters<MoveFileRequest>) -> ToolResult {
        req.validate()?;
        let valid_from = validate_path(req.source(), &self.allowed_directories).await?;
        let valid_to = validate_path(req.destination(), &self.allowed_directories).await?;
        let result = self
            .file_operations
            .move_file(&valid_from, &valid_to)
            .await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(description = "Search for files and directories matching a pattern")]
    async fn search_files(&self, Parameters(req): Parameters<SearchFilesRequest>) -> ToolResult {
        req.validate()?;
        let valid_path = validate_path(req.path(), &self.allowed_directories).await?;
        let result = self
            .file_operations
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
        let result = self.file_operations.get_file_info(&valid_path).await?;
        Ok(CallToolResult::success(vec![result.into()]))
    }

    #[tool(description = "Returns the list of directories that this server is allowed to access")]
    async fn list_allowed_directories(&self) -> ToolResult {
        let directories: Vec<String> = self
            .allowed_directories
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let result = format!("Allowed directories:\n{}", directories.join("\n"));
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }
}

#[tool_handler]
impl ServerHandler for FileSystemService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("FileSystem MCP Server for secure file operations. Tools: read_text_file, read_media_file, read_multiple_files, write_file, edit_file, create_directory, list_directory, list_directory_with_sizes, directory_tree, move_file, search_files, get_file_info, list_allowed_directories. All operations are restricted to allowed directories for security. Resources: fs://status, fs://help, fs://allowed-directories.".to_string()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                self.create_resource_text("fs://status", "server-status"),
                self.create_resource_text("fs://help", "help-documentation"),
                self.create_resource_text("fs://allowed-directories", "allowed-directories-list"),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match uri.as_str() {
            "fs://status" => {
                let status = self.generate_status_content();
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(status, uri)],
                })
            }
            "fs://help" => {
                let help = self.generate_help_content();
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(help, uri)],
                })
            }
            "fs://allowed-directories" => {
                let directories = self.generate_allowed_directories_content();
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(directories, uri)],
                })
            }
            _ => Err(FileSystemMcpError::ValidationError {
                message: format!("Resource not found: {}", uri),
                path: uri.to_string(),
                operation: "read_resource".to_string(),
                data: serde_json::json!({
                    "available_resources": ["fs://status", "fs://help", "fs://allowed-directories"]
                }),
            }
            .into()),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
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
