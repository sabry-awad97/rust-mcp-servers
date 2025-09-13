use async_trait::async_trait;
use std::path::Path;

use crate::{errors::FileSystemMcpResult, models::responses::ReadFileResponse};

/// Domain trait for file reading operations
///
/// Provides a clean abstraction for different file reading strategies,
/// enabling dependency injection and testability following SOLID principles.
#[async_trait]
pub trait FileReader: Send + Sync {
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
}
