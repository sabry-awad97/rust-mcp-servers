use async_recursion::async_recursion;
use async_trait::async_trait;
use globset::{Glob, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
};

use crate::{
    domain::FileOperations,
    errors::{FileSystemMcpError, FileSystemMcpResult},
    models::{
        requests::SortBy,
        responses::{ReadFileResponse, WriteFileResponse},
    },
};

/// Reusable directory entry information
#[derive(Debug, Clone)]
struct DirectoryEntry {
    name: String,
    file_type: String,
    size: u64,
    is_directory: bool,
    modified: Option<std::time::SystemTime>,
}

/// Tree entry for directory tree representation
#[derive(Debug, Serialize, Deserialize)]
struct TreeEntry {
    /// Name of the entry
    pub name: String,
    /// Type of the entry (file or directory)
    #[serde(rename = "type")]
    pub entry_type: String,
    /// Children entries (only for directories)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeEntry>>,
}

/// Application service implementing file operations
///
/// This service provides concrete implementations for all file operations
/// following SOLID principles and Domain-Driven Design patterns.
pub struct FileService;

impl FileService {
    /// Create a new FileService instance
    pub fn new() -> Self {
        Self
    }

    /// Reusable function to read file content as bytes using Node.js-style streaming
    ///
    /// This private method provides the core streaming functionality that can be
    /// reused by both text and media file reading operations.
    async fn read_file_bytes(&self, path: &Path) -> FileSystemMcpResult<Vec<u8>> {
        let file = File::open(path)
            .await
            .map_err(|_| FileSystemMcpError::PermissionDenied {
                path: path.display().to_string(),
            })?;

        // Use buffered reader for streaming chunks like Node.js
        let mut reader = BufReader::new(file);
        let mut contents = Vec::new();

        // Stream file in chunks
        const CHUNK_SIZE: usize = 8192; // 8KB chunks like Node.js default
        let mut buffer = vec![0u8; CHUNK_SIZE];

        loop {
            let bytes_read = reader.read(&mut buffer).await.map_err(|_| {
                FileSystemMcpError::PermissionDenied {
                    path: path.display().to_string(),
                }
            })?;

            if bytes_read == 0 {
                break; // End of file reached
            }

            // Append chunk to contents
            contents.extend_from_slice(&buffer[..bytes_read]);
        }

        Ok(contents)
    }

    /// Helper method to get file metadata
    async fn get_file_size(&self, path: &Path) -> Result<u64, std::io::Error> {
        let metadata = fs::metadata(path).await?;
        Ok(metadata.len())
    }

    /// Helper method to check if path exists
    async fn path_exists(&self, path: &Path) -> bool {
        self.get_file_size(path).await.is_ok()
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

    /// Efficiently read and collect directory entries with metadata
    async fn read_directory_entries(path: &Path) -> FileSystemMcpResult<Vec<DirectoryEntry>> {
        let mut entries = fs::read_dir(path)
            .await
            .map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to list directory: {}", e),
                path: path.display().to_string(),
            })?;

        let mut directory_entries = Vec::new();

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
            let is_directory = metadata.is_dir();
            let size = if is_directory { 0 } else { metadata.len() };
            let modified = metadata.modified().ok();

            let file_type = if is_directory {
                "[DIR]".to_string()
            } else if metadata.is_symlink() {
                "[SYMLINK]".to_string()
            } else {
                // Extract file extension for better type identification
                match std::path::Path::new(&name).extension() {
                    Some(ext) => format!("{} [FILE]", ext.to_string_lossy().to_lowercase()),
                    None => "[FILE]".to_string(),
                }
            };

            directory_entries.push(DirectoryEntry {
                name,
                file_type,
                size,
                is_directory,
                modified,
            });
        }

        Ok(directory_entries)
    }

    /// Sort directory entries based on the specified criteria
    fn sort_directory_entries(entries: &mut [DirectoryEntry], sort_by: &SortBy) {
        match sort_by {
            SortBy::Name => entries.sort_by(|a, b| a.name.cmp(&b.name)),
            SortBy::Size => entries.sort_by(|a, b| b.size.cmp(&a.size)),
            SortBy::Modified => entries.sort_by(|a, b| {
                match (a.modified, b.modified) {
                    (Some(a_time), Some(b_time)) => b_time.cmp(&a_time),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.name.cmp(&b.name), // Fallback to name
                }
            }),
        }
    }

    /// Get appropriate icon for file type
    fn get_file_icon(file_type: &str) -> &'static str {
        match file_type {
            "[DIR]" => "📁",
            "[SYMLINK]" => "🔗",
            t if t.contains("rs [FILE]") => "🦀",
            t if t.contains("js [FILE]") => "📜",
            t if t.contains("ts [FILE]") => "📘",
            t if t.contains("py [FILE]") => "🐍",
            t if t.contains("json [FILE]") => "📋",
            t if t.contains("toml [FILE]") => "⚙️",
            t if t.contains("yaml [FILE]") || t.contains("yml [FILE]") => "📄",
            t if t.contains("md [FILE]") => "📝",
            t if t.contains("txt [FILE]") => "📄",
            t if t.contains("log [FILE]") => "📊",
            t if t.contains("png [FILE]")
                || t.contains("jpg [FILE]")
                || t.contains("jpeg [FILE]")
                || t.contains("gif [FILE]") =>
            {
                "🖼️"
            }
            t if t.contains("pdf [FILE]") => "📕",
            t if t.contains("zip [FILE]")
                || t.contains("tar [FILE]")
                || t.contains("gz [FILE]") =>
            {
                "📦"
            }
            _ => "📄",
        }
    }

    /// Format detailed directory listing with statistics
    fn format_detailed_listing(entries: &[DirectoryEntry]) -> (Vec<String>, String) {
        let mut output = Vec::new();
        let mut total_files = 0;
        let mut total_dirs = 0;
        let mut total_size = 0;

        // Group by type for better organization
        let (directories, files): (Vec<_>, Vec<_>) =
            entries.iter().partition(|entry| entry.is_directory);

        if !directories.is_empty() {
            output.push("📂 Directories:".to_string());
            for dir in &directories {
                output.push(format!("  📁 {}/", dir.name));
                total_dirs += 1;
            }
            output.push(String::new());
        }

        if !files.is_empty() {
            output.push("📄 Files:".to_string());
            for file in &files {
                let icon = Self::get_file_icon(&file.file_type);
                let size_str = Self::format_size(file.size);
                output.push(format!("  {} {} ({:>8})", icon, file.name, size_str));
                total_files += 1;
                total_size += file.size;
            }
        }

        let stats = format!(
            "📊 Summary: {} directories, {} files | Total size: {}",
            total_dirs,
            total_files,
            Self::format_size(total_size)
        );

        (output, stats)
    }

    /// Format file size in human readable format
    fn format_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    #[async_recursion::async_recursion]
    async fn build_tree(
        base_path: &Path,
        current_path: &Path,
        exclude_patterns: &[String],
    ) -> Result<Vec<TreeEntry>, io::Error> {
        let mut entries = tokio::fs::read_dir(current_path).await?;
        let mut tree = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Calculate relative path from the base directory
            let relative_path = entry_path
                .strip_prefix(base_path)
                .unwrap_or(&entry_path)
                .to_string_lossy()
                .replace('\\', "/"); // Normalize path separators

            // Build globset for pattern matching
            let should_exclude = if exclude_patterns.is_empty() {
                false
            } else {
                let mut builder = GlobSetBuilder::new();

                // Add all patterns to the builder
                for pattern in exclude_patterns {
                    if let Ok(glob) = Glob::new(pattern) {
                        builder.add(glob);
                    }

                    // Also add patterns with ** prefix for nested matching
                    if !pattern.starts_with("**/")
                        && let Ok(nested_glob) = Glob::new(&format!("**/{}", pattern))
                    {
                        builder.add(nested_glob);
                    }

                    // For directory patterns like "components/*", don't match the directory itself
                    // We only want to exclude the contents, not the directory
                }

                if let Ok(globset) = builder.build() {
                    globset.is_match(&relative_path) || globset.is_match(&name)
                } else {
                    false
                }
            };

            if should_exclude {
                continue;
            }

            let metadata = entry.metadata().await?;
            let mut tree_entry = TreeEntry {
                name,
                entry_type: if metadata.is_dir() {
                    "[DIR]".to_string()
                } else {
                    "[FILE]".to_string()
                },
                children: None,
            };

            if metadata.is_dir() {
                tree_entry.children =
                    Some(Self::build_tree(base_path, &entry_path, exclude_patterns).await?);
            }

            tree.push(tree_entry);
        }

        Ok(tree)
    }

    #[async_recursion]
    async fn search_recursive(
        root_path: &Path,
        current_path: &Path,
        search_glob: &Glob,
        exclude_globset: &Option<globset::GlobSet>,
        results: &mut Vec<String>,
    ) -> FileSystemMcpResult<()> {
        let mut entries =
            fs::read_dir(current_path)
                .await
                .map_err(|e| FileSystemMcpError::IoError {
                    message: format!("Failed to read directory: {}", e),
                    path: current_path.display().to_string(),
                })?;

        while let Some(entry) =
            entries
                .next_entry()
                .await
                .map_err(|e| FileSystemMcpError::IoError {
                    message: format!("Failed to read directory entry: {}", e),
                    path: current_path.display().to_string(),
                })?
        {
            let entry_path = entry.path();
            let relative_path = entry_path
                .strip_prefix(root_path)
                .unwrap_or(&entry_path)
                .to_string_lossy()
                .replace('\\', "/");

            // Check exclude patterns
            if let Some(globset) = exclude_globset
                && globset.is_match(&relative_path)
            {
                continue;
            }

            // Check if matches search pattern
            if search_glob.compile_matcher().is_match(&relative_path) {
                results.push(entry_path.display().to_string());
            }

            // Recurse into directories
            if entry.metadata().await.map(|m| m.is_dir()).unwrap_or(false) {
                Self::search_recursive(
                    root_path,
                    &entry_path,
                    search_glob,
                    exclude_globset,
                    results,
                )
                .await?;
            }
        }

        Ok(())
    }
}

impl Default for FileService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileOperations for FileService {
    /// Read the entire contents of a file using reusable streaming function
    async fn read_entire_file(&self, path: &Path) -> FileSystemMcpResult<ReadFileResponse> {
        let bytes = self.read_file_bytes(path).await?;
        let contents = String::from_utf8_lossy(&bytes).to_string();
        Ok(ReadFileResponse::text(contents))
    }

    /// Read the first N lines using streaming with early termination
    async fn read_file_head(
        &self,
        path: &Path,
        lines: usize,
    ) -> FileSystemMcpResult<ReadFileResponse> {
        if lines == 0 {
            return Ok(ReadFileResponse::text(String::new()));
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

        Ok(ReadFileResponse::text(result_lines.join("\n")))
    }

    /// Read the last N lines using memory-efficient circular buffer
    async fn read_file_tail(
        &self,
        path: &Path,
        lines: usize,
    ) -> FileSystemMcpResult<ReadFileResponse> {
        if lines == 0 {
            return Ok(ReadFileResponse::text(String::new()));
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
        Ok(ReadFileResponse::text(
            circular_buffer
                .into_iter()
                .collect::<Vec<String>>()
                .join("\n"),
        ))
    }

    /// Read a media file and return base64-encoded content with MIME type
    async fn read_media_file(&self, path: &Path) -> FileSystemMcpResult<ReadFileResponse> {
        let bytes = self.read_file_bytes(path).await?;
        Ok(ReadFileResponse::new(bytes, path))
    }

    /// Read files concurrently using futures::join_all for scalability with many files
    async fn read_files(
        &self,
        paths: &[std::path::PathBuf],
    ) -> Vec<Result<crate::models::responses::ReadFileResponse, FileSystemMcpError>> {
        use futures::future::join_all;

        let futures: Vec<_> = paths
            .iter()
            .map(|path| self.read_entire_file(path))
            .collect();

        join_all(futures).await
    }

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
        let mut entries = Self::read_directory_entries(path).await?;
        Self::sort_directory_entries(&mut entries, &SortBy::Name);

        let (directories, files): (Vec<_>, Vec<_>) =
            entries.iter().partition(|entry| entry.is_directory);

        let mut output = Vec::new();
        output.push(format!("📁 Directory: {}", path.display()));
        output.push(String::new());

        if !directories.is_empty() {
            output.push("📂 Directories:".to_string());
            for dir in &directories {
                output.push(format!("  📁 {}/", dir.name));
            }
            output.push(String::new());
        }

        if !files.is_empty() {
            output.push("📄 Files:".to_string());
            for file in &files {
                let icon = Self::get_file_icon(&file.file_type);
                let size_info = if file.size > 0 {
                    format!(" ({})", Self::format_size(file.size))
                } else {
                    String::new()
                };
                output.push(format!("  {} {}{}", icon, file.name, size_info));
            }
            output.push(String::new());
        }

        output.push(format!(
            "📊 Summary: {} directories, {} files",
            directories.len(),
            files.len()
        ));

        Ok(WriteFileResponse::new(
            output.join("\n"),
            path.display().to_string(),
            None,
            false,
        ))
    }

    async fn list_directory_with_sizes(
        &self,
        path: &Path,
        sort_by: &SortBy,
    ) -> FileSystemMcpResult<WriteFileResponse> {
        let mut entries = Self::read_directory_entries(path).await?;
        Self::sort_directory_entries(&mut entries, sort_by);

        let mut output = Vec::new();
        output.push(format!(
            "📁 Directory: {} (sorted by {:?})",
            path.display(),
            sort_by
        ));
        output.push(String::new());

        let (content, stats) = Self::format_detailed_listing(&entries);
        output.extend(content);

        if !entries.is_empty() {
            output.push(String::new());
            output.push(stats);
        } else {
            output.push("📂 Empty directory".to_string());
        }

        Ok(WriteFileResponse::new(
            output.join("\n"),
            path.display().to_string(),
            None,
            false,
        ))
    }

    async fn directory_tree(
        &self,
        path: &Path,
        exclude_patterns: &[String],
    ) -> FileSystemMcpResult<WriteFileResponse> {
        match Self::build_tree(path, path, exclude_patterns).await {
            Ok(tree) => Ok(WriteFileResponse::new(
                serde_json::to_string_pretty(&tree).unwrap(),
                path.display().to_string(),
                None,
                false,
            )),
            Err(e) => Err(FileSystemMcpError::IoError {
                message: format!("Failed to build directory tree: {}", e),
                path: path.display().to_string(),
            }),
        }
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

    async fn search_files(
        &self,
        path: &Path,
        pattern: &str,
        _allowed_directories: &[PathBuf],
        exclude_patterns: &[String],
    ) -> FileSystemMcpResult<WriteFileResponse> {
        let mut results = Vec::new();

        // Build globset for pattern matching
        let search_glob = Glob::new(pattern).map_err(|e| FileSystemMcpError::ValidationError {
            message: format!("Invalid search pattern: {}", e),
            path: path.display().to_string(),
            operation: "search_files".to_string(),
            data: serde_json::json!({
                "error": "Invalid glob pattern",
                "pattern": pattern
            }),
        })?;

        let mut exclude_globset = None;
        if !exclude_patterns.is_empty() {
            let mut builder = GlobSetBuilder::new();
            for exclude_pattern in exclude_patterns {
                if let Ok(glob) = Glob::new(exclude_pattern) {
                    builder.add(glob);
                }
            }
            exclude_globset = builder.build().ok();
        }

        Self::search_recursive(path, path, &search_glob, &exclude_globset, &mut results).await?;

        let results_json =
            serde_json::to_string_pretty(&results).map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to serialize search results: {}", e),
                path: path.display().to_string(),
            })?;

        Ok(WriteFileResponse::new(
            results_json,
            path.display().to_string(),
            None,
            false,
        ))
    }

    async fn get_file_info(&self, path: &Path) -> FileSystemMcpResult<WriteFileResponse> {
        let metadata = fs::metadata(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FileSystemMcpError::PathNotFound {
                    path: path.display().to_string(),
                }
            } else {
                FileSystemMcpError::IoError {
                    message: format!("Failed to get file metadata: {}", e),
                    path: path.display().to_string(),
                }
            }
        })?;

        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let file_type = if metadata.is_dir() {
            "[DIRECTORY]".to_string()
        } else if metadata.is_file() {
            "[FILE]".to_string()
        } else {
            "[OTHER]".to_string()
        };

        let file_info = DirectoryEntry {
            name: file_name,
            file_type,
            size: metadata.len(),
            is_directory: metadata.is_dir(),
            modified: metadata.modified().ok(),
        };

        let info_json = serde_json::json!({
            "name": file_info.name,
            "type": file_info.file_type,
            "size": file_info.size,
            "is_directory": file_info.is_directory,
            "modified": file_info.modified.map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }),
            "path": path.display().to_string(),
            "permissions": {
                "readable": true,
                "writable": !metadata.permissions().readonly(),
                "executable": false
            }
        });

        let info_string =
            serde_json::to_string_pretty(&info_json).map_err(|e| FileSystemMcpError::IoError {
                message: format!("Failed to serialize file info: {}", e),
                path: path.display().to_string(),
            })?;

        Ok(WriteFileResponse::new(
            info_string,
            path.display().to_string(),
            None,
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, sync::Arc};
    use tempfile::{NamedTempFile, TempDir};

    async fn create_test_file(content: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(content.as_bytes())
            .expect("Failed to write test content");
        temp_file
    }

    #[tokio::test]
    async fn test_read_entire_file() {
        let service = FileService::new();
        let temp_file = create_test_file("line1\nline2\nline3").await;

        let result = service.read_entire_file(temp_file.path()).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        if let crate::models::responses::FileContent::Text(content) = response.content {
            assert_eq!(content, "line1\nline2\nline3");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_read_file_head() {
        let service = FileService::new();
        let temp_file = create_test_file("line1\nline2\nline3\nline4\nline5").await;

        let result = service.read_file_head(temp_file.path(), 3).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        if let crate::models::responses::FileContent::Text(content) = response.content {
            assert_eq!(content, "line1\nline2\nline3");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_read_file_head_zero_lines() {
        let service = FileService::new();
        let temp_file = create_test_file("line1\nline2\nline3").await;

        let result = service.read_file_head(temp_file.path(), 0).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        if let crate::models::responses::FileContent::Text(content) = response.content {
            assert_eq!(content, "");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_read_file_tail() {
        let service = FileService::new();
        let temp_file = create_test_file("line1\nline2\nline3\nline4\nline5").await;

        let result = service.read_file_tail(temp_file.path(), 3).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        if let crate::models::responses::FileContent::Text(content) = response.content {
            assert_eq!(content, "line3\nline4\nline5");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_read_file_tail_zero_lines() {
        let service = FileService::new();
        let temp_file = create_test_file("line1\nline2\nline3").await;

        let result = service.read_file_tail(temp_file.path(), 0).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        if let crate::models::responses::FileContent::Text(content) = response.content {
            assert_eq!(content, "");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let service = FileService::new();
        let nonexistent_path = Path::new("/nonexistent/file.txt");

        let result = service.read_entire_file(nonexistent_path).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::PermissionDenied { .. }
        ));
    }

    #[tokio::test]
    async fn test_read_files_success() {
        let service = FileService::new();

        // Create multiple test files
        let temp_file1 = create_test_file("content of file 1").await;
        let temp_file2 = create_test_file("content of file 2").await;
        let temp_file3 = create_test_file("content of file 3").await;

        let paths = vec![
            temp_file1.path().to_path_buf(),
            temp_file2.path().to_path_buf(),
            temp_file3.path().to_path_buf(),
        ];

        let results = service.read_files(&paths).await;

        // All files should be read successfully
        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
        assert!(results[2].is_ok());

        // Verify content
        if let Ok(response) = &results[0] {
            if let crate::models::responses::FileContent::Text(content) = &response.content {
                assert_eq!(content, "content of file 1");
            } else {
                panic!("Expected text content");
            }
        }

        if let Ok(response) = &results[1] {
            if let crate::models::responses::FileContent::Text(content) = &response.content {
                assert_eq!(content, "content of file 2");
            } else {
                panic!("Expected text content");
            }
        }

        if let Ok(response) = &results[2] {
            if let crate::models::responses::FileContent::Text(content) = &response.content {
                assert_eq!(content, "content of file 3");
            } else {
                panic!("Expected text content");
            }
        }
    }

    #[tokio::test]
    async fn test_read_files_empty_list() {
        let service = FileService::new();
        let paths: Vec<std::path::PathBuf> = vec![];

        let results = service.read_files(&paths).await;

        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_read_files_mixed_success_and_failure() {
        let service = FileService::new();

        // Create one valid file and one invalid path
        let temp_file = create_test_file("valid content").await;
        let nonexistent_path = std::path::PathBuf::from("/nonexistent/file.txt");

        let paths = vec![temp_file.path().to_path_buf(), nonexistent_path];

        let results = service.read_files(&paths).await;

        // Should have results for both attempts
        assert_eq!(results.len(), 2);

        // First file should succeed
        assert!(results[0].is_ok());
        if let Ok(response) = &results[0] {
            if let crate::models::responses::FileContent::Text(content) = &response.content {
                assert_eq!(content, "valid content");
            } else {
                panic!("Expected text content");
            }
        }

        // Second file should fail
        assert!(results[1].is_err());
        assert!(matches!(
            results[1].as_ref().unwrap_err(),
            FileSystemMcpError::PermissionDenied { .. }
        ));
    }

    #[tokio::test]
    async fn test_read_files_all_failures() {
        let service = FileService::new();

        let paths = vec![
            std::path::PathBuf::from("/nonexistent/file1.txt"),
            std::path::PathBuf::from("/nonexistent/file2.txt"),
            std::path::PathBuf::from("/nonexistent/file3.txt"),
        ];

        let results = service.read_files(&paths).await;

        // All should fail
        assert_eq!(results.len(), 3);
        assert!(results[0].is_err());
        assert!(results[1].is_err());
        assert!(results[2].is_err());

        // Verify error types
        for result in &results {
            assert!(matches!(
                result.as_ref().unwrap_err(),
                FileSystemMcpError::PermissionDenied { .. }
            ));
        }
    }

    #[tokio::test]
    async fn test_read_files_single_file() {
        let service = FileService::new();
        let temp_file = create_test_file("single file content").await;

        let paths = vec![temp_file.path().to_path_buf()];

        let results = service.read_files(&paths).await;

        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());

        if let Ok(response) = &results[0] {
            if let crate::models::responses::FileContent::Text(content) = &response.content {
                assert_eq!(content, "single file content");
            } else {
                panic!("Expected text content");
            }
        }
    }

    #[tokio::test]
    async fn test_read_files_large_batch() {
        let service = FileService::new();

        // Create 10 test files to test concurrent processing
        let mut temp_files = Vec::new();
        let mut paths = Vec::new();

        for i in 0..10 {
            let temp_file = create_test_file(&format!("content of file {}", i)).await;
            paths.push(temp_file.path().to_path_buf());
            temp_files.push(temp_file); // Keep files alive
        }

        let results = service.read_files(&paths).await;

        // All files should be read successfully
        assert_eq!(results.len(), 10);

        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok(), "File {} should be read successfully", i);

            if let Ok(response) = result {
                if let crate::models::responses::FileContent::Text(content) = &response.content {
                    assert_eq!(content, &format!("content of file {}", i));
                } else {
                    panic!("Expected text content for file {}", i);
                }
            }
        }
    }

    async fn create_temp_file_with_content(content: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(content.as_bytes())
            .expect("Failed to write test content");
        temp_file
    }

    #[tokio::test]
    async fn test_write_file_new() {
        let service = FileService::new();
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
        let service = FileService::new();
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
        let service = FileService::new();
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
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let result = service.list_directory(temp_dir.path()).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.message.contains("📁 Directory:"));
        assert!(
            response
                .message
                .contains("📊 Summary: 0 directories, 0 files")
        );
    }

    #[tokio::test]
    async fn test_list_directory_with_files() {
        let service = FileService::new();
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
        assert!(response.message.contains("📁 Directory:"));
        assert!(response.message.contains("📄 Files:"));

        // Check that all files are listed with emojis
        assert!(response.message.contains("📄 test.txt"));
        assert!(response.message.contains("⚙️ config.toml"));
        assert!(response.message.contains("🦀 script.rs"));
        assert!(response.message.contains("📄 no_extension"));

        // Check summary
        assert!(
            response
                .message
                .contains("📊 Summary: 0 directories, 4 files")
        );
    }

    #[tokio::test]
    async fn test_list_directory_with_subdirectories() {
        let service = FileService::new();
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
        assert!(response.message.contains("📁 Directory:"));
        assert!(response.message.contains("📂 Directories:"));
        assert!(response.message.contains("📄 Files:"));

        // Check that directories are listed correctly
        assert!(response.message.contains("📁 subdir1/"));
        assert!(response.message.contains("📁 subdir2/"));
        assert!(response.message.contains("📝 readme.md"));

        // Check summary
        assert!(
            response
                .message
                .contains("📊 Summary: 2 directories, 1 files")
        );

        // Directories should not have size information
        assert!(!response.message.contains("subdir1 - directory ("));
        assert!(!response.message.contains("subdir2 - directory ("));
    }

    #[tokio::test]
    async fn test_list_directory_sorted_output() {
        let service = FileService::new();
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
        let service = FileService::new();
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
        let service = FileService::new();
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
        assert!(content.contains("📁 docs/"));
        assert!(content.contains("📁 src/"));
        assert!(content.contains("⚙️ Cargo.toml"));
        assert!(content.contains("📝 README.md"));
        assert!(content.contains("🦀 main.rs"));
        assert!(content.contains("📋 data.json"));

        // Check sections are present
        assert!(content.contains("📂 Directories:"));
        assert!(content.contains("📄 Files:"));
        assert!(content.contains("📊 Summary: 2 directories, 4 files"));
    }

    #[tokio::test]
    async fn test_list_directory_with_sizes_empty() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let result = service
            .list_directory_with_sizes(temp_dir.path(), &SortBy::Name)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.message.contains("📁 Directory:"));
        assert!(response.message.contains("📂 Empty directory"));
    }

    #[tokio::test]
    async fn test_list_directory_with_sizes_mixed_content() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create test files with different sizes
        fs::write(temp_dir.path().join("small.txt"), "Hi")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("large.txt"), "A".repeat(1024))
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("subdir"))
            .await
            .unwrap();

        let result = service
            .list_directory_with_sizes(temp_dir.path(), &SortBy::Name)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();

        // Check file entries with sizes
        assert!(response.message.contains("📄 large.txt"));
        assert!(response.message.contains("📄 small.txt"));
        assert!(response.message.contains("📁 subdir/"));

        // Check statistics
        assert!(
            response
                .message
                .contains("📊 Summary: 1 directories, 2 files")
        );
        assert!(response.message.contains("Total size:"));
    }

    #[tokio::test]
    async fn test_list_directory_with_sizes_sort_by_size() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create files with different sizes
        fs::write(temp_dir.path().join("tiny.txt"), "x")
            .await
            .unwrap(); // 1 byte
        fs::write(temp_dir.path().join("huge.txt"), "X".repeat(2048))
            .await
            .unwrap(); // 2048 bytes
        fs::write(temp_dir.path().join("medium.txt"), "M".repeat(512))
            .await
            .unwrap(); // 512 bytes

        let result = service
            .list_directory_with_sizes(temp_dir.path(), &SortBy::Size)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let lines: Vec<&str> = response.message.lines().collect();

        // Find file positions (should be sorted by size, largest first)
        let huge_pos = lines
            .iter()
            .position(|line| line.contains("huge.txt"))
            .unwrap();
        let medium_pos = lines
            .iter()
            .position(|line| line.contains("medium.txt"))
            .unwrap();
        let tiny_pos = lines
            .iter()
            .position(|line| line.contains("tiny.txt"))
            .unwrap();

        assert!(huge_pos < medium_pos);
        assert!(medium_pos < tiny_pos);
    }

    #[tokio::test]
    async fn test_list_directory_with_sizes_sort_by_name() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create files in non-alphabetical order
        fs::write(temp_dir.path().join("zebra.txt"), "content")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("alpha.txt"), "content")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("beta.txt"), "content")
            .await
            .unwrap();

        let result = service
            .list_directory_with_sizes(temp_dir.path(), &SortBy::Name)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let lines: Vec<&str> = response.message.lines().collect();

        // Find file positions (should be sorted alphabetically)
        let alpha_pos = lines
            .iter()
            .position(|line| line.contains("alpha.txt"))
            .unwrap();
        let beta_pos = lines
            .iter()
            .position(|line| line.contains("beta.txt"))
            .unwrap();
        let zebra_pos = lines
            .iter()
            .position(|line| line.contains("zebra.txt"))
            .unwrap();

        assert!(alpha_pos < beta_pos);
        assert!(beta_pos < zebra_pos);
    }

    #[tokio::test]
    async fn test_list_directory_with_sizes_human_readable_sizes() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create files with specific sizes to test formatting
        fs::write(temp_dir.path().join("bytes.txt"), "A".repeat(500))
            .await
            .unwrap(); // 500 B
        fs::write(temp_dir.path().join("kilobytes.txt"), "B".repeat(1536))
            .await
            .unwrap(); // 1.5 KB
        fs::write(temp_dir.path().join("megabytes.txt"), "C".repeat(1_572_864))
            .await
            .unwrap(); // 1.5 MB

        let result = service
            .list_directory_with_sizes(temp_dir.path(), &SortBy::Size)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();

        // Check human-readable size formatting
        assert!(response.message.contains("1.5 MB"));
        assert!(response.message.contains("1.5 KB"));
        assert!(response.message.contains("500 B"));
    }

    #[tokio::test]
    async fn test_list_directory_with_sizes_directories_no_size() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create directories and files
        fs::create_dir(temp_dir.path().join("dir1")).await.unwrap();
        fs::create_dir(temp_dir.path().join("dir2")).await.unwrap();
        fs::write(temp_dir.path().join("file.txt"), "content")
            .await
            .unwrap();

        let result = service
            .list_directory_with_sizes(temp_dir.path(), &SortBy::Name)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();

        // Directories should not have size information displayed
        let lines: Vec<&str> = response.message.lines().collect();
        let dir_lines: Vec<&str> = lines
            .iter()
            .filter(|line| line.contains("[DIR]"))
            .cloned()
            .collect();

        for dir_line in dir_lines {
            // Directory lines should end with just the name, no size
            assert!(!dir_line.contains("B"));
            assert!(!dir_line.contains("KB"));
            assert!(!dir_line.contains("MB"));
        }
    }

    #[tokio::test]
    async fn test_list_directory_with_sizes_statistics_accuracy() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create known content
        fs::write(temp_dir.path().join("file1.txt"), "A".repeat(100))
            .await
            .unwrap(); // 100 bytes
        fs::write(temp_dir.path().join("file2.txt"), "B".repeat(200))
            .await
            .unwrap(); // 200 bytes
        fs::create_dir(temp_dir.path().join("dir1")).await.unwrap();
        fs::create_dir(temp_dir.path().join("dir2")).await.unwrap();

        let result = service
            .list_directory_with_sizes(temp_dir.path(), &SortBy::Name)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();

        // Verify exact statistics
        assert!(
            response
                .message
                .contains("📊 Summary: 2 directories, 2 files")
        );
        assert!(response.message.contains("Total size: 300 B"));
    }

    #[tokio::test]
    async fn test_list_directory_with_sizes_nonexistent_path() {
        let service = FileService::new();
        let nonexistent_path = std::path::Path::new("/nonexistent/directory");

        let result = service
            .list_directory_with_sizes(nonexistent_path, &SortBy::Name)
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FileSystemMcpError::IoError { .. }
        ));
    }

    #[tokio::test]
    async fn test_move_file() {
        let service = FileService::new();
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
    async fn test_write_file_with_nested_directories() {
        let service = FileService::new();
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
        let service = FileService::new();
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
        let service = FileService::new();
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
        let service = FileService::new();
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

        let service = Arc::new(FileService::new());
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
        let service = FileService::new();
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
        let service = FileService::new();
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
        let service = FileService::new();
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
        let service = FileService::new();

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
        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

        let service = FileService::new();
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

    #[tokio::test]
    async fn test_directory_tree_empty_directory() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let result = service.directory_tree(temp_dir.path(), &[]).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let tree: Vec<TreeEntry> = serde_json::from_str(&response.message).unwrap();
        assert!(tree.is_empty());
    }

    #[tokio::test]
    async fn test_directory_tree_with_files_and_directories() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create test structure
        fs::write(temp_dir.path().join("file1.txt"), "content1")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("file2.rs"), "content2")
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("subdir1"))
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("subdir2"))
            .await
            .unwrap();

        // Create nested structure
        fs::write(temp_dir.path().join("subdir1/nested_file.txt"), "nested")
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("subdir1/nested_dir"))
            .await
            .unwrap();

        let result = service.directory_tree(temp_dir.path(), &[]).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let tree: Vec<TreeEntry> = serde_json::from_str(&response.message).unwrap();

        // Should have 4 entries at root level
        assert_eq!(tree.len(), 4);

        // Check file entries
        let files: Vec<_> = tree.iter().filter(|e| e.entry_type == "[FILE]").collect();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.name == "file1.txt"));
        assert!(files.iter().any(|f| f.name == "file2.rs"));

        // Check directory entries
        let dirs: Vec<_> = tree.iter().filter(|e| e.entry_type == "[DIR]").collect();
        assert_eq!(dirs.len(), 2);
        assert!(dirs.iter().any(|d| d.name == "subdir1"));
        assert!(dirs.iter().any(|d| d.name == "subdir2"));

        // Check nested structure in subdir1
        let subdir1 = dirs.iter().find(|d| d.name == "subdir1").unwrap();
        assert!(subdir1.children.is_some());
        let children = subdir1.children.as_ref().unwrap();
        assert_eq!(children.len(), 2);
        assert!(
            children
                .iter()
                .any(|c| c.name == "nested_file.txt" && c.entry_type == "[FILE]")
        );
        assert!(
            children
                .iter()
                .any(|c| c.name == "nested_dir" && c.entry_type == "[DIR]")
        );
    }

    #[tokio::test]
    async fn test_directory_tree_with_exclude_patterns() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create test structure
        fs::write(temp_dir.path().join("file1.txt"), "content1")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("file2.rs"), "content2")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("temp.log"), "log content")
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("target"))
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("src")).await.unwrap();

        // Test excluding by extension
        let exclude_patterns = vec!["*.log".to_string(), "target".to_string()];
        let result = service
            .directory_tree(temp_dir.path(), &exclude_patterns)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let tree: Vec<TreeEntry> = serde_json::from_str(&response.message).unwrap();

        // Should exclude temp.log and target directory
        assert_eq!(tree.len(), 3); // file1.txt, file2.rs, src
        assert!(tree.iter().any(|e| e.name == "file1.txt"));
        assert!(tree.iter().any(|e| e.name == "file2.rs"));
        assert!(tree.iter().any(|e| e.name == "src"));
        assert!(!tree.iter().any(|e| e.name == "temp.log"));
        assert!(!tree.iter().any(|e| e.name == "target"));
    }

    #[tokio::test]
    async fn test_directory_tree_with_wildcard_patterns() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create test structure
        fs::write(temp_dir.path().join("test1.txt"), "content1")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("test2.txt"), "content2")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("readme.md"), "readme")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("config.json"), "config")
            .await
            .unwrap();

        // Test excluding all .txt files
        let exclude_patterns = vec!["*.txt".to_string()];
        let result = service
            .directory_tree(temp_dir.path(), &exclude_patterns)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let tree: Vec<TreeEntry> = serde_json::from_str(&response.message).unwrap();

        // Should only have readme.md and config.json
        assert_eq!(tree.len(), 2);
        assert!(tree.iter().any(|e| e.name == "readme.md"));
        assert!(tree.iter().any(|e| e.name == "config.json"));
        assert!(!tree.iter().any(|e| e.name == "test1.txt"));
        assert!(!tree.iter().any(|e| e.name == "test2.txt"));
    }

    #[tokio::test]
    async fn test_directory_tree_nested_exclusion() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create nested structure
        fs::create_dir(temp_dir.path().join("src")).await.unwrap();
        fs::create_dir(temp_dir.path().join("src/components"))
            .await
            .unwrap();
        fs::write(temp_dir.path().join("src/main.rs"), "main code")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "lib code")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("src/components/button.rs"), "button")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("src/components/input.rs"), "input")
            .await
            .unwrap();

        // Test excluding specific nested files - use more specific patterns
        let exclude_patterns = vec!["lib.rs".to_string(), "src/components/*".to_string()];
        let result = service
            .directory_tree(temp_dir.path(), &exclude_patterns)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let tree: Vec<TreeEntry> = serde_json::from_str(&response.message).unwrap();

        // Should have src directory
        assert_eq!(tree.len(), 1);
        let src_dir = &tree[0];
        assert_eq!(src_dir.name, "src");
        assert_eq!(src_dir.entry_type, "[DIR]");

        // src should contain main.rs and components directory (lib.rs excluded)
        let src_children = src_dir.children.as_ref().unwrap();
        assert_eq!(src_children.len(), 2);
        assert!(src_children.iter().any(|c| c.name == "main.rs"));
        assert!(src_children.iter().any(|c| c.name == "components"));
        assert!(!src_children.iter().any(|c| c.name == "lib.rs"));

        // components directory should be empty due to exclusion
        let components_dir = src_children
            .iter()
            .find(|c| c.name == "components")
            .unwrap();
        let components_children = components_dir.children.as_ref().unwrap();
        assert!(components_children.is_empty());
    }

    #[tokio::test]
    async fn test_directory_tree_nonexistent_path() {
        let service = FileService::new();
        let nonexistent_path = Path::new("/nonexistent/path/that/does/not/exist");

        let result = service.directory_tree(nonexistent_path, &[]).await;
        assert!(result.is_err());

        if let Err(FileSystemMcpError::IoError { message, path }) = result {
            assert!(message.contains("Failed to build directory tree"));
            assert_eq!(path, nonexistent_path.display().to_string());
        } else {
            panic!("Expected IoError for nonexistent path");
        }
    }

    #[tokio::test]
    async fn test_directory_tree_json_format() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create simple structure
        fs::write(temp_dir.path().join("test.txt"), "content")
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("folder"))
            .await
            .unwrap();

        let result = service.directory_tree(temp_dir.path(), &[]).await;
        assert!(result.is_ok());

        let response = result.unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&response.message).unwrap();
        assert!(parsed.is_array());

        // Verify structure
        let tree: Vec<TreeEntry> = serde_json::from_str(&response.message).unwrap();
        assert_eq!(tree.len(), 2);

        // Check JSON contains expected fields
        assert!(response.message.contains("\"name\""));
        assert!(response.message.contains("\"type\""));
        assert!(response.message.contains("\"children\""));
        assert!(response.message.contains("[FILE]"));
        assert!(response.message.contains("[DIR]"));
    }

    #[tokio::test]
    async fn test_directory_tree_deep_nesting() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create deep nested structure
        let deep_path = temp_dir.path().join("level1/level2/level3");
        fs::create_dir_all(&deep_path).await.unwrap();
        fs::write(deep_path.join("deep_file.txt"), "deep content")
            .await
            .unwrap();

        let result = service.directory_tree(temp_dir.path(), &[]).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let tree: Vec<TreeEntry> = serde_json::from_str(&response.message).unwrap();

        // Navigate through the nested structure
        assert_eq!(tree.len(), 1);
        let level1 = &tree[0];
        assert_eq!(level1.name, "level1");
        assert_eq!(level1.entry_type, "[DIR]");

        let level1_children = level1.children.as_ref().unwrap();
        assert_eq!(level1_children.len(), 1);
        let level2 = &level1_children[0];
        assert_eq!(level2.name, "level2");

        let level2_children = level2.children.as_ref().unwrap();
        assert_eq!(level2_children.len(), 1);
        let level3 = &level2_children[0];
        assert_eq!(level3.name, "level3");

        let level3_children = level3.children.as_ref().unwrap();
        assert_eq!(level3_children.len(), 1);
        let deep_file = &level3_children[0];
        assert_eq!(deep_file.name, "deep_file.txt");
        assert_eq!(deep_file.entry_type, "[FILE]");
        assert!(deep_file.children.is_none());
    }

    #[tokio::test]
    async fn test_search_files_basic_pattern() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create test files
        fs::write(temp_dir.path().join("test1.txt"), "content1")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("test2.rs"), "content2")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("readme.md"), "readme")
            .await
            .unwrap();

        let result = service
            .search_files(temp_dir.path(), "*.txt", &[], &[])
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let results: Vec<String> = serde_json::from_str(&response.message).unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].ends_with("test1.txt"));
    }

    #[tokio::test]
    async fn test_search_files_recursive() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create nested structure
        fs::create_dir(temp_dir.path().join("src")).await.unwrap();
        fs::create_dir(temp_dir.path().join("src/components"))
            .await
            .unwrap();

        fs::write(temp_dir.path().join("main.rs"), "main code")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "lib code")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("src/components/button.rs"), "button")
            .await
            .unwrap();

        let result = service
            .search_files(temp_dir.path(), "*.rs", &[], &[])
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let results: Vec<String> = serde_json::from_str(&response.message).unwrap();

        assert_eq!(results.len(), 3);
        assert!(results.iter().any(|r| r.ends_with("main.rs")));
        assert!(results.iter().any(|r| r.ends_with("lib.rs")));
        assert!(results.iter().any(|r| r.ends_with("button.rs")));
    }

    #[tokio::test]
    async fn test_search_files_with_exclude_patterns() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create test files
        fs::write(temp_dir.path().join("main.rs"), "main code")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("lib.rs"), "lib code")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("test.rs"), "test code")
            .await
            .unwrap();

        let exclude_patterns = vec!["**/lib.rs".to_string()];
        let result = service
            .search_files(temp_dir.path(), "*.rs", &[], &exclude_patterns)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let results: Vec<String> = serde_json::from_str(&response.message).unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.ends_with("main.rs")));
        assert!(results.iter().any(|r| r.ends_with("test.rs")));
        assert!(!results.iter().any(|r| r.ends_with("lib.rs")));
    }

    #[tokio::test]
    async fn test_search_files_wildcard_patterns() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create nested structure
        fs::create_dir(temp_dir.path().join("src")).await.unwrap();
        fs::create_dir(temp_dir.path().join("tests")).await.unwrap();

        fs::write(temp_dir.path().join("src/main.rs"), "main")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("tests/integration.rs"), "test")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("readme.txt"), "readme")
            .await
            .unwrap();

        let result = service
            .search_files(temp_dir.path(), "**/main.rs", &[], &[])
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let results: Vec<String> = serde_json::from_str(&response.message).unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].ends_with("main.rs"));
    }

    #[tokio::test]
    async fn test_search_files_no_matches() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create test files that won't match
        fs::write(temp_dir.path().join("test.txt"), "content")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("readme.md"), "readme")
            .await
            .unwrap();

        let result = service
            .search_files(temp_dir.path(), "*.rs", &[], &[])
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let results: Vec<String> = serde_json::from_str(&response.message).unwrap();

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_files_invalid_pattern() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let result = service
            .search_files(temp_dir.path(), "[invalid", &[], &[])
            .await;
        assert!(result.is_err());

        if let Err(FileSystemMcpError::ValidationError {
            message, operation, ..
        }) = result
        {
            assert!(message.contains("Invalid search pattern"));
            assert_eq!(operation, "search_files");
        } else {
            panic!("Expected ValidationError for invalid pattern");
        }
    }

    #[tokio::test]
    async fn test_search_files_nonexistent_directory() {
        let service = FileService::new();
        let nonexistent_path = Path::new("/nonexistent/path");

        let result = service
            .search_files(nonexistent_path, "*.txt", &[], &[])
            .await;
        assert!(result.is_err());

        if let Err(FileSystemMcpError::IoError { message, .. }) = result {
            assert!(message.contains("Failed to read directory"));
        } else {
            panic!("Expected IoError for nonexistent directory");
        }
    }

    #[tokio::test]
    async fn test_search_files_complex_exclude_patterns() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create complex nested structure
        fs::create_dir_all(temp_dir.path().join("src/components"))
            .await
            .unwrap();
        fs::create_dir_all(temp_dir.path().join("target/debug"))
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("tests")).await.unwrap();

        fs::write(temp_dir.path().join("src/main.rs"), "main")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("src/components/button.rs"), "button")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("target/debug/app.exe"), "binary")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("tests/integration.rs"), "test")
            .await
            .unwrap();

        let exclude_patterns = vec!["target/**".to_string(), "**/components/*".to_string()];
        let result = service
            .search_files(temp_dir.path(), "**/*", &[], &exclude_patterns)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let results: Vec<String> = serde_json::from_str(&response.message).unwrap();

        // Should find main.rs and integration.rs, but not button.rs or app.exe
        assert!(results.iter().any(|r| r.ends_with("main.rs")));
        assert!(results.iter().any(|r| r.ends_with("integration.rs")));
        assert!(!results.iter().any(|r| r.ends_with("button.rs")));
        assert!(!results.iter().any(|r| r.ends_with("app.exe")));
    }

    #[tokio::test]
    async fn test_search_files_directory_matching() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create directories and files
        fs::create_dir(temp_dir.path().join("src")).await.unwrap();
        fs::create_dir(temp_dir.path().join("tests")).await.unwrap();
        fs::write(temp_dir.path().join("readme.txt"), "readme")
            .await
            .unwrap();

        // Search for directories
        let result = service.search_files(temp_dir.path(), "src", &[], &[]).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let results: Vec<String> = serde_json::from_str(&response.message).unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].ends_with("src"));
    }

    #[tokio::test]
    async fn test_get_file_info_file() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_file.txt");

        // Create test file
        fs::write(&file_path, "test content").await.unwrap();

        let result = service.get_file_info(&file_path).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let info: serde_json::Value = serde_json::from_str(&response.message).unwrap();

        assert_eq!(info["name"], "test_file.txt");
        assert_eq!(info["type"], "[FILE]");
        assert_eq!(info["size"], 12); // "test content" is 12 bytes
        assert_eq!(info["is_directory"], false);
        assert!(info["path"].as_str().unwrap().ends_with("test_file.txt"));
        assert!(info["permissions"]["readable"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_get_file_info_directory() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let dir_path = temp_dir.path().join("test_dir");

        // Create test directory
        fs::create_dir(&dir_path).await.unwrap();

        let result = service.get_file_info(&dir_path).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let info: serde_json::Value = serde_json::from_str(&response.message).unwrap();

        assert_eq!(info["name"], "test_dir");
        assert_eq!(info["type"], "[DIRECTORY]");
        assert_eq!(info["is_directory"], true);
        assert!(info["path"].as_str().unwrap().ends_with("test_dir"));
        assert!(info["permissions"]["readable"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_get_file_info_nonexistent() {
        let service = FileService::new();
        let nonexistent_path = Path::new("/nonexistent/file.txt");

        let result = service.get_file_info(nonexistent_path).await;
        assert!(result.is_err());

        if let Err(FileSystemMcpError::PathNotFound { path }) = result {
            assert_eq!(path, nonexistent_path.display().to_string());
        } else {
            panic!("Expected PathNotFound error for nonexistent file");
        }
    }

    #[tokio::test]
    async fn test_get_file_info_empty_file() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("empty_file.txt");

        // Create empty file
        fs::write(&file_path, "").await.unwrap();

        let result = service.get_file_info(&file_path).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let info: serde_json::Value = serde_json::from_str(&response.message).unwrap();

        assert_eq!(info["name"], "empty_file.txt");
        assert_eq!(info["type"], "[FILE]");
        assert_eq!(info["size"], 0);
        assert_eq!(info["is_directory"], false);
    }

    #[tokio::test]
    async fn test_get_file_info_large_file() {
        let service = FileService::new();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("large_file.txt");

        // Create file with known size
        let content = "a".repeat(1024); // 1KB file
        fs::write(&file_path, &content).await.unwrap();

        let result = service.get_file_info(&file_path).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let info: serde_json::Value = serde_json::from_str(&response.message).unwrap();

        assert_eq!(info["name"], "large_file.txt");
        assert_eq!(info["type"], "[FILE]");
        assert_eq!(info["size"], 1024);
        assert_eq!(info["is_directory"], false);
    }
}
