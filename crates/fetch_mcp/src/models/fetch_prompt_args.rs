use rmcp::schemars;
use serde::{Deserialize, Serialize};

/// Arguments for fetch prompt
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FetchPromptArgs {
    /// URL to fetch
    pub url: String,
}
