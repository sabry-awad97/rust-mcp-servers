use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::*,
    prompt, prompt_handler, prompt_router, schemars,
    service::RequestContext,
    tool, tool_handler, tool_router,
};

use crate::core::provider::TimeServer;
use crate::core::{
    error::McpResult,
    models::{ConvertTimeRequest, GetCurrentTimeRequest},
};
use serde::{Deserialize, Serialize};

/// Arguments for timezone conversion prompt with completion support
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Convert time between timezones with smart completion")]
pub struct TimezoneConversionArgs {
    #[schemars(description = "Source timezone (IANA format, e.g., 'America/New_York')")]
    pub source_timezone: String,
    #[schemars(description = "Time in 24-hour format (HH:MM, e.g., '14:30')")]
    pub time: String,
    #[schemars(description = "Target timezone (IANA format, e.g., 'Europe/London')")]
    pub target_timezone: String,
}

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

    /// Fuzzy matching with scoring for completion suggestions
    fn fuzzy_match(&self, query: &str, candidates: &[String]) -> Vec<String> {
        if query.is_empty() {
            return candidates.iter().take(10).map(|s| s.to_string()).collect();
        }

        let query_lower = query.to_lowercase();
        let mut scored_matches = Vec::new();

        for candidate in candidates {
            let candidate_lower = candidate.to_lowercase();

            let score = if candidate_lower == query_lower {
                1000 // Exact match
            } else if candidate_lower.starts_with(&query_lower) {
                900 // Prefix match  
            } else if candidate_lower.contains(&query_lower) {
                800 // Contains substring
            } else if self.is_acronym_match(&query_lower, candidate) {
                700 // Acronym match (e.g., "ny" → "America/New_York")
            } else if self.is_subsequence_match(&query_lower, &candidate_lower) {
                680 // Subsequence match (e.g., "utc" → "UTC")
            } else if self.is_single_letter_match(&query_lower, candidate) {
                650 // Single letter match (e.g., "u" → "UTC")
            } else {
                continue; // No match
            };

            scored_matches.push((candidate.to_string(), score));
        }

        // Sort by score (desc) then alphabetically
        scored_matches.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        scored_matches
            .into_iter()
            .take(10)
            .map(|(name, _)| name)
            .collect()
    }

    /// Check if query matches as acronym (first letters of words or camelCase)
    fn is_acronym_match(&self, query: &str, candidate: &str) -> bool {
        let query_chars: Vec<char> = query.chars().collect();

        // Extract first letters from words (split by slash and underscore)
        let mut first_chars: Vec<char>;

        // Split by slash for timezone names like "America/New_York"
        let parts: Vec<&str> = candidate.split('/').collect();
        if parts.len() > 1 {
            // Multi-part case (e.g., "America/New_York" -> "ANY")
            first_chars = parts
                .into_iter()
                .flat_map(|part| {
                    // Split by underscore and get first letters
                    part.split('_')
                        .filter_map(|word| word.chars().next())
                        .map(|c| c.to_lowercase().next().unwrap_or('\0'))
                })
                .collect();
        } else {
            // Single word case - extract uppercase letters for camelCase
            first_chars = candidate
                .chars()
                .filter(|c| c.is_uppercase())
                .map(|c| c.to_lowercase().next().unwrap_or('\0'))
                .collect();

            // If no uppercase letters found, just use first letter
            if first_chars.is_empty()
                && !candidate.is_empty()
                && let Some(first) = candidate.chars().next()
            {
                first_chars.push(first.to_lowercase().next().unwrap_or('\0'));
            }
        }

        if query_chars.len() != first_chars.len() {
            return false;
        }

        query_chars
            .iter()
            .zip(first_chars.iter())
            .all(|(q, c)| q.to_lowercase().next().unwrap_or('\0') == *c)
    }

    /// Check if query is a subsequence of candidate (e.g., "utc" in "UTC")
    fn is_subsequence_match(&self, query: &str, candidate_lower: &str) -> bool {
        let query_chars: Vec<char> = query.chars().collect();
        let candidate_chars: Vec<char> = candidate_lower.chars().collect();

        let mut query_idx = 0;

        for &candidate_char in &candidate_chars {
            if query_idx < query_chars.len() && query_chars[query_idx] == candidate_char {
                query_idx += 1;
            }
        }

        query_idx == query_chars.len()
    }

    /// Check if query matches first letter of single word
    fn is_single_letter_match(&self, query: &str, candidate: &str) -> bool {
        if query.len() != 1 {
            return false;
        }

        let query_char = query
            .chars()
            .next()
            .unwrap()
            .to_lowercase()
            .next()
            .unwrap_or('\0');
        let first_char = candidate
            .chars()
            .next()
            .unwrap_or('\0')
            .to_lowercase()
            .next()
            .unwrap_or('\0');

        query_char == first_char
    }

    /// Get timezone names from chrono-tz for completion
    fn get_timezone_candidates(&self) -> Vec<String> {
        use chrono_tz::TZ_VARIANTS;

        // Convert all timezone variants to strings
        // We'll prioritize common ones and limit the total for performance
        let mut timezones: Vec<String> =
            TZ_VARIANTS.iter().map(|tz| tz.name().to_string()).collect();

        // Sort alphabetically for consistent ordering
        timezones.sort();

        // For completion performance, we can limit to a reasonable number
        // or implement smarter filtering based on popularity
        timezones
    }

    /// Get time format suggestions dynamically generated
    fn get_time_format_candidates(&self) -> Vec<String> {
        let mut times = Vec::new();

        // Generate all hours (00:00 to 23:00)
        for hour in 0..24 {
            times.push(format!("{:02}:00", hour));
        }

        // Add common half-hour times
        for hour in 0..24 {
            times.push(format!("{:02}:30", hour));
        }

        // Add common quarter-hour times
        for hour in 0..24 {
            times.push(format!("{:02}:15", hour));
            times.push(format!("{:02}:45", hour));
        }

        // Sort for consistent ordering
        times.sort();
        times
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

    /// Interactive timezone conversion with completion support
    #[prompt(
        name = "timezone_conversion",
        description = "Convert time between timezones with smart completion"
    )]
    async fn timezone_conversion(
        &self,
        Parameters(args): Parameters<TimezoneConversionArgs>,
    ) -> McpResult<GetPromptResult> {
        let result = self.time_server.convert_time(
            &args.source_timezone,
            &args.time,
            &args.target_timezone,
        )?;

        let messages = vec![
            PromptMessage::new_text(
                PromptMessageRole::User,
                format!(
                    "Convert {} from {} to {}",
                    args.time, args.source_timezone, args.target_timezone
                ),
            ),
            PromptMessage::new_text(
                PromptMessageRole::Assistant,
                format!(
                    "Time conversion result:\n\n\
                     **Source:** {} ({})\n\
                     **Target:** {} ({})\n\n\
                     **Details:**\n\
                     • Source DST: {}\n\
                     • Target DST: {}\n\
                     • Day of week: {}",
                    result.source.timezone,
                    result.source.datetime,
                    result.target.timezone,
                    result.target.datetime,
                    result.source.is_dst,
                    result.target.is_dst,
                    result.target.day_of_week
                ),
            ),
        ];

        Ok(GetPromptResult {
            description: Some(format!(
                "Convert {} from {} to {}",
                args.time, args.source_timezone, args.target_timezone
            )),
            messages,
        })
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
                .enable_completions()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(format!(
                "Time MCP Server for timezone operations with smart completion:\n\n\
                 Tools:\n\
                 • get_current_time: Get current time (timezone completion available)\n\
                 • convert_time: Convert between timezones (all fields have completion)\n\n\
                 Completion features:\n\
                 • Fuzzy matching for timezone names ('ny' → 'America/New_York')\n\
                 • Time format suggestions (HH:MM format)\n\
                 • Context-aware suggestions\n\n\
                 Local timezone: {}",
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

    async fn complete(
        &self,
        request: CompleteRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CompleteResult, McpError> {
        let candidates = match &request.r#ref {
            Reference::Prompt(prompt_ref) => {
                tracing::debug!(
                    "Time completion - prompt: {}, argument: {}, value: '{}'",
                    prompt_ref.name,
                    request.argument.name,
                    request.argument.value
                );

                // The current timezone_guidance prompt doesn't take arguments
                // But if we had prompts with timezone arguments, we could provide completion
                match prompt_ref.name.as_str() {
                    "timezone_guidance" => {
                        // This prompt doesn't take arguments, so no completion needed
                        vec![]
                    }
                    "timezone_conversion" => {
                        // Provide completion for timezone conversion prompt arguments
                        match request.argument.name.as_str() {
                            "source_timezone" | "target_timezone" => self.get_timezone_candidates(),
                            "time" => self.get_time_format_candidates(),
                            _ => vec![],
                        }
                    }
                    _ => {
                        // For any future prompts that might have timezone-related arguments
                        match request.argument.name.as_str() {
                            "source_timezone" | "target_timezone" => self.get_timezone_candidates(),
                            "time" => self.get_time_format_candidates(),
                            _ => vec![],
                        }
                    }
                }
            }
            Reference::Resource(_resource_ref) => {
                tracing::debug!(
                    "Time completion - resource completion not implemented, argument: {}",
                    request.argument.name
                );
                vec![]
            }
        };

        let suggestions = self.fuzzy_match(&request.argument.value, &candidates);

        let completion = CompletionInfo {
            values: suggestions,
            total: None,
            has_more: Some(false),
        };

        Ok(CompleteResult { completion })
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
