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

use serde_json::json;
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the JSON input
    let input = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "get_struct_docs",
            "arguments": {
                "crate_name": "surrealdb",
                "struct_name": "Surreal"
            }
        }
    });

    // Convert to string
    let input_str = serde_json::to_string(&input)?;

    // Create command with stdin pipe
    let mut child = Command::new("cargo")
        .args(["run", "--manifest-path", "docs-rs-mcp/Cargo.toml"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // Write to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input_str.as_bytes())?;
    }

    // Wait for the command to complete and get output
    let output = child.wait_with_output()?;

    // Print the output
    println!("Response from docs-rs-mcp:");
    println!("{}", String::from_utf8_lossy(&output.stdout));

    if !output.status.success() {
        println!("Error output:");
        println!("{}", String::from_utf8_lossy(&output.stderr));
        return Err("Command failed".into());
    }

    Ok(())
}
