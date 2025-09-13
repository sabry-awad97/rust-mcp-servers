use async_trait::async_trait;
use std::collections::VecDeque;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

use crate::domain::file_reader::FileReader;
use crate::errors::{FileSystemMcpError, FileSystemMcpResult};

/// Concrete implementation of FileReader using streaming I/O operations
///
/// This service provides memory-efficient file reading capabilities using
/// asynchronous streaming patterns. It follows SOLID principles with
/// single responsibility for file reading operations.
pub struct FileReaderService;

impl FileReaderService {
    /// Create a new instance of FileReaderService
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileReaderService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileReader for FileReaderService {
    /// Read the entire contents of a file using buffered streaming
    async fn read_entire_file(&self, path: &Path) -> FileSystemMcpResult<String> {
        let mut file =
            File::open(path)
                .await
                .map_err(|_| FileSystemMcpError::PermissionDenied {
                    path: path.display().to_string(),
                })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).await.map_err(|_| {
            FileSystemMcpError::PermissionDenied {
                path: path.display().to_string(),
            }
        })?;

        Ok(contents)
    }

    /// Read the first N lines using streaming with early termination
    async fn read_file_head(&self, path: &Path, lines: usize) -> FileSystemMcpResult<String> {
        if lines == 0 {
            return Ok(String::new());
        }

        let file = File::open(path)
            .await
            .map_err(|_| FileSystemMcpError::PermissionDenied {
                path: path.display().to_string(),
            })?;

        let reader = BufReader::new(file);
        let mut lines_stream = reader.lines();
        let mut result_lines = Vec::with_capacity(lines);

        // Read only the requested number of lines
        for _ in 0..lines {
            match lines_stream.next_line().await {
                Ok(Some(line)) => result_lines.push(line),
                Ok(None) => break, // End of file reached
                Err(_) => {
                    return Err(FileSystemMcpError::PermissionDenied {
                        path: path.display().to_string(),
                    });
                }
            }
        }

        Ok(result_lines.join("\n"))
    }

    /// Read the last N lines using memory-efficient circular buffer
    async fn read_file_tail(&self, path: &Path, lines: usize) -> FileSystemMcpResult<String> {
        if lines == 0 {
            return Ok(String::new());
        }

        let file = File::open(path)
            .await
            .map_err(|_| FileSystemMcpError::PermissionDenied {
                path: path.display().to_string(),
            })?;

        let reader = BufReader::new(file);
        let mut lines_stream = reader.lines();
        let mut circular_buffer: VecDeque<String> = VecDeque::with_capacity(lines);

        // Read all lines and maintain a circular buffer of the last N lines
        while let Some(line) =
            lines_stream
                .next_line()
                .await
                .map_err(|_| FileSystemMcpError::PermissionDenied {
                    path: path.display().to_string(),
                })?
        {
            if circular_buffer.len() == lines {
                circular_buffer.pop_front();
            }
            circular_buffer.push_back(line);
        }

        // Join the lines in the circular buffer
        Ok(circular_buffer
            .into_iter()
            .collect::<Vec<String>>()
            .join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    async fn create_test_file(content: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(content.as_bytes())
            .expect("Failed to write test content");
        temp_file
    }

    #[tokio::test]
    async fn test_read_entire_file() {
        let service = FileReaderService::new();
        let temp_file = create_test_file("line1\nline2\nline3").await;

        let result = service.read_entire_file(temp_file.path()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "line1\nline2\nline3");
    }

    #[tokio::test]
    async fn test_read_file_head() {
        let service = FileReaderService::new();
        let temp_file = create_test_file("line1\nline2\nline3\nline4\nline5").await;

        let result = service.read_file_head(temp_file.path(), 3).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "line1\nline2\nline3");
    }

    #[tokio::test]
    async fn test_read_file_head_zero_lines() {
        let service = FileReaderService::new();
        let temp_file = create_test_file("line1\nline2\nline3").await;

        let result = service.read_file_head(temp_file.path(), 0).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[tokio::test]
    async fn test_read_file_tail() {
        let service = FileReaderService::new();
        let temp_file = create_test_file("line1\nline2\nline3\nline4\nline5").await;

        let result = service.read_file_tail(temp_file.path(), 3).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "line3\nline4\nline5");
    }

    #[tokio::test]
    async fn test_read_file_tail_zero_lines() {
        let service = FileReaderService::new();
        let temp_file = create_test_file("line1\nline2\nline3").await;

        let result = service.read_file_tail(temp_file.path(), 0).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let service = FileReaderService::new();
        let nonexistent_path = Path::new("/nonexistent/file.txt");

        let result = service.read_entire_file(nonexistent_path).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::PermissionDenied { .. }
        ));
    }
}
