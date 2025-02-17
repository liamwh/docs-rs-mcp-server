#!/usr/bin/env cargo-script
//! ```cargo
//! [package]
//! edition = "2021"
//!
//! [dependencies]
//! serde = { version = "1.0", features = ["derive"] }
//! serde_json = "1.0"
//! ```

extern crate serde;
extern crate serde_json;

use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build and install the binary
    println!("Building and installing MCP binary...");
    let status = Command::new("cargo")
        .args(["install", "--path", "docs-rs-mcp"])
        .status()?;

    if !status.success() {
        return Err("Failed to install binary".into());
    }

    // Get config file path
    let config_path = if cfg!(target_os = "macos") {
        let home = env::var("HOME")?;
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Claude")
            .join("claude_desktop_config.json")
    } else {
        return Err("Currently only macOS is supported".into());
    };

    println!("Config path: {}", config_path.display());

    // Read existing config or create new one
    let config_str = fs::read_to_string(&config_path).unwrap_or_else(|_| "{}".to_string());
    println!("Current config:\n{}", config_str);
    let mut config: Value = serde_json::from_str(&config_str)?;

    // Get binary path
    let home = env::var("HOME")?;
    let binary_path = format!("{home}/.cargo/bin/docs-rs-mcp");
    println!("Setting binary path to: {}", binary_path);

    let mut config_changed = false;

    // Update config
    if let Value::Object(ref mut map) = config {
        // Ensure mcpServers exists and is an object
        if !map.contains_key("mcpServers") {
            map.insert("mcpServers".to_string(), json!({}));
            config_changed = true;
        }

        if let Some(Value::Object(ref mut servers)) = map.get_mut("mcpServers") {
            let current_value = servers.get("docs-rs-mcp");
            let new_value = json!({
                "command": binary_path
            });

            if current_value != Some(&new_value) {
                servers.insert("docs-rs-mcp".to_string(), new_value);
                config_changed = true;
                println!("Added/updated docs-rs-mcp server configuration");
            } else {
                println!("Config already has correct entry for docs-rs-mcp");
            }
        }
    }

    if config_changed {
        // Write updated config
        let config_str = serde_json::to_string_pretty(&config)?;
        println!("Writing updated config:\n{}", config_str);
        fs::write(&config_path, config_str)?;
        println!("Config file updated successfully.");
    } else {
        println!("No changes needed to config file.");
    }

    println!("Installation complete. Please restart Claude Desktop.");
    Ok(())
}
