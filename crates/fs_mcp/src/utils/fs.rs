//! Filesystem Utilities Module
//!
//! This module provides secure filesystem operations and path validation utilities
//! for the MCP filesystem server. All functions implement strict security measures
//! to prevent path traversal attacks and ensure operations stay within allowed directories.
//!
//! # Security Features
//!
//! - **Path Canonicalization**: All paths are resolved to their canonical form to prevent
//!   symbolic link attacks and relative path traversal
//! - **Directory Allowlist**: Operations are restricted to explicitly allowed directories
//! - **Permission Validation**: Ensures read/write access before performing operations
//! - **Path Traversal Protection**: Prevents `../` and other traversal attempts
//!
//! # Core Functions
//!
//! - [`validate_path`] - Validates and canonicalizes a path within allowed directories
//! - [`is_path_allowed`] - Checks if a path is within allowed directory boundaries
//! - [`resolve_directories`] - Resolves and validates directory paths for configuration
//! - [`validate_directories`] - Validates directory permissions and accessibility
//!
//! # Usage Example
//!
//! ```rust
//! use std::path::PathBuf;
//! use crate::utils::fs::{resolve_directories, validate_path};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set up allowed directories
//!     let allowed = resolve_directories(vec![PathBuf::from("/safe/directory")]).await?;
//!
//!     // Validate a file path
//!     let file_path = PathBuf::from("/safe/directory/file.txt");
//!     let canonical_path = validate_path(&file_path, &allowed).await?;
//!     Ok(())
//! }
//! ```
//!
//! # Error Handling
//!
//! All functions return [`FileSystemMcpResult`] which provides detailed error information:
//! - [`FileSystemMcpError::DirectoryNotFound`] - Path does not exist
//! - [`FileSystemMcpError::PermissionDenied`] - Access denied or path outside allowed directories
//! - [`FileSystemMcpError::ValidationError`] - Configuration or validation failure

use std::path::PathBuf;
use tokio::fs;

use crate::errors::{FileSystemMcpError, FileSystemMcpResult};

/// Resolve and canonicalize directory paths
///
/// This function takes a list of directory paths and resolves them to their canonical
/// forms, applying security validation and normalization. It serves as the primary
/// configuration function for setting up allowed directories.
///
/// # Arguments
///
/// * `directories` - Vector of directory paths to resolve (can be relative or absolute)
///
/// # Returns
///
/// * `Ok(Vec<PathBuf>)` - Vector of canonical directory paths
/// * `Err(FileSystemMcpError)` - If any directory cannot be resolved or validated
///
/// # Behavior
///
/// - If `directories` is empty, uses the current working directory as fallback
/// - All paths are canonicalized to resolve symbolic links and relative components
/// - Validates that each path exists and is actually a directory
/// - Fails fast on the first invalid directory
///
/// # Errors
///
/// * [`FileSystemMcpError::ValidationError`] - If current directory cannot be determined
/// * [`FileSystemMcpError::DirectoryNotFound`] - If a directory does not exist
/// * [`FileSystemMcpError::PermissionDenied`] - If a directory exists but cannot be accessed
///
/// # Examples
///
/// ```rust
/// use std::path::PathBuf;
/// use crate::utils::fs::resolve_directories;
///
/// // Resolve multiple directories
/// let dirs = vec![
///     PathBuf::from("/home/user/documents"),
///     PathBuf::from("./local/files"),
/// ];
///
/// match resolve_directories(dirs) {
///     Ok(canonical_dirs) => {
///         for dir in canonical_dirs {
///             println!("Allowed directory: {}", dir.display());
///         }
///     }
///     Err(e) => eprintln!("Failed to resolve directories: {}", e),
/// }
///
/// // Use current directory as fallback
/// let current_only = resolve_directories(vec![])?;
/// ```
pub async fn resolve_directories(directories: Vec<PathBuf>) -> FileSystemMcpResult<Vec<PathBuf>> {
    let directories = if directories.is_empty() {
        vec![
            std::env::current_dir().map_err(|_| FileSystemMcpError::ValidationError {
                message: "Failed to get current directory".to_string(),
                path: "".to_string(),
                operation: "resolve_directories".to_string(),
                data: serde_json::json!({"error": "Failed to get current directory"}),
            })?,
        ]
    } else {
        directories
    };

    let mut resolved = Vec::new();

    for dir in directories {
        let canonical = dir.canonicalize().map_err(|_| {
            if dir.exists() {
                FileSystemMcpError::PermissionDenied {
                    path: dir.display().to_string(),
                }
            } else {
                FileSystemMcpError::PathNotFound {
                    path: dir.display().to_string(),
                }
            }
        })?;

        let metadata =
            fs::metadata(&canonical)
                .await
                .map_err(|_| FileSystemMcpError::PermissionDenied {
                    path: canonical.display().to_string(),
                })?;

        if !metadata.is_dir() {
            return Err(FileSystemMcpError::PermissionDenied {
                path: canonical.display().to_string(),
            });
        }

        resolved.push(canonical);
    }

    Ok(resolved)
}

/// Validate directory configuration for consistency and security
///
/// This function performs permission and accessibility validation on a set of
/// canonical directory paths. It ensures that all directories can be read,
/// which is essential for filesystem operations.
///
/// # Arguments
///
/// * `directories` - Slice of canonical directory paths to validate
///
/// # Returns
///
/// * `Ok(())` - If all directories are accessible
/// * `Err(FileSystemMcpError::PermissionDenied)` - If any directory cannot be read
///
/// # Validation Checks
///
/// - Attempts to read each directory to verify permissions
/// - Logs successful validation at debug level
/// - Fails fast on the first inaccessible directory
///
/// # Security Notes
///
/// This function should be called after [`resolve_directories`] to ensure that
/// not only do the directories exist, but they are also accessible for operations.
///
/// # Examples
///
/// ```rust
/// use crate::utils::fs::{resolve_directories, validate_directories};
///
/// // First resolve directories
/// let dirs = resolve_directories(vec![PathBuf::from("/safe/dir")]).await?;
///
/// // Then validate permissions
/// validate_directories(&dirs).await?
///
/// println!("All directories are accessible!");
/// ```
///
/// # Errors
///
/// * [`FileSystemMcpError::PermissionDenied`] - If any directory cannot be read,
///   with detailed error message including the specific directory and system error
pub async fn validate_directories(directories: &[PathBuf]) -> FileSystemMcpResult<()> {
    for dir in directories {
        if let Err(e) = fs::read_dir(dir).await {
            return Err(FileSystemMcpError::PermissionDenied {
                path: format!("{}: {}", dir.display(), e),
            });
        }
    }

    tracing::debug!("Directory validation completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Test resolve_directories function
    #[tokio::test]
    async fn test_resolve_directories() {
        // Test with existing directories
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        let paths = vec![
            temp_dir1.path().to_path_buf(),
            temp_dir2.path().to_path_buf(),
        ];

        let result = resolve_directories(paths.clone()).await;
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0], temp_dir1.path().canonicalize().unwrap());
        assert_eq!(resolved[1], temp_dir2.path().canonicalize().unwrap());

        // Test with empty input (should use current directory)
        let result = resolve_directories(vec![]).await;
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(
            resolved[0],
            std::env::current_dir().unwrap().canonicalize().unwrap()
        );

        // Test with non-existent directory
        let non_existent = PathBuf::from("/this/path/does/not/exist");
        let result = resolve_directories(vec![non_existent]).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::PathNotFound { .. }
        ));

        // Test with file instead of directory
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_a_dir.txt");
        tokio::fs::write(&file_path, "content").await.unwrap();
        let result = resolve_directories(vec![file_path]).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::PermissionDenied { .. }
        ));
    }

    /// Test validate_directories function
    #[tokio::test]
    async fn test_validate_directories() {
        // Test with valid readable directories
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        let canonical1 = temp_dir1.path().canonicalize().unwrap();
        let canonical2 = temp_dir2.path().canonicalize().unwrap();
        let dirs = vec![canonical1, canonical2];

        let result = validate_directories(&dirs).await;
        assert!(result.is_ok());

        // Test with non-existent directory
        let non_existent = PathBuf::from("/this/path/does/not/exist");
        let result = validate_directories(&[non_existent]).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::PermissionDenied { .. }
        ));
    }

    /// Test edge cases and error conditions
    #[tokio::test]
    async fn test_edge_cases() {
        // Test resolve_directories with mixed valid/invalid paths
        let temp_dir = TempDir::new().unwrap();
        let valid_path = temp_dir.path().to_path_buf();
        let invalid_path = PathBuf::from("/invalid/path");
        let result = resolve_directories(vec![valid_path, invalid_path]).await;
        assert!(result.is_err()); // Should fail on first invalid path
    }
}
