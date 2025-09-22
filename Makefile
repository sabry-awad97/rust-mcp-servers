# Simple Makefile for MCP CLI Client

.PHONY: run install build clean help

# Default target
help:
	@echo "MCP CLI Commands:"
	@echo "  make run     - Run the chat CLI"
	@echo "  make install - Install dependencies"
	@echo "  make build   - Type check TypeScript"
	@echo "  make clean   - Clean node_modules"

# Run the CLI
run:
	cd chat-cli && bun run index.ts

# Install dependencies
install:
	cd chat-cli && bun install

# Type check
build:
	cd chat-cli && bunx tsc --noEmit

# Clean up
clean:
	cd chat-cli && rm -rf node_modules
