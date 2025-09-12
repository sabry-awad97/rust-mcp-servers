use rmcp::{
    RoleServer, ServerHandler,
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::*,
    prompt, prompt_handler, prompt_router,
    service::RequestContext,
    tool, tool_handler, tool_router,
};

use crate::core::provider::TimeServer;
use crate::core::{
    error::McpResult,
    models::{ConvertTimeRequest, GetCurrentTimeRequest},
};

/// Time MCP Server with timezone operations
#[derive(Clone)]
pub struct TimeService {
    time_server: TimeServer,
    local_timezone_name: String, // Cache this
    tool_router: ToolRouter<TimeService>,
    prompt_router: PromptRouter<TimeService>,
}

impl TimeService {
    pub fn new() -> Self {
        let time_server = TimeServer::new();
        let local_timezone_name = time_server.local_timezone.to_string();

        Self {
            time_server,
            local_timezone_name,
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    fn create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    pub(crate) fn get_local_timezone_name(&self) -> &str {
        &self.local_timezone_name
    }

    fn generate_status_content(&self) -> McpResult<String> {
        let current_time = self
            .time_server
            .get_current_time(&self.local_timezone_name)?;

        Ok(format!(
            r#"Time MCP Server Status

Server: Running
Local Timezone: {}
Current Local Time: {}
Day of Week: {}
DST Active: {}
Tools Available: 2
Prompts Available: 1
Resources Available: 3

Capabilities:
- Current time queries for any IANA timezone
- Time conversion between timezones
- Automatic DST handling
- Local timezone detection"#,
            current_time.timezone,
            current_time.datetime,
            current_time.day_of_week,
            current_time.is_dst
        ))
    }

    fn generate_help_content(&self) -> String {
        format!(
            r#"Time MCP Server Help

TOOLS:
- get_current_time: Get current time in a specific timezone
  - timezone: IANA timezone name (required)
  - Example: {{"timezone": "America/New_York"}}

- convert_time: Convert time between timezones
  - source_timezone: Source IANA timezone name (required)
  - time: Time in 24-hour format HH:MM (required)
  - target_timezone: Target IANA timezone name (required)
  - Example: {{"source_timezone": "America/New_York", "time": "14:30", "target_timezone": "Europe/London"}}

PROMPTS:
- timezone_guidance: Get best practices for timezone usage

RESOURCES:
- time://status: Current server status and local time
- time://help: This help documentation
- time://timezones: List of common IANA timezone names

LOCAL TIMEZONE: {}

EXAMPLE USAGE:

Get Current Time:
```json
{{
  "timezone": "Asia/Tokyo"
}}
```

Convert Time:
```json
{{
  "source_timezone": "America/Los_Angeles",
  "time": "09:00",
  "target_timezone": "Europe/Paris"
}}
```

TIMEZONE FORMAT:
- Use full IANA names: 'America/New_York', 'Europe/London'
- Avoid abbreviations: 'EST', 'PST' (ambiguous)
- Time format: 24-hour HH:MM (e.g., '09:30', '14:45')

DST HANDLING:
- Automatically detects and handles daylight saving time
- Returns 'is_dst' field indicating DST status
- Time differences account for DST offsets"#,
            self.local_timezone_name
        )
    }

    fn generate_timezone_list_content(&self) -> &'static str {
        r#"Common IANA Timezone Names

AMERICAS:
- America/New_York (Eastern Time)
- America/Chicago (Central Time)
- America/Denver (Mountain Time)
- America/Los_Angeles (Pacific Time)
- America/Toronto (Eastern Time - Canada)
- America/Vancouver (Pacific Time - Canada)
- America/Mexico_City (Central Time - Mexico)
- America/Sao_Paulo (Brazil Time)
- America/Argentina/Buenos_Aires (Argentina Time)

EUROPE:
- Europe/London (Greenwich Mean Time)
- Europe/Paris (Central European Time)
- Europe/Berlin (Central European Time)
- Europe/Rome (Central European Time)
- Europe/Madrid (Central European Time)
- Europe/Amsterdam (Central European Time)
- Europe/Stockholm (Central European Time)
- Europe/Moscow (Moscow Time)

ASIA:
- Asia/Tokyo (Japan Standard Time)
- Asia/Shanghai (China Standard Time)
- Asia/Hong_Kong (Hong Kong Time)
- Asia/Singapore (Singapore Time)
- Asia/Seoul (Korea Standard Time)
- Asia/Kolkata (India Standard Time)
- Asia/Dubai (Gulf Standard Time)
- Asia/Bangkok (Indochina Time)

OCEANIA:
- Australia/Sydney (Australian Eastern Time)
- Australia/Melbourne (Australian Eastern Time)
- Australia/Perth (Australian Western Time)
- Pacific/Auckland (New Zealand Time)

AFRICA:
- Africa/Cairo (Eastern European Time)
- Africa/Johannesburg (South Africa Time)
- Africa/Lagos (West Africa Time)

SPECIAL:
- UTC (Coordinated Universal Time)
- GMT (Greenwich Mean Time - same as UTC)

Note: Many timezones observe Daylight Saving Time (DST) and will automatically
adjust their offsets during DST periods."#
    }
}

impl Default for TimeService {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl TimeService {
    #[tool(description = "Get current time in a specific timezone")]
    pub(crate) async fn get_current_time(
        &self,
        Parameters(req): Parameters<GetCurrentTimeRequest>,
    ) -> McpResult<CallToolResult> {
        let result = self.time_server.get_current_time(&req.timezone)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }

    #[tool(description = "Convert time between timezones")]
    pub(crate) async fn convert_time(
        &self,
        Parameters(req): Parameters<ConvertTimeRequest>,
    ) -> McpResult<CallToolResult> {
        let result =
            self.time_server
                .convert_time(&req.source_timezone, &req.time, &req.target_timezone)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }
}

#[prompt_router]
impl TimeService {
    /// Generate guidance for effective timezone usage
    #[prompt(name = "timezone_guidance")]
    async fn timezone_guidance(
        &self,
        _ctx: RequestContext<rmcp::RoleServer>,
    ) -> McpResult<Vec<PromptMessage>> {
        let local_tz = self.get_local_timezone_name();
        let guidance = format!(
            r#"Timezone Best Practices:

1. **IANA Timezone Names**
   - Use full IANA timezone names (e.g., 'America/New_York', 'Europe/London')
   - Avoid abbreviations like 'EST' or 'PST' as they can be ambiguous
   - Your local timezone is detected as: {}

2. **Time Format**
   - Use 24-hour format (HH:MM) for time conversion
   - Examples: '09:30', '14:45', '23:15'
   - Leading zeros are required for single-digit hours

3. **Daylight Saving Time**
   - DST transitions are automatically handled
   - The 'is_dst' field indicates if DST is currently active
   - Time differences account for DST offsets

4. **Common Timezones**
   - UTC: 'UTC'
   - New York: 'America/New_York'
   - Los Angeles: 'America/Los_Angeles'
   - London: 'Europe/London'
   - Tokyo: 'Asia/Tokyo'
   - Sydney: 'Australia/Sydney'
   - Cairo: 'Africa/Cairo'

5. **Error Handling**
   - Invalid timezone names will return an error
   - Invalid time formats will be rejected
   - Ambiguous times during DST transitions are handled"#,
            local_tz
        );

        Ok(vec![PromptMessage {
            role: PromptMessageRole::Assistant,
            content: PromptMessageContent::text(guidance),
        }])
    }
}

#[tool_handler]
#[prompt_handler]
impl ServerHandler for TimeService {
    fn get_info(&self) -> ServerInfo {
        let local_tz = self.get_local_timezone_name();
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(format!(
                "Time MCP Server for timezone operations. Tools: get_current_time, convert_time. Local timezone: {}. Use IANA timezone names.",
                local_tz
            )),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<rmcp::RoleServer>,
    ) -> McpResult<ListResourcesResult> {
        Ok(ListResourcesResult {
            resources: vec![
                self.create_resource_text("time://status", "server-status"),
                self.create_resource_text("time://help", "help-documentation"),
                self.create_resource_text("time://timezones", "timezone-list"),
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
            "time://status" => {
                let status = self.generate_status_content()?;
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(status, uri)],
                })
            }
            "time://help" => {
                let help = self.generate_help_content();
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(help, uri)],
                })
            }
            "time://timezones" => {
                let common_timezones = self.generate_timezone_list_content();
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(common_timezones, uri)],
                })
            }
            _ => Err(crate::core::error::TimeServerError::ResourceNotFound {
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
        tracing::info!("Time MCP Server initialized successfully");
        Ok(self.get_info())
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::{ServiceExt, transport::stdio};

    let service = TimeService::new().serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rmcp::handler::server::wrapper::Parameters;
    use rmcp::model::ProtocolVersion;

    use crate::core::models::{ConvertTimeRequest, GetCurrentTimeRequest};
    use crate::core::provider::TimeServer;
    use crate::server::TimeService;

    #[tokio::test]
    async fn test_get_current_time() {
        let service = TimeService::new();

        println!(
            "Detected local timezone: {}",
            service.get_local_timezone_name()
        );

        let req = GetCurrentTimeRequest {
            timezone: "UTC".to_string(),
        };

        let result = service.get_current_time(Parameters(req)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_current_time_invalid_timezone() {
        let service = TimeService::new();

        let req = GetCurrentTimeRequest {
            timezone: "Invalid/Timezone".to_string(),
        };

        let result = service.get_current_time(Parameters(req)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_convert_time() {
        let service = TimeService::new();

        let req = ConvertTimeRequest {
            source_timezone: "UTC".to_string(),
            time: "12:00".to_string(),
            target_timezone: "America/New_York".to_string(),
        };

        let result = service.convert_time(Parameters(req)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_convert_time_invalid_format() {
        let service = TimeService::new();

        let req = ConvertTimeRequest {
            source_timezone: "UTC".to_string(),
            time: "25:00".to_string(), // Invalid hour
            target_timezone: "America/New_York".to_string(),
        };

        let result = service.convert_time(Parameters(req)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_convert_time_invalid_timezone() {
        let service = TimeService::new();

        let req = ConvertTimeRequest {
            source_timezone: "Invalid/Timezone".to_string(),
            time: "12:00".to_string(),
            target_timezone: "UTC".to_string(),
        };

        let result = service.convert_time(Parameters(req)).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_time_server_creation() {
        let server = TimeServer::new();
        // Should not panic and should have a valid local timezone
        assert!(!server.local_timezone.to_string().is_empty());
    }

    #[test]
    fn test_service_creation() {
        use rmcp::Service;

        let service = TimeService::new();
        let info = service.get_info();

        assert_eq!(info.protocol_version, ProtocolVersion::V_2024_11_05);
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.prompts.is_some());
        assert!(info.capabilities.resources.is_some());
        assert!(info.instructions.is_some());
    }

    #[test]
    fn test_timezone_parsing() {
        let server = TimeServer::new();

        // Valid timezone
        let result = server.parse_timezone("UTC");
        assert!(result.is_ok());

        // Invalid timezone
        let result = server.parse_timezone("Invalid/Timezone");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dst_handling() {
        let service = TimeService::new();

        // Test with a timezone that observes DST
        let req = GetCurrentTimeRequest {
            timezone: "America/New_York".to_string(),
        };

        let result = service.get_current_time(Parameters(req)).await;
        assert!(result.is_ok());

        if let Ok(call_result) = result {
            // Should have content with timezone info
            assert!(!call_result.content.is_empty());
        }
    }

    #[test]
    fn test_cached_timezone_name() {
        let service = TimeService::new();
        let name1 = service.get_local_timezone_name();
        let name2 = service.get_local_timezone_name();

        // Should return the same reference (cached)
        assert_eq!(name1, name2);
        assert!(!name1.is_empty());
    }
}
