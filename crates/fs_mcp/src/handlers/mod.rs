pub mod fs_tools;

pub use fs_tools::FileSystemService;

use crate::config::Config;

pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::{ServiceExt, transport::stdio};

    let service = FileSystemService::new(config.allowed_directories)
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })?;

    service.waiting().await?;
    Ok(())
}
