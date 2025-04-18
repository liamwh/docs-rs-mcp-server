use docs_rs_mcp::tools::StructDocsTool;
use mcp_sdk::tools::Tool;
use serde_json::json;

#[test]
fn test_get_struct_docs() {
    let tool = StructDocsTool::new();

    // Test with latest version
    let input = json!({
        "crate_name": "surrealdb",
        "struct_name": "Surreal"
    });

    let result = tool.call(Some(input.clone())).unwrap();
    verify_struct_docs(&result);

    // Test with specific version
    let input_with_version = json!({
        "crate_name": "surrealdb",
        "struct_name": "Surreal",
        "version": "2.2.0"
    });

    let result_with_version = tool.call(Some(input_with_version)).unwrap();
    verify_struct_docs(&result_with_version);
}

fn verify_struct_docs(result: &mcp_sdk::types::CallToolResponse) {
    if let mcp_sdk::types::ToolResponseContent::Text { text } = &result.content[0] {
        let docs: serde_json::Value = serde_json::from_str(text).unwrap();
        let docs = docs.as_object().unwrap();

        assert_eq!(docs["name"].as_str().unwrap(), "Surreal");
        assert_eq!(docs["crate_name"].as_str().unwrap(), "surrealdb");
        assert!(!docs["description"].as_str().unwrap().is_empty());

        // Verify we have methods
        let methods = docs["methods"].as_array().unwrap();
        assert!(!methods.is_empty());

        // Check first method has required fields
        let first_method = &methods[0];
        assert!(first_method["name"].as_str().unwrap().len() > 0);
        assert!(first_method["signature"].as_str().unwrap().len() > 0);
        assert!(first_method["description"].as_str().unwrap().len() > 0);

        // Verify traits
        let traits = docs["traits"].as_array().unwrap();
        assert!(!traits.is_empty());

        // Verify fields if any exist
        let fields = docs["fields"].as_array().unwrap();
        if !fields.is_empty() {
            let first_field = &fields[0];
            assert!(first_field["name"].as_str().unwrap().len() > 0);
            assert!(first_field["type_name"].as_str().unwrap().len() > 0);
        }
    } else {
        panic!("Expected Text response");
    }
}
