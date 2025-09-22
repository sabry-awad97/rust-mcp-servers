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
use serde_json::json;

#[derive(Clone)]
pub struct FetchService {
    tool_router: ToolRouter<FetchService>,
    prompt_router: PromptRouter<FetchService>,
    custom_user_agent: Option<String>,
    ignore_robots_txt: bool,
    proxy_url: Option<String>,
}

impl FetchService {
    pub fn new(
        custom_user_agent: Option<String>,
        ignore_robots_txt: bool,
        proxy_url: Option<String>,
    ) -> Self {
        Self {
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
            custom_user_agent,
            ignore_robots_txt,
            proxy_url,
        }
    }
}

impl Default for FetchService {
    fn default() -> Self {
        Self::new(None, false, None)
    }
}

#[tool_router]
impl FetchService {}

#[prompt_router]
impl FetchService {}

#[tool_handler]
#[prompt_handler]
impl ServerHandler for FetchService {
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
