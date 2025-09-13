use async_trait::async_trait;
use std::path::Path;
use tokio::fs;

use crate::{
    domain::FileWriter,
    errors::{FileSystemMcpError, FileSystemMcpResult},
    models::responses::WriteFileResponse,
};

/// Application service implementing file writing operations
///
/// This service provides concrete implementations for all file writing operations
/// following SOLID principles and Domain-Driven Design patterns.
pub struct FileWriterService;

impl FileWriterService {
    /// Create a new FileWriterService instance
    pub fn new() -> Self {
        Self
    }

    /// Helper method to get file metadata
    async fn get_file_size(&self, path: &Path) -> Result<u64, std::io::Error> {
        let metadata = fs::metadata(path).await?;
        Ok(metadata.len())
    }

    /// Helper method to check if path exists
    async fn path_exists(&self, path: &Path) -> bool {
        fs::metadata(path).await.is_ok()
    }

    /// Helper method to ensure parent directory exists
    async fn ensure_parent_dir(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent()
            && !self.path_exists(parent).await
        {
            fs::create_dir_all(parent).await?;
        }
        Ok(())
    }
}

impl Default for FileWriterService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileWriter for FileWriterService {
    async fn write_file(
        &self,
        path: &Path,
        content: &str,
    ) -> FileSystemMcpResult<WriteFileResponse> {
        use tokio::io::AsyncWriteExt;

        let file_existed = self.path_exists(path).await;

        // Ensure parent directory exists
        self.ensure_parent_dir(path)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to create parent directory: {}", e),
                path: path.display().to_string(),
            })?;

        // Security: Try exclusive creation first to prevent symlink attacks
        let exclusive_result = fs::OpenOptions::new()
            .write(true)
            .create_new(true) // Fails if file exists (equivalent to 'wx' flag)
            .open(path)
            .await;

        match exclusive_result {
            Ok(mut file) => {
                // File didn't exist, write directly
                file.write_all(content.as_bytes()).await.map_err(|e| {
                    FileSystemMcpError::IoError {
                        message: format!("Failed to write file: {}", e),
                        path: path.display().to_string(),
                    }
                })?;

                file.flush()
                    .await
                    .map_err(|e| FileSystemMcpError::IoError {
                        message: format!("Failed to flush file: {}", e),
                        path: path.display().to_string(),
                    })?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Security: Use atomic rename to prevent race conditions and symlink attacks
                let random_suffix = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let temp_path = if let Some(extension) = path.extension() {
                    path.with_extension(format!(
                        "{}.{:016x}.tmp",
                        extension.to_string_lossy(),
                        random_suffix
                    ))
                } else {
                    path.with_extension(format!("{:016x}.tmp", random_suffix))
                };

                // Write to temporary file first
                let mut temp_file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temp_path)
                    .await
                    .map_err(|e| FileSystemMcpError::IoError {
                        message: format!("Failed to create temporary file: {}", e),
                        path: temp_path.display().to_string(),
                    })?;

                temp_file.write_all(content.as_bytes()).await.map_err(|e| {
                    // Cleanup on failure
                    let _ = std::fs::remove_file(&temp_path);
                    FileSystemMcpError::IoError {
                        message: format!("Failed to write to temporary file: {}", e),
                        path: temp_path.display().to_string(),
                    }
                })?;

                temp_file.flush().await.map_err(|e| {
                    // Cleanup on failure
                    let _ = std::fs::remove_file(&temp_path);
                    FileSystemMcpError::IoError {
                        message: format!("Failed to flush temporary file: {}", e),
                        path: temp_path.display().to_string(),
                    }
                })?;

                // Atomic rename - replaces target file atomically and doesn't follow symlinks
                fs::rename(&temp_path, path).await.map_err(|e| {
                    // Cleanup on failure
                    let _ = std::fs::remove_file(&temp_path);
                    FileSystemMcpError::IoError {
                        message: format!("Failed to rename temporary file: {}", e),
                        path: format!("{} -> {}", temp_path.display(), path.display()),
                    }
                })?;
            }
            Err(e) => {
                return Err(FileSystemMcpError::IoError {
                    message: format!("Failed to open file for writing: {}", e),
                    path: path.display().to_string(),
                });
            }
        }

        let size = content.len() as u64;
        Ok(WriteFileResponse::file_written(path, size, !file_existed))
    }

    async fn append_to_file(
        &self,
        path: &Path,
        content: &str,
    ) -> FileSystemMcpResult<WriteFileResponse> {
        use tokio::io::AsyncWriteExt;

        let file_existed = self.path_exists(path).await;

        // Ensure parent directory exists
        self.ensure_parent_dir(path)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to create parent directory: {}", e),
                path: path.display().to_string(),
            })?;

        // Open file in append mode
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to open file for appending: {}", e),
                path: path.display().to_string(),
            })?;

        // Write content
        file.write_all(content.as_bytes())
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to append to file: {}", e),
                path: path.display().to_string(),
            })?;

        file.flush()
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to flush file: {}", e),
                path: path.display().to_string(),
            })?;

        let total_size =
            self.get_file_size(path)
                .await
                .map_err(|e| FileSystemMcpError::IoError {
                    message: format!("Failed to get file size: {}", e),
                    path: path.display().to_string(),
                })?;

        Ok(WriteFileResponse::file_written(
            path,
            total_size,
            !file_existed,
        ))
    }

    async fn create_directory(&self, path: &Path) -> FileSystemMcpResult<WriteFileResponse> {
        fs::create_dir_all(path)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to create directory: {}", e),
                path: path.display().to_string(),
            })?;

        Ok(WriteFileResponse::directory_created(path))
    }

    async fn delete_file(&self, path: &Path) -> FileSystemMcpResult<WriteFileResponse> {
        if !self.path_exists(path).await {
            return Err(FileSystemMcpError::PathNotFound {
                path: path.display().to_string(),
            });
        }

        fs::remove_file(path)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to delete file: {}", e),
                path: path.display().to_string(),
            })?;

        Ok(WriteFileResponse::deleted(path, false))
    }

    async fn delete_directory(&self, path: &Path) -> FileSystemMcpResult<WriteFileResponse> {
        if !self.path_exists(path).await {
            return Err(FileSystemMcpError::PathNotFound {
                path: path.display().to_string(),
            });
        }

        fs::remove_dir_all(path)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to delete directory: {}", e),
                path: path.display().to_string(),
            })?;

        Ok(WriteFileResponse::deleted(path, true))
    }

    async fn move_file(&self, from: &Path, to: &Path) -> FileSystemMcpResult<WriteFileResponse> {
        if !self.path_exists(from).await {
            return Err(FileSystemMcpError::PathNotFound {
                path: from.display().to_string(),
            });
        }

        // Ensure destination parent directory exists
        self.ensure_parent_dir(to)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to create destination directory: {}", e),
                path: to.display().to_string(),
            })?;

        fs::rename(from, to)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to move file/directory: {}", e),
                path: format!("{} -> {}", from.display(), to.display()),
            })?;

        Ok(WriteFileResponse::moved(from, to))
    }

    async fn copy_file(&self, from: &Path, to: &Path) -> FileSystemMcpResult<WriteFileResponse> {
        if !self.path_exists(from).await {
            return Err(FileSystemMcpError::PathNotFound {
                path: from.display().to_string(),
            });
        }

        // Ensure destination parent directory exists
        self.ensure_parent_dir(to)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to create destination directory: {}", e),
                path: to.display().to_string(),
            })?;

        fs::copy(from, to)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to copy file: {}", e),
                path: format!("{} -> {}", from.display(), to.display()),
            })?;

        let size = self
            .get_file_size(to)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to get copied file size: {}", e),
                path: to.display().to_string(),
            })?;

        Ok(WriteFileResponse::copied(from, to, size))
    }

    async fn write_binary_file(
        &self,
        path: &Path,
        data: &[u8],
    ) -> FileSystemMcpResult<WriteFileResponse> {
        let file_existed = self.path_exists(path).await;

        // Ensure parent directory exists
        self.ensure_parent_dir(path)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to create parent directory: {}", e),
                path: path.display().to_string(),
            })?;

        // Write the binary data
        fs::write(path, data)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to write binary file: {}", e),
                path: path.display().to_string(),
            })?;

        let size = data.len() as u64;
        Ok(WriteFileResponse::file_written(path, size, !file_existed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, sync::Arc};
    use tempfile::{NamedTempFile, TempDir};

    async fn create_temp_file_with_content(content: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(content.as_bytes())
            .expect("Failed to write test content");
        temp_file
    }

    #[tokio::test]
    async fn test_write_file_new() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_file.txt");
        let content = "Hello, World!";

        let result = service.write_file(&file_path, content).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.created);
        assert_eq!(response.size, Some(content.len() as u64));

        // Verify file was actually written
        let written_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(written_content, content);
    }

    #[tokio::test]
    async fn test_write_file_overwrite() {
        let service = FileWriterService::new();
        let temp_file = create_temp_file_with_content("original content").await;
        let new_content = "new content";

        let result = service.write_file(temp_file.path(), new_content).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.created); // File already existed
        assert_eq!(response.size, Some(new_content.len() as u64));

        // Verify file was overwritten
        let written_content = fs::read_to_string(temp_file.path()).await.unwrap();
        assert_eq!(written_content, new_content);
    }

    #[tokio::test]
    async fn test_append_to_file() {
        let service = FileWriterService::new();
        let temp_file = create_temp_file_with_content("original").await;
        let append_content = " appended";

        let result = service
            .append_to_file(temp_file.path(), append_content)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.created); // File already existed

        // Verify content was appended
        let final_content = fs::read_to_string(temp_file.path()).await.unwrap();
        assert_eq!(final_content, "original appended");
    }

    #[tokio::test]
    async fn test_create_directory() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let new_dir = temp_dir.path().join("new_directory");

        let result = service.create_directory(&new_dir).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.created);

        // Verify directory was created
        assert!(new_dir.exists());
        assert!(new_dir.is_dir());
    }

    #[tokio::test]
    async fn test_delete_file() {
        let service = FileWriterService::new();
        let temp_file = create_temp_file_with_content("test content").await;
        let file_path = temp_file.path().to_path_buf();

        // File should exist initially
        assert!(file_path.exists());

        let result = service.delete_file(&file_path).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.created);

        // Verify file was deleted
        assert!(!file_path.exists());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_file() {
        let service = FileWriterService::new();
        let nonexistent_path = std::path::Path::new("/nonexistent/file.txt");

        let result = service.delete_file(nonexistent_path).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::PathNotFound { .. }
        ));
    }

    #[tokio::test]
    async fn test_move_file() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_file = create_temp_file_with_content("test content").await;
        let source_path = temp_file.path().to_path_buf();
        let dest_path = temp_dir.path().join("moved_file.txt");

        let result = service.move_file(&source_path, &dest_path).await;
        assert!(result.is_ok());

        // Verify file was moved
        assert!(!source_path.exists());
        assert!(dest_path.exists());

        let content = fs::read_to_string(&dest_path).await.unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_copy_file() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_file = create_temp_file_with_content("test content").await;
        let source_path = temp_file.path();
        let dest_path = temp_dir.path().join("copied_file.txt");

        let result = service.copy_file(source_path, &dest_path).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.created);

        // Verify both files exist with same content
        assert!(source_path.exists());
        assert!(dest_path.exists());

        let source_content = fs::read_to_string(source_path).await.unwrap();
        let dest_content = fs::read_to_string(&dest_path).await.unwrap();
        assert_eq!(source_content, dest_content);
        assert_eq!(dest_content, "test content");
    }

    #[tokio::test]
    async fn test_write_binary_file() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("binary_file.bin");
        let binary_data = vec![0x00, 0x01, 0x02, 0x03, 0xFF];

        let result = service.write_binary_file(&file_path, &binary_data).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.created);
        assert_eq!(response.size, Some(binary_data.len() as u64));

        // Verify binary data was written correctly
        let written_data = fs::read(&file_path).await.unwrap();
        assert_eq!(written_data, binary_data);
    }

    #[tokio::test]
    async fn test_write_file_with_nested_directories() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let nested_path = temp_dir
            .path()
            .join("level1")
            .join("level2")
            .join("file.txt");
        let content = "nested file content";

        let result = service.write_file(&nested_path, content).await;
        assert!(result.is_ok());

        // Verify parent directories were created
        assert!(nested_path.parent().unwrap().exists());

        // Verify file content
        let written_content = fs::read_to_string(&nested_path).await.unwrap();
        assert_eq!(written_content, content);
    }

    #[tokio::test]
    async fn test_write_file_exclusive_creation() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("exclusive_test.txt");
        let content = "exclusive creation test";

        // First write should use exclusive creation path
        let result = service.write_file(&file_path, content).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.created);
        assert_eq!(response.size, Some(content.len() as u64));

        // Verify file was created
        assert!(file_path.exists());
        let written_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(written_content, content);
    }

    #[tokio::test]
    async fn test_write_file_atomic_rename() {
        let service = FileWriterService::new();
        let temp_file = create_temp_file_with_content("original content").await;
        let file_path = temp_file.path();
        let new_content = "atomic rename test content";

        // This should trigger the atomic rename path since file exists
        let result = service.write_file(file_path, new_content).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.created); // File already existed
        assert_eq!(response.size, Some(new_content.len() as u64));

        // Verify content was replaced atomically
        let written_content = fs::read_to_string(file_path).await.unwrap();
        assert_eq!(written_content, new_content);
    }

    #[tokio::test]
    async fn test_write_file_with_extension_temp_naming() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_file.txt");

        // Create the file first to trigger atomic rename path
        fs::write(&file_path, "original content").await.unwrap();
        assert!(file_path.exists());

        let new_content = "test content for extension handling";

        let count_temp_files = async |dir| {
            let mut count = 0;
            if let Ok(mut entries) = fs::read_dir(dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.ends_with(".tmp") {
                        count += 1;
                    }
                }
            }
            count
        };

        // Count temp files before operation
        let temp_files_before = count_temp_files(temp_dir.path()).await;

        // Perform the write
        let result = service.write_file(&file_path, new_content).await;
        assert!(result.is_ok(), "Write failed: {:?}", result.err());

        // Verify no new temporary files are left behind
        let temp_files_after = count_temp_files(temp_dir.path()).await;
        assert_eq!(
            temp_files_before, temp_files_after,
            "Temporary files left behind after write operation"
        );

        // Verify final file content
        let written_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(written_content, new_content);
    }

    #[tokio::test]
    async fn test_write_file_concurrent_operations() {
        use tokio::task::JoinSet;

        let service = Arc::new(FileWriterService::new());
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Test concurrent writes to different files
        let mut join_set = JoinSet::new();
        let mut expected_contents = Vec::new();

        for i in 0..5 {
            let service_clone = service.clone();
            let file_path = temp_dir.path().join(format!("concurrent_test_{}.txt", i));
            let content = format!("concurrent content {}", i);
            expected_contents.push((file_path.clone(), content.clone()));

            join_set.spawn(async move { service_clone.write_file(&file_path, &content).await });
        }

        // Wait for all writes to complete
        let mut results = Vec::new();
        while let Some(result) = join_set.join_next().await {
            results.push(result.unwrap());
        }

        // Verify all writes succeeded
        for result in results {
            assert!(result.is_ok());
        }

        // Verify all files have correct content
        for (file_path, expected_content) in expected_contents {
            assert!(file_path.exists());
            let actual_content = fs::read_to_string(&file_path).await.unwrap();
            assert_eq!(actual_content, expected_content);
        }
    }

    #[tokio::test]
    async fn test_write_file_large_content() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("large_file.txt");

        // Create a large content string (1MB)
        let large_content = "A".repeat(1024 * 1024);

        let result = service.write_file(&file_path, &large_content).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.created);
        assert_eq!(response.size, Some(large_content.len() as u64));

        // Verify content integrity
        let written_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(written_content.len(), large_content.len());
        assert_eq!(written_content, large_content);
    }

    #[tokio::test]
    async fn test_write_file_empty_content() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("empty_file.txt");
        let empty_content = "";

        let result = service.write_file(&file_path, empty_content).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.created);
        assert_eq!(response.size, Some(0));

        // Verify empty file was created
        assert!(file_path.exists());
        let written_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(written_content, "");
    }

    #[tokio::test]
    async fn test_write_file_unicode_content() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("unicode_file.txt");
        let unicode_content = "Hello 世界! 🦀 Rust is awesome! ñáéíóú";

        let result = service.write_file(&file_path, unicode_content).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.created);
        assert_eq!(response.size, Some(unicode_content.len() as u64));

        // Verify Unicode content integrity
        let written_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(written_content, unicode_content);
    }

    #[tokio::test]
    async fn test_write_file_permission_error_simulation() {
        let service = FileWriterService::new();

        // Try to write to a path that should cause permission issues
        // Note: This test might behave differently on different platforms
        let invalid_path = if cfg!(windows) {
            std::path::Path::new("C:\\Windows\\System32\\test_file.txt")
        } else {
            std::path::Path::new("/root/test_file.txt")
        };

        let result = service.write_file(invalid_path, "test content").await;

        // Should fail with an IoError
        assert!(result.is_err());
        if let Err(FileSystemMcpError::IoError { message, .. }) = result {
            assert!(message.contains("Failed to"));
        } else {
            panic!("Expected IoError");
        }
    }

    #[tokio::test]
    async fn test_write_file_no_extension() {
        let service = FileWriterService::new();
        let temp_file = create_temp_file_with_content("original").await;

        // Create a file path without extension
        let parent = temp_file.path().parent().unwrap();
        let no_ext_path = parent.join("file_no_extension");

        // Create the file first to trigger atomic rename path
        fs::write(&no_ext_path, "initial").await.unwrap();

        let new_content = "content for file without extension";
        let result = service.write_file(&no_ext_path, new_content).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.created); // File already existed

        // Verify content
        let written_content = fs::read_to_string(&no_ext_path).await.unwrap();
        assert_eq!(written_content, new_content);
    }
}
