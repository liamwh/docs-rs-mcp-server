#!/usr/bin/env -S just --justfile
# ^ A shebang isn't required, but allows a justfile to be executed
#   like a script, with `./justfile test`, for example.

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set dotenv-load := true

# Show available commands
default:
    @just --list --justfile {{justfile()}}

# Install the MCP binary and configure Claude Desktop
install-mcp:
    chmod +x scripts/install-claude-config.rs
    cargo script scripts/install-claude-config.rs
