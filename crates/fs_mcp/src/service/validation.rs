use std::path::{Path, PathBuf};
use tokio::fs;

use crate::{
    errors::{FileSystemMcpError, FileSystemMcpResult},
    utils::path::{expand_home, is_path_within_allowed_directories, normalize_path},
};

pub trait Validate {
    fn validate(&self) -> FileSystemMcpResult<()>;
}

/// Validate that a path is within allowed directories
///
/// This function performs comprehensive security validation following the TypeScript implementation:
/// - Expands home directory (~) references
/// - Converts to absolute path
/// - Checks path boundaries before file operations
/// - Handles symlinks by checking their real path to prevent symlink attacks
/// - For non-existent files, validates parent directory permissions
///
/// # Arguments
///
/// * `requested_path` - The filesystem path to validate (can be relative or absolute)
/// * `allowed_directories` - Slice of canonical directory paths that are permitted
///
/// # Returns
///
/// * `Ok(PathBuf)` - The validated real path
/// * `Err(FileSystemMcpError)` - If validation fails
///
/// # Security Features
///
/// - **Home expansion**: Handles ~ references safely
/// - **Path normalization**: Resolves . and .. components
/// - **Boundary checking**: Ensures path is within allowed directories
/// - **Symlink protection**: Validates symlink targets to prevent attacks
/// - **Parent validation**: For new files, checks parent directory permissions
///
/// # Examples
///
/// ```rust
/// use std::path::PathBuf;
/// use crate::service::validation::validate_path;
///
/// let allowed_dirs = vec![PathBuf::from("/safe/directory")];
/// let file_path = "~/documents/file.txt";
///
/// match validate_path(file_path, &allowed_dirs).await {
///     Ok(real_path) => println!("Valid path: {}", real_path.display()),
///     Err(e) => eprintln!("Validation failed: {}", e),
/// }
/// ```
pub async fn validate_path(
    requested_path: &str,
    allowed_directories: &[PathBuf],
) -> FileSystemMcpResult<PathBuf> {
    // Step 1: Expand home directory references
    let expanded_path = expand_home(requested_path);

    // Step 2: Convert to absolute path
    let absolute_path = if Path::new(&expanded_path).is_absolute() {
        PathBuf::from(&expanded_path)
    } else {
        std::env::current_dir()
            .map_err(|_| FileSystemMcpError::ValidationError {
                message: "Failed to get current directory".to_string(),
                path: expanded_path.clone(),
                operation: "validate_path".to_string(),
                data: serde_json::json!({"error": "Failed to get current directory"}),
            })?
            .join(&expanded_path)
    };

    // Step 3: Normalize the path
    let normalized_requested = normalize_path(&absolute_path);

    // Step 4: Security check - verify path is within allowed directories before file operations
    if !is_path_within_allowed_directories(&normalized_requested, allowed_directories) {
        return Err(FileSystemMcpError::PermissionDenied {
            path: format!(
                "Access denied - path outside allowed directories: {} not in [{}]",
                absolute_path.display(),
                allowed_directories
                    .iter()
                    .map(|d| d.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }

    // Step 5: Handle symlinks by checking their real path to prevent symlink attacks
    match fs::canonicalize(&absolute_path).await {
        Ok(real_path) => {
            let normalized_real = normalize_path(&real_path);
            if !is_path_within_allowed_directories(&normalized_real, allowed_directories) {
                return Err(FileSystemMcpError::PermissionDenied {
                    path: format!(
                        "Access denied - symlink target outside allowed directories: {} not in [{}]",
                        real_path.display(),
                        allowed_directories
                            .iter()
                            .map(|d| d.display().to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
            }
            Ok(real_path)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Step 6: For new files that don't exist yet, verify parent directory
            let parent_dir =
                absolute_path
                    .parent()
                    .ok_or_else(|| FileSystemMcpError::ValidationError {
                        message: "Path has no parent directory".to_string(),
                        path: absolute_path.display().to_string(),
                        operation: "validate_path".to_string(),
                        data: serde_json::json!({"error": "Path has no parent directory"}),
                    })?;

            match fs::canonicalize(parent_dir).await {
                Ok(real_parent_path) => {
                    let normalized_parent = normalize_path(&real_parent_path);
                    if !is_path_within_allowed_directories(&normalized_parent, allowed_directories)
                    {
                        return Err(FileSystemMcpError::PermissionDenied {
                            path: format!(
                                "Access denied - parent directory outside allowed directories: {} not in [{}]",
                                real_parent_path.display(),
                                allowed_directories
                                    .iter()
                                    .map(|d| d.display().to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        });
                    }
                    Ok(absolute_path)
                }
                Err(_) => Err(FileSystemMcpError::PathNotFound {
                    path: format!("Parent directory does not exist: {}", parent_dir.display()),
                }),
            }
        }
        Err(e) => Err(FileSystemMcpError::PermissionDenied {
            path: format!("Cannot access path {}: {}", absolute_path.display(), e),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Test validate_path function
    #[tokio::test]
    async fn test_validate_path() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();
        let allowed_dirs = vec![temp_path.clone()];

        // Test valid file path
        let valid_file = temp_path.join("valid.txt");
        tokio::fs::write(&valid_file, "content").await.unwrap();
        let result = validate_path(&valid_file.display().to_string(), &allowed_dirs).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), valid_file.canonicalize().unwrap());

        // Test invalid path (outside allowed directories)
        let temp_dir2 = TempDir::new().unwrap();
        let invalid_file = temp_dir2.path().join("invalid.txt");
        tokio::fs::write(&invalid_file, "content").await.unwrap();
        let result = validate_path(&invalid_file.display().to_string(), &allowed_dirs).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::PermissionDenied { .. }
        ));

        // Test non-existent path - should return PermissionDenied since parent validation fails
        let non_existent = temp_path.join("does_not_exist.txt");
        let result = validate_path(&non_existent.display().to_string(), &allowed_dirs).await;
        assert!(result.is_err());
        // The error should be PermissionDenied because the path doesn't exist
        // and the initial boundary check fails before we get to parent directory validation
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::PermissionDenied { .. }
        ));
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
        assert!(is_path_within_allowed_directories(
            &sub_file,
            std::slice::from_ref(&allowed_path)
        ));

        // Test that parent directory access is blocked
        if let Some(parent) = allowed_path.parent() {
            let parent_file = parent.join("parent_file.txt");
            if parent_file.exists() || tokio::fs::write(&parent_file, "content").await.is_ok() {
                assert!(!is_path_within_allowed_directories(
                    &parent_file,
                    &[allowed_path]
                ));
            }
        }
    }

    /// Test edge cases and error conditions
    #[tokio::test]
    async fn test_edge_cases() {
        // Test is_path_allowed with empty allowed directories
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "content").await.unwrap();
        assert!(!is_path_within_allowed_directories(&file_path, &[]));

        // Test validate_path with empty allowed directories
        let result = validate_path(&file_path.display().to_string(), &[]).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::PermissionDenied { .. }
        ));
    }
}
