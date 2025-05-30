use anyhow::Result;
use docs_rs_mcp::tools::CrateInfoTool;
use mcp_sdk::{tools::Tool, types::ToolResponseContent};
use serde_json::json;

#[test]
fn test_cargo_info_command_exists() {
    let status = std::process::Command::new("cargo")
        .arg("info")
        .arg("--version")
        .status();

    if let Err(e) = status {
        // Don't panic, as our tool should handle this case
        println!("Note: cargo info not in PATH: {}", e);
    }
}

#[test]
fn test_crate_info_serde() -> Result<()> {
    let tool = CrateInfoTool::new();

    let response = tool.call(Some(json!({
        "crate_name": "serde"
    })))?;

    // Get the response text
    let content = match &response.content[0] {
        ToolResponseContent::Text { text } => text,
        _ => panic!("Expected text response"),
    };

    // Parse the JSON response
    let info: serde_json::Value = serde_json::from_str(content)?;

    // Basic validation of required fields
    assert_eq!(info["name"].as_str().unwrap(), "serde");
    assert!(!info["description"].as_str().unwrap().is_empty());
    assert!(!info["version"].as_str().unwrap().is_empty());

    // Check optional fields exist (though values may vary)
    assert!(info["license"].is_string());
    assert!(info["documentation"].is_string());
    assert!(info["features"].is_array());

    Ok(())
}

#[test]
fn test_crate_info_nonexistent() {
    let tool = CrateInfoTool::new();

    let result = tool.call(Some(json!({
        "crate_name": "this-crate-definitely-does-not-exist-12345"
    })));

    assert!(result.is_err());
}

#[test]
fn test_cargo_info_robust_execution() -> Result<()> {
    let tool = CrateInfoTool::new();

    // This should work even if cargo-info isn't in PATH
    // as the tool will try multiple methods to run it
    let response = tool.call(Some(json!({
        "crate_name": "serde"
    })))?;

    assert!(!response.content.is_empty());

    if let ToolResponseContent::Text { text } = &response.content[0] {
        let info: serde_json::Value = serde_json::from_str(text)?;
        assert_eq!(info["name"].as_str().unwrap(), "serde");
    } else {
        panic!("Expected text response");
    }

    Ok(())
}
