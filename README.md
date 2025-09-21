# 🦀 Rust MCP Servers

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://github.com/sabry-awad97/rust-mcp-servers/workflows/CI/badge.svg)](https://github.com/sabry-awad97/rust-mcp-servers/actions)

A collection of high-performance **Model Context Protocol (MCP) servers** built with Rust, providing specialized tools and resources for AI assistants and applications.

## 🌟 Available Servers

| Server                                        | Description                                              | Status     | Crates.io                                                                                                                 |
| --------------------------------------------- | -------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------- |
| [**Time MCP Server**](./crates/time_mcp/)     | Timezone-aware time operations with DST handling         | ✅ Stable  | [![Crates.io](https://img.shields.io/crates/v/mcp-server-time.svg)](https://crates.io/crates/mcp-server-time)             |
| [**Filesystem MCP Server**](./crates/fs_mcp/) | Secure filesystem operations with directory allowlisting | ✅ Stable  | [![Crates.io](https://img.shields.io/crates/v/mcp-server-filesystem.svg)](https://crates.io/crates/mcp-server-filesystem) |
| **Database MCP Server**                       | Database queries and operations                          | 🚧 Planned | -                                                                                                                         |
| **Web MCP Server**                            | HTTP requests and web scraping                           | 🚧 Planned | -                                                                                                                         |
| **System MCP Server**                         | System information and monitoring                        | 🚧 Planned | -                                                                                                                         |

## 🚀 Quick Start

### Prerequisites

- **Rust 1.85+** - [Install Rust](https://rustup.rs/)
- **Git** - For cloning the repository

### Installation

#### Install Individual Servers

```bash
# Install the Time MCP Server
cargo install mcp-server-time

# Install the Filesystem MCP Server
cargo install mcp-server-filesystem

# Future servers will be available similarly
# cargo install mcp-server-database
```

#### Build from Source

```bash
# Clone the repository
git clone https://github.com/sabry-awad97/rust-mcp-servers.git
cd rust-mcp-servers

# Build all servers
cargo build --release

# Build specific server
cargo build --release --bin mcp-server-time
cargo build --release --bin mcp-server-filesystem

# Run tests
cargo test
```

## 🛠️ Usage

### With Claude Desktop

Add servers to your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "time": {
      "command": "mcp-server-time"
    },
    "filesystem": {
      "command": "mcp-server-filesystem",
      "args": ["--allowed-dir", "/path/to/your/projects"]
    }
  }
}
```

### With MCP Inspector

Test any server using the MCP Inspector:

```bash
# Test Time MCP Server
npx @modelcontextprotocol/inspector mcp-server-time

# Test Filesystem MCP Server
npx @modelcontextprotocol/inspector mcp-server-filesystem --allowed-dir /path/to/test
```

### Direct Integration

```rust
use mcp_server_time::TimeService;
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = TimeService::new()
        .serve(stdio())
        .await?;

    service.waiting().await?;
    Ok(())
}
```

## 🏗️ Architecture

This workspace follows a modular architecture with shared dependencies and consistent patterns:

```
rust-mcp-servers/
├── crates/
│   ├── time_mcp/           # Time operations server
│   ├── fs_mcp/             # Filesystem operations server
│   ├── database_mcp/       # Database server (planned)
│   └── shared/             # Shared utilities (planned)
├── examples/               # Usage examples
├── docs/                   # Documentation
└── scripts/                # Build and deployment scripts
```

### Design Principles

- **🔧 Modular Design** - Each server is a separate crate with focused functionality
- **⚡ High Performance** - Built with Rust for speed and safety
- **🛡️ Robust Error Handling** - Comprehensive error messages and graceful failures
- **📚 Rich Documentation** - Extensive docs and examples for each server
- **🧪 Comprehensive Testing** - Unit, integration, and end-to-end tests
- **🔄 Consistent APIs** - Standardized patterns across all servers

## 🎯 Roadmap

### Phase 1: Core Servers ✅

- [x] Time MCP Server - Timezone operations and time conversion
- [x] Filesystem MCP Server - Secure file operations, search, and management

### Phase 2: System Integration 🚧

- [ ] System MCP Server - System monitoring, process management, and info

### Phase 3: Data & Web 📋

- [ ] Database MCP Server - SQL queries, schema inspection, and data operations
- [ ] Web MCP Server - HTTP requests, web scraping, and API interactions

### Phase 4: Advanced Features 🔮

- [ ] AI MCP Server - Integration with AI models and embeddings
- [ ] Network MCP Server - Network diagnostics and monitoring
- [ ] Security MCP Server - Security scanning and analysis tools

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Setup

1. **Clone and Setup**

   ```bash
   git clone https://github.com/sabry-awad97/rust-mcp-servers.git
   cd rust-mcp-servers
   ```

2. **Install Dependencies**

   ```bash
   # Install Rust toolchain
   rustup update stable

   # Install development tools
   cargo install cargo-watch cargo-tarpaulin
   ```

3. **Development Workflow**

   ```bash
   # Run tests with watch
   cargo watch -x test

   # Check code quality
   cargo clippy -- -D warnings
   cargo fmt --check

   # Generate coverage report
   cargo tarpaulin --out html
   ```

### Adding a New MCP Server

1. Create a new crate in `crates/your_server_mcp/`
2. Follow the established patterns from `time_mcp`
3. Implement the required MCP interfaces
4. Add comprehensive tests and documentation
5. Update this README with your server information

## 📖 Documentation

- **[MCP Specification](https://spec.modelcontextprotocol.io/)** - Official MCP documentation
- **[RMCP Framework](https://docs.rs/rmcp/)** - Rust MCP implementation docs
- **[Individual Server Docs](./crates/)** - Detailed documentation for each server

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests for specific server
cargo test --package mcp-server-time
cargo test --package mcp-server-filesystem

# Run with coverage
cargo tarpaulin --workspace

# Integration tests
cargo test --test integration
```

## 📊 Performance

All servers are built with performance in mind:

- **Memory Efficient** - Minimal memory footprint
- **Fast Startup** - Quick initialization times
- **Concurrent** - Async/await for high concurrency
- **Resource Aware** - Proper resource management and cleanup

## 🔒 Security

- **Input Validation** - All inputs are validated and sanitized
- **Error Handling** - No sensitive information in error messages
- **Dependencies** - Regular security audits with `cargo audit`
- **Best Practices** - Following Rust security guidelines

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **[Model Context Protocol](https://modelcontextprotocol.io/)** - For the excellent protocol specification
- **[RMCP](https://crates.io/crates/rmcp)** - Rust MCP implementation framework
- **Rust Community** - For the amazing ecosystem and tools

## 📞 Support & Community

- 📖 **Documentation**: [docs.rs/mcp-server-\*](https://docs.rs/)
- 🐛 **Issues**: [GitHub Issues](https://github.com/sabry-awad97/rust-mcp-servers/issues)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/sabry-awad97/rust-mcp-servers/discussions)
- 📧 **Contact**: [dr.sabry1997@gmail.com](mailto:dr.sabry1997@gmail.com)

## 🌟 Star History

[![Star History Chart](https://api.star-history.com/svg?repos=sabry-awad97/rust-mcp-servers&type=Date)](https://star-history.com/#sabry-awad97/rust-mcp-servers&Date)

---

<div align="center">
  <strong>Built with ❤️ and 🦀 for the MCP ecosystem</strong>
  <br>
  <sub>Making AI assistants more powerful, one server at a time</sub>
</div>
