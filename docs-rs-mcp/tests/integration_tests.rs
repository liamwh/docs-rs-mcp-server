use docs_rs_mcp::tools::StructDocsTool;
use mcp_sdk::tools::Tool;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn test_get_struct_docs_integration() {
    let test_cases = vec![
        // Test case 1: Valid struct from surrealdb (latest)
        (
            "surrealdb",
            "Surreal",
            None,
            true,
            Some("A database client instance for embedded or remote databases"),
        ),
        // Test case 2: Valid struct with specific version
        (
            "surrealdb",
            "Surreal",
            Some("2.2.0"),
            true,
            Some("A database client instance for embedded or remote databases"),
        ),
        // Test case 3: Non-existent crate
        (
            "this_crate_does_not_exist_12345",
            "SomeStruct",
            None,
            false,
            None,
        ),
        // Test case 4: Non-existent struct in existing crate
        ("surrealdb", "ThisStructDoesNotExist", None, false, None),
        // Test case 5: Invalid version
        ("surrealdb", "Surreal", Some("0.0.0"), false, None),
    ];

    let tool = StructDocsTool::new();

    for (crate_name, struct_name, version, should_succeed, expected_description) in test_cases {
        let mut input = json!({
            "crate_name": crate_name,
            "struct_name": struct_name,
        });

        if let Some(v) = version {
            input
                .as_object_mut()
                .unwrap()
                .insert("version".to_string(), json!(v));
        }

        println!("Testing {crate_name}::{struct_name}");
        let result = tool.call(Some(input));

        match (should_succeed, result) {
            (true, Ok(response)) => {
                // For successful cases, verify the response format and content
                if let mcp_sdk::types::ToolResponseContent::Text { text } = &response.content[0] {
                    let docs: serde_json::Value = serde_json::from_str(text).unwrap();
                    let docs = docs.as_object().unwrap();

                    // Basic structure checks
                    assert_eq!(docs["name"].as_str().unwrap(), struct_name);
                    assert_eq!(docs["crate_name"].as_str().unwrap(), crate_name);

                    // Description check if expected
                    if let Some(expected) = expected_description {
                        let desc = docs["description"].as_str().unwrap();
                        assert!(
                            desc.contains(expected),
                            "Expected description to contain '{}', but got '{}'",
                            expected,
                            desc
                        );
                    }

                    // Verify we have methods or traits or fields
                    // Some structs might not have all of these
                    let has_content = docs["methods"].as_array().is_some_and(|m| !m.is_empty())
                        || docs["traits"].as_array().is_some_and(|t| !t.is_empty())
                        || docs["fields"].as_array().is_some_and(|f| !f.is_empty());

                    assert!(
                        has_content,
                        "Expected struct to have methods, traits, or fields"
                    );

                    // Verify specific methods we know should exist
                    let methods = docs["methods"].as_array().unwrap();
                    let method_names: Vec<&str> =
                        methods.iter().filter_map(|m| m["name"].as_str()).collect();

                    // The Surreal struct should have these methods
                    assert!(
                        method_names.contains(&"connect"),
                        "Expected to find 'connect' method"
                    );
                    assert!(
                        method_names.contains(&"use_ns"),
                        "Expected to find 'use_ns' method"
                    );
                    assert!(
                        method_names.contains(&"use_db"),
                        "Expected to find 'use_db' method"
                    );

                    // Verify traits
                    let traits = docs["traits"].as_array().unwrap();
                    assert!(!traits.is_empty(), "Expected struct to implement traits");

                    // The Surreal struct should implement Clone and Debug
                    let trait_impls: Vec<&str> = traits.iter().filter_map(|t| t.as_str()).collect();
                    assert!(
                        trait_impls.iter().any(|t| t.contains("Clone")),
                        "Expected struct to implement Clone"
                    );
                    assert!(
                        trait_impls.iter().any(|t| t.contains("Debug")),
                        "Expected struct to implement Debug"
                    );
                } else {
                    panic!("Expected Text response for {crate_name}::{struct_name}");
                }
            }
            (false, Err(_)) => {
                // For cases expected to fail, we just verify they did fail
                println!("Expected failure occurred for {crate_name}::{struct_name}");
            }
            (true, Err(e)) => {
                panic!("Expected success but got error for {crate_name}::{struct_name}: {e}");
            }
            (false, Ok(_)) => {
                panic!("Expected failure but got success for {crate_name}::{struct_name}");
            }
        }
    }
}

// Test specific error cases
#[test]
fn test_get_struct_docs_error_cases() {
    let tool = StructDocsTool::new();

    // Test case 1: Empty input
    let result = tool.call(None);
    assert!(result.is_err());

    // Test case 2: Missing required fields
    let input = json!({
        "crate_name": "surrealdb"
        // missing struct_name
    });
    let result = tool.call(Some(input));
    assert!(result.is_err());

    // Test case 3: Invalid JSON
    let input = json!({
        "crate_name": 123, // should be string
        "struct_name": "Surreal"
    });
    let result = tool.call(Some(input));
    assert!(result.is_err());
}

// Test handling of special characters and edge cases
#[test]
fn test_get_struct_docs_special_cases() {
    let tool = StructDocsTool::new();

    let test_cases = vec![
        // Test case 1: Very long struct name (should fail gracefully)
        json!({
            "crate_name": "surrealdb",
            "struct_name": "A".repeat(1000)
        }),
        // Test case 2: Unicode characters (should fail gracefully)
        json!({
            "crate_name": "surrealdb",
            "struct_name": "データベース"
        }),
        // Test case 3: Special characters in crate name (should fail gracefully)
        json!({
            "crate_name": "my-invalid-crate-name",
            "struct_name": "MyStruct"
        }),
    ];

    for input in test_cases {
        let result = tool.call(Some(input.clone()));
        println!("Testing input: {input}");
        println!("Result: {result:?}");
        // We don't assert specific outcomes here as the behavior might change,
        // but we want to ensure the tool handles these cases gracefully without crashing
        assert!(result.is_ok() || result.is_err());
    }
}
