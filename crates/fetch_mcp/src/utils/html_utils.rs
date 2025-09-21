/// Convert HTML content to Markdown
pub async fn extract_content_from_html(html: &str) -> String {
    let md = html2md::rewrite_html_streaming(html, false).await;
    if md.trim().is_empty() {
        "<error>Page failed to be simplified from HTML</error>".to_string()
    } else {
        md
    }
}
