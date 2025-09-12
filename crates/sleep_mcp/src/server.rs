use rmcp::{
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::*,
    prompt, prompt_handler, prompt_router,
    service::RequestContext,
    tool, tool_handler, tool_router, RoleServer, ServerHandler,
};

use crate::core::provider::SleepServer;
use crate::core::{
    error::McpResult,
    models::{GetStatusRequest, SleepRequest, SleepUntilRequest},
};

/// Available resource URIs for the Sleep MCP Server
pub const AVAILABLE_RESOURCES: &[&str] = &["sleep://status", "sleep://help", "sleep://examples"];

/// Sleep MCP Server with sleep and delay operations
#[derive(Clone)]
pub struct SleepService {
    sleep_server: SleepServer,
    tool_router: ToolRouter<SleepService>,
    prompt_router: PromptRouter<SleepService>,
}

impl SleepService {
    pub fn new() -> Self {
        let sleep_server = SleepServer::new();

        Self {
            sleep_server,
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    fn create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    fn generate_status_content(&self) -> McpResult<String> {
        let status = self.sleep_server.get_status(true);

        Ok(format!(
            r#"Sleep MCP Server Status

Server: Running
Current Status: {}
Max Sleep Duration: 30 minutes
Tools Available: 4
Resources Available: 3

Current Operation:
{}

Capabilities:
- Sleep for specific durations (ms, s, m, h)
- Sleep until specific times (ISO 8601)
- Real-time status tracking with progress
- Automatic cancellation on server restart
- Duration validation and safety limits"#,
            if status.is_sleeping {
                "Sleeping"
            } else {
                "Idle"
            },
            if status.is_sleeping {
                format!(
                    "- Duration: {}ms
- Progress: {:.1}%
- Remaining: {}ms
- Start Time: {}",
                    status.current_duration_ms.unwrap_or(0),
                    status.progress_percent.unwrap_or(0.0),
                    status.remaining_ms.unwrap_or(0),
                    status.start_time.as_deref().unwrap_or("Unknown")
                )
            } else {
                "- No active sleep operation".to_string()
            }
        ))
    }

    fn generate_help_content(&self) -> String {
        r#"Sleep MCP Server Help

TOOLS:
- sleep: Sleep for a specific duration
  - duration: Duration string (required) - e.g., "1s", "500ms", "2m", "1h"
  - message: Optional message to include in result
  - Example: {"duration": "5s", "message": "Taking a short break"}

- sleep_until: Sleep until a specific time
  - target_time: ISO 8601 timestamp (required) - e.g., "2025-01-15T14:30:00Z"
  - message: Optional message to include in result
  - Example: {"target_time": "2025-01-15T14:30:00Z", "message": "Waiting for meeting"}

- get_sleep_status: Get current sleep operation status
  - detailed: Include detailed timing information (optional, default: false)
  - Example: {"detailed": true}

- cancel_sleep: Cancel the current sleep operation
  - No parameters required
  - Example: {}

RESOURCES:
- sleep://status: Current server status and active operations
- sleep://help: This help documentation
- sleep://examples: Usage examples and best practices

DURATION FORMATS:
- Milliseconds: "500ms", "1000ms"
- Seconds: "1s", "5s", "1.5s", "30s"
- Minutes: "1m", "5m", "2.5m", "30m"
- Hours: Not supported (max duration is 30 minutes)

TIME FORMATS:
- ISO 8601: "2025-01-15T14:30:00Z"
- With timezone: "2025-01-15T14:30:00+02:00"
- Millisecond precision: "2025-01-15T14:30:00.123Z"

SAFETY LIMITS:
- Maximum sleep duration: 30 minutes
- Minimum sleep duration: 1 millisecond
- Target times must be in the future
- All operations are automatically cancelled on server restart

EXAMPLE USAGE:

Short Sleep:
```json
{
  "duration": "2s",
  "message": "Brief pause"
}
```

Sleep Until Time:
```json
{
  "target_time": "2025-01-15T15:00:00Z",
  "message": "Wait for hourly sync"
}
```

Check Status:
```json
{
  "detailed": true
}
```

ERROR HANDLING:
- Invalid durations return format examples
- Durations too long show maximum allowed
- Past target times are rejected
- Progress tracking handles edge cases"#
            .to_string()
    }

    fn generate_examples_content(&self) -> &'static str {
        r#"Sleep MCP Server Examples

BASIC OPERATIONS:

1. Quick Sleep (2 seconds):
   Tool: sleep
   Parameters: {"duration": "2s"}

2. Precise Timing (500ms):
   Tool: sleep
   Parameters: {"duration": "500ms", "message": "Precise timing"}

3. Longer Sleep (5 minutes):
   Tool: sleep
   Parameters: {"duration": "5m", "message": "Coffee break"}

4. Sleep Until Specific Time:
   Tool: sleep_until
   Parameters: {"target_time": "2025-01-15T14:30:00Z"}

5. Check Current Status:
   Tool: get_sleep_status
   Parameters: {"detailed": true}

COMMON USE CASES:

Rate Limiting:
- Sleep between API calls: {"duration": "1s"}
- Batch processing delays: {"duration": "100ms"}

Scheduled Operations:
- Wait for specific time: {"target_time": "2025-01-15T09:00:00Z"}
- Hourly synchronization: {"target_time": "2025-01-15T15:00:00Z"}

Testing and Debugging:
- Simulate slow operations: {"duration": "3s"}
- Add delays for observation: {"duration": "500ms"}

Workflow Coordination:
- Pause between steps: {"duration": "2s", "message": "Step 1 complete"}
- Wait for external systems: {"duration": "10s", "message": "Waiting for database"}

DURATION EXAMPLES:

Milliseconds:
- "1ms" - Minimum sleep
- "100ms" - Short delay
- "500ms" - Half second
- "1000ms" - One second

Seconds:
- "0.1s" - 100 milliseconds
- "1s" - One second
- "1.5s" - 1.5 seconds
- "30s" - Thirty seconds

Minutes:
- "1m" - One minute
- "2.5m" - 2.5 minutes
- "15m" - Fifteen minutes
- "30m" - Maximum allowed (30 minutes)

TIMESTAMP EXAMPLES:

UTC Times:
- "2025-01-15T14:30:00Z"
- "2025-12-31T23:59:59Z"
- "2025-06-15T12:00:00.000Z"

With Timezone:
- "2025-01-15T14:30:00+02:00"
- "2025-01-15T14:30:00-05:00"
- "2025-01-15T14:30:00+09:00"

ERROR EXAMPLES:

Invalid Duration:
- "invalid" → Shows valid formats
- "1x" → Unknown unit
- "-1s" → Negative duration

Duration Too Long:
- "1h" → Exceeds 30-minute limit
- "45m" → Exceeds maximum
- "2000s" → Over 30 minutes

Invalid Timestamp:
- "2020-01-01T00:00:00Z" → Past time
- "invalid-time" → Format error
- "2025-13-01T00:00:00Z" → Invalid date

MONITORING PROGRESS:

While a sleep operation is active, you can check progress:

Status Response:
```json
{
  "is_sleeping": true,
  "current_duration_ms": 5000,
  "start_time": "2025-01-15T14:30:00Z",
  "expected_end_time": "2025-01-15T14:30:05Z",
  "progress_percent": 60.0,
  "remaining_ms": 2000
}
```

This shows:
- 5-second sleep in progress
- 60% complete
- 2 seconds remaining
- Started at 14:30:00, will end at 14:30:05"#
    }
}

impl Default for SleepService {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl SleepService {
    #[tool(description = "Sleep for a specific duration")]
    async fn sleep(&self, Parameters(req): Parameters<SleepRequest>) -> McpResult<CallToolResult> {
        let result = self
            .sleep_server
            .sleep_for(&req.duration, req.message)
            .await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }

    #[tool(description = "Sleep until a specific time")]
    async fn sleep_until(
        &self,
        Parameters(req): Parameters<SleepUntilRequest>,
    ) -> McpResult<CallToolResult> {
        let result = self
            .sleep_server
            .sleep_until(&req.target_time, req.message)
            .await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }

    #[tool(description = "Get current sleep operation status")]
    async fn get_sleep_status(
        &self,
        Parameters(req): Parameters<GetStatusRequest>,
    ) -> McpResult<CallToolResult> {
        let status = self.sleep_server.get_status(req.detailed);
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&status).unwrap(),
        )]))
    }

    #[tool(description = "Cancel the current sleep operation")]
    async fn cancel_sleep(&self) -> McpResult<CallToolResult> {
        let result = self.sleep_server.cancel_sleep().await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }
}

#[prompt_router]
impl SleepService {
    /// Generate guidance for effective sleep operation usage
    #[prompt(name = "sleep_guidance")]
    async fn sleep_guidance(
        &self,
        _ctx: RequestContext<rmcp::RoleServer>,
    ) -> McpResult<Vec<PromptMessage>> {
        let guidance = r#"Sleep Operation Best Practices:

1. **Duration Formats**
   - Use clear units: "1s", "500ms", "2m"
   - Decimal precision: "1.5s", "2.5m"
   - Maximum duration: 30 minutes

2. **Time Formats**
   - Use ISO 8601: "2025-01-15T14:30:00Z"
   - Include timezone if needed: "2025-01-15T14:30:00+02:00"
   - Target times must be in the future

3. **Common Use Cases**
   - Rate limiting: Short sleeps (100ms-1s)
   - Batch processing: Medium sleeps (1s-1m)
   - Scheduled operations: Sleep until specific times

4. **Monitoring**
   - Use get_sleep_status to track progress
   - Check remaining time and progress percentage
   - Operations auto-cancel on server restart

5. **Error Prevention**
   - Validate durations before use
   - Check target times are in future
   - Stay within 30-minute limit
   - Use appropriate precision for your use case

6. **Performance Tips**
   - Prefer shorter sleeps for responsive operations
   - Use sleep_until for precise timing
   - Monitor progress for long operations
   - Include descriptive messages for debugging"#;

        Ok(vec![PromptMessage {
            role: PromptMessageRole::Assistant,
            content: PromptMessageContent::text(guidance),
        }])
    }
}

#[tool_handler]
#[prompt_handler]
impl ServerHandler for SleepService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Sleep MCP Server for delay and timing operations. Tools: sleep, sleep_until, get_sleep_status. Maximum duration: 30 minutes.".to_string()
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<rmcp::RoleServer>,
    ) -> McpResult<ListResourcesResult> {
        Ok(ListResourcesResult {
            resources: vec![
                self.create_resource_text("sleep://status", "server-status"),
                self.create_resource_text("sleep://help", "help-documentation"),
                self.create_resource_text("sleep://examples", "usage-examples"),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<rmcp::RoleServer>,
    ) -> McpResult<ReadResourceResult> {
        match uri.as_str() {
            "sleep://status" => {
                let status = self.generate_status_content()?;
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(status, uri)],
                })
            }
            "sleep://help" => {
                let help = self.generate_help_content();
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(help, uri)],
                })
            }
            "sleep://examples" => {
                let examples = self.generate_examples_content();
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(examples, uri)],
                })
            }
            _ => Err(crate::core::error::SleepServerError::ResourceNotFound {
                uri: uri.to_string(),
            }
            .into()),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<rmcp::RoleServer>,
    ) -> McpResult<ListResourceTemplatesResult> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> McpResult<InitializeResult> {
        tracing::info!("Sleep MCP Server initialized successfully");
        Ok(self.get_info())
    }
}

/// Run the Sleep MCP server
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::{transport::stdio, ServiceExt};

    let service = SleepService::new().serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = SleepService::new();
        let info = service.get_info();
        assert!(info.instructions.is_some());
        assert!(info.instructions.unwrap().contains("Sleep MCP Server"));
    }

    #[test]
    fn test_available_resources() {
        assert_eq!(AVAILABLE_RESOURCES.len(), 3);
        assert!(AVAILABLE_RESOURCES.contains(&"sleep://status"));
        assert!(AVAILABLE_RESOURCES.contains(&"sleep://help"));
        assert!(AVAILABLE_RESOURCES.contains(&"sleep://examples"));
    }

    #[tokio::test]
    async fn test_sleep_tool() {
        let service = SleepService::new();
        let req = SleepRequest {
            duration: "100ms".to_string(),
            message: Some("Test".to_string()),
        };

        let result = service.sleep(Parameters(req)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_status_tool() {
        let service = SleepService::new();
        let req = GetStatusRequest { detailed: true };

        let result = service.get_sleep_status(Parameters(req)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resource_generation() {
        let service = SleepService::new();

        let status = service.generate_status_content();
        assert!(status.is_ok());
        assert!(status.unwrap().contains("Sleep MCP Server Status"));

        let help = service.generate_help_content();
        assert!(help.contains("Sleep MCP Server Help"));

        let examples = service.generate_examples_content();
        assert!(examples.contains("Sleep MCP Server Examples"));
    }
}
