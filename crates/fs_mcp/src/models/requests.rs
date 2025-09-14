use derive_getters::Getters;
use rmcp::schemars;
use serde::Deserialize;

use crate::{
    errors::{FileSystemMcpError, FileSystemMcpResult},
    service::validation::Validate,
};

/// Request to read a text file
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct ReadTextFileRequest {
    /// Path to the file to read
    path: String,
    /// If provided, returns only the last N lines of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    tail: Option<usize>,
    /// If provided, returns only the first N lines of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    head: Option<usize>,
}

impl Validate for ReadTextFileRequest {
    fn validate(&self) -> FileSystemMcpResult<()> {
        if self.path.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid path".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Path is empty"}),
            });
        }

        if self.tail.is_some() && self.head.is_some() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Conflicting parameters provided".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Both tail and head are provided"}),
            });
        }
        Ok(())
    }
}

/// Request to read a media file
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct ReadMediaFileRequest {
    /// Path to the media file to read
    path: String,
}

impl Validate for ReadMediaFileRequest {
    fn validate(&self) -> FileSystemMcpResult<()> {
        if self.path.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid path".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Path is empty"}),
            });
        }

        Ok(())
    }
}

/// Request to read multiple files
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct ReadMultipleFilesRequest {
    /// Array of file paths to read
    paths: Vec<String>,
}

impl Validate for ReadMultipleFilesRequest {
    fn validate(&self) -> FileSystemMcpResult<()> {
        if self.paths.is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid paths".to_string(),
                path: self.paths.to_vec().join(", "),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Paths are empty"}),
            });
        }

        for path in &self.paths {
            if path.trim().is_empty() {
                return Err(FileSystemMcpError::ValidationError {
                    message: "Invalid path".to_string(),
                    path: path.clone(),
                    operation: "validate".to_string(),
                    data: serde_json::json!({"error": "Path is empty"}),
                });
            }
        }

        Ok(())
    }
}

/// Request to write a file
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct WriteFileRequest {
    /// Path to the file to write
    path: String,
    /// Content to write to the file
    content: String,
}

impl Validate for WriteFileRequest {
    fn validate(&self) -> Result<(), FileSystemMcpError> {
        if self.path.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid path".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Path is empty"}),
            });
        }

        // Additional validation for content size (optional safety check)
        if self.content.len() > 100_000_000 {
            // 100MB limit
            return Err(FileSystemMcpError::ValidationError {
                message: "Content too large".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({
                    "error": "Content exceeds maximum size limit",
                    "max_size": 100_000_000,
                    "actual_size": self.content.len()
                }),
            });
        }

        Ok(())
    }
}

/// Edit operation for file editing
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct EditOperation {
    /// Text to search for - must match exactly
    old_text: String,
    /// Text to replace with
    new_text: String,
}

impl EditOperation {
    /// Create a new EditOperation instance
    #[cfg(test)]
    pub fn new(old_text: String, new_text: String) -> Self {
        Self { old_text, new_text }
    }
}

/// Request to edit a file
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct EditFileRequest {
    /// Path to the file to edit
    path: String,
    /// Array of edit operations to perform
    edits: Vec<EditOperation>,
    /// Preview changes using git-style diff format
    #[serde(default)]
    dry_run: bool,
}

impl Validate for EditOperation {
    fn validate(&self) -> Result<(), FileSystemMcpError> {
        if self.old_text.is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid edit operation".to_string(),
                path: "edit_operation".to_string(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "old_text cannot be empty"}),
            });
        }

        // Note: new_text can be empty (for deletions), so we don't validate it

        // Validate that old_text doesn't contain only whitespace
        if self.old_text.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid edit operation".to_string(),
                path: "edit_operation".to_string(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "old_text cannot contain only whitespace"}),
            });
        }

        Ok(())
    }
}

impl Validate for EditFileRequest {
    fn validate(&self) -> Result<(), FileSystemMcpError> {
        if self.path.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid path".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Path is empty"}),
            });
        }

        if self.edits.is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "No edit operations provided".to_string(),
                path: self.path.clone(),
                operation: "validate".to_string(),
                data: serde_json::json!({"error": "Edits array is empty"}),
            });
        }

        // Validate each edit operation
        for (index, edit) in self.edits.iter().enumerate() {
            edit.validate().map_err(|mut e| {
                // Add context about which edit operation failed
                if let FileSystemMcpError::ValidationError { ref mut data, .. } = e
                    && let Some(obj) = data.as_object_mut()
                {
                    obj.insert("edit_index".to_string(), serde_json::json!(index));
                }
                e
            })?;
        }

        Ok(())
    }
}

/// Request to create a directory
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct CreateDirectoryRequest {
    /// Path to the directory to create
    path: String,
}

impl Validate for CreateDirectoryRequest {
    fn validate(&self) -> FileSystemMcpResult<()> {
        if self.path.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid path".to_string(),
                path: self.path.clone(),
                operation: "create_directory".to_string(),
                data: serde_json::json!({
                    "error": "Path cannot be empty",
                    "provided_path": self.path
                }),
            });
        }
        Ok(())
    }
}

/// Request to list directory contents
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct ListDirectoryRequest {
    /// Path to the directory to list
    path: String,
}

impl Validate for ListDirectoryRequest {
    fn validate(&self) -> FileSystemMcpResult<()> {
        if self.path.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid path".to_string(),
                path: self.path.clone(),
                operation: "list_directory".to_string(),
                data: serde_json::json!({
                    "error": "Path cannot be empty",
                    "provided_path": self.path
                }),
            });
        }
        Ok(())
    }
}

/// Sort options for directory listings
#[derive(Debug, Deserialize, schemars::JsonSchema, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    /// Sort by name (alphabetical)
    #[default]
    Name,
    /// Sort by file size (largest first)
    Size,
    /// Sort by modification time (newest first)
    Modified,
}

/// Request to list directory contents with sizes
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct ListDirectoryWithSizesRequest {
    /// Path to the directory to list
    path: String,
    /// Sort entries by name or size
    #[serde(rename = "sortBy", default)]
    sort_by: SortBy,
}

impl Validate for ListDirectoryWithSizesRequest {
    fn validate(&self) -> FileSystemMcpResult<()> {
        if self.path.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid path".to_string(),
                path: self.path.clone(),
                operation: "list_directory_with_sizes".to_string(),
                data: serde_json::json!({
                    "error": "Path cannot be empty",
                    "provided_path": self.path
                }),
            });
        }
        Ok(())
    }
}

/// Request to get directory tree
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct DirectoryTreeRequest {
    /// Path to the directory
    path: String,
    /// Patterns to exclude from the tree
    #[serde(default)]
    exclude_patterns: Vec<String>,
}

impl Validate for DirectoryTreeRequest {
    fn validate(&self) -> FileSystemMcpResult<()> {
        if self.path.trim().is_empty() {
            return Err(FileSystemMcpError::ValidationError {
                message: "Invalid path".to_string(),
                path: self.path.clone(),
                operation: "directory_tree".to_string(),
                data: serde_json::json!({
                    "error": "Path cannot be empty",
                    "provided_path": self.path
                }),
            });
        }
        Ok(())
    }
}
