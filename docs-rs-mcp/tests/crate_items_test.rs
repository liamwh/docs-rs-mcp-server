use anyhow::Result;
use docs_rs_mcp::tools::CrateItemsTool;
use mcp_sdk::{tools::Tool, types::ToolResponseContent};
use serde_json::json;

#[test]
fn test_crate_items_serde() -> Result<()> {
    let tool = CrateItemsTool::new();

    let response = tool.call(Some(json!({
        "crate_name": "serde"
    })))?;

    // Get the response text
    let content = match &response.content[0] {
        ToolResponseContent::Text { text } => text.as_str(),
        _ => panic!("Expected text response"),
    };

    // Parse the JSON response
    let info: serde_json::Value = serde_json::from_str(content)?;

    // Basic validation
    assert_eq!(info["crate_name"].as_str().unwrap(), "serde");
    assert!(!info["version"].as_str().unwrap().is_empty());

    let items = info["items"].as_object().unwrap();

    // Serde should have these categories
    assert!(items.contains_key("Structs"));
    assert!(items.contains_key("Traits"));

    // Check some known items
    let traits = items["Traits"].as_array().unwrap();
    assert!(traits
        .iter()
        .any(|t| t["name"].as_str().unwrap() == "Serialize"));
    assert!(traits
        .iter()
        .any(|t| t["name"].as_str().unwrap() == "Deserialize"));

    // Verify links
    for (_, category_items) in items {
        for item in category_items.as_array().unwrap() {
            assert!(item["doc_link"]
                .as_str()
                .unwrap()
                .starts_with("https://docs.rs"));
        }
    }

    Ok(())
}

#[test]
fn test_crate_items_scraper() -> Result<()> {
    let tool = CrateItemsTool::new();

    println!("Fetching scraper crate items...");
    let response = tool.call(Some(json!({
        "crate_name": "scraper",
        "version": "0.22.0"
    })))?;

    println!("Got response, extracting content...");
    let content = match &response.content[0] {
        ToolResponseContent::Text { text } => {
            println!("Raw response content: {}", text);
            text.as_str()
        }
        _ => panic!("Expected text response"),
    };

    println!("Parsing JSON response...");
    let info: serde_json::Value = serde_json::from_str(content)?;

    // Basic validation
    println!("Validating basic crate info...");
    assert_eq!(info["crate_name"].as_str().unwrap(), "scraper");
    assert_eq!(info["version"].as_str().unwrap(), "0.22.0");

    let items = info["items"].as_object().unwrap();
    println!("Found categories: {:?}", items.keys().collect::<Vec<_>>());

    // Verify all expected categories exist
    println!("Checking for required categories...");
    assert!(items.contains_key("Structs"), "Missing Structs category");
    assert!(items.contains_key("Enums"), "Missing Enums category");
    assert!(items.contains_key("Traits"), "Missing Traits category");
    assert!(
        items.contains_key("Type Aliases"),
        "Missing Type Aliases category"
    );

    // Verify specific structs
    println!("Checking structs...");
    let structs = items["Structs"].as_array().unwrap();
    let struct_names: Vec<&str> = structs
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    println!("Found structs: {:?}", struct_names);

    assert!(
        struct_names.contains(&"element_ref::ElementRef"),
        "Missing ElementRef struct"
    );
    assert!(struct_names.contains(&"html::Html"), "Missing Html struct");
    assert!(
        struct_names.contains(&"selector::Selector"),
        "Missing Selector struct"
    );
    assert!(
        struct_names.contains(&"selector::CssLocalName"),
        "Missing CssLocalName struct"
    );

    // Verify specific enums
    println!("Checking enums...");
    let enums = items["Enums"].as_array().unwrap();
    let enum_names: Vec<&str> = enums.iter().map(|e| e["name"].as_str().unwrap()).collect();
    println!("Found enums: {:?}", enum_names);

    assert!(
        enum_names.contains(&"CaseSensitivity"),
        "Missing CaseSensitivity enum"
    );
    assert!(enum_names.contains(&"node::Node"), "Missing Node enum");

    // Verify specific traits
    println!("Checking traits...");
    let traits = items["Traits"].as_array().unwrap();
    let trait_names: Vec<&str> = traits.iter().map(|t| t["name"].as_str().unwrap()).collect();
    println!("Found traits: {:?}", trait_names);

    assert!(trait_names.contains(&"Element"), "Missing Element trait");
    assert!(
        trait_names.contains(&"selectable::Selectable"),
        "Missing Selectable trait"
    );

    // Verify links format
    println!("Checking documentation links...");
    for (category, category_items) in items {
        println!("Checking links for category: {}", category);
        for item in category_items.as_array().unwrap() {
            let doc_link = item["doc_link"].as_str().unwrap();
            println!("  Link: {}", doc_link);
            assert!(
                doc_link.starts_with("https://docs.rs/scraper/0.22.0/scraper/"),
                "Invalid link prefix: {}",
                doc_link
            );
            assert!(
                doc_link.ends_with(".html"),
                "Invalid link format: {}",
                doc_link
            );
        }
    }

    Ok(())
}

#[test]
fn test_crate_items_nonexistent() {
    let tool = CrateItemsTool::new();

    let result = tool.call(Some(json!({
        "crate_name": "this-crate-definitely-does-not-exist-12345"
    })));

    assert!(result.is_err());
}

#[test]
fn test_crate_items_invalid_version() -> Result<()> {
    let tool = CrateItemsTool::new();

    let result = tool.call(Some(json!({
        "crate_name": "scraper",
        "version": "999.999.999"
    })));

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Failed to fetch docs.rs page"));
    }

    Ok(())
}
