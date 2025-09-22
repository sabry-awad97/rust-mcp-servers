use crate::errors::FetchServerError;
use reqwest::{Client, Proxy};
use std::time::Duration;

/// Build a reqwest client with optional proxy
pub fn build_client(proxy_url: Option<&String>) -> Result<Client, FetchServerError> {
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10));

    if let Some(proxy_url) = proxy_url
        && let Ok(proxy) = Proxy::all(proxy_url)
    {
        builder = builder.proxy(proxy);
    }

    builder.build().map_err(|e| FetchServerError::ClientError {
        message: e.to_string(),
    })
}
