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
}

impl Default for FetchService {
    fn default() -> Self {
        Self::new(None, false, None)
    }
}
