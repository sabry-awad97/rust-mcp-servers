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
//! - [`FileSystemMcpError::InvalidDirectory`] - Path exists but is not a directory
//! - [`FileSystemMcpError::ValidationError`] - Configuration or validation failure

use std::path::PathBuf;
use tokio::fs;

use crate::errors::{FileSystemMcpError, FileSystemMcpResult};

/// Validate that a path is within allowed directories
///
/// This function performs comprehensive security validation by canonicalizing the input path
/// and ensuring it falls within one of the allowed directories. It prevents path traversal
/// attacks and ensures all operations stay within security boundaries.
///
/// # Arguments
///
/// * `path` - The filesystem path to validate (can be relative or absolute)
/// * `allowed_directories` - Slice of canonical directory paths that are permitted
///
/// # Returns
///
/// * `Ok(PathBuf)` - The canonical form of the validated path
/// * `Err(FileSystemMcpError)` - If validation fails
///
/// # Errors
///
/// * [`FileSystemMcpError::DirectoryNotFound`] - If the path does not exist
/// * [`FileSystemMcpError::PermissionDenied`] - If path exists but cannot be accessed,
///   or if the path is outside allowed directories
///
/// # Security
///
/// This function implements multiple security layers:
/// - Path canonicalization to resolve symbolic links and relative components
/// - Boundary checking against allowed directories
/// - Permission validation
///
/// # Examples
///
/// ```rust
/// use std::path::PathBuf;
/// use crate::utils::fs::validate_path;
///
/// let allowed_dirs = vec![PathBuf::from("/safe/directory")];
/// let file_path = PathBuf::from("/safe/directory/file.txt");
///
/// match validate_path(&file_path, &allowed_dirs).await {
///     Ok(canonical_path) => println!("Valid path: {}", canonical_path.display()),
///     Err(e) => eprintln!("Validation failed: {}", e),
/// }
/// ```
pub async fn validate_path(
    path: &std::path::Path,
    allowed_directories: &[PathBuf],
) -> FileSystemMcpResult<PathBuf> {
    let path_owned = path.to_path_buf();

    let canonical_path = tokio::task::spawn_blocking(move || {
        path_owned.canonicalize().map_err(|_| {
            if path_owned.exists() {
                FileSystemMcpError::PermissionDenied {
                    path: path_owned.display().to_string(),
                }
            } else {
                FileSystemMcpError::DirectoryNotFound {
                    path: path_owned.display().to_string(),
                }
            }
        })
    })
    .await
    .map_err(|_| FileSystemMcpError::ValidationError {
        message: "Path canonicalization task failed".to_string(),
    })??;

    if !is_path_allowed(&canonical_path, allowed_directories).await {
        return Err(FileSystemMcpError::PermissionDenied {
            path: canonical_path.display().to_string(),
        });
    }

    Ok(canonical_path)
}

/// Check if a given path is within allowed directories
///
/// This function performs a security check to determine if a path falls within
/// the boundaries of allowed directories. It canonicalizes the path and checks
/// if it starts with any of the allowed directory paths.
///
/// # Arguments
///
/// * `path` - The filesystem path to check (will be canonicalized)
/// * `allowed_directories` - Slice of canonical directory paths that are permitted
///
/// # Returns
///
/// * `true` - If the path is within allowed directories
/// * `false` - If the path is outside allowed directories or cannot be canonicalized
///
/// # Security Notes
///
/// - Returns `false` for any path that cannot be canonicalized (fail-safe behavior)
/// - Uses canonical path comparison to prevent symbolic link attacks
/// - Implements prefix matching to allow subdirectory access
///
/// # Examples
///
/// ```rust
/// use std::path::PathBuf;
/// use crate::utils::fs::is_path_allowed;
///
/// let allowed_dirs = vec![PathBuf::from("/safe/directory")];
///
/// // This will return true (within allowed directory)
/// let safe_file = PathBuf::from("/safe/directory/file.txt");
/// assert!(is_path_allowed(&safe_file, &allowed_dirs).await);
///
/// // This will return false (outside allowed directory)
/// let unsafe_file = PathBuf::from("/etc/passwd");
/// assert!(!is_path_allowed(&unsafe_file, &allowed_dirs).await);
/// ```
pub async fn is_path_allowed(path: &std::path::Path, allowed_directories: &[PathBuf]) -> bool {
    let path_owned = path.to_path_buf();
    let allowed_dirs_owned = allowed_directories.to_vec();

    tokio::task::spawn_blocking(move || {
        let canonical_path = match path_owned.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };

        allowed_dirs_owned
            .iter()
            .any(|allowed_dir| canonical_path.starts_with(allowed_dir))
    })
    .await
    .unwrap_or(false)
}

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
/// * [`FileSystemMcpError::InvalidDirectory`] - If a path exists but is not a directory
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
                FileSystemMcpError::DirectoryNotFound {
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
            return Err(FileSystemMcpError::InvalidDirectory {
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
    use std::fs;
    use tempfile::TempDir;

    /// Test path security validation
    #[tokio::test]
    async fn test_path_security_validation() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Test that paths within the allowed directory are accepted
        let allowed_file = temp_path.join("test.txt");
        tokio::fs::write(&allowed_file, "test").await.unwrap();

        // Canonicalize the temp_path first
        let canonical_temp = temp_path.canonicalize().unwrap();
        assert!(is_path_allowed(&allowed_file, std::slice::from_ref(&canonical_temp)).await);

        // Test that paths outside the allowed directory are rejected
        let temp_dir2 = TempDir::new().unwrap();
        let disallowed_file = temp_dir2.path().join("test.txt");
        tokio::fs::write(&disallowed_file, "test").await.unwrap();
        assert!(!is_path_allowed(&disallowed_file, &[canonical_temp]).await);
    }

    /// Test is_path_allowed with various scenarios
    #[tokio::test]
    async fn test_is_path_allowed_comprehensive() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        // Test subdirectory access
        let subdir = temp_path.join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();
        let subfile = subdir.join("file.txt");
        tokio::fs::write(&subfile, "content").await.unwrap();
        assert!(is_path_allowed(&subfile, std::slice::from_ref(&temp_path)).await);

        // Test nested subdirectory access
        let nested_dir = subdir.join("nested");
        tokio::fs::create_dir(&nested_dir).await.unwrap();
        let nested_file = nested_dir.join("nested.txt");
        tokio::fs::write(&nested_file, "nested").await.unwrap();
        assert!(is_path_allowed(&nested_file, std::slice::from_ref(&temp_path)).await);

        // Test multiple allowed directories
        let temp_dir2 = TempDir::new().unwrap();
        let temp_path2 = temp_dir2.path().canonicalize().unwrap();
        let file2 = temp_path2.join("file2.txt");
        tokio::fs::write(&file2, "content2").await.unwrap();
        assert!(is_path_allowed(&file2, &[temp_path.clone(), temp_path2]).await);

        // Test non-existent path
        let non_existent = temp_path.join("does_not_exist.txt");
        assert!(!is_path_allowed(&non_existent, &[temp_path]).await);
    }

    /// Test validate_path function
    #[tokio::test]
    async fn test_validate_path() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();
        let allowed_dirs = vec![temp_path.clone()];

        // Test valid file path
        let valid_file = temp_path.join("valid.txt");
        tokio::fs::write(&valid_file, "content").await.unwrap();
        let result = validate_path(&valid_file, &allowed_dirs).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), valid_file.canonicalize().unwrap());

        // Test invalid path (outside allowed directories)
        let temp_dir2 = TempDir::new().unwrap();
        let invalid_file = temp_dir2.path().join("invalid.txt");
        tokio::fs::write(&invalid_file, "content").await.unwrap();
        let result = validate_path(&invalid_file, &allowed_dirs).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::PermissionDenied { .. }
        ));

        // Test non-existent path
        let non_existent = temp_path.join("does_not_exist.txt");
        let result = validate_path(&non_existent, &allowed_dirs).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::DirectoryNotFound { .. }
        ));
    }

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
            FileSystemMcpError::DirectoryNotFound { .. }
        ));

        // Test with file instead of directory
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_a_dir.txt");
        tokio::fs::write(&file_path, "content").await.unwrap();
        let result = resolve_directories(vec![file_path]).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::InvalidDirectory { .. }
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
        // Test is_path_allowed with empty allowed directories
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "content").await.unwrap();
        assert!(!is_path_allowed(&file_path, &[]).await);

        // Test validate_path with empty allowed directories
        let result = validate_path(&file_path, &[]).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::PermissionDenied { .. }
        ));

        // Test resolve_directories with mixed valid/invalid paths
        let temp_dir = TempDir::new().unwrap();
        let valid_path = temp_dir.path().to_path_buf();
        let invalid_path = PathBuf::from("/invalid/path");
        let result = resolve_directories(vec![valid_path, invalid_path]).await;
        assert!(result.is_err()); // Should fail on first invalid path
    }

    /// Test path traversal security
    #[tokio::test]
    async fn test_path_traversal_security() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_path = temp_dir.path().canonicalize().unwrap();

        // Create a subdirectory
        let subdir = allowed_path.join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();

        // Test that we can access files in subdirectory
        let sub_file = subdir.join("file.txt");
        tokio::fs::write(&sub_file, "content").await.unwrap();
        assert!(is_path_allowed(&sub_file, std::slice::from_ref(&allowed_path)).await);

        // Test that parent directory access is blocked
        if let Some(parent) = allowed_path.parent() {
            let parent_file = parent.join("parent_file.txt");
            if parent_file.exists() || tokio::fs::write(&parent_file, "content").await.is_ok() {
                assert!(!is_path_allowed(&parent_file, &[allowed_path]).await);
            }
        }
    }
}
