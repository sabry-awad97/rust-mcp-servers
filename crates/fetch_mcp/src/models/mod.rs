use derive_getters::Getters;
use rmcp::schemars;
use serde::Deserialize;

use crate::{errors::FetchServerError, services::Validate};

fn default_max_length() -> usize {
    5000
}

/// Parameters for fetching a URL
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct FetchRequest {
    /// URL to fetch
    url: String,
    #[serde(default = "default_max_length")]
    /// Maximum number of characters to return
    max_length: usize,
    #[serde(default)]
    /// On return output starting at this character index, useful if a previous fetch was truncated and more context is required
    start_index: usize,
    /// Get the actual HTML content of the requested page, without simplification.
    #[serde(default)]
    raw: bool,
}

impl FetchRequest {
    #[cfg(test)]
    pub const INVALID: Self = Self {
        url: String::new(),
        max_length: 0,
        start_index: 0,
        raw: false,
    };
}

impl Validate for FetchRequest {
    fn validate(&self) -> Result<(), FetchServerError> {
        if self.url.is_empty() {
            return Err(FetchServerError::InvalidParams {
                message: "URL is required".to_string(),
            });
        }

        if self.max_length == 0 || self.max_length > 1_000_000 {
            return Err(FetchServerError::InvalidParams {
                message: "max_length must be between 1 and 1,000,000".to_string(),
            });
        }

        Ok(())
    }
}

/// Arguments for fetch prompt
#[derive(Debug, Deserialize, schemars::JsonSchema, Getters)]
pub struct FetchPromptArgs {
    /// URL to fetch
    url: String,
}

impl Validate for FetchPromptArgs {
    fn validate(&self) -> Result<(), FetchServerError> {
        if self.url.is_empty() {
            return Err(FetchServerError::InvalidParams {
                message: "URL is required".to_string(),
            });
        }

        Ok(())
    }
}
