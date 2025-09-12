# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-09-12

### Added

- 🎉 Initial release of Time MCP Server
- 🌍 Global timezone support with all IANA timezones
- 🔄 Time conversion between different timezones
- 🌅 Automatic daylight saving time (DST) handling
- 📍 Local timezone detection using system settings
- 🛡️ Comprehensive error handling with detailed messages
- 🧹 Automatic input sanitization (whitespace trimming)
- 📚 Built-in help and documentation resources

### Tools

- `get_current_time` - Get current time in any IANA timezone
- `convert_time` - Convert time between different timezones

### Resources

- `time://status` - Server status and current local time
- `time://help` - Comprehensive help documentation with examples
- `time://timezones` - List of common IANA timezone names by region

### Features

- Supports all IANA timezone names (America/New_York, Europe/London, etc.)
- Handles DST transitions automatically
- Returns structured time data with timezone info, day of week, and DST status
- Calculates and displays time differences between zones
- Robust error messages for invalid inputs
- Local timezone auto-detection on startup

### Technical Details

- Built with Rust using the rmcp framework
- Uses chrono-tz for timezone calculations
- Implements Model Context Protocol (MCP) specification
- Communicates via stdio for integration with MCP clients
- Comprehensive test suite with 14+ test cases
- Follows Rust best practices and SOLID principles

[Unreleased]: https://github.com/sabry-awad97/rust-mcp-servers/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/sabry-awad97/rust-mcp-servers/releases/tag/v0.1.0
