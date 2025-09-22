# 🌐 Fetch MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-server-fetch.svg)](https://crates.io/crates/mcp-server-fetch)
[![Documentation](https://docs.rs/mcp-server-fetch/badge.svg)](https://docs.rs/mcp-server-fetch)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A powerful **Model Context Protocol (MCP) server** that provides secure web content fetching with **robots.txt compliance**, HTML-to-markdown conversion, content truncation, and comprehensive HTTP operations.

## ✨ Features

- 🌐 **Web Content Fetching** - Retrieve content from any HTTP/HTTPS URL
- 🤖 **Robots.txt Compliance** - Automatic robots.txt checking for autonomous fetching
- 📝 **HTML to Markdown** - Intelligent conversion of HTML content to clean markdown
- ✂️ **Content Truncation** - Configurable content length limits with continuation support
- 🔄 **Raw HTML Mode** - Option to retrieve unprocessed HTML content
- 🕵️ **Custom User Agents** - Configurable user agent strings for different use cases
- 🌐 **Proxy Support** - HTTP proxy configuration for network environments
- 🛡️ **Security First** - Safe URL validation and error handling
- 📊 **Flexible Parameters** - Configurable max length, start index, and content format
- 🎯 **Dual Modes** - Both tool and prompt interfaces for different use cases
- 🧹 **Input Validation** - Comprehensive parameter validation and sanitization
- 🎯 **SOLID Architecture** - Clean, maintainable, and testable codebase

## 🚀 Installation & Usage

### Install from Crates.io

```bash
cargo install mcp-server-fetch
```

### Run the Server

```bash
# Start the MCP server (communicates via stdio)
mcp-server-fetch

# Use custom user agent
mcp-server-fetch --user-agent "MyApp/1.0"

# Ignore robots.txt restrictions
mcp-server-fetch --ignore-robots-txt

# Use HTTP proxy
mcp-server-fetch --proxy-url "http://proxy.example.com:8080"

# Enable debug logging
LOG_LEVEL=debug mcp-server-fetch
```

### Test with MCP Inspector

```bash
# Install and run the MCP Inspector to test the server
npx @modelcontextprotocol/inspector mcp-server-fetch
```

### Use with Claude Desktop

Add to your Claude Desktop MCP configuration:

```json
{
  "mcpServers": {
    "fetch": {
      "command": "mcp-server-fetch",
      "args": ["--user-agent", "Claude-Desktop/1.0"]
    }
  }
}
```

## 🛠️ Available Tools

### `fetch`

Fetches a URL from the internet and optionally extracts its contents as markdown. This tool provides internet access capabilities with intelligent content processing.

**Parameters:**

- `url` (string): The URL to fetch
- `max_length` (optional number): Maximum number of characters to return (default: 5000, max: 1,000,000)
- `start_index` (optional number): Starting character index for content extraction (default: 0)
- `raw` (optional boolean): Return raw HTML content without markdown conversion (default: false)

**Example Request:**

```json
{
  "url": "https://example.com/article",
  "max_length": 10000,
  "start_index": 0,
  "raw": false
}
```

**Example Response:**

```json
{
  "content": [
    {
      "type": "text",
      "text": "Contents of https://example.com/article:\n\n# Article Title\n\nThis is the converted markdown content..."
    }
  ]
}
```

**Content Truncation:**

When content exceeds the `max_length`, the response includes continuation instructions:

```
Content truncated. Call the fetch tool with a start_index of 5000 to get more content.
```

**Robots.txt Compliance:**

The server automatically checks robots.txt for autonomous fetching:

- ✅ Allowed URLs proceed normally
- ❌ Disallowed URLs return an error with robots.txt information
- 🔧 Use `--ignore-robots-txt` flag to bypass restrictions

## 📚 Available Prompts

### `fetch`

Manual URL fetching prompt that retrieves and processes web content for immediate use in conversations.

**Parameters:**

- `url` (string): The URL to fetch

**Example Usage:**

```
Use the fetch prompt with URL: https://news.example.com/latest
```

**Response:**

Returns a prompt message containing the fetched and processed content, ready for use in the conversation context.

## 🔧 Configuration

### Command Line Options

```bash
mcp-server-fetch [OPTIONS]

Options:
      --user-agent <USER_AGENT>    Custom User-Agent string to use for requests
      --ignore-robots-txt          Ignore robots.txt restrictions
      --proxy-url <PROXY_URL>      Proxy URL to use for requests (e.g., http://proxy:8080)
  -h, --help                       Print help information
  -V, --version                    Print version information
```

### Environment Variables

- `LOG_LEVEL`: Set logging level (trace, debug, info, warn, error)

### User Agent Modes

The server uses different user agents depending on the context:

- **Autonomous Mode**: `ModelContextProtocol/1.0 (Autonomous; +https://github.com/modelcontextprotocol/servers)`
- **Manual Mode**: `ModelContextProtocol/1.0 (User-Specified; +https://github.com/modelcontextprotocol/servers)`
- **Custom**: Your specified user agent string

### Security Model

The server implements several security measures:

- **URL Validation**: All URLs are validated before fetching
- **Robots.txt Compliance**: Automatic checking for autonomous operations
- **Content Limits**: Configurable size limits prevent abuse
- **Error Sanitization**: Safe error messages without sensitive information
- **Proxy Support**: Secure proxy configuration for network environments

## 📖 Usage Examples

### With Claude Desktop

Once configured, you can ask Claude:

> "Fetch the latest news from https://news.example.com"

> "Get the content from this documentation page: https://docs.example.com/api"

> "Retrieve the raw HTML from https://example.com without markdown conversion"

> "Fetch the first 2000 characters from this long article: https://blog.example.com/long-post"

### With MCP Inspector

```bash
# Test the server interactively
npx @modelcontextprotocol/inspector mcp-server-fetch

# Try these operations:
# 1. Use fetch tool with different URLs
# 2. Test content truncation with max_length
# 3. Try raw HTML mode
# 4. Test robots.txt compliance
# 5. Use the fetch prompt for immediate content
```

### Advanced Usage Examples

**Fetching Large Content in Chunks:**

```json
{
  "url": "https://example.com/large-document",
  "max_length": 5000,
  "start_index": 0
}
```

Follow up with:

```json
{
  "url": "https://example.com/large-document",
  "max_length": 5000,
  "start_index": 5000
}
```

**Raw HTML Extraction:**

```json
{
  "url": "https://example.com/complex-page",
  "raw": true,
  "max_length": 10000
}
```

**Custom Configuration:**

```bash
# Production setup with custom user agent and proxy
mcp-server-fetch \
  --user-agent "MyCompany-AI/2.0 (+https://mycompany.com/bot)" \
  --proxy-url "http://corporate-proxy:8080"
```

## 🚨 Error Handling

The server provides detailed error messages for common issues:

- **Invalid URL**: Clear feedback for malformed URLs
- **Network Errors**: Helpful messages for connection issues
- **Robots.txt Violations**: Specific guidance about autonomous fetching restrictions
- **Content Limits**: Information about size restrictions and truncation
- **Validation Errors**: Specific feedback on parameter validation failures
- **Proxy Errors**: Clear messages for proxy configuration issues

**Example Error Response:**

```json
{
  "error": {
    "code": -32602,
    "message": "Robots.txt disallows autonomous fetching of this URL",
    "data": {
      "url": "https://example.com/restricted",
      "robots_txt_url": "https://example.com/robots.txt",
      "user_agent": "ModelContextProtocol/1.0 (Autonomous)"
    }
  }
}
```

## 🧪 Testing

Run the comprehensive test suite:

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test fetch_service
cargo test validation
cargo test server

# Run with coverage
cargo tarpaulin --out html
```

### Integration Testing

```bash
# Test with real URLs (requires internet)
cargo test --features integration-tests

# Test robots.txt compliance
cargo test robots_txt_tests

# Test content processing
cargo test content_processing
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Setup

1. Clone the repository
2. Install Rust (1.70+ required)
3. Run `cargo build`
4. Run `cargo test`

### Code Style

This project follows SOLID principles and Domain-Driven Design:

- **Clean Architecture**: Separation of concerns with clear layers
- **Dependency Injection**: Testable and maintainable code
- **Comprehensive Testing**: Unit and integration tests
- **Error Handling**: Robust error types and handling

Run `cargo fmt` and `cargo clippy` before submitting.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [rmcp](https://crates.io/crates/rmcp) - Rust MCP implementation
- HTTP client powered by [reqwest](https://crates.io/crates/reqwest)
- HTML to Markdown conversion via [fast_html2md](https://crates.io/crates/fast_html2md)
- URL parsing using [url](https://crates.io/crates/url)
- Async runtime provided by [tokio](https://crates.io/crates/tokio)

## 📞 Support

- 📖 [Documentation](https://docs.rs/mcp-server-fetch)
- 🐛 [Issue Tracker](https://github.com/sabry-awad97/rust-mcp-servers/issues)
- 💬 [Discussions](https://github.com/sabry-awad97/rust-mcp-servers/discussions)

---

<div align="center">
  <strong>Made with ❤️ for secure web content fetching in the MCP ecosystem</strong>
</div>
