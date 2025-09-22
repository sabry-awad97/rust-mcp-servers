use url::Url;

use crate::{
    errors::FetchServerError,
    utils::{build_client, extract_content_from_html, get_robots_txt_url},
};

const DEFAULT_USER_AGENT_AUTONOMOUS: &str =
    "ModelContextProtocol/1.0 (Autonomous; +https://github.com/modelcontextprotocol/servers)";
const DEFAULT_USER_AGENT_MANUAL: &str =
    "ModelContextProtocol/1.0 (User-Specified; +https://github.com/modelcontextprotocol/servers)";

#[derive(Clone)]
pub struct FetchService {
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
            custom_user_agent,
            ignore_robots_txt,
            proxy_url,
        }
    }

    pub fn get_user_agent_autonomous(&self) -> &str {
        self.custom_user_agent
            .as_deref()
            .unwrap_or(DEFAULT_USER_AGENT_AUTONOMOUS)
    }

    pub fn get_user_agent_manual(&self) -> &str {
        self.custom_user_agent
            .as_deref()
            .unwrap_or(DEFAULT_USER_AGENT_MANUAL)
    }

    /// Check if the URL can be fetched autonomously according to robots.txt
    pub async fn check_may_autonomously_fetch_url(
        &self,
        url: &str,
    ) -> Result<(), FetchServerError> {
        if self.ignore_robots_txt {
            return Ok(());
        }

        let robots_txt_url = get_robots_txt_url(url)?;

        // Create client with proxy if configured
        let client = build_client(self.proxy_url.as_ref())?;

        let user_agent = self.get_user_agent_autonomous();

        let response = client
            .get(&robots_txt_url)
            .header("User-Agent", user_agent)
            .send()
            .await
            .map_err(|e| FetchServerError::RobotsFetchError {
                url: robots_txt_url.clone(),
                message: e.to_string(),
            })?;

        let status = response.status();

        if status == 401 || status == 403 {
            return Err(FetchServerError::RobotsForbidden {
                url: robots_txt_url,
                message: format!(
                    "When fetching robots.txt, received status {} so assuming that autonomous fetching is not allowed, the user can try manually fetching by using the fetch prompt",
                    status.as_u16()
                ),
            });
        }

        if status.is_client_error() {
            // 4xx errors other than 401/403 are treated as "no robots.txt", so allow fetching
            return Ok(());
        }

        let robots_txt = response
            .text()
            .await
            .map_err(|e| FetchServerError::ContentError {
                message: e.to_string(),
            })?;

        // Simple robots.txt parsing - check for Disallow rules
        let processed_robots = robots_txt
            .lines()
            .filter(|line| !line.trim().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");

        // Basic robots.txt checking - look for User-agent: * and Disallow: /
        let mut current_user_agent = "";
        let mut disallowed_paths = Vec::new();

        for line in processed_robots.lines() {
            let line = line.trim();
            if line.to_lowercase().starts_with("user-agent:") {
                current_user_agent = line.split(':').nth(1).unwrap_or("").trim();
            } else if line.to_lowercase().starts_with("disallow:")
                && (current_user_agent == "*" || current_user_agent.is_empty())
            {
                let path = line.split(':').nth(1).unwrap_or("").trim();
                disallowed_paths.push(path);
            }
        }

        // Check if the URL path is disallowed
        let parsed_url = Url::parse(url).unwrap();
        let url_path = parsed_url.path();

        for disallowed in &disallowed_paths {
            if !disallowed.is_empty() && (*disallowed == "/" || url_path.starts_with(disallowed)) {
                return Err(FetchServerError::RobotsDisallowed {
                    url: url.to_string(),
                    message: format!(
                        "The sites robots.txt specifies that autonomous fetching of this page is not allowed, <useragent>{}</useragent>\n<url>{}</url><robots>\n{}\n</robots>\nThe assistant must let the user know that it failed to view the page. The assistant may provide further guidance based on the above information.\nThe assistant can tell the user that they can try manually fetching the page by using the fetch prompt within their UI.",
                        user_agent, url, robots_txt
                    ),
                });
            }
        }

        Ok(())
    }

    pub async fn fetch_url(
        &self,
        url: &str,
        user_agent: &str,
        force_raw: bool,
    ) -> Result<(String, String), FetchServerError> {
        let client = build_client(self.proxy_url.as_ref())?;
        let response = client
            .get(url)
            .header("User-Agent", user_agent)
            .send()
            .await
            .map_err(|e| FetchServerError::FetchError {
                url: url.to_string(),
                message: e.to_string(),
            })?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            return Err(FetchServerError::HttpError {
                url: url.to_string(),
                status: status.as_u16(),
            });
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let page_raw = response
            .text()
            .await
            .map_err(|e| FetchServerError::ContentError {
                message: e.to_string(),
            })?;

        let is_page_html = page_raw.get(..100).unwrap_or(&page_raw).contains("<html")
            || content_type.contains("text/html")
            || content_type.is_empty();

        if is_page_html && !force_raw {
            let markdown = extract_content_from_html(&page_raw).await;
            Ok((markdown, String::new()))
        } else {
            let prefix = format!(
                "Content type {} cannot be simplified to markdown, but here is the raw content:\n",
                content_type
            );
            Ok((page_raw, prefix))
        }
    }
}

impl Default for FetchService {
    fn default() -> Self {
        Self::new(None, false, None)
    }
}
