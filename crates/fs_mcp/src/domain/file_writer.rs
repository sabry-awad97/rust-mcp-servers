use async_trait::async_trait;
use std::path::{Path, PathBuf};

use crate::{
    errors::FileSystemMcpResult,
    models::{requests::SortBy, responses::WriteFileResponse},
};

/// Domain trait for file writing operations
///
/// Provides a clean abstraction for different file writing strategies,
/// enabling dependency injection and testability following SOLID principles.
#[async_trait]
pub trait FileWriter: Send + Sync {
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

    /// Delete a file
    ///
    /// # Arguments
    /// * `path` - The file path to delete
    ///
    /// # Returns
    /// * `Ok(WriteFileResponse)` - Success response
    /// * `Err(FileSystemMcpError)` - If the file cannot be deleted
    async fn delete_file(&self, path: &Path) -> FileSystemMcpResult<WriteFileResponse>;

    /// Delete a directory and all its contents recursively
    ///
    /// # Arguments
    /// * `path` - The directory path to delete
    ///
    /// # Returns
    /// * `Ok(WriteFileResponse)` - Success response
    /// * `Err(FileSystemMcpError)` - If the directory cannot be deleted
    async fn delete_directory(&self, path: &Path) -> FileSystemMcpResult<WriteFileResponse>;

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

    /// Copy a file to a new location
    ///
    /// # Arguments
    /// * `from` - The source file path
    /// * `to` - The destination file path
    ///
    /// # Returns
    /// * `Ok(WriteFileResponse)` - Success response
    /// * `Err(FileSystemMcpError)` - If the copy operation fails
    async fn copy_file(&self, from: &Path, to: &Path) -> FileSystemMcpResult<WriteFileResponse>;

    /// Write binary data to a file
    ///
    /// # Arguments
    /// * `path` - The file path to write to
    /// * `data` - The binary data to write
    ///
    /// # Returns
    /// * `Ok(WriteFileResponse)` - Success response with file information
    /// * `Err(FileSystemMcpError)` - If the file cannot be written
    async fn write_binary_file(
        &self,
        path: &Path,
        data: &[u8],
    ) -> FileSystemMcpResult<WriteFileResponse>;

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
