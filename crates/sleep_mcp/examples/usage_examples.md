# Usage Examples

This document provides practical examples of using the Sleep MCP Server.

## Installation

```bash
cargo install mcp-server-sleep
```

## Claude Desktop Integration

### Configuration

Add to your Claude Desktop MCP configuration file:

**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "sleep": {
      "command": "node",
      "command": "mcp-server-sleep",
      "disabled": false,
      "autoApprove": [],
      "timeout": 300
    }
  }
}
```

**Note:** The timeout parameter specifies the maximum time (in milliseconds) that the MCP server will wait for a response before timing out. This is particularly important for the sleep tool, as setting a timeout that's shorter than your sleep duration will cause the operation to fail. Make sure your timeout value is always greater than the maximum sleep duration you plan to use.

### Example Conversations

Once configured, you can have natural conversations with Claude:

**User:** "Sleep for 30 seconds while waiting for the deployment"

**Claude:** I'll start a 30-second sleep operation for you.
_[Uses sleep tool with duration: "30s", message: "Waiting for deployment"]_

**User:** "Sleep until 2 PM UTC for the scheduled maintenance window"

**Claude:** I'll set up a sleep operation until 2 PM UTC.
_[Uses sleep_until tool with target_time: "2024-01-15T14:00:00Z", message: "Scheduled maintenance window"]_

**User:** "What's the status of the current sleep operation?"

**Claude:** Let me check the current sleep operation status for you.
_[Uses get_sleep_status tool with detailed: true]_

**User:** "Cancel the current sleep operation"

**Claude:** I'll cancel the current sleep operation.
_[Uses cancel_sleep tool]_

## MCP Inspector Testing

### Install and Run Inspector

```bash
# Install the MCP Inspector
npm install -g @modelcontextprotocol/inspector

# Test the server
npx @modelcontextprotocol/inspector mcp-server-sleep
```

### Test Tools

#### Sleep for Duration

```json
{
  "name": "sleep",
  "arguments": {
    "duration": "10s",
    "message": "Testing sleep functionality"
  }
}
```

#### Sleep Until Time

```json
{
  "name": "sleep_until",
  "arguments": {
    "target_time": "2024-01-15T14:30:00Z",
    "message": "Waiting for scheduled task"
  }
}
```

#### Get Sleep Status

```json
{
  "name": "get_sleep_status",
  "arguments": {
    "detailed": true
  }
}
```

#### Cancel Sleep

```json
{
  "name": "cancel_sleep",
  "arguments": {}
}
```

### Test Resources

- `sleep://status` - View server status and active operations
- `sleep://help` - Get help documentation
- `sleep://examples` - Usage examples and patterns

## Command Line Testing

### Direct stdio Communication

```bash
# Start server and send JSON-RPC request
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "sleep", "arguments": {"duration": "5s"}}}' | mcp-server-sleep
```

### Testing with jq for Pretty Output

```bash
# Sleep with formatted output
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "sleep", "arguments": {"duration": "10s", "message": "Test sleep"}}}' | mcp-server-sleep | jq '.'
```

## Common Use Cases

### Test Automation

```bash
# Add delay between test steps
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "sleep", "arguments": {"duration": "2s", "message": "Waiting for UI to load"}}}' | mcp-server-sleep

# Wait for service startup
echo '{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "sleep", "arguments": {"duration": "30s", "message": "Service startup delay"}}}' | mcp-server-sleep
```

### Deployment Automation

```bash
# Wait for deployment to complete
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "sleep_until", "arguments": {"target_time": "2024-01-15T02:00:00Z", "message": "Maintenance window"}}}' | mcp-server-sleep
```

### Rate Limiting

```bash
# Add delay between API calls
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "sleep", "arguments": {"duration": "1s", "message": "Rate limiting delay"}}}' | mcp-server-sleep
```

### Progress Monitoring

```bash
# Check status of long-running operation
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "get_sleep_status", "arguments": {"detailed": true}}}' | mcp-server-sleep
```

## Error Handling Examples

### Invalid Duration Format

```bash
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "sleep", "arguments": {"duration": "invalid"}}}' | mcp-server-sleep
```

### Duration Too Long

```bash
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "sleep", "arguments": {"duration": "2h"}}}' | mcp-server-sleep
```

### Past Time

```bash
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "sleep_until", "arguments": {"target_time": "2020-01-01T00:00:00Z"}}}' | mcp-server-sleep
```

## Logging and Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug mcp-server-sleep
```

### Enable Trace Logging

```bash
RUST_LOG=trace mcp-server-sleep
```

### Log to File

```bash
RUST_LOG=info mcp-server-sleep 2> server.log
```
