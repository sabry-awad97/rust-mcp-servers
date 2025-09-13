use derive_getters::Getters;
use rmcp::schemars;
use serde::Deserialize;

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
