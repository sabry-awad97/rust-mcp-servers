use base64::{Engine, engine::general_purpose};
use rmcp::model::Content;
use std::{fmt, path::Path};

/// File content types for different file formats
#[derive(Debug, Clone, PartialEq)]
pub enum FileContent {
    /// Plain text content
    Text(String),
    /// Base64 encoded binary content
    Binary(String),
}

/// Response for read_text_file, read_media_file tools
#[derive(Debug)]
pub struct ReadFileResponse {
    /// Content of the file as either text or base64-encoded binary
    pub content: FileContent,
    /// MIME type of the file
    pub mime_type: String,
}

impl ReadFileResponse {
    /// Create a new ReadFileResponse from raw bytes, automatically determining content type
    pub fn new(bytes: Vec<u8>, path: &Path) -> Self {
        let mime_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        let content = if mime_type.starts_with("text/") {
            // For text files, convert bytes to UTF-8 string
            FileContent::Text(String::from_utf8_lossy(&bytes).to_string())
        } else {
            // For binary files, encode as base64
            let base64_content = general_purpose::STANDARD.encode(&bytes);
            FileContent::Binary(base64_content)
        };

        Self { content, mime_type }
    }

    /// Create a text file response
    pub fn text(content: String) -> Self {
        Self {
            content: FileContent::Text(content),
            mime_type: "text/plain".to_string(),
        }
    }

    /// Create a binary file response
    pub fn binary(base64_content: String, mime_type: String) -> Self {
        Self {
            content: FileContent::Binary(base64_content),
            mime_type,
        }
    }
}

impl fmt::Display for ReadFileResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.content {
            FileContent::Text(text) => write!(f, "Text file ({}): {}", self.mime_type, text),
            FileContent::Binary(base64) => {
                write!(f, "Binary file ({}): {}", self.mime_type, base64)
            }
        }
    }
}

impl From<ReadFileResponse> for Content {
    fn from(value: ReadFileResponse) -> Self {
        match value.content {
            FileContent::Text(text) => Content::text(text),
            FileContent::Binary(base64_data) => {
                if value.mime_type.starts_with("image/") {
                    Content::image(base64_data, value.mime_type)
                } else {
                    Content::text(format!(
                        "Binary file ({}): {}",
                        value.mime_type, base64_data
                    ))
                }
            }
        }
    }
}
