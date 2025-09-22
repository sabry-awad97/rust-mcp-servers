mod html_utils;
pub use html_utils::extract_content_from_html;

mod http_client;
pub use http_client::build_client;

mod robots_utils;
pub use robots_utils::get_robots_txt_url;
