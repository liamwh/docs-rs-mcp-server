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

# Test a crate and module
test-module package test:
    cargo nextest run --filterset "package({{package}}) & test({{test}})"

stdin-test:
    chmod +x scripts/stdin-test.rs
    cargo script scripts/stdin-test.rs