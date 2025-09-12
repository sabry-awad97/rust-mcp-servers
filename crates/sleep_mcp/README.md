# 😴 Sleep MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-server-sleep.svg)](https://crates.io/crates/mcp-server-sleep)
[![Documentation](https://docs.rs/mcp-server-sleep/badge.svg)](https://docs.rs/mcp-server-sleep)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/sabry-awad97/rust-mcp-servers/workflows/CI/badge.svg)](https://github.com/sabry-awad97/rust-mcp-servers/actions)

A comprehensive **Model Context Protocol (MCP) server** that provides sleep and delay operations for testing, automation, and workflow control with precise timing, status tracking, and cancellation capabilities.

## ✨ Features

- ⏰ **Duration-based Sleep** - Sleep for specified durations (ms, s, m, h)
- 🎯 **Time-based Sleep** - Sleep until specific ISO 8601 timestamps
- 📊 **Real-time Progress** - Monitor active operations with progress reporting
- ❌ **Cancellation Support** - Cancel ongoing sleep operations gracefully
- 🛡️ **Safety Limits** - Built-in limits (max 30 minutes per operation)
- 🔍 **Status Tracking** - Detailed status information and timing data
- 🧹 **Input Validation** - Comprehensive error handling and input sanitization
- 📚 **Rich Documentation** - Built-in help and usage examples

## 🚀 Installation & Usage

### Install from Crates.io

```bash
cargo install mcp-server-sleep
```

### Run the Server

```bash
# Start the MCP server (communicates via stdio)
mcp-server-sleep
```

### Test with MCP Inspector

```bash
# Install and run the MCP Inspector to test the server
npx @modelcontextprotocol/inspector mcp-server-sleep
```

### Use with Claude Desktop

Add to your Claude Desktop MCP configuration:

```json
{
  "mcpServers": {
    "sleep": {
      "command": "mcp-server-sleep"
    }
  }
}
```

## 🛠️ Available Tools

### `sleep`

Sleep for a specified duration with flexible format support.

**Parameters:**

- `duration` (string): Duration to sleep (e.g., "5s", "2m", "500ms")
- `message` (string, optional): Custom message to display during sleep

**Example Request:**

```json
{
  "duration": "30s",
  "message": "Waiting for process to complete"
}
```

**Example Response:**

```json
{
  "success": true,
  "duration": "30s",
  "start_time": "2025-01-15T14:00:00Z",
  "end_time": "2025-01-15T14:00:30Z",
  "message": "Sleep completed successfully",
  "elapsed": "30.001s"
}
```

### `sleep_until`

Sleep until a specific timestamp with ISO 8601 support.

**Parameters:**

- `target_time` (string): ISO 8601 timestamp to sleep until
- `message` (string, optional): Custom message to display during sleep

**Example Request:**

```json
{
  "target_time": "2025-01-15T14:30:00Z",
  "message": "Waiting for scheduled maintenance"
}
```

**Example Response:**

```json
{
  "success": true,
  "duration": "15m 30s",
  "start_time": "2025-01-15T14:14:30Z",
  "end_time": "2025-01-15T14:30:00Z",
  "message": "Sleep completed successfully",
  "elapsed": "15m 30.001s"
}
```

### `get_sleep_status`

Get real-time status and progress of the current sleep operation.

**Parameters:**

- `detailed` (boolean, optional): Include detailed timing information (default: false)

**Example Request:**

```json
{
  "detailed": true
}
```

**Example Response:**

```json
{
  "active": true,
  "duration": "2m",
  "elapsed": "45.2s",
  "remaining": "1m 14.8s",
  "progress": 37.67,
  "start_time": "2025-01-15T14:00:00Z",
  "estimated_end": "2025-01-15T14:02:00Z",
  "message": "Processing data..."
}
```

### `cancel_sleep`

Cancel the current sleep operation gracefully.

**Parameters:** None

**Example Request:**

```json
{}
```

**Example Response:**

```json
{
  "success": true,
  "message": "Sleep operation cancelled",
  "elapsed": "23.4s",
  "was_active": true
}
```

## 📚 Available Resources

### `sleep://status`

Current server status, active operations, and system information.

### `sleep://help`

Comprehensive help documentation with examples and best practices.

### `sleep://examples`

Usage examples, common patterns, and automation workflows.

## ⏱️ Supported Formats

The server supports flexible duration and time formats for maximum usability.

### Duration Formats

- **Milliseconds**: `500ms`, `1000ms`
- **Seconds**: `30s`, `1.5s`, `10s`
- **Minutes**: `5m`, `2.5m`, `30m`
- **Hours**: `0.25h`, `0.5h` (limited to 30 minutes max)

### Time Formats (ISO 8601)

- **UTC Time**: `2025-01-15T14:30:00Z`
- **With Timezone**: `2025-01-15T14:30:00+02:00`
- **With Milliseconds**: `2025-01-15T14:30:00.123Z`

## 🔧 Configuration

The server automatically handles timing and provides configurable safety limits. Maximum sleep duration is set to 30 minutes to prevent resource abuse.

## 📖 Usage Examples

### With Claude Desktop

Once configured, you can ask Claude:

> "Sleep for 30 seconds while waiting for the deployment"

> "Sleep until 2 PM UTC for the scheduled maintenance window"

> "What's the status of the current sleep operation?"

### With MCP Inspector

```bash
# Test sleep operation
npx @modelcontextprotocol/inspector mcp-server-sleep

# Then use the tool:
# Tool: sleep
# Parameters: {"duration": "10s", "message": "Testing sleep"}
```

### Command Line Testing

```bash
# Start the server and test via stdio
echo '{"method": "tools/call", "params": {"name": "sleep", "arguments": {"duration": "5s"}}}' | mcp-server-sleep
```

## 🚨 Error Handling

The server provides detailed error messages for common issues:

- **Invalid Duration**: Shows expected format (number + unit)
- **Duration Too Long**: Indicates maximum allowed duration (30m)
- **Past Time**: Validates that target times are in the future
- **Operation Cancelled**: Confirms successful cancellation

## 🧪 Testing

Run the test suite:

```bash
cargo test
```

Run with coverage:

```bash
cargo tarpaulin --out html
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Setup

1. Clone the repository
2. Install Rust (1.70+ required)
3. Run `cargo build`
4. Run `cargo test`

### Code Style

This project follows the Rust standard formatting. Run `cargo fmt` before submitting.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [rmcp](https://crates.io/crates/rmcp) - Rust MCP implementation
- Async runtime powered by [tokio](https://crates.io/crates/tokio)
- Time handling via [chrono](https://crates.io/crates/chrono)

## 📞 Support

- 📖 [Documentation](https://docs.rs/mcp-server-sleep)
- 🐛 [Issue Tracker](https://github.com/sabry-awad97/rust-mcp-servers/issues)
- 💬 [Discussions](https://github.com/sabry-awad97/rust-mcp-servers/discussions)

---

<div align="center">
  <strong>Made with ❤️ for the MCP ecosystem</strong>
</div>
