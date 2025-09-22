use crate::errors::FetchServerError;
use url::Url;

/// Build robots.txt URL from a given webpage URL
pub fn get_robots_txt_url(url: &str) -> Result<String, FetchServerError> {
    let parsed = Url::parse(url).map_err(|_| FetchServerError::InvalidUrl {
        url: url.to_string(),
    })?;

    let robots_url = format!(
        "{}://{}/robots.txt",
        parsed.scheme(),
        parsed.host_str().unwrap_or("")
    );
    Ok(robots_url)
}
