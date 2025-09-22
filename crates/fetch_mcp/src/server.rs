use crate::services::{FetchService, Validate};
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
    service: FetchService,
}

impl FetchServer {
    pub fn new(service: FetchService) -> Self {
        Self {
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
            service,
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
        Parameters(req): Parameters<FetchRequest>,
    ) -> Result<CallToolResult, McpError> {
        req.validate()?;
        // Check robots.txt for autonomous fetching
        self.service
            .check_may_autonomously_fetch_url(req.url())
            .await
            .map_err(|e| -> McpError { e.into() })?;

        let (content, prefix) = self
            .service
            .fetch_url(
                req.url(),
                self.service.get_user_agent_autonomous(),
                req.raw().to_owned(),
            )
            .await?;

        let original_length = content.len();
        let final_content = if *req.start_index() >= original_length {
            "<error>No more content available.</error>".to_string()
        } else {
            let end_index = std::cmp::min(*req.start_index() + *req.max_length(), original_length);
            let truncated_content = &content[*req.start_index()..end_index];

            if truncated_content.is_empty() {
                "<error>No more content available.</error>".to_string()
            } else {
                let mut result = truncated_content.to_string();
                let actual_content_length = truncated_content.len();
                let remaining_content =
                    original_length - (req.start_index() + actual_content_length);

                // Add continuation prompt if content was truncated
                if actual_content_length == *req.max_length() && remaining_content > 0 {
                    let next_start = req.start_index() + actual_content_length;
                    result.push_str(&format!("\n\n<error>Content truncated. Call the fetch tool with a start_index of {} to get more content.</error>", next_start));
                }
                result
            }
        };
        let response_text = format!("{}Contents of {}:\n{}", prefix, req.url(), final_content);

        Ok(CallToolResult::success(vec![Content::text(response_text)]))
    }
}

#[prompt_router]
impl FetchServer {
    /// Fetch a URL and extract its contents as markdown
    #[prompt(name = "fetch")]
    async fn fetch_prompt(
        &self,
        Parameters(args): Parameters<FetchPromptArgs>,
        _ctx: RequestContext<rmcp::RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        args.validate()?;
        match self
            .service
            .fetch_url(args.url(), self.service.get_user_agent_manual(), false)
            .await
            .map_err(|e| -> McpError { e.into() })
        {
            Ok((content, prefix)) => {
                let full_content = format!("{}{}", prefix, content);
                Ok(GetPromptResult {
                    description: Some(format!("Contents of {}", args.url())),
                    messages: vec![PromptMessage {
                        role: PromptMessageRole::User,
                        content: PromptMessageContent::text(full_content),
                    }],
                })
            }
            Err(e) => Ok(GetPromptResult {
                description: Some(format!("Failed to fetch {}", args.url())),
                messages: vec![PromptMessage {
                    role: PromptMessageRole::User,
                    content: PromptMessageContent::text(format!("Error: {}", e)),
                }],
            }),
        }
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
    let service = FetchService::new(user_agent, ignore_robots_txt, proxy_url);
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

        // Test invalid
        let result = server.fetch(Parameters(FetchRequest::INVALID)).await;
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
