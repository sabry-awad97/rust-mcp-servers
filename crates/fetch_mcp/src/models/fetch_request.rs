use rmcp::schemars;
use serde::Deserialize;

fn default_max_length() -> usize {
    5000
}

/// Parameters for fetching a URL
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FetchRequest {
    /// URL to fetch
    pub url: String,
    #[serde(default = "default_max_length")]
    /// Maximum number of characters to return
    pub max_length: usize,
    #[serde(default)]
    /// On return output starting at this character index, useful if a previous fetch was truncated and more context is required
    pub start_index: usize,
    /// Get the actual HTML content of the requested page, without simplification.
    #[serde(default)]
    pub raw: bool,
}
