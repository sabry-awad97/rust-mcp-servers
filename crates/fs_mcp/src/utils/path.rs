use std::path::{Path, PathBuf};

/// Expand home directory (~) in path
///
/// Converts paths starting with ~ to the user's home directory
pub fn expand_home(path: &str) -> String {
    if path.starts_with('~')
        && let Some(home) = dirs::home_dir()
    {
        return path.replacen('~', &home.to_string_lossy(), 1);
    }
    path.to_string()
}

/// Normalize path by resolving components and converting to canonical form
pub fn normalize_path(path: &Path) -> PathBuf {
    path.components()
        .fold(PathBuf::new(), |mut result, component| {
            match component {
                std::path::Component::ParentDir => {
                    result.pop();
                }
                std::path::Component::CurDir => {
                    // Skip current directory components
                }
                _ => {
                    result.push(component);
                }
            }
            result
        })
}

/// Check if a given path is within allowed directories
///
/// This function performs a security check to determine if a path falls within
/// the boundaries of allowed directories using prefix matching.
pub fn is_path_within_allowed_directories(path: &Path, allowed_directories: &[PathBuf]) -> bool {
    // Canonicalize the path first to handle symlinks and relative paths
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false, // Non-existent paths are not allowed
    };

    allowed_directories
        .iter()
        .any(|allowed_dir| canonical_path.starts_with(allowed_dir))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use std::path::PathBuf;

    /// Test expand_home function with various scenarios
    #[test]
    fn test_expand_home() {
        // Test tilde expansion when home directory is available
        if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy();

            // Test basic tilde expansion
            let result = expand_home("~");
            assert_eq!(result, home_str);

            // Test tilde with path
            let result = expand_home("~/documents");
            assert_eq!(result, format!("{}/documents", home_str));

            // Test tilde with nested path
            let result = expand_home("~/documents/projects/rust");
            assert_eq!(result, format!("{}/documents/projects/rust", home_str));
        }

        // Test paths that don't start with tilde
        let result = expand_home("/absolute/path");
        assert_eq!(result, "/absolute/path");

        let result = expand_home("relative/path");
        assert_eq!(result, "relative/path");

        let result = expand_home("./current/path");
        assert_eq!(result, "./current/path");

        let result = expand_home("../parent/path");
        assert_eq!(result, "../parent/path");

        // Test empty string
        let result = expand_home("");
        assert_eq!(result, "");

        // Test path that contains tilde but doesn't start with it
        let result = expand_home("/path/with/~tilde/inside");
        assert_eq!(result, "/path/with/~tilde/inside");
    }

    /// Test normalize_path function with different path components
    #[test]
    fn test_normalize_path() {
        // Test simple path normalization
        let path = Path::new("simple/path");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::from("simple/path"));

        // Test path with current directory components
        let path = Path::new("./current/./directory/.");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::from("current/directory"));

        // Test path with parent directory components
        let path = Path::new("parent/../child");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::from("child"));

        // Test complex path with mixed components
        let path = Path::new("a/./b/../c/d/../../e");
        let result = normalize_path(path);
        // On Windows, this results in "a\e" due to path separator differences
        // On Unix, this results in "e"
        #[cfg(windows)]
        assert_eq!(result, PathBuf::from("a\\e"));
        #[cfg(not(windows))]
        assert_eq!(result, PathBuf::from("e"));

        // Test path that goes beyond root (should result in empty path)
        let path = Path::new("../../../..");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::new());

        // Test absolute path normalization (on Unix-like systems)
        #[cfg(unix)]
        {
            let path = Path::new("/absolute/./path/../normalized");
            let result = normalize_path(path);
            assert_eq!(result, PathBuf::from("/absolute/normalized"));
        }

        // Test Windows-style path normalization
        #[cfg(windows)]
        {
            let path = Path::new("C:\\windows\\./system32\\..\\temp");
            let result = normalize_path(path);
            assert_eq!(result, PathBuf::from("C:\\windows\\temp"));
        }
    }

    /// Test edge cases for path normalization
    #[test]
    fn test_normalize_path_edge_cases() {
        // Test empty path
        let path = Path::new("");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::new());

        // Test single dot
        let path = Path::new(".");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::new());

        // Test single parent directory
        let path = Path::new("..");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::new());

        // Test multiple consecutive dots
        let path = Path::new("./././.");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::new());

        // Test multiple consecutive parent dirs
        let path = Path::new("../../..");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::new());

        // Test path that normalizes to single component
        let path = Path::new("a/b/../..");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::new());

        // Test path with trailing separators (behavior may vary by platform)
        let path = Path::new("path/to/dir/");
        let result = normalize_path(path);
        // The trailing separator handling depends on the platform
        assert!(result == PathBuf::from("path/to/dir") || result == PathBuf::from("path/to/dir/"));
    }

    /// Test expand_home with special characters and Unicode
    #[test]
    fn test_expand_home_special_cases() {
        // Test with Unicode characters
        let result = expand_home("~/文档/项目");
        if let Some(home) = dirs::home_dir() {
            assert_eq!(result, format!("{}/文档/项目", home.to_string_lossy()));
        } else {
            assert_eq!(result, "~/文档/项目");
        }

        // Test with spaces
        let result = expand_home("~/My Documents/Project Files");
        if let Some(home) = dirs::home_dir() {
            assert_eq!(
                result,
                format!("{}/My Documents/Project Files", home.to_string_lossy())
            );
        } else {
            assert_eq!(result, "~/My Documents/Project Files");
        }

        // Test with special characters
        let result = expand_home("~/path with spaces & symbols!");
        if let Some(home) = dirs::home_dir() {
            assert_eq!(
                result,
                format!("{}/path with spaces & symbols!", home.to_string_lossy())
            );
        } else {
            assert_eq!(result, "~/path with spaces & symbols!");
        }
    }

    /// Test normalize_path with complex scenarios
    #[test]
    fn test_normalize_path_complex() {
        // Test deeply nested path with normalization
        let path = Path::new("a/b/c/../../d/./e/../f");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::from("a/d/f"));

        // Test path that creates and removes directories
        let path = Path::new("base/create/../remove/./keep");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::from("base/remove/keep"));

        // Test alternating current and parent directories
        let path = Path::new("./a/../b/./c/../d");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::from("b/d"));

        // Test path that results in going up from root
        let path = Path::new("a/../b/../c/../../d");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::from("d"));
    }

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
        assert!(is_path_within_allowed_directories(
            &allowed_file,
            std::slice::from_ref(&canonical_temp)
        ));

        // Test that paths outside the allowed directory are rejected
        let temp_dir2 = TempDir::new().unwrap();
        let disallowed_file = temp_dir2.path().join("test.txt");
        tokio::fs::write(&disallowed_file, "test").await.unwrap();
        assert!(!is_path_within_allowed_directories(
            &disallowed_file,
            &[canonical_temp]
        ));
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
        assert!(is_path_within_allowed_directories(
            &subfile,
            std::slice::from_ref(&temp_path)
        ));

        // Test nested subdirectory access
        let nested_dir = subdir.join("nested");
        tokio::fs::create_dir(&nested_dir).await.unwrap();
        let nested_file = nested_dir.join("nested.txt");
        tokio::fs::write(&nested_file, "nested").await.unwrap();
        assert!(is_path_within_allowed_directories(
            &nested_file,
            std::slice::from_ref(&temp_path)
        ));

        // Test multiple allowed directories
        let temp_dir2 = TempDir::new().unwrap();
        let temp_path2 = temp_dir2.path().canonicalize().unwrap();
        let file2 = temp_path2.join("file2.txt");
        tokio::fs::write(&file2, "content2").await.unwrap();
        assert!(is_path_within_allowed_directories(
            &file2,
            &[temp_path.clone(), temp_path2]
        ));

        // Test non-existent path
        let non_existent = temp_path.join("does_not_exist.txt");
        assert!(!is_path_within_allowed_directories(
            &non_existent,
            &[temp_path]
        ));
    }
}
