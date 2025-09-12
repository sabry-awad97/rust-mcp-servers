# Changelog

All notable changes to the Sleep MCP Server will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-09-12

### Added

- Initial release of Sleep MCP Server
- Core sleep functionality with duration-based sleeping
- Sleep until specific timestamp functionality
- Real-time status tracking and progress reporting
- Operation cancellation capabilities
- Comprehensive error handling with detailed messages
- Input validation and sanitization
- Safety limits (maximum 30-minute sleep duration)
- Support for multiple duration formats (s, m, h, ms)
- ISO 8601 timestamp parsing for sleep_until operations
- MCP server interface with tools and resources
- Structured logging with configurable levels
- Complete test coverage for all functionality
- Professional documentation and examples

### Tools

- `sleep` - Sleep for a specified duration
- `sleep_until` - Sleep until a specific timestamp
- `get_status` - Get current operation status and progress
- `cancel_sleep` - Cancel ongoing sleep operations

### Resources

- `sleep://status` - Server and operation status information
- `sleep://help` - Comprehensive help and usage guide
- `sleep://examples` - Usage examples and best practices

### Prompts

- `sleep_examples` - Interactive examples and usage patterns

### Features

- Duration parsing with flexible formats
- Progress calculation and reporting
- Graceful operation cancellation
- Thread-safe operation tracking
- Comprehensive error types and handling
- Input trimming and validation
- ISO 8601 timestamp support
- Async/await based implementation
