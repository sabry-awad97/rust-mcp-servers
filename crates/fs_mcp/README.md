# 📁 Filesystem MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-server-filesystem.svg)](https://crates.io/crates/mcp-server-filesystem)
[![Documentation](https://docs.rs/mcp-server-filesystem/badge.svg)](https://docs.rs/mcp-server-filesystem)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A comprehensive **Model Context Protocol (MCP) server** that provides secure filesystem operations with **directory allowlisting**, robust error handling, and comprehensive file management capabilities.

## ✨ Features

- 🔒 **Security First** - Directory allowlisting prevents unauthorized access
- 📖 **Comprehensive File Reading** - Text files, media files, and batch operations
- ✏️ **Advanced File Editing** - Line-based edits with git-style diff output
- 📁 **Directory Management** - Create, list, and navigate directory structures
- 🔍 **Powerful Search** - Pattern-based file search with exclusion filters
- 🌳 **Tree Views** - Recursive directory tree visualization as JSON
- 📊 **File Information** - Detailed metadata including size, permissions, and timestamps
- 🚚 **File Operations** - Move, rename, and organize files safely
- 🛡️ **Robust Error Handling** - Comprehensive error messages with context
- 🧹 **Input Validation** - Automatic path validation and sanitization
- 📚 **Rich Documentation** - Built-in help and operation guidance
- 🎯 **SOLID Architecture** - Clean, maintainable, and testable codebase

## 🚀 Installation & Usage

### Install from Crates.io

```bash
cargo install mcp-server-filesystem
```

### Run the Server

```bash
# Start the MCP server with allowed directories (communicates via stdio)
mcp-server-filesystem --allowed-dir /path/to/allowed/directory

# Allow multiple directories
mcp-server-filesystem --allowed-dir /home/user/projects --allowed-dir /tmp/workspace

# Enable debug logging
mcp-server-filesystem --allowed-dir /path/to/dir --log-level debug
```

### Test with MCP Inspector

```bash
# Install and run the MCP Inspector to test the server
npx @modelcontextprotocol/inspector mcp-server-filesystem --allowed-dir /path/to/test/dir
```

### Use with Claude Desktop

Add to your Claude Desktop MCP configuration:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "mcp-server-filesystem",
      "args": ["--allowed-dir", "/path/to/your/projects", "--allowed-dir", "/path/to/documents"]
    }
  }
}
```

## 🛠️ Available Tools

### File Reading Operations

### `read_text_file`

Read the complete contents of a text file with optional head/tail functionality.

**Parameters:**

- `path` (string): Path to the file to read
- `head` (optional number): Read only the first N lines
- `tail` (optional number): Read only the last N lines

**Example Request:**

```json
{
  "path": "/home/user/projects/README.md",
  "head": 10
}
```

**Example Response:**

```json
{
  "content": "# My Project\n\nThis is a sample project...",
  "path": "/home/user/projects/README.md",
  "size": 1024,
  "encoding": "utf-8"
}
```

### `read_media_file`

Read image or audio files and return base64 encoded data with MIME type detection.

**Parameters:**

- `path` (string): Path to the media file

**Example Request:**

```json
{
  "path": "/home/user/images/photo.jpg"
}
```

**Example Response:**

```json
{
  "content": "/9j/4AAQSkZJRgABAQEAYABgAAD...",
  "mime_type": "image/jpeg",
  "path": "/home/user/images/photo.jpg",
  "size": 245760
}
```

### `read_multiple_files`

Read multiple files simultaneously for efficient batch operations.

**Parameters:**

- `paths` (array of strings): Array of file paths to read

**Example Request:**

```json
{
  "paths": [
    "/home/user/config.json",
    "/home/user/settings.yaml",
    "/home/user/data.txt"
  ]
}
```

### File Writing Operations

### `write_file`

Create a new file or completely overwrite an existing file with new content.

**Parameters:**

- `path` (string): Path where the file should be written
- `content` (string): Content to write to the file

**Example Request:**

```json
{
  "path": "/home/user/projects/new_file.txt",
  "content": "Hello, World!\nThis is a new file."
}
```

### `edit_file`

Make line-based edits to a text file with git-style diff output.

**Parameters:**

- `path` (string): Path to the file to edit
- `edits` (array): Array of edit operations
- `dry_run` (optional boolean): Preview changes without applying them

**Edit Operation Format:**

```json
{
  "old_text": "original line content",
  "new_text": "new line content"
}
```

**Example Request:**

```json
{
  "path": "/home/user/config.py",
  "edits": [
    {
      "old_text": "DEBUG = False",
      "new_text": "DEBUG = True"
    }
  ],
  "dry_run": false
}
```

### Directory Operations

### `create_directory`

Create a new directory or ensure a directory exists, including parent directories.

**Parameters:**

- `path` (string): Path of the directory to create

**Example Request:**

```json
{
  "path": "/home/user/projects/new_project/src"
}
```

### `list_directory`

Get a detailed listing of all files and directories in a specified path.

**Parameters:**

- `path` (string): Path to the directory to list

**Example Response:**

```json
{
  "entries": [
    {
      "name": "README.md",
      "type": "[FILE]",
      "size": 1024
    },
    {
      "name": "src",
      "type": "[DIRECTORY]",
      "size": 0
    }
  ],
  "path": "/home/user/projects",
  "total_entries": 2
}
```

### `list_directory_with_sizes`

Get a detailed listing with file sizes and sorting options.

**Parameters:**

- `path` (string): Path to the directory to list
- `sort_by` (string): Sort criteria ("name", "size", "modified")

### `directory_tree`

Get a recursive tree view of files and directories as JSON.

**Parameters:**

- `path` (string): Root path for the tree
- `exclude_patterns` (optional array): Glob patterns to exclude

**Example Request:**

```json
{
  "path": "/home/user/projects",
  "exclude_patterns": ["*.log", "node_modules/**", ".git/**"]
}
```

### File Management Operations

### `move_file`

Move or rename files and directories safely.

**Parameters:**

- `source` (string): Source path
- `destination` (string): Destination path

**Example Request:**

```json
{
  "source": "/home/user/old_name.txt",
  "destination": "/home/user/documents/new_name.txt"
}
```

### `search_files`

Search for files and directories matching a pattern with exclusion support.

**Parameters:**

- `path` (string): Directory to search in
- `pattern` (string): Glob pattern to match
- `exclude_patterns` (optional array): Patterns to exclude

**Example Request:**

```json
{
  "path": "/home/user/projects",
  "pattern": "*.rs",
  "exclude_patterns": ["target/**", "*.tmp"]
}
```

### `get_file_info`

Retrieve detailed metadata about a file or directory.

**Parameters:**

- `path` (string): Path to the file or directory

**Example Response:**

```json
{
  "name": "config.json",
  "type": "[FILE]",
  "size": 2048,
  "is_directory": false,
  "modified": 1642694400,
  "path": "/home/user/config.json",
  "permissions": {
    "readable": true,
    "writable": true,
    "executable": false
  }
}
```

### Utility Operations

### `list_allowed_directories`

Returns the list of directories that this server is allowed to access.

**Example Response:**

```
Allowed directories:
/home/user/projects
/home/user/documents
/tmp/workspace
```

## 🔧 Configuration

### Command Line Options

```bash
mcp-server-filesystem [OPTIONS]

Options:
  -a, --allowed-dir <PATH>    Add an allowed directory (can be used multiple times)
  -l, --log-level <LEVEL>     Set logging level [default: info] [possible values: trace, debug, info, warn, error]
  -f, --log-format <FORMAT>   Set log format [default: pretty] [possible values: pretty, json, compact]
      --help                  Print help information
      --version               Print version information
```

### Security Model

The server implements a strict security model:

- **Directory Allowlisting**: Only specified directories can be accessed
- **Path Validation**: All paths are validated and normalized
- **Symlink Protection**: Symlinks are handled safely with warnings
- **Size Limits**: Configurable file size limits prevent abuse
- **Error Sanitization**: Error messages don't leak sensitive information

## 📖 Usage Examples

### With Claude Desktop

Once configured, you can ask Claude:

> "Read the contents of my project's README.md file"

> "Create a new directory structure for a Python project"

> "Search for all Python files in my project and show their contents"

> "Edit my configuration file to enable debug mode"

> "Show me a tree view of my project directory, excluding build artifacts"

### With MCP Inspector

```bash
# Test the server interactively
npx @modelcontextprotocol/inspector mcp-server-filesystem --allowed-dir /path/to/test

# Try these operations:
# 1. Use read_text_file to examine files
# 2. Use directory_tree to explore structure
# 3. Use search_files to find specific files
# 4. Use edit_file to make changes with dry_run: true
```

### Command Line Testing

```bash
# Use MCP Inspector for interactive testing
npx @modelcontextprotocol/inspector mcp-server-filesystem --allowed-dir /home/user/projects

# Test specific operations:
# - File reading: read_text_file, read_multiple_files
# - Directory operations: list_directory, directory_tree
# - File management: write_file, edit_file, move_file
# - Search: search_files with various patterns
```

## 🚨 Error Handling

The server provides detailed error messages for common issues:

- **Path Not Found**: Clear indication when files or directories don't exist
- **Permission Denied**: Helpful messages for access issues
- **Invalid Patterns**: Guidance for correct glob pattern syntax
- **Validation Errors**: Specific feedback on parameter validation failures
- **Security Violations**: Safe error messages for unauthorized access attempts

## 🧪 Testing

Run the comprehensive test suite:

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test read_files
cargo test search_files
cargo test get_file_info

# Run with coverage
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
- File operations powered by [tokio](https://crates.io/crates/tokio)
- Pattern matching via [globset](https://crates.io/crates/globset)
- MIME type detection using [mime_guess](https://crates.io/crates/mime_guess)

## 📞 Support

- 📖 [Documentation](https://docs.rs/mcp-server-filesystem)
- 🐛 [Issue Tracker](https://github.com/sabry-awad97/rust-mcp-servers/issues)
- 💬 [Discussions](https://github.com/sabry-awad97/rust-mcp-servers/discussions)

---

<div align="center">
  <strong>Made with ❤️ for secure filesystem operations in the MCP ecosystem</strong>
</div>
