# 🕐 Time MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-server-time.svg)](https://crates.io/crates/mcp-server-time)
[![Documentation](https://docs.rs/mcp-server-time/badge.svg)](https://docs.rs/mcp-server-time)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A comprehensive **Model Context Protocol (MCP) server** that provides timezone-aware time operations with **smart completion**, automatic DST handling, and local timezone detection.

## ✨ Features

- 🌍 **Global Timezone Support** - Works with all 400+ IANA timezones
- 🔄 **Time Conversion** - Convert time between different timezones
- 🎯 **Smart Completion** - Fuzzy matching for timezone names and time formats
- 🌅 **Automatic DST Handling** - Seamlessly handles daylight saving transitions
- 📍 **Local Timezone Detection** - Automatically detects system timezone
- 🛡️ **Robust Error Handling** - Comprehensive error messages with suggestions
- 🧹 **Input Sanitization** - Automatically trims whitespace from inputs
- 📚 **Rich Documentation** - Built-in help and timezone references
- 🚀 **Interactive Prompts** - Guided timezone conversion with completion
- 🔧 **Optional Logging** - Configurable logging via LOG_LEVEL environment variable

## 🚀 Installation & Usage

### Install from Crates.io

```bash
cargo install mcp-server-time
```

### Run the Server

```bash
# Start the MCP server (communicates via stdio)
mcp-server-time
```

### Test with MCP Inspector

```bash
# Install and run the MCP Inspector to test the server
npx @modelcontextprotocol/inspector mcp-server-time
```

### Use with Claude Desktop

Add to your Claude Desktop MCP configuration:

```json
{
  "mcpServers": {
    "time": {
      "command": "mcp-server-time",
      "args": [],
      "env": {
        "LOG_LEVEL": "debug"
      }
    }
  }
}
```

## 🛠️ Available Tools

### Smart Completion Features

**NEW in v0.2.0**: The server now provides intelligent completion for:

- **Timezone Names**: Fuzzy matching (e.g., "ny" → "America/New_York", "tok" → "Asia/Tokyo")
- **Time Formats**: All 15-minute intervals (00:00, 00:15, 00:30, 00:45, etc.)
- **Context-Aware**: Suggestions adapt based on what you're typing

### Tools

### `get_current_time`

Get the current time in any IANA timezone.

**Parameters:**

- `timezone` (string): IANA timezone name (e.g., "America/New_York", "Europe/London")

**Example Request:**

```json
{
  "timezone": "Asia/Tokyo"
}
```

**Example Response:**

```json
{
  "timezone": "Asia/Tokyo",
  "datetime": "2025-01-15T14:30:00+09:00",
  "day_of_week": "Monday",
  "is_dst": false
}
```

### `convert_time`

Convert time between different timezones.

**Parameters:**

- `source_timezone` (string): Source IANA timezone name
- `time` (string): Time in 24-hour format (HH:MM)
- `target_timezone` (string): Target IANA timezone name

**Example Request:**

```json
{
  "source_timezone": "America/Los_Angeles",
  "time": "09:00",
  "target_timezone": "Europe/Paris"
}
```

**Example Response:**

```json
{
  "source": {
    "timezone": "America/Los_Angeles",
    "datetime": "2025-01-15T09:00:00-08:00",
    "day_of_week": "Monday",
    "is_dst": false
  },
  "target": {
    "timezone": "Europe/Paris",
    "datetime": "2025-01-15T18:00:00+01:00",
    "day_of_week": "Monday",
    "is_dst": false
  },
  "time_difference": "+9h"
}
```

## 💬 Available Prompts

### `timezone_guidance`

Get comprehensive guidance on timezone best practices, IANA naming conventions, and DST handling.

### `timezone_conversion` ⭐ NEW

Interactive timezone conversion with **smart completion support**. This prompt provides:

- **Fuzzy timezone matching**: Type partial names for suggestions
- **Time format completion**: Get suggestions for valid time formats
- **Rich conversion results**: Detailed information with DST status

**Parameters:**

- `source_timezone` (string): Source IANA timezone (with completion)
- `time` (string): Time in HH:MM format (with completion)
- `target_timezone` (string): Target IANA timezone (with completion)

## 📚 Available Resources

### `time://status`

Current server status, local timezone, and system information.

### `time://help`

Comprehensive help documentation with examples and best practices.

### `time://timezones`

List of common IANA timezone names organized by region.

## 🌐 Supported Timezones

The server dynamically supports **all 400+ IANA timezone names** from `chrono-tz`. The completion system provides fuzzy matching for easy discovery. Here are some common examples:

### Americas

- `America/New_York` - Eastern Time
- `America/Los_Angeles` - Pacific Time
- `America/Chicago` - Central Time
- `America/Toronto` - Eastern Time (Canada)
- `America/Sao_Paulo` - Brazil Time

### Europe

- `Europe/London` - Greenwich Mean Time
- `Europe/Paris` - Central European Time
- `Europe/Berlin` - Central European Time
- `Europe/Moscow` - Moscow Time

### Asia

- `Asia/Tokyo` - Japan Standard Time
- `Asia/Shanghai` - China Standard Time
- `Asia/Kolkata` - India Standard Time
- `Asia/Dubai` - Gulf Standard Time

### Special

- `UTC` - Coordinated Universal Time
- `GMT` - Greenwich Mean Time

## 🔧 Configuration

### Environment Variables

**NEW in v0.3.0**: Optional logging configuration

- `LOG_LEVEL` (optional): Set logging level (debug, info, warn, error). If not set, logging is disabled for better performance.

```bash
# Run without logging (default)
mcp-server-time

# Run with debug logging
LOG_LEVEL=debug mcp-server-time

# Run with info logging
LOG_LEVEL=info mcp-server-time
```

### Timezone Detection

The server automatically detects your local timezone. You can override this by setting environment variables or using the builder pattern (if implemented).

## 📖 Usage Examples

### With Claude Desktop

Once configured, you can ask Claude:

> "What time is it in Tokyo right now?"

> "Convert 2 PM Los Angeles time to London time"

> "What's the time difference between New York and Sydney?"

> "Use the timezone conversion prompt to convert 14:30 from Europe/London to Asia/Tokyo"

**NEW**: Try the interactive `timezone_conversion` prompt for guided conversion with smart completion!

### With MCP Inspector

```bash
# Test the server with smart completion
npx @modelcontextprotocol/inspector mcp-server-time
```

**Try these features:**

1. **Tools**: Use `get_current_time` or `convert_time`
2. **Prompts**: Try `timezone_conversion` with smart completion:
   - Type "ny" in timezone fields → see "America/New_York" suggested
   - Type "14" in time field → see "14:00", "14:15", "14:30", "14:45" suggested
3. **Resources**: Browse `time://help` for documentation

### Command Line Testing

The MCP protocol requires proper initialization. Use the MCP Inspector for testing:

```bash
# Use MCP Inspector for interactive testing
npx @modelcontextprotocol/inspector mcp-server-time

# Or test with a proper MCP client that handles the initialization handshake
```

**Note**: Direct stdio testing requires implementing the full MCP protocol handshake (initialize → tools/list → tools/call).

## 🚨 Error Handling

The server provides detailed error messages for common issues:

- **Invalid Timezone**: Suggests similar timezone names
- **Invalid Time Format**: Shows expected format (HH:MM)
- **Ambiguous Time**: Handles DST transition edge cases
- **Resource Not Found**: Lists available resources

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
- Timezone data from [chrono-tz](https://crates.io/crates/chrono-tz)
- Local timezone detection via [iana-time-zone](https://crates.io/crates/iana-time-zone)

## 📞 Support

- 📖 [Documentation](https://docs.rs/mcp-server-time)
- 🐛 [Issue Tracker](https://github.com/sabry-awad97/rust-mcp-servers/issues)
- 💬 [Discussions](https://github.com/sabry-awad97/rust-mcp-servers/discussions)

---

<div align="center">
  <strong>Made with ❤️ for the MCP ecosystem</strong>
</div>
