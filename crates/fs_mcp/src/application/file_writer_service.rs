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

    /// Normalize line endings to Unix format (\n)
    fn normalize_line_endings(text: &str) -> String {
        text.replace("\r\n", "\n")
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

    async fn apply_file_edits(
        &self,
        path: &Path,
        edits: &[crate::models::requests::EditOperation],
        dry_run: &bool,
    ) -> FileSystemMcpResult<WriteFileResponse> {
        // Read and normalize file content
        let original_content =
            fs::read_to_string(path)
                .await
                .map_err(|e| FileSystemMcpError::IoError {
                    message: format!("Failed to read file for editing: {}", e),
                    path: path.display().to_string(),
                })?;

        let mut modified_content = Self::normalize_line_endings(&original_content);

        // Apply edits sequentially
        for edit in edits {
            let normalized_old = Self::normalize_line_endings(edit.old_text());
            let normalized_new = Self::normalize_line_endings(edit.new_text());

            // Try exact match first
            if modified_content.contains(&normalized_old) {
                modified_content = modified_content.replacen(&normalized_old, &normalized_new, 1);
                continue;
            }

            // Try line-by-line matching with whitespace flexibility
            let old_lines: Vec<&str> = normalized_old.split('\n').collect();
            let content_lines: Vec<&str> = modified_content.split('\n').collect();
            let mut match_found = false;

            for i in 0..=(content_lines.len().saturating_sub(old_lines.len())) {
                if i + old_lines.len() > content_lines.len() {
                    break;
                }

                let potential_match = &content_lines[i..i + old_lines.len()];

                // Compare lines with normalized whitespace
                let is_match = old_lines
                    .iter()
                    .zip(potential_match.iter())
                    .all(|(old_line, content_line)| old_line.trim() == content_line.trim());

                if is_match {
                    // Preserve original indentation of first line
                    let original_indent = content_lines[i]
                        .chars()
                        .take_while(|c| c.is_whitespace())
                        .collect::<String>();

                    // Calculate the base indentation of the new text (from first non-empty line)
                    let new_text_lines: Vec<&str> = normalized_new.split('\n').collect();
                    let base_new_indent = new_text_lines
                        .iter()
                        .find(|line| !line.trim().is_empty())
                        .map(|line| {
                            line.chars()
                                .take_while(|c| c.is_whitespace())
                                .collect::<String>()
                        })
                        .unwrap_or_default();

                    let new_lines: Vec<String> = new_text_lines
                        .iter()
                        .enumerate()
                        .map(|(j, line)| {
                            if j == 0 {
                                // First line: use original indentation
                                format!("{}{}", original_indent, line.trim_start())
                            } else if line.trim().is_empty() {
                                // Empty lines remain empty
                                String::new()
                            } else {
                                // Subsequent lines: preserve relative indentation structure
                                let line_indent = line
                                    .chars()
                                    .take_while(|c| c.is_whitespace())
                                    .collect::<String>();

                                // Calculate relative indentation from the base indentation of new text
                                let relative_indent_size =
                                    if line_indent.len() >= base_new_indent.len() {
                                        line_indent.len() - base_new_indent.len()
                                    } else {
                                        0
                                    };

                                format!(
                                    "{}{}{}",
                                    original_indent,
                                    " ".repeat(relative_indent_size),
                                    line.trim_start()
                                )
                            }
                        })
                        .collect();

                    // Replace the matched lines
                    let mut new_content_lines = content_lines[..i].to_vec();
                    new_content_lines.extend(new_lines.iter().map(|s| s.as_str()));
                    new_content_lines.extend(&content_lines[i + old_lines.len()..]);

                    modified_content = new_content_lines.join("\n");
                    match_found = true;
                    break;
                }
            }

            if !match_found {
                return Err(FileSystemMcpError::ValidationError {
                    message: "Could not find exact match for edit".to_string(),
                    path: path.display().to_string(),
                    operation: "apply_edit".to_string(),
                    data: serde_json::json!({
                        "error": "No matching text found",
                        "old_text": edit.old_text()
                    }),
                });
            }
        }

        if *dry_run {
            // Return preview without modifying file
            Ok(WriteFileResponse::new(
                format!("Dry run completed. {} edits would be applied.", edits.len()),
                path.display().to_string(),
                Some(modified_content.len() as u64),
                false,
            ))
        } else {
            // Apply changes using secure write
            self.write_file(path, &modified_content).await
        }
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

    async fn list_directory(&self, path: &Path) -> FileSystemMcpResult<WriteFileResponse> {
        let mut entries = fs::read_dir(path)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to list directory: {}", e),
                path: path.display().to_string(),
            })?;

        let mut results = Vec::new();
        while let Some(entry) =
            entries
                .next_entry()
                .await
                .map_err(|e| FileSystemMcpError::IoError {
                    message: format!("Failed to read directory entry: {}", e),
                    path: path.display().to_string(),
                })?
        {
            let metadata = entry
                .metadata()
                .await
                .map_err(|e| FileSystemMcpError::IoError {
                    message: format!("Failed to get metadata: {}", e),
                    path: path.display().to_string(),
                })?;
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = if metadata.is_dir() {
                "directory".to_string()
            } else if metadata.is_symlink() {
                "symlink".to_string()
            } else {
                // Extract file extension for better type identification
                match std::path::Path::new(&name).extension() {
                    Some(ext) => format!("{} file", ext.to_string_lossy().to_lowercase()),
                    None => "file".to_string(),
                }
            };
            let size = if metadata.is_file() {
                format!(" ({} bytes)", metadata.len())
            } else {
                String::new()
            };
            results.push(format!("{} - {}{}", name, file_type, size));
        }

        results.sort();

        Ok(WriteFileResponse::new(
            format!(
                "Directory listing completed successfully:\n{}",
                results.join("\n")
            ),
            path.display().to_string(),
            None,
            false,
        ))
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
    async fn test_list_directory_empty() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let result = service.list_directory(temp_dir.path()).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(
            response
                .message
                .contains("Directory listing completed successfully")
        );
        // Empty directory should have minimal content
        let lines: Vec<&str> = response.message.lines().collect();
        assert_eq!(lines.len(), 1); // Just the header line
    }

    #[tokio::test]
    async fn test_list_directory_with_files() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create test files with different extensions
        let test_file1 = temp_dir.path().join("test.txt");
        let test_file2 = temp_dir.path().join("config.toml");
        let test_file3 = temp_dir.path().join("script.rs");
        let test_file4 = temp_dir.path().join("no_extension");

        fs::write(&test_file1, "Hello world").await.unwrap();
        fs::write(&test_file2, "[section]\nkey=value")
            .await
            .unwrap();
        fs::write(&test_file3, "fn main() {}").await.unwrap();
        fs::write(&test_file4, "binary data").await.unwrap();

        let result = service.list_directory(temp_dir.path()).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(
            response
                .message
                .contains("Directory listing completed successfully")
        );

        // Check that all files are listed with correct types
        assert!(response.message.contains("test.txt - txt file"));
        assert!(response.message.contains("config.toml - toml file"));
        assert!(response.message.contains("script.rs - rs file"));
        assert!(response.message.contains("no_extension - file"));

        // Check that file sizes are included
        assert!(response.message.contains("bytes"));
    }

    #[tokio::test]
    async fn test_list_directory_with_subdirectories() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create subdirectories
        let sub_dir1 = temp_dir.path().join("subdir1");
        let sub_dir2 = temp_dir.path().join("subdir2");
        fs::create_dir(&sub_dir1).await.unwrap();
        fs::create_dir(&sub_dir2).await.unwrap();

        // Create a file in the main directory
        let test_file = temp_dir.path().join("readme.md");
        fs::write(&test_file, "# Test").await.unwrap();

        let result = service.list_directory(temp_dir.path()).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(
            response
                .message
                .contains("Directory listing completed successfully")
        );

        // Check that directories are listed correctly
        assert!(response.message.contains("subdir1 - directory"));
        assert!(response.message.contains("subdir2 - directory"));
        assert!(response.message.contains("readme.md - md file"));

        // Directories should not have size information
        assert!(!response.message.contains("subdir1 - directory ("));
        assert!(!response.message.contains("subdir2 - directory ("));
    }

    #[tokio::test]
    async fn test_list_directory_sorted_output() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create files in non-alphabetical order
        let files = ["zebra.txt", "alpha.txt", "beta.txt"];
        for file in &files {
            let file_path = temp_dir.path().join(file);
            fs::write(&file_path, "content").await.unwrap();
        }

        let result = service.list_directory(temp_dir.path()).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let content = response.message;

        // Find positions of each file in the output
        let alpha_pos = content.find("alpha.txt").unwrap();
        let beta_pos = content.find("beta.txt").unwrap();
        let zebra_pos = content.find("zebra.txt").unwrap();

        // Verify alphabetical order
        assert!(alpha_pos < beta_pos);
        assert!(beta_pos < zebra_pos);
    }

    #[tokio::test]
    async fn test_list_directory_nonexistent() {
        let service = FileWriterService::new();
        let nonexistent_path = std::path::Path::new("/nonexistent/directory");

        let result = service.list_directory(nonexistent_path).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::IoError { .. }
        ));
    }

    #[tokio::test]
    async fn test_list_directory_mixed_content() {
        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create mixed content: files, directories, different extensions
        fs::create_dir(temp_dir.path().join("docs")).await.unwrap();
        fs::create_dir(temp_dir.path().join("src")).await.unwrap();

        fs::write(temp_dir.path().join("Cargo.toml"), "[package]")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("README.md"), "# Project")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("data.json"), "{}")
            .await
            .unwrap();

        let result = service.list_directory(temp_dir.path()).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let content = response.message;

        // Verify all items are present with correct types
        assert!(content.contains("docs - directory"));
        assert!(content.contains("src - directory"));
        assert!(content.contains("Cargo.toml - toml file"));
        assert!(content.contains("README.md - md file"));
        assert!(content.contains("main.rs - rs file"));
        assert!(content.contains("data.json - json file"));

        // Verify sorting (directories and files mixed but alphabetically sorted)
        let lines: Vec<&str> = content.lines().skip(1).collect(); // Skip header
        let mut sorted_lines = lines.clone();
        sorted_lines.sort();
        assert_eq!(lines, sorted_lines);
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

    #[tokio::test]
    async fn test_apply_file_edits_exact_match() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_edit.txt");

        let original_content = "Hello world\nThis is a test\nEnd of file";
        fs::write(&file_path, original_content).await.unwrap();

        let edits = vec![EditOperation::new(
            "Hello world".to_string(),
            "Hello Rust".to_string(),
        )];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_ok());

        let final_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(final_content, "Hello Rust\nThis is a test\nEnd of file");
    }

    #[tokio::test]
    async fn test_apply_file_edits_whitespace_flexible() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_whitespace.txt");

        let original_content = "    function test() {\n        return true;\n    }";
        fs::write(&file_path, original_content).await.unwrap();

        let edits = vec![EditOperation::new(
            "function test() {\n    return true;\n}".to_string(),
            "function test() {\n    return false;\n}".to_string(),
        )];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_ok());

        let final_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(
            final_content,
            "    function test() {\n        return false;\n    }"
        );
    }

    #[tokio::test]
    async fn test_apply_file_edits_preserve_indentation() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_indent.txt");

        let original_content =
            "class Test {\n    method1() {\n        console.log('test');\n    }\n}";
        fs::write(&file_path, original_content).await.unwrap();

        let edits = vec![EditOperation::new(
            "method1() {\n    console.log('test');\n}".to_string(),
            "method1() {\n    console.log('updated');\n    return true;\n}".to_string(),
        )];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_ok());

        let final_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(
            final_content,
            "class Test {\n    method1() {\n        console.log('updated');\n        return true;\n    }\n}"
        );
    }

    #[tokio::test]
    async fn test_apply_file_edits_multiple_edits() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_multiple.txt");

        let original_content = "let x = 1;\nlet y = 2;\nlet z = 3;";
        fs::write(&file_path, original_content).await.unwrap();

        let edits = vec![
            EditOperation::new("let x = 1;".to_string(), "let x = 10;".to_string()),
            EditOperation::new("let y = 2;".to_string(), "let y = 20;".to_string()),
            EditOperation::new("let z = 3;".to_string(), "let z = 30;".to_string()),
        ];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_ok());

        let final_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(final_content, "let x = 10;\nlet y = 20;\nlet z = 30;");
    }

    #[tokio::test]
    async fn test_apply_file_edits_dry_run() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_dry_run.txt");

        let original_content = "Hello world";
        fs::write(&file_path, original_content).await.unwrap();

        let edits = vec![EditOperation::new("Hello".to_string(), "Hi".to_string())];

        let result = service.apply_file_edits(&file_path, &edits, &true).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.message.contains("Dry run completed"));
        assert!(response.message.contains("1 edits would be applied"));

        // Verify original file unchanged
        let unchanged_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(unchanged_content, original_content);
    }

    #[tokio::test]
    async fn test_apply_file_edits_line_ending_normalization() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_line_endings.txt");

        // Create file with Windows line endings
        let original_content = "Hello\r\nWorld\r\nTest";
        fs::write(&file_path, original_content).await.unwrap();

        let edits = vec![EditOperation::new(
            "Hello\nWorld".to_string(), // Unix line endings in edit
            "Hi\nEveryone".to_string(),
        )];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_ok());

        let final_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(final_content, "Hi\nEveryone\nTest");
    }

    #[tokio::test]
    async fn test_apply_file_edits_deletion() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_deletion.txt");

        let original_content = "Keep this line\nDelete this line\nKeep this too";
        fs::write(&file_path, original_content).await.unwrap();

        let edits = vec![EditOperation::new(
            "Delete this line\n".to_string(), // Empty string for deletion
            "".to_string(),
        )];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_ok());

        let final_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(final_content, "Keep this line\nKeep this too");
    }

    #[tokio::test]
    async fn test_apply_file_edits_insertion() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_insertion.txt");

        let original_content = "Line 1\nLine 3";
        fs::write(&file_path, original_content).await.unwrap();

        let edits = vec![EditOperation::new(
            "Line 1\nLine 3".to_string(),
            "Line 1\nLine 2\nLine 3".to_string(),
        )];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_ok());

        let final_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(final_content, "Line 1\nLine 2\nLine 3");
    }

    #[tokio::test]
    async fn test_apply_file_edits_no_match_error() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_no_match.txt");

        let original_content = "Hello world";
        fs::write(&file_path, original_content).await.unwrap();

        let edits = vec![EditOperation::new(
            "Goodbye world".to_string(), // This doesn't exist
            "Hi world".to_string(),
        )];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_err());

        if let Err(FileSystemMcpError::ValidationError { message, .. }) = result {
            assert!(message.contains("Could not find exact match"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[tokio::test]
    async fn test_apply_file_edits_complex_indentation() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_complex_indent.txt");

        let original_content =
            "    if (condition) {\n        doSomething();\n        doMore();\n    }";
        fs::write(&file_path, original_content).await.unwrap();

        let edits = vec![EditOperation::new(
            "if (condition) {\n    doSomething();\n    doMore();\n}".to_string(),
            "if (condition) {\n    doSomething();\n    doMore();\n    doEvenMore();\n}".to_string(),
        )];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_ok());

        let final_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(
            final_content,
            "    if (condition) {\n        doSomething();\n        doMore();\n        doEvenMore();\n    }"
        );
    }

    #[tokio::test]
    async fn test_apply_file_edits_empty_file() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_empty.txt");

        fs::write(&file_path, "").await.unwrap();

        let edits = vec![EditOperation::new(
            "".to_string(),
            "Hello world".to_string(),
        )];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_ok());

        let final_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(final_content, "Hello world");
    }

    #[tokio::test]
    async fn test_apply_file_edits_unicode_content() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_unicode.txt");

        let original_content = "Hello 世界\nRust is 🦀";
        fs::write(&file_path, original_content).await.unwrap();

        let edits = vec![EditOperation::new(
            "Hello 世界".to_string(),
            "你好 World".to_string(),
        )];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_ok());

        let final_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(final_content, "你好 World\nRust is 🦀");
    }

    #[tokio::test]
    async fn test_apply_file_edits_sequential_dependency() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_sequential.txt");

        let original_content = "Step 1\nStep 2\nStep 3";
        fs::write(&file_path, original_content).await.unwrap();

        // Each edit depends on the result of the previous one
        let edits = vec![
            EditOperation::new("Step 1".to_string(), "Phase 1".to_string()),
            EditOperation::new(
                "Phase 1\nStep 2".to_string(),
                "Phase 1\nPhase 2".to_string(),
            ),
            EditOperation::new(
                "Phase 2\nStep 3".to_string(),
                "Phase 2\nPhase 3".to_string(),
            ),
        ];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_ok());

        let final_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(final_content, "Phase 1\nPhase 2\nPhase 3");
    }

    #[tokio::test]
    async fn test_apply_file_edits_nonexistent_file() {
        use crate::models::requests::EditOperation;

        let service = FileWriterService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("nonexistent.txt");

        let edits = vec![EditOperation::new(
            "test".to_string(),
            "updated".to_string(),
        )];

        let result = service.apply_file_edits(&file_path, &edits, &false).await;
        assert!(result.is_err());

        if let Err(FileSystemMcpError::IoError { message, .. }) = result {
            assert!(message.contains("Failed to read file for editing"));
        } else {
            panic!("Expected IoError");
        }
    }
}
