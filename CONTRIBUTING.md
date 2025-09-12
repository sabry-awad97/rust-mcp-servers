# Contributing to Rust MCP Servers

Thank you for your interest in contributing to the Rust MCP Servers project! This document provides guidelines and information for contributors.

## 🚀 Getting Started

### Prerequisites

- **Rust 1.85+** - [Install Rust](https://rustup.rs/)
- **Git** - For version control
- **IDE/Editor** - VS Code with rust-analyzer recommended

### Development Setup

1. **Fork and Clone**

   ```bash
   git clone https://github.com/sabry-awad97/rust-mcp-servers.git
   cd rust-mcp-servers
   ```

2. **Install Development Tools**

   ```bash
   # Essential tools
   cargo install cargo-watch cargo-tarpaulin cargo-audit

   # Optional but recommended
   cargo install cargo-expand cargo-machete
   ```

3. **Verify Setup**
   ```bash
   cargo build
   cargo test
   cargo clippy
   ```

## 🏗️ Project Structure

```
rust-mcp-servers/
├── crates/
│   ├── time_mcp/           # Time operations server
│   └── your_server_mcp/    # Your new server
├── examples/               # Usage examples
├── docs/                   # Additional documentation
├── scripts/                # Build and deployment scripts
└── .github/                # CI/CD workflows
```

## 📝 Coding Standards

### Code Style

- **Formatting**: Use `cargo fmt` (rustfmt)
- **Linting**: Pass `cargo clippy -- -D warnings`
- **Documentation**: Document all public APIs with `///` comments
- **Testing**: Maintain test coverage above 80%

### Design Principles

1. **SOLID Principles** - Follow single responsibility, open/closed, etc.
2. **Domain-Driven Design** - Organize code by business domains
3. **Error Handling** - Use `Result<T, E>` and custom error types
4. **Performance** - Prefer zero-cost abstractions
5. **Safety** - Leverage Rust's type system for correctness

### File Organization

```rust
// Filename: your_feature.rs
// Folder: /crates/server_name/src/core/

//! Module documentation
//!
//! Describes the purpose and usage of this module.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Public struct with documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YourStruct {
    /// Field documentation
    pub field: String,
}

impl YourStruct {
    /// Constructor documentation
    pub fn new(field: String) -> Self {
        Self { field }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_your_struct_creation() {
        let instance = YourStruct::new("test".to_string());
        assert_eq!(instance.field, "test");
    }
}
```

## 🔧 Adding a New MCP Server

### 1. Create the Crate Structure

```bash
mkdir -p crates/your_server_mcp/src/{core,examples}
cd crates/your_server_mcp
```

### 2. Setup Cargo.toml

```toml
[package]
name = "mcp-server-your-name"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "Description of your MCP server"
documentation = "https://docs.rs/mcp-server-your-name"
readme = "README.md"
homepage = "https://github.com/sabry-awad97/rust-mcp-servers"
repository = "https://github.com/sabry-awad97/rust-mcp-servers"
license = "MIT"
keywords = ["mcp", "your-domain", "server"]
categories = ["command-line-utilities", "web-programming"]

[[bin]]
name = "mcp-server-your-name"
path = "src/main.rs"

[dependencies]
rmcp = { workspace = true, features = ["transport-io", "server", "schemars"] }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
tokio = { workspace = true, features = ["full"] }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter"] }
thiserror = { workspace = true }
```

### 3. Implement Core Structure

```rust
// src/lib.rs
//! # Your MCP Server
//!
//! Description of what your server does.

mod core;
mod server;

pub use server::YourService;

// src/main.rs
use mcp_server_your_name::YourService;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let service = YourService::new()
        .serve(stdio())
        .await?;

    service.waiting().await?;
    Ok(())
}
```

### 4. Required Files

- `README.md` - Comprehensive documentation
- `CHANGELOG.md` - Version history
- `LICENSE` - MIT license
- `examples/` - Usage examples

## 🧪 Testing Guidelines

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Arrange
        let input = "test input";

        // Act
        let result = your_function(input);

        // Assert
        assert_eq!(result, expected_output);
    }

    #[tokio::test]
    async fn test_async_functionality() {
        let service = YourService::new();
        let result = service.async_method().await;
        assert!(result.is_ok());
    }
}
```

### Test Categories

- **Unit Tests** - Test individual functions and methods
- **Integration Tests** - Test component interactions
- **End-to-End Tests** - Test complete workflows
- **Property Tests** - Test with generated inputs (optional)

### Running Tests

```bash
# All tests
cargo test

# Specific server
cargo test --package mcp-server-your-name

# With coverage
cargo tarpaulin --package mcp-server-your-name

# Watch mode
cargo watch -x test
```

## 📚 Documentation

### Code Documentation

- Use `///` for public API documentation
- Include examples in doc comments
- Document error conditions and panics
- Use `//!` for module-level documentation

### README Template

Each server should have a comprehensive README with:

- Clear description and features
- Installation instructions
- Usage examples
- API documentation
- Error handling examples
- Contributing guidelines

## 🔄 Development Workflow

### 1. Create Feature Branch

```bash
git checkout -b feature/your-feature-name
```

### 2. Development Cycle

```bash
# Make changes
# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings

# Check security
cargo audit
```

### 3. Commit Guidelines

Use conventional commits:

```
feat: add new MCP server for file operations
fix: resolve timezone parsing issue
docs: update README with new examples
test: add integration tests for time conversion
refactor: extract common error handling
```

### 4. Pull Request Process

1. **Update Documentation** - README, CHANGELOG, etc.
2. **Add Tests** - Ensure good test coverage
3. **Run CI Checks** - All checks must pass
4. **Request Review** - Tag relevant maintainers

## 🚀 Release Process

### Version Bumping

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** - Breaking changes
- **MINOR** - New features (backward compatible)
- **PATCH** - Bug fixes (backward compatible)

### Publishing to Crates.io

```bash
# Dry run
cargo publish --dry-run

# Publish
cargo publish
```

## 🤝 Community Guidelines

### Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Help newcomers learn
- Focus on technical merit

### Communication

- **Issues** - Bug reports and feature requests
- **Discussions** - General questions and ideas
- **Pull Requests** - Code contributions
- **Email** - Private or sensitive matters

## 🛠️ Tools and Resources

### Recommended Tools

- **IDE**: VS Code with rust-analyzer
- **Debugging**: `cargo expand`, `cargo machete`
- **Profiling**: `cargo flamegraph`, `perf`
- **Documentation**: `cargo doc --open`

### Useful Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [RMCP Documentation](https://docs.rs/rmcp/)

## ❓ Getting Help

- **Documentation**: Check existing docs first
- **Issues**: Search existing issues
- **Discussions**: Ask questions in GitHub Discussions
- **Discord/Chat**: Join the MCP community channels

## 🏆 Recognition

Contributors will be:

- Listed in the project README
- Mentioned in release notes
- Given credit in documentation
- Invited to maintainer discussions (for significant contributions)

---

Thank you for contributing to Rust MCP Servers! Your efforts help make AI assistants more powerful and accessible. 🚀
