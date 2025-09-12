# Changelog

All notable changes to the Sleep MCP Server will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-09-12

### Added

- **New `sleep_blocking` tool** - Blocking sleep operation that waits for completion before returning
- **Non-blocking architecture** - Sleep operations now run in background with operation tracking
- **Concurrent operation management** - Support for multiple simultaneous sleep operations
- **Operation ID tracking** - Unique identifiers for each sleep operation
- **Individual operation cancellation** - Cancel specific operations by ID with `cancel_operation` tool
- **Enhanced status reporting** - Detailed status with multiple operation tracking
- **Background task manager** - Professional task lifecycle management with cancellation tokens
- **Operation progress tracking** - Real-time progress updates for all active operations
- **Cleanup mechanisms** - Automatic cleanup of old completed operations

### Changed

- **BREAKING**: `sleep` tool now returns immediately with operation ID (non-blocking)
- **BREAKING**: `get_sleep_status` now supports `operation_id` parameter for specific queries
- **Enhanced**: Status responses now include multiple operations and detailed progress
- **Enhanced**: Cancellation responses now include operation counts and details
- **Improved**: Error handling with better type definitions and validation
- **Updated**: All example files and documentation to reflect new architecture

### Tools

- `sleep` - Non-blocking sleep with operation ID return (CHANGED)
- `sleep_blocking` - Blocking sleep that waits for completion (NEW)
- `sleep_until` - Sleep until specific timestamp (unchanged)
- `get_sleep_status` - Enhanced status with operation-specific queries (ENHANCED)
- `cancel_operation` - Cancel specific operation by ID (NEW)
- `cancel_sleep` - Cancel all active operations (ENHANCED)

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
