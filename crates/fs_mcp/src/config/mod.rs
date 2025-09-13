use std::path::PathBuf;

/// Configuration derived from CLI arguments
#[derive(Debug, Clone)]
pub struct Config {
    pub allowed_directories: Vec<PathBuf>,
}
