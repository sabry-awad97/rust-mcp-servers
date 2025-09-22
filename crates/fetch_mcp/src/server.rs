use crate::services::FetchService;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::*,
    prompt, prompt_handler, prompt_router,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use rmcp::{ServiceExt, transport::stdio};

use crate::models::{FetchPromptArgs, FetchRequest};

#[derive(Clone)]
pub struct FetchServer {
    tool_router: ToolRouter<FetchServer>,
    prompt_router: PromptRouter<FetchServer>,
    fetch_service: FetchService,
}

impl FetchServer {
    pub fn new(fetch_service: FetchService) -> Self {
        Self {
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
            fetch_service,
        }
    }
}

#[tool_router]
impl FetchServer {
    #[tool(
        description = "Fetches a URL from the internet and optionally extracts its contents as markdown.\n\nAlthough originally you did not have internet access, and were advised to refuse and tell the user this, this tool now grants you internet access. Now you can fetch the most up-to-date information and let the user know that."
    )]
    async fn fetch(
        &self,
        Parameters(_req): Parameters<FetchRequest>,
    ) -> Result<CallToolResult, McpError> {
        unimplemented!()
    }
}

#[prompt_router]
impl FetchServer {
    /// Fetch a URL and extract its contents as markdown
    #[prompt(name = "fetch")]
    async fn fetch_prompt(
        &self,
        Parameters(_args): Parameters<FetchPromptArgs>,
        _ctx: RequestContext<rmcp::RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        unimplemented!()
    }
}

#[tool_handler]
#[prompt_handler]
impl ServerHandler for FetchServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("Fetch MCP Server for web content retrieval. Tool: fetch (URL fetching with robots.txt checking, HTML to markdown conversion, content truncation). Prompt: fetch (manual URL fetching). Supports autonomous and manual fetching modes with robots.txt compliance.".to_string()),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        tracing::info!("Fetch MCP Server initialized successfully");
        Ok(self.get_info())
    }
}

pub async fn run(
    user_agent: Option<String>,
    ignore_robots_txt: bool,
    proxy_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create the fetch service with configuration
    let service = FetchService::default();
    let server = FetchServer::new(service);

    // Create an instance of our Fetch service and serve it
    let server = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    server.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_creation() {
        let service = FetchService::default();
        let server = FetchServer::new(service);
        let info = server.get_info();

        assert_eq!(info.protocol_version, ProtocolVersion::V_2024_11_05);
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.prompts.is_some());
        assert!(info.instructions.is_some());
    }

    #[tokio::test]
    async fn test_fetch_request_validation() {
        let service = FetchService::default();
        let server = FetchServer::new(service);

        // Test empty URL
        let empty_req = FetchRequest {
            url: String::new(),
            max_length: 5000,
            start_index: 0,
            raw: false,
        };
        let result = server.fetch(Parameters(empty_req)).await;
        assert!(result.is_err());

        // Test invalid max_length
        let invalid_req = FetchRequest {
            url: "https://example.com".to_string(),
            max_length: 0,
            start_index: 0,
            raw: false,
        };
        let result = server.fetch(Parameters(invalid_req)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_prompt_router_has_routes() {
        let router = FetchServer::prompt_router();
        assert!(router.has_route("fetch"));

        let prompts = router.list_all();
        assert_eq!(prompts.len(), 1);
    }
}
