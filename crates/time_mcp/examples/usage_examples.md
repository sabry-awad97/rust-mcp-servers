# Usage Examples

This document provides practical examples of using the Time MCP Server.

## Installation

```bash
cargo install mcp-server-time
```

## Claude Desktop Integration

### Configuration

Add to your Claude Desktop MCP configuration file:

**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "time": {
      "command": "mcp-server-time"
    }
  }
}
```

### Example Conversations

Once configured, you can have natural conversations with Claude:

**User:** "What time is it in Tokyo right now?"

**Claude:** I'll check the current time in Tokyo for you.
_[Uses get_current_time tool with timezone: "Asia/Tokyo"]_

**User:** "Convert 2 PM Los Angeles time to London time"

**Claude:** I'll convert 2 PM Los Angeles time to London time.
_[Uses convert_time tool with source_timezone: "America/Los_Angeles", time: "14:00", target_timezone: "Europe/London"]_

**User:** "What's the time difference between New York and Sydney?"

**Claude:** I'll find the time difference between New York and Sydney by converting the current time.
_[Uses convert_time tool to show the time difference]_

## MCP Inspector Testing

### Install and Run Inspector

```bash
# Install the MCP Inspector
npm install -g @modelcontextprotocol/inspector

# Test the server
npx @modelcontextprotocol/inspector mcp-server-time
```

### Test Tools

#### Get Current Time

```json
{
  "name": "get_current_time",
  "arguments": {
    "timezone": "Europe/Paris"
  }
}
```

#### Convert Time

```json
{
  "name": "convert_time",
  "arguments": {
    "source_timezone": "America/New_York",
    "time": "15:30",
    "target_timezone": "Asia/Singapore"
  }
}
```

### Test Resources

- `time://status` - View server status
- `time://help` - Get help documentation
- `time://timezones` - List available timezones

## Command Line Testing

### Direct stdio Communication

```bash
# Start server and send JSON-RPC request
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "get_current_time", "arguments": {"timezone": "UTC"}}}' | mcp-server-time
```

### Testing with jq for Pretty Output

```bash
# Get current time with formatted output
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "get_current_time", "arguments": {"timezone": "America/Chicago"}}}' | mcp-server-time | jq '.'
```

## Common Use Cases

### Meeting Scheduling

```bash
# Find what time a 3 PM EST meeting would be in various timezones
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "convert_time", "arguments": {"source_timezone": "America/New_York", "time": "15:00", "target_timezone": "Europe/London"}}}' | mcp-server-time

echo '{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "convert_time", "arguments": {"source_timezone": "America/New_York", "time": "15:00", "target_timezone": "Asia/Tokyo"}}}' | mcp-server-time
```

### Travel Planning

```bash
# Check arrival time when flying from LA to Paris
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "convert_time", "arguments": {"source_timezone": "America/Los_Angeles", "time": "08:00", "target_timezone": "Europe/Paris"}}}' | mcp-server-time
```

### Business Hours Check

```bash
# Check if it's business hours (9 AM - 5 PM) in different locations
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "get_current_time", "arguments": {"timezone": "Asia/Shanghai"}}}' | mcp-server-time
```

## Error Handling Examples

### Invalid Timezone

```bash
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "get_current_time", "arguments": {"timezone": "Invalid/Timezone"}}}' | mcp-server-time
```

### Invalid Time Format

```bash
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "convert_time", "arguments": {"source_timezone": "UTC", "time": "25:00", "target_timezone": "UTC"}}}' | mcp-server-time
```

## Logging and Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug mcp-server-time
```

### Enable Trace Logging

```bash
RUST_LOG=trace mcp-server-time
```

### Log to File

```bash
RUST_LOG=info mcp-server-time 2> server.log
```

## Integration with Other Tools

### Using with curl (if running HTTP wrapper)

```bash
# Note: This requires an HTTP wrapper around the MCP server
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "tools/call", "params": {"name": "get_current_time", "arguments": {"timezone": "Australia/Sydney"}}}'
```

### Shell Script Integration

```bash
#!/bin/bash
# get_time.sh - Get current time in specified timezone

TIMEZONE=${1:-UTC}
RESULT=$(echo "{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"tools/call\", \"params\": {\"name\": \"get_current_time\", \"arguments\": {\"timezone\": \"$TIMEZONE\"}}}" | mcp-server-time)
echo "$RESULT" | jq -r '.result.content[0].text' | jq -r '.datetime'
```

Usage:

```bash
chmod +x get_time.sh
./get_time.sh "America/New_York"
./get_time.sh "Asia/Tokyo"
```
