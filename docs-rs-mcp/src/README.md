# MCP Server for docs.rs

This is a MCP server for docs.rs. It allows you to search the docs.rs documentation for a given query.

## Tools

- **lookup_crate**
  - Lookup a crate by name
  - Input: `crate_name` (string)
  - Returns: `crate_name` (string)

## How to Build and Run Example Locally

## Prerequisites

- The latest version of Claude Desktop installed
- Install [Rust](https://www.rust-lang.org/tools/install)

## Build and Install Binary

```bash
cd docs-mcp
cargo install --path .
```

This will build the binary and install it to your local cargo bin directory. Later you will need to configure Claude Desktop to use this binary.

### Configure Claude Desktop

If you are using macOS, open the `claude_desktop_config.json` file in a text editor:

```bash
code ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

Modify the `claude_desktop_config.json` file to include the following configuration:
(replace YOUR_USERNAME with your actual username):

```json
{
  "mcpServers": {
    "mcp_example_file_system": {
      "command": "/Users/YOUR_USERNAME/.cargo/bin/file_system"
    }
  }
}
```

Save the file, and restart Claude Desktop.
