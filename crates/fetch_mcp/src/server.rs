use crate::services::FetchService;
use rmcp::{ServiceExt, transport::stdio};

pub async fn run(
    user_agent: Option<String>,
    ignore_robots_txt: bool,
    proxy_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create the fetch service with configuration
    let service = FetchService::new(user_agent, ignore_robots_txt, proxy_url);

    // Create an instance of our Fetch service and serve it
    let service = service.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;

    Ok(())
}
