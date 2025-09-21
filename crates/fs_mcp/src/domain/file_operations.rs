use async_trait::async_trait;
use std::path::{Path, PathBuf};

use crate::{
    errors::FileSystemMcpResult,
    models::{
        requests::SortBy,
        responses::{ReadFileResponse, WriteFileResponse},
    },
};

/// Domain trait for file operations
///
/// Provides a clean abstraction for different file operations,
/// enabling dependency injection and testability following SOLID principles.
#[async_trait]
pub trait FileOperations: Send + Sync {
    /// Read the entire contents of a file as a string
    ///
    /// # Arguments
    /// * `path` - The file path to read
    ///
    /// # Returns
    /// * `Ok(String)` - The complete file contents
    /// * `Err(FileSystemMcpError)` - If the file cannot be read
    async fn read_entire_file(&self, path: &Path) -> FileSystemMcpResult<ReadFileResponse>;

    /// Read the first N lines of a file
    ///
    /// # Arguments
    /// * `path` - The file path to read
    /// * `lines` - Number of lines to read from the beginning
    ///
    /// # Returns
    /// * `Ok(String)` - The first N lines joined with newlines
    /// * `Err(FileSystemMcpError)` - If the file cannot be read
    async fn read_file_head(
        &self,
        path: &Path,
        lines: usize,
    ) -> FileSystemMcpResult<ReadFileResponse>;

    /// Read the last N lines of a file using memory-efficient streaming
    ///
    /// # Arguments
    /// * `path` - The file path to read
    /// * `lines` - Number of lines to read from the end
    ///
    /// # Returns
    /// * `Ok(String)` - The last N lines joined with newlines
    /// * `Err(FileSystemMcpError)` - If the file cannot be read
    async fn read_file_tail(
        &self,
        path: &Path,
        lines: usize,
    ) -> FileSystemMcpResult<ReadFileResponse>;

    /// Read the entire contents of a file as a string
    ///
    /// # Arguments
    /// * `path` - The file path to read
    ///
    /// # Returns
    /// * `Ok(ReadMediaFileResponse)` - The complete file contents as base64 encoded data and MIME type
    /// * `Err(FileSystemMcpError)` - If the file cannot be read
    async fn read_media_file(&self, path: &Path) -> FileSystemMcpResult<ReadFileResponse>;

    /// Read files concurrently using futures::join_all for scalability with many files
    ///
    /// # Arguments
    /// * `paths` - A slice of file paths to read
    ///
    /// # Returns
    /// * `Vec<FileSystemMcpResult<ReadFileResponse>>` - A vector of results for each file read operation
    async fn read_files(
        &self,
        paths: &[std::path::PathBuf],
    ) -> Vec<FileSystemMcpResult<crate::models::responses::ReadFileResponse>>;
    /// Write content to a file, creating it if it doesn't exist
    ///
    /// # Arguments
    /// * `path` - The file path to write to
    /// * `content` - The content to write to the file
    ///
    /// # Returns
    /// * `Ok(WriteFileResponse)` - Success response with file information
    /// * `Err(FileSystemMcpError)` - If the file cannot be written
    async fn write_file(
        &self,
        path: &Path,
        content: &str,
    ) -> FileSystemMcpResult<WriteFileResponse>;

    /// Create a new directory and all necessary parent directories
    ///
    /// # Arguments
    /// * `path` - The directory path to create
    ///
    /// # Returns
    /// * `Ok(WriteFileResponse)` - Success response with directory information
    /// * `Err(FileSystemMcpError)` - If the directory cannot be created
    async fn create_directory(&self, path: &Path) -> FileSystemMcpResult<WriteFileResponse>;

    /// List the contents of a directory
    ///
    /// # Arguments
    /// * `path` - The directory path to list
    ///
    /// # Returns
    /// * `Ok(ListDirectoryResponse)` - Success response with directory contents
    /// * `Err(FileSystemMcpError)` - If the directory cannot be listed
    async fn list_directory(&self, path: &Path) -> FileSystemMcpResult<WriteFileResponse>;

    /// List the contents of a directory with sizes
    ///
    /// # Arguments
    /// * `path` - The directory path to list
    ///
    /// # Returns
    /// * `Ok(ListDirectoryResponse)` - Success response with directory contents
    /// * `Err(FileSystemMcpError)` - If the directory cannot be listed
    async fn list_directory_with_sizes(
        &self,
        path: &Path,
        sort_by: &SortBy,
    ) -> FileSystemMcpResult<WriteFileResponse>;

    /// List the contents of a directory as a JSON tree
    ///
    /// # Arguments
    /// * `path` - The directory path to list
    /// * `exclude_patterns` - Patterns to exclude from the tree
    ///
    /// # Returns
    /// * `Ok(ListDirectoryResponse)` - Success response with directory contents
    /// * `Err(FileSystemMcpError)` - If the directory cannot be listed
    async fn directory_tree(
        &self,
        path: &Path,
        exclude_patterns: &[String],
    ) -> FileSystemMcpResult<WriteFileResponse>;

    /// Move/rename a file or directory
    ///
    /// # Arguments
    /// * `from` - The source path
    /// * `to` - The destination path
    ///
    /// # Returns
    /// * `Ok(WriteFileResponse)` - Success response
    /// * `Err(FileSystemMcpError)` - If the move operation fails
    async fn move_file(&self, from: &Path, to: &Path) -> FileSystemMcpResult<WriteFileResponse>;

    /// Search for files and directories matching a pattern
    ///
    /// # Arguments
    /// * `path` - The directory path to search
    /// * `pattern` - The pattern to search for
    /// * `allowed_directories` - List of allowed directories
    /// * `exclude_patterns` - Patterns to exclude from the search
    ///
    /// # Returns
    /// * `Ok(ListDirectoryResponse)` - Success response with directory contents
    /// * `Err(FileSystemMcpError)` - If the directory cannot be listed
    async fn search_files(
        &self,
        path: &Path,
        pattern: &str,
        allowed_directories: &[PathBuf],
        exclude_patterns: &[String],
    ) -> FileSystemMcpResult<WriteFileResponse>;

    /// Get file information
    ///
    /// # Arguments
    /// * `path` - The file path to get information for
    ///
    /// # Returns
    /// * `Ok(WriteFileResponse)` - Success response with file information
    /// * `Err(FileSystemMcpError)` - If the file cannot be retrieved
    async fn get_file_info(&self, path: &Path) -> FileSystemMcpResult<WriteFileResponse>;

    /// Apply multiple edit operations to a file
    ///
    /// # Arguments
    /// * `path` - The file path to edit
    /// * `edits` - Array of edit operations to apply
    /// * `dry_run` - If true, return preview without modifying file
    ///
    /// # Returns
    /// * `Ok(WriteFileResponse)` - Success response with edit information
    /// * `Err(FileSystemMcpError)` - If the edits cannot be applied
    async fn apply_file_edits(
        &self,
        path: &Path,
        edits: &[crate::models::requests::EditOperation],
        dry_run: &bool,
    ) -> FileSystemMcpResult<WriteFileResponse>;
}
