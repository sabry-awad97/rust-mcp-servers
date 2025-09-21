use rmcp::schemars;
use serde::Deserialize;

fn default_max_length() -> usize {
    5000
}

/// Parameters for fetching a URL
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FetchRequest {
    pub url: String,
    #[serde(default = "default_max_length")]
    pub max_length: usize,
    #[serde(default)]
    pub start_index: usize,
    #[serde(default)]
    pub raw: bool,
}
