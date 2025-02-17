#!/usr/bin/env bash
set -euo pipefail

# Build and install the binary
echo "Building and installing MCP binary..."
cd docs-mcp
cargo install --path .

# Get the config file path based on OS
if [[ "$OSTYPE" == "darwin"* ]]; then
    CONFIG_PATH="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
else
    echo "Currently only macOS is supported"
    exit 1
fi

echo "Config path: $CONFIG_PATH"

# Create config directory if it doesn't exist
mkdir -p "$(dirname "$CONFIG_PATH")"

# Create default config if file doesn't exist
if [ ! -f "$CONFIG_PATH" ]; then
    echo '{}' > "$CONFIG_PATH"
fi

# Get the full path to the installed binary
BINARY_PATH="$HOME/.cargo/bin/docs-mcp"
echo "Binary path: $BINARY_PATH"

echo "Current config content:"
cat "$CONFIG_PATH"

# Update the config file using jaq with proper quoting for paths with spaces
jaq -r --arg binary_path "$BINARY_PATH" '
    .mcpServers = (
        .mcpServers // {}
        | .mcp_example_file_system = {
            "command": $binary_path
        }
    )
' "$CONFIG_PATH" > "${CONFIG_PATH}.tmp"

if [ $? -eq 0 ]; then
    mv "${CONFIG_PATH}.tmp" "$CONFIG_PATH"
    echo -e "\nUpdated config content:"
    cat "$CONFIG_PATH"
else
    echo "jaq command failed"
    exit 1
fi

echo "Installation complete. Please restart Claude Desktop."