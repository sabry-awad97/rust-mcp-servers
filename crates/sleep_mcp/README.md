# 😴 Sleep MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-server-sleep.svg)](https://crates.io/crates/mcp-server-sleep)
[![Documentation](https://docs.rs/mcp-server-sleep/badge.svg)](https://docs.rs/mcp-server-sleep)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/sabry-awad97/rust-mcp-servers/workflows/CI/badge.svg)](https://github.com/sabry-awad97/rust-mcp-servers/actions)

A comprehensive **Model Context Protocol (MCP) server** that provides both blocking and non-blocking sleep operations for testing, automation, and workflow control with precise timing, concurrent operation management, and advanced cancellation capabilities.

## ✨ Features

- ⏰ **Dual Sleep Modes** - Both blocking and non-blocking sleep operations
- 🔄 **Concurrent Operations** - Manage multiple sleep operations simultaneously
- 🎯 **Time-based Sleep** - Sleep until specific ISO 8601 timestamps
- 📊 **Real-time Progress** - Monitor active operations with detailed progress reporting
- 🆔 **Operation Tracking** - Unique operation IDs for precise management
- ❌ **Advanced Cancellation** - Cancel individual operations or all at once
- 🛡️ **Safety Limits** - Built-in limits (max 30 minutes per operation)
- 🔍 **Enhanced Status** - Comprehensive status information with operation details
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

### `sleep` (Non-blocking)

Sleep for a specified duration with flexible format support. Returns immediately with an operation ID for tracking.

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
  "operation_id": "abc123-def456-789",
  "message": "Sleep operation started",
  "duration_ms": 30000,
  "duration_str": "30s",
  "expected_completion": "2025-01-15T14:00:30Z"
}
```

### `sleep_blocking` (Blocking)

Sleep for a specified duration and wait for completion before returning results.

**Parameters:**

- `duration` (string): Duration to sleep (e.g., "5s", "2m", "500ms")
- `message` (string, optional): Custom message to display during sleep

**Example Request:**

```json
{
  "duration": "5s",
  "message": "Synchronous wait"
}
```

**Example Response:**

```json
{
  "duration_ms": 5000,
  "duration_str": "5s",
  "start_time": "2025-01-15T14:00:00Z",
  "end_time": "2025-01-15T14:00:05Z",
  "completed": true,
  "message": "Synchronous wait"
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

Get real-time status and progress of sleep operations with support for querying specific operations.

**Parameters:**

- `detailed` (boolean, optional): Include detailed timing information (default: false)
- `operation_id` (string, optional): Query specific operation by ID

**Example Request:**

```json
{
  "detailed": true,
  "operation_id": "abc123-def456-789"
}
```

**Example Response:**

```json
{
  "is_sleeping": true,
  "active_operations": 2,
  "operations": [
    {
      "operation_id": "abc123-def456-789",
      "status": "Running",
      "duration_ms": 120000,
      "duration_str": "2m",
      "start_time": "2025-01-15T14:00:00Z",
      "expected_end_time": "2025-01-15T14:02:00Z",
      "progress_percent": 37.67,
      "remaining_ms": 74800,
      "message": "Processing data..."
    }
  ],
  "current_operation": {
    "operation_id": "abc123-def456-789",
    "status": "Running",
    "progress_percent": 37.67
  }
}
```

### `cancel_operation`

Cancel a specific sleep operation by its operation ID.

**Parameters:**

- `operation_id` (string): The operation ID to cancel

**Example Request:**

```json
{
  "operation_id": "abc123-def456-789"
}
```

**Example Response:**

```json
{
  "success": true,
  "message": "Operation abc123-def456-789 cancelled successfully",
  "operation_id": "abc123-def456-789"
}
```

### `cancel_sleep`

Cancel all active sleep operations gracefully.

**Parameters:** None

**Example Request:**

```json
{}
```

**Example Response:**

```json
{
  "success": true,
  "message": "2 sleep operations cancelled",
  "cancelled_count": 2
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

> "Sleep for 30 seconds while waiting for the deployment" (non-blocking)

> "Do a blocking sleep for 5 seconds and wait for completion"

> "Sleep until 2 PM UTC for the scheduled maintenance window"

> "What's the status of all sleep operations?"

> "Cancel operation abc123-def456"

> "Cancel all active sleep operations"

### With MCP Inspector

```bash
# Test sleep operation
npx @modelcontextprotocol/inspector mcp-server-sleep

# Then use the tools:
# Tool: sleep (non-blocking)
# Parameters: {"duration": "10s", "message": "Testing non-blocking sleep"}

# Tool: sleep_blocking (blocking)
# Parameters: {"duration": "5s", "message": "Testing blocking sleep"}

# Tool: get_sleep_status
# Parameters: {"detailed": true}

# Tool: cancel_operation
# Parameters: {"operation_id": "operation-id-here"}
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
